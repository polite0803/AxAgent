// SPDX-License-Identifier: AGPL-3.0-only

//! 中心化 LLM 调用入口 — 所有约束在此生效
//!
//! 所有 `adapter.chat()` / `adapter.chat_stream()` 调用应当经过
//! [`execute_llm`] / [`execute_llm_stream`]，以获得统一的 PromptGuard 过滤、
//! 上下文窗口管理、缓存命中检查、后置处理和审计记录。
//!
//! 向后兼容：通过 [`LlmCallConfig`] 的 Option 字段控制各功能开关，
//! 不设置时走最少开销路径。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::task::{Context, Poll};
use std::time::Instant;

use crate::audit_trail::{AuditEntry, AuditRecorder};
use crate::cache_interceptor::LlmCacheKey;
use crate::confidence::{ConfidenceAction, ConfidenceConfig};
use crate::prompt_guard::PromptGuard;
use crate::provider::{ProviderAdapter, ProviderRequestContext};
use crate::types::{ChatContent, ChatRequest, ChatResponse, ChatStreamChunk, TokenUsage};
use futures::TryStreamExt;
use futures::stream::{self, Stream};

use crate::retry_policy::RetryPolicy;

/// LLM 调用结果（标准化包装器）
pub struct LlmCallResult {
    pub response: ChatResponse,
    pub usage: LlmUsage,
    pub duration_ms: u64,
    pub cached: bool,
}

impl LlmCallResult {
    pub fn from_raw(response: ChatResponse, duration_ms: u64, cached: bool) -> Self {
        let usage = LlmUsage {
            prompt_tokens: response.usage.prompt_tokens,
            completion_tokens: response.usage.completion_tokens,
            total_tokens: response.usage.total_tokens,
        };
        Self { response, usage, duration_ms, cached }
    }
}

/// Token 用量统计
#[derive(Default, Clone, Debug)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// LLM 调用配置 — 所有约束功能通过 Option 控制
#[derive(Clone)]
pub struct LlmCallConfig {
    /// PromptGuard 过滤器（可选）
    pub prompt_guard: Option<Arc<dyn PromptGuard>>,
    /// 上下文窗口上限（token 数），None 表示不限制
    pub max_context_tokens: Option<u32>,
    /// 为输出保留的 token 数（在 max_context_tokens 中预留）
    pub reserved_output_tokens: Option<u32>,
    /// 审计记录器（可选）
    pub audit_recorder: Option<Arc<dyn AuditRecorder>>,
    /// 是否启用严格模式
    pub strict_mode: bool,
    /// 是否启用缓存
    pub cache_enabled: bool,
    /// 会话标识（用于审计记录）
    pub session_id: Option<String>,
    /// 重试/降级策略（可选），配置后 execute_llm 自动带重试和超时
    pub retry_policy: Option<RetryPolicy>,
    /// 输入脱敏器（可选），对 LLM 输入中的敏感信息做屏蔽
    pub input_sanitizer: Option<Arc<dyn crate::tool::InputSanitizer>>,
    /// 置信度阈值（可选），低于阈值触发降级/拦截
    pub confidence_threshold: Option<f64>,
    /// 置信度配置（可选）
    pub confidence_config: Option<ConfidenceConfig>,
    /// 缓存拦截器（可选），配置后自动做缓存命中检查和写入
    pub cache: Option<Arc<dyn crate::cache_interceptor::HarnessCache>>,
    /// 缓存 TTL 秒数（默认 300）
    pub cache_ttl_secs: u64,
    /// 节点 ID（用于审计记录）
    pub node_id: Option<String>,
    /// 工作流 ID（用于审计记录）
    pub workflow_id: Option<String>,
}

impl Default for LlmCallConfig {
    fn default() -> Self {
        Self {
            prompt_guard: None,
            max_context_tokens: None,
            reserved_output_tokens: Some(4000),
            audit_recorder: None,
            strict_mode: false,
            cache_enabled: false,
            session_id: None,
            retry_policy: None,
            input_sanitizer: None,
            confidence_threshold: None,
            confidence_config: None,
            cache: None,
            cache_ttl_secs: 300,
            node_id: None,
            workflow_id: None,
        }
    }
}

/// 预处理后的调用请求（PromptGuard + 脱敏 + 截断 + 缓存键）
struct PreparedCall {
    request: ChatRequest,
    cache_key: Option<LlmCacheKey>,
}

