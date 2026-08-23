// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::proxy as proxy_err;
use axagent_agent_macro::agent_command;
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::Manager;

#[agent_command(domain = desktop, safety = Caution, call_mode = Manual, description = "最小化窗口")]
#[tauri::command]
pub async fn minimize_window(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = desktop, safety = Caution, call_mode = Manual, description = "切换窗口最大化状态")]
#[tauri::command]
pub async fn toggle_maximize_window(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized().map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })? {
        window.unmaximize().map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
    } else {
        window.maximize().map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
    }
}

#[agent_command(domain = desktop, safety = Caution, call_mode = Manual, description = "设置窗口置顶")]
#[tauri::command]
pub async fn set_always_on_top(window: tauri::Window, enabled: bool) -> Result<(), String> {
    window.set_always_on_top(enabled).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = desktop, safety = Caution, call_mode = Manual, description = "设置关闭到托盘")]
#[tauri::command]
pub async fn set_close_to_tray(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.close_to_tray.store(enabled, Ordering::Release);
    Ok(())
}

#[agent_command(domain = desktop, safety = Caution, call_mode = Manual, description = "应用启动设置")]
#[tauri::command]
pub async fn apply_startup_settings(
    window: tauri::Window,
    app: tauri::AppHandle,
    always_on_top: bool,
    close_to_tray: bool,
) -> Result<(), String> {
    window.set_always_on_top(always_on_top).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let state = app.state::<AppState>();
    state.close_to_tray.store(close_to_tray, Ordering::Relaxed);
    Ok(())
}

#[agent_command(domain = desktop, safety = Dangerous, call_mode = Manual, description = "强制退出应用")]
#[tauri::command]
pub async fn force_quit(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[agent_command(domain = desktop, safety = Safe, call_mode = Manual, description = "获取桌面端能力")]
#[tauri::command]
pub async fn get_desktop_capabilities() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!([
        { "key": "tray", "supported": true },
        { "key": "global_shortcut", "supported": true },
        { "key": "protocol_handler", "supported": true },
        { "key": "mini_window", "supported": true },
        { "key": "notification", "supported": cfg!(feature = "notification") }
    ]))
}

#[derive(Debug, Serialize)]
pub struct DesktopNotificationResult {
    pub sent: bool,
    pub method: String, // "native" | "log"
}

#[agent_command(domain = desktop, safety = Caution, call_mode = Manual, description = "发送桌面通知")]
#[tauri::command]
pub async fn send_desktop_notification(
    app: tauri::AppHandle,
    title: String,
    body: String,
) -> Result<DesktopNotificationResult, String> {
    #[cfg(feature = "notification")]
    {
        use tauri_plugin_notification::NotificationExt;
        match app.notification().builder().title(&title).body(&body).show() {
            Ok(()) => {
                return Ok(DesktopNotificationResult { sent: true, method: "native".to_string() });
            },
            Err(e) => {
                tracing::warn!("Native notification failed, falling back to log: {}", e);
            },
        }
    }
    tracing::info!(
        title = %title,
        body = %body,
        "Desktop notification (placeholder — notification plugin not available or failed)"
    );
    Ok(DesktopNotificationResult { sent: false, method: "log".to_string() })
}

#[agent_command(domain = desktop, safety = Safe, call_mode = Manual, description = "获取窗口状态")]
#[tauri::command]
pub async fn get_window_state() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "width": 1200,
        "height": 800,
        "maximized": false,
        "visible": true
    }))
}

/// SECURITY: 仅在调试构建中开放 DevTools 命令，防止生产环境中通过 IPC 调用打开 DevTools。
#[agent_command(domain = desktop, safety = Caution, call_mode = Manual, description = "打开开发者工具")]
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn open_devtools(webview_window: tauri::WebviewWindow) -> Result<(), String> {
    webview_window.open_devtools();
    Ok(())
}

#[agent_command(domain = desktop, safety = Caution, call_mode = Manual, description = "打开开发者工具")]
#[cfg(not(debug_assertions))]
#[tauri::command]
pub async fn open_devtools() -> Result<(), String> {
    Err("DevTools are disabled in release builds".to_string())
}

#[agent_command(domain = desktop, safety = Safe, call_mode = Manual, description = "测试代理连接")]
#[tauri::command]
pub async fn test_proxy(
    _proxy_type: String,
    proxy_address: String,
    proxy_port: u16,
) -> Result<serde_json::Value, String> {
    use std::time::Instant;
    use tokio::net::TcpStream;
    use tokio::time::{Duration, timeout};

    let is_private = proxy_address == "127.0.0.1"
        || proxy_address == "localhost"
        || proxy_address == "0.0.0.0"
        || proxy_address == "::1"
        || proxy_address.starts_with("10.")
        || proxy_address.starts_with("192.168.")
        || proxy_address.starts_with("169.254.")
        || proxy_address.contains(':')
        || !proxy_address.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_');

    if !is_private {
        if let Some(second_octet) = proxy_address
            .strip_prefix("172.")
            .and_then(|rest| rest.split('.').next())
            .and_then(|s| s.parse::<u32>().ok())
        {
            if (16..=31).contains(&second_octet) {
                return Err("Cannot test proxy with internal/private addresses".into());
            }
        }
    }

    if is_private {
        return Err(ErrorResponse::err(proxy_err::ADDRESS_NOT_ALLOWED));
    }

    let addr = format!("{}:{}", proxy_address, proxy_port);
    let start = Instant::now();

    // SECURITY (S2): 脱敏错误信息，防止泄露内网拓扑或内部网络细节。
    match timeout(Duration::from_secs(5), TcpStream::connect(&addr)).await {
        Ok(Ok(_stream)) => {
            let latency = start.elapsed().as_millis();
            Ok(serde_json::json!({ "ok": true, "latency_ms": latency }))
        },
        Ok(Err(_e)) => {
            Ok(serde_json::json!({ "ok": false, "error": "Connection refused or failed" }))
        },
        Err(_) => Ok(serde_json::json!({ "ok": false, "error": "Connection timed out (5s)" })),
    }
}

#[agent_command(domain = desktop, safety = Safe, call_mode = Manual, description = "列出系统字体")]
#[tauri::command]
#[cfg(not(target_os = "android"))]
pub async fn list_system_fonts() -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(|| {
        let source = font_kit::source::SystemSource::new();
        let mut families = source.all_families().map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        families.sort_by_key(|a| a.to_lowercase());
        Ok(families)
    })
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?
}
