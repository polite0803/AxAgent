use std::collections::HashSet;
use std::sync::Arc;

use axagent_core::entity::{notes, wiki_pages, wikis};
use axagent_core::markdown_parser::MarkdownParser;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use serde::{Deserialize, Serialize};

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
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub enum LintIssueType {
    BrokenLink { page: String, link: String },
    MissingIndexEntry { page: String },
    OrphanPage { page: String },
    StaleOverview,
    IncompleteSourceSummary { source: String },
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
        let note_model = notes::Entity::find_by_id(note_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Note {} not found", note_id))?;

        let note = axagent_core::repo::note::model_to_note(note_model);
        let mut issues = Vec::new();

        self.check_frontmatter(&note, &mut issues);
        self.check_links(&note, &mut issues).await?;
        self.check_structure(&note, &mut issues);
        self.check_content_quality(&note, &mut issues);

        let score = Self::calculate_score(&issues);

        Ok(LintResult {
            note_id: note_id.to_string(),
            issues,
            score,
        })
    }

    pub async fn lint_vault(&self, wiki_id: &str) -> Result<Vec<LintResult>, String> {
        let db_notes = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(wiki_id))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        let mut all_titles: HashSet<String> = HashSet::new();
        let mut linked_titles: HashSet<String> = HashSet::new();
        let mut note_ids: Vec<String> = Vec::new();

        for n in &db_notes {
            let note = axagent_core::repo::note::model_to_note(n.clone());
            all_titles.insert(note.title.clone());
            note_ids.push(note.id.clone());
        }

        for n in &db_notes {
            let note = axagent_core::repo::note::model_to_note(n.clone());
            let mut issues = Vec::new();

            self.check_frontmatter(&note, &mut issues);
            self.check_links(&note, &mut issues).await?;
            self.check_structure(&note, &mut issues);
            self.check_content_quality(&note, &mut issues);

            let parsed = self.parser.parse(&note.content);
            for link in &parsed.links {
                if link.link_type == "wiki" {
                    linked_titles.insert(link.target.clone());
                }
            }

            let score = Self::calculate_score(&issues);
            results.push(LintResult {
                note_id: note.id.clone(),
                issues,
                score,
            });
        }

        self.check_index_completeness(wiki_id, &all_titles, &mut results)
            .await?;
        self.check_orphan_pages(&note_ids, &linked_titles, &mut results)
            .await?;

        Ok(results)
    }

    fn check_frontmatter(
        &self,
        note: &axagent_core::repo::note::Note,
        issues: &mut Vec<LintIssue>,
    ) {
        if note.title.is_empty() {
            issues.push(LintIssue {
                severity: IssueSeverity::Error,
                code: "missing-title".to_string(),
                message: "Missing title in frontmatter".to_string(),
                line: None,
            });
        }

        if note.author.is_empty() {
            issues.push(LintIssue {
                severity: IssueSeverity::Warning,
                code: "missing-author".to_string(),
                message: "Missing author field".to_string(),
                line: None,
            });
        }

        let parsed = self.parser.parse(&note.content);
        if parsed.frontmatter.tags.is_empty() && note.author == "llm" {
            issues.push(LintIssue {
                severity: IssueSeverity::Info,
                code: "missing-tags".to_string(),
                message: "LLM page has no tags".to_string(),
                line: None,
            });
        }
    }

    async fn check_links(
        &self,
        note: &axagent_core::repo::note::Note,
        issues: &mut Vec<LintIssue>,
    ) -> Result<(), String> {
        let parsed = self.parser.parse(&note.content);

        for link in &parsed.links {
            if link.link_type != "wiki" {
                continue;
            }

            let target_exists = notes::Entity::find()
                .filter(notes::Column::VaultId.eq(&note.vault_id))
                .filter(notes::Column::Title.eq(&link.target))
                .filter(notes::Column::IsDeleted.eq(0))
                .one(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?
                .is_some();

            if !target_exists {
                issues.push(LintIssue {
                    severity: IssueSeverity::Warning,
                    code: "broken-link".to_string(),
                    message: format!("Broken link to [[{}]]", link.target),
                    line: None,
                });
            }
        }

        let backlink_count = axagent_core::entity::note_backlinks::Entity::find()
            .filter(axagent_core::entity::note_backlinks::Column::TargetNoteId.eq(&note.id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .len();

        if backlink_count == 0 && note.author == "llm" {
            issues.push(LintIssue {
                severity: IssueSeverity::Info,
                code: "no-backlinks".to_string(),
                message: "No other pages reference this page".to_string(),
                line: None,
            });
        }

        Ok(())
    }

    fn check_structure(&self, note: &axagent_core::repo::note::Note, issues: &mut Vec<LintIssue>) {
        if note.content.len() < 100 {
            issues.push(LintIssue {
                severity: IssueSeverity::Warning,
                code: "content-too-short".to_string(),
                message: format!("Content is very short ({} chars)", note.content.len()),
                line: None,
            });
        }

        if note.content.len() > 50000 {
            issues.push(LintIssue {
                severity: IssueSeverity::Info,
                code: "content-too-long".to_string(),
                message: "Content is very long, consider splitting into sub-pages".to_string(),
                line: None,
            });
        }

        let has_h2 = note.content.lines().any(|l| l.trim().starts_with("## "));
        if !has_h2 && note.content.len() > 500 {
            issues.push(LintIssue {
                severity: IssueSeverity::Info,
                code: "no-sections".to_string(),
                message: "No H2 sections found, consider structuring content".to_string(),
                line: None,
            });
        }
    }

    fn check_content_quality(
        &self,
        note: &axagent_core::repo::note::Note,
        issues: &mut Vec<LintIssue>,
    ) {
        let lower = note.content.to_lowercase();

        for phrase in &[
            "unknown",
            "not sure",
            "cannot determine",
            "i don't know",
            "todo",
        ] {
            if lower.contains(phrase) {
                issues.push(LintIssue {
                    severity: IssueSeverity::Warning,
                    code: "low-quality-phrase".to_string(),
                    message: format!("Content contains low-quality phrase: \"{}\"", phrase),
                    line: None,
                });
            }
        }

        if !lower.contains("[") && note.author == "llm" {
            issues.push(LintIssue {
                severity: IssueSeverity::Info,
                code: "no-wikilinks".to_string(),
                message: "No wikilinks found. Consider linking to related concepts.".to_string(),
                line: None,
            });
        }
    }

    async fn check_index_completeness(
        &self,
        wiki_id: &str,
        all_titles: &HashSet<String>,
        results: &mut Vec<LintResult>,
    ) -> Result<(), String> {
        let wiki = wikis::Entity::find_by_id(wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let index_path = std::path::Path::new(&wiki.root_path)
            .join("notes")
            .join("index.md");

        if !index_path.exists() {
            results.push(LintResult {
                note_id: "index".to_string(),
                issues: vec![LintIssue {
                    severity: IssueSeverity::Error,
                    code: "missing-index".to_string(),
                    message: "index.md is missing".to_string(),
                    line: None,
                }],
                score: 0.0,
            });
            return Ok(());
        }

        let index_content = tokio::fs::read_to_string(&index_path)
            .await
            .map_err(|e| e.to_string())?;

        let mut indexed_titles: HashSet<String> = HashSet::new();
        let wiki_link_re = regex::Regex::new(r"\[\[([^\]|]+)").ok();
        if let Some(re) = wiki_link_re {
            for cap in re.captures_iter(&index_content) {
                if let Some(m) = cap.get(1) {
                    indexed_titles.insert(m.as_str().to_string());
                }
            }
        }

        let mut missing: Vec<String> = Vec::new();
        for title in all_titles {
            if title == "Index" || title == "Operation Log" || title == "Overview" {
                continue;
            }
            if !indexed_titles.contains(title.as_str()) {
                missing.push(title.clone());
            }
        }

        if !missing.is_empty() {
            results.push(LintResult {
                note_id: "index".to_string(),
                issues: vec![LintIssue {
                    severity: IssueSeverity::Warning,
                    code: "missing-index-entry".to_string(),
                    message: format!("index.md is missing entries for: {}", missing.join(", ")),
                    line: None,
                }],
                score: 0.5,
            });
        }

        Ok(())
    }

    async fn check_orphan_pages(
        &self,
        note_ids: &[String],
        linked_titles: &HashSet<String>,
        results: &mut Vec<LintResult>,
    ) -> Result<(), String> {
        for note_id in note_ids {
            let note = notes::Entity::find_by_id(note_id.as_str())
                .one(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;

            if let Some(n) = note {
                let note_ref = axagent_core::repo::note::model_to_note(n);
                if note_ref.title == "Index"
                    || note_ref.title == "Operation Log"
                    || note_ref.title == "Overview"
                {
                    continue;
                }

                if note_ref.author == "llm" && !linked_titles.contains(&note_ref.title) {
                    let backlinks = axagent_core::entity::note_backlinks::Entity::find()
                        .filter(
                            axagent_core::entity::note_backlinks::Column::TargetNoteId
                                .eq(note_id.as_str()),
                        )
                        .all(self.db.as_ref())
                        .await
                        .map_err(|e| e.to_string())?
                        .len();

                    if backlinks == 0 {
                        results.push(LintResult {
                            note_id: note_id.clone(),
                            issues: vec![LintIssue {
                                severity: IssueSeverity::Warning,
                                code: "orphan-page".to_string(),
                                message: format!(
                                    "Page '{}' is not referenced by any other page",
                                    note_ref.title
                                ),
                                line: None,
                            }],
                            score: 0.3,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn calculate_score(issues: &[LintIssue]) -> f64 {
        let mut score = 1.0_f64;

        for issue in issues {
            match issue.severity {
                IssueSeverity::Error => score -= 0.3,
                IssueSeverity::Warning => score -= 0.1,
                IssueSeverity::Info => score -= 0.02,
            }
        }

        score.clamp(0.0, 1.0)
    }

    pub async fn update_quality_score(&self, note_id: &str) -> Result<f64, String> {
        let result = self.lint_note(note_id).await?;

        let wiki_page = wiki_pages::Entity::find()
            .filter(wiki_pages::Column::NoteId.eq(note_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        if let Some(wp) = wiki_page {
            let mut am = wp.into_active_model();
            am.quality_score = Set(Some(result.score));
            am.last_linted_at = Set(Some(chrono::Utc::now().timestamp()));
            am.update(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(result.score)
    }

    pub async fn auto_fix(
        &self,
        wiki_id: &str,
        note_id: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let mut fixed = Vec::new();

        if let Some(nid) = note_id {
            let result = self.lint_note(nid).await?;
            let link_re =
                regex::Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]").map_err(|e| e.to_string())?;
            for issue in &result.issues {
                if issue.code == "broken-link" {
                    let note = axagent_core::repo::note::get_note(self.db.as_ref(), nid)
                        .await
                        .map_err(|e| e.to_string())?;

                    let mut content = note.content.clone();

                    let valid_titles: HashSet<String> = notes::Entity::find()
                        .filter(notes::Column::VaultId.eq(&note.vault_id))
                        .filter(notes::Column::IsDeleted.eq(0))
                        .all(self.db.as_ref())
                        .await
                        .map_err(|e| e.to_string())?
                        .iter()
                        .map(|n| n.title.clone())
                        .collect();

                    content = link_re
                        .replace_all(&content, |caps: &regex::Captures| {
                            let target = caps.get(1).unwrap().as_str();
                            if valid_titles.contains(target) {
                                caps.get(0).unwrap().as_str().to_string()
                            } else {
                                format!("`{}` (broken link)", target)
                            }
                        })
                        .to_string();

                    if content != note.content {
                        let input = axagent_core::repo::note::UpdateNoteInput {
                            title: None,
                            content: Some(content),
                            page_type: None,
                            related_pages: None,
                        };
                        axagent_core::repo::note::update_note(self.db.as_ref(), nid, input)
                            .await
                            .map_err(|e| e.to_string())?;
                        fixed.push(format!("Fixed broken links in {}", nid));
                    }
                }
            }
        } else {
            let results = self.lint_vault(wiki_id).await?;
            for result in results {
                if result.issues.iter().any(|i| i.code == "missing-index") {
                    fixed.push("Index is missing. Run compile to regenerate it.".to_string());
                }
                if result
                    .issues
                    .iter()
                    .any(|i| i.code == "missing-index-entry")
                {
                    fixed.push(
                        "Index has missing entries. Run compile to regenerate it.".to_string(),
                    );
                }
            }
        }

        Ok(fixed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_core::repo::note::Note;

    fn make_note(title: &str, author: &str, content: &str) -> Note {
        Note {
            id: "test-note-id".to_string(),
            vault_id: "test-vault".to_string(),
            title: title.to_string(),
            file_path: "test.md".to_string(),
            content: content.to_string(),
            content_hash: "hash".to_string(),
            author: author.to_string(),
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
        }
    }

    async fn make_lint_checker() -> LintChecker {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        LintChecker::new(Arc::new(db))
    }

    #[test]
    fn test_calculate_score_no_issues() {
        let issues: Vec<LintIssue> = vec![];
        let score = LintChecker::calculate_score(&issues);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_calculate_score_single_error() {
        let issues = vec![LintIssue {
            severity: IssueSeverity::Error,
            code: "test".to_string(),
            message: "error".to_string(),
            line: None,
        }];
        let score = LintChecker::calculate_score(&issues);
        assert!((score - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_score_single_warning() {
        let issues = vec![LintIssue {
            severity: IssueSeverity::Warning,
            code: "test".to_string(),
            message: "warning".to_string(),
            line: None,
        }];
        let score = LintChecker::calculate_score(&issues);
        assert!((score - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_score_single_info() {
        let issues = vec![LintIssue {
            severity: IssueSeverity::Info,
            code: "test".to_string(),
            message: "info".to_string(),
            line: None,
        }];
        let score = LintChecker::calculate_score(&issues);
        assert!((score - 0.98).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_score_clamped_to_zero() {
        let issues: Vec<LintIssue> = (0..10)
            .map(|_| LintIssue {
                severity: IssueSeverity::Error,
                code: "test".to_string(),
                message: "error".to_string(),
                line: None,
            })
            .collect();
        let score = LintChecker::calculate_score(&issues);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_calculate_score_mixed_severities() {
        let issues = vec![
            LintIssue {
                severity: IssueSeverity::Error,
                code: "e1".to_string(),
                message: "err".to_string(),
                line: None,
            },
            LintIssue {
                severity: IssueSeverity::Warning,
                code: "w1".to_string(),
                message: "warn".to_string(),
                line: None,
            },
            LintIssue {
                severity: IssueSeverity::Info,
                code: "i1".to_string(),
                message: "info".to_string(),
                line: None,
            },
        ];
        let score = LintChecker::calculate_score(&issues);
        assert!((score - 0.58).abs() < f64::EPSILON);
    }

    #[test]
    fn test_issue_severity_equality() {
        assert_eq!(IssueSeverity::Error, IssueSeverity::Error);
        assert_eq!(IssueSeverity::Warning, IssueSeverity::Warning);
        assert_eq!(IssueSeverity::Info, IssueSeverity::Info);
        assert_ne!(IssueSeverity::Error, IssueSeverity::Warning);
    }

    #[test]
    fn test_lint_issue_type_variants() {
        let types = [
            LintIssueType::BrokenLink {
                page: "p".to_string(),
                link: "l".to_string(),
            },
            LintIssueType::MissingIndexEntry {
                page: "p".to_string(),
            },
            LintIssueType::OrphanPage {
                page: "p".to_string(),
            },
            LintIssueType::StaleOverview,
            LintIssueType::IncompleteSourceSummary {
                source: "s".to_string(),
            },
        ];
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_lint_result_serialization() {
        let result = LintResult {
            note_id: "n1".to_string(),
            issues: vec![LintIssue {
                severity: IssueSeverity::Warning,
                code: "broken-link".to_string(),
                message: "Broken link".to_string(),
                line: Some(5),
            }],
            score: 0.9,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: LintResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.note_id, "n1");
        assert_eq!(deserialized.issues.len(), 1);
        assert!((deserialized.score - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lint_issue_serialization() {
        let issue = LintIssue {
            severity: IssueSeverity::Error,
            code: "missing-title".to_string(),
            message: "Missing title".to_string(),
            line: Some(1),
        };
        let json = serde_json::to_string(&issue).unwrap();
        let deserialized: LintIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.severity, IssueSeverity::Error);
        assert_eq!(deserialized.line, Some(1));
    }

    #[test]
    fn test_calculate_score_multiple_errors() {
        let issues = vec![
            LintIssue {
                severity: IssueSeverity::Error,
                code: "e1".to_string(),
                message: "err1".to_string(),
                line: None,
            },
            LintIssue {
                severity: IssueSeverity::Error,
                code: "e2".to_string(),
                message: "err2".to_string(),
                line: None,
            },
            LintIssue {
                severity: IssueSeverity::Error,
                code: "e3".to_string(),
                message: "err3".to_string(),
                line: None,
            },
        ];
        let score = LintChecker::calculate_score(&issues);
        assert!((score - 0.1).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_check_frontmatter_missing_title() {
        let checker = make_lint_checker().await;
        let note =
            make_note("", "llm", "---\ntitle: \n---\nSome content here that is long enough.");
        let mut issues = Vec::new();
        checker.check_frontmatter(&note, &mut issues);
        assert!(issues.iter().any(|i| i.code == "missing-title"));
        assert!(issues.iter().any(|i| i.severity == IssueSeverity::Error));
    }

    #[tokio::test]
    async fn test_check_frontmatter_missing_author() {
        let checker = make_lint_checker().await;
        let note = make_note("Test Title", "", "---\ntitle: Test\n---\nSome content here.");
        let mut issues = Vec::new();
        checker.check_frontmatter(&note, &mut issues);
        assert!(issues.iter().any(|i| i.code == "missing-author"));
        assert!(issues.iter().any(|i| i.severity == IssueSeverity::Warning));
    }

    #[tokio::test]
    async fn test_check_frontmatter_missing_tags_llm() {
        let checker = make_lint_checker().await;
        let note =
            make_note("Test Title", "llm", "---\ntitle: Test\n---\nSome content without tags.");
        let mut issues = Vec::new();
        checker.check_frontmatter(&note, &mut issues);
        assert!(issues.iter().any(|i| i.code == "missing-tags"));
        assert!(issues.iter().any(|i| i.severity == IssueSeverity::Info));
    }

    #[tokio::test]
    async fn test_check_frontmatter_has_tags_no_issue() {
        let checker = make_lint_checker().await;
        let note = make_note(
            "Test Title",
            "llm",
            "---\ntitle: Test\ntags: [AI, ML]\n---\nSome content with tags.",
        );
        let mut issues = Vec::new();
        checker.check_frontmatter(&note, &mut issues);
        assert!(!issues.iter().any(|i| i.code == "missing-tags"));
    }

    #[tokio::test]
    async fn test_check_frontmatter_non_llm_no_missing_tags_issue() {
        let checker = make_lint_checker().await;
        let note =
            make_note("Test Title", "user", "---\ntitle: Test\n---\nSome content without tags.");
        let mut issues = Vec::new();
        checker.check_frontmatter(&note, &mut issues);
        assert!(!issues.iter().any(|i| i.code == "missing-tags"));
    }

    #[tokio::test]
    async fn test_check_structure_too_short() {
        let checker = make_lint_checker().await;
        let note = make_note("Short", "llm", "Hi");
        let mut issues = Vec::new();
        checker.check_structure(&note, &mut issues);
        assert!(issues.iter().any(|i| i.code == "content-too-short"));
        assert!(issues.iter().any(|i| i.severity == IssueSeverity::Warning));
    }

    #[tokio::test]
    async fn test_check_structure_too_long() {
        let checker = make_lint_checker().await;
        let long_content = "x".repeat(50001);
        let note = make_note("Long", "llm", &long_content);
        let mut issues = Vec::new();
        checker.check_structure(&note, &mut issues);
        assert!(issues.iter().any(|i| i.code == "content-too-long"));
        assert!(issues.iter().any(|i| i.severity == IssueSeverity::Info));
    }

    #[tokio::test]
    async fn test_check_structure_no_sections() {
        let checker = make_lint_checker().await;
        let content = "A".repeat(501);
        let note = make_note("No Sections", "llm", &content);
        let mut issues = Vec::new();
        checker.check_structure(&note, &mut issues);
        assert!(issues.iter().any(|i| i.code == "no-sections"));
    }

    #[tokio::test]
    async fn test_check_structure_has_sections_ok() {
        let checker = make_lint_checker().await;
        let content = "## Section 1\n\nSome content here. More content. Even more content. And a bit more to reach the minimum threshold for the test.";
        let note = make_note("Good Structure", "llm", content);
        let mut issues = Vec::new();
        checker.check_structure(&note, &mut issues);
        assert!(!issues.iter().any(|i| i.code == "no-sections"));
        assert!(!issues.iter().any(|i| i.code == "content-too-short"));
    }

    #[tokio::test]
    async fn test_check_structure_short_with_sections_no_short_warning() {
        let checker = make_lint_checker().await;
        let content = "## Section\n\nShort but has section.";
        let note = make_note("Short Sections", "llm", content);
        let mut issues = Vec::new();
        checker.check_structure(&note, &mut issues);
        assert!(issues.iter().any(|i| i.code == "content-too-short"));
        assert!(!issues.iter().any(|i| i.code == "no-sections"));
    }

    #[tokio::test]
    async fn test_check_content_quality_low_quality_phrases() {
        let checker = make_lint_checker().await;
        let note = make_note("Low Quality", "llm", "The answer is unknown. I am not sure about this. TODO: fill in later. Cannot determine the result. I don't know the answer.");
        let mut issues = Vec::new();
        checker.check_content_quality(&note, &mut issues);
        let low_quality_issues: Vec<&LintIssue> = issues
            .iter()
            .filter(|i| i.code == "low-quality-phrase")
            .collect();
        assert!(!low_quality_issues.is_empty());
        assert!(low_quality_issues
            .iter()
            .all(|i| i.severity == IssueSeverity::Warning));
    }

    #[tokio::test]
    async fn test_check_content_quality_no_wikilinks_llm() {
        let checker = make_lint_checker().await;
        let note = make_note(
            "No Links",
            "llm",
            "This content has no wiki links at all. It is just plain text without any brackets.",
        );
        let mut issues = Vec::new();
        checker.check_content_quality(&note, &mut issues);
        assert!(issues.iter().any(|i| i.code == "no-wikilinks"));
        assert!(issues.iter().any(|i| i.severity == IssueSeverity::Info));
    }

    #[tokio::test]
    async fn test_check_content_quality_has_links_ok() {
        let checker = make_lint_checker().await;
        let note = make_note(
            "Has Links",
            "llm",
            "This content has [[wiki links]] and [regular links](http://example.com).",
        );
        let mut issues = Vec::new();
        checker.check_content_quality(&note, &mut issues);
        assert!(!issues.iter().any(|i| i.code == "no-wikilinks"));
    }

    #[tokio::test]
    async fn test_check_content_quality_non_llm_no_wikilink_warning() {
        let checker = make_lint_checker().await;
        let note = make_note(
            "User Note",
            "user",
            "This content has no wiki links at all. It is just plain text.",
        );
        let mut issues = Vec::new();
        checker.check_content_quality(&note, &mut issues);
        assert!(!issues.iter().any(|i| i.code == "no-wikilinks"));
    }

    #[test]
    fn test_lint_issue_type_debug_format() {
        let issue_type = LintIssueType::BrokenLink {
            page: "test".to_string(),
            link: "link".to_string(),
        };
        let debug_str = format!("{:?}", issue_type);
        assert!(debug_str.contains("BrokenLink"));
    }

    #[test]
    fn test_issue_severity_serialization() {
        let error_json = serde_json::to_string(&IssueSeverity::Error).unwrap();
        assert!(error_json.contains("Error"));
        let warning_json = serde_json::to_string(&IssueSeverity::Warning).unwrap();
        assert!(warning_json.contains("Warning"));
        let info_json = serde_json::to_string(&IssueSeverity::Info).unwrap();
        assert!(info_json.contains("Info"));

        let deserialized: IssueSeverity = serde_json::from_str(&error_json).unwrap();
        assert_eq!(deserialized, IssueSeverity::Error);
    }

    #[test]
    fn test_calculate_score_boundary() {
        let issues = vec![
            LintIssue {
                severity: IssueSeverity::Error,
                code: "e".to_string(),
                message: "err".to_string(),
                line: None,
            },
            LintIssue {
                severity: IssueSeverity::Warning,
                code: "w".to_string(),
                message: "warn".to_string(),
                line: None,
            },
        ];
        let score = LintChecker::calculate_score(&issues);
        assert!((score - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lint_result_with_multiple_issues() {
        let result = LintResult {
            note_id: "n1".to_string(),
            issues: vec![
                LintIssue {
                    severity: IssueSeverity::Error,
                    code: "e1".to_string(),
                    message: "err1".to_string(),
                    line: Some(1),
                },
                LintIssue {
                    severity: IssueSeverity::Warning,
                    code: "w1".to_string(),
                    message: "warn1".to_string(),
                    line: Some(5),
                },
                LintIssue {
                    severity: IssueSeverity::Info,
                    code: "i1".to_string(),
                    message: "info1".to_string(),
                    line: None,
                },
            ],
            score: 0.58,
        };
        assert_eq!(result.issues.len(), 3);
        assert_eq!(result.issues[0].line, Some(1));
        assert_eq!(result.issues[1].line, Some(5));
        assert_eq!(result.issues[2].line, None);
    }

    #[test]
    fn test_lint_issue_with_line_number() {
        let issue = LintIssue {
            severity: IssueSeverity::Warning,
            code: "broken-link".to_string(),
            message: "Broken link at line 10".to_string(),
            line: Some(10),
        };
        assert_eq!(issue.line, Some(10));
        assert_eq!(issue.code, "broken-link");
    }

    #[test]
    fn test_lint_issue_without_line_number() {
        let issue = LintIssue {
            severity: IssueSeverity::Error,
            code: "missing-title".to_string(),
            message: "Missing title".to_string(),
            line: None,
        };
        assert_eq!(issue.line, None);
    }

    #[test]
    fn test_calculate_score_all_warnings() {
        let issues: Vec<LintIssue> = (0..5)
            .map(|i| LintIssue {
                severity: IssueSeverity::Warning,
                code: format!("w{}", i),
                message: format!("warn{}", i),
                line: None,
            })
            .collect();
        let score = LintChecker::calculate_score(&issues);
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_score_all_info() {
        let issues: Vec<LintIssue> = (0..5)
            .map(|i| LintIssue {
                severity: IssueSeverity::Info,
                code: format!("i{}", i),
                message: format!("info{}", i),
                line: None,
            })
            .collect();
        let score = LintChecker::calculate_score(&issues);
        assert!((score - 0.9).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_check_content_quality_specific_phrases() {
        let checker = make_lint_checker().await;

        let note_unknown = make_note("T", "llm", "The result is unknown at this time.");
        let mut issues = Vec::new();
        checker.check_content_quality(&note_unknown, &mut issues);
        assert!(issues
            .iter()
            .any(|i| i.code == "low-quality-phrase" && i.message.contains("unknown")));

        let note_todo = make_note("T", "llm", "TODO: implement this feature later.");
        let mut issues2 = Vec::new();
        checker.check_content_quality(&note_todo, &mut issues2);
        assert!(issues2
            .iter()
            .any(|i| i.code == "low-quality-phrase" && i.message.contains("todo")));
    }

    #[tokio::test]
    async fn test_check_frontmatter_all_present() {
        let checker = make_lint_checker().await;
        let note = make_note(
            "Good Title",
            "llm",
            "---\ntitle: Good Title\ntags: [test]\n---\nContent with [[links]].",
        );
        let mut issues = Vec::new();
        checker.check_frontmatter(&note, &mut issues);
        assert!(!issues.iter().any(|i| i.code == "missing-title"));
        assert!(!issues.iter().any(|i| i.code == "missing-author"));
        assert!(!issues.iter().any(|i| i.code == "missing-tags"));
    }

    #[test]
    fn test_lint_result_serialization_roundtrip() {
        let result = LintResult {
            note_id: "n1".to_string(),
            issues: vec![
                LintIssue {
                    severity: IssueSeverity::Error,
                    code: "e1".to_string(),
                    message: "err".to_string(),
                    line: Some(3),
                },
                LintIssue {
                    severity: IssueSeverity::Warning,
                    code: "w1".to_string(),
                    message: "warn".to_string(),
                    line: None,
                },
            ],
            score: 0.6,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: LintResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.note_id, result.note_id);
        assert_eq!(back.issues.len(), result.issues.len());
        assert!((back.score - result.score).abs() < f64::EPSILON);
    }
}
