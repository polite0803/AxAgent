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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_issue_type_equality() {
        assert_eq!(ValidationIssueType::HashMismatch, ValidationIssueType::HashMismatch);
        assert_eq!(ValidationIssueType::MissingInDatabase, ValidationIssueType::MissingInDatabase);
        assert_eq!(ValidationIssueType::MissingInFilesystem, ValidationIssueType::MissingInFilesystem);
        assert_eq!(ValidationIssueType::OrphanInVectorStore, ValidationIssueType::OrphanInVectorStore);
        assert_ne!(ValidationIssueType::HashMismatch, ValidationIssueType::MissingInDatabase);
    }

    #[test]
    fn test_validation_issue_serialization() {
        let issue = ValidationIssue {
            note_id: "n1".to_string(),
            title: "Test Note".to_string(),
            issue_type: ValidationIssueType::HashMismatch,
            message: "Hash mismatch detected".to_string(),
        };
        let json = serde_json::to_string(&issue).unwrap();
        let deserialized: ValidationIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.note_id, "n1");
        assert_eq!(deserialized.issue_type, ValidationIssueType::HashMismatch);
    }

    #[test]
    fn test_validation_report_serialization() {
        let report = ValidationReport {
            wiki_id: "wiki-1".to_string(),
            total_notes: 10,
            consistent_notes: 8,
            issues: vec![ValidationIssue {
                note_id: "n2".to_string(),
                title: "Broken".to_string(),
                issue_type: ValidationIssueType::MissingInFilesystem,
                message: "File missing".to_string(),
            }],
            checked_at: 1234567890,
        };
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: ValidationReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.wiki_id, "wiki-1");
        assert_eq!(deserialized.total_notes, 10);
        assert_eq!(deserialized.consistent_notes, 8);
        assert_eq!(deserialized.issues.len(), 1);
    }

    #[test]
    fn test_validation_issue_type_all_variants() {
        let variants = vec![
            ValidationIssueType::HashMismatch,
            ValidationIssueType::MissingInDatabase,
            ValidationIssueType::MissingInFilesystem,
            ValidationIssueType::OrphanInVectorStore,
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_validation_report_empty_issues() {
        let report = ValidationReport {
            wiki_id: "wiki-2".to_string(),
            total_notes: 5,
            consistent_notes: 5,
            issues: vec![],
            checked_at: 0,
        };
        assert!(report.issues.is_empty());
        assert_eq!(report.total_notes, report.consistent_notes);
    }

    #[test]
    fn test_validation_issue_hash_mismatch_message() {
        let issue = ValidationIssue {
            note_id: "n1".to_string(),
            title: "Test".to_string(),
            issue_type: ValidationIssueType::HashMismatch,
            message: "Hash mismatch: file=abc, db=def".to_string(),
        };
        assert!(issue.message.contains("Hash mismatch"));
    }

    #[test]
    fn test_validation_issue_missing_in_database() {
        let issue = ValidationIssue {
            note_id: "n1".to_string(),
            title: "Orphan".to_string(),
            issue_type: ValidationIssueType::MissingInDatabase,
            message: "Note has no wiki_page entry".to_string(),
        };
        assert_eq!(issue.issue_type, ValidationIssueType::MissingInDatabase);
    }

    #[test]
    fn test_validation_report_checked_at() {
        let report = ValidationReport {
            wiki_id: "w1".to_string(),
            total_notes: 1,
            consistent_notes: 1,
            issues: vec![],
            checked_at: chrono::Utc::now().timestamp(),
        };
        assert!(report.checked_at > 0);
    }

    #[test]
    fn test_validation_issue_orphan_in_vector_store() {
        let issue = ValidationIssue {
            note_id: "n1".to_string(),
            title: "Unknown".to_string(),
            issue_type: ValidationIssueType::OrphanInVectorStore,
            message: "Wiki page references non-existent note".to_string(),
        };
        assert_eq!(issue.issue_type, ValidationIssueType::OrphanInVectorStore);
    }

    #[test]
    fn test_validation_issue_type_serialize_deserialize() {
        let types = vec![
            ValidationIssueType::HashMismatch,
            ValidationIssueType::MissingInDatabase,
            ValidationIssueType::MissingInFilesystem,
            ValidationIssueType::OrphanInVectorStore,
        ];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let deserialized: ValidationIssueType = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, deserialized);
        }
    }

    #[test]
    fn test_validation_issue_all_types() {
        let issue1 = ValidationIssue {
            note_id: "n1".to_string(),
            title: "Note 1".to_string(),
            issue_type: ValidationIssueType::HashMismatch,
            message: "Hash mismatch: file=abc, db=def".to_string(),
        };
        let issue2 = ValidationIssue {
            note_id: "n2".to_string(),
            title: "Note 2".to_string(),
            issue_type: ValidationIssueType::MissingInDatabase,
            message: "Note has no wiki_page entry".to_string(),
        };
        let issue3 = ValidationIssue {
            note_id: "n3".to_string(),
            title: "Note 3".to_string(),
            issue_type: ValidationIssueType::MissingInFilesystem,
            message: "File does not exist on filesystem".to_string(),
        };
        let issue4 = ValidationIssue {
            note_id: "n4".to_string(),
            title: "Note 4".to_string(),
            issue_type: ValidationIssueType::OrphanInVectorStore,
            message: "Wiki page references non-existent note".to_string(),
        };
        let issues = vec![issue1, issue2, issue3, issue4];
        assert_eq!(issues.len(), 4);
        assert_eq!(issues[0].issue_type, ValidationIssueType::HashMismatch);
        assert_eq!(issues[1].issue_type, ValidationIssueType::MissingInDatabase);
        assert_eq!(issues[2].issue_type, ValidationIssueType::MissingInFilesystem);
        assert_eq!(issues[3].issue_type, ValidationIssueType::OrphanInVectorStore);
    }

    #[test]
    fn test_validation_report_with_multiple_issues() {
        let report = ValidationReport {
            wiki_id: "wiki-multi".to_string(),
            total_notes: 10,
            consistent_notes: 6,
            issues: vec![
                ValidationIssue {
                    note_id: "n1".to_string(),
                    title: "Note 1".to_string(),
                    issue_type: ValidationIssueType::HashMismatch,
                    message: "Hash mismatch".to_string(),
                },
                ValidationIssue {
                    note_id: "n2".to_string(),
                    title: "Note 2".to_string(),
                    issue_type: ValidationIssueType::MissingInFilesystem,
                    message: "File missing".to_string(),
                },
                ValidationIssue {
                    note_id: "n3".to_string(),
                    title: "Note 3".to_string(),
                    issue_type: ValidationIssueType::OrphanInVectorStore,
                    message: "Orphan".to_string(),
                },
                ValidationIssue {
                    note_id: "n4".to_string(),
                    title: "Note 4".to_string(),
                    issue_type: ValidationIssueType::MissingInDatabase,
                    message: "No wiki page".to_string(),
                },
            ],
            checked_at: chrono::Utc::now().timestamp(),
        };
        assert_eq!(report.issues.len(), 4);
        assert_eq!(report.total_notes - report.consistent_notes, 4);
    }

    #[test]
    fn test_validation_report_serialization_roundtrip() {
        let report = ValidationReport {
            wiki_id: "wiki-serde".to_string(),
            total_notes: 100,
            consistent_notes: 95,
            issues: vec![
                ValidationIssue {
                    note_id: "n1".to_string(),
                    title: "Broken Note".to_string(),
                    issue_type: ValidationIssueType::HashMismatch,
                    message: "Hash mismatch: file=abc123, db=def456".to_string(),
                },
            ],
            checked_at: 1700000000,
        };
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: ValidationReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.wiki_id, "wiki-serde");
        assert_eq!(deserialized.total_notes, 100);
        assert_eq!(deserialized.consistent_notes, 95);
        assert_eq!(deserialized.issues.len(), 1);
        assert_eq!(deserialized.checked_at, 1700000000);
    }

    #[test]
    fn test_validation_issue_note_id_preserved() {
        let issue = ValidationIssue {
            note_id: "note-uuid-12345".to_string(),
            title: "Important Note".to_string(),
            issue_type: ValidationIssueType::MissingInFilesystem,
            message: "File does not exist on filesystem".to_string(),
        };
        let json = serde_json::to_string(&issue).unwrap();
        let deserialized: ValidationIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.note_id, "note-uuid-12345");
        assert_eq!(deserialized.title, "Important Note");
    }

    #[test]
    fn test_validation_issue_type_inequality() {
        assert_ne!(ValidationIssueType::HashMismatch, ValidationIssueType::MissingInDatabase);
        assert_ne!(ValidationIssueType::MissingInFilesystem, ValidationIssueType::OrphanInVectorStore);
        assert_ne!(ValidationIssueType::HashMismatch, ValidationIssueType::OrphanInVectorStore);
    }

    #[test]
    fn test_validation_report_zero_notes() {
        let report = ValidationReport {
            wiki_id: "empty-wiki".to_string(),
            total_notes: 0,
            consistent_notes: 0,
            issues: vec![],
            checked_at: 0,
        };
        assert_eq!(report.total_notes, 0);
        assert_eq!(report.consistent_notes, 0);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn test_validation_report_all_inconsistent() {
        let report = ValidationReport {
            wiki_id: "bad-wiki".to_string(),
            total_notes: 5,
            consistent_notes: 0,
            issues: vec![
                ValidationIssue {
                    note_id: "n1".to_string(),
                    title: "A".to_string(),
                    issue_type: ValidationIssueType::HashMismatch,
                    message: "mismatch".to_string(),
                },
                ValidationIssue {
                    note_id: "n2".to_string(),
                    title: "B".to_string(),
                    issue_type: ValidationIssueType::MissingInFilesystem,
                    message: "missing".to_string(),
                },
            ],
            checked_at: 1700000000,
        };
        assert_eq!(report.consistent_notes, 0);
        assert!(report.total_notes > report.consistent_notes);
    }

    #[test]
    fn test_validation_issue_debug_format() {
        let issue = ValidationIssue {
            note_id: "n1".to_string(),
            title: "Test".to_string(),
            issue_type: ValidationIssueType::HashMismatch,
            message: "msg".to_string(),
        };
        let debug = format!("{:?}", issue);
        assert!(debug.contains("HashMismatch"));
        assert!(debug.contains("n1"));
    }

    #[test]
    fn test_validation_issue_type_debug_format() {
        let debug = format!("{:?}", ValidationIssueType::HashMismatch);
        assert!(debug.contains("HashMismatch"));
        let debug = format!("{:?}", ValidationIssueType::MissingInDatabase);
        assert!(debug.contains("MissingInDatabase"));
        let debug = format!("{:?}", ValidationIssueType::MissingInFilesystem);
        assert!(debug.contains("MissingInFilesystem"));
        let debug = format!("{:?}", ValidationIssueType::OrphanInVectorStore);
        assert!(debug.contains("OrphanInVectorStore"));
    }

    #[test]
    fn test_validation_report_debug_format() {
        let report = ValidationReport {
            wiki_id: "w1".to_string(),
            total_notes: 3,
            consistent_notes: 2,
            issues: vec![],
            checked_at: 123,
        };
        let debug = format!("{:?}", report);
        assert!(debug.contains("w1"));
        assert!(debug.contains("3"));
    }

    #[test]
    fn test_calculate_content_hash_deterministic() {
        let hash1 = calculate_content_hash("hello world");
        let hash2 = calculate_content_hash("hello world");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_calculate_content_hash_different_content() {
        let hash1 = calculate_content_hash("hello world");
        let hash2 = calculate_content_hash("hello universe");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_calculate_content_hash_empty_string() {
        let hash = calculate_content_hash("");
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_validation_issue_message_content() {
        let issue = ValidationIssue {
            note_id: "n1".to_string(),
            title: "Test".to_string(),
            issue_type: ValidationIssueType::HashMismatch,
            message: "Hash mismatch: file=abc, db=def".to_string(),
        };
        assert!(issue.message.contains("file="));
        assert!(issue.message.contains("db="));
    }

    #[test]
    fn test_validation_report_checked_at_timestamp() {
        let before = chrono::Utc::now().timestamp();
        let report = ValidationReport {
            wiki_id: "w1".to_string(),
            total_notes: 1,
            consistent_notes: 1,
            issues: vec![],
            checked_at: chrono::Utc::now().timestamp(),
        };
        let after = chrono::Utc::now().timestamp();
        assert!(report.checked_at >= before);
        assert!(report.checked_at <= after);
    }
}