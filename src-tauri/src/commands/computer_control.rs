use axagent_core::computer_control;
use axagent_core::screen_capture::CaptureRegion;
use axagent_core::ui_automation::UIElementQuery;

#[tauri::command]
pub async fn screen_capture(
    monitor: Option<u32>,
    region: Option<CaptureRegion>,
    window_title: Option<String>,
) -> Result<serde_json::Value, String> {
    computer_control::screen_capture(monitor, region, window_title)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_ui_elements(query: UIElementQuery) -> Result<Vec<serde_json::Value>, String> {
    computer_control::find_ui_elements(query)
        .await
        .map(|elems| {
            elems
                .iter()
                .map(|e| serde_json::to_value(e).unwrap())
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_click(x: f64, y: f64, button: Option<String>) -> Result<(), String> {
    computer_control::mouse_click(x, y, button)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn type_text(text: String, x: Option<f64>, y: Option<f64>) -> Result<(), String> {
    computer_control::type_text(text, x, y)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn press_key(key: String, modifiers: Vec<String>) -> Result<(), String> {
    computer_control::press_key(key, modifiers)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_scroll(x: f64, y: f64, delta: i32) -> Result<(), String> {
    computer_control::mouse_scroll(x, y, delta)
        .await
        .map_err(|e| e.to_string())
}
