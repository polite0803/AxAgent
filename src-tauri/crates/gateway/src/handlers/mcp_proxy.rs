// SPDX-License-Identifier: AGPL-3.0-only

//! MCP 代理路由：通过 Gateway API 暴露 MCP 工具调用。
//!
//! 使用 harness `McpServerStore` / `McpClientService` trait，
//! 不直接依赖 `axagent-mcp` / `axagent-entities`。

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;

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
// Handlers
// ---------------------------------------------------------------------------

/// GET /v1/mcp/servers — 列出可用的 MCP 服务器。
pub async fn list_mcp_servers(
    State(state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    match state.mcp_store.list_enabled().await {
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
    let server = match state.mcp_store.get_by_id(&server_id).await {
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

    match state.mcp_client.discover_tools(&server).await {
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
    let server = match state.mcp_store.get_by_id(&server_id).await {
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

    match state
        .mcp_client
        .call_tool(&server, &body.tool_name, body.arguments)
        .await
    {
        Ok(result) => (
            StatusCode::OK,
            Json(json!({
                "content": result.content,
                "is_error": result.is_error,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Failed to call tool: {e}") })),
        )
            .into_response(),
    }
}
