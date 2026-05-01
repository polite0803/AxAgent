use std::sync::Arc;

use regex::Regex;
use serde::{Deserialize, Serialize};

use axagent_core::entity::{notes, wiki_operations, wiki_pages, wiki_sources, wikis};
use axagent_core::repo::note::{calculate_content_hash, CreateNoteInput, Note, UpdateNoteInput};
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_core::utils::gen_id;
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};

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
    llm_adapter: Arc<dyn ProviderAdapter>,
    llm_ctx: ProviderRequestContext,
    llm_model: String,
    #[allow(dead_code)]
    quality_threshold: f64,
}

impl WikiCompiler {
    pub fn new(
        db: Arc<DatabaseConnection>,
        llm_adapter: Arc<dyn ProviderAdapter>,
        llm_ctx: ProviderRequestContext,
        llm_model: String,
    ) -> Self {
        Self {
            db,
            llm_adapter,
            llm_ctx,
            llm_model,
            quality_threshold: 0.5,
        }
    }

    pub async fn compile(
        &self,
        wiki_id: &str,
        source_ids: Vec<String>,
    ) -> Result<CompileResult, String> {
        let schema = self.read_schema(wiki_id).await?;
        let sources = self.load_sources(wiki_id, &source_ids).await?;

        if sources.is_empty() {
            return Err("No valid sources to compile".to_string());
        }

        let source_contents = self.read_source_contents(&sources).await?;
        let pages = self.llm_compile(&schema, &source_contents).await?;

        let mut result = CompileResult {
            new_pages: Vec::new(),
            updated_pages: Vec::new(),
            errors: Vec::new(),
        };

        let compiled_source_ids: std::collections::HashSet<String> =
            sources.iter().map(|s| s.id.clone()).collect();

        for page in &pages {
            let mut page_with_sources = page.clone();
            let mut merged_ids = compiled_source_ids.clone();
            for sid in &page.source_ids {
                merged_ids.insert(sid.clone());
            }
            page_with_sources.source_ids = merged_ids.into_iter().collect();

            let page_clone = page_with_sources.clone();
            match self.save_page(wiki_id, &page_with_sources).await {
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

        let _ = self.update_index(wiki_id).await;
        let _ = self.update_overview(wiki_id).await;
        let _ = self.update_log(wiki_id, "compile", &result).await;

        Ok(result)
    }

    async fn read_schema(&self, wiki_id: &str) -> Result<String, String> {
        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let schema_path = std::path::Path::new(&wiki.root_path).join("SCHEMA.md");
        if schema_path.exists() {
            tokio::fs::read_to_string(&schema_path)
                .await
                .map_err(|e| format!("Failed to read SCHEMA.md: {}", e))
        } else {
            Ok(Self::default_schema())
        }
    }

    fn default_schema() -> String {
        r#"# LLM Wiki Schema

## Page Types
- `concept`: Abstract concept or idea. Include definition, properties, and wiki link references to related concepts.
- `entity`: Concrete entity (person, product, company, etc.). Include description, attributes, and related entities.
- `comparison`: Side-by-side comparison of two or more items.
- `source_summary`: Summary of a source material with key points.

## Quality Requirements
- Each page must have at least 3 sentences
- Concept/entity pages must reference related pages with [[wikilinks]]
- Source summaries must cite the original source
- No uncertain language ("I don't know", "cannot determine")
"#.to_string()
    }

    async fn load_sources(
        &self,
        wiki_id: &str,
        source_ids: &[String],
    ) -> Result<Vec<wiki_sources::Model>, String> {
        let sources = wiki_sources::Entity::find()
            .filter(wiki_sources::Column::WikiId.eq(wiki_id))
            .filter(
                wiki_sources::Column::Id
                    .is_in(source_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
            )
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(sources)
    }

    async fn read_source_contents(
        &self,
        sources: &[wiki_sources::Model],
    ) -> Result<Vec<(wiki_sources::Model, String)>, String> {
        let mut results = Vec::new();
        for source in sources {
            let path = std::path::Path::new(&source.source_path);
            if path.exists() {
                let content = tokio::fs::read_to_string(path)
                    .await
                    .unwrap_or_else(|_| format!("[Content not readable: {}]", source.source_path));
                results.push((source.clone(), content));
            } else {
                let wiki = wikis::Entity::find_by_id(&source.wiki_id)
                    .one(self.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())?;
                if let Some(w) = wiki {
                    let alt_path = std::path::Path::new(&w.root_path).join("raw").join(
                        std::path::Path::new(&source.source_path)
                            .file_name()
                            .unwrap_or_default(),
                    );
                    if alt_path.exists() {
                        let content = tokio::fs::read_to_string(&alt_path)
                            .await
                            .unwrap_or_else(|_| format!("[Content not readable: {:?}]", alt_path));
                        results.push((source.clone(), content));
                        continue;
                    }
                }
                results.push((
                    source.clone(),
                    format!("[File not found: {}]", source.source_path),
                ));
            }
        }
        Ok(results)
    }

    async fn llm_compile(
        &self,
        schema: &str,
        source_contents: &[(wiki_sources::Model, String)],
    ) -> Result<Vec<CompiledPage>, String> {
        let sources_text: Vec<String> = source_contents
            .iter()
            .enumerate()
            .map(|(i, (source, content))| {
                format!(
                    "## Source {}: {}\nID: {}\nContent:\n{}\n",
                    i + 1,
                    source.title,
                    source.id,
                    if content.len() > 8000 {
                        format!("{}... [truncated]", &content[..8000])
                    } else {
                        content.clone()
                    }
                )
            })
            .collect();

        let prompt = format!(
            "You are a knowledge engineer. Based on the SCHEMA and source materials below, \
            compile structured wiki pages.\n\n\
            SCHEMA:\n{}\n\n\
            SOURCE MATERIALS:\n{}\n\n\
            OUTPUT INSTRUCTIONS:\n\
            Output each page as a JSON object inside a ```json fenced code block. \
            Include multiple ```json blocks for multiple pages.\n\
            Each page object must have these fields:\n\
            - \"title\": The page title (concise, descriptive)\n\
            - \"content\": Full markdown content with frontmatter. Use [[wikilinks]] to reference \
            other concepts/entities. Include #tags in the frontmatter.\n\
            - \"page_type\": One of: \"concept\", \"entity\", \"comparison\", \"source_summary\"\n\
            - \"source_ids\": Array of source IDs that this page was derived from\n\n\
            Example output:\n\
            ```json\n\
            {{\"title\": \"Machine Learning\", \"content\": \"---\\ntitle: Machine Learning\\nauthor: llm\\npage_type: concept\\ntags: [AI, ML]\\n---\\n\\n\
            # Machine Learning\\n\\nDetailed content here...\\n\", \"page_type\": \"concept\", \"source_ids\": [\"src_1\"]}}\n\
            ```\n\n\
            Generate pages for:\n\
            1. Source summaries for each source\n\
            2. All distinct concepts found\n\
            3. All distinct entities found\n\
            4. Comparisons where applicable\n\
            5. Ensure each concept page links to related concepts with [[wikilinks]]",
            schema,
            sources_text.join("\n\n")
        );

        let request = ChatRequest {
            model: self.llm_model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(
                        "You are a precise knowledge engineer. Output ONLY valid JSON inside ```json fenced code blocks. \
                        Each block is a separate wiki page. Never output text outside code blocks."
                            .to_string(),
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(prompt),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            stream: false,
            temperature: Some(0.3),
            max_tokens: Some(16384),
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        let response = self
            .llm_adapter
            .chat(&self.llm_ctx, request)
            .await
            .map_err(|e| format!("LLM call failed: {}", e))?;

        let raw_text = response.content;
        Self::parse_llm_response(&raw_text)
    }

    fn parse_llm_response(raw_text: &str) -> Result<Vec<CompiledPage>, String> {
        let json_re = Regex::new(r"```json\s*\n?([\s\S]*?)```").map_err(|e| e.to_string())?;
        let mut pages = Vec::new();

        for cap in json_re.captures_iter(raw_text) {
            let json_str = cap
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if json_str.is_empty() {
                continue;
            }

            let clean_json = json_str
                .replace('\u{201c}', "\"")
                .replace('\u{201d}', "\"")
                .replace('\u{2018}', "'")
                .replace('\u{2019}', "'");

            match serde_json::from_str::<serde_json::Value>(&clean_json) {
                Ok(value) => {
                    if value.is_object() {
                        if let Ok(page) = serde_json::from_value::<CompiledPage>(value.clone()) {
                            if !page.content.is_empty()
                                && !page.title.is_empty()
                                && Self::is_valid_page_type(&page.page_type)
                            {
                                pages.push(page);
                            }
                        }
                    } else if value.is_array() {
                        if let Ok(arr) = serde_json::from_value::<Vec<CompiledPage>>(value) {
                            for page in arr {
                                if !page.content.is_empty()
                                    && !page.title.is_empty()
                                    && Self::is_valid_page_type(&page.page_type)
                                {
                                    pages.push(page);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse JSON block: {}. Raw: {}",
                        e,
                        &clean_json[..clean_json.len().min(200)]
                    );
                }
            }
        }

        if pages.is_empty() {
            let fallback = Self::try_markdown_parse(raw_text);
            if fallback.is_empty() {
                return Err(format!(
                    "No valid pages could be parsed from LLM response. Response: {}",
                    &raw_text[..raw_text.len().min(500)]
                ));
            }
            return Ok(fallback);
        }

        Ok(pages)
    }

    fn try_markdown_parse(raw_text: &str) -> Vec<CompiledPage> {
        let mut pages = Vec::new();
        let h2_re = Regex::new(r"^## (.+)$").ok();
        let mut current_title: Option<String> = None;
        let mut current_content = Vec::new();
        let mut titles_seen = Vec::new();

        for line in raw_text.lines() {
            if let Some(ref re) = h2_re {
                if let Some(cap) = re.captures(line) {
                    if let Some(title) = current_title.take() {
                        let content = current_content.join("\n");
                        if !content.trim().is_empty() {
                            pages.push(CompiledPage {
                                title: title.clone(),
                                content,
                                page_type: infer_page_type(&title),
                                source_ids: Vec::new(),
                            });
                        }
                        current_content = Vec::new();
                    }
                    current_title = Some(cap.get(1).unwrap().as_str().to_string());
                    titles_seen.push(current_title.clone().unwrap());
                    continue;
                }
            }
            if current_title.is_some() {
                current_content.push(line.to_string());
            }
        }

        if let Some(title) = current_title {
            let content = current_content.join("\n");
            if !content.trim().is_empty() {
                pages.push(CompiledPage {
                    title,
                    content,
                    page_type: infer_page_type(titles_seen.last().unwrap_or(&String::new())),
                    source_ids: Vec::new(),
                });
            }
        }

        pages
    }

    fn is_valid_page_type(pt: &str) -> bool {
        matches!(pt, "concept" | "entity" | "comparison" | "source_summary")
            || pt == "index"
            || pt == "log"
            || pt == "overview"
    }

    async fn save_page(&self, wiki_id: &str, page: &CompiledPage) -> Result<(Note, bool), String> {
        let slug = page
            .title
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else if c == ' ' {
                    '-'
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .to_lowercase();

        let dir = self.page_type_dir(&page.page_type);
        let file_path = format!("notes/{}/{}.md", dir, slug);

        let existing_note = self
            .find_existing_note_by_title(wiki_id, &page.title)
            .await?;

        if let Some(ref note) = existing_note {
            if !self.should_overwrite(note).await? {
                return Ok((note.clone(), false));
            }

            let content_hash = calculate_content_hash(&page.content);
            if note.content_hash == content_hash {
                return Ok((note.clone(), false));
            }

            let input = UpdateNoteInput {
                title: Some(page.title.clone()),
                content: Some(page.content.clone()),
                page_type: Some(page.page_type.clone()),
                related_pages: None,
            };

            let updated_note =
                axagent_core::repo::note::update_note(self.db.as_ref(), &note.id, input)
                    .await
                    .map_err(|e| e.to_string())?;

            self.update_wiki_page(&updated_note, page).await?;

            let wiki = wikis::Entity::find_by_id(wiki_id)
                .one(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

            let note_path = std::path::Path::new(&wiki.root_path)
                .join("notes")
                .join(&file_path);
            if let Some(parent) = note_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            let _ = tokio::fs::write(&note_path, &page.content).await;

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

        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let note_path = std::path::Path::new(&wiki.root_path)
            .join("notes")
            .join(&file_path);
        if let Some(parent) = note_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&note_path, &page.content).await;

        let _ = axagent_core::repo::wiki::increment_note_count(self.db.as_ref(), wiki_id).await;

        Ok((note, false))
    }

    fn page_type_dir(&self, page_type: &str) -> &str {
        match page_type {
            "concept" => "concepts",
            "entity" => "entities",
            "comparison" => "comparisons",
            "source_summary" => "sources",
            "index" => "",
            "log" => "",
            "overview" => "",
            _ => "pages",
        }
    }

    async fn find_existing_note_by_title(
        &self,
        wiki_id: &str,
        title: &str,
    ) -> Result<Option<Note>, String> {
        let db_notes = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(wiki_id))
            .filter(notes::Column::Title.eq(title))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(db_notes
            .into_iter()
            .next()
            .map(|n| axagent_core::repo::note::model_to_note(n)))
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
            am.update(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;
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
            am.compiled_source_hash = Set(Some(calculate_content_hash(&page.content)));
            am.update(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    async fn create_wiki_page(
        &self,
        wiki_id: &str,
        note: &Note,
        page: &CompiledPage,
    ) -> Result<(), String> {
        let wiki_page_model = wiki_pages::ActiveModel {
            id: Set(gen_id()),
            wiki_id: Set(wiki_id.to_string()),
            note_id: Set(note.id.clone()),
            page_type: Set(page.page_type.clone()),
            title: Set(page.title.clone()),
            source_ids: Set(Some(
                serde_json::to_value(&page.source_ids).unwrap_or_default(),
            )),
            quality_score: Set(None),
            last_linted_at: Set(None),
            last_compiled_at: Set(chrono::Utc::now().timestamp()),
            compiled_source_hash: Set(Some(calculate_content_hash(&page.content))),
            created_at: Set(chrono::Utc::now().timestamp()),
            updated_at: Set(chrono::Utc::now().timestamp()),
        };

        wiki_page_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn update_index(&self, wiki_id: &str) -> Result<(), String> {
        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let db_notes = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(wiki_id))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let mut index = String::from("# Wiki Index\n\n");
        index.push_str(&format!(
            "Last updated: {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
        ));

        let mut by_type: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for note in &db_notes {
            let note_ref = axagent_core::repo::note::model_to_note(note.clone());
            let pt = note_ref.page_type.unwrap_or_else(|| "note".to_string());
            by_type.entry(pt).or_default().push(note.title.clone());
        }

        for page_type in &["concept", "entity", "comparison", "source_summary"] {
            if let Some(titles) = by_type.get(*page_type) {
                index.push_str(&format!("## {}\n\n", page_types_heading(page_type)));
                for title in titles {
                    index.push_str(&format!("- [[{}]]\n", title));
                }
                index.push('\n');
            }
        }

        if let Some(other) = by_type.get("note") {
            index.push_str("## Notes\n\n");
            for title in other {
                index.push_str(&format!("- [[{}]]\n", title));
            }
        }

        let index_path = std::path::Path::new(&wiki.root_path)
            .join("notes")
            .join("index.md");
        if let Some(parent) = index_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        tokio::fs::write(&index_path, &index)
            .await
            .map_err(|e| e.to_string())?;

        self.upsert_system_note(wiki_id, "Index", "index", &index, "notes/index.md")
            .await
    }

    async fn update_log(
        &self,
        wiki_id: &str,
        operation: &str,
        result: &CompileResult,
    ) -> Result<(), String> {
        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let log_path = std::path::Path::new(&wiki.root_path)
            .join("notes")
            .join("log.md");

        let mut existing = String::new();
        if log_path.exists() {
            existing = tokio::fs::read_to_string(&log_path)
                .await
                .unwrap_or_default();
        }

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
        let entry = format!(
            "## {} - {}\n- Operation: {}\n- New pages: {}\n- Updated pages: {}\n- Errors: {}\n\n",
            timestamp,
            operation,
            operation,
            result.new_pages.len(),
            result.updated_pages.len(),
            result.errors.len()
        );

        let new_log = if existing.is_empty() {
            format!("# Operation Log\n\n{}", entry)
        } else if let Some(pos) = existing.find("\n") {
            let (header, rest) = existing.split_at(pos);
            format!("{}\n{}{}", header, entry, rest)
        } else {
            format!("{}\n{}", existing, entry)
        };

        tokio::fs::write(&log_path, &new_log)
            .await
            .map_err(|e| e.to_string())?;

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

        log_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        self.upsert_system_note(wiki_id, "Operation Log", "log", &new_log, "notes/log.md")
            .await
    }

    async fn update_overview(&self, wiki_id: &str) -> Result<(), String> {
        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let db_notes = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(wiki_id))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let source_count = wiki_sources::Entity::find()
            .filter(wiki_sources::Column::WikiId.eq(wiki_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .len();

        let mut overview = format!(
            "# Wiki Overview\n\n\
            - **Name**: {}\n\
            - **Schema Version**: {}\n\
            - **Total Pages**: {}\n\
            - **Total Sources**: {}\n\
            - **Last Updated**: {}\n\n",
            wiki.name,
            wiki.schema_version,
            db_notes.len(),
            source_count,
            chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
        );

        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for note in &db_notes {
            let pt = note.page_type.clone().unwrap_or_else(|| "note".to_string());
            *counts.entry(pt).or_insert(0) += 1;
        }

        overview.push_str("## Page Type Distribution\n\n");
        for (pt, count) in &counts {
            overview.push_str(&format!("- {}: {}\n", pt, count));
        }

        overview.push_str("\n## Recent Activity\n\nSee [[Operation Log]] for details.\n");

        let overview_path = std::path::Path::new(&wiki.root_path)
            .join("notes")
            .join("overview.md");
        if let Some(parent) = overview_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        tokio::fs::write(&overview_path, &overview)
            .await
            .map_err(|e| e.to_string())?;

        self.upsert_system_note(
            wiki_id,
            "Overview",
            "overview",
            &overview,
            "notes/overview.md",
        )
        .await
    }

    async fn upsert_system_note(
        &self,
        wiki_id: &str,
        title: &str,
        page_type: &str,
        content: &str,
        file_path: &str,
    ) -> Result<(), String> {
        let existing = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(wiki_id))
            .filter(notes::Column::Title.eq(title))
            .filter(notes::Column::IsDeleted.eq(0))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let content_hash = calculate_content_hash(content);

        if let Some(note) = existing {
            if note.content_hash == content_hash {
                return Ok(());
            }
            let mut am = note.into_active_model();
            am.content = Set(content.to_string());
            am.content_hash = Set(content_hash);
            am.updated_at = Set(chrono::Utc::now().timestamp());
            am.update(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;
        } else {
            let input = CreateNoteInput {
                vault_id: wiki_id.to_string(),
                title: title.to_string(),
                file_path: file_path.to_string(),
                content: content.to_string(),
                author: "llm".to_string(),
                page_type: Some(page_type.to_string()),
                source_refs: None,
            };
            let _ = axagent_core::repo::note::create_note(self.db.as_ref(), input)
                .await
                .map_err(|e| e.to_string())?;
        }

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

        let lower = page.content.to_lowercase();
        if lower.contains("i don't know")
            || lower.contains("cannot determine")
            || lower.contains("i'm not sure")
            || lower.contains("我无法确定")
            || lower.contains("我不知道")
        {
            score -= 0.3;
        }

        let sentence_count = page.content.split('.').count();
        if sentence_count < 3 {
            score -= 0.1;
        }

        score.clamp(0.0, 1.0)
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

fn page_types_heading(pt: &str) -> &str {
    match pt {
        "concept" => "Concepts",
        "entity" => "Entities",
        "comparison" => "Comparisons",
        "source_summary" => "Source Summaries",
        _ => "Other",
    }
}

fn infer_page_type(title: &str) -> String {
    let lower = title.to_lowercase();
    if lower.contains(" vs ") || lower.contains(" vs. ") || lower.contains("comparison") {
        "comparison".to_string()
    } else if lower.contains("source") || lower.contains("summary") || lower.contains("article") {
        "source_summary".to_string()
    } else if lower.contains("inc.") || lower.contains("corp.") || lower.contains("ltd.") {
        "entity".to_string()
    } else {
        "concept".to_string()
    }
}
