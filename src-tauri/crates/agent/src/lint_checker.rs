use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use axagent_core::entity::{notes, wiki_pages};
use axagent_core::markdown_parser::MarkdownParser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub note_id: String,
    pub issues: Vec<LintIssue>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintIssue {
    pub severity: IssueSeverity,
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueSeverity::Error => write!(f, "error"),
            IssueSeverity::Warning => write!(f, "warning"),
            IssueSeverity::Info => write!(f, "info"),
        }
    }
}

pub struct LintChecker {
    db: Arc<DatabaseConnection>,
    parser: MarkdownParser,
}

impl LintChecker {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            parser: MarkdownParser::new(),
        }
    }

    pub async fn lint_note(&self, note_id: &str) -> Result<LintResult, String> {
        let note = notes::Entity::find_by_id(note_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Note {} not found", note_id))?;

        let mut issues = Vec::new();

        issues.extend(self.check_frontmatter(&note).await?);
        issues.extend(self.check_links(&note).await?);
        issues.extend(self.check_structure(&note).await?);
        issues.extend(self.check_content_quality(&note).await?);

        let score = self.calculate_score(&issues);

        Ok(LintResult {
            note_id: note_id.to_string(),
            issues,
            score,
        })
    }

    async fn check_frontmatter(&self, note: &notes::Model) -> Result<Vec<LintIssue>, String> {
        let mut issues = Vec::new();
        let parsed = self.parser.parse(&note.content);

        if parsed.frontmatter.title.is_none() {
            issues.push(LintIssue {
                severity: IssueSeverity::Warning,
                code: "MISSING_TITLE".to_string(),
                message: "Frontmatter missing 'title' field".to_string(),
                line: None,
            });
        }

        if parsed.frontmatter.author.is_none() {
            issues.push(LintIssue {
                severity: IssueSeverity::Warning,
                code: "MISSING_AUTHOR".to_string(),
                message: "Frontmatter missing 'author' field".to_string(),
                line: None,
            });
        }

        if parsed.frontmatter.tags.is_empty() {
            issues.push(LintIssue {
                severity: IssueSeverity::Info,
                code: "NO_TAGS".to_string(),
                message: "No tags defined in frontmatter".to_string(),
                line: None,
            });
        }

        Ok(issues)
    }

    async fn check_links(&self, note: &notes::Model) -> Result<Vec<LintIssue>, String> {
        let mut issues = Vec::new();
        let parsed = self.parser.parse(&note.content);

        let valid_targets: HashSet<String> = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(&note.vault_id))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|n| n.title.to_lowercase())
            .collect();

        for link in &parsed.links {
            if link.link_type == "wiki" && !valid_targets.contains(&link.target.to_lowercase()) {
                issues.push(LintIssue {
                    severity: IssueSeverity::Warning,
                    code: "BROKEN_LINK".to_string(),
                    message: format!("Broken link: [[{}]]", link.target),
                    line: None,
                });
            }
        }

        let backlinks = axagent_core::entity::note_backlinks::Entity::find()
            .filter(axagent_core::entity::note_backlinks::Column::TargetNoteId.eq(&note.id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        if backlinks.is_empty() && !parsed.links.is_empty() {
            issues.push(LintIssue {
                severity: IssueSeverity::Info,
                code: "NO_BACKLINKS".to_string(),
                message: "This note has outgoing links but no backlinks".to_string(),
                line: None,
            });
        }

        Ok(issues)
    }

    async fn check_structure(&self, note: &notes::Model) -> Result<Vec<LintIssue>, String> {
        let mut issues = Vec::new();
        let content = &note.content;

        if content.len() < 100 {
            issues.push(LintIssue {
                severity: IssueSeverity::Warning,
                code: "TOO_SHORT".to_string(),
                message: format!("Note is very short ({} chars)", content.len()),
                line: None,
            });
        }

        if content.len() > 10000 {
            issues.push(LintIssue {
                severity: IssueSeverity::Warning,
                code: "TOO_LONG".to_string(),
                message: format!("Note is very long ({} chars)", content.len()),
                line: None,
            });
        }

        let heading_count = content.lines().filter(|line| line.starts_with("## ")).count();

        if heading_count == 0 {
            issues.push(LintIssue {
                severity: IssueSeverity::Warning,
                code: "NO_HEADINGS".to_string(),
                message: "Note has no section headings".to_string(),
                line: None,
            });
        }

        Ok(issues)
    }

    async fn check_content_quality(&self, note: &notes::Model) -> Result<Vec<LintIssue>, String> {
        let mut issues = Vec::new();
        let content = &note.content.to_lowercase();

        let low_quality_phrases = [
            ("unknown", "Uses 'unknown' which may indicate incomplete information"),
            ("not sure", "Uses 'not sure' which may indicate uncertainty"),
            ("cannot determine", "Uses 'cannot determine' which may indicate incomplete analysis"),
            ("todo", "Contains 'TODO' which may indicate incomplete work"),
        ];

        for (phrase, msg) in &low_quality_phrases {
            if content.contains(phrase) {
                issues.push(LintIssue {
                    severity: IssueSeverity::Warning,
                    code: "LOW_QUALITY_PHRASE".to_string(),
                    message: msg.to_string(),
                    line: None,
                });
            }
        }

        let wiki_link_count = note.content.matches("[[").count();
        if wiki_link_count == 0 {
            issues.push(LintIssue {
                severity: IssueSeverity::Info,
                code: "NO_WIKI_LINKS".to_string(),
                message: "Note has no wiki links to other notes".to_string(),
                line: None,
            });
        }

        Ok(issues)
    }

    fn calculate_score(&self, issues: &[LintIssue]) -> f64 {
        if issues.is_empty() {
            return 1.0_f64;
        }

        let mut score = 1.0_f64;
        for issue in issues {
            match issue.severity {
                IssueSeverity::Error => score -= 0.3,
                IssueSeverity::Warning => score -= 0.1,
                IssueSeverity::Info => score -= 0.02,
            }
        }

        score.max(0.0).min(1.0)
    }

    pub async fn update_quality_score(&self, note_id: &str) -> Result<f64, String> {
        let result = self.lint_note(note_id).await?;

        let active_model = wiki_pages::ActiveModel {
            note_id: Set(note_id.to_string()),
            quality_score: Set(Some(result.score)),
            last_linted_at: Set(Some(chrono::Utc::now().timestamp())),
            ..Default::default()
        };

        active_model
            .update(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        Ok(result.score)
    }
}