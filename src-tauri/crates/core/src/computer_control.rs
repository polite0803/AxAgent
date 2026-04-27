use crate::screen_capture::{CaptureRegion, ScreenCapture};
use crate::ui_automation::{KeyModifier, MouseButton, UIAutomation, UIElementQuery};
use anyhow::Result;

pub async fn screen_capture(
    monitor: Option<u32>,
    region: Option<CaptureRegion>,
    window_title: Option<String>,
) -> Result<serde_json::Value> {
    let capture = ScreenCapture::new();
    let result = match (region, window_title) {
        (Some(r), _) => capture.capture_region(r).await,
        (_, Some(title)) => capture.capture_window(&title).await,
        _ => capture.capture_full(monitor).await,
    };
    Ok(serde_json::to_value(result?)?)
}

pub async fn find_ui_elements(
    query: UIElementQuery,
) -> Result<Vec<crate::ui_automation::UIElement>> {
    UIAutomation::get_accessible_elements(&query).await
}

pub async fn mouse_click(x: f64, y: f64, button: Option<String>) -> Result<()> {
    let btn = match button.as_deref().unwrap_or("left") {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    UIAutomation::click(x, y, btn).await
}

pub async fn type_text(text: String, x: Option<f64>, y: Option<f64>) -> Result<()> {
    UIAutomation::type_text(&text, x, y).await
}

pub async fn press_key(key: String, modifiers: Vec<String>) -> Result<()> {
    let mods: Vec<KeyModifier> = modifiers
        .iter()
        .map(|m| match m.as_str() {
            "alt" => KeyModifier::Alt,
            "control" | "ctrl" => KeyModifier::Control,
            "shift" => KeyModifier::Shift,
            "super" | "meta" | "win" => KeyModifier::Super,
            _ => KeyModifier::Control,
        })
        .collect();
    UIAutomation::press_key(&key, mods).await
}

pub async fn mouse_scroll(x: f64, y: f64, delta: i32) -> Result<()> {
    UIAutomation::scroll(x, y, delta).await
}
