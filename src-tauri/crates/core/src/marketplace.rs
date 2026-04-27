use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::{workflow_template, workflow_template_version};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExport {
    pub template: workflow_template::Model,
    pub versions: Vec<workflow_template_version::Model>,
    pub exported_at: DateTime<Utc>,
    pub version: String,
}

impl WorkflowExport {
    pub fn new(
        template: workflow_template::Model,
        versions: Vec<workflow_template_version::Model>,
    ) -> Self {
        Self {
            template,
            versions,
            exported_at: Utc::now(),
            version: "1.0".to_string(),
        }
    }

    pub fn serialize(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn deserialize(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceTemplate {
    pub id: String,
    pub template_id: String,
    pub author_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub icon: String,
    pub tags: Option<String>,
    pub downloads: i64,
    pub rating_average: f32,
    pub rating_count: i32,
    pub is_featured: bool,
    pub is_verified: bool,
}

impl MarketplaceTemplate {
    pub fn new(template_id: &str, author_id: &str, name: &str, category: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            template_id: template_id.to_string(),
            author_id: author_id.to_string(),
            name: name.to_string(),
            description: None,
            category: category.to_string(),
            icon: "Bot".to_string(),
            tags: None,
            downloads: 0,
            rating_average: 0.0,
            rating_count: 0,
            is_featured: false,
            is_verified: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceReview {
    pub id: String,
    pub marketplace_id: String,
    pub user_id: String,
    pub rating: i32,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl MarketplaceReview {
    pub fn new(marketplace_id: &str, user_id: &str, rating: i32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            marketplace_id: marketplace_id.to_string(),
            user_id: user_id.to_string(),
            rating,
            comment: None,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category;

impl Category {
    pub const PRODUCTIVITY: &'static str = "Productivity";
    pub const DEVELOPMENT: &'static str = "Development";
    pub const DATA: &'static str = "Data";
    pub const AUTOMATION: &'static str = "Automation";
    pub const AI: &'static str = "AI";
    pub const BUSINESS: &'static str = "Business";

    pub fn all() -> Vec<&'static str> {
        vec![
            Self::PRODUCTIVITY,
            Self::DEVELOPMENT,
            Self::DATA,
            Self::AUTOMATION,
            Self::AI,
            Self::BUSINESS,
        ]
    }

    pub fn icon(category: &str) -> &'static str {
        match category {
            Self::PRODUCTIVITY => "Calendar",
            Self::DEVELOPMENT => "Code",
            Self::DATA => "Database",
            Self::AUTOMATION => "Robot",
            Self::AI => "Robot",
            Self::BUSINESS => "Bank",
            _ => "Folder",
        }
    }
}

pub struct MarketplaceService;

impl MarketplaceService {
    pub fn generate_import_code(export: &WorkflowExport) -> String {
        format!(
            r#"# Workflow Import: {}

Created: {}
Version: {}

To import this workflow, use the workflow import feature in the application.
"#,
            export.template.name,
            export.exported_at.format("%Y-%m-%d %H:%M:%S UTC"),
            export.version
        )
    }

    pub fn validate_template_for_publish(
        template: &workflow_template::Model,
    ) -> Result<(), String> {
        if template.name.is_empty() {
            return Err("Template name is required".to_string());
        }
        if template.nodes.is_empty() {
            return Err("Template must have at least one node".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_icons() {
        assert_eq!(Category::icon("Productivity"), "Calendar");
        assert_eq!(Category::icon("Development"), "Code");
        assert_eq!(Category::icon("Unknown"), "Folder");
    }

    #[test]
    fn test_workflow_export_serialize() {
        let export = WorkflowExport::new(
            workflow_template::Model {
                id: "test-id".to_string(),
                name: "Test Template".to_string(),
                description: Some("A test template".to_string()),
                icon: "Bot".to_string(),
                tags: None,
                version: 1,
                is_preset: false,
                is_editable: true,
                is_public: false,
                trigger_config: None,
                nodes: "{}".to_string(),
                edges: "[]".to_string(),
                input_schema: None,
                output_schema: None,
                variables: None,
                error_config: None,
                composite_source: None,
                created_at: 0,
                updated_at: 0,
            },
            vec![],
        );

        let json = export.serialize().unwrap();
        assert!(json.contains("Test Template"));
    }
}
