use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use axagent_core::entity::{notes, wiki_pages, wiki_sources, wiki_operations, wikis};
use axagent_core::repo::note::{CreateNoteInput, Note};
use axagent_core::utils::gen_id;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, Set};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledPage {
    pub title: String,
    pub content: String,
    pub page_type: String,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResult {
    pub new_pages: Vec<CompiledPage>,
    pub updated_pages: Vec<CompiledPage>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageCompileResult {
    pub page: CompiledPage,
    pub score: f64,
}

pub struct WikiCompiler {
    db: Arc<DatabaseConnection>,
}

impl WikiCompiler {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn compile(&self, wiki_id: &str, source_ids: Vec<String>) -> Result<CompileResult, String> {
        let schema = self.read_schema(wiki_id).await?;
        let sources = self.load_sources(wiki_id, &source_ids).await?;
        let pages = self.llm_compile(&schema, &sources).await?;

        let mut result = CompileResult {
            new_pages: Vec::new(),
            updated_pages: Vec::new(),
            errors: Vec::new(),
        };

        for page in pages {
            let page_clone = page.clone();
            match self.save_page(wiki_id, &page).await {
                Ok((note, is_updated)) => {
                    if is_updated {
                        result.updated_pages.push(page_clone.clone());
                    } else {
                        result.new_pages.push(page_clone.clone());
                    }

                    if let Err(e) = self.update_quality_score(&note, &page_clone).await {
                        tracing::warn!("Failed to update quality score: {}", e);
                    }
                }
                Err(e) => result.errors.push(e),
            }
        }

        self.update_index(wiki_id).await?;
        self.update_log(wiki_id, "compile", &result).await?;

        Ok(result)
    }

    async fn read_schema(&self, wiki_id: &str) -> Result<String, String> {
        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let schema_path = Path::new(&wiki.root_path).join("SCHEMA.md");
        tokio::fs::read_to_string(&schema_path)
            .await
            .map_err(|e| e.to_string())
    }

    async fn load_sources(&self, wiki_id: &str, source_ids: &[String]) -> Result<Vec<wiki_sources::Model>, String> {
        let sources = wiki_sources::Entity::find()
            .filter(wiki_sources::Column::WikiId.eq(wiki_id))
            .filter(wiki_sources::Column::Id.is_in(source_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(sources)
    }

    async fn llm_compile(&self, schema: &str, sources: &[wiki_sources::Model]) -> Result<Vec<CompiledPage>, String> {
        let source_contents: Vec<String> = sources
            .iter()
            .map(|s| format!("## {}\nSource: {}\n", s.title, s.source_path))
            .collect();

        let _prompt = format!(
            r#"你是知识工程师。根据以下 SCHEMA 和原始资料，编译成结构化 wiki 页面。

SCHEMA:
{}

原始资料:
{}

输出要求：
1. 为每个独立概念/实体创建页面
2. 每个页面使用 Markdown 格式，包含 frontmatter
3. 创建源摘要页面
"#,
            schema,
            source_contents.join("\n\n")
        );

        Ok(vec![])
    }

    async fn save_page(&self, wiki_id: &str, page: &CompiledPage) -> Result<(Note, bool), String> {
        let file_path = format!("notes/{}.md", page.title.replace(" ", "-").to_lowercase());

        let existing_note = self.find_existing_note_by_title(wiki_id, &page.title).await?;

        if let Some(ref note) = existing_note {
            if !self.should_overwrite(note).await? {
                return Err(format!("Note '{}' exists and should not be overwritten (user edited or not LLM authored)", page.title));
            }

            let content_hash = axagent_core::repo::note::calculate_content_hash(&page.content);
            if note.content_hash == content_hash {
                return Ok((note.clone(), false));
            }

            let input = axagent_core::repo::note::UpdateNoteInput {
                title: Some(page.title.clone()),
                content: Some(page.content.clone()),
                page_type: Some(page.page_type.clone()),
                related_pages: None,
            };

            let updated_note = axagent_core::repo::note::update_note(self.db.as_ref(), &note.id, input)
                .await
                .map_err(|e| e.to_string())?;

            self.update_wiki_page(&updated_note, page).await?;

            return Ok((updated_note, true));
        }

        let input = CreateNoteInput {
            vault_id: wiki_id.to_string(),
            title: page.title.clone(),
            file_path: file_path.clone(),
            content: page.content.clone(),
            author: "llm".to_string(),
            page_type: Some(page.page_type.clone()),
            source_refs: Some(page.source_ids.clone()),
        };

        let note = axagent_core::repo::note::create_note(self.db.as_ref(), input)
            .await
            .map_err(|e| e.to_string())?;

        self.create_wiki_page(wiki_id, &note, page).await?;

        Ok((note, false))
    }

    async fn find_existing_note_by_title(&self, wiki_id: &str, title: &str) -> Result<Option<Note>, String> {
        let notes = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(wiki_id))
            .filter(notes::Column::Title.eq(title))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(notes.into_iter().next().map(|n| axagent_core::repo::note::model_to_note(n)))
    }

    async fn update_quality_score(&self, note: &Note, page: &CompiledPage) -> Result<(), String> {
        let score = self.calculate_quality_score(page).await;

        let wiki_page = wiki_pages::Entity::find()
            .filter(wiki_pages::Column::NoteId.eq(&note.id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        if let Some(wp) = wiki_page {
            let mut am = wp.into_active_model();
            am.quality_score = Set(Some(score));
            am.last_linted_at = Set(Some(chrono::Utc::now().timestamp()));
            am.update(self.db.as_ref()).await.map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    async fn update_wiki_page(&self, note: &Note, page: &CompiledPage) -> Result<(), String> {
        let wiki_page = wiki_pages::Entity::find()
            .filter(wiki_pages::Column::NoteId.eq(&note.id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        if let Some(wp) = wiki_page {
            let mut am = wp.into_active_model();
            am.last_compiled_at = Set(chrono::Utc::now().timestamp());
            am.compiled_source_hash = Set(Some(axagent_core::repo::note::calculate_content_hash(&page.content)));
            am.update(self.db.as_ref()).await.map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    async fn create_wiki_page(&self, wiki_id: &str, note: &Note, page: &CompiledPage) -> Result<(), String> {
        let wiki_page_model = wiki_pages::ActiveModel {
            id: Set(gen_id()),
            wiki_id: Set(wiki_id.to_string()),
            note_id: Set(note.id.clone()),
            page_type: Set(page.page_type.clone()),
            title: Set(page.title.clone()),
            source_ids: Set(Some(serde_json::to_value(&page.source_ids).unwrap_or_default())),
            quality_score: Set(None),
            last_linted_at: Set(None),
            last_compiled_at: Set(chrono::Utc::now().timestamp()),
            compiled_source_hash: Set(Some(axagent_core::repo::note::calculate_content_hash(&page.content))),
            created_at: Set(chrono::Utc::now().timestamp()),
            updated_at: Set(chrono::Utc::now().timestamp()),
        };

        wiki_page_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn update_index(&self, _wiki_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn update_log(&self, wiki_id: &str, operation: &str, result: &CompileResult) -> Result<(), String> {
        let log_model = wiki_operations::ActiveModel {
            wiki_id: Set(wiki_id.to_string()),
            operation_type: Set(operation.to_string()),
            target_type: Set("compile".to_string()),
            target_id: Set(gen_id()),
            status: Set(if result.errors.is_empty() {
                "completed"
            } else {
                "partial"
            }
            .to_string()),
            details_json: Set(Some(serde_json::to_value(result).unwrap_or_default())),
            error_message: Set(None),
            created_at: Set(chrono::Utc::now().timestamp()),
            completed_at: Set(Some(chrono::Utc::now().timestamp())),
            ..Default::default()
        };

        log_model.insert(self.db.as_ref()).await.map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn calculate_quality_score(&self, page: &CompiledPage) -> f64 {
        let mut score = 1.0_f64;

        if page.content.len() < 100 {
            score -= 0.2;
        }

        if !page.content.contains("[[") {
            score -= 0.1;
        }

        if page.content.to_lowercase().contains("我不知道")
            || page.content.to_lowercase().contains("无法确定")
        {
            score -= 0.3;
        }

        score.max(0.0).min(1.0)
    }

    pub async fn should_overwrite(&self, note: &Note) -> Result<bool, String> {
        if note.author != "llm" {
            return Ok(false);
        }

        if note.user_edited {
            return Ok(false);
        }

        Ok(true)
    }
}