/// 中心化 LLM 调用入口（一次性响应）
///
/// # 参数
/// - `adapter`: Provider 适配器
/// - `ctx`: Provider 请求上下文
/// - `request`: 待发送的聊天请求（会被修改：消息内容经 PromptGuard 过滤）
/// - `config`: 调用配置（约束开关）
///
/// # 返回
/// 标准化的 [`LlmCallResult`]，包含响应、用量、耗时和缓存状态。
pub async fn execute_llm(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    request: ChatRequest,
    config: &LlmCallConfig,
) -> Result<LlmCallResult, String> {
    let start = Instant::now();

    let prepared = prepare_llm_call(request, config)?;
    let request = prepared.request;
    let cache_key = prepared.cache_key;

    // ── 缓存命中检查 ──
    if let Some(ref cache) = config.cache
        && let Some(ref key) = cache_key
        && let Some(cached) = cache.get(key).await
    {
        tracing::info!("[execute_llm] 缓存命中: model={}", request.model);
        let cached_response: ChatResponse =
            serde_json::from_value(cached.clone()).unwrap_or_default();
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(LlmCallResult {
            response: cached_response,
            usage: LlmUsage::default(),
            duration_ms,
            cached: true,
        });
    }

    // ── 实际调用（带可选的重试策略包装） ──
    let response = if let Some(ref policy) = config.retry_policy {
        let cloned_request = request.clone();
        policy
            .execute_with_retry(|| async {
                adapter.chat(ctx, cloned_request.clone()).await.map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| {
                let err = format!("LLM 调用失败（重试耗尽）: {e}");
                tracing::error!("[execute_llm] {}", &err);
                record_failure_audit(config, &err, start);
                err
            })?
    } else {
        adapter.chat(ctx, request.clone()).await.map_err(|e| {
            let err = format!("LLM 调用失败: {e}");
            tracing::error!("[execute_llm] {}", &err);
            record_failure_audit(config, &err, start);
            err
        })?
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let result = LlmCallResult::from_raw(response, duration_ms, false);

    // ── 写入缓存（调用成功后） ──
    if let Some(ref cache) = config.cache
        && let Some(ref key) = cache_key
        && let Ok(val) = serde_json::to_value(&result.response)
    {
        cache.set(key.clone(), val, config.cache_ttl_secs).await;
    }

    // ── 后置：置信度检查（如果配置了阈值） ──
    if let Some(threshold) = config.confidence_threshold {
        let response_text = &result.response.content;
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response_text) {
            let confidence = parsed.get("confidence").and_then(|c| c.as_f64()).unwrap_or(1.0);
            if confidence < threshold {
                tracing::warn!(
                    "[execute_llm] 置信度 {:.2} 低于阈值 {:.2}，触发降级",
                    confidence,
                    threshold
                );
                if let Some(ref conf_cfg) = config.confidence_config {
                    match conf_cfg.on_low_confidence {
                        ConfidenceAction::Block => {
                            return Err(format!("低置信度: {confidence:.2} < {threshold:.2}"));
                        },
                        ConfidenceAction::WarnAndContinue => {
                            // 只是警告，继续
                        },
                        ConfidenceAction::FallbackToDefault => {
                            if let Some(ref default) = conf_cfg.default_output {
                                return Ok(LlmCallResult {
                                    response: ChatResponse {
                                        content: default.to_string(),
                                        ..Default::default()
                                    },
                                    usage: LlmUsage::default(),
                                    duration_ms,
                                    cached: false,
                                });
                            }
                        },
                    }
                }
            }
        }
    }

    // ── 审计记录 ──
    record_success_audit(config, &request, &result.response.content, start);

    tracing::debug!(
        "[execute_llm] 完成: {} tokens in {}ms",
        result.usage.total_tokens,
        result.duration_ms,
    );

    Ok(result)
}

/// 中心化 LLM 流式调用入口
///
/// 与 [`execute_llm`] 共享同一套前置约束（PromptGuard / 脱敏 / 上下文截断 /
/// 缓存命中短路 / 审计 / 置信度），返回按原样透传的 chunk 流。
///
/// 流结束后（收到 `done` chunk）执行审计与置信度检查；若置信度低于阈值且策略为
/// `Block`，会在流的末尾追加一个 `Err` 项。缓存写入在流结束后异步完成。
///
/// 注意：流式变体不应用 `retry_policy`（流式重试语义复杂，由各调用方自行决定）。
///
/// # 参数
/// - `adapter`: Provider 适配器
/// - `ctx`: Provider 请求上下文
/// - `request`: 待发送的聊天请求
/// - `config`: 调用配置（约束开关）
/// - `cancel_token`: 取消令牌（透传给 `chat_stream`）
pub async fn execute_llm_stream(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    request: ChatRequest,
    config: &LlmCallConfig,
    cancel_token: Option<Arc<AtomicBool>>,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamChunk, String>> + Send>>, String> {
    let prepared = prepare_llm_call(request, config)?;

    // ── 缓存命中短路：合成一个 content + done 流 ──
    if let Some(ref cache) = config.cache
        && let Some(ref key) = prepared.cache_key
        && let Some(cached) = cache.get(key).await
    {
        let cached_response: ChatResponse =
            serde_json::from_value(cached.clone()).unwrap_or_default();
        tracing::info!("[execute_llm_stream] 缓存命中: model={}", prepared.request.model);
        let usage = cached_response.usage.clone();
        let chunks: Vec<Result<ChatStreamChunk, String>> = vec![
            Ok(ChatStreamChunk {
                content: Some(cached_response.content),
                thinking: None,
                done: false,
                is_final: None,
                usage: None,
                tool_calls: None,
            }),
            Ok(ChatStreamChunk {
                content: None,
                thinking: None,
                done: true,
                is_final: None,
                usage: Some(usage),
                tool_calls: None,
            }),
        ];
        return Ok(Box::pin(stream::iter(chunks)));
    }

    let inner = Box::pin(
        adapter.chat_stream(ctx, prepared.request.clone(), cancel_token).map_err(|e| e.to_string()),
    );

    let wrapped = ExecuteLlmStream {
        inner,
        config: config.clone(),
        phase: StreamPhase::Streaming,
        content: String::new(),
        thinking: String::new(),
        usage: TokenUsage::default(),
        start: Instant::now(),
        prepared,
    };
    Ok(Box::pin(wrapped))
}

/// 共享预处理：PromptGuard 过滤 → 输入脱敏 → 上下文截断 → 构建缓存键
///
/// 若 PromptGuard 阻断，记录审计并返回 `Err`。
fn prepare_llm_call(
    mut request: ChatRequest,
    config: &LlmCallConfig,
) -> Result<PreparedCall, String> {
    // ── 1. 前置拦截：PromptGuard 过滤 ──
    if let Some(ref guard) = config.prompt_guard {
        for msg in &mut request.messages {
            match &mut msg.content {
                ChatContent::Text(text) => match guard.process_user_input(text) {
                    Ok(safe) => {
                        *text = safe;
                    },
                    Err(blocked) => {
                        let err = format!("PromptGuard 阻断: {blocked}");
                        tracing::warn!("[execute_llm] {}", &err);
                        record_block_audit(config, &err);
                        return Err(err);
                    },
                },
                ChatContent::Multipart(parts) => {
                    let mut modified = false;
                    for part in parts.iter_mut() {
                        if let Some(ref text) = part.text {
                            match guard.process_user_input(text) {
                                Ok(safe) => {
                                    if &safe != text {
                                        part.text = Some(safe);
                                        modified = true;
                                    }
                                },
                                Err(blocked) => {
                                    let err = format!("PromptGuard 阻断: {blocked}");
                                    tracing::warn!("[execute_llm] {}", &err);
                                    record_block_audit(config, &err);
                                    return Err(err);
                                },
                            }
                        }
                    }
                    if modified {
                        tracing::debug!("[execute_llm] PromptGuard 已过滤部分内容");
                    }
                },
            }
        }
    }

    // ── 1.5 输入脱敏 ──
    if let Some(ref sanitizer) = config.input_sanitizer {
        for msg in &mut request.messages {
            if let ChatContent::Text(ref text) = msg.content {
                msg.content = ChatContent::Text(sanitizer.sanitize_input(text, "llm_input"));
            }
        }
    }

    // ── 2. 上下文窗口管理（简易 token 估算） ──
    if let Some(max_tokens) = config.max_context_tokens {
        let reserved = config.reserved_output_tokens.unwrap_or(4000);
        let available_input_tokens = max_tokens.saturating_sub(reserved) as usize;

        let mut estimated_tokens: usize = request
            .messages
            .iter()
            .map(|m| {
                let text = match &m.content {
                    ChatContent::Text(t) => t.clone(),
                    ChatContent::Multipart(parts) => {
                        parts.iter().filter_map(|p| p.text.as_deref()).collect::<Vec<_>>().join(" ")
                    },
                };
                estimate_tokens(&text)
            })
            .sum();

        if estimated_tokens > available_input_tokens {
            tracing::warn!(
                "[execute_llm] 上下文估算 {estimated_tokens} token 超过限制 {available_input_tokens}，执行截断"
            );
            while estimated_tokens > available_input_tokens && request.messages.len() > 2 {
                if let Some(pos) = request.messages.iter().position(|m| m.role != "system") {
                    let text_content = match &request.messages[pos].content {
                        ChatContent::Text(t) => Some(t.clone()),
                        ChatContent::Multipart(parts) => {
                            let t = parts
                                .iter()
                                .filter_map(|p| p.text.as_deref())
                                .collect::<Vec<_>>()
                                .join(" ");
                            if t.is_empty() { None } else { Some(t) }
                        },
                    };
                    if let Some(text) = text_content {
                        let old_est = estimate_tokens(&text);

                        let summary_len = text.len() / 3;
                        let summary_text = &text[..summary_len.min(500)];
                        let summary = format!("[截断摘要] {summary_text}");

                        let new_est = estimate_tokens(&summary);

                        request.messages[pos].content = ChatContent::Text(summary);
                        estimated_tokens = estimated_tokens.saturating_sub(old_est) + new_est;
                    } else {
                        request.messages.remove(pos);
                        estimated_tokens = request
                            .messages
                            .iter()
                            .map(|m| {
                                let text = match &m.content {
                                    ChatContent::Text(t) => t.clone(),
                                    ChatContent::Multipart(parts) => parts
                                        .iter()
                                        .filter_map(|p| p.text.as_deref())
                                        .collect::<Vec<_>>()
                                        .join(" "),
                                };
                                estimate_tokens(&text)
                            })
                            .sum();
                    }
                } else {
                    break;
                }
            }
            tracing::info!(
                "[execute_llm] 上下文已截断，当前估算: {estimated_tokens}/{available_input_tokens}"
            );
        }
    }

    // ── 3. 缓存键 ──
    let cache_key = if config.cache.is_some() {
        Some(build_cache_key(&request))
    } else {
        None
    };

    Ok(PreparedCall { request, cache_key })
}

/// 简易 token 估算：中文 * 2 + 非中文 / 4 + 10
fn estimate_tokens(text: &str) -> usize {
    let chinese_chars: usize = text.chars().filter(|&c| c as u32 > 0x2E80).count();
    let non_chinese = text.len().saturating_sub(chinese_chars);
    chinese_chars * 2 + non_chinese / 4 + 10
}

/// 流式执行包装器：透传 chunk，在流结束后执行审计 / 置信度 / 缓存写入
struct ExecuteLlmStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatStreamChunk, String>> + Send>>,
    config: LlmCallConfig,
    phase: StreamPhase,
    content: String,
    thinking: String,
    usage: TokenUsage,
    start: Instant,
    prepared: PreparedCall,
}

#[derive(Default)]
enum StreamPhase {
    #[default]
    Streaming,
    /// 收到 done（或 inner 结束），待执行审计 / 置信度 / 缓存写入
    Post,
    /// 异步缓存写入进行中
    Caching(Option<Pin<Box<dyn Future<Output = ()> + Send>>>),
    Done,
}

impl Stream for ExecuteLlmStream {
    type Item = Result<ChatStreamChunk, String>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            let phase = std::mem::take(&mut this.phase);
            match phase {
                StreamPhase::Streaming => match this.inner.as_mut().poll_next(cx) {
                    Poll::Pending => {
                        this.phase = StreamPhase::Streaming;
                        return Poll::Pending;
                    },
                    Poll::Ready(None) => {
                        // inner 提前结束（无 done），进入后置阶段
                        this.phase = StreamPhase::Post;
                    },
                    Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                    Poll::Ready(Some(Ok(chunk))) => {
                        if let Some(c) = &chunk.content {
                            this.content.push_str(c);
                        }
                        if let Some(t) = &chunk.thinking {
                            this.thinking.push_str(t);
                        }
                        if let Some(ref u) = chunk.usage {
                            this.usage = u.clone();
                        }
                        this.phase = if chunk.done {
                            StreamPhase::Post
                        } else {
                            StreamPhase::Streaming
                        };
                        return Poll::Ready(Some(Ok(chunk)));
                    },
                },
                StreamPhase::Post => {
                    let duration_ms = this.start.elapsed().as_millis() as u64;

                    // 审计
                    if let Some(ref recorder) = this.config.audit_recorder {
                        let input_text = serde_json::to_string(&this.prepared.request.messages)
                            .unwrap_or_default();
                        let output_hash = sha256(&this.content);
                        recorder.record(AuditEntry {
                            execution_type: "llm_call".to_string(),
                            session_id: this.config.session_id.clone(),
                            node_id: this.config.node_id.clone(),
                            workflow_id: this.config.workflow_id.clone(),
                            tool_name: None,
                            input_hash: sha256(&input_text),
                            output_hash,
                            duration_ms,
                            status: "success".to_string(),
                            error: None,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            id: uuid::Uuid::new_v4().to_string(),
                        });
                    }

                    // 置信度
                    let mut block = false;
                    if let Some(threshold) = this.config.confidence_threshold {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&this.content)
                        {
                            let confidence =
                                parsed.get("confidence").and_then(|c| c.as_f64()).unwrap_or(1.0);
                            if confidence < threshold {
                                tracing::warn!(
                                    "[execute_llm_stream] 置信度 {:.2} 低于阈值 {:.2}",
                                    confidence,
                                    threshold
                                );
                                if let Some(ref conf_cfg) = this.config.confidence_config
                                    && matches!(conf_cfg.on_low_confidence, ConfidenceAction::Block)
                                {
                                    block = true;
                                }
                            }
                        }
                    }
                    if block {
                        return Poll::Ready(Some(Err(format!(
                            "低置信度阻断（阈值 {:.2}）",
                            this.config.confidence_threshold.unwrap_or(0.0)
                        ))));
                    }

                    // 缓存写入（异步，转交 Caching 阶段）
                    if let Some(ref cache) = this.config.cache
                        && let Some(ref key) = this.prepared.cache_key
                    {
                        let resp = ChatResponse {
                            content: std::mem::take(&mut this.content),
                            thinking: if this.thinking.is_empty() {
                                None
                            } else {
                                Some(std::mem::take(&mut this.thinking))
                            },
                            usage: this.usage.clone(),
                            ..Default::default()
                        };
                        if let Ok(val) = serde_json::to_value(&resp) {
                            let cache = cache.clone();
                            let key = key.clone();
                            let ttl = this.config.cache_ttl_secs;
                            let fut = async move {
                                cache.set(key, val, ttl).await;
                            };
                            this.phase = StreamPhase::Caching(Some(Box::pin(fut)));
                            return Poll::Pending;
                        }
                    }

                    this.phase = StreamPhase::Done;
                    return Poll::Ready(None);
                },
                StreamPhase::Caching(mut fut_opt) => {
                    if let Some(fut) = fut_opt.as_mut() {
                        if let Poll::Pending = fut.as_mut().poll(cx) {
                            this.phase = StreamPhase::Caching(fut_opt);
                            return Poll::Pending;
                        }
                    }
                    this.phase = StreamPhase::Done;
                    return Poll::Ready(None);
                },
                StreamPhase::Done => return Poll::Ready(None),
            }
        }
    }
}

