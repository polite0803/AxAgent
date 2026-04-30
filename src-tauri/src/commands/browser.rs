use axagent_core::browser_automation::{ExtractedElement, NavigateResult, ScreenshotResult};
use tauri::State;

use crate::AppState;

/// 懒初始化浏览器客户端（如果尚未启动的话）
/// 从 AppState 管理生命周期，替代原来不安全的 static mut 全局变量
async fn ensure_browser_client(
    state: &AppState,
) -> Result<(), String> {
    let mut client_guard = state.browser_client.lock().await;
    if client_guard.is_none() {
        let client = axagent_core::browser_automation::PlaywrightClient::launch()
            .await
            .map_err(|e| e.to_string())?;
        *client_guard = Some(client);
    }
    Ok(())
}

#[tauri::command]
pub async fn browser_navigate(
    state: State<'_, AppState>,
    url: String,
) -> Result<NavigateResult, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client.navigate(&url).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_screenshot(
    state: State<'_, AppState>,
    full_page: Option<bool>,
) -> Result<ScreenshotResult, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client
        .screenshot(full_page.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_click(
    state: State<'_, AppState>,
    selector: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client.click(&selector).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_fill(
    state: State<'_, AppState>,
    selector: String,
    value: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client
        .fill(&selector, &value)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_type(
    state: State<'_, AppState>,
    selector: String,
    text: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client
        .type_text(&selector, &text)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_extract_text(
    state: State<'_, AppState>,
    selector: String,
) -> Result<String, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client
        .extract_text(&selector)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_extract_all(
    state: State<'_, AppState>,
    selector: String,
) -> Result<Vec<ExtractedElement>, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client
        .extract_all(&selector)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_get_content(
    state: State<'_, AppState>,
) -> Result<String, String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client.get_content().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_wait_for(
    state: State<'_, AppState>,
    selector: String,
    timeout: Option<u32>,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client
        .wait_for(&selector, timeout)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_select(
    state: State<'_, AppState>,
    selector: String,
    value: String,
) -> Result<(), String> {
    ensure_browser_client(&state).await?;
    let mut guard = state.browser_client.lock().await;
    let client = guard.as_mut().ok_or("浏览器客户端未初始化")?;
    client
        .select_option(&selector, &value)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_close(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut guard = state.browser_client.lock().await;
    if let Some(mut client) = guard.take() {
        client.close().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
