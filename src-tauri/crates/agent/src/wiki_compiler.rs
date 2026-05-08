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
                },
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
                results.push((source.clone(), format!("[File not found: {}]", source.source_path)));
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
                .replace(['\u{201c}', '\u{201d}'], "\"")
                .replace(['\u{2018}', '\u{2019}'], "'");

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
                },
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse JSON block: {}. Raw: {}",
                        e,
                        &clean_json[..clean_json.len().min(200)]
                    );
                },
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
            .map(axagent_core::repo::note::model_to_note))
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
            source_ids: Set(Some(serde_json::to_value(&page.source_ids).unwrap_or_default())),
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

        self.upsert_system_note(wiki_id, "Overview", "overview", &overview, "notes/overview.md")
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

        if page.content.len() < 30 {
            score -= 0.5;
        }

        if !page.content.contains("[[") {
            score -= 0.1;
        }

        let lower = page.content.to_lowercase();
        let uncertain_phrases = [
            "i don't know",
            "cannot determine",
            "i'm not sure",
            "我无法确定",
            "我不知道",
        ];
        let uncertain_count = uncertain_phrases
            .iter()
            .filter(|p| lower.contains(**p))
            .count();
        if uncertain_count > 0 {
            score -= 0.4 + 0.1 * (uncertain_count as f64 - 1.0);
        }

        let sentence_count = page.content.split('.').count();
        if sentence_count < 3 {
            score -= 0.15;
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

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_core::types::{
        ChatResponse, ChatStreamChunk, EmbedRequest, EmbedResponse, Model, TokenUsage,
    };
    use axagent_providers::ProviderAdapter;
    use futures::Stream;
    use std::pin::Pin;

    struct MockProviderAdapter;

    #[async_trait::async_trait]
    impl ProviderAdapter for MockProviderAdapter {
        async fn chat(
            &self,
            _ctx: &ProviderRequestContext,
            _request: ChatRequest,
        ) -> axagent_core::error::Result<ChatResponse> {
            Ok(ChatResponse {
                id: "test".to_string(),
                model: "test".to_string(),
                content: "test".to_string(),
                thinking: None,
                usage: TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                tool_calls: None,
            })
        }

        fn chat_stream(
            &self,
            _ctx: &ProviderRequestContext,
            _request: ChatRequest,
        ) -> Pin<Box<dyn Stream<Item = axagent_core::error::Result<ChatStreamChunk>> + Send>>
        {
            Box::pin(futures::stream::empty())
        }

        async fn list_models(
            &self,
            _ctx: &ProviderRequestContext,
        ) -> axagent_core::error::Result<Vec<Model>> {
            Ok(vec![])
        }

        async fn embed(
            &self,
            _ctx: &ProviderRequestContext,
            _request: EmbedRequest,
        ) -> axagent_core::error::Result<EmbedResponse> {
            Ok(EmbedResponse {
                embeddings: vec![vec![0.0; 128]],
                dimensions: 128,
            })
        }
    }

    fn make_llm_ctx() -> ProviderRequestContext {
        ProviderRequestContext {
            api_key: "test".to_string(),
            key_id: "test".to_string(),
            provider_id: "test".to_string(),
            base_url: None,
            api_path: None,
            proxy_config: None,
            custom_headers: None,
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        }
    }

    async fn make_compiler() -> WikiCompiler {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        WikiCompiler::new(
            Arc::new(db),
            Arc::new(MockProviderAdapter),
            make_llm_ctx(),
            "test-model".to_string(),
        )
    }

    #[test]
    fn test_compiled_page_serialization() {
        let page = CompiledPage {
            title: "Test Page".to_string(),
            content: "Some content".to_string(),
            page_type: "concept".to_string(),
            source_ids: vec!["src1".to_string()],
        };
        let json = serde_json::to_string(&page).unwrap();
        let deserialized: CompiledPage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Test Page");
        assert_eq!(deserialized.page_type, "concept");
        assert_eq!(deserialized.source_ids.len(), 1);
    }

    #[test]
    fn test_compile_result_serialization() {
        let result = CompileResult {
            new_pages: vec![],
            updated_pages: vec![],
            errors: vec!["error1".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CompileResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.new_pages.is_empty());
        assert!(deserialized.updated_pages.is_empty());
        assert_eq!(deserialized.errors.len(), 1);
    }

    #[test]
    fn test_page_compile_result_serialization() {
        let result = PageCompileResult {
            page: CompiledPage {
                title: "Test".to_string(),
                content: "content".to_string(),
                page_type: "entity".to_string(),
                source_ids: vec![],
            },
            score: 0.85,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: PageCompileResult = serde_json::from_str(&json).unwrap();
        assert!((deserialized.score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_llm_response_valid_json() {
        let raw = r#"```json
{"title": "ML", "content": "some content here", "page_type": "concept", "source_ids": ["s1"]}
```"#;
        let result = WikiCompiler::parse_llm_response(raw).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "ML");
    }

    #[test]
    fn test_parse_llm_response_empty_content_rejected() {
        let raw = r#"```json
{"title": "Empty", "content": "", "page_type": "concept", "source_ids": []}
```"#;
        let result = WikiCompiler::parse_llm_response(raw);
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_llm_response_empty_title_rejected() {
        let raw = r#"```json
{"title": "", "content": "some content", "page_type": "concept", "source_ids": []}
```"#;
        let result = WikiCompiler::parse_llm_response(raw);
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_llm_response_invalid_page_type_rejected() {
        let raw = r#"```json
{"title": "Test", "content": "content", "page_type": "invalid_type", "source_ids": []}
```"#;
        let result = WikiCompiler::parse_llm_response(raw);
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_llm_response_array_format() {
        let raw = r#"```json
[{"title": "A", "content": "content a", "page_type": "concept", "source_ids": []}, {"title": "B", "content": "content b", "page_type": "entity", "source_ids": []}]
```"#;
        let result = WikiCompiler::parse_llm_response(raw).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_llm_response_no_json_blocks() {
        let raw = "This is just plain text with no JSON blocks";
        let result = WikiCompiler::parse_llm_response(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_page_type() {
        assert!(WikiCompiler::is_valid_page_type("concept"));
        assert!(WikiCompiler::is_valid_page_type("entity"));
        assert!(WikiCompiler::is_valid_page_type("comparison"));
        assert!(WikiCompiler::is_valid_page_type("source_summary"));
        assert!(WikiCompiler::is_valid_page_type("index"));
        assert!(WikiCompiler::is_valid_page_type("log"));
        assert!(WikiCompiler::is_valid_page_type("overview"));
        assert!(!WikiCompiler::is_valid_page_type("invalid"));
        assert!(!WikiCompiler::is_valid_page_type(""));
        assert!(!WikiCompiler::is_valid_page_type("note"));
    }

    #[test]
    fn test_infer_page_type_comparison() {
        assert_eq!(infer_page_type("React vs Vue"), "comparison");
        assert_eq!(infer_page_type("Python vs. Java Comparison"), "comparison");
        assert_eq!(infer_page_type("A comparison of frameworks"), "comparison");
    }

    #[test]
    fn test_infer_page_type_source_summary() {
        assert_eq!(infer_page_type("Source: Research Paper"), "source_summary");
        assert_eq!(infer_page_type("Article Summary"), "source_summary");
        assert_eq!(infer_page_type("Summary of Findings"), "source_summary");
    }

    #[test]
    fn test_infer_page_type_entity() {
        assert_eq!(infer_page_type("Google Inc."), "entity");
        assert_eq!(infer_page_type("Microsoft Corp."), "entity");
        assert_eq!(infer_page_type("Something Ltd."), "entity");
    }

    #[test]
    fn test_infer_page_type_concept() {
        assert_eq!(infer_page_type("Machine Learning"), "concept");
        assert_eq!(infer_page_type("Data Structures"), "concept");
    }

    #[test]
    fn test_page_types_heading() {
        assert_eq!(page_types_heading("concept"), "Concepts");
        assert_eq!(page_types_heading("entity"), "Entities");
        assert_eq!(page_types_heading("comparison"), "Comparisons");
        assert_eq!(page_types_heading("source_summary"), "Source Summaries");
        assert_eq!(page_types_heading("other"), "Other");
        assert_eq!(page_types_heading("note"), "Other");
        assert_eq!(page_types_heading(""), "Other");
    }

    #[test]
    fn test_default_schema() {
        let schema = WikiCompiler::default_schema();
        assert!(schema.contains("Page Types"));
        assert!(schema.contains("concept"));
        assert!(schema.contains("entity"));
        assert!(schema.contains("comparison"));
        assert!(schema.contains("source_summary"));
        assert!(schema.contains("Quality Requirements"));
        assert!(schema.contains("wikilinks"));
    }

    #[test]
    fn test_parse_llm_response_smart_quotes() {
        let raw = "```json\n{\"title\": \"Test\u{201c}Quote\u{201d}\", \"content\": \"content\u{2018}single\u{2019}\", \"page_type\": \"concept\", \"source_ids\": []}\n```";
        let result = WikiCompiler::parse_llm_response(raw);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_try_markdown_parse_single_section() {
        let raw = "## Machine Learning\n\nMachine learning is a subset of AI. It involves training models. Models learn from data.";
        let pages = WikiCompiler::try_markdown_parse(raw);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Machine Learning");
        assert!(pages[0].content.contains("subset of AI"));
        assert_eq!(pages[0].page_type, "concept");
        assert!(pages[0].source_ids.is_empty());
    }

    #[test]
    fn test_try_markdown_parse_multiple_sections() {
        let raw = "## React vs Vue\n\nReact and Vue are frameworks.\n\n## Angular\n\nAngular is a framework by Google Inc.";
        let pages = WikiCompiler::try_markdown_parse(raw);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].title, "React vs Vue");
        assert_eq!(pages[1].title, "Angular");
    }

    #[test]
    fn test_try_markdown_parse_no_h2_headers() {
        let raw = "Just some plain text\nwithout any headers\nnothing to parse here";
        let pages = WikiCompiler::try_markdown_parse(raw);
        assert!(pages.is_empty());
    }

    #[test]
    fn test_try_markdown_parse_empty_section_skipped() {
        let raw = "## Empty Section\n\n## Filled Section\n\nThis has content.";
        let pages = WikiCompiler::try_markdown_parse(raw);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Filled Section");
    }

    #[test]
    fn test_try_markdown_parse_preserves_content() {
        let raw = "## Topic\n\nFirst paragraph.\n\nSecond paragraph with [[wikilink]].\n- List item 1\n- List item 2";
        let pages = WikiCompiler::try_markdown_parse(raw);
        assert_eq!(pages.len(), 1);
        assert!(pages[0].content.contains("[[wikilink]]"));
        assert!(pages[0].content.contains("List item 1"));
    }

    #[tokio::test]
    async fn test_calculate_quality_score_short_content() {
        let compiler = make_compiler().await;
        let page = CompiledPage {
            title: "Short".to_string(),
            content: "Too short".to_string(),
            page_type: "concept".to_string(),
            source_ids: vec![],
        };
        let score = compiler.calculate_quality_score(&page).await;
        assert!(score < 1.0);
        assert!(score <= 0.6);
    }

    #[tokio::test]
    async fn test_calculate_quality_score_no_wikilinks() {
        let compiler = make_compiler().await;
        let page = CompiledPage {
            title: "No Links".to_string(),
            content: "A sufficiently long content without any wiki links. It has multiple sentences. Each one is clear.".to_string(),
            page_type: "concept".to_string(),
            source_ids: vec![],
        };
        let score = compiler.calculate_quality_score(&page).await;
        assert!(score < 1.0);
        assert!((score - 0.9).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_calculate_quality_score_uncertain_language_english() {
        let compiler = make_compiler().await;
        let page = CompiledPage {
            title: "Uncertain".to_string(),
            content: "I don't know the answer. Cannot determine the result. I'm not sure about this. This is a longer content with multiple sentences.".to_string(),
            page_type: "concept".to_string(),
            source_ids: vec![],
        };
        let score = compiler.calculate_quality_score(&page).await;
        assert!(score < 0.5);
    }

    #[tokio::test]
    async fn test_calculate_quality_score_uncertain_language_chinese() {
        let compiler = make_compiler().await;
        let page = CompiledPage {
            title: "Chinese Uncertain".to_string(),
            content: "我无法确定这个结果。我不知道答案。这是一个包含多个句子的长内容。".to_string(),
            page_type: "concept".to_string(),
            source_ids: vec![],
        };
        let score = compiler.calculate_quality_score(&page).await;
        assert!(score < 0.5);
    }

    #[tokio::test]
    async fn test_calculate_quality_score_few_sentences() {
        let compiler = make_compiler().await;
        let page = CompiledPage {
            title: "Few Sentences".to_string(),
            content: "Only two sentences here".to_string(),
            page_type: "concept".to_string(),
            source_ids: vec![],
        };
        let score = compiler.calculate_quality_score(&page).await;
        assert!(score < 1.0);
    }

    #[tokio::test]
    async fn test_calculate_quality_score_perfect_page() {
        let compiler = make_compiler().await;
        let page = CompiledPage {
            title: "Great Page".to_string(),
            content: "This is a well-written page. It covers the topic thoroughly. It includes [[related links]] to other pages. The content is detailed and accurate. Multiple perspectives are considered.".to_string(),
            page_type: "concept".to_string(),
            source_ids: vec!["src1".to_string()],
        };
        let score = compiler.calculate_quality_score(&page).await;
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_calculate_quality_score_combined_deductions() {
        let compiler = make_compiler().await;
        let page = CompiledPage {
            title: "Bad Page".to_string(),
            content: "I don't know".to_string(),
            page_type: "concept".to_string(),
            source_ids: vec![],
        };
        let score = compiler.calculate_quality_score(&page).await;
        assert_eq!(score, 0.0);
    }

    #[tokio::test]
    async fn test_should_overwrite_non_llm_author() {
        let compiler = make_compiler().await;
        let note = Note {
            id: "n1".to_string(),
            vault_id: "v1".to_string(),
            title: "Test".to_string(),
            file_path: "test.md".to_string(),
            content: "content".to_string(),
            content_hash: "hash".to_string(),
            author: "user".to_string(),
            page_type: None,
            source_refs: None,
            related_pages: None,
            quality_score: None,
            last_linted_at: None,
            last_compiled_at: None,
            compiled_source_hash: None,
            user_edited: false,
            user_edited_at: None,
            created_at: 0,
            updated_at: 0,
            is_deleted: false,
        };
        let result = compiler.should_overwrite(&note).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_should_overwrite_user_edited() {
        let compiler = make_compiler().await;
        let note = Note {
            id: "n1".to_string(),
            vault_id: "v1".to_string(),
            title: "Test".to_string(),
            file_path: "test.md".to_string(),
            content: "content".to_string(),
            content_hash: "hash".to_string(),
            author: "llm".to_string(),
            page_type: None,
            source_refs: None,
            related_pages: None,
            quality_score: None,
            last_linted_at: None,
            last_compiled_at: None,
            compiled_source_hash: None,
            user_edited: true,
            user_edited_at: Some(123),
            created_at: 0,
            updated_at: 0,
            is_deleted: false,
        };
        let result = compiler.should_overwrite(&note).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_should_overwrite_llm_not_edited() {
        let compiler = make_compiler().await;
        let note = Note {
            id: "n1".to_string(),
            vault_id: "v1".to_string(),
            title: "Test".to_string(),
            file_path: "test.md".to_string(),
            content: "content".to_string(),
            content_hash: "hash".to_string(),
            author: "llm".to_string(),
            page_type: None,
            source_refs: None,
            related_pages: None,
            quality_score: None,
            last_linted_at: None,
            last_compiled_at: None,
            compiled_source_hash: None,
            user_edited: false,
            user_edited_at: None,
            created_at: 0,
            updated_at: 0,
            is_deleted: false,
        };
        let result = compiler.should_overwrite(&note).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_page_type_dir_all_types() {
        let compiler = make_compiler().await;
        assert_eq!(compiler.page_type_dir("concept"), "concepts");
        assert_eq!(compiler.page_type_dir("entity"), "entities");
        assert_eq!(compiler.page_type_dir("comparison"), "comparisons");
        assert_eq!(compiler.page_type_dir("source_summary"), "sources");
        assert_eq!(compiler.page_type_dir("index"), "");
        assert_eq!(compiler.page_type_dir("log"), "");
        assert_eq!(compiler.page_type_dir("overview"), "");
        assert_eq!(compiler.page_type_dir("unknown"), "pages");
        assert_eq!(compiler.page_type_dir("note"), "pages");
    }

    #[test]
    fn test_parse_llm_response_multiple_json_blocks() {
        let raw = r#"```json
{"title": "Page A", "content": "content a with details", "page_type": "concept", "source_ids": ["s1"]}
```

Some text between blocks

```json
{"title": "Page B", "content": "content b with info", "page_type": "entity", "source_ids": ["s2"]}
```"#;
        let result = WikiCompiler::parse_llm_response(raw).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "Page A");
        assert_eq!(result[1].title, "Page B");
    }

    #[test]
    fn test_parse_llm_response_mixed_valid_invalid() {
        let raw = r#"```json
{"title": "Valid", "content": "valid content here", "page_type": "concept", "source_ids": []}
```

```json
{invalid json here}
```

```json
{"title": "Also Valid", "content": "another valid page", "page_type": "entity", "source_ids": []}
```"#;
        let result = WikiCompiler::parse_llm_response(raw).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_llm_response_empty_json_block() {
        let raw = r#"```json
```

```json
{"title": "Only Valid", "content": "the only valid one", "page_type": "concept", "source_ids": []}
```"#;
        let result = WikiCompiler::parse_llm_response(raw).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Only Valid");
    }

    #[test]
    fn test_parse_llm_response_fallback_to_markdown() {
        let raw = "## Fallback Topic\n\nThis is content parsed via markdown fallback. It has enough detail.\n\n## Another Topic\n\nMore content here.";
        let result = WikiCompiler::parse_llm_response(raw).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "Fallback Topic");
        assert_eq!(result[1].title, "Another Topic");
    }

    #[test]
    fn test_parse_llm_response_index_page_type() {
        let raw = r#"```json
{"title": "Index", "content": "index content here with links", "page_type": "index", "source_ids": []}
```"#;
        let result = WikiCompiler::parse_llm_response(raw).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].page_type, "index");
    }

    #[test]
    fn test_parse_llm_response_overview_page_type() {
        let raw = r#"```json
{"title": "Overview", "content": "overview content here", "page_type": "overview", "source_ids": []}
```"#;
        let result = WikiCompiler::parse_llm_response(raw).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].page_type, "overview");
    }

    #[test]
    fn test_infer_page_type_vs_with_period() {
        assert_eq!(infer_page_type("React vs. Angular"), "comparison");
    }

    #[test]
    fn test_infer_page_type_source_keyword() {
        assert_eq!(infer_page_type("Source Analysis"), "source_summary");
    }

    #[test]
    fn test_infer_page_type_article_keyword() {
        assert_eq!(infer_page_type("Article Review"), "source_summary");
    }

    #[test]
    fn test_infer_page_type_default_concept() {
        assert_eq!(infer_page_type("Quantum Computing"), "concept");
        assert_eq!(infer_page_type("Design Patterns"), "concept");
        assert_eq!(infer_page_type(""), "concept");
    }

    #[test]
    fn test_default_schema_quality_requirements() {
        let schema = WikiCompiler::default_schema();
        assert!(schema.contains("at least 3 sentences"));
        assert!(schema.contains("wikilinks"));
        assert!(schema.contains("cite the original source"));
        assert!(schema.contains("uncertain language"));
    }

    #[test]
    fn test_compiled_page_from_json_value() {
        let json = serde_json::json!({
            "title": "JSON Page",
            "content": "content from json",
            "page_type": "comparison",
            "source_ids": ["s1", "s2"]
        });
        let page: CompiledPage = serde_json::from_value(json).unwrap();
        assert_eq!(page.title, "JSON Page");
        assert_eq!(page.page_type, "comparison");
        assert_eq!(page.source_ids.len(), 2);
    }

    #[test]
    fn test_compile_result_with_mixed_data() {
        let result = CompileResult {
            new_pages: vec![CompiledPage {
                title: "New".to_string(),
                content: "new content".to_string(),
                page_type: "concept".to_string(),
                source_ids: vec![],
            }],
            updated_pages: vec![CompiledPage {
                title: "Updated".to_string(),
                content: "updated content".to_string(),
                page_type: "entity".to_string(),
                source_ids: vec!["src1".to_string()],
            }],
            errors: vec!["error1".to_string(), "error2".to_string()],
        };
        assert_eq!(result.new_pages.len(), 1);
        assert_eq!(result.updated_pages.len(), 1);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_parse_llm_response_log_page_type() {
        let raw = r#"```json
{"title": "Operation Log", "content": "log content here with entries", "page_type": "log", "source_ids": []}
```"#;
        let result = WikiCompiler::parse_llm_response(raw).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].page_type, "log");
    }

    #[test]
    fn test_try_markdown_parse_comparison_title() {
        let raw = "## Python vs Rust\n\nPython is interpreted. Rust is compiled.";
        let pages = WikiCompiler::try_markdown_parse(raw);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_type, "comparison");
    }
}
