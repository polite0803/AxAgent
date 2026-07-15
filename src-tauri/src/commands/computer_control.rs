// SPDX-License-Identifier: AGPL-3.0-only

use axagent_kit::computer_control;
use axagent_kit::permission;
use axagent_kit::screen_capture::CaptureRegion;
use axagent_kit::ui_automation::UIElementQuery;

/// 授予 computer_control 权限（由前端授权流程调用）。
/// 实际状态保存在 `axagent_kit::permission`，命令与 AI 工具路径共用同一闸门。
#[tauri::command]
pub fn grant_computer_control_permission() {
    permission::grant_computer_control();
}

/// 撤销 computer_control 权限。
#[tauri::command]
pub fn revoke_computer_control_permission() {
    permission::revoke_computer_control();
}

/// 查询当前 computer_control 权限状态。
#[tauri::command]
pub fn is_computer_control_granted() -> bool {
    permission::is_computer_control_granted()
}

// 说明：所有 computer_control 敏感操作的实际权限校验已下沉到
// `axagent_kit::computer_control::*` 内部（ensure_computer_control_granted），
// 因此无论调用来自 Tauri 命令、AI 工具（ComputerUseTool）还是视觉分析命令，
// 都必须经过同一 C4 闸门，避免任一入口绕过授权。

#[tauri::command]
pub async fn screen_capture(
    monitor: Option<u32>,
    region: Option<CaptureRegion>,
    window_title: Option<String>,
) -> Result<serde_json::Value, String> {
    computer_control::screen_capture(monitor, region, window_title).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_ui_elements(query: UIElementQuery) -> Result<Vec<serde_json::Value>, String> {
    computer_control::find_ui_elements(query)
        .await
        .map(|elems| elems.iter().filter_map(|e| serde_json::to_value(e).ok()).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_click(x: f64, y: f64, button: Option<String>) -> Result<(), String> {
    computer_control::mouse_click(x, y, button).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn type_text(text: String, x: Option<f64>, y: Option<f64>) -> Result<(), String> {
    computer_control::type_text(text, x, y).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn press_key(key: String, modifiers: Vec<String>) -> Result<(), String> {
    computer_control::press_key(key, modifiers).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_scroll(x: f64, y: f64, delta: i32) -> Result<(), String> {
    computer_control::mouse_scroll(x, y, delta).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_move(x: f64, y: f64) -> Result<(), String> {
    computer_control::mouse_move(x, y).await.map_err(|e| e.to_string())
}
