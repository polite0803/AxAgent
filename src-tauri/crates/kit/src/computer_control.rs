// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "computer-use")]

use crate::permission::ensure_computer_control_granted;
use crate::screen_capture::{CaptureRegion, ScreenCapture};
use crate::ui_automation::{KeyModifier, MouseButton, UIAutomation, UIElementQuery};
use anyhow::Result;

pub async fn screen_capture(
    monitor: Option<u32>,
    region: Option<CaptureRegion>,
    window_title: Option<String>,
) -> Result<serde_json::Value> {
    ensure_computer_control_granted().map_err(|e| anyhow::anyhow!(e))?;
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
    ensure_computer_control_granted().map_err(|e| anyhow::anyhow!(e))?;
    UIAutomation::get_accessible_elements(&query).await
}

pub async fn mouse_click(x: f64, y: f64, button: Option<String>) -> Result<()> {
    ensure_computer_control_granted().map_err(|e| anyhow::anyhow!(e))?;
    let btn = match button.as_deref().unwrap_or("left") {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    UIAutomation::click(x, y, btn).await
}

pub async fn type_text(text: String, x: Option<f64>, y: Option<f64>) -> Result<()> {
    ensure_computer_control_granted().map_err(|e| anyhow::anyhow!(e))?;
    UIAutomation::type_text(&text, x, y).await
}

pub async fn press_key(key: String, modifiers: Vec<String>) -> Result<()> {
    ensure_computer_control_granted().map_err(|e| anyhow::anyhow!(e))?;
    let mut mods: Vec<KeyModifier> = Vec::with_capacity(modifiers.len());
    for m in &modifiers {
        match m.as_str() {
            "alt" => mods.push(KeyModifier::Alt),
            "control" | "ctrl" => mods.push(KeyModifier::Control),
            "shift" => mods.push(KeyModifier::Shift),
            "super" | "meta" | "win" => mods.push(KeyModifier::Super),
            // 未知修饰键不再静默降级为 Control，直接报错，避免意图被悄悄篡改
            other => anyhow::bail!("未知的修饰键: '{}'（仅支持 alt/ctrl/shift/meta）", other),
        }
    }
    UIAutomation::press_key(&key, mods).await
}

pub async fn mouse_scroll(x: f64, y: f64, delta: i32) -> Result<()> {
    ensure_computer_control_granted().map_err(|e| anyhow::anyhow!(e))?;
    UIAutomation::scroll(x, y, delta).await
}

pub async fn mouse_move(x: f64, y: f64) -> Result<()> {
    ensure_computer_control_granted().map_err(|e| anyhow::anyhow!(e))?;
    UIAutomation::move_mouse(x, y).await
}
