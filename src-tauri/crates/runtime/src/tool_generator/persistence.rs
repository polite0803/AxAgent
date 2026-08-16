// SPDX-License-Identifier: AGPL-3.0-only

use super::types::GeneratedTool;
use axagent_harness::core_error::Result;
use axagent_harness::repositories::InsertGeneratedToolInput;

/// Persist a generated tool to the database
pub async fn persist_to_db(tool: &GeneratedTool) -> Result<()> {
    persist_with_source(tool, None).await
}

/// Persist a runtime-evolution generated tool to the database.
///
/// 在 `source_info` 中写入 `source = "runtime_evolution"` 标记，
/// 供重启时 `load_runtime_evolution_tools` 识别并自动注册回 ToolRegistry。
pub async fn persist_runtime_evolution(tool: &GeneratedTool) -> Result<()> {
    persist_with_source(tool, Some("runtime_evolution")).await
}

async fn persist_with_source(tool: &GeneratedTool, source: Option<&str>) -> Result<()> {
    let input_schema = serde_json::to_string(&tool.input_schema)
        .map_err(|e| axagent_harness::core_error::AxAgentError::Validation(e.to_string()))?;
    let output_schema = serde_json::to_string(&tool.output_schema)
        .map_err(|e| axagent_harness::core_error::AxAgentError::Validation(e.to_string()))?;
    let implementation = serde_json::to_string(&tool.implementation)
        .map_err(|e| axagent_harness::core_error::AxAgentError::Validation(e.to_string()))?;

    let mut source_info = tool.source_info.clone();
    source_info.source = source.map(|s| s.to_string());
    let source_info = serde_json::to_string(&source_info)
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
        .map_err(axagent_harness::core_error::AxAgentError::Internal)
}
