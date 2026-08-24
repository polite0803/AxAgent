// SPDX-License-Identifier: AGPL-3.0-only

use axagent_agent_macro::agent_command;

use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::AppState;

/// Generated tool info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedToolInfo {
    pub id: String,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    #[serde(rename = "originalName")]
    pub original_name: String,
    #[serde(rename = "originalDescription")]
    pub original_description: String,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
}

#[agent_command(domain = tool, safety = Safe, call_mode = StateOnly, description = "列出所有已生成的工具")]
#[tauri::command]
pub async fn list_generated_tools(
    state: State<'_, AppState>,
) -> Result<Vec<GeneratedToolInfo>, String> {
    let db: &DatabaseConnection = state.harness.db();
    let tools = axagent_dao::repo::generated_tool::list_generated_tools(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(tools
        .into_iter()
        .map(|t| GeneratedToolInfo {
            id: t.id,
            tool_name: t.tool_name,
            original_name: t.original_name,
            original_description: t.original_description,
            created_at: t.created_at,
        })
        .collect())
}

#[agent_command(domain = tool, safety = Dangerous, call_mode = StateInput, description = "删除指定的已生成工具")]
#[tauri::command]
pub async fn delete_generated_tool(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let db: &DatabaseConnection = state.harness.db();
    axagent_dao::repo::generated_tool::delete_generated_tool(db, &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
