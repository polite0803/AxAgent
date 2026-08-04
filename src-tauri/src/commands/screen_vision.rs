// SPDX-License-Identifier: AGPL-3.0-only

use axagent_kit::permission::ensure_computer_control_granted;
use axagent_kit::screen_vision::UIElementInfo;
use serde::{Deserialize, Serialize};
use tauri::State;
use agent_macro::agent_command;

use crate::AppState;
use crate::commands::provider_ctx::{VisionContext, build_vision_context};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenAnalysisResult {
    pub elements: Vec<UIElementInfo>,
    pub suggested_actions: Vec<SuggestedActionInfo>,
    pub reasoning: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestedActionInfo {
    pub action_type: String,
    pub target_element: String,
    pub description: String,
    pub reasoning: String,
    pub x: f64,
    pub y: f64,
}

async fn capture_screenshot(
    monitor_index: Option<u32>,
) -> Result<axagent_kit::screen_capture::ScreenCaptureResult, String> {
    let capture = axagent_kit::screen_capture::ScreenCapture::new();
    capture.capture_full(monitor_index).await.map_err(|e| format!("Screen capture failed: {}", e))
}

fn map_actions_to_info(
    actions: &[axagent_kit::screen_vision::SuggestedAction],
    elements: &[UIElementInfo],
) -> Vec<SuggestedActionInfo> {
    actions
        .iter()
        .map(|action| {
            let (x, y) = elements
                .iter()
                .find(|e| e.name == action.target_element)
                .map(|element| {
                    (
                        element.bounds.x + element.bounds.width / 2.0,
                        element.bounds.y + element.bounds.height / 2.0,
                    )
                })
                .unwrap_or((0.0, 0.0));

            SuggestedActionInfo {
                action_type: format!("{:?}", action.action_type).to_lowercase(),
                target_element: action.target_element.clone(),
                description: action.description.clone(),
                reasoning: action.reasoning.clone(),
                x,
                y,
            }
        })
        .collect()
}

#[agent_command(domain = vision, safety = Safe, call_mode = StateInput, description = "分析屏幕内容")]
#[tauri::command]
pub async fn analyze_screen(
    state: State<'_, AppState>,
    task_description: String,
    monitor_index: Option<u32>,
    provider_id: String,
    model_id: String,
) -> Result<ScreenAnalysisResult, String> {
    ensure_computer_control_granted()?;
    let screenshot = capture_screenshot(monitor_index).await?;
    let VisionContext { adapter, ctx } =
        build_vision_context(state.harness.db(), state.harness.master_key(), &provider_id).await?;

    let analysis = axagent_providers::screen_vision::analyze_screen(
        adapter.as_ref(),
        &ctx,
        model_id,
        &screenshot.image_base64,
        &task_description,
    )
    .await
    .map_err(|e| format!("Screen analysis failed: {}", e))?;

    let suggested_actions = map_actions_to_info(&analysis.suggested_actions, &analysis.elements);

    Ok(ScreenAnalysisResult {
        elements: analysis.elements,
        suggested_actions,
        reasoning: analysis.reasoning,
        confidence: analysis.confidence,
    })
}

#[agent_command(domain = vision, safety = Safe, call_mode = StateInput, description = "分析图片")]
#[tauri::command]
pub async fn analyze_image(
    state: State<'_, AppState>,
    image_base64: String,
    task: String,
    provider_id: String,
    model_id: String,
) -> Result<axagent_agent::VisionResult, String> {
    let task_enum: axagent_agent::VisionTask = serde_json::from_str(&format!("\"{}\"", task))
        .map_err(|e| format!("Invalid vision task '{}': {}", task, e))?;

    let image_data = if let Some(stripped) = image_base64.strip_prefix("data:image/png;base64,") {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(stripped)
            .map_err(|e| format!("Failed to decode base64 image: {}", e))?
    } else {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(&image_base64)
            .map_err(|e| format!("Failed to decode base64 image: {}", e))?
    };

    let VisionContext { adapter, ctx } =
        build_vision_context(state.harness.db(), state.harness.master_key(), &provider_id).await?;

    let pipeline = axagent_agent::VisionPipeline::new(adapter, ctx, model_id);

    pipeline
        .analyze(&image_data, task_enum)
        .await
        .map_err(|e| format!("Image analysis failed: {}", e))
}

#[agent_command(domain = vision, safety = Safe, call_mode = StateInput, description = "在屏幕上查找元素")]
#[tauri::command]
pub async fn find_element_on_screen(
    state: State<'_, AppState>,
    element_description: String,
    monitor_index: Option<u32>,
    provider_id: String,
    model_id: String,
) -> Result<Option<UIElementInfo>, String> {
    ensure_computer_control_granted()?;
    let screenshot = capture_screenshot(monitor_index).await?;
    let VisionContext { adapter, ctx } =
        build_vision_context(state.harness.db(), state.harness.master_key(), &provider_id).await?;

    axagent_providers::screen_vision::find_element(
        adapter.as_ref(),
        &ctx,
        model_id,
        &screenshot.image_base64,
        &element_description,
    )
    .await
    .map_err(|e| format!("Element search failed: {}", e))
}

#[agent_command(domain = vision, safety = Safe, call_mode = StateInput, description = "建议屏幕操作")]
#[tauri::command]
pub async fn suggest_screen_action(
    state: State<'_, AppState>,
    current_task: String,
    monitor_index: Option<u32>,
    provider_id: String,
    model_id: String,
) -> Result<Vec<SuggestedActionInfo>, String> {
    ensure_computer_control_granted()?;
    let screenshot = capture_screenshot(monitor_index).await?;
    let VisionContext { adapter, ctx } =
        build_vision_context(state.harness.db(), state.harness.master_key(), &provider_id).await?;

    // 单次模型调用同时返回 elements 与 suggested_actions（analyze_screen 的提示已包含二者），
    // 避免重复调用大模型浪费 token（修复 #20）
    let analysis = axagent_providers::screen_vision::analyze_screen(
        adapter.as_ref(),
        &ctx,
        model_id,
        &screenshot.image_base64,
        &current_task,
    )
    .await
    .map_err(|e| format!("Screen analysis failed: {}", e))?;

    Ok(map_actions_to_info(&analysis.suggested_actions, &analysis.elements))
}

#[agent_command(domain = vision, safety = Caution, call_mode = Manual, description = "点击指定屏幕坐标")]
#[tauri::command]
pub async fn click_element_at_position(
    x: f64,
    y: f64,
    button: Option<String>,
) -> Result<(), String> {
    ensure_computer_control_granted()?;
    use axagent_kit::ui_automation::MouseButton;

    let btn = match button.as_deref().unwrap_or("left") {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    };

    axagent_kit::ui_automation::UIAutomation::click(x, y, btn)
        .await
        .map_err(|e| format!("Click failed: {}", e))?;

    Ok(())
}

#[agent_command(domain = vision, safety = Caution, call_mode = Manual, description = "执行视觉操作（点击/双击/右键/输入/悬停）")]
#[tauri::command]
pub async fn execute_vision_action(
    action_type: String,
    x: f64,
    y: f64,
    text: Option<String>,
) -> Result<(), String> {
    ensure_computer_control_granted()?;
    use axagent_kit::ui_automation::UIAutomation;

    match action_type.to_lowercase().as_str() {
        "click" => {
            UIAutomation::click(x, y, axagent_kit::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Click failed: {}", e))?;
        },
        "double_click" | "doubleclick" => {
            UIAutomation::click(x, y, axagent_kit::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Click failed: {}", e))?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            UIAutomation::click(x, y, axagent_kit::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Double click failed: {}", e))?;
        },
        "right_click" | "rightclick" => {
            UIAutomation::click(x, y, axagent_kit::ui_automation::MouseButton::Right)
                .await
                .map_err(|e| format!("Right click failed: {}", e))?;
        },
        "type" | "input" => {
            if let Some(text) = text {
                UIAutomation::type_text(&text, Some(x), Some(y))
                    .await
                    .map_err(|e| format!("Type failed: {}", e))?;
            }
        },
        "hover" => {
            UIAutomation::move_mouse(x, y).await.map_err(|e| format!("Hover failed: {}", e))?;
        },
        _ => {
            return Err(format!("Unknown action type: {}", action_type));
        },
    }

    Ok(())
}
