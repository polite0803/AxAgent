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
use std::sync::Arc;
use std::time::Instant;
use tokio_stream::wrappers::ReceiverStream;

use axagent_harness::conversation_model::TokenUsage;
use axagent_harness::types::*;
use axagent_harness::url_utils::resolve_base_url_for_type;
use axagent_harness::usage_pricing::{cost_for_tokens, pricing_for_model};
use axagent_harness::{ProviderAdapter, ProviderRequestContext};

use crate::auth::AuthenticatedKey;
use crate::handlers::error::{error_response, provider_type_to_str, record_log};
use crate::handlers::models::{
    build_provider_public_id_map, parse_model_field, resolve_provider_for_model,
};
use crate::routing::select_provider_index;
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

/// 基于实际 token 用量估算美元成本。
///
/// 优先级：
/// 1. provider 自带的 per-model 价格（`input_price_per_mtok` / `output_price_per_mtok`）。
///    此路径不覆盖 cache 维度（provider 元数据未提供 cache 单价），按 input+output 计算。
/// 2. 全局定价表 [`pricing_for_model`]，覆盖 input/output/cache_creation/cache_read 四维。
/// 3. 都无定价信息时返回 `0.0`，dao 层原样落库（不阻断用量记录）。
///
/// 设计原则：成本估算为"尽力而为"，缺失定价时降级而非报错，
/// 避免网关热路径因定价表滞后而拒绝记录用量。
fn estimate_cost_usd(model_id: &str, usage: &TokenUsage, provider_model: Option<&Model>) -> f64 {
    // 1) provider 自带价格
    if let Some(m) = provider_model
        && let (Some(inp), Some(out)) = (m.input_price_per_mtok, m.output_price_per_mtok)
    {
        return cost_for_tokens(usage.input_tokens, inp)
            + cost_for_tokens(usage.output_tokens, out);
    }
    // 2) 全局定价表
    if let Some(pricing) = pricing_for_model(model_id) {
        return pricing.cost_for(*usage).total_cost_usd();
    }
    // 3) 未知定价
    0.0
}

