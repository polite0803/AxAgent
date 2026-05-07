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
}
