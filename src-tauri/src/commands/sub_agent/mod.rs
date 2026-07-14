// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::agent as agent_err;
use serde_json::Value;
use tauri::State;

/// 列出注册表中所有子代理
#[tauri::command]
pub async fn sub_agent_list(app_state: State<'_, AppState>) -> Result<Vec<Value>, String> {
    let registry = app_state.sub_agent_registry.read().await;
    let agents = registry.list_all();
    Ok(agents.iter().filter_map(|a| serde_json::to_value(a).ok()).collect())
}

/// 按 ID 获取指定子代理
#[tauri::command]
pub async fn sub_agent_get(
    app_state: State<'_, AppState>,
    agent_id: String,
) -> Result<Value, String> {
    let registry = app_state.sub_agent_registry.read().await;
    let agent = registry.get(&agent_id).ok_or_else(|| ErrorResponse::err(agent_err::NOT_FOUND))?;
    serde_json::to_value(agent).map_err(|e| e.to_string())
}

/// 获取父代理的子代理
#[tauri::command]
pub async fn sub_agent_get_children(
    app_state: State<'_, AppState>,
    parent_id: String,
) -> Result<Vec<Value>, String> {
    let registry = app_state.sub_agent_registry.read().await;
    let children = registry.get_children(&parent_id);
    Ok(children.iter().filter_map(|c| serde_json::to_value(c).ok()).collect())
}

/// 获取代理的待处理消息
#[tauri::command]
pub async fn sub_agent_get_messages(
    app_state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<Value>, String> {
    let registry = app_state.sub_agent_registry.read().await;
    let messages = registry.message_bus().peek_all(&agent_id);
    Ok(messages.iter().filter_map(|m| serde_json::to_value(m).ok()).collect())
}