/// 在 provider 的 models 列表中查找指定 model_id（enabled）。
fn find_enabled_model<'a>(provider: &'a ProviderConfig, model_id: &str) -> Option<&'a Model> {
    provider.models.iter().find(|m| m.enabled && m.model_id == model_id)
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

    // 解析 provider 与 canonical model_id。
    //
    // 智能路由分支：当 model 字段为 bare name（无 provider 前缀）且被多个
    // enabled provider 同时支持时，按 `routing_strategy` 选首选 provider；
    // 其余场景（显式前缀 / 唯一匹配 / 无匹配）保持原有 `resolve_provider_for_model`
    // 行为不变，避免破坏存量显式指定 provider 的请求。
    let (provider, model_id) = match &parsed.provider_hint {
        None => {
            // 收集所有 enabled 且支持该 model 的 provider。
            let candidates: Vec<&ProviderConfig> = providers
                .iter()
                .filter(|p| {
                    p.enabled && p.models.iter().any(|m| m.enabled && m.model_id == parsed.model_id)
                })
                .collect();

            match candidates.len() {
                0 => {
                    // 无匹配 → 走原有解析逻辑（返回 NOT_FOUND）
                    match resolve_provider_for_model(&providers, &public_id_map, &parsed) {
                        Ok(pair) => pair,
                        Err(resp) => return resp,
                    }
                },
                1 => {
                    // 唯一匹配 → 直接用（与原逻辑一致）
                    ((*candidates[0]).clone(), parsed.model_id.clone())
                },
                _ => {
                    // 多匹配 → 智能路由
                    let idx = match select_provider_index(
                        state.routing_strategy,
                        &candidates,
                        &parsed.model_id,
                        &state.latency_tracker,
                        &state.round_robin_cursor,
                    ) {
                        Some(i) => i,
                        None => {
                            return error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "No provider available for routing",
                            );
                        },
                    };
                    ((*candidates[idx]).clone(), parsed.model_id.clone())
                },
            }
        },
        Some(_) => {
            // 显式 provider 前缀 → 走原有解析逻辑
            match resolve_provider_for_model(&providers, &public_id_map, &parsed) {
                Ok(pair) => pair,
                Err(resp) => return resp,
            }
        },
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

    // 取 provider 自带的 model 元数据（用于成本估算的 per-model 价格）。
    // 缺失时回退到全局定价表，不影响用量记录。
    let provider_model = find_enabled_model(&provider, &model_id).cloned();

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
            provider_model,
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
            provider_model.as_ref(),
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
    provider_model: Option<&Model>,
    start_time: Instant,
) -> axum::response::Response {
    let mut current_ctx = initial_ctx.clone();
    let mut last_error: Option<axagent_harness::core_error::AxAgentError> = None;
    let request = Arc::new(request);

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
                // 估算成本并记录用量
                let cost_usd = estimate_cost_usd(model_id, &response.usage, provider_model);
                let _ = state
                    .adapter
                    .gateway_keys()
                    .record_usage(
                        &gateway_key.id,
                        provider_id,
                        Some(model_id),
                        response.usage.input_tokens as u64,
                        response.usage.output_tokens as u64,
                        response.usage.cache_read_input_tokens as u64,
                        cost_usd,
                    )
                    .await;

                let elapsed = start_time.elapsed().as_millis() as i64;
                // 记录延迟样本，供 Latency 策略使用
                state.latency_tracker.record(provider_id, elapsed as u64);
                record_log!(
                    &state.adapter,
                    gateway_key,
                    "POST",
                    "/v1/chat/completions",
                    Some(model_id),
                    provider_id,
                    200,
                    elapsed,
                    response.usage.input_tokens as i64,
                    response.usage.output_tokens as i64,
                    None
                );

                return Json(build_non_stream_response_body(&response)).into_response();
            },
            Err(e) => {
                let should_retry = attempt < KEY_FAILOVER_MAX_RETRIES
                    && (matches!(&e, axagent_harness::core_error::AxAgentError::Provider(msg) if is_retriable_key_error(msg))
                        || matches!(&e, axagent_harness::core_error::AxAgentError::Execution { context, .. } if is_retriable_key_error(context)));

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

    let elapsed = start_time.elapsed().as_millis() as i64;
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
    provider_model: Option<Model>,
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
    let latency_tracker = state.latency_tracker.clone();
    let key = gateway_key.clone();
    let prov_id = provider_id.to_string();
    let mod_id = model_id.to_string();

    tokio::spawn(async move {
        let mut total_prompt = 0u32;
        let mut total_completion = 0u32;
        let mut total_cached = 0u32;
        let mut total_cache_creation = 0u32;
        let mut stream_error: Option<String> = None;

        // G7 dojo.v2 Typed Event：为本次 stream 维护事件序列器
        let run_id = crate::handlers::dojo_event::DojoEventSequencer::generate_run_id();
        let mut sequencer = crate::handlers::dojo_event::DojoEventSequencer::new(&run_id);

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
                        total_prompt = usage.input_tokens;
                        total_completion = usage.output_tokens;
                        total_cached = usage.cache_read_input_tokens;
                        total_cache_creation = usage.cache_creation_input_tokens;
                    }

                    if chunk.done {
                        // 流结束前关闭推理段（如果存在）
                        let mut final_events = Vec::new();
                        if let Some(think_end) = sequencer.close_thinking_if_open() {
                            final_events.push(think_end);
                        }
                        final_events.push(sequencer.done_event());

                        let data = build_stream_final_response_body(
                            &model_str,
                            total_prompt,
                            total_completion,
                            total_cached,
                            total_cache_creation,
                            &final_events,
                        );
                        if tx.send(Ok(Event::default().data(data.to_string()))).await.is_err() {
                            break;
                        }
                        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                        break;
                    }

                    // 根据 chunk 内容生成 dojo_event 列表
                    let dojo_events = sequencer.events_for_chunk(&chunk);
                    if let Some(data) =
                        build_stream_chunk_response_body(&model_str, &chunk, &dojo_events)
                        && tx.send(Ok(Event::default().data(data.to_string()))).await.is_err()
                    {
                        // Client disconnected after checking is_closed() but before sending
                        // Break immediately to avoid wasting more tokens
                        break;
                    }
                },
                Err(e) => {
                    stream_error = Some(e.to_string());
                    tracing::error!(error = %e, "Stream processing error in chat completion");
                    // 发送 dojo error 事件 + OpenAI 兼容错误
                    let err_event = sequencer.error_event("Stream processing error");
                    let data = json!({
                        "error": { "message": "Stream processing error" },
                        "dojo_event": [err_event],
                    });
                    let _ = tx.send(Ok(Event::default().data(data.to_string()))).await;
                    break;
                },
            }
        }

        // 估算成本并记录用量
        let usage = TokenUsage {
            input_tokens: total_prompt,
            output_tokens: total_completion,
            cache_creation_input_tokens: total_cache_creation,
            cache_read_input_tokens: total_cached,
            cache_miss_input_tokens: None,
        };
        let cost_usd = estimate_cost_usd(&mod_id, &usage, provider_model.as_ref());
        let _ = platform_adapter
            .gateway_keys()
            .record_usage(
                &key.id,
                &prov_id,
                Some(&mod_id),
                total_prompt as u64,
                total_completion as u64,
                total_cached as u64,
                cost_usd,
            )
            .await;

        let elapsed = start_time.elapsed().as_millis() as i64;
        // 记录延迟样本，供 Latency 策略使用
        latency_tracker.record(&prov_id, elapsed as u64);
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
        ("prompt_tokens".to_string(), json!(response.usage.input_tokens)),
        ("completion_tokens".to_string(), json!(response.usage.output_tokens)),
        ("total_tokens".to_string(), json!(response.usage.total_tokens())),
        (
            "prompt_tokens_details".to_string(),
            json!({
                "cached_tokens": response.usage.cache_read_input_tokens,
            }),
        ),
    ]);
    if response.usage.cache_creation_input_tokens > 0 {
        usage.insert(
            "cache_creation_input_tokens".to_string(),
            json!(response.usage.cache_creation_input_tokens),
        );
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
    dojo_events: &[crate::handlers::dojo_event::DojoEvent],
) -> Option<serde_json::Value> {
    let mut delta = serde_json::Map::new();

    if let Some(content) = chunk.content.as_deref().filter(|value| !value.is_empty()) {
        delta.insert("content".to_string(), json!(content));
    }
    if let Some(reasoning) = chunk.thinking.as_deref().filter(|value| !value.is_empty()) {
        delta.insert("reasoning_content".to_string(), json!(reasoning));
    }

    // P0: 不再丢弃 delta 为空的 chunk。初始 role chunk、usage 更新 chunk
    // 等都可能 content/thinking 为空，但对客户端建立会话和跟踪流状态至关重要。
    // 即使 delta 和 dojo_events 都为空也返回带空 delta 的 chunk，保证 SSE 流心跳连续。
    let mut body = json!({
        "id": "chatcmpl-gateway",
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{
            "index": 0,
            "delta": delta,
            "finish_reason": null,
        }]
    });
    if !dojo_events.is_empty() {
        body["dojo_event"] = json!(dojo_events);
    }
    Some(body)
}

pub(crate) fn build_stream_final_response_body(
    model: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    cached_tokens: u32,
    cache_creation_tokens: u32,
    dojo_events: &[crate::handlers::dojo_event::DojoEvent],
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

    let mut body = json!({
        "id": "chatcmpl-gateway",
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": "stop",
        }],
        "usage": usage,
    });
    if !dojo_events.is_empty() {
        body["dojo_event"] = json!(dojo_events);
    }
    body
}
