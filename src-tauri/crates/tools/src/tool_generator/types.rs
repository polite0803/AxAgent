use serde::{Deserialize, Serialize};

/// Input for tool generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGenerationInput {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
}

/// Generated tool implementation method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GeneratedToolImplementation {
    /// Prompt template: executed by LLM at runtime
    PromptTemplate {
        template: String,
        model: Option<String>,
        temperature: Option<f32>,
    },
    /// Script code: future extension
    Script { language: String, code: String },
}

/// A generated tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTool {
    pub tool_name: String,
    pub implementation: GeneratedToolImplementation,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub source_info: GeneratedToolSourceInfo,
}

/// Source traceability info for a generated tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedToolSourceInfo {
    pub original_name: String,
    pub original_description: String,
    pub generation_method: String,
    pub agent_model: Option<String>,
    pub generated_at: i64,
}
