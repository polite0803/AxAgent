use crate::AppState;
use axagent_core::types::*;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

#[tauri::command]
pub async fn list_mcp_servers(state: State<'_, AppState>) -> Result<Vec<McpServer>, String> {
    axagent_core::repo::mcp_server::list_mcp_servers(&state.sea_db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_mcp_server(
    state: State<'_, AppState>,
    input: CreateMcpServerInput,
) -> Result<McpServer, String> {
    axagent_core::repo::mcp_server::create_mcp_server(&state.sea_db, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_mcp_server(
    state: State<'_, AppState>,
    id: String,
    input: CreateMcpServerInput,
) -> Result<McpServer, String> {
    axagent_core::repo::mcp_server::update_mcp_server(&state.sea_db, &id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_mcp_server(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_core::repo::mcp_server::delete_mcp_server(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_mcp_server(
    _state: State<'_, AppState>,
    _id: String,
) -> Result<serde_json::Value, String> {
    // Mock implementation — return success with capabilities
    Ok(serde_json::json!({"ok": true, "capabilities": ["tools"]}))
}

#[tauri::command]
pub async fn list_mcp_tools(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<Vec<ToolDescriptor>, String> {
    axagent_core::repo::mcp_server::list_tools_for_server(&state.sea_db, &server_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn discover_mcp_tools(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<ToolDescriptor>, String> {
    if id.starts_with("builtin-") {
        let registry = state.local_tool_registry.lock().await;
        let groups = registry.get_tool_groups();
        if let Some(group) = groups.into_iter().find(|g| g.group_id == id) {
            let tools: Vec<ToolDescriptor> = group
                .tools
                .into_iter()
                .map(|t| ToolDescriptor {
                    id: format!("{}-{}", id, t.tool_name),
                    server_id: id.clone(),
                    name: t.tool_name,
                    description: Some(t.description),
                    input_schema_json: Some(t.input_schema.to_string()),
                })
                .collect();
            return Ok(tools);
        }
        return Err(format!("Builtin server '{}' not found", id));
    }

    let server = axagent_core::repo::mcp_server::get_mcp_server(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())?;

    let timeout_secs = server.discover_timeout_secs.unwrap_or(30) as u64;
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    let tools = match server.transport.as_str() {
        "stdio" => {
            let command = server
                .command
                .as_deref()
                .ok_or_else(|| "stdio server has no command configured".to_string())?;
            let args: Vec<String> = server
                .args_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            let env: std::collections::HashMap<String, String> = server
                .env_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            tokio::time::timeout(
                timeout_duration,
                axagent_core::mcp_client::discover_tools_stdio(command, &args, &env),
            )
            .await
            .map_err(|_| format!("Tool discovery timed out after {}s", timeout_secs))?
            .map_err(|e| e.to_string())?
        }
        "http" => {
            let endpoint = server
                .endpoint
                .as_deref()
                .ok_or_else(|| "HTTP server has no endpoint configured".to_string())?;
            tokio::time::timeout(
                timeout_duration,
                axagent_core::mcp_client::discover_tools_http(endpoint),
            )
            .await
            .map_err(|_| format!("Tool discovery timed out after {}s", timeout_secs))?
            .map_err(|e| e.to_string())?
        }
        "sse" => {
            let endpoint = server
                .endpoint
                .as_deref()
                .ok_or_else(|| "SSE server has no endpoint configured".to_string())?;
            tokio::time::timeout(
                timeout_duration,
                axagent_core::mcp_client::discover_tools_sse(endpoint),
            )
            .await
            .map_err(|_| format!("Tool discovery timed out after {}s", timeout_secs))?
            .map_err(|e| e.to_string())?
        }
        other => return Err(format!("Unsupported transport: {}", other)),
    };

    axagent_core::repo::mcp_server::save_tool_descriptors(&state.sea_db, &id, tools)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_tool_executions(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<ToolExecution>, String> {
    axagent_core::repo::tool_execution::list_tool_executions(&state.sea_db, &conversation_id)
        .await
        .map_err(|e| e.to_string())
}

/// Hot-reload an MCP server's tools into the active agent session.
/// Discovers tools from the server and emits an event so the frontend
/// can update its tool list without restarting the application.
#[tauri::command]
pub async fn hot_reload_mcp_server(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    // 1. Discover tools from the server
    let tools = discover_mcp_tools_inner(&state, &id).await?;

    // 2. Save discovered tools to DB
    axagent_core::repo::mcp_server::save_tool_descriptors(&state.sea_db, &id, tools.clone())
        .await
        .map_err(|e| e.to_string())?;

    // 3. Evict any cached connections for this server in the MCP pool
    //    so the next tool call will establish a fresh connection
    {
        let pool = axagent_core::mcp_client::global_mcp_pool();
        pool.evict_by_server_id(&id);
    }

    // 4. Emit event so frontend can update its tool list
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    let _ = app.emit(
        "mcp-server-hot-reloaded",
        serde_json::json!({
            "serverId": id,
            "toolCount": tools.len(),
            "toolNames": tool_names,
        }),
    );

    Ok(serde_json::json!({
        "ok": true,
        "serverId": id,
        "toolCount": tools.len(),
    }))
}

/// Inner implementation of tool discovery (shared between discover_mcp_tools and hot_reload_mcp_server).
async fn discover_mcp_tools_inner(
    state: &AppState,
    id: &str,
) -> Result<Vec<axagent_core::mcp_client::DiscoveredTool>, String> {
    if id.starts_with("builtin-") {
        let registry = state.local_tool_registry.lock().await;
        let groups = registry.get_tool_groups();
        if let Some(group) = groups.into_iter().find(|g| g.group_id == id) {
            let tools: Vec<axagent_core::mcp_client::DiscoveredTool> = group
                .tools
                .into_iter()
                .map(|t| axagent_core::mcp_client::DiscoveredTool {
                    name: t.tool_name,
                    description: Some(t.description),
                    input_schema: Some(t.input_schema),
                })
                .collect();
            return Ok(tools);
        }
        return Err(format!("Builtin server '{}' not found", id));
    }

    let server = axagent_core::repo::mcp_server::get_mcp_server(&state.sea_db, id)
        .await
        .map_err(|e| e.to_string())?;

    let timeout_secs = server.discover_timeout_secs.unwrap_or(30) as u64;
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    let tools = match server.transport.as_str() {
        "stdio" => {
            let command = server
                .command
                .as_deref()
                .ok_or_else(|| "stdio server has no command configured".to_string())?;
            let args: Vec<String> = server
                .args_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            let env: std::collections::HashMap<String, String> = server
                .env_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            tokio::time::timeout(
                timeout_duration,
                axagent_core::mcp_client::discover_tools_stdio(command, &args, &env),
            )
            .await
            .map_err(|_| format!("Tool discovery timed out after {}s", timeout_secs))?
            .map_err(|e| e.to_string())?
        }
        "http" => {
            let endpoint = server
                .endpoint
                .as_deref()
                .ok_or_else(|| "HTTP server has no endpoint configured".to_string())?;
            tokio::time::timeout(
                timeout_duration,
                axagent_core::mcp_client::discover_tools_http(endpoint),
            )
            .await
            .map_err(|_| format!("Tool discovery timed out after {}s", timeout_secs))?
            .map_err(|e| e.to_string())?
        }
        "sse" => {
            let endpoint = server
                .endpoint
                .as_deref()
                .ok_or_else(|| "SSE server has no endpoint configured".to_string())?;
            tokio::time::timeout(
                timeout_duration,
                axagent_core::mcp_client::discover_tools_sse(endpoint),
            )
            .await
            .map_err(|_| format!("Tool discovery timed out after {}s", timeout_secs))?
            .map_err(|e| e.to_string())?
        }
        other => return Err(format!("Unsupported transport: {}", other)),
    };

    Ok(tools)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredMcpServer {
    pub name: String,
    pub package_name: String,
    pub description: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub transport: String,
}

#[tauri::command]
pub async fn discover_available_mcp_servers() -> Result<Vec<DiscoveredMcpServer>, String> {
    Ok(Vec::new())
}
