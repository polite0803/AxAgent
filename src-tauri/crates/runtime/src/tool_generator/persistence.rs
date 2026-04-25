use sea_orm::DatabaseConnection;

use super::types::GeneratedTool;
use axagent_core::error::Result;

/// Persist a generated tool to the database
pub async fn persist_to_db(
    tool: &GeneratedTool,
    db: &DatabaseConnection,
) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let input_schema = serde_json::to_string(&tool.input_schema)
        .map_err(|e| axagent_core::error::AxAgentError::Validation(e.to_string()))?;
    let output_schema = serde_json::to_string(&tool.output_schema)
        .map_err(|e| axagent_core::error::AxAgentError::Validation(e.to_string()))?;
    let implementation = serde_json::to_string(&tool.implementation)
        .map_err(|e| axagent_core::error::AxAgentError::Validation(e.to_string()))?;
    let source_info = serde_json::to_string(&tool.source_info)
        .map_err(|e| axagent_core::error::AxAgentError::Validation(e.to_string()))?;

    axagent_core::repo::generated_tool::insert_generated_tool(
        db,
        &id,
        &tool.tool_name,
        &tool.source_info.original_name,
        &tool.source_info.original_description,
        &input_schema,
        &output_schema,
        &implementation,
        &source_info,
        tool.source_info.generated_at,
    )
    .await
}
