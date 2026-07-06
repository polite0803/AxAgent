// SPDX-License-Identifier: AGPL-3.0-only

//! MCP 代理路由：通过 Gateway API 暴露 MCP 工具调用。
//!
//! 复用 `axagent-mcp` crate 的 `call_tool_unified` / `discover_tools_unified`，
//! 让外部 HTTP 客户端无需经过 Tauri 即可调用 MCP 工具。

use std::collections::HashMap;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use sea_orm::EntityTrait;
use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};
use serde::Deserialize;
use serde_json::json;

use axagent_mcp::mcp_client;

use crate::auth::AuthenticatedKey;
use crate::server::GatewayAppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct McpCallToolRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
pub struct McpServerInfo {
    pub id: String,
    pub name: String,
    pub transport: String,
}

// ---------------------------------------------------------------------------
// MCP server 查询
// ---------------------------------------------------------------------------

/// 从数据库查询启用的 MCP server 列表（使用 SeaORM Entity）。
async fn query_enabled_servers(
    db: &sea_orm::DatabaseConnection,
) -> Result<Vec<axagent_entities::mcp_servers::Model>, String> {
    use axagent_entities::mcp_servers;

    mcp_servers::Entity::find()
        .filter(mcp_servers::Column::Enabled.eq(1))
        .order_by_asc(mcp_servers::Column::Name)
        .all(db)
        .await
        .map_err(|e| format!("Failed to query MCP servers: {e}"))
}

/// 从数据库查询单个 MCP server（使用 SeaORM Entity）。
async fn query_server_by_id(
    db: &sea_orm::DatabaseConnection,
    server_id: &str,
) -> Result<Option<axagent_entities::mcp_servers::Model>, String> {
    use axagent_entities::mcp_servers;

    mcp_servers::Entity::find_by_id(server_id.to_string())
        .one(db)
        .await
        .map_err(|e| format!("Failed to query MCP server: {e}"))
}

#[allow(clippy::type_complexity)]
fn build_call_args(
    server: &axagent_entities::mcp_servers::Model,
) -> (Option<&str>, Option<Vec<String>>, HashMap<String, String>, Option<&str>) {
    let command = server.command.as_deref();
    let args: Option<Vec<String>> = server
        .args_json
        .as_ref()
        .and_then(|j| serde_json::from_str(j).ok());
    let env: HashMap<String, String> = server
        .env_json
        .as_ref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    let endpoint = server.endpoint.as_deref();
    (command, args, env, endpoint)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /v1/mcp/servers — 列出可用的 MCP 服务器。
pub async fn list_mcp_servers(
    State(state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    match query_enabled_servers(&state.db).await {
        Ok(servers) => {
            let info: Vec<McpServerInfo> = servers
                .into_iter()
                .map(|s| McpServerInfo {
                    id: s.id,
                    name: s.name,
                    transport: s.transport,
                })
                .collect();
            (StatusCode::OK, Json(json!(info))).into_response()
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))).into_response(),
    }
}

/// POST /v1/mcp/servers/{server_id}/tools/list — 发现指定服务器的工具列表。
pub async fn discover_mcp_tools(
    State(state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(server_id): Path<String>,
) -> impl IntoResponse {
    let server = match query_server_by_id(&state.db, &server_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("MCP server not found: {server_id}") })),
            )
                .into_response();
        },
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e })))
                .into_response();
        },
    };

    let (command, args, env, endpoint) = build_call_args(&server);
    let args_ref: Option<&[String]> = args.as_deref();

    match mcp_client::discover_tools_unified(
        &server.transport,
        command,
        args_ref,
        Some(&env),
        endpoint,
    )
    .await
    {
        Ok(tools) => (StatusCode::OK, Json(json!(tools))).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Failed to discover tools: {e}") })),
        )
            .into_response(),
    }
}

/// POST /v1/mcp/servers/{server_id}/tools/call — 执行指定服务器的工具调用。
pub async fn call_mcp_tool(
    State(state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(server_id): Path<String>,
    Json(body): Json<McpCallToolRequest>,
) -> impl IntoResponse {
    let server = match query_server_by_id(&state.db, &server_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("MCP server not found: {server_id}") })),
            )
                .into_response();
        },
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e })))
                .into_response();
        },
    };

    let (command, args, env, endpoint) = build_call_args(&server);
    let args_ref: Option<&[String]> = args.as_deref();

    match mcp_client::call_tool_unified(
        &server.transport,
        command,
        args_ref,
        Some(&env),
        endpoint,
        &body.tool_name,
        body.arguments,
    )
    .await
    {
        Ok(result) => {
            // McpToolResult 未实现 Serialize，手动构建 JSON 响应
            (
                StatusCode::OK,
                Json(json!({
                    "content": result.content,
                    "is_error": result.is_error,
                })),
            )
                .into_response()
        },
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Failed to call tool: {e}") })),
        )
            .into_response(),
    }
}
