// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(not(target_os = "android"))]
use crate::screen_capture::{CaptureRegion, ScreenCapture};
#[cfg(not(target_os = "android"))]
use crate::ui_automation::{KeyModifier, MouseButton, UIAutomation, UIElementQuery};
#[cfg(not(target_os = "android"))]
use anyhow::Result;

#[cfg(target_os = "android")]
use axagent_harness::constants::android_msg;

#[cfg(not(target_os = "android"))]
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

#[cfg(not(target_os = "android"))]
pub async fn find_ui_elements(
    query: UIElementQuery,
) -> Result<Vec<crate::ui_automation::UIElement>> {
    UIAutomation::get_accessible_elements(&query).await
}

#[cfg(not(target_os = "android"))]
pub async fn mouse_click(x: f64, y: f64, button: Option<String>) -> Result<()> {
    let btn = match button.as_deref().unwrap_or("left") {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    UIAutomation::click(x, y, btn).await
}

#[cfg(not(target_os = "android"))]
pub async fn type_text(text: String, x: Option<f64>, y: Option<f64>) -> Result<()> {
    UIAutomation::type_text(&text, x, y).await
}

#[cfg(not(target_os = "android"))]
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

#[cfg(not(target_os = "android"))]
pub async fn mouse_scroll(x: f64, y: f64, delta: i32) -> Result<()> {
    UIAutomation::scroll(x, y, delta).await
}

#[cfg(not(target_os = "android"))]
pub async fn mouse_move(x: f64, y: f64) -> Result<()> {
    UIAutomation::move_mouse(x, y).await
}

#[cfg(target_os = "android")]
pub async fn screen_capture(
    _monitor: Option<u32>,
    _region: Option<crate::screen_capture::CaptureRegion>,
    _window_title: Option<String>,
) -> anyhow::Result<serde_json::Value> {
    Err(anyhow::anyhow!(android_msg::COMPUTER_CONTROL_NOT_AVAILABLE))
}

#[cfg(target_os = "android")]
pub async fn find_ui_elements(
    _query: crate::ui_automation::UIElementQuery,
) -> anyhow::Result<Vec<crate::ui_automation::UIElement>> {
    Err(anyhow::anyhow!(android_msg::COMPUTER_CONTROL_NOT_AVAILABLE))
}

#[cfg(target_os = "android")]
pub async fn mouse_click(_x: f64, _y: f64, _button: Option<String>) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(android_msg::COMPUTER_CONTROL_NOT_AVAILABLE))
}

#[cfg(target_os = "android")]
pub async fn type_text(_text: String, _x: Option<f64>, _y: Option<f64>) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(android_msg::COMPUTER_CONTROL_NOT_AVAILABLE))
}

#[cfg(target_os = "android")]
pub async fn press_key(_key: String, _modifiers: Vec<String>) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(android_msg::COMPUTER_CONTROL_NOT_AVAILABLE))
}

#[cfg(target_os = "android")]
pub async fn mouse_scroll(_x: f64, _y: f64, _delta: i32) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(android_msg::COMPUTER_CONTROL_NOT_AVAILABLE))
}

#[cfg(target_os = "android")]
pub async fn mouse_move(_x: f64, _y: f64) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(android_msg::COMPUTER_CONTROL_NOT_AVAILABLE))
}
