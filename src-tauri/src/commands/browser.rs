// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(not(target_os = "android"))]
use axagent_kit::browser_automation::{ExtractedElement, NavigateResult, ScreenshotResult};
#[cfg(not(target_os = "android"))]
use tauri::State;

#[cfg(not(target_os = "android"))]
use crate::AppState;
#[cfg(not(target_os = "android"))]
use crate::commands::error::ErrorResponse;
#[cfg(not(target_os = "android"))]
use crate::commands::error_code::browser as browser_err;

#[cfg(not(target_os = "android"))]
async fn ensure_browser_client(state: &AppState) -> Result<(), String> {
    let mut client_guard = state.browser_client.lock().await;
    if client_guard.is_none() {
        let client = axagent_kit::browser_automation::PlaywrightClient::launch()
            .await
            .map_err(|e| e.to_string())?;
        *client_guard = Some(client);
    }
    Ok(())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_navigate(
    state: State<'_, AppState>,
    url: String,
) -> Result<NavigateResult, String> {
    // SECURITY (S4): SSRF 防护 — 仅允许 http/https 协议，阻止内网地址
    let parsed =
        reqwest::Url::parse(&url).map_err(|_| ErrorResponse::new(browser_err::INVALID_URL))?;
    match parsed.scheme() {
        "http" | "https" => {},
        _ => return Err(ErrorResponse::new(browser_err::SCHEME_NOT_ALLOWED).into()),
    }
    let host = parsed.host_str().unwrap_or("");
    let host_lower = host.to_lowercase();
    if host_lower == "localhost"
        || host_lower == "0.0.0.0"
        || host_lower == "::1"
        || host_lower == "[::1]"
        || host_lower.starts_with("127.")
        || host_lower.starts_with("10.")
        || host_lower.starts_with("192.168.")
        || host_lower.starts_with("169.254.")
        || host_lower.starts_with("172.")
    {
        return Err(ErrorResponse::new(browser_err::ADDRESS_NOT_ALLOWED).into());
    }
    // 检查非 IP 字面量是否解析为私网
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4)
                if v4.is_loopback()
                    || v4.is_private()
                    || v4.is_unspecified()
                    || v4.is_link_local() =>
            {
                return Err(ErrorResponse::new(browser_err::ADDRESS_NOT_ALLOWED).into());
            },
            std::net::IpAddr::V6(v6)
                if v6.is_loopback()
                    || v6.is_unspecified()
                    || v6.segments()[0] & 0xFFC0 == 0xFE80 =>
            {
                return Err(ErrorResponse::new(browser_err::ADDRESS_NOT_ALLOWED).into());
            },
            _ => {},
        }
    }

    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.navigate(&url).await.map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_screenshot(
    state: State<'_, AppState>,
    full_page: Option<bool>,
) -> Result<ScreenshotResult, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client
        .screenshot(full_page.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_click(state: State<'_, AppState>, selector: String) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.click(&selector).await.map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_fill(
    state: State<'_, AppState>,
    selector: String,
    value: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client
        .fill(&selector, &value)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_type(
    state: State<'_, AppState>,
    selector: String,
    text: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client
        .type_text(&selector, &text)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_extract_text(
    state: State<'_, AppState>,
    selector: String,
) -> Result<String, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client
        .extract_text(&selector)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_extract_all(
    state: State<'_, AppState>,
    selector: String,
) -> Result<Vec<ExtractedElement>, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client
        .extract_all(&selector)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_get_content(state: State<'_, AppState>) -> Result<String, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.get_content().await.map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_wait_for(
    state: State<'_, AppState>,
    selector: String,
    timeout: Option<u32>,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client
        .wait_for(&selector, timeout)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_select(
    state: State<'_, AppState>,
    selector: String,
    value: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard
        .as_mut()
        .ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client
        .select_option(&selector, &value)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_close(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.browser_client.lock().await;
    if let Some(mut client) = guard.take() {
        client.close().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_navigate(_url: String) -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_screenshot(_full_page: Option<bool>) -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_click(_selector: String) -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_fill(_selector: String, _value: String) -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_type(_selector: String, _text: String) -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_extract_text(_selector: String) -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_extract_all(_selector: String) -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_get_content() -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_wait_for(_selector: String, _timeout: Option<u32>) -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_select(_selector: String, _value: String) -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_close() -> Result<(), String> {
    Err("Browser automation is not available on Android".to_string())
}
