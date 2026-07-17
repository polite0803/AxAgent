// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::mcp as mcp_err;
use axagent_harness::types::*;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

#[tauri::command]
pub async fn list_mcp_servers(state: State<'_, AppState>) -> Result<Vec<McpServer>, String> {
    axagent_dao::repo::mcp_server::list_mcp_servers(state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn create_mcp_server(
    state: State<'_, AppState>,
    input: CreateMcpServerInput,
) -> Result<McpServer, String> {
    axagent_dao::repo::mcp_server::create_mcp_server(state.harness.db(), input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn update_mcp_server(
    state: State<'_, AppState>,
    id: String,
    input: CreateMcpServerInput,
) -> Result<McpServer, String> {
    axagent_dao::repo::mcp_server::update_mcp_server(state.harness.db(), &id, input).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

#[tauri::command]
pub async fn delete_mcp_server(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_dao::repo::mcp_server::delete_mcp_server(state.harness.db(), &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[tauri::command]
pub async fn test_mcp_server(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    const TEST_TIMEOUT_SECS: u64 = 10;

    let server = axagent_dao::repo::mcp_server::get_mcp_server(state.harness.db(), &id)
        .await
        .map_err(|e| format!("获取 MCP 服务器配置失败: {e}"))?;

    if !server.enabled {
        let err = ErrorResponse::new(mcp_err::SERVER_NOT_ENABLED);
        return Ok(
            serde_json::json!({"ok": false, "error": serde_json::to_string(&err).unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))}),
        );
    }

    // Builtin servers don't need real connection testing
    if server.transport == "builtin" {
        let tools = axagent_dao::repo::mcp_server::list_tools_for_server(state.harness.db(), &id)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        return Ok(serde_json::json!({
            "ok": true,
            "capabilities": {"tools": true},
            "toolCount": tools.len(),
            "serverInfo": {"name": server.name, "version": "builtin"}
        }));
    }

    let timeout_duration = std::time::Duration::from_secs(TEST_TIMEOUT_SECS);

    tokio::time::timeout(timeout_duration, async {
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

                let tools = axagent_mcp::mcp_client::discover_tools_stdio(command, &args, &env)
                    .await
                    .map_err(|e| {
                        serde_json::to_string(
                            &ErrorResponse::new(mcp_err::CONNECT_FAILED).with_detail(e.to_string()),
                        )
                        .unwrap_or_else(|e| {
                            format!("{{\"error\":\"serialization failed: {}\"}}", e)
                        })
                    })?;
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

                // 解析 OAuth 凭据（持久化存储 → 环境变量），无则按匿名连接
                let auth = axagent_mcp::mcp_client::resolve_oauth_header(Some(&id)).await;

                let tools = if server.transport == "http" {
                    axagent_mcp::mcp_client::discover_tools_http(endpoint, auth.as_deref())
                        .await
                        .map_err(|e| {
                            serde_json::to_string(
                                &ErrorResponse::new(mcp_err::CONNECT_FAILED)
                                    .with_detail(e.to_string()),
                            )
                            .unwrap_or_else(|e| {
                                format!("{{\"error\":\"serialization failed: {}\"}}", e)
                            })
                        })?
                } else {
                    axagent_mcp::mcp_client::discover_tools_sse(endpoint, auth.as_deref())
                        .await
                        .map_err(|e| {
                            serde_json::to_string(
                                &ErrorResponse::new(mcp_err::CONNECT_FAILED)
                                    .with_detail(e.to_string()),
                            )
                            .unwrap_or_else(|e| {
                                format!("{{\"error\":\"serialization failed: {}\"}}", e)
                            })
                        })?
                };
                Ok::<_, String>(serde_json::json!({
                    "ok": true,
                    "capabilities": {"tools": true},
                    "toolCount": tools.len(),
                    "toolNames": tools.iter().map(|t| &t.name).collect::<Vec<_>>()
                }))
            },
            other => Err(serde_json::to_string(
                &ErrorResponse::new(mcp_err::TRANSPORT_UNSUPPORTED).with_detail(other),
            )
            .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))),
        }
    })
    .await
    .map_err(|_| {
        serde_json::to_string(
            &ErrorResponse::new(mcp_err::TIMEOUT)
                .with_detail(format!("连接测试超时（{} 秒）", TEST_TIMEOUT_SECS)),
        )
        .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))
    })?
}

