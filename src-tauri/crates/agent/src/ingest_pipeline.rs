use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;

use axagent_core::entity::wiki_sources;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_core::utils::gen_id;
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IngestSourceType {
    WebArticle,
    Paper,
    Book,
    RawMarkdown,
    Docx,
    Pdf,
    Xlsx,
    Pptx,
}

impl IngestSourceType {
    pub fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "application/pdf" => Some(Self::Paper),
            "text/markdown" | "text/plain" => Some(Self::RawMarkdown),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                Some(Self::Docx)
            },
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some(Self::Xlsx),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
                Some(Self::Pptx)
            },
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSource {
    pub source_type: IngestSourceType,
    pub path: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub folder_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub source_id: String,
    pub raw_path: String,
    pub title: String,
    pub pages_generated: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created_date: Option<String>,
    pub page_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMention {
    pub name: String,
    pub entity_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptMention {
    pub name: String,
    pub description: String,
    pub related_concepts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argument {
    pub claim: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHint {
    pub target_page_title: String,
    pub relationship: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub existing_claim: String,
    pub new_claim: String,
    pub resolution_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSuggestion {
    pub title: String,
    pub page_type: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewItem {
    pub question: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAnalysis {
    pub entities: Vec<EntityMention>,
    pub concepts: Vec<ConceptMention>,
    pub arguments: Vec<Argument>,
    pub connections: Vec<ConnectionHint>,
    pub contradictions: Vec<Contradiction>,
    pub suggested_structure: Vec<PageSuggestion>,
    pub review_items: Vec<ReviewItem>,
    pub search_queries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedPage {
    pub title: String,
    pub content: String,
    pub page_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IngestCacheEntry {
    pub source_path: String,
    pub content_hash: String,
    pub source_id: String,
    pub processed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IngestCache {
    pub entries: Vec<IngestCacheEntry>,
}

pub struct IngestPipeline {
    db: Arc<DatabaseConnection>,
    llm_adapter: Option<Arc<dyn ProviderAdapter>>,
    llm_ctx: Option<ProviderRequestContext>,
    llm_model: Option<String>,
}

impl IngestPipeline {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            llm_adapter: None,
            llm_ctx: None,
            llm_model: None,
        }
    }

    pub fn with_llm(
        mut self,
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: String,
    ) -> Self {
        self.llm_adapter = Some(adapter);
        self.llm_ctx = Some(ctx);
        self.llm_model = Some(model);
        self
    }

    pub async fn ingest(
        &self,
        wiki_id: &str,
        source: IngestSource,
    ) -> Result<IngestResult, String> {
        let parsed = self.parse_source(&source).await?;
        let content_hash = self.compute_sha256(&parsed);

        if let Some(cached) = self.check_cache(wiki_id, &source, &content_hash).await? {
            return Ok(cached);
        }

        let metadata = self.extract_metadata(&parsed).await?;
        let raw_path = self.save_to_raw(wiki_id, &source, &parsed).await?;
        let source_record = self
            .save_source_record(
                wiki_id,
                &raw_path,
                &source,
                &metadata,
                &parsed,
                &content_hash,
            )
            .await?;

        let mut pages_generated = 0usize;

        if let (Some(ref adapter), Some(ref ctx), Some(ref model)) =
            (&self.llm_adapter, &self.llm_ctx, &self.llm_model)
        {
            let purpose = self.load_purpose(wiki_id).await.ok();
            let analysis = self
                .analyze_source(adapter.as_ref(), ctx, model, &parsed, purpose.as_deref())
                .await?;

            pages_generated = self
                .generate_wiki_pages(
                    adapter.as_ref(),
                    ctx,
                    model,
                    wiki_id,
                    &source_record.id,
                    &raw_path,
                    &analysis,
                )
                .await?;
        }

        self.update_cache(wiki_id, &source, &content_hash, &source_record.id)
            .await?;

        Ok(IngestResult {
            source_id: source_record.id,
            raw_path,
            title: metadata.title.unwrap_or_else(|| "Untitled".to_string()),
            pages_generated,
        })
    }

    pub async fn ingest_text(
        &self,
        wiki_id: &str,
        text: &str,
        source_type: IngestSourceType,
    ) -> Result<IngestResult, String> {
        let content_hash = self.compute_sha256(text);

        let id = gen_id();
        let extension = match source_type {
            IngestSourceType::Pdf => "pdf",
            IngestSourceType::Docx => "docx",
            IngestSourceType::Xlsx => "xlsx",
            IngestSourceType::Pptx => "pptx",
            IngestSourceType::WebArticle => "html",
            _ => "md",
        };
        let raw_path = format!("~/axagent-notes/{}/raw/{}.{}", wiki_id, id, extension);

        let path = PathBuf::from(&raw_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        fs::write(&path, text).await.map_err(|e| e.to_string())?;

        let metadata = self.extract_metadata(text).await?;
        let mime_type = match source_type {
            IngestSourceType::Pdf => "application/pdf",
            IngestSourceType::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            },
            IngestSourceType::Xlsx => {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            },
            IngestSourceType::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            },
            IngestSourceType::WebArticle => "text/html",
            _ => "text/markdown",
        };

        let am = wiki_sources::ActiveModel {
            id: Set(id.clone()),
            wiki_id: Set(wiki_id.to_string()),
            source_type: Set(format!("{:?}", source_type).to_lowercase()),
            source_path: Set(raw_path.clone()),
            title: Set(metadata
                .title
                .clone()
                .unwrap_or_else(|| "Untitled".to_string())),
            mime_type: Set(mime_type.to_string()),
            size_bytes: Set(0),
            content_hash: Set(content_hash.clone()),
            metadata_json: Set(Some(serde_json::to_value(&metadata).unwrap_or_default())),
            created_at: Set(chrono::Utc::now().timestamp()),
            updated_at: Set(chrono::Utc::now().timestamp()),
        };

        am.insert(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let mut pages_generated = 0usize;

        if let (Some(ref adapter), Some(ref ctx), Some(ref model)) =
            (&self.llm_adapter, &self.llm_ctx, &self.llm_model)
        {
            let purpose = self.load_purpose(wiki_id).await.ok();
            let analysis = self
                .analyze_source(adapter.as_ref(), ctx, model, text, purpose.as_deref())
                .await?;

            pages_generated = self
                .generate_wiki_pages(
                    adapter.as_ref(),
                    ctx,
                    model,
                    wiki_id,
                    &id,
                    &raw_path,
                    &analysis,
                )
                .await?;
        }

        Ok(IngestResult {
            source_id: id,
            raw_path,
            title: metadata.title.unwrap_or_else(|| "Untitled".to_string()),
            pages_generated,
        })
    }

    async fn analyze_source(
        &self,
        adapter: &dyn ProviderAdapter,
        ctx: &ProviderRequestContext,
        model: &str,
        content: &str,
        purpose: Option<&str>,
    ) -> Result<SourceAnalysis, String> {
        let purpose_context = purpose
            .map(|p| format!("\n\n## Wiki Purpose\n{}", p))
            .unwrap_or_default();

        let truncated = if content.len() > 16000 {
            format!("{}... [truncated]", &content[..16000])
        } else {
            content.to_string()
        };

        let prompt = format!(
            r#"You are an LLM Wiki analysis assistant. Analyze the source content below and output a structured analysis in JSON format.

{purpose_context}

## Source Content
```
{truncated}
```

## Analysis Requirements
Output a JSON object with the following fields:

1. **entities**: Array of key entities found. Each has:
   - "name": Entity name
   - "entity_type": Type (person, organization, product, technology, etc.)
   - "description": Brief description

2. **concepts**: Array of core concepts. Each has:
   - "name": Concept name
   - "description": Explanation
   - "related_concepts": Array of related concept names

3. **arguments**: Array of main arguments or viewpoints. Each has:
   - "claim": The claim
   - "evidence": Array of supporting evidence points

4. **connections**: Array of suggested connections to existing wiki pages:
   - "target_page_title": Suggested page title to link to
   - "relationship": How it relates (e.g., "is_a", "part_of", "related_to")
   - "confidence": "high", "medium", or "low"

5. **contradictions**: Array of potential contradictions or tensions:
   - "existing_claim": What existing knowledge might say
   - "new_claim": What this source says
   - "resolution_note": How to reconcile

6. **suggested_structure**: Array of suggested wiki pages to create:
   - "title": Page title
   - "page_type": One of "entity", "concept", "source-summary", "comparison"
   - "summary": Brief summary of what this page should contain

7. **review_items**: Array of items requiring human judgment:
   - "question": The question for human review
   - "context": Relevant context

8. **search_queries**: Array of search queries for deeper research

Output ONLY valid JSON inside a ```json fenced code block."#
        );

        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(
                    "You are a precise knowledge analysis engine. Output ONLY valid JSON inside a ```json fenced code block. Never output text outside code blocks."
                        .to_string(),
                ),
                tool_calls: None,
                tool_call_id: None,
            }, ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
            }],
            stream: false,
            temperature: Some(0.3),
            max_tokens: Some(8192),
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

        let response = adapter
            .chat(ctx, request)
            .await
            .map_err(|e| format!("Analysis LLM call failed: {}", e))?;

        Self::parse_analysis_json(&response.content)
    }

    fn parse_analysis_json(raw_text: &str) -> Result<SourceAnalysis, String> {
        let re = regex::Regex::new(r"```json\s*\n?([\s\S]*?)```").map_err(|e| e.to_string())?;

        if let Some(cap) = re.captures(raw_text) {
            let json_str = cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            if !json_str.is_empty() {
                let analysis: SourceAnalysis = serde_json::from_str(json_str)
                    .map_err(|e| format!("Failed to parse analysis JSON: {}", e))?;
                return Ok(analysis);
            }
        }

        let trimmed = raw_text.trim();
        serde_json::from_str(trimmed).map_err(|e| format!("Failed to parse analysis JSON: {}", e))
    }

    async fn generate_wiki_pages(
        &self,
        adapter: &dyn ProviderAdapter,
        ctx: &ProviderRequestContext,
        model: &str,
        wiki_id: &str,
        source_id: &str,
        raw_path: &str,
        analysis: &SourceAnalysis,
    ) -> Result<usize, String> {
        let analysis_json = serde_json::to_string_pretty(analysis).map_err(|e| e.to_string())?;

        let suggestions_text: Vec<String> = analysis
            .suggested_structure
            .iter()
            .enumerate()
            .map(|(i, s)| {
                format!(
                    "{}. [{}/{}] {}: {}",
                    i + 1,
                    s.page_type,
                    s.title,
                    s.title,
                    s.summary
                )
            })
            .collect();

        let prompt = format!(
            r#"You are an LLM Wiki generation engine. Based on the analysis below, generate wiki pages.

## Source Analysis
{analysis_json}

## Raw Source Path
{raw_path}

## Generation Instructions
Generate wiki pages for the following suggested structure:
{}

## Page Format
Each page must be valid JSON inside a ```json fenced code block with these fields:
- "title": Page title (concise, descriptive)
- "content": Full markdown content with YAML frontmatter. Use [[wikilinks]] for cross-references. The frontmatter must include:
  - type: page type
  - title: page title
  - author: "llm"
  - sources: [{source_id}]
  - created_at: current timestamp
- "page_type": One of "entity", "concept", "source-summary", "comparison"

## Requirements
- Every concept page must link to related concepts with [[wikilinks]]
- Source summaries must cite the original source
- No uncertain language ("I don't know", "cannot determine")
- Generate at minimum a source-summary page
- Output each page in its own ```json block"#,
            suggestions_text.join("\n"),
            source_id = source_id,
        );

        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(
                    "You are a precise wiki page generator. Output ONLY valid JSON inside ```json fenced code blocks. Each block is a separate wiki page. Never output text outside code blocks."
                        .to_string(),
                ),
                tool_calls: None,
                tool_call_id: None,
            }, ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
            }],
            stream: false,
            temperature: Some(0.4),
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

        let response = adapter
            .chat(ctx, request)
            .await
            .map_err(|e| format!("Generation LLM call failed: {}", e))?;

        let pages = self.parse_pages_from_response(&response.content, source_id)?;
        let count = pages.len();

        for page in &pages {
            self.save_generated_page(wiki_id, source_id, page).await?;
        }

        Ok(count)
    }

    fn parse_pages_from_response(
        &self,
        raw_text: &str,
        _default_source_id: &str,
    ) -> Result<Vec<GeneratedPage>, String> {
        let re = regex::Regex::new(r"```json\s*\n?([\s\S]*?)```").map_err(|e| e.to_string())?;

        let mut pages = Vec::new();

        for cap in re.captures_iter(raw_text) {
            let json_str = cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            if json_str.is_empty() {
                continue;
            }

            let clean = json_str
                .replace(['\u{201c}', '\u{201d}'], "\"")
                .replace(['\u{2018}', '\u{2019}'], "'");

            match serde_json::from_str::<GeneratedPage>(&clean) {
                Ok(page) => {
                    if !page.title.is_empty() && !page.content.is_empty() {
                        pages.push(page);
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse generated page: {}. Raw: {}",
                        e,
                        &clean[..clean.len().min(200)]
                    );
                },
            }
        }

        if pages.is_empty() {
            return Err(format!(
                "No valid pages parsed from LLM response. Response: {}",
                &raw_text[..raw_text.len().min(500)]
            ));
        }

        Ok(pages)
    }

    async fn save_generated_page(
        &self,
        wiki_id: &str,
        source_id: &str,
        page: &GeneratedPage,
    ) -> Result<(), String> {
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

        let dir = match page.page_type.as_str() {
            "entity" => "entities",
            "concept" => "concepts",
            "comparison" => "comparisons",
            _ => "notes",
        };

        let file_path = format!("notes/{}/{}.md", dir, slug);

        let wiki = axagent_core::entity::wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let note_path = std::path::Path::new(&wiki.root_path).join(&file_path);
        if let Some(parent) = note_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        fs::write(&note_path, &page.content)
            .await
            .map_err(|e| e.to_string())?;

        let input = axagent_core::repo::note::CreateNoteInput {
            vault_id: wiki_id.to_string(),
            title: page.title.clone(),
            file_path: file_path.clone(),
            content: page.content.clone(),
            author: "llm".to_string(),
            page_type: Some(page.page_type.clone()),
            source_refs: Some(vec![source_id.to_string()]),
        };

        let _ = axagent_core::repo::note::create_note(self.db.as_ref(), input)
            .await
            .map_err(|e| format!("Failed to create note: {}", e))?;

        Ok(())
    }

    fn compute_sha256(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn cache_path(wiki_id: &str) -> String {
        format!("~/axagent-notes/{}/.cache/ingest_cache.json", wiki_id)
    }

    async fn check_cache(
        &self,
        wiki_id: &str,
        source: &IngestSource,
        content_hash: &str,
    ) -> Result<Option<IngestResult>, String> {
        let path = Self::cache_path(wiki_id);
        if let Ok(data) = tokio::fs::read_to_string(&path).await {
            let cache: IngestCache =
                serde_json::from_str(&data).unwrap_or(IngestCache { entries: vec![] });
            if let Some(entry) = cache
                .entries
                .iter()
                .find(|e| e.content_hash == content_hash)
            {
                tracing::info!("Cache hit for source: {}", source.path);
                return Ok(Some(IngestResult {
                    source_id: entry.source_id.clone(),
                    raw_path: format!("~/axagent-notes/{}/raw/{}", wiki_id, entry.source_id),
                    title: "Cached".to_string(),
                    pages_generated: 0,
                }));
            }
        }
        Ok(None)
    }

    async fn update_cache(
        &self,
        wiki_id: &str,
        source: &IngestSource,
        content_hash: &str,
        source_id: &str,
    ) -> Result<(), String> {
        let path = Self::cache_path(wiki_id);
        let mut cache: IngestCache = tokio::fs::read_to_string(&path)
            .await
            .map(|d| serde_json::from_str(&d).unwrap_or(IngestCache { entries: vec![] }))
            .unwrap_or(IngestCache { entries: vec![] });

        cache.entries.retain(|e| e.source_path != source.path);
        cache.entries.push(IngestCacheEntry {
            source_path: source.path.clone(),
            content_hash: content_hash.to_string(),
            source_id: source_id.to_string(),
            processed_at: chrono::Utc::now().timestamp(),
        });

        let cache_dir = std::path::Path::new(&path).parent().unwrap().to_path_buf();
        fs::create_dir_all(&cache_dir)
            .await
            .map_err(|e| e.to_string())?;
        fs::write(
            &path,
            serde_json::to_string_pretty(&cache).map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn load_purpose(&self, wiki_id: &str) -> Result<String, String> {
        let wiki = axagent_core::entity::wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let purpose_path = std::path::Path::new(&wiki.root_path).join("purpose.md");
        if purpose_path.exists() {
            tokio::fs::read_to_string(&purpose_path)
                .await
                .map_err(|e| format!("Failed to read purpose.md: {}", e))
        } else {
            Err("purpose.md not found".to_string())
        }
    }

    async fn parse_source(&self, source: &IngestSource) -> Result<String, String> {
        match source.source_type {
            IngestSourceType::WebArticle => {
                if let Some(url) = &source.url {
                    self.fetch_web_content(url).await
                } else {
                    tokio::fs::read_to_string(&source.path)
                        .await
                        .map_err(|e| e.to_string())
                }
            },
            IngestSourceType::RawMarkdown => tokio::fs::read_to_string(&source.path)
                .await
                .map_err(|e| e.to_string()),
            IngestSourceType::Pdf => self.extract_pdf_text(&source.path).await,
            IngestSourceType::Docx => self.extract_docx_text(&source.path).await,
            _ => tokio::fs::read_to_string(&source.path)
                .await
                .map_err(|e| e.to_string()),
        }
    }

    async fn fetch_web_content(&self, url: &str) -> Result<String, String> {
        let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
        let body = response.text().await.map_err(|e| e.to_string())?;
        Ok(body)
    }

    async fn extract_pdf_text(&self, path: &str) -> Result<String, String> {
        let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        let text = pdf_extract::extract_text_from_mem(&bytes).map_err(|e| e.to_string())?;
        Ok(text)
    }

    async fn extract_docx_text(&self, path: &str) -> Result<String, String> {
        let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        let doc = docx_rs::read_docx(&bytes).map_err(|e| e.to_string())?;

        let mut text = String::new();
        for child in doc.document.children {
            if let docx_rs::DocumentChild::Paragraph(p) = child {
                for child in p.children {
                    if let docx_rs::ParagraphChild::Run(run) = child {
                        for child in run.children {
                            if let docx_rs::RunChild::Text(t) = child {
                                text.push_str(&t.text);
                            }
                        }
                    }
                }
                text.push('\n');
            }
        }

        Ok(text)
    }

    async fn extract_metadata(&self, content: &str) -> Result<SourceMetadata, String> {
        let mut title = None;
        if !content.is_empty() {
            for line in content.lines().take(20) {
                let trimmed = line.trim();
                if let Some(t) = trimmed.strip_prefix("# ") {
                    title = Some(t.to_string());
                    break;
                }
                if let Some(t) = trimmed.strip_prefix("Title: ") {
                    title = Some(t.to_string());
                    break;
                }
            }
        }

        let mut author = None;
        for line in content.lines().take(30) {
            let trimmed = line.trim();
            if let Some(a) = trimmed.strip_prefix("Author: ") {
                author = Some(a.to_string());
                break;
            }
            if let Some(a) = trimmed.strip_prefix("author: ") {
                author = Some(a.to_string());
                break;
            }
        }

        let mut created_date = None;
        for line in content.lines().take(30) {
            let trimmed = line.trim();
            if let Some(d) = trimmed.strip_prefix("Date: ") {
                created_date = Some(d.to_string());
                break;
            }
        }

        Ok(SourceMetadata {
            title,
            author,
            created_date,
            page_count: None,
        })
    }

    async fn save_to_raw(
        &self,
        wiki_id: &str,
        source: &IngestSource,
        content: &str,
    ) -> Result<String, String> {
        let extension = match source.source_type {
            IngestSourceType::Pdf => "pdf",
            IngestSourceType::Docx => "docx",
            IngestSourceType::Xlsx => "xlsx",
            IngestSourceType::Pptx => "pptx",
            IngestSourceType::WebArticle => "html",
            _ => "md",
        };

        let raw_path = format!("~/axagent-notes/{}/raw/{}.{}", wiki_id, gen_id(), extension);

        let path = PathBuf::from(&raw_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        fs::write(&path, content).await.map_err(|e| e.to_string())?;

        Ok(raw_path)
    }

    async fn save_source_record(
        &self,
        wiki_id: &str,
        raw_path: &str,
        source: &IngestSource,
        metadata: &SourceMetadata,
        _content: &str,
        content_hash: &str,
    ) -> Result<wiki_sources::Model, String> {
        let id = gen_id();
        let mime_type = match source.source_type {
            IngestSourceType::Pdf => "application/pdf",
            IngestSourceType::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            },
            IngestSourceType::Xlsx => {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            },
            IngestSourceType::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            },
            IngestSourceType::WebArticle => "text/html",
            _ => "text/markdown",
        };

        let am = wiki_sources::ActiveModel {
            id: Set(id),
            wiki_id: Set(wiki_id.to_string()),
            source_type: Set(format!("{:?}", source.source_type).to_lowercase()),
            source_path: Set(raw_path.to_string()),
            title: Set(metadata
                .title
                .clone()
                .unwrap_or_else(|| "Untitled".to_string())),
            mime_type: Set(mime_type.to_string()),
            size_bytes: Set(0),
            content_hash: Set(content_hash.to_string()),
            metadata_json: Set(Some(serde_json::to_value(metadata).unwrap_or_default())),
            created_at: Set(chrono::Utc::now().timestamp()),
            updated_at: Set(chrono::Utc::now().timestamp()),
        };

        am.insert(self.db.as_ref()).await.map_err(|e| e.to_string())
    }
}
