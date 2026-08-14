// SPDX-License-Identifier: AGPL-3.0-only

use agent_macro::agent_command;

#[cfg(not(target_os = "android"))]
use axagent_kit::browser_automation::{
    ExtractedElement, NavigateResult, PlaywrightClient, ScreenshotResult, validate_browser_url,
};
#[cfg(not(target_os = "android"))]
use tauri::State;

#[cfg(not(target_os = "android"))]
use crate::AppState;
#[cfg(not(target_os = "android"))]
use crate::commands::error::ErrorCategory;
#[cfg(not(target_os = "android"))]
use crate::commands::error::ErrorResponse;
#[cfg(not(target_os = "android"))]
use crate::commands::error_code::browser as browser_err;

#[cfg(not(target_os = "android"))]
async fn ensure_browser_client(state: &AppState) -> Result<(), String> {
    let mut client_guard = state.browser_client.lock().await;
    // 健康检查：若已有实例但子进程已退出，则重建（修复 #14）
    let needs_launch = match client_guard.as_mut() {
        Some(c) => !c.is_alive(),
        None => true,
    };
    if needs_launch {
        let client = PlaywrightClient::launch().await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        *client_guard = Some(client);
    }
    Ok(())
}

#[agent_command(domain = browser, safety = Caution, call_mode = StateInput, description = "浏览器导航到指定URL")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_navigate(
    state: State<'_, AppState>,
    url: String,
) -> Result<NavigateResult, String> {
    // SECURITY (S4): SSRF 防护 — 统一调用可复用的校验函数。
    // 仅允许 http/https，且禁止内网/回环/链路本地/保留地址（含 IPv4 映射 IPv6、
    // 0.0.0.0/8，并对域名做 DNS 解析校验），见 axagent_kit::browser_automation::validate_browser_url。
    if let Err(detail) = validate_browser_url(&url) {
        return Err(ErrorResponse::new(browser_err::ADDRESS_NOT_ALLOWED)
            .with_detail(detail)
            .with_category(ErrorCategory::PermissionDenied)
            .into());
    }

    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.navigate(&url).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Safe, call_mode = StateInput, description = "浏览器页面截图")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_screenshot(
    state: State<'_, AppState>,
    full_page: Option<bool>,
) -> Result<ScreenshotResult, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.screenshot(full_page.unwrap_or(false)).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Caution, call_mode = StateInput, description = "浏览器点击元素")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_click(state: State<'_, AppState>, selector: String) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.click(&selector).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Caution, call_mode = StateInput, description = "浏览器填充表单")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_fill(
    state: State<'_, AppState>,
    selector: String,
    value: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.fill(&selector, &value).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Caution, call_mode = StateInput, description = "浏览器输入文本")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_type(
    state: State<'_, AppState>,
    selector: String,
    text: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.type_text(&selector, &text).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Safe, call_mode = StateInput, description = "浏览器提取元素文本")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_extract_text(
    state: State<'_, AppState>,
    selector: String,
) -> Result<String, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.extract_text(&selector).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Safe, call_mode = StateInput, description = "浏览器提取所有元素")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_extract_all(
    state: State<'_, AppState>,
    selector: String,
) -> Result<Vec<ExtractedElement>, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.extract_all(&selector).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Safe, call_mode = StateOnly, description = "浏览器获取页面内容")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_get_content(state: State<'_, AppState>) -> Result<String, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.get_content().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Caution, call_mode = StateInput, description = "浏览器等待元素")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_wait_for(
    state: State<'_, AppState>,
    selector: String,
    timeout: Option<u32>,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.wait_for(&selector, timeout).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Caution, call_mode = StateInput, description = "浏览器选择选项")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_select(
    state: State<'_, AppState>,
    selector: String,
    value: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or(ErrorResponse::new(browser_err::NOT_INITIALIZED))?;
    client.select_option(&selector, &value).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = browser, safety = Caution, call_mode = StateOnly, description = "关闭浏览器")]
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn browser_close(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.browser_client.lock().await;
    if let Some(mut client) = guard.take() {
        client.close().await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }
    Ok(())
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器导航（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_navigate(_url: String) -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器截图（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_screenshot(_full_page: Option<bool>) -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器点击元素（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_click(_selector: String) -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器填充表单（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_fill(_selector: String, _value: String) -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器输入文本（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_type(_selector: String, _text: String) -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器提取文本（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_extract_text(_selector: String) -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器提取元素（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_extract_all(_selector: String) -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器获取内容（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_get_content() -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器等待元素（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_wait_for(_selector: String, _timeout: Option<u32>) -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "浏览器选择选项（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_select(_selector: String, _value: String) -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}

#[agent_command(domain = browser, safety = Safe, call_mode = Manual, description = "关闭浏览器（Android 不可用）")]
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn browser_close() -> Result<(), String> {
    Err(ErrorResponse::err_with_detail(
        browser_err::NOT_INITIALIZED,
        "Browser automation is not available on Android",
    ))
}
