// SPDX-License-Identifier: AGPL-3.0-only

use super::types::GeneratedTool;
use axagent_harness::core_error::Result;
use axagent_harness::repositories::InsertGeneratedToolInput;

/// Persist a generated tool to the database
pub async fn persist_to_db(tool: &GeneratedTool) -> Result<()> {
    let input_schema = serde_json::to_string(&tool.input_schema)
        .map_err(|e| axagent_harness::core_error::AxAgentError::Validation(e.to_string()))?;
    let output_schema = serde_json::to_string(&tool.output_schema)
        .map_err(|e| axagent_harness::core_error::AxAgentError::Validation(e.to_string()))?;
    let implementation = serde_json::to_string(&tool.implementation)
        .map_err(|e| axagent_harness::core_error::AxAgentError::Validation(e.to_string()))?;
    let source_info = serde_json::to_string(&tool.source_info)
        .map_err(|e| axagent_harness::core_error::AxAgentError::Validation(e.to_string()))?;

    axagent_harness::repositories::generated_tool_repository()
        .insert_generated_tool(InsertGeneratedToolInput {
            tool_name: tool.tool_name.clone(),
            original_name: tool.source_info.original_name.clone(),
            original_description: tool.source_info.original_description.clone(),
            input_schema,
            output_schema,
            implementation,
            source_info,
            created_at: tool.source_info.generated_at,
        })
        .await
        .map_err(|e| axagent_harness::core_error::AxAgentError::Internal(e))
}
