use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Execution entry type for atomic skills
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryType {
    Builtin,
    Mcp,
    Local,
    Plugin,
}

impl std::fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::Builtin => write!(f, "builtin"),
            EntryType::Mcp => write!(f, "mcp"),
            EntryType::Local => write!(f, "local"),
            EntryType::Plugin => write!(f, "plugin"),
        }
    }
}

/// Atomic skill business model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub entry_type: EntryType,
    pub entry_ref: String,
    pub category: String,
    pub tags: Vec<String>,
    pub version: String,
    pub enabled: bool,
    pub source: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Result of executing an atomic skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicSkillExecutionResult {
    pub skill_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub execution_time_ms: u64,
    pub error: Option<AtomicSkillError>,
}

/// Error information from atomic skill execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicSkillError {
    pub error_type: String,
    pub message: String,
}

/// Parameters for creating a new atomic skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAtomicSkillParams {
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub entry_type: EntryType,
    pub entry_ref: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
    pub source: Option<String>,
}

/// Parameters for updating an atomic skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAtomicSkillParams {
    pub name: Option<String>,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub entry_type: Option<EntryType>,
    pub entry_ref: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
    pub source: Option<String>,
}

/// Filter for listing atomic skills
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AtomicSkillFilter {
    pub category: Option<String>,
    pub source: Option<String>,
    pub enabled: Option<bool>,
}

/// Atomic skill node config for workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicSkillNodeConfig {
    pub skill_id: String,
    pub input_mapping: HashMap<String, String>,
    pub output_var: String,
}
