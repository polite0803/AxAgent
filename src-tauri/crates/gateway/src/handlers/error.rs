use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use std::sync::Arc;

use axagent_harness::types::{ProviderConfig, ProviderType};
use axagent_harness::url_utils::resolve_base_url_for_type;
use axagent_harness::{ProviderProxyConfig, ProviderRequestContext};

/// `record_log!` — record a request log entry via the platform adapter.
///
/// Used by every gateway handler to emit a usage / latency / error row
/// regardless of the response status.  Failures are downgraded to
/// `tracing::warn!` so the handler is not affected by a log-table outage.
#[macro_export]
macro_rules! record_log {
    ($adapter:expr, $key:expr, $method:expr, $path:expr, $model_id:expr, $provider_id:expr, $status:expr, $elapsed:expr, $prompt:expr, $completion:expr, $error:expr) => {
        let _ = $adapter
            .request_log()
            .record_request_log(
                &$key.id,
                &$key.name,
                $method,
                $path,
                $model_id,
                Some($provider_id),
                $status,
                $elapsed,
                $prompt,
                $completion,
                $error,
            )
            .await
            .map_err(|e| tracing::warn!(%e, "Failed to record request log"))
            .ok();
    };
}
pub use record_log;

/// Standard JSON error envelope used by every gateway handler.
pub(crate) fn error_response(status: StatusCode, message: &str) -> axum::response::Response {
    (
        status,
        Json(json!({
            "error": {
                "message": message,
                "type": "api_error",
            }
        })),
    )
        .into_response()
}

/// Map `ProviderType` to the string key used in the `ProviderRegistry`.
pub(crate) fn provider_type_to_str(pt: &ProviderType) -> &'static str {
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

/// Resolve the Hermes / OpenClaw provider and build a `ProviderRequestContext`.
///
/// All `/api/jobs*` endpoints are proxy-routed to whichever Hermes- or
/// OpenClaw-typed provider is configured first.  This helper centralises
/// the "find provider → fetch & decrypt active key → build ctx" dance so
/// each handler stays a thin pass-through.
pub(crate) async fn resolve_hermes_provider_context(
    adapter: &Arc<dyn axagent_harness::PlatformAdapter>,
    _registry: &dyn axagent_harness::registry::ProviderRegistry,
) -> Result<(ProviderConfig, ProviderRequestContext), axum::response::Response> {
    let providers: Vec<ProviderConfig> = match adapter.providers().list_providers().await {
        Ok(p) => p
            .into_iter()
            .filter(|p| matches!(p.provider_type, ProviderType::OpenClaw | ProviderType::Hermes))
            .collect(),
        Err(e) => {
            return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()));
        },
    };

    let provider = match providers.first() {
        Some(p) => p.clone(),
        None => {
            return Err(error_response(
                StatusCode::BAD_GATEWAY,
                "No Hermes/OpenClaw provider configured",
            ));
        },
    };

    let provider_key = match adapter.providers().get_active_key(&provider.id).await {
        Ok(k) => k,
        Err(_) => {
            return Err(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("No active API key for provider '{}'", provider.name),
            ));
        },
    };

    let api_key = match adapter.crypto().decrypt_key(&provider_key.key_encrypted) {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to decrypt provider key: {}", e);
            return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal key error"));
        },
    };

    let global_settings = adapter.settings().get_settings().await.unwrap_or_default();
    let resolved_proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    let ctx = ProviderRequestContext {
        api_key,
        key_id: provider_key.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: resolved_proxy,
        custom_headers: None,
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    Ok((provider, ctx))
}
