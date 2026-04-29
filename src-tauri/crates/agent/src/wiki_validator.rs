use std::path::Path;
use std::sync::Arc;

use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use serde::{Deserialize, Serialize};

use axagent_core::entity::{notes, wikis};
use axagent_core::repo::note::calculate_content_hash;
use axagent_core::error::{AxAgentError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub note_id: String,
    pub title: String,
    pub issue_type: ValidationIssueType,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationIssueType {
    HashMismatch,
    MissingInDatabase,
    MissingInFilesystem,
    OrphanInVectorStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub wiki_id: String,
    pub total_notes: usize,
    pub consistent_notes: usize,
    pub issues: Vec<ValidationIssue>,
    pub checked_at: i64,
}

pub struct WikiValidator {
    db: Arc<DatabaseConnection>,
}

impl WikiValidator {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn validate_wiki(&self, wiki_id: &str) -> Result<ValidationReport> {
        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AxAgentError::NotFound(format!("Wiki {} not found", wiki_id)))?;

        let mut issues = Vec::new();
        let mut consistent_count = 0;

        let db_notes = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(wiki_id))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await?;

        let wiki_pages = axagent_core::entity::wiki_pages::Entity::find()
            .filter(axagent_core::entity::wiki_pages::Column::WikiId.eq(wiki_id))
            .all(self.db.as_ref())
            .await?;

        let wiki_root = Path::new(&wiki.root_path);
        let notes_dir = wiki_root.join("notes");

        for note_model in &db_notes {
            let note_path = notes_dir.join(&note_model.file_path);

            let current_hash = if note_path.exists() {
                match tokio::fs::read_to_string(&note_path).await {
                    Ok(content) => Some(calculate_content_hash(&content)),
                    Err(e) => {
                        issues.push(ValidationIssue {
                            note_id: note_model.id.clone(),
                            title: note_model.title.clone(),
                            issue_type: ValidationIssueType::MissingInFilesystem,
                            message: format!("Cannot read file: {}", e),
                        });
                        None
                    }
                }
            } else {
                issues.push(ValidationIssue {
                    note_id: note_model.id.clone(),
                    title: note_model.title.clone(),
                    issue_type: ValidationIssueType::MissingInFilesystem,
                    message: "File does not exist on filesystem".to_string(),
                });
                None
            };

            if let Some(hash) = current_hash {
                if hash != note_model.content_hash {
                    issues.push(ValidationIssue {
                        note_id: note_model.id.clone(),
                        title: note_model.title.clone(),
                        issue_type: ValidationIssueType::HashMismatch,
                        message: format!(
                            "Hash mismatch: file={}, db={}",
                            hash, note_model.content_hash
                        ),
                    });
                } else {
                    consistent_count += 1;
                }
            }

            let wiki_page = wiki_pages.iter().find(|wp| wp.note_id == note_model.id);
            if wiki_page.is_none() {
                issues.push(ValidationIssue {
                    note_id: note_model.id.clone(),
                    title: note_model.title.clone(),
                    issue_type: ValidationIssueType::MissingInDatabase,
                    message: "Note has no wiki_page entry".to_string(),
                });
            }
        }

        let orphan_vector_items = self.find_orphan_vector_items(wiki_id, &db_notes).await?;

        issues.extend(orphan_vector_items);

        Ok(ValidationReport {
            wiki_id: wiki_id.to_string(),
            total_notes: db_notes.len(),
            consistent_notes: consistent_count,
            issues,
            checked_at: chrono::Utc::now().timestamp(),
        })
    }

    async fn find_orphan_vector_items(
        &self,
        wiki_id: &str,
        db_notes: &[notes::Model],
    ) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        let valid_note_ids: std::collections::HashSet<String> =
            db_notes.iter().map(|n| n.id.clone()).collect();

        let wiki_page_note_ids: Vec<String> = axagent_core::entity::wiki_pages::Entity::find()
            .filter(axagent_core::entity::wiki_pages::Column::WikiId.eq(wiki_id))
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(|wp| wp.note_id.clone())
            .collect();

        for note_id in wiki_page_note_ids {
            if !valid_note_ids.contains(&note_id) {
                let title = db_notes.iter()
                    .find(|n| n.id == note_id)
                    .map(|n| n.title.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                issues.push(ValidationIssue {
                    note_id: note_id.clone(),
                    title,
                    issue_type: ValidationIssueType::OrphanInVectorStore,
                    message: "Wiki page references non-existent note".to_string(),
                });
            }
        }

        Ok(issues)
    }

    pub async fn repair_note(&self, note_id: &str) -> Result<()> {
        let note = notes::Entity::find_by_id(note_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AxAgentError::NotFound(format!("Note {} not found", note_id)))?;

        let wiki = wikis::Entity::find_by_id(&note.vault_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AxAgentError::NotFound(format!("Wiki {} not found", note.vault_id)))?;

        let note_path = Path::new(&wiki.root_path).join("notes").join(&note.file_path);

        if note_path.exists() {
            let content = tokio::fs::read_to_string(&note_path).await
                .map_err(|e| AxAgentError::Internal(format!("Failed to read file: {}", e)))?;

            let new_hash = calculate_content_hash(&content);

            let mut am = note.into_active_model();
            am.content = axagent_core::sea_orm::Set(content);
            am.content_hash = axagent_core::sea_orm::Set(new_hash);
            am.updated_at = axagent_core::sea_orm::Set(chrono::Utc::now().timestamp());
            am.update(self.db.as_ref()).await?;
        }

        Ok(())
    }
}