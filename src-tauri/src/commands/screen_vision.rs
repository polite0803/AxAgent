use axagent_core::screen_vision::{ScreenAnalysis, UIElementInfo};
use axagent_core::types::ProviderType;
use axagent_providers::registry::ProviderRegistry;
use axagent_providers::ProviderRequestContext;
use serde::{Deserialize, Serialize};
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

fn provider_type_to_registry_key(pt: &ProviderType) -> &'static str {
    match pt {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    }
}

#[tauri::command]
pub async fn analyze_screen(
    state: State<'_, AppState>,
    task_description: String,
    monitor_index: Option<u32>,
    provider_id: String,
    model_id: String,
) -> Result<ScreenAnalysisResult, String> {
    let capture = axagent_core::screen_capture::ScreenCapture::new();
    let screenshot = capture
        .capture_full(monitor_index)
        .await
        .map_err(|e| format!("Screen capture failed: {}", e))?;

    // Get provider from DB
    let provider = axagent_core::repo::provider::get_provider(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;

    // Get active key
    let key_row = axagent_core::repo::provider::get_active_key(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;

    // Decrypt key
    let decrypted_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
            .map_err(|e| e.to_string())?;

    // Get global settings for proxy
    let global_settings = axagent_core::repo::settings::get_settings(&state.sea_db)
        .await
        .unwrap_or_default();
    let resolved_proxy =
        axagent_core::types::ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    // Get adapter
    let registry = ProviderRegistry::create_default();
    let registry_key = provider_type_to_registry_key(&provider.provider_type);
    let adapter = registry
        .get(registry_key)
        .ok_or_else(|| format!("No adapter for provider type: {:?}", provider.provider_type))?;

    // Build context
    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id,
        provider_id: provider.id,
        base_url: Some(axagent_providers::resolve_base_url_for_type(
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

    // Analyze screen
    let analysis = axagent_providers::screen_vision::analyze_screen(
        adapter.as_ref(),
        &ctx,
        model_id,
        &screenshot.image_base64,
        &task_description,
    )
    .await
    .map_err(|e| format!("Screen analysis failed: {}", e))?;

    let suggested_actions: Vec<SuggestedActionInfo> = analysis
        .suggested_actions
        .iter()
        .map(|action| {
            let (x, y) = if let Some(element) = analysis
                .elements
                .iter()
                .find(|e| e.name == action.target_element)
            {
                (
                    element.bounds.x + element.bounds.width / 2.0,
                    element.bounds.y + element.bounds.height / 2.0,
                )
            } else {
                (0.0, 0.0)
            };

            SuggestedActionInfo {
                action_type: format!("{:?}", action.action_type).to_lowercase(),
                target_element: action.target_element.clone(),
                description: action.description.clone(),
                reasoning: action.reasoning.clone(),
                x,
                y,
            }
        })
        .collect();

    Ok(ScreenAnalysisResult {
        elements: analysis.elements,
        suggested_actions,
        reasoning: analysis.reasoning,
        confidence: analysis.confidence,
    })
}

#[tauri::command]
pub async fn find_element_on_screen(
    state: State<'_, AppState>,
    element_description: String,
    monitor_index: Option<u32>,
    provider_id: String,
    model_id: String,
) -> Result<Option<UIElementInfo>, String> {
    let capture = axagent_core::screen_capture::ScreenCapture::new();
    let screenshot = capture
        .capture_full(monitor_index)
        .await
        .map_err(|e| format!("Screen capture failed: {}", e))?;

    // Get provider from DB
    let provider = axagent_core::repo::provider::get_provider(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;

    // Get active key
    let key_row = axagent_core::repo::provider::get_active_key(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;

    // Decrypt key
    let decrypted_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
            .map_err(|e| e.to_string())?;

    // Get global settings for proxy
    let global_settings = axagent_core::repo::settings::get_settings(&state.sea_db)
        .await
        .unwrap_or_default();
    let resolved_proxy =
        axagent_core::types::ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    // Get adapter
    let registry = ProviderRegistry::create_default();
    let registry_key = provider_type_to_registry_key(&provider.provider_type);
    let adapter = registry
        .get(registry_key)
        .ok_or_else(|| format!("No adapter for provider type: {:?}", provider.provider_type))?;

    // Build context
    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id,
        provider_id: provider.id,
        base_url: Some(axagent_providers::resolve_base_url_for_type(
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
    let capture = axagent_core::screen_capture::ScreenCapture::new();
    let screenshot = capture
        .capture_full(monitor_index)
        .await
        .map_err(|e| format!("Screen capture failed: {}", e))?;

    // Get provider from DB
    let provider = axagent_core::repo::provider::get_provider(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;

    // Get active key
    let key_row = axagent_core::repo::provider::get_active_key(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;

    // Decrypt key
    let decrypted_key =
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
            .map_err(|e| e.to_string())?;

    // Get global settings for proxy
    let global_settings = axagent_core::repo::settings::get_settings(&state.sea_db)
        .await
        .unwrap_or_default();
    let resolved_proxy =
        axagent_core::types::ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    // Get adapter
    let registry = ProviderRegistry::create_default();
    let registry_key = provider_type_to_registry_key(&provider.provider_type);
    let adapter = registry
        .get(registry_key)
        .ok_or_else(|| format!("No adapter for provider type: {:?}", provider.provider_type))?;

    // Build context
    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id,
        provider_id: provider.id,
        base_url: Some(axagent_providers::resolve_base_url_for_type(
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

    let actions = axagent_providers::screen_vision::suggest_next_action(
        adapter.as_ref(),
        &ctx,
        model_id,
        &screenshot.image_base64,
        &current_task,
    )
    .await
    .map_err(|e| format!("Screen analysis failed: {}", e))?;

    // We need to parse the analysis to get elements for coordinates
    // Alternatively, call analyze_screen first
    let analysis = axagent_providers::screen_vision::analyze_screen(
        adapter.as_ref(),
        &ctx,
        model_id,
        &screenshot.image_base64,
        &current_task,
    )
    .await
    .map_err(|e| format!("Screen analysis failed: {}", e))?;

    let suggested_actions: Vec<SuggestedActionInfo> = actions
        .iter()
        .map(|action| {
            let (x, y) = if let Some(element) = analysis
                .elements
                .iter()
                .find(|e| e.name == action.target_element)
            {
                (
                    element.bounds.x + element.bounds.width / 2.0,
                    element.bounds.y + element.bounds.height / 2.0,
                )
            } else {
                (0.0, 0.0)
            };

            SuggestedActionInfo {
                action_type: format!("{:?}", action.action_type).to_lowercase(),
                target_element: action.target_element.clone(),
                description: action.description.clone(),
                reasoning: action.reasoning.clone(),
                x,
                y,
            }
        })
        .collect();

    Ok(suggested_actions)
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
        }
        "double_click" | "doubleclick" => {
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Click failed: {}", e))?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Double click failed: {}", e))?;
        }
        "right_click" | "rightclick" => {
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Right)
                .await
                .map_err(|e| format!("Right click failed: {}", e))?;
        }
        "type" | "input" => {
            if let Some(text) = text {
                UIAutomation::type_text(&text, Some(x), Some(y))
                    .await
                    .map_err(|e| format!("Type failed: {}", e))?;
            }
        }
        "hover" => {
            UIAutomation::move_mouse(x, y)
                .await
                .map_err(|e| format!("Hover failed: {}", e))?;
        }
        _ => {
            return Err(format!("Unknown action type: {}", action_type));
        }
    }

    Ok(())
}
