//! PTY（伪终端）命令 — 后端基础设施已存在（crates/runtime/src/pty.rs），
//! 但尚未通过 AppState 暴露。当前返回友好错误信息，待 PtyManager 集成后启用。

use tauri::command;

#[command]
pub async fn pty_create_session() -> Result<String, String> {
    Err("PTY functionality is not yet available".into())
}

#[command]
pub async fn pty_kill_session() -> Result<(), String> {
    Err("PTY functionality is not yet available".into())
}

#[command]
pub async fn pty_remove_session() -> Result<(), String> {
    Err("PTY functionality is not yet available".into())
}

#[command]
pub async fn pty_write() -> Result<(), String> {
    Err("PTY functionality is not yet available".into())
}

#[command]
pub async fn pty_resize() -> Result<(), String> {
    Err("PTY functionality is not yet available".into())
}

#[command]
pub async fn pty_list_sessions() -> Result<Vec<serde_json::Value>, String> {
    Err("PTY functionality is not yet available".into())
}

#[command]
pub async fn pty_analyze_output() -> Result<serde_json::Value, String> {
    Err("PTY functionality is not yet available".into())
}

#[command]
pub async fn pty_get_suggestions() -> Result<Vec<serde_json::Value>, String> {
    Err("PTY functionality is not yet available".into())
}
