// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::OnceLock;

use axagent_agent_macro::agent_command;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};

const QUICKBAR_LABEL: &str = "quickbar";
const QUICKBAR_WIDTH: f64 = 650.0;
const QUICKBAR_HEIGHT: f64 = 400.0;
const FALLBACK_URL_STR: &str = "http://localhost:1420/index.html?__route=quickbar";
static FALLBACK_URL: OnceLock<Url> = OnceLock::new();

fn quickbar_url(app: &AppHandle) -> WebviewUrl {
    match app.config().build.dev_url.as_ref() {
        Some(dev_url) => {
            let base = dev_url.as_str().trim_end_matches('/');
            WebviewUrl::External(
                format!("{}/index.html?__route=quickbar", base).parse().unwrap_or_else(|_| {
                    tracing::warn!("quickbar dev_url 格式无效，使用默认 URL");
                    FALLBACK_URL
                        .get_or_init(|| {
                            FALLBACK_URL_STR.parse().expect("hardcoded fallback URL is valid")
                        })
                        .clone()
                }),
            )
        },
        None => WebviewUrl::App("index.html?__route=quickbar".into()),
    }
}

#[agent_command(domain = quickbar, safety = Caution, call_mode = StateOnly, description = "显示快捷栏窗口")]
#[tauri::command]
pub async fn show_quickbar(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(QUICKBAR_LABEL) {
        window.show().map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        window.set_focus().map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        let _ = window.center();
        return Ok(());
    }

    let url = quickbar_url(&app);

    let window = WebviewWindowBuilder::new(&app, QUICKBAR_LABEL, url)
        .title("AxAgent QuickBar")
        .inner_size(QUICKBAR_WIDTH, QUICKBAR_HEIGHT)
        .min_inner_size(400.0, 52.0)
        .decorations(false)
        .always_on_top(true)
        .resizable(true)
        .visible(true)
        .center()
        .build()
        .map_err(|e| format!("Failed to create quickbar window: {}", e))?;

    window.set_focus().map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    Ok(())
}

#[agent_command(domain = quickbar, safety = Caution, call_mode = StateOnly, description = "隐藏快捷栏窗口")]
#[tauri::command]
pub async fn hide_quickbar(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(QUICKBAR_LABEL) {
        window.hide().map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }
    Ok(())
}
