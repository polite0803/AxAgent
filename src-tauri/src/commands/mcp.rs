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
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    const TEST_TIMEOUT_SECS: u64 = 10;

    let server = axagent_core::repo::mcp_server::get_mcp_server(&state.sea_db, &id)
        .await
        .map_err(|e| format!("获取 MCP 服务器配置失败: {e}"))?;

    if !server.enabled {
        return Ok(serde_json::json!({"ok": false, "error": "服务器未启用"}));
    }

    // Builtin servers don't need real connection testing
    if server.transport == "builtin" {
        let tools = axagent_core::repo::mcp_server::list_tools_for_server(&state.sea_db, &id)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(serde_json::json!({
            "ok": true,
            "capabilities": {"tools": true},
            "toolCount": tools.len(),
            "serverInfo": {"name": server.name, "version": "builtin"}
        }));
    }

    let timeout_duration = std::time::Duration::from_secs(TEST_TIMEOUT_SECS);

    let result = tokio::time::timeout(timeout_duration, async {
        match server.transport.as_str() {
            "stdio" => {
                let command = server
                    .command
                    .as_deref()
                    .ok_or_else(|| "stdio 服务器缺少 command 配置".to_string())?;
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

                let tools = axagent_core::mcp_client::discover_tools_stdio(command, &args, &env)
                    .await
                    .map_err(|e| format!("连接失败: {e}"))?;
                Ok::<_, String>(serde_json::json!({
                    "ok": true,
                    "capabilities": {"tools": true},
                    "toolCount": tools.len(),
                    "toolNames": tools.iter().map(|t| &t.name).collect::<Vec<_>>()
                }))
            },
            "http" | "sse" => {
                let endpoint = server
                    .endpoint
                    .as_deref()
                    .ok_or_else(|| format!("{} 服务器缺少 endpoint 配置", server.transport))?;

                let tools = if server.transport == "http" {
                    axagent_core::mcp_client::discover_tools_http(endpoint)
                        .await
                        .map_err(|e| format!("连接失败: {e}"))?
                } else {
                    axagent_core::mcp_client::discover_tools_sse(endpoint)
                        .await
                        .map_err(|e| format!("连接失败: {e}"))?
                };
                Ok::<_, String>(serde_json::json!({
                    "ok": true,
                    "capabilities": {"tools": true},
                    "toolCount": tools.len(),
                    "toolNames": tools.iter().map(|t| &t.name).collect::<Vec<_>>()
                }))
            },
            other => Err(format!("不支持的传输类型: {other}")),
        }
    })
    .await
    .map_err(|_| format!("连接测试超时（{} 秒）", TEST_TIMEOUT_SECS))?;

    result
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
    // 委托给统一的内部实现
    let discovered = discover_mcp_tools_inner(&state, &id).await?;

    // 持久化到 DB（使用原始 DiscoveredTool）
    axagent_core::repo::mcp_server::save_tool_descriptors(&state.sea_db, &id, discovered.clone())
        .await
        .map_err(|e| e.to_string())?;

    // 转换为 ToolDescriptor 返回前端
    let tools: Vec<ToolDescriptor> = discovered
        .into_iter()
        .map(|t| ToolDescriptor {
            id: format!("{}-{}", id, t.name),
            server_id: id.clone(),
            name: t.name,
            description: t.description,
            input_schema_json: t.input_schema.map(|s| s.to_string()),
        })
        .collect();

    Ok(tools)
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
    // Builtin servers: 从 DB 的 tool_descriptors 表读取（已持久化的工具列表）
    if id.starts_with("builtin-") {
        let descriptors = axagent_core::repo::mcp_server::list_tools_for_server(&state.sea_db, id)
            .await
            .map_err(|e| e.to_string())?;
        let tools: Vec<axagent_core::mcp_client::DiscoveredTool> = descriptors
            .into_iter()
            .map(|d| axagent_core::mcp_client::DiscoveredTool {
                name: d.name,
                description: d.description,
                input_schema: d
                    .input_schema_json
                    .and_then(|s| serde_json::from_str(&s).ok()),
            })
            .collect();
        return Ok(tools);
    }

    let server = axagent_core::repo::mcp_server::get_mcp_server(&state.sea_db, id)
        .await
        .map_err(|e| e.to_string())?;

    let timeout_secs = server.discover_timeout_secs.unwrap_or(30) as u64;
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    let command = server.command.as_deref();
    let args: Option<Vec<String>> = server
        .args_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());
    let env: Option<std::collections::HashMap<String, String>> = server
        .env_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());
    let endpoint = server.endpoint.as_deref();

    // 使用统一的发现入口
    let tools = tokio::time::timeout(
        timeout_duration,
        axagent_core::mcp_client::discover_tools_unified(
            &server.transport,
            command,
            args.as_deref(),
            env.as_ref(),
            endpoint,
        ),
    )
    .await
    .map_err(|_| format!("工具发现超时（{} 秒）", timeout_secs))?
    .map_err(|e| e.to_string())?;

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
    let mut servers: Vec<DiscoveredMcpServer> = Vec::new();

    // 1. 从官方注册表获取预置条目
    let official = axagent_tools::mcp::registry::official_registry();
    for entry in official {
        let transport = match entry.transport {
            axagent_tools::mcp::McpTransport::Stdio => "stdio",
            axagent_tools::mcp::McpTransport::Http => "http",
            axagent_tools::mcp::McpTransport::Sse => "sse",
            axagent_tools::mcp::McpTransport::Ws => "ws",
            _ => "stdio",
        };
        servers.push(DiscoveredMcpServer {
            name: entry.name.clone(),
            package_name: entry.command.clone(),
            description: Some(entry.description),
            command: entry.command,
            args: entry.args,
            transport: transport.to_string(),
        });
    }

    // 2. 从 settings.json 中的 mcpServers 配置扫描已安装的服务器
    let config_paths = discover_mcp_config_paths();
    for path in config_paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(mcp_servers) = root.get("mcpServers").and_then(|v| v.as_object()) {
                    for (name, config) in mcp_servers {
                        let command = config
                            .get("command")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let args: Vec<String> = config
                            .get("args")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let transport = config
                            .get("transport")
                            .and_then(|v| v.as_str())
                            .unwrap_or("stdio")
                            .to_string();

                        servers.push(DiscoveredMcpServer {
                            name: name.clone(),
                            package_name: command.clone(),
                            description: config
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            command,
                            args,
                            transport,
                        });
                    }
                }
            }
        }
    }

    Ok(servers)
}

/// 扫描 settings.json 配置文件路径
fn discover_mcp_config_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    paths.push(home.join(".axagent").join("settings.json"));

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(".axagent").join("settings.json"));
        paths.push(cwd.join(".axagent").join("settings.local.json"));
    }

    paths
}
