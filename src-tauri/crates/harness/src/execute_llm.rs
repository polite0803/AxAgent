//! 中心化 LLM 调用入口 — 所有约束在此生效
//!
//! 所有 `adapter.chat()` 调用应当经过 `execute_llm()`，
//! 以获得统一的 PromptGuard 过滤、上下文窗口管理、缓存命中检查、
//! 后置处理和审计记录。
//!
//! 向后兼容：通过 `LlmCallConfig` 的 Option 字段控制各功能开关，
//! 不设置时走最少开销路径。

use crate::audit_trail::{AuditEntry, AuditRecorder};
use crate::prompt_guard::PromptGuard;
use crate::provider::{ProviderAdapter, ProviderRequestContext};
use crate::types::{ChatContent, ChatRequest, ChatResponse};
use std::sync::Arc;

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
        Self {
            response,
            usage,
            duration_ms,
            cached,
        }
    }
}

/// Token 用量统计
#[derive(Default, Clone, Debug)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

use crate::retry_policy::RetryPolicy;

/// LLM 调用配置 — 所有约束功能通过 Option 控制
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
    pub confidence_config: Option<crate::confidence::ConfidenceConfig>,
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

/// 中心化 LLM 调用入口 — 所有约束在此生效
///
/// # 参数
/// - `adapter`: Provider 适配器
/// - `ctx`: Provider 请求上下文
/// - `request`: 待发送的聊天请求（会被修改：消息内容经 PromptGuard 过滤）
/// - `config`: 调用配置（约束开关）
///
/// # 返回
/// 标准化的 `LlmCallResult`，包含响应、用量、耗时和缓存状态。
pub async fn execute_llm(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    mut request: ChatRequest,
    config: &LlmCallConfig,
) -> Result<LlmCallResult, String> {
    let start = std::time::Instant::now();

    // ── 1. 前置拦截：PromptGuard 过滤 ──
    if let Some(ref guard) = config.prompt_guard {
        for msg in &mut request.messages {
            match &mut msg.content {
                ChatContent::Text(text) => {
                    match guard.process_user_input(text) {
                        Ok(safe) => {
                            *text = safe;
                        },
                        Err(blocked) => {
                            let err = format!("PromptGuard 阻断: {}", blocked);
                            tracing::warn!("[execute_llm] {}", &err);
                            // 记录审计（如果配置）
                            if let Some(ref recorder) = config.audit_recorder {
                                recorder.record(AuditEntry {
                                    execution_type: "llm_call".into(),
                                    duration_ms: 0,
                                    status: "blocked".into(),
                                    error: Some(err.clone()),
                                    ..Default::default()
                                });
                            }
                            return Err(err);
                        },
                    }
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
                                    let err = format!("PromptGuard 阻断: {}", blocked);
                                    tracing::warn!("[execute_llm] {}", &err);
                                    if let Some(ref recorder) = config.audit_recorder {
                                        recorder.record(AuditEntry {
                                            execution_type: "llm_call".into(),
                                            duration_ms: 0,
                                            status: "blocked".into(),
                                            error: Some(err.clone()),
                                            ..Default::default()
                                        });
                                    }
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

    // ── 1.5 输入脱敏（PromptGuard 之后、adapter.chat 之前） ──
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
                    ChatContent::Multipart(parts) => parts
                        .iter()
                        .filter_map(|p| p.text.as_deref())
                        .collect::<Vec<_>>()
                        .join(" "),
                };
                // 简易估算：中文 * 2 + 非中文 / 4 + 10
                let chinese_chars: usize = text.chars().filter(|&c| c as u32 > 0x2E80).count();
                let non_chinese = text.len().saturating_sub(chinese_chars);
                chinese_chars * 2 + non_chinese / 4 + 10
            })
            .sum();

        if estimated_tokens > available_input_tokens {
            tracing::warn!(
                "[execute_llm] 上下文估算 {estimated_tokens} token 超过限制 {available_input_tokens}，执行截断"
            );
            // 从最早的非 system 消息开始截断
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
                        let old_chinese: usize =
                            text.chars().filter(|&c| c as u32 > 0x2E80).count();
                        let old_non = text.len().saturating_sub(old_chinese);
                        let old_est = old_chinese * 2 + old_non / 4 + 10;

                        let summary_len = text.len() / 3;
                        let summary_text = &text[..summary_len.min(500)];
                        let summary = format!("[截断摘要] {summary_text}");

                        let new_chinese: usize =
                            summary.chars().filter(|&c| c as u32 > 0x2E80).count();
                        let new_non = summary.len().saturating_sub(new_chinese);
                        let new_est = new_chinese * 2 + new_non / 4 + 10;

                        request.messages[pos].content = ChatContent::Text(summary);
                        estimated_tokens = estimated_tokens.saturating_sub(old_est) + new_est;
                    } else {
                        request.messages.remove(pos);
                        // 重新估算
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
                                let chinese_chars: usize =
                                    text.chars().filter(|&c| c as u32 > 0x2E80).count();
                                let non_chinese = text.len().saturating_sub(chinese_chars);
                                chinese_chars * 2 + non_chinese / 4 + 10
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

    // ── 3. 缓存命中检查 ──
    let cache_key = if config.cache.is_some() {
        Some(build_cache_key(&request))
    } else {
        None
    };
    if let Some(ref cache) = config.cache {
        if let Some(ref key) = cache_key {
            if let Some(cached) = cache.get(key).await {
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
        }
    }

    // ── 4. 实际调用（带可选的重试策略包装） ──
    let response = if let Some(ref policy) = config.retry_policy {
        let cloned_request = request.clone();
        let policy_result = policy
            .execute_with_retry(|| async {
                adapter
                    .chat(ctx, cloned_request.clone())
                    .await
                    .map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| {
                let err = format!("LLM 调用失败（重试耗尽）: {e}");
                tracing::error!("[execute_llm] {}", &err);
                if let Some(ref recorder) = config.audit_recorder {
                    recorder.record(AuditEntry {
                        execution_type: "llm_call".into(),
                        duration_ms: start.elapsed().as_millis() as u64,
                        status: "failed".into(),
                        error: Some(err.clone()),
                        ..Default::default()
                    });
                }
                err
            })?;
        policy_result
    } else {
        adapter.chat(ctx, request.clone()).await.map_err(|e| {
            let err = format!("LLM 调用失败: {e}");
            tracing::error!("[execute_llm] {}", &err);
            if let Some(ref recorder) = config.audit_recorder {
                recorder.record(AuditEntry {
                    execution_type: "llm_call".into(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    status: "failed".into(),
                    error: Some(err.clone()),
                    ..Default::default()
                });
            }
            err
        })?
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    let result = LlmCallResult::from_raw(response, duration_ms, false);

    // ── 写入缓存（调用成功后） ──
    if let Some(ref cache) = config.cache {
        if let Some(ref key) = cache_key {
            if let Ok(val) = serde_json::to_value(&result.response) {
                cache.set(key.clone(), val, config.cache_ttl_secs).await;
            }
        }
    }

    // ── 5. 后置：置信度检查（如果配置了阈值） ──
    if let Some(threshold) = config.confidence_threshold {
        let response_text = &result.response.content;
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response_text) {
            let confidence = parsed
                .get("confidence")
                .and_then(|c| c.as_f64())
                .unwrap_or(1.0);
            if confidence < threshold {
                tracing::warn!(
                    "[execute_llm] 置信度 {:.2} 低于阈值 {:.2}，触发降级",
                    confidence,
                    threshold
                );
                if let Some(ref conf_cfg) = config.confidence_config {
                    match conf_cfg.on_low_confidence {
                        crate::confidence::ConfidenceAction::Block => {
                            return Err(format!("低置信度: {confidence:.2} < {threshold:.2}"));
                        },
                        crate::confidence::ConfidenceAction::WarnAndContinue => {
                            // 只是警告，继续
                        },
                        crate::confidence::ConfidenceAction::FallbackToDefault => {
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

    // ── 6. 审计记录 — 无条件执行（只要配置了 recorder） ──
    if let Some(ref recorder) = config.audit_recorder {
        let response_text = &result.response.content;
        let input_text = serde_json::to_string(&request).unwrap_or_default();
        recorder.record(AuditEntry {
            execution_type: "llm_call".to_string(),
            session_id: config.session_id.clone(),
            node_id: config.node_id.clone(),
            workflow_id: config.workflow_id.clone(),
            tool_name: None,
            input_hash: sha256(&input_text),
            output_hash: sha256(response_text),
            duration_ms: result.duration_ms,
            status: "success".to_string(),
            error: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            id: uuid::Uuid::new_v4().to_string(),
        });
    }

    tracing::debug!(
        "[execute_llm] 完成: {} tokens in {}ms",
        result.usage.total_tokens,
        result.duration_ms,
    );

    Ok(result)
}

/// 从 ChatRequest 构建缓存键
fn build_cache_key(request: &ChatRequest) -> crate::cache_interceptor::LlmCacheKey {
    use std::hash::{Hash, Hasher};
    let messages_json = serde_json::to_string(&request.messages).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    messages_json.hash(&mut hasher);
    let messages_hash = format!("{:x}", hasher.finish());
    crate::cache_interceptor::LlmCacheKey {
        model: request.model.clone(),
        messages_hash,
        temperature: request.temperature,
    }
}

/// 计算 SHA256 十六进制字符串
fn sha256(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
