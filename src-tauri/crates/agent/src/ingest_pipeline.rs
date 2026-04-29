use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::fs;

use axagent_core::entity::wiki_sources;
use axagent_core::utils::gen_id;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

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
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => Some(Self::Docx),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some(Self::Xlsx),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => Some(Self::Pptx),
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub source_id: String,
    pub raw_path: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created_date: Option<String>,
    pub page_count: Option<i32>,
}

pub struct IngestPipeline {
    db: Arc<DatabaseConnection>,
}

impl IngestPipeline {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn ingest(
        &self,
        wiki_id: &str,
        source: IngestSource,
    ) -> Result<IngestResult, String> {
        let parsed = self.parse_source(&source).await?;
        let metadata = self.extract_metadata(&parsed).await?;
        let raw_path = self.save_to_raw(wiki_id, &source, &parsed).await?;
        let source_record = self.save_source_record(wiki_id, &raw_path, &source, &metadata, &parsed).await?;

        Ok(IngestResult {
            source_id: source_record.id,
            raw_path,
            title: metadata.title.unwrap_or_else(|| "Untitled".to_string()),
        })
    }

    async fn parse_source(&self, source: &IngestSource) -> Result<String, String> {
        match source.source_type {
            IngestSourceType::WebArticle => {
                if let Some(url) = &source.url {
                    self.fetch_web_content(url).await
                } else {
                    tokio::fs::read_to_string(&source.path).await.map_err(|e| e.to_string())
                }
            }
            IngestSourceType::RawMarkdown => {
                tokio::fs::read_to_string(&source.path).await.map_err(|e| e.to_string())
            }
            IngestSourceType::Pdf => {
                self.extract_pdf_text(&source.path).await
            }
            IngestSourceType::Docx => {
                self.extract_docx_text(&source.path).await
            }
            _ => {
                tokio::fs::read_to_string(&source.path).await.map_err(|e| e.to_string())
            }
        }
    }

    async fn fetch_web_content(&self, url: &str) -> Result<String, String> {
        let response = reqwest::get(url)
            .await
            .map_err(|e| e.to_string())?;
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

        let raw_path = format!(
            "~/axagent-notes/{}/raw/{}.{}",
            wiki_id,
            gen_id(),
            extension
        );

        let path = PathBuf::from(&raw_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
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
        content: &str,
    ) -> Result<wiki_sources::Model, String> {
        let id = gen_id();
        let mime_type = match source.source_type {
            IngestSourceType::Pdf => "application/pdf",
            IngestSourceType::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            IngestSourceType::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            IngestSourceType::Pptx => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            IngestSourceType::WebArticle => "text/html",
            _ => "text/markdown",
        };

        let content_hash = format!("{:x}", md5::compute(content.as_bytes()));

        let am = wiki_sources::ActiveModel {
            id: Set(id),
            wiki_id: Set(wiki_id.to_string()),
            source_type: Set(format!("{:?}", source.source_type).to_lowercase()),
            source_path: Set(raw_path.to_string()),
            title: Set(metadata.title.clone().unwrap_or_else(|| "Untitled".to_string())),
            mime_type: Set(mime_type.to_string()),
            size_bytes: Set(0),
            content_hash: Set(content_hash),
            metadata_json: Set(Some(serde_json::to_value(metadata).unwrap_or_default())),
            created_at: Set(chrono::Utc::now().timestamp()),
            updated_at: Set(chrono::Utc::now().timestamp()),
        };

        am.insert(self.db.as_ref()).await.map_err(|e| e.to_string())
    }
}