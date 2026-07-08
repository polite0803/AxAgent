// SPDX-License-Identifier: AGPL-3.0-only

use axagent_kit::computer_control;
use axagent_kit::screen_capture::CaptureRegion;
use axagent_kit::ui_automation::UIElementQuery;
use std::sync::atomic::{AtomicBool, Ordering};

/// SECURITY (C4): 全局 computer_control 权限开关。
/// 默认关闭，只有用户在前端显式授权后才设为 true。
static COMPUTER_CONTROL_GRANTED: AtomicBool = AtomicBool::new(false);

/// 授予 computer_control 权限（由前端授权流程调用）。
#[tauri::command]
pub fn grant_computer_control_permission() {
    COMPUTER_CONTROL_GRANTED.store(true, Ordering::SeqCst);
    tracing::info!("computer_control permission granted");
}

/// 撤销 computer_control 权限。
#[tauri::command]
pub fn revoke_computer_control_permission() {
    COMPUTER_CONTROL_GRANTED.store(false, Ordering::SeqCst);
    tracing::info!("computer_control permission revoked");
}

/// 查询当前 computer_control 权限状态。
#[tauri::command]
pub fn is_computer_control_granted() -> bool {
    COMPUTER_CONTROL_GRANTED.load(Ordering::SeqCst)
}

/// SECURITY (C4): 权限检查中间层。
/// 在执行任何 computer_control 敏感命令前调用，验证用户是否已显式授权。
/// 未授权时返回明确的权限拒绝错误，防止未授权的前端调用绕过权限控制。
fn check_computer_control_permission() -> Result<(), String> {
    if COMPUTER_CONTROL_GRANTED.load(Ordering::SeqCst) {
        Ok(())
    } else {
        Err(
            "Permission denied: computer_control capability has not been granted. \
             The user must explicitly authorize computer control access before \
             screen capture, mouse/keyboard automation commands can be used."
                .to_string(),
        )
    }
}

#[tauri::command]
pub async fn screen_capture(
    monitor: Option<u32>,
    region: Option<CaptureRegion>,
    window_title: Option<String>,
) -> Result<serde_json::Value, String> {
    check_computer_control_permission()?;
    computer_control::screen_capture(monitor, region, window_title).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_ui_elements(query: UIElementQuery) -> Result<Vec<serde_json::Value>, String> {
    check_computer_control_permission()?;
    computer_control::find_ui_elements(query)
        .await
        .map(|elems| elems.iter().filter_map(|e| serde_json::to_value(e).ok()).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_click(x: f64, y: f64, button: Option<String>) -> Result<(), String> {
    check_computer_control_permission()?;
    computer_control::mouse_click(x, y, button).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn type_text(text: String, x: Option<f64>, y: Option<f64>) -> Result<(), String> {
    check_computer_control_permission()?;
    computer_control::type_text(text, x, y).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn press_key(key: String, modifiers: Vec<String>) -> Result<(), String> {
    check_computer_control_permission()?;
    computer_control::press_key(key, modifiers).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_scroll(x: f64, y: f64, delta: i32) -> Result<(), String> {
    check_computer_control_permission()?;
    computer_control::mouse_scroll(x, y, delta).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_move(x: f64, y: f64) -> Result<(), String> {
    check_computer_control_permission()?;
    computer_control::mouse_move(x, y).await.map_err(|e| e.to_string())
}
