// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_runtime::pty::{PtySessionConfig, PtySessionStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Manager;
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyCreateConfig {
    pub shell: Option<String>,
    pub cwd: Option<String>,
    pub env: HashMap<String, String>,
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtySessionInfo {
    pub id: String,
    pub status: String,
    pub shell: Option<String>,
    pub cwd: Option<String>,
    pub rows: u16,
    pub cols: u16,
}

impl PtySessionInfo {
    fn from_status(id: String, status: PtySessionStatus) -> Self {
        let status_str = match status {
            PtySessionStatus::Starting => "starting",
            PtySessionStatus::Running => "running",
            PtySessionStatus::Exited => "exited",
            PtySessionStatus::Error => "error",
        };
        Self { id, status: status_str.to_string(), shell: None, cwd: None, rows: 24, cols: 80 }
    }
}

#[command]
pub async fn pty_create_session(
    app: tauri::AppHandle,
    config: PtyCreateConfig,
) -> Result<String, String> {
    let state = app.state::<AppState>();
    let pty_config = PtySessionConfig {
        shell: config.shell,
        cwd: config.cwd,
        env: config.env,
        rows: config.rows,
        cols: config.cols,
    };
    let session_id = uuid::Uuid::new_v4().to_string();
    state.pty_manager.create_session(&session_id, pty_config).await?;
    Ok(session_id)
}

#[command]
pub async fn pty_kill_session(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.pty_manager.kill_session(&id).await
}

#[command]
pub async fn pty_remove_session(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.pty_manager.remove_session(&id).await
}

#[command]
pub async fn pty_write(app: tauri::AppHandle, id: String, data: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    match state.pty_manager.get_session(&id).await {
        Some(session) => session.write_str(&data).await,
        None => Err(format!("PTY session '{}' not found", id)),
    }
}

#[command]
pub async fn pty_resize(
    app: tauri::AppHandle,
    id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    match state.pty_manager.get_session(&id).await {
        Some(session) => session.resize(rows, cols).await,
        None => Err(format!("PTY session '{}' not found", id)),
    }
}

#[command]
pub async fn pty_list_sessions(app: tauri::AppHandle) -> Result<Vec<PtySessionInfo>, String> {
    let state = app.state::<AppState>();
    let sessions = state.pty_manager.list_sessions().await;
    Ok(sessions.into_iter().map(|(id, status)| PtySessionInfo::from_status(id, status)).collect())
}

#[command]
pub async fn pty_analyze_output(
    _app: tauri::AppHandle,
    _id: String,
) -> Result<serde_json::Value, String> {
    // 输出分析功能待后续 LLM 集成
    Ok(serde_json::json!({
        "has_errors": false,
        "errors": [],
        "last_exit_code": null,
        "last_command": null,
        "summary": "分析功能开发中"
    }))
}

#[command]
pub async fn pty_get_suggestions(
    _app: tauri::AppHandle,
    _id: String,
) -> Result<Vec<serde_json::Value>, String> {
    // 建议功能待后续集成
    Ok(vec![])
}
