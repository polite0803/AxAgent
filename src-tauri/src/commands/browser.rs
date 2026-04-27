use axagent_core::browser_automation::{ExtractedElement, NavigateResult, ScreenshotResult};

#[allow(static_mut_refs)]
static mut BROWSER_CLIENT: Option<axagent_core::browser_automation::PlaywrightClient> = None;

#[allow(static_mut_refs)]
fn get_browser_client(
) -> Result<&'static mut axagent_core::browser_automation::PlaywrightClient, String> {
    unsafe {
        if BROWSER_CLIENT.is_none() {
            let client = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    axagent_core::browser_automation::PlaywrightClient::launch().await
                })
                .map_err(|e| e.to_string())?;
            BROWSER_CLIENT = Some(client);
        }
        BROWSER_CLIENT
            .as_mut()
            .ok_or_else(|| "Browser client not initialized".to_string())
    }
}

#[tauri::command]
pub async fn browser_navigate(url: String) -> Result<NavigateResult, String> {
    let client = get_browser_client()?;
    client.navigate(&url).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_screenshot(full_page: Option<bool>) -> Result<ScreenshotResult, String> {
    let client = get_browser_client()?;
    client
        .screenshot(full_page.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_click(selector: String) -> Result<(), String> {
    let client = get_browser_client()?;
    client.click(&selector).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_fill(selector: String, value: String) -> Result<(), String> {
    let client = get_browser_client()?;
    client
        .fill(&selector, &value)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_type(selector: String, text: String) -> Result<(), String> {
    let client = get_browser_client()?;
    client
        .type_text(&selector, &text)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_extract_text(selector: String) -> Result<String, String> {
    let client = get_browser_client()?;
    client
        .extract_text(&selector)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_extract_all(selector: String) -> Result<Vec<ExtractedElement>, String> {
    let client = get_browser_client()?;
    client
        .extract_all(&selector)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_get_content() -> Result<String, String> {
    let client = get_browser_client()?;
    client.get_content().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_wait_for(selector: String, timeout: Option<u32>) -> Result<(), String> {
    let client = get_browser_client()?;
    client
        .wait_for(&selector, timeout)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_select(selector: String, value: String) -> Result<(), String> {
    let client = get_browser_client()?;
    client
        .select_option(&selector, &value)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_close() -> Result<(), String> {
    #[allow(static_mut_refs)]
    unsafe {
        if let Some(mut client) = BROWSER_CLIENT.take() {
            client.close().await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
