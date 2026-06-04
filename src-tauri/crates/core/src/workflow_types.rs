//! Workflow type definitions
//!
//! 由 `axagent-harness` 提供定义，本模块仅做 re-export。
//! 附加类型（依赖 `axagent-entities` 的 From impl）在此定义。

pub use axagent_harness::workflow_types::*;

use axagent_entities::workflow_template;

// ── 模板相关的 From impl ──────────────────────
// 需要 entity 类型，不能在 harness 中定义。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateInput {
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Vec<String>,
    pub trigger_config: Option<super::workflow_types::TriggerConfig>,
    pub nodes: Vec<super::workflow_types::WorkflowNode>,
    pub edges: Vec<super::workflow_types::WorkflowEdge>,
    pub input_schema: Option<super::workflow_types::JsonSchema>,
    pub output_schema: Option<super::workflow_types::JsonSchema>,
    pub variables: Vec<super::workflow_types::Variable>,
    pub error_config: Option<super::workflow_types::ErrorConfig>,
    pub tool_defs: Option<Vec<super::workflow_types::RhaiToolDef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Vec<String>,
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    pub trigger_config: Option<super::workflow_types::TriggerConfig>,
    pub nodes: Vec<super::workflow_types::WorkflowNode>,
    pub edges: Vec<super::workflow_types::WorkflowEdge>,
    pub input_schema: Option<super::workflow_types::JsonSchema>,
    pub output_schema: Option<super::workflow_types::JsonSchema>,
    pub variables: Vec<super::workflow_types::Variable>,
    pub error_config: Option<super::workflow_types::ErrorConfig>,
    pub tool_defs: Option<Vec<super::workflow_types::RhaiToolDef>>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<workflow_template::Model> for WorkflowTemplateResponse {
    fn from(model: workflow_template::Model) -> Self {
        let tags: Vec<String> = model
            .tags
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default();

        let trigger_config: Option<super::workflow_types::TriggerConfig> = model
            .trigger_config
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok());

        let nodes: Vec<super::workflow_types::WorkflowNode> =
            serde_json::from_str(&model.nodes).unwrap_or_default();
        let edges: Vec<super::workflow_types::WorkflowEdge> =
            serde_json::from_str(&model.edges).unwrap_or_default();
        let input_schema: Option<super::workflow_types::JsonSchema> = model
            .input_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        let output_schema: Option<super::workflow_types::JsonSchema> = model
            .output_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        let variables_vec: Vec<super::workflow_types::Variable> = model
            .variables
            .as_ref()
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();
        let error_config: Option<super::workflow_types::ErrorConfig> = model
            .error_config
            .as_ref()
            .and_then(|e| serde_json::from_str(e).ok());

        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            icon: model.icon,
            tags,
            version: model.version,
            is_preset: model.is_preset,
            is_editable: model.is_editable,
            is_public: model.is_public,
            trigger_config,
            nodes,
            edges,
            input_schema,
            output_schema,
            variables: variables_vec,
            error_config,
            tool_defs: None,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

