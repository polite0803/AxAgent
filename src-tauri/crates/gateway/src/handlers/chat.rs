// SPDX-License-Identifier: AGPL-3.0-only
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{
    IntoResponse, Json,
    sse::{Event, KeepAlive, Sse},
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

/// Maximum key failover retries for a single request.
const KEY_FAILOVER_MAX_RETRIES: u32 = 2;

/// Checks whether a provider error is retriable by switching to the next key.
fn is_retriable_key_error(msg: &str) -> bool {
    msg.contains("API error 401")
        || msg.contains("API error 403")
        || msg.contains("API error 429")
        || msg.contains("401 Unauthorized")
        || msg.contains("403 Forbidden")
        || msg.contains("rate limit")
}

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
        Err(_e) => {
            tracing::error!(error = ?_e, "Failed to list providers for chat completion");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occurred while listing providers",
            );
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

    let api_key = match state.adapter.crypto().decrypt_key(&provider_key.key_encrypted) {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to decrypt provider key: {}", e);
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal key error");
        },
    };

    let provider_type_str = provider_type_to_str(&provider.provider_type);

    let global_settings = state.adapter.settings().get_settings().await.unwrap_or_default();
    let resolved_proxy = axagent_harness::types::provider_model::resolve_provider_proxy(
        &provider.proxy_config,
        &global_settings,
    );

    let ctx = ProviderRequestContext {
        api_key,
        key_id: provider_key.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: resolved_proxy,
        custom_headers: provider.custom_headers.as_ref().and_then(|s| {
            serde_json::from_str(s)
                .map_err(|e| tracing::warn!(error = %e, "Failed to parse custom headers"))
                .ok()
        }),
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
        handle_non_stream_with_failover(
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

/// Non-streaming handler with automatic key failover.
///
/// On 401/403/429 errors, the current key is reported as failed and the
/// next available key (via the provider's round-robin) is tried, up to
/// `KEY_FAILOVER_MAX_RETRIES`.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_non_stream_with_failover(
    adapter: &dyn ProviderAdapter,
    initial_ctx: &ProviderRequestContext,
    request: ChatRequest,
    state: &GatewayAppState,
    gateway_key: &GatewayKey,
    provider_id: &str,
    model_id: &str,
    start_time: Instant,
) -> axum::response::Response {
    let mut current_ctx = initial_ctx.clone();
    let mut last_error: Option<axagent_harness::core_error::AxAgentError> = None;

    for attempt in 0..=KEY_FAILOVER_MAX_RETRIES {
        match adapter.chat(&current_ctx, request.clone()).await {
            Ok(response) => {
                if attempt > 0 {
                    tracing::info!(
                        key_id = %current_ctx.key_id,
                        attempt,
                        "Chat completion succeeded after key failover"
                    );
                }
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

                return Json(build_non_stream_response_body(&response)).into_response();
            },
            Err(e) => {
                let should_retry = attempt < KEY_FAILOVER_MAX_RETRIES
                    && matches!(&e, axagent_harness::core_error::AxAgentError::Provider(msg) if is_retriable_key_error(msg));

                if should_retry {
                    let _ = state
                        .adapter
                        .providers()
                        .report_key_failure(&current_ctx.key_id, &e.to_string())
                        .await;

                    match state.adapter.providers().get_active_key(provider_id).await {
                        Ok(new_key) if new_key.id != current_ctx.key_id => {
                            match state.adapter.crypto().decrypt_key(&new_key.key_encrypted) {
                                Ok(new_api_key) => {
                                    tracing::info!(
                                        old_key = %current_ctx.key_id,
                                        new_key = %new_key.id,
                                        attempt,
                                        "Failing over to next API key"
                                    );
                                    current_ctx.key_id = new_key.id;
                                    current_ctx.api_key = new_api_key;
                                    continue;
                                },
                                Err(decrypt_err) => {
                                    tracing::error!(
                                        key_id = %new_key.id,
                                        error = %decrypt_err,
                                        "Failed to decrypt failover key"
                                    );
                                },
                            }
                        },
                        Ok(_) => {
                            tracing::debug!("No alternative key available for failover");
                        },
                        Err(err) => {
                            tracing::warn!(error = %err, "Failed to fetch next key for failover");
                        },
                    }
                }

                last_error = Some(e);
                break;
            },
        }
    }

    let elapsed = start_time.elapsed().as_millis() as i32;
    let err_msg = last_error.as_ref().map(|e| e.to_string()).unwrap_or_default();
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
        Some(&err_msg)
    );

    tracing::error!(error = %err_msg, provider = %provider_id, model = %model_id, "Chat completion request failed");
    error_response(StatusCode::BAD_GATEWAY, "Chat completion request failed")
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
    let llm_config = axagent_harness::LlmCallConfig::default();
    let mut stream =
        match axagent_harness::execute_llm_stream(adapter, ctx, request, &llm_config, None).await {
            Ok(s) => s,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, &e),
        };

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
            // P2-22: 显式监听客户端断开 —— tx.closed() 触发时立即跳出，
            // 避免 provider 继续耗 token 推给已断开的客户端。
            if tx.is_closed() {
                tracing::info!(
                    provider = %prov_id,
                    model = %model_str,
                    chunks_sent = total_completion,
                    "SSE 客户端已断开，提前终止上游 stream"
                );
                break;
            }
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
                        && tx.send(Ok(Event::default().data(data.to_string()))).await.is_err()
                    {
                        break;
                    }
                },
                Err(e) => {
                    stream_error = Some(e.to_string());
                    tracing::error!(error = %e, "Stream processing error in chat completion");
                    let data = json!({
                        "error": { "message": "Stream processing error" }
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
    Sse::new(sse_stream).keep_alive(KeepAlive::default()).into_response()
}

fn build_non_stream_response_body(response: &ChatResponse) -> serde_json::Value {
    let mut message = serde_json::Map::from_iter([
        ("role".to_string(), json!("assistant")),
        ("content".to_string(), json!(response.content)),
    ]);
    if let Some(reasoning) = response.thinking.as_deref().filter(|value| !value.is_empty()) {
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
