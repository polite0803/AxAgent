use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{
    sse::{Event, KeepAlive, Sse},
    IntoResponse, Json,
};
use futures::StreamExt;
use serde_json::json;
use std::collections::HashSet;
use std::convert::Infallible;
use std::time::Instant;
use tokio_stream::wrappers::ReceiverStream;

use axagent_harness::types::*;
use axagent_harness::url_utils::resolve_base_url_for_type;
use axagent_harness::{ProviderAdapter, ProviderRequestContext};

use crate::auth::AuthenticatedKey;
use crate::handlers::error::{error_response, provider_type_to_str, record_log};
use crate::handlers::models::{
    build_provider_public_id_map, parse_model_field, resolve_provider_for_model,
};
use crate::server::GatewayAppState;

/// POST /v1/chat/completions — main proxy handler
pub async fn chat_completions(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(request): Json<ChatRequest>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    // Fetch providers once — used for both model-field parsing and resolution.
    // Filter to only chat-completions-compatible provider types.
    let providers: Vec<ProviderConfig> = match state.adapter.providers().list_providers().await {
        Ok(p) => p
            .into_iter()
            .filter(|p| {
                matches!(
                    p.provider_type,
                    ProviderType::OpenAI
                        | ProviderType::OpenClaw
                        | ProviderType::Hermes
                        | ProviderType::Ollama
                )
            })
            .collect(),
        Err(e) => {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        },
    };
    let public_id_map = build_provider_public_id_map(&providers);
    let known_public_ids: HashSet<String> = public_id_map.values().cloned().collect();

    // Parse model field: supports "provider_public_id/model_id" (preferred),
    // legacy "provider_id:model_id" (compat), or bare "model_id".
    let parsed = parse_model_field(&request.model, &known_public_ids);

    // Resolve the provider and canonical model_id.
    let (provider, model_id) = match resolve_provider_for_model(&providers, &public_id_map, &parsed)
    {
        Ok(pair) => pair,
        Err(resp) => return resp,
    };

    // Get active key and decrypt
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

    let provider_type_str = provider_type_to_str(&provider.provider_type);

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
        custom_headers: provider
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: request.api_mode.clone(),
        conversation: request.conversation.clone(),
        previous_response_id: request.previous_response_id.clone(),
        store_response: request.store,
    };

    let adapter = match state.provider_registry.get(provider_type_str) {
        Some(a) => a,
        None => {
            // Fallback to openai-compatible for custom providers
            match state.provider_registry.get("openai") {
                Some(a) => a,
                None => {
                    return error_response(
                        StatusCode::BAD_GATEWAY,
                        &format!("No adapter for provider type '{}'", provider_type_str),
                    );
                },
            }
        },
    };

    let adapter_ref: &dyn ProviderAdapter = &*adapter;
    if request.stream {
        handle_stream(
            adapter_ref,
            &ctx,
            request,
            &state,
            &gateway_key,
            &provider.id,
            &model_id,
            start_time,
        )
        .await
    } else {
        handle_non_stream(
            adapter_ref,
            &ctx,
            request,
            &state,
            &gateway_key,
            &provider.id,
            &model_id,
            start_time,
        )
        .await
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_non_stream(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    request: ChatRequest,
    state: &GatewayAppState,
    gateway_key: &GatewayKey,
    provider_id: &str,
    model_id: &str,
    start_time: Instant,
) -> axum::response::Response {
    match adapter.chat(ctx, request).await {
        Ok(response) => {
            // Record usage
            let _ = state
                .adapter
                .gateway_keys()
                .record_usage(
                    &gateway_key.id,
                    provider_id,
                    Some(model_id),
                    response.usage.prompt_tokens as u64,
                    response.usage.completion_tokens as u64,
                    response.usage.cache_read_tokens.unwrap_or(0) as u64,
                )
                .await;

            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                "/v1/chat/completions",
                Some(model_id),
                provider_id,
                200,
                elapsed,
                response.usage.prompt_tokens as i64,
                response.usage.completion_tokens as i64,
                None
            );

            Json(build_non_stream_response_body(&response)).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                "/v1/chat/completions",
                Some(model_id),
                provider_id,
                502,
                elapsed,
                0,
                0,
                Some(&e.to_string())
            );

            error_response(StatusCode::BAD_GATEWAY, &e.to_string())
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_stream(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    request: ChatRequest,
    state: &GatewayAppState,
    gateway_key: &GatewayKey,
    provider_id: &str,
    model_id: &str,
    start_time: Instant,
) -> axum::response::Response {
    let model_str = model_id.to_string();
    let mut stream = adapter.chat_stream(ctx, request, None);

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(32);
    let platform_adapter = state.adapter.clone();
    let key = gateway_key.clone();
    let prov_id = provider_id.to_string();
    let mod_id = model_id.to_string();

    tokio::spawn(async move {
        let mut total_prompt = 0u32;
        let mut total_completion = 0u32;
        let mut total_cached = 0u32;
        let mut total_cache_creation = 0u32;
        let mut stream_error: Option<String> = None;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Some(usage) = &chunk.usage {
                        total_prompt = usage.prompt_tokens;
                        total_completion = usage.completion_tokens;
                        total_cached = usage.cache_read_tokens.unwrap_or(0);
                        total_cache_creation = usage.cache_creation_tokens.unwrap_or(0);
                    }

                    if chunk.done {
                        let data = build_stream_final_response_body(
                            &model_str,
                            total_prompt,
                            total_completion,
                            total_cached,
                            total_cache_creation,
                        );
                        let _ = tx.send(Ok(Event::default().data(data.to_string()))).await;
                        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                        break;
                    }

                    if let Some(data) = build_stream_chunk_response_body(&model_str, &chunk)
                        && tx
                            .send(Ok(Event::default().data(data.to_string())))
                            .await
                            .is_err()
                    {
                        break;
                    }
                },
                Err(e) => {
                    stream_error = Some(e.to_string());
                    let data = json!({
                        "error": { "message": e.to_string() }
                    });
                    let _ = tx.send(Ok(Event::default().data(data.to_string()))).await;
                    break;
                },
            }
        }

        // Record usage
        let _ = platform_adapter
            .gateway_keys()
            .record_usage(
                &key.id,
                &prov_id,
                Some(&mod_id),
                total_prompt as u64,
                total_completion as u64,
                total_cached as u64,
            )
            .await;

        let elapsed = start_time.elapsed().as_millis() as i32;
        let status_code = if stream_error.is_some() { 502 } else { 200 };
        record_log!(
            &platform_adapter,
            key,
            "POST",
            "/v1/chat/completions",
            Some(&mod_id),
            &prov_id,
            status_code,
            elapsed,
            total_prompt as i64,
            total_completion as i64,
            stream_error.as_deref()
        );
    });

    let sse_stream = ReceiverStream::new(rx);
    Sse::new(sse_stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

fn build_non_stream_response_body(response: &ChatResponse) -> serde_json::Value {
    let mut message = serde_json::Map::from_iter([
        ("role".to_string(), json!("assistant")),
        ("content".to_string(), json!(response.content)),
    ]);
    if let Some(reasoning) = response
        .thinking
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        message.insert("reasoning_content".to_string(), json!(reasoning));
    }

    let mut usage = serde_json::Map::from_iter([
        ("prompt_tokens".to_string(), json!(response.usage.prompt_tokens)),
        ("completion_tokens".to_string(), json!(response.usage.completion_tokens)),
        ("total_tokens".to_string(), json!(response.usage.total_tokens)),
        (
            "prompt_tokens_details".to_string(),
            json!({
                "cached_tokens": response.usage.cache_read_tokens.unwrap_or(0),
            }),
        ),
    ]);
    if let Some(cache_creation) = response.usage.cache_creation_tokens
        && cache_creation > 0
    {
        usage.insert("cache_creation_input_tokens".to_string(), json!(cache_creation));
    }

    json!({
        "id": response.id,
        "object": "chat.completion",
        "model": response.model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": "stop",
        }],
        "usage": usage,
    })
}

