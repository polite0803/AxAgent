// SPDX-License-Identifier: AGPL-3.0-only

//! Agent 会话管理命令 — 暴露 AgentSessionBroker 能力给前端 UI。
//!
//! 这三个命令同时服务于：
//! - 桌面 App 前端的会话管理面板
//! - MCP McpAgentServer 的 status / cancel 工具（通过 Arc<dyn AgentSessionBroker>）
//! - 测试 / 调试 AgentSessionBroker 契约

use std::sync::Arc;

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::mcp as mcp_err;
use axagent_agent_macro::agent_command;
use axagent_harness::AgentSessionStatusView;
use tauri::{Emitter, State};

#[agent_command(domain = agent, safety = Safe, call_mode = StateOnly, description = "查询 agent 会话状态")]
#[tauri::command]
pub async fn agent_session_status(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<AgentSessionStatusView, String> {
    let broker: Arc<dyn axagent_harness::AgentSessionBroker> = state.agent_session_manager.clone();

    broker.get_session_status(&session_id).await.map_err(|e| {
        if e.starts_with("session_not_found") {
            String::from(
                ErrorResponse::new(mcp_err::AGENT_SESSION_NOT_FOUND)
                    .with_category(crate::commands::error::ErrorCategory::Retryable)
                    .with_detail(e)
                    .with_param("sessionId", session_id.clone()),
            )
        } else {
            e
        }
    })
}

#[agent_command(domain = agent, safety = Caution, call_mode = StateOnly, description = "取消 agent 会话执行")]
#[tauri::command]
pub async fn agent_session_cancel(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
) -> Result<(), String> {
    let broker: Arc<dyn axagent_harness::AgentSessionBroker> = state.agent_session_manager.clone();

    broker.cancel_session(&session_id).await.map_err(|e| {
        if e.starts_with("session_not_found") {
            String::from(
                ErrorResponse::new(mcp_err::AGENT_SESSION_NOT_FOUND)
                    .with_category(crate::commands::error::ErrorCategory::Retryable)
                    .with_detail(e)
                    .with_param("sessionId", session_id.clone()),
            )
        } else {
            String::from(
                ErrorResponse::new(mcp_err::AGENT_SESSION_CANCEL_FAILED)
                    .with_category(crate::commands::error::ErrorCategory::Retryable)
                    .with_detail(e)
                    .with_param("sessionId", session_id.clone()),
            )
        }
    })?;

    let _ = app.emit("agent-session-cancelled", serde_json::json!({ "sessionId": session_id }));
    Ok(())
}

#[agent_command(domain = agent, safety = Safe, call_mode = StateOnly, description = "列出活跃 agent 会话")]
#[tauri::command]
pub async fn agent_session_list(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let broker: Arc<dyn axagent_harness::AgentSessionBroker> = state.agent_session_manager.clone();

    broker.list_session_ids().await.map_err(|e| {
        String::from(
            ErrorResponse::new(mcp_err::CONNECT_FAILED)
                .with_category(crate::commands::error::ErrorCategory::Unrecoverable)
                .with_detail(e),
        )
    })
}
