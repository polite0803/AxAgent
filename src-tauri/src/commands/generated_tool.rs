use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::AppState;

/// Generated tool info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[tauri::command]
pub async fn list_generated_tools(
    state: State<'_, AppState>,
) -> Result<Vec<GeneratedToolInfo>, String> {
    let db: &DatabaseConnection = &state.sea_db;
    let tools = axagent_core::repo::generated_tool::list_generated_tools(db)
        .await
        .map_err(|e| e.to_string())?;

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

#[tauri::command]
pub async fn delete_generated_tool(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let db: &DatabaseConnection = &state.sea_db;
    axagent_core::repo::generated_tool::delete_generated_tool(db, &id)
        .await
        .map_err(|e| e.to_string())
}
