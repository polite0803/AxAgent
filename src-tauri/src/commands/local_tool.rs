use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// 单个本地工具信息（前端 DTO）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolInfo {
    #[serde(rename = "toolName")]
    pub tool_name: String,
    pub description: String,
}

/// 本地工具组信息（前端 DTO）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolGroupInfo {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "groupName")]
    pub group_name: String,
    pub enabled: bool,
    pub tools: Vec<LocalToolInfo>,
}

fn to_local_group(g: axagent_tools::registry::ToolGroupInfo) -> LocalToolGroupInfo {
    LocalToolGroupInfo {
        group_id: g.group_id,
        group_name: g.group_name,
        enabled: g.enabled,
        tools: g
            .tools
            .into_iter()
            .map(|t| LocalToolInfo {
                tool_name: t.name,
                description: t.description,
            })
            .collect(),
    }
}

#[tauri::command]
pub async fn list_local_tools(
    state: State<'_, AppState>,
) -> Result<Vec<LocalToolGroupInfo>, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(&state.sea_db).await;
    Ok(registry
        .get_tool_groups()
        .into_iter()
        .map(to_local_group)
        .collect())
}

#[tauri::command]
pub async fn toggle_local_tool(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<LocalToolGroupInfo, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(&state.sea_db).await;
    registry
        .toggle_group(&state.sea_db, &group_id)
        .await
        .map_err(|e| e.to_string())?;

    let groups = registry.get_tool_groups();
    let group = groups
        .into_iter()
        .find(|g| g.group_id == group_id)
        .ok_or_else(|| format!("Group '{}' not found", group_id))?;
    Ok(to_local_group(group))
}
