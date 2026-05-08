use sea_orm::{DatabaseConnection, EntityTrait};

pub const DEFAULT_PURPOSE_TEMPLATE: &str = r#"# {wiki_name}

## Purpose

[Describe the purpose and goals of this wiki - what knowledge do you want to accumulate?]

## Key Questions

- [Key question 1]
- [Key question 2]
- [Key question 3]

## Research Scope

[Define the scope and boundaries of research for this wiki]

## Thesis

[As knowledge accumulates, what core thesis or conclusion do you hope to form?]

## Evolving Notes

[Record the evolution of this purpose over time]
- {date}: Initial creation
"#;

pub struct PurposeManager;

impl PurposeManager {
    pub async fn load(db: &DatabaseConnection, wiki_id: &str) -> Result<String, String> {
        let wiki = axagent_core::entity::wikis::Entity::find_by_id(wiki_id)
            .one(db)
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

    pub async fn save(db: &DatabaseConnection, wiki_id: &str, content: &str) -> Result<(), String> {
        let wiki = axagent_core::entity::wikis::Entity::find_by_id(wiki_id)
            .one(db)
            .await
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let purpose_path = std::path::Path::new(&wiki.root_path).join("purpose.md");
        if let Some(parent) = purpose_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        tokio::fs::write(&purpose_path, content)
            .await
            .map_err(|e| format!("Failed to write purpose.md: {}", e))
    }

    pub async fn initialize(
        db: &DatabaseConnection,
        wiki_id: &str,
        wiki_name: &str,
    ) -> Result<(), String> {
        let content = DEFAULT_PURPOSE_TEMPLATE
            .replace("{wiki_name}", wiki_name)
            .replace("{date}", &chrono::Utc::now().format("%Y-%m-%d").to_string());

        Self::save(db, wiki_id, &content).await
    }

    pub async fn exists(db: &DatabaseConnection, wiki_id: &str) -> Result<bool, String> {
        let wiki = axagent_core::entity::wikis::Entity::find_by_id(wiki_id)
            .one(db)
            .await
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or_else(|| format!("Wiki {} not found", wiki_id))?;

        let purpose_path = std::path::Path::new(&wiki.root_path).join("purpose.md");
        Ok(purpose_path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_purpose_template_contains_wiki_name_placeholder() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("{wiki_name}"));
    }

    #[test]
    fn test_default_purpose_template_contains_date_placeholder() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("{date}"));
    }

    #[test]
    fn test_default_purpose_template_has_purpose_section() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("## Purpose"));
    }

    #[test]
    fn test_default_purpose_template_has_key_questions_section() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("## Key Questions"));
    }

    #[test]
    fn test_default_purpose_template_has_research_scope_section() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("## Research Scope"));
    }

    #[test]
    fn test_default_purpose_template_has_thesis_section() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("## Thesis"));
    }

    #[test]
    fn test_default_purpose_template_has_evolving_notes_section() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("## Evolving Notes"));
    }

    #[test]
    fn test_default_purpose_template_replace_wiki_name() {
        let content = DEFAULT_PURPOSE_TEMPLATE.replace("{wiki_name}", "My Wiki");
        assert!(content.contains("# My Wiki"));
        assert!(!content.contains("{wiki_name}"));
    }

    #[test]
    fn test_default_purpose_template_replace_date() {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let content = DEFAULT_PURPOSE_TEMPLATE
            .replace("{wiki_name}", "Test")
            .replace("{date}", &today);
        assert!(content.contains(&today));
        assert!(!content.contains("{date}"));
    }

    #[test]
    fn test_default_purpose_template_full_replacement() {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let content = DEFAULT_PURPOSE_TEMPLATE
            .replace("{wiki_name}", "Research Wiki")
            .replace("{date}", &today);
        assert!(content.starts_with("# Research Wiki"));
        assert!(content.contains(&today));
        assert!(!content.contains("{wiki_name}"));
        assert!(!content.contains("{date}"));
    }

    #[test]
    fn test_default_purpose_template_is_not_empty() {
        assert!(!DEFAULT_PURPOSE_TEMPLATE.is_empty());
    }

    #[test]
    fn test_template_starts_with_heading() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.starts_with("# "));
    }

    #[test]
    fn test_template_has_key_question_placeholders() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("[Key question 1]"));
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("[Key question 2]"));
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("[Key question 3]"));
    }

    #[test]
    fn test_template_has_describe_purpose_placeholder() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("[Describe the purpose"));
    }

    #[test]
    fn test_template_has_research_scope_placeholder() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("[Define the scope"));
    }

    #[test]
    fn test_template_has_thesis_placeholder() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("[As knowledge accumulates"));
    }

    #[test]
    fn test_template_has_evolving_notes_placeholder() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("[Record the evolution"));
    }

    #[test]
    fn test_template_date_in_evolving_notes() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("- {date}: Initial creation"));
    }

    #[test]
    fn test_template_replace_with_empty_wiki_name() {
        let content = DEFAULT_PURPOSE_TEMPLATE.replace("{wiki_name}", "");
        assert!(content.starts_with("# "));
        assert!(!content.contains("{wiki_name}"));
    }

    #[test]
    fn test_template_replace_with_special_characters() {
        let content = DEFAULT_PURPOSE_TEMPLATE
            .replace("{wiki_name}", "Wiki <>&\"'")
            .replace("{date}", "2025-01-01");
        assert!(content.contains("Wiki <>&\"'"));
        assert!(!content.contains("{wiki_name}"));
    }

    #[test]
    fn test_template_replace_with_unicode_wiki_name() {
        let content = DEFAULT_PURPOSE_TEMPLATE
            .replace("{wiki_name}", "知识库")
            .replace("{date}", "2025-01-01");
        assert!(content.contains("# 知识库"));
        assert!(!content.contains("{wiki_name}"));
    }

    #[test]
    fn test_template_replace_with_long_wiki_name() {
        let long_name = "A".repeat(1000);
        let content = DEFAULT_PURPOSE_TEMPLATE.replace("{wiki_name}", &long_name);
        assert!(content.contains(&long_name));
        assert!(!content.contains("{wiki_name}"));
    }

    #[test]
    fn test_template_date_format_matches_chrono() {
        let date_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let parsed = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d");
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_template_multiple_replacements_independent() {
        let content1 = DEFAULT_PURPOSE_TEMPLATE
            .replace("{wiki_name}", "Wiki A")
            .replace("{date}", "2025-01-01");
        let content2 = DEFAULT_PURPOSE_TEMPLATE
            .replace("{wiki_name}", "Wiki B")
            .replace("{date}", "2025-12-31");
        assert!(content1.contains("# Wiki A"));
        assert!(content2.contains("# Wiki B"));
        assert!(content1.contains("2025-01-01"));
        assert!(content2.contains("2025-12-31"));
    }

    #[test]
    fn test_template_sections_order() {
        let purpose_pos = DEFAULT_PURPOSE_TEMPLATE.find("## Purpose").unwrap();
        let questions_pos = DEFAULT_PURPOSE_TEMPLATE.find("## Key Questions").unwrap();
        let scope_pos = DEFAULT_PURPOSE_TEMPLATE.find("## Research Scope").unwrap();
        let thesis_pos = DEFAULT_PURPOSE_TEMPLATE.find("## Thesis").unwrap();
        let notes_pos = DEFAULT_PURPOSE_TEMPLATE.find("## Evolving Notes").unwrap();

        assert!(purpose_pos < questions_pos);
        assert!(questions_pos < scope_pos);
        assert!(scope_pos < thesis_pos);
        assert!(thesis_pos < notes_pos);
    }

    #[test]
    fn test_purpose_manager_unit_struct() {
        let _manager = PurposeManager;
    }

    #[test]
    fn test_template_has_markdown_list_items() {
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("- [Key question"));
        assert!(DEFAULT_PURPOSE_TEMPLATE.contains("- {date}:"));
    }

    #[test]
    fn test_template_no_double_replacement_wiki_name() {
        let content = DEFAULT_PURPOSE_TEMPLATE.replace("{wiki_name}", "__WIKI_NAME_PLACEHOLDER__");
        let content = content.replace("__WIKI_NAME_PLACEHOLDER__", "{wiki_name}");
        let count = content.matches("{wiki_name}").count();
        assert_eq!(count, 1);
    }
}
