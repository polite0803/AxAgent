use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde_json::json;
use std::time::Instant;

use axagent_harness::ProviderRequestContext;
use axagent_harness::types::*;
use axagent_harness::url_utils::resolve_base_url_for_type;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::{error_response, record_log};
use crate::server::GatewayAppState;

/// GET /v1/responses/{response_id} — retrieve a stored response
pub async fn get_response(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(response_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let providers: Vec<ProviderConfig> = match state.adapter.providers().list_providers().await {
        Ok(p) => p
            .into_iter()
            .filter(|p| {
                matches!(
                    p.provider_type,
                    ProviderType::OpenAI
                        | ProviderType::OpenClaw
                        | ProviderType::Hermes
                        | ProviderType::OpenAIResponses
                )
            })
            .collect(),
        Err(e) => {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        },
    };

    let provider = match providers.first() {
        Some(p) => p,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No Responses API provider configured");
        },
    };

    let provider_key = match state.adapter.providers().get_active_key(&provider.id).await {
        Ok(k) => k,
        Err(_) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("No active API key for provider '{}'", provider.name),
            );
        },
    };

    let api_key = match state
        .adapter
        .crypto()
        .decrypt_key(&provider_key.key_encrypted)
    {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to decrypt provider key: {}", e);
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal key error");
        },
    };

    let global_settings = state
        .adapter
        .settings()
        .get_settings()
        .await
        .unwrap_or_default();
    let resolved_proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    let ctx = ProviderRequestContext {
        api_key,
        key_id: provider_key.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: resolved_proxy,
        custom_headers: None,
        api_mode: Some("codex_responses".to_string()),
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let adapter = match state.provider_registry.get("openai_responses") {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No Responses API adapter available");
        },
    };

    match adapter.get_response(&ctx, &response_id).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/v1/responses/{}", response_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/v1/responses/{}", response_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to get response: {}", e))
        },
    }
}

/// DELETE /v1/responses/{response_id} — delete a stored response
pub async fn delete_response(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(response_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let providers: Vec<ProviderConfig> = match state.adapter.providers().list_providers().await {
        Ok(p) => p
            .into_iter()
            .filter(|p| {
                matches!(
                    p.provider_type,
                    ProviderType::OpenAI
                        | ProviderType::OpenClaw
                        | ProviderType::Hermes
                        | ProviderType::OpenAIResponses
                )
            })
            .collect(),
        Err(e) => {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        },
    };

    let provider = match providers.first() {
        Some(p) => p,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No Responses API provider configured");
        },
    };

    let provider_key = match state.adapter.providers().get_active_key(&provider.id).await {
        Ok(k) => k,
        Err(_) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("No active API key for provider '{}'", provider.name),
            );
        },
    };

    let api_key = match state
        .adapter
        .crypto()
        .decrypt_key(&provider_key.key_encrypted)
    {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to decrypt provider key: {}", e);
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal key error");
        },
    };

    let global_settings = state
        .adapter
        .settings()
        .get_settings()
        .await
        .unwrap_or_default();
    let resolved_proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    let ctx = ProviderRequestContext {
        api_key,
        key_id: provider_key.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: resolved_proxy,
        custom_headers: None,
        api_mode: Some("codex_responses".to_string()),
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let adapter = match state.provider_registry.get("openai_responses") {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No Responses API adapter available");
        },
    };

    match adapter.delete_response(&ctx, &response_id).await {
        Ok(_) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "DELETE",
                &format!("/v1/responses/{}", response_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            Json(json!({ "deleted": true, "id": response_id })).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "DELETE",
                &format!("/v1/responses/{}", response_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to delete response: {}", e))
        },
    }
}