pub(crate) fn build_stream_chunk_response_body(
    model: &str,
    chunk: &ChatStreamChunk,
) -> Option<serde_json::Value> {
    let mut delta = serde_json::Map::new();

    if let Some(content) = chunk.content.as_deref().filter(|value| !value.is_empty()) {
        delta.insert("content".to_string(), json!(content));
    }
    if let Some(reasoning) = chunk.thinking.as_deref().filter(|value| !value.is_empty()) {
        delta.insert("reasoning_content".to_string(), json!(reasoning));
    }

    if delta.is_empty() {
        None
    } else {
        Some(json!({
            "id": "chatcmpl-gateway",
            "object": "chat.completion.chunk",
            "model": model,
            "choices": [{
                "index": 0,
                "delta": delta,
                "finish_reason": null,
            }]
        }))
    }
}

pub(crate) fn build_stream_final_response_body(
    model: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    cached_tokens: u32,
    cache_creation_tokens: u32,
) -> serde_json::Value {
    let mut usage = serde_json::Map::from_iter([
        ("prompt_tokens".to_string(), json!(prompt_tokens)),
        ("completion_tokens".to_string(), json!(completion_tokens)),
        ("total_tokens".to_string(), json!(prompt_tokens + completion_tokens)),
        (
            "prompt_tokens_details".to_string(),
            json!({
                "cached_tokens": cached_tokens,
            }),
        ),
    ]);
    if cache_creation_tokens > 0 {
        usage.insert("cache_creation_input_tokens".to_string(), json!(cache_creation_tokens));
    }

    json!({
        "id": "chatcmpl-gateway",
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": "stop",
        }],
        "usage": usage,
    })
}
