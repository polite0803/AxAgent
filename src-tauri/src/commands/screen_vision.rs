// SPDX-License-Identifier: AGPL-3.0-only

use axagent_core::screen_vision::UIElementInfo;
use axagent_harness::types::ProviderType;
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::AppState;

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

fn resolve_provider_adapter(
    provider_type: &ProviderType,
) -> Result<Arc<dyn ProviderAdapter>, String> {
    match provider_type {
        ProviderType::OpenAI => Ok(Arc::new(axagent_providers::openai::OpenAIAdapter::new())),
        ProviderType::OpenAIResponses => {
            Ok(Arc::new(axagent_providers::openai_responses::OpenAIResponsesAdapter::new()))
        },
        ProviderType::Anthropic => {
            Ok(Arc::new(axagent_providers::anthropic::AnthropicAdapter::new()))
        },
        ProviderType::Gemini => Ok(Arc::new(axagent_providers::gemini::GeminiAdapter::new())),
        ProviderType::OpenClaw => Ok(Arc::new(axagent_providers::openclaw::OpenClawAdapter::new())),
        ProviderType::Hermes => Ok(Arc::new(axagent_providers::hermes::HermesAdapter::new())),
        ProviderType::Ollama => Ok(Arc::new(axagent_providers::ollama::OllamaAdapter::new())),
    }
}

async fn capture_screenshot(
    monitor_index: Option<u32>,
) -> Result<axagent_core::screen_capture::ScreenCaptureResult, String> {
    let capture = axagent_core::screen_capture::ScreenCapture::new();
    capture
        .capture_full(monitor_index)
        .await
        .map_err(|e| format!("Screen capture failed: {}", e))
}

struct VisionContext {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
}

async fn build_vision_context(
    db: &sea_orm::DatabaseConnection,
    master_key: &[u8; 32],
    provider_id: &str,
) -> Result<VisionContext, String> {
    let provider = axagent_core::repo::provider::get_provider(db, provider_id)
        .await
        .map_err(|e| e.to_string())?;

    let key_row = axagent_core::repo::provider::get_active_key(db, provider_id)
        .await
        .map_err(|e| e.to_string())?;

    let decrypted_key = axagent_core::crypto::decrypt_key(&key_row.key_encrypted, master_key)
        .map_err(|e| e.to_string())?;

    let global_settings = axagent_core::repo::settings::get_settings(db)
        .await
        .unwrap_or_default();
    let resolved_proxy = axagent_harness::types::ProviderProxyConfig::resolve(
        &provider.proxy_config,
        &global_settings,
    );

    let adapter = resolve_provider_adapter(&provider.provider_type)?;

    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id,
        provider_id: provider.id,
        base_url: Some(axagent_harness::url_utils::resolve_base_url_for_type(
            &provider.api_host,
            &provider.provider_type,
        )),
        api_path: provider.api_path,
        proxy_config: resolved_proxy,
        custom_headers: provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    Ok(VisionContext { adapter, ctx })
}

fn map_actions_to_info(
    actions: &[axagent_core::screen_vision::SuggestedAction],
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

#[tauri::command]
pub async fn analyze_screen(
    state: State<'_, AppState>,
    task_description: String,
    monitor_index: Option<u32>,
    provider_id: String,
    model_id: String,
) -> Result<ScreenAnalysisResult, String> {
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

#[tauri::command]
pub async fn find_element_on_screen(
    state: State<'_, AppState>,
    element_description: String,
    monitor_index: Option<u32>,
    provider_id: String,
    model_id: String,
) -> Result<Option<UIElementInfo>, String> {
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

#[tauri::command]
pub async fn suggest_screen_action(
    state: State<'_, AppState>,
    current_task: String,
    monitor_index: Option<u32>,
    provider_id: String,
    model_id: String,
) -> Result<Vec<SuggestedActionInfo>, String> {
    let screenshot = capture_screenshot(monitor_index).await?;
    let VisionContext { adapter, ctx } =
        build_vision_context(state.harness.db(), state.harness.master_key(), &provider_id).await?;

    let actions = axagent_providers::screen_vision::suggest_next_action(
        adapter.as_ref(),
        &ctx,
        model_id.clone(),
        &screenshot.image_base64,
        &current_task,
    )
    .await
    .map_err(|e| format!("Screen analysis failed: {}", e))?;

    let analysis = axagent_providers::screen_vision::analyze_screen(
        adapter.as_ref(),
        &ctx,
        model_id,
        &screenshot.image_base64,
        &current_task,
    )
    .await
    .map_err(|e| format!("Screen analysis failed: {}", e))?;

    Ok(map_actions_to_info(&actions, &analysis.elements))
}

#[tauri::command]
pub async fn click_element_at_position(
    x: f64,
    y: f64,
    button: Option<String>,
) -> Result<(), String> {
    use axagent_core::ui_automation::MouseButton;

    let btn = match button.as_deref().unwrap_or("left") {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    };

    axagent_core::ui_automation::UIAutomation::click(x, y, btn)
        .await
        .map_err(|e| format!("Click failed: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn execute_vision_action(
    action_type: String,
    x: f64,
    y: f64,
    text: Option<String>,
) -> Result<(), String> {
    use axagent_core::ui_automation::UIAutomation;

    match action_type.to_lowercase().as_str() {
        "click" => {
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Click failed: {}", e))?;
        },
        "double_click" | "doubleclick" => {
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Click failed: {}", e))?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Double click failed: {}", e))?;
        },
        "right_click" | "rightclick" => {
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Right)
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
            UIAutomation::move_mouse(x, y)
                .await
                .map_err(|e| format!("Hover failed: {}", e))?;
        },
        _ => {
            return Err(format!("Unknown action type: {}", action_type));
        },
    }

    Ok(())
}
