use crate::AppState;
use axagent_core::types::*;
use std::time::Instant;
use tauri::State;

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> Result<Vec<ProviderConfig>, String> {
    axagent_core::repo::provider::list_providers_merged(&state.sea_db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_provider(
    state: State<'_, AppState>,
    input: CreateProviderInput,
) -> Result<ProviderConfig, String> {
    axagent_core::repo::provider::create_provider(&state.sea_db, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_provider(
    state: State<'_, AppState>,
    id: String,
    input: UpdateProviderInput,
) -> Result<ProviderConfig, String> {
    let real_id = axagent_core::repo::provider::resolve_provider_id(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())?;
    axagent_core::repo::provider::update_provider(&state.sea_db, &real_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_provider(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Virtual built-in providers have no DB row — deletion is a no-op (they'll reappear)
    if id.starts_with("builtin_") {
        return Ok(());
    }
    axagent_core::repo::provider::delete_provider(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_provider(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let real_id = axagent_core::repo::provider::resolve_provider_id(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())?;
    axagent_core::repo::provider::toggle_provider(&state.sea_db, &real_id, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_provider_key(
    state: State<'_, AppState>,
    provider_id: String,
    raw_key: String,
) -> Result<ProviderKey, String> {
    let real_id = axagent_core::repo::provider::resolve_provider_id(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;
    let encrypted =
        axagent_core::crypto::encrypt_key(&raw_key, &state.master_key).map_err(|e| e.to_string())?;
    let prefix = if raw_key.len() >= 8 {
        format!("{}...", &raw_key[..8])
    } else {
        raw_key.clone()
    };
    axagent_core::repo::provider::add_provider_key(&state.sea_db, &real_id, &encrypted, &prefix)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_provider_key(
    state: State<'_, AppState>,
    key_id: String,
    raw_key: String,
) -> Result<ProviderKey, String> {
    let encrypted =
        axagent_core::crypto::encrypt_key(&raw_key, &state.master_key).map_err(|e| e.to_string())?;
    let prefix = if raw_key.len() >= 8 {
        format!("{}...", &raw_key[..8])
    } else {
        raw_key.clone()
    };
    axagent_core::repo::provider::update_provider_key(&state.sea_db, &key_id, &encrypted, &prefix)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_provider_key(state: State<'_, AppState>, key_id: String) -> Result<(), String> {
    axagent_core::repo::provider::delete_provider_key(&state.sea_db, &key_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_provider_key(
    state: State<'_, AppState>,
    key_id: String,
    enabled: bool,
) -> Result<(), String> {
    axagent_core::repo::provider::toggle_provider_key(&state.sea_db, &key_id, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_decrypted_provider_key(
    state: State<'_, AppState>,
    key_id: String,
) -> Result<String, String> {
    let key_row = axagent_core::repo::provider::get_provider_key(&state.sea_db, &key_id)
        .await
        .map_err(|e| e.to_string())?;
    axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_provider_key(
    state: State<'_, AppState>,
    key_id: String,
) -> Result<bool, String> {
    let key_row = axagent_core::repo::provider::get_provider_key(&state.sea_db, &key_id)
        .await
        .map_err(|e| e.to_string())?;
    let decrypted = axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
        .map_err(|e| e.to_string())?;
    let provider = axagent_core::repo::provider::get_provider(&state.sea_db, &key_row.provider_id)
        .await
        .map_err(|e| e.to_string())?;
    // Use the registry to validate by listing models
    let registry = axagent_providers::registry::ProviderRegistry::create_default();
    let provider_type_str = match provider.provider_type {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    };
    let adapter = registry
        .get(provider_type_str)
        .ok_or_else(|| format!("No adapter for provider type: {}", provider_type_str))?;
    let global_settings = axagent_core::repo::settings::get_settings(&state.sea_db)
        .await
        .unwrap_or_default();
    let resolved_proxy =
        axagent_core::types::ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);
    let ctx = axagent_providers::ProviderRequestContext {
        api_key: decrypted,
        key_id: key_id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(axagent_providers::resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
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
    let valid = match adapter.validate_key(&ctx).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Key validation failed for key {}: {}", key_id, e);
            // Update as invalid, then return the error
            let _ = axagent_core::repo::provider::update_key_validation(&state.sea_db, &key_id, false).await;
            return Err(e.to_string());
        }
    };
    // Update validation timestamp
    axagent_core::repo::provider::update_key_validation(&state.sea_db, &key_id, valid)
        .await
        .map_err(|e| e.to_string())?;
    Ok(valid)
}

#[tauri::command]
pub async fn save_models(
    state: State<'_, AppState>,
    provider_id: String,
    models: Vec<Model>,
) -> Result<(), String> {
    let real_id = axagent_core::repo::provider::resolve_provider_id(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;
    axagent_core::repo::provider::save_models(&state.sea_db, &real_id, &models)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_model(
    state: State<'_, AppState>,
    provider_id: String,
    model_id: String,
    enabled: bool,
) -> Result<Model, String> {
    let real_id = axagent_core::repo::provider::resolve_provider_id(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;
    axagent_core::repo::provider::toggle_model(&state.sea_db, &real_id, &model_id, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_model_params(
    state: State<'_, AppState>,
    provider_id: String,
    model_id: String,
    overrides: ModelParamOverrides,
) -> Result<Model, String> {
    let real_id = axagent_core::repo::provider::resolve_provider_id(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;
    axagent_core::repo::provider::update_model_params(
        &state.sea_db,
        &real_id,
        &model_id,
        overrides,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_remote_models(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<Vec<Model>, String> {
    let real_id = axagent_core::repo::provider::resolve_provider_id(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;
    let provider = axagent_core::repo::provider::get_provider(&state.sea_db, &real_id)
        .await
        .map_err(|e| e.to_string())?;
    // Get an enabled key for the provider
    let key_row = axagent_core::repo::provider::get_active_key(&state.sea_db, &real_id)
        .await
        .map_err(|e| e.to_string())?;
    let decrypted = axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
        .map_err(|e| e.to_string())?;
    let registry = axagent_providers::registry::ProviderRegistry::create_default();
    let provider_type_str = match provider.provider_type {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    };
    let adapter = registry
        .get(provider_type_str)
        .ok_or_else(|| format!("No adapter for provider type: {}", provider_type_str))?;
    let global_settings = axagent_core::repo::settings::get_settings(&state.sea_db)
        .await
        .unwrap_or_default();
    let resolved_proxy =
        axagent_core::types::ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);
    let ctx = axagent_providers::ProviderRequestContext {
        api_key: decrypted,
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(axagent_providers::resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
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
    adapter.list_models(&ctx).await.map_err(|e| e.to_string())
}

/// Test a single model's availability by sending a minimal chat request.
/// Returns latency in milliseconds on success.
#[tauri::command]
pub async fn test_model(
    state: State<'_, AppState>,
    provider_id: String,
    model_id: String,
) -> Result<u64, String> {
    let real_id = axagent_core::repo::provider::resolve_provider_id(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;
    let provider = axagent_core::repo::provider::get_provider(&state.sea_db, &real_id)
        .await
        .map_err(|e| e.to_string())?;
    let key_row = axagent_core::repo::provider::get_active_key(&state.sea_db, &real_id)
        .await
        .map_err(|e| e.to_string())?;
    let decrypted = axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
        .map_err(|e| e.to_string())?;
    let registry = axagent_providers::registry::ProviderRegistry::create_default();
    let provider_type_str = match provider.provider_type {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    };
    let adapter = registry
        .get(provider_type_str)
        .ok_or_else(|| format!("No adapter for provider type: {}", provider_type_str))?;
    let global_settings = axagent_core::repo::settings::get_settings(&state.sea_db)
        .await
        .unwrap_or_default();
    let resolved_proxy =
        axagent_core::types::ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);
    let ctx = axagent_providers::ProviderRequestContext {
        api_key: decrypted,
        key_id: key_row.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(axagent_providers::resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
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
    let request = ChatRequest {
        model: model_id,
        messages: vec![ChatMessage {
            role: "user".into(),
            content: ChatContent::Text("hi".into()),
            tool_calls: None,
            tool_call_id: None,
        }],
        stream: false,
        temperature: None,
        top_p: None,
        max_tokens: Some(1),
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
    };
    let start = Instant::now();
    adapter
        .chat(&ctx, request)
        .await
        .map_err(|e| e.to_string())?;
    Ok(start.elapsed().as_millis() as u64)
}

#[tauri::command]
pub async fn reorder_providers(
    state: State<'_, AppState>,
    provider_ids: Vec<String>,
) -> Result<(), String> {
    // Materialize any virtual built-in providers so sort_order can be persisted
    let mut real_ids = Vec::with_capacity(provider_ids.len());
    for id in &provider_ids {
        let real_id = axagent_core::repo::provider::resolve_provider_id(&state.sea_db, id)
            .await
            .map_err(|e| e.to_string())?;
        real_ids.push(real_id);
    }
    axagent_core::repo::provider::reorder_providers(&state.sea_db, &real_ids)
        .await
        .map_err(|e| e.to_string())
}