#[tauri::command]
pub async fn list_mcp_tools(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<Vec<ToolDescriptor>, String> {
    axagent_dao::repo::mcp_server::list_tools_for_server(state.harness.db(), &server_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

#[tauri::command]
pub async fn discover_mcp_tools(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<ToolDescriptor>, String> {
    // 委托给统一的内部实现
    let discovered = discover_mcp_tools_inner(&state, &id).await?;

    // 持久化到 DB（使用原始 DiscoveredTool）
    axagent_dao::repo::mcp_server::save_tool_descriptors(
        state.harness.db(),
        &id,
        discovered.clone(),
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 转换为 ToolDescriptor 返回前端
    let tools: Vec<ToolDescriptor> = discovered
        .into_iter()
        .map(|t| ToolDescriptor {
            id: format!("{}-{}", id, t.name),
            server_id: id.clone(),
            name: t.name,
            description: t.description,
            input_schema_json: t.input_schema.map(|s| s.to_string()),
            ..Default::default()
        })
        .collect();

    Ok(tools)
}

#[tauri::command]
pub async fn list_tool_executions(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<ToolExecution>, String> {
    axagent_dao::repo::tool_execution::list_tool_executions(state.harness.db(), &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
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
    axagent_dao::repo::mcp_server::save_tool_descriptors(state.harness.db(), &id, tools.clone())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    // 3. Evict any cached connections for this server in the MCP pool
    //    so the next tool call will establish a fresh connection
    {
        #[cfg(not(target_os = "android"))]
        {
            let pool = axagent_mcp::mcp_client::global_mcp_pool();
            pool.evict_by_server_id(&id);
        }
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
) -> Result<Vec<axagent_mcp::mcp_client::DiscoveredTool>, String> {
    // Builtin servers: 从 DB 的 tool_descriptors 表读取（已持久化的工具列表）
    if id.starts_with("builtin-") {
        let descriptors =
            axagent_dao::repo::mcp_server::list_tools_for_server(state.harness.db(), id)
                .await
                .map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
        let tools: Vec<axagent_mcp::mcp_client::DiscoveredTool> = descriptors
            .into_iter()
            .map(|d| axagent_mcp::mcp_client::DiscoveredTool {
                name: d.name,
                description: d.description,
                input_schema: d.input_schema_json.and_then(|s| serde_json::from_str(&s).ok()),
            })
            .collect();
        return Ok(tools);
    }

    let server = axagent_dao::repo::mcp_server::get_mcp_server(state.harness.db(), id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let timeout_secs = server.discover_timeout_secs.unwrap_or(30) as u64;
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    let command = server.command.as_deref();
    let args: Option<Vec<String>> =
        server.args_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let env: Option<std::collections::HashMap<String, String>> =
        server.env_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let endpoint = server.endpoint.as_deref();

    // 使用统一的发现入口（携带 server_id 以注入 OAuth 凭据）
    let tools = tokio::time::timeout(
        timeout_duration,
        axagent_mcp::mcp_client::discover_tools_unified(
            &server.transport,
            command,
            args.as_deref(),
            env.as_ref(),
            endpoint,
            Some(id),
        ),
    )
    .await
    .map_err(|_| {
        serde_json::to_string(
            &ErrorResponse::new(mcp_err::TOOL_DISCOVERY_TIMEOUT)
                .with_detail(format!("工具发现超时（{} 秒）", timeout_secs)),
        )
        .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))
    })?
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

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
                                arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
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

/// 手动为 MCP 服务器写入 OAuth 凭据（适用于使用静态 Bearer Token /
/// Personal Access Token 的服务器，或外部已完成授权后回填）。
#[tauri::command]
pub async fn store_mcp_oauth_token(
    server_id: String,
    token: String,
    refresh_token: Option<String>,
    expires_in_secs: Option<u64>,
    scopes: Option<Vec<String>>,
    token_endpoint: Option<String>,
    client_id: Option<String>,
) -> Result<(), String> {
    let store = axagent_mcp::mcp_oauth::McpOAuthStore::try_global()
        .ok_or_else(|| "MCP OAuth 存储尚未初始化".to_string())?;
    let expires_at = expires_in_secs.map(|secs| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + secs
    });
    store
        .store(
            &server_id,
            axagent_mcp::mcp_oauth::McpOAuthCredentials {
                access_token: token,
                refresh_token,
                expires_at,
                scopes: scopes.unwrap_or_default(),
                token_endpoint,
                client_id,
            },
        )
        .await;
    Ok(())
}

/// 为受保护的 MCP 服务器发起 OAuth 2.1 (PKCE) 授权，返回需用户在浏览器打开的 URL。
#[tauri::command]
pub async fn begin_mcp_oauth_authorization(
    server_id: String,
    authorization_endpoint: String,
    token_endpoint: String,
    client_id: String,
    redirect_uri: String,
    scopes: Vec<String>,
) -> Result<String, String> {
    axagent_mcp::mcp_oauth::begin_oauth_authorization(
        &server_id,
        &authorization_endpoint,
        &token_endpoint,
        &client_id,
        &redirect_uri,
        &scopes,
    )
}

/// 用授权码（浏览器回调所得）兑换并持久化 OAuth token。
#[tauri::command]
pub async fn complete_mcp_oauth_authorization(
    server_id: String,
    code: String,
) -> Result<(), String> {
    axagent_mcp::mcp_oauth::complete_oauth_authorization(&server_id, &code).await
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

// ── H1: MCP Resources support ──

/// List all resources from an MCP server.
#[tauri::command]
pub async fn list_mcp_resources(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<axagent_harness::mcp_types::McpResource>, String> {
    let server = axagent_dao::repo::mcp_server::get_mcp_server(state.harness.db(), &id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let timeout_secs = server.discover_timeout_secs.unwrap_or(30) as u64;
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    let command = server.command.as_deref();
    let args: Option<Vec<String>> =
        server.args_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let env: Option<std::collections::HashMap<String, String>> =
        server.env_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let endpoint = server.endpoint.as_deref();

    tokio::time::timeout(
        timeout_duration,
        axagent_mcp::mcp_client::list_resources_unified(
            &server.transport,
            command,
            args.as_deref(),
            env.as_ref(),
            endpoint,
            Some(&id),
        ),
    )
    .await
    .map_err(|_| {
        serde_json::to_string(
            &ErrorResponse::new(mcp_err::TOOL_DISCOVERY_TIMEOUT)
                .with_detail(format!("资源列表查询超时（{} 秒）", timeout_secs)),
        )
        .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))
    })?
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// Read a specific resource from an MCP server.
#[tauri::command]
pub async fn read_mcp_resource(
    state: State<'_, AppState>,
    id: String,
    uri: String,
) -> Result<Vec<axagent_harness::mcp_types::McpResourceContent>, String> {
    let server = axagent_dao::repo::mcp_server::get_mcp_server(state.harness.db(), &id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let timeout_duration = std::time::Duration::from_secs(30);

    let command = server.command.as_deref();
    let args: Option<Vec<String>> =
        server.args_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let env: Option<std::collections::HashMap<String, String>> =
        server.env_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let endpoint = server.endpoint.as_deref();

    tokio::time::timeout(
        timeout_duration,
        axagent_mcp::mcp_client::read_resource_unified(
            &server.transport,
            command,
            args.as_deref(),
            env.as_ref(),
            endpoint,
            &uri,
            Some(&id),
        ),
    )
    .await
    .map_err(|_| "MCP 资源读取超时（30 秒）".to_string())?
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── H1: MCP Prompts support ──

/// List all prompts from an MCP server.
#[tauri::command]
pub async fn list_mcp_prompts(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<axagent_harness::mcp_types::McpPrompt>, String> {
    let server = axagent_dao::repo::mcp_server::get_mcp_server(state.harness.db(), &id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let timeout_secs = server.discover_timeout_secs.unwrap_or(30) as u64;
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    let command = server.command.as_deref();
    let args: Option<Vec<String>> =
        server.args_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let env: Option<std::collections::HashMap<String, String>> =
        server.env_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let endpoint = server.endpoint.as_deref();

    tokio::time::timeout(
        timeout_duration,
        axagent_mcp::mcp_client::list_prompts_unified(
            &server.transport,
            command,
            args.as_deref(),
            env.as_ref(),
            endpoint,
            Some(&id),
        ),
    )
    .await
    .map_err(|_| {
        serde_json::to_string(
            &ErrorResponse::new(mcp_err::TOOL_DISCOVERY_TIMEOUT)
                .with_detail(format!("提示词列表查询超时（{} 秒）", timeout_secs)),
        )
        .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))
    })?
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// Render a prompt from an MCP server with the given arguments.
#[tauri::command]
pub async fn get_mcp_prompt(
    state: State<'_, AppState>,
    id: String,
    name: String,
    args: serde_json::Value,
) -> Result<axagent_harness::mcp_types::McpPromptResult, String> {
    let server = axagent_dao::repo::mcp_server::get_mcp_server(state.harness.db(), &id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let timeout_duration = std::time::Duration::from_secs(30);

    let command = server.command.as_deref();
    let args_list: Option<Vec<String>> =
        server.args_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let env: Option<std::collections::HashMap<String, String>> =
        server.env_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let endpoint = server.endpoint.as_deref();

    tokio::time::timeout(
        timeout_duration,
        axagent_mcp::mcp_client::get_prompt_unified(
            &server.transport,
            command,
            args_list.as_deref(),
            env.as_ref(),
            endpoint,
            &name,
            args,
            Some(&id),
        ),
    )
    .await
    .map_err(|_| "MCP 提示词渲染超时（30 秒）".to_string())?
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
