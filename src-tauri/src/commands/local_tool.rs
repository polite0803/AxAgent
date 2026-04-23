use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Information about a single local tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolInfo {
    #[serde(rename = "toolName")]
    pub tool_name: String,
    pub description: String,
}

/// Information about a local tool group (for UI display).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolGroupInfo {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "groupName")]
    pub group_name: String,
    pub enabled: bool,
    pub tools: Vec<LocalToolInfo>,
}

#[tauri::command]
pub async fn list_local_tools(
    state: State<'_, AppState>,
) -> Result<Vec<LocalToolGroupInfo>, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(&state.sea_db).await;

    let groups = registry.get_tool_groups();
    let result: Vec<LocalToolGroupInfo> = groups
        .into_iter()
        .map(|g| LocalToolGroupInfo {
            group_id: g.group_id,
            group_name: g.group_name,
            enabled: g.enabled,
            tools: g
                .tools
                .into_iter()
                .map(|t| LocalToolInfo {
                    tool_name: t.tool_name,
                    description: t.description,
                })
                .collect(),
        })
        .collect();

    Ok(result)
}

#[tauri::command]
pub async fn toggle_local_tool(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<LocalToolGroupInfo, String> {
    let mut registry = state.local_tool_registry.lock().await;
    registry.load_enabled_state(&state.sea_db).await;

    let new_enabled = registry
        .toggle_group(&state.sea_db, &group_id)
        .await
        .map_err(|e| e.to_string())?;

    // Return the updated group info
    let groups = registry.get_tool_groups();
    let group = groups
        .into_iter()
        .find(|g| g.group_id == group_id)
        .ok_or_else(|| format!("Group '{}' not found", group_id))?;

    Ok(LocalToolGroupInfo {
        group_id: group.group_id,
        group_name: group.group_name,
        enabled: new_enabled,
        tools: group
            .tools
            .into_iter()
            .map(|t| LocalToolInfo {
                tool_name: t.tool_name,
                description: t.description,
            })
            .collect(),
    })
}