/// 记录 PromptGuard 阻断审计
fn record_block_audit(config: &LlmCallConfig, err: &str) {
    if let Some(ref recorder) = config.audit_recorder {
        recorder.record(AuditEntry {
            execution_type: "llm_call".into(),
            duration_ms: 0,
            status: "blocked".into(),
            error: Some(err.to_string()),
            ..Default::default()
        });
    }
}

/// 记录调用失败审计
fn record_failure_audit(config: &LlmCallConfig, err: &str, start: Instant) {
    if let Some(ref recorder) = config.audit_recorder {
        recorder.record(AuditEntry {
            execution_type: "llm_call".into(),
            duration_ms: start.elapsed().as_millis() as u64,
            status: "failed".into(),
            error: Some(err.to_string()),
            ..Default::default()
        });
    }
}

/// 记录调用成功审计
fn record_success_audit(
    config: &LlmCallConfig,
    request: &ChatRequest,
    response_text: &str,
    start: Instant,
) {
    if let Some(ref recorder) = config.audit_recorder {
        let input_text = serde_json::to_string(request).unwrap_or_default();
        recorder.record(AuditEntry {
            execution_type: "llm_call".to_string(),
            session_id: config.session_id.clone(),
            node_id: config.node_id.clone(),
            workflow_id: config.workflow_id.clone(),
            tool_name: None,
            input_hash: sha256(&input_text),
            output_hash: sha256(response_text),
            duration_ms: start.elapsed().as_millis() as u64,
            status: "success".to_string(),
            error: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            id: uuid::Uuid::new_v4().to_string(),
        });
    }
}

/// 从 ChatRequest 构建缓存键
fn build_cache_key(request: &ChatRequest) -> LlmCacheKey {
    use std::hash::{Hash, Hasher};
    let messages_json = serde_json::to_string(&request.messages).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    messages_json.hash(&mut hasher);
    let messages_hash = format!("{:x}", hasher.finish());
    LlmCacheKey { model: request.model.clone(), messages_hash, temperature: request.temperature }
}

/// 计算 SHA256 十六进制字符串
fn sha256(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
