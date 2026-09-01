// SPDX-License-Identifier: AGPL-3.0-only

//! Indexing pipeline for knowledge base documents and memory items.
//!
//! Provides functions to:
//! - Parse an `embedding_provider` string ("providerId::model_id")
//! - Build a `ProviderRequestContext` for embedding API calls
//! - Generate embeddings via provider adapters
//! - Index knowledge base documents and memory items via the unified RAG layer
//! - Search knowledge base / memory vectors via the unified RAG layer
//! - Collect RAG context for conversation injection

use sea_orm::DatabaseConnection;

use std::sync::Arc;

use axagent_credential::{CredentialManager, CredentialType};
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::*;
use axagent_harness::{
    ProviderAdapter, ProviderRequestContext, url_utils::resolve_base_url_for_type,
};
use axagent_search::rag::{self, ChunkStrategy, KnowledgeRAG, LlmCallFn, MemoryRAG};
use axagent_search::vector_store::{VectorSearchResult, VectorStore};

// ── AsyncEmbedFn implementation ──────────────────────────────────────────────

/// Concrete implementation of `AsyncEmbedFn` that uses provider adapters.
///
/// 通过 `DEFAULT_PROVIDER_REGISTRY`（OnceLock 缓存）共享 provider registry，
/// 避免每次 embed 都 `RuntimeHarness::new`（之前会丢弃 adapter cache）。
#[derive(Clone)]
pub struct ProviderEmbedFn;

static DEFAULT_PROVIDER_REGISTRY: std::sync::OnceLock<
    std::sync::Arc<dyn axagent_harness::registry::ProviderRegistry>,
> = std::sync::OnceLock::new();

fn default_provider_registry()
-> &'static std::sync::Arc<dyn axagent_harness::registry::ProviderRegistry> {
    DEFAULT_PROVIDER_REGISTRY.get_or_init(|| {
        std::sync::Arc::new(axagent_providers::registry::ProviderRegistry::create_default())
            as std::sync::Arc<dyn axagent_harness::registry::ProviderRegistry>
    })
}

#[async_trait::async_trait]
impl rag::AsyncEmbedFn for ProviderEmbedFn {
    async fn generate(
        &self,
        db: &DatabaseConnection,
        master_key: &[u8; 32],
        embedding_provider: &str,
        texts: Vec<String>,
        dimensions: Option<usize>,
    ) -> Result<EmbedResponse> {
        generate_embeddings(
            db,
            master_key,
            default_provider_registry(),
            embedding_provider,
            texts,
            dimensions,
        )
        .await
    }
}

// ── RAG Pipeline LLM helper ───────────────────────────────────────────────────

/// Build a `LlmCallFn` from the first enabled provider in the DB.
/// Used by the RAG pipeline for query enhancement LLM calls.
pub async fn build_rag_llm_fn(
    _db: &DatabaseConnection,
    master_key: &[u8; 32],
) -> Option<LlmCallFn> {
    let bridge = axagent_runtime::llm_bridge::build_llm_bridge_from_db(master_key).await?;

    Some(Arc::new(move |prompt: String| {
        let bridge = bridge.clone();
        Box::pin(async move { bridge.call_llm("", &prompt).await.map_err(AxAgentError::Provider) })
    }))
}

// ── Low-level embedding utilities ────────────────────────────────────────────

/// Parse an embedding_provider string like "providerId::model_id" into (provider_id, model_id).
pub fn parse_embedding_provider(embedding_provider: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = embedding_provider.splitn(2, "::").collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(AxAgentError::Provider(format!(
            "Invalid embedding_provider format '{}'. Expected 'providerId::model_id'",
            embedding_provider
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// 旧格式 embedding_provider 解析结果缓存。
///
/// `generate_embeddings` 在索引分批 / 每次搜索时都会调用解析逻辑，
/// 不缓存会导致每个批次重复查询 DB（get_provider 含 3 条 SQL）并刷屏 WARN。
/// key 为原始字符串，value 为补全后的完整格式；解析失败不缓存，
/// 保证用户修复配置后可以立即重试成功（配置正确后字符串变为完整格式，走快速路径）。
static RESOLVED_EMBEDDING_PROVIDER_CACHE: std::sync::LazyLock<
    tokio::sync::RwLock<std::collections::HashMap<String, String>>,
> = std::sync::LazyLock::new(|| tokio::sync::RwLock::new(std::collections::HashMap::new()));

/// Resolve an embedding_provider string that may be in legacy format (only `provider_id`)
/// into a canonical `"providerId::model_id"` string.
///
/// 旧格式兼容：早期版本在 knowledge base / memory namespace 中只存了 provider_id，
/// 未存 model_id。此处自动补全该 provider 下第一个启用的 Embedding 类型模型，
/// 拼成完整格式，避免嵌入链路因格式不完整而失败。
///
/// 兜底策略（与 capability_embedding 的跨 provider 扫描一致）：
/// - 目标 provider 下无启用的 Embedding 模型时，跨 provider 扫描第一个启用的
///   Embedding 模型作为兜底，保证嵌入链路仍可用（语义质量由配置决定）。
///
/// 返回 `(resolved_string, was_legacy)`，`was_legacy=false` 表示输入已是完整格式。
pub async fn resolve_embedding_provider(
    db: &DatabaseConnection,
    embedding_provider: &str,
) -> Result<(String, bool)> {
    if embedding_provider.contains("::") {
        return Ok((embedding_provider.to_string(), false));
    }

    // 命中缓存：跳过 DB 查询与重复 WARN
    if let Some(resolved) =
        RESOLVED_EMBEDDING_PROVIDER_CACHE.read().await.get(embedding_provider).cloned()
    {
        return Ok((resolved, true));
    }

    let provider_id = embedding_provider.trim();
    if provider_id.is_empty() {
        tracing::warn!("[indexing] embedding_provider 为空，尝试跨 provider 扫描可用的嵌入模型");
    } else {
        tracing::warn!(
            "[indexing] 检测到旧格式 embedding_provider（仅 provider_id），尝试自动补全 model_id。请尽快在设置页面重新选择嵌入模型以彻底修复 embedding_provider={}",
            provider_id
        );
    }

    // 优先在目标 provider 下查找启用的 Embedding 模型（空 provider_id 跳过直接兜底）
    let model_id = if provider_id.is_empty() {
        None
    } else {
        axagent_dao::repo::provider::get_provider(db, provider_id)
            .await
            .ok()
            .filter(|p| p.enabled)
            .and_then(|p| {
                p.models
                    .into_iter()
                    .find(|m| m.model_type == ModelType::Embedding && m.enabled)
                    .map(|m| m.model_id)
            })
    };

    if let Some(model_id) = model_id {
        let resolved = format!("{}::{}", provider_id, model_id);
        tracing::warn!("[indexing] 已自动补全嵌入模型为 {}", resolved);
        RESOLVED_EMBEDDING_PROVIDER_CACHE
            .write()
            .await
            .insert(embedding_provider.to_string(), resolved.clone());
        return Ok((resolved, true));
    }

    // 目标 provider 无可用 Embedding 模型：跨 provider 兜底扫描
    if !provider_id.is_empty() {
        tracing::warn!(
            "[indexing] provider '{}' 下未找到 Embedding 类型模型，尝试跨 provider 兜底扫描 provider_id={}",
            provider_id,
            provider_id
        );
    }
    let providers = axagent_dao::repo::provider::list_providers_merged(db).await?;
    for fallback in &providers {
        if !fallback.enabled {
            continue;
        }
        let Some(model) =
            fallback.models.iter().find(|m| m.model_type == ModelType::Embedding && m.enabled)
        else {
            continue;
        };
        let resolved = format!("{}::{}", fallback.id, model.model_id);
        tracing::warn!(
            "[indexing] 使用跨 provider 兜底嵌入模型（{provider_id} => {resolved}），请尽快在设置页为该容器配置正确的嵌入模型 container_provider={provider_id} fallback_provider={} fallback_model={}",
            fallback.id,
            model.model_id
        );
        RESOLVED_EMBEDDING_PROVIDER_CACHE
            .write()
            .await
            .insert(embedding_provider.to_string(), resolved.clone());
        return Ok((resolved, true));
    }

    Err(AxAgentError::Provider(format!(
        "provider '{}' 下未找到可用的 Embedding 类型模型，且系统中无其他可用的 Embedding 模型，请在设置页面配置嵌入模型",
        provider_id
    )))
}

/// Build a ProviderRequestContext for an embedding provider.
pub async fn build_embed_context(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    provider_id: &str,
) -> Result<(ProviderRequestContext, ProviderConfig)> {
    let provider = axagent_dao::repo::provider::get_provider(db, provider_id).await?;

    // 本地推理供应商（llama.cpp / ollama）无需 API key：无 key 行时降级为空 key，
    // 避免 embedding 链路被 `get_active_key` 的 NotFound 卡死。
    let local_no_key = matches!(
        provider.provider_type,
        axagent_harness::types::ProviderType::LlamaCpp
            | axagent_harness::types::ProviderType::Ollama
    );
    let (decrypted_key, key_id) = if local_no_key {
        match axagent_dao::repo::provider::get_active_key(db, provider_id).await {
            Ok(key_row) => {
                let k = axagent_crypto::decrypt_key(&key_row.key_encrypted, master_key)
                    .unwrap_or_default();
                (k, key_row.id.clone())
            },
            Err(_) => (String::new(), String::new()),
        }
    } else {
        let key_row = axagent_dao::repo::provider::get_active_key(db, provider_id).await?;
        (axagent_crypto::decrypt_key(&key_row.key_encrypted, master_key)?, key_row.id.clone())
    };

    let global_settings = axagent_dao::repo::settings::get_settings(db).await.unwrap_or_default();
    let resolved_proxy = axagent_harness::types::provider_model::resolve_provider_proxy(
        &provider.proxy_config,
        &global_settings,
    );

    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id,
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: None,
        proxy_config: resolved_proxy,
        custom_headers: provider.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    Ok((ctx, provider))
}

/// Maximum single-item token threshold: items exceeding this estimate are pre-split
/// into sub-pieces before embedding.
const EMBED_MAX_TOKENS_PER_ITEM: usize = 384;

/// Per-batch token budget: accumulated estimated tokens across items in a batch
/// must not exceed this value (conservatively under a 512-token server limit).
const EMBED_BATCH_TOKEN_BUDGET: usize = 450;

/// Maximum number of retry attempts for a single embedding API call.
const EMBED_MAX_RETRIES: u32 = 3;

/// Base delay in milliseconds for exponential backoff on retry.
const EMBED_RETRY_BASE_DELAY_MS: u64 = 500;

/// Roughly estimate the number of tokens in a text string.
///
/// CJK characters count as 1 token each; other characters average ~0.33 token/char
/// (i.e., ~1 token per 3 non-CJK characters). The estimate is deliberately
/// conservative (over-estimate slightly) to stay safely under server limits.
fn estimate_tokens(text: &str) -> usize {
    let mut cjk = 0usize;
    let mut other = 0usize;
    for ch in text.chars() {
        if is_cjk(ch) {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    cjk + other.div_ceil(3)
}

/// Returns true for characters in CJK / fullwidth ranges where 1 char ≈ 1 token.
fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x3000..=0x30FF      // CJK symbols/punctuation, Japanese kana
        | 0x3400..=0x4DBF    // Extension A
        | 0x4E00..=0x9FFF    // CJK Unified Ideographs
        | 0xAC00..=0xD7AF    // Hangul Syllables
        | 0xF900..=0xFAFF    // CJK Compatibility Ideographs
        | 0xFF00..=0xFFEF    // Fullwidth forms
    )
}

/// Split a text into substrings each estimated to be at most `limit` tokens.
///
/// Uses linear scaling by character ratio and splits on char boundaries.
/// Falls back recursively if a piece still exceeds the limit (handles estimation
/// errors), with a depth cap to prevent infinite loops.
fn split_by_tokens(text: &str, limit: usize) -> Vec<String> {
    let est = estimate_tokens(text);
    if est <= limit {
        return vec![text.to_string()];
    }
    let total_chars = text.chars().count();
    let target_chars = ((total_chars as f64) * (limit as f64) / (est as f64)).max(1.0) as usize;

    let bytes = text.len();
    let mut pieces: Vec<String> = Vec::new();
    let mut start = 0usize;
    while start < bytes {
        let mut end = (start + target_chars).min(bytes);
        while end < bytes && !text.is_char_boundary(end) {
            end += 1;
        }
        let piece = &text[start..end];
        if !piece.trim().is_empty() {
            pieces.push(piece.to_string());
        }
        if end >= bytes {
            break;
        }
        start = end;
    }

    // Recursive re-split: handle estimation errors where pieces still exceed limit
    let mut result: Vec<String> = Vec::new();
    for (depth, p) in pieces.into_iter().enumerate() {
        if estimate_tokens(&p) > limit && p.chars().count() > 1 && depth < 8 {
            result.extend(split_by_tokens(&p, limit));
        } else {
            result.push(p);
        }
    }
    if result.is_empty() {
        vec![text.to_string()]
    } else {
        result
    }
}

/// Average multiple vectors of the same dimension into one.
/// Used to merge sub-piece embeddings back into a single vector for the original text.
fn average_vectors(vecs: &[Vec<f32>]) -> Vec<f32> {
    if vecs.is_empty() {
        return Vec::new();
    }
    let dim = vecs[0].len();
    let mut acc = vec![0f32; dim];
    for v in vecs {
        for (i, x) in v.iter().enumerate() {
            if i < dim {
                acc[i] += x;
            }
        }
    }
    let n = vecs.len() as f32;
    for x in acc.iter_mut() {
        *x /= n;
    }
    acc
}

/// Detect whether an embedding error is caused by "input too large" (server-side
/// physical batch size / token limit). These errors cannot be resolved by retry
/// and must be handled by splitting the input.
fn is_too_large_error(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("too large")
        || m.contains("input is too long")
        || (m.contains("token") && m.contains("exceed"))
}

/// Detect whether an embedding error is **permanent** — retry will never succeed.
///
/// 分类依据：
/// - HTTP 4xx（除 429 限流外）都是客户端侧错误：模型不存在、鉴权失败、
///   模型已 EOL、参数非法等，服务端状态不会在短时间内改变。
/// - 本地连接失败（`connection refused` / `no such host` / `error sending request`）
///   不属于永久错误 — 本地推理服务可能尚未启动完毕，重试有意义。
/// - HTTP 5xx 服务器故障同样非永久，应重试。
///
/// 返回 `true` 时调用方应**立即上抛错误、不进入退避循环**。
fn is_permanent_http_error(msg: &str) -> bool {
    let m = msg.to_lowercase();

    // 先定位是否为 HTTP API 错误，提取状态码
    // 格式来源: providers/src/openai.rs → "OpenAI embed API error {status}: {body}"
    if let Some(status_str) =
        m.split("openai embed api error ").nth(1).and_then(|rest| rest.split(':').next())
    {
        if let Ok(code) = status_str.trim().parse::<u16>() {
            return (400..500).contains(&code) && code != 429;
        }
    }

    // 兜底：错误消息中携带明确的永久语义
    m.contains("end of life")
        || m.contains("has reached its end")
        || m.contains("model not found")
        || m.contains("no such model")
        || m.contains("invalid api key")
        || m.contains("invalid api-key")
        || m.contains("unauthorized")
}

/// Split text into two roughly equal halves at a character boundary.
/// Returns the original text as a single-element vec if it cannot be divided further.
fn bisect_text(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= 1 {
        return vec![text.to_string()];
    }
    let mid = chars.len() / 2;
    let left: String = chars[..mid].iter().collect();
    let right: String = chars[mid..].iter().collect();
    vec![left, right]
}

/// Embed a single text item, guaranteeing exactly one output vector.
///
/// If the server rejects it as too large, recursively bisect the text, embed each
/// half, and average the resulting vectors. This is the leaf-level fallback for
/// all over-limit cases.
async fn embed_one_safe(
    text: &str,
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: &str,
    dimensions: Option<usize>,
) -> Result<Vec<f32>> {
    let request =
        EmbedRequest { model: model.to_string(), input: vec![text.to_string()], dimensions };
    match embed_with_retry(adapter, ctx, request).await {
        Ok(resp) => Ok(resp.embeddings.into_iter().next().unwrap_or_default()),
        Err(e) => {
            if is_too_large_error(&e.to_string()) {
                let halves = bisect_text(text);
                if halves.len() <= 1 {
                    return Err(e);
                }
                let mut vecs = Vec::with_capacity(halves.len());
                for h in &halves {
                    vecs.push(Box::pin(embed_one_safe(h, adapter, ctx, model, dimensions)).await?);
                }
                Ok(average_vectors(&vecs))
            } else {
                Err(e)
            }
        },
    }
}

/// Embed a batch of text items, guaranteeing output length equals input length.
///
/// If the entire batch is rejected as too large, recursively bisect the batch.
/// Single-item overflow is delegated to `embed_one_safe`.
async fn embed_batch_safe(
    batch: &[String],
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: &str,
    dimensions: Option<usize>,
) -> Result<Vec<Vec<f32>>> {
    if batch.is_empty() {
        return Ok(Vec::new());
    }
    let request = EmbedRequest { model: model.to_string(), input: batch.to_vec(), dimensions };
    match embed_with_retry(adapter, ctx, request).await {
        Ok(resp) => Ok(resp.embeddings),
        Err(e) => {
            if is_too_large_error(&e.to_string()) {
                if batch.len() == 1 {
                    // Single item still over limit: delegate to embed_one_safe for recursive bisection
                    Ok(vec![
                        Box::pin(embed_one_safe(&batch[0], adapter, ctx, model, dimensions))
                            .await?,
                    ])
                } else {
                    let mid = batch.len() / 2;
                    let mut left =
                        Box::pin(embed_batch_safe(&batch[..mid], adapter, ctx, model, dimensions))
                            .await?;
                    let right =
                        Box::pin(embed_batch_safe(&batch[mid..], adapter, ctx, model, dimensions))
                            .await?;
                    left.extend(right);
                    Ok(left)
                }
            } else {
                Err(e)
            }
        },
    }
}

/// Embed a flat list of sub-piece texts, batching them by token budget.
///
/// Returns `(vectors, dimensions)` where vectors are 1:1 with the input pieces.
/// Items are accumulated into batches up to `EMBED_BATCH_TOKEN_BUDGET` to
/// minimize API calls while staying under the per-call token limit.
/// Over-limit batches are handled recursively by `embed_batch_safe`.
async fn embed_flat_pieces(
    pieces: &[String],
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: &str,
    dimensions: Option<usize>,
) -> Result<(Vec<Vec<f32>>, usize)> {
    if pieces.is_empty() {
        return Ok((Vec::new(), 0));
    }

    // Budget-based batching: accumulate estimated tokens, flush at budget boundary
    let mut batches: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut sum = 0usize;
    for (i, p) in pieces.iter().enumerate() {
        let est = estimate_tokens(p);
        if !cur.is_empty() && sum + est > EMBED_BATCH_TOKEN_BUDGET {
            batches.push(std::mem::take(&mut cur));
            sum = 0;
        }
        cur.push(i);
        sum += est;
    }
    if !cur.is_empty() {
        batches.push(cur);
    }

    let mut out: Vec<Vec<f32>> = Vec::with_capacity(pieces.len());
    let mut dims = 0usize;
    for idxs in &batches {
        let batch: Vec<String> = idxs.iter().map(|&i| pieces[i].clone()).collect();
        let vecs = embed_batch_safe(&batch, adapter, ctx, model, dimensions).await?;
        if dims == 0 && !vecs.is_empty() {
            dims = vecs[0].len();
        }
        out.extend(vecs);
    }
    Ok((out, dims))
}

/// Generate embeddings for a list of texts using the specified provider.
///
/// Texts are embedded with a per-call token budget and automatic over-size bisection.
/// Each batch is retried up to `EMBED_MAX_RETRIES` times with exponential backoff.
///
/// 单条 token 上限保护：若某条文本估算 token 数超过 `EMBED_MAX_TOKENS_PER_ITEM`，
/// 会在内部先子分片、分别嵌入，再对子向量取均值，从而兼容单条上限较低的
/// embedding 服务端（避免 `500 input too large` 被无限重试）。
///
/// `provider_registry` 由调用方传入（通常来自 `state.harness.provider_registry()`），
/// 不再内部 `RuntimeHarness::new` 重复创建（之前会丢弃 adapter cache）。
pub async fn generate_embeddings(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    provider_registry: &std::sync::Arc<dyn axagent_harness::registry::ProviderRegistry>,
    embedding_provider: &str,
    texts: Vec<String>,
    dimensions: Option<usize>,
) -> Result<EmbedResponse> {
    // 兼容旧格式：仅存 provider_id 时自动补全第一个启用的 Embedding 模型
    let (resolved_provider, _was_legacy) =
        resolve_embedding_provider(db, embedding_provider).await?;
    let (provider_id, model_id) = parse_embedding_provider(&resolved_provider)?;
    let (ctx, provider_config) = build_embed_context(db, master_key, &provider_id).await?;

    let registry_key = axagent_harness::types::provider_model::provider_registry_key(
        &provider_config.provider_type,
    );
    let adapter = provider_registry.get(registry_key).ok_or_else(|| {
        AxAgentError::Provider(format!("Unsupported provider type: {}", registry_key))
    })?;

    // ── Single-item token limit protection ─────────────────────────────────
    // Expand texts that exceed EMBED_MAX_TOKENS_PER_ITEM into sub-pieces,
    // tracking which original text each group of sub-pieces belongs to.
    // After embedding, sub-piece vectors for the same original text are
    // averaged back to a single vector, preserving the output contract
    // of N texts → N vectors.
    let mut groups: Vec<Vec<String>> = Vec::with_capacity(texts.len());
    let mut flat: Vec<String> = Vec::new();
    for t in &texts {
        if estimate_tokens(t) <= EMBED_MAX_TOKENS_PER_ITEM {
            groups.push(vec![t.clone()]);
            flat.push(t.clone());
        } else {
            let subs = split_by_tokens(t, EMBED_MAX_TOKENS_PER_ITEM);
            groups.push(subs.clone());
            flat.extend(subs);
        }
    }

    // ── Batch embed (token-budget batching + automatic bisection) ─────────
    // embed_flat_pieces handles both "batch too large" and "single item too large"
    // cases, guaranteeing that arbitrary input lengths are successfully embedded
    // with output vectors 1:1 to `flat`.
    let (all_flat, dims) = embed_flat_pieces(&flat, &*adapter, &ctx, &model_id, dimensions).await?;
    let first_dimensions = if dims > 0 { Some(dims) } else { None };

    // ── Reconstruct per-original-text vectors via averaging ───────────────
    let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
    let mut cursor = 0usize;
    for group in &groups {
        if group.len() == 1 {
            embeddings.push(all_flat[cursor].clone());
            cursor += 1;
        } else {
            let avg = average_vectors(&all_flat[cursor..cursor + group.len()]);
            embeddings.push(avg);
            cursor += group.len();
        }
    }

    Ok(EmbedResponse { embeddings, dimensions: first_dimensions.unwrap_or(0) })
}

/// Execute a single embedding request with retry and exponential backoff.
///
/// 两类错误**立即上抛、不进入退避循环**：
/// 1. 输入过大（`is_too_large_error`）— 交由上层二分兜底处理。
/// 2. 永久性 HTTP 错误（`is_permanent_http_error`）— 4xx 非 429 类、模型 EOL、
///    鉴权失败等，重试永远不会成功，立即失败让调用方（如 capability_provider 的
///    跨 provider 探测）切下一个候选。
async fn embed_with_retry(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    request: EmbedRequest,
) -> Result<EmbedResponse> {
    let mut last_err_msg = String::new();

    for attempt in 0..EMBED_MAX_RETRIES {
        match adapter.embed(ctx, request.clone()).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                last_err_msg = e.to_string();

                // 不可重试错误：立即上抛
                if is_too_large_error(&last_err_msg) {
                    return Err(AxAgentError::Provider(last_err_msg));
                }
                if is_permanent_http_error(&last_err_msg) {
                    tracing::warn!("Embedding 遇到永久错误（不重试）: {}", last_err_msg);
                    return Err(AxAgentError::Provider(last_err_msg));
                }

                if attempt + 1 < EMBED_MAX_RETRIES {
                    let delay = EMBED_RETRY_BASE_DELAY_MS
                        .saturating_mul(2u64.checked_pow(attempt).unwrap_or(u64::MAX / 2))
                        .min(60_000);
                    tracing::warn!(
                        "Embedding attempt {}/{} failed, retrying in {}ms: {}",
                        attempt + 1,
                        EMBED_MAX_RETRIES,
                        delay,
                        e
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                } else {
                    tracing::error!("Embedding failed after {} attempts: {}", EMBED_MAX_RETRIES, e);
                }
            },
        }
    }

    Err(AxAgentError::Provider(format!(
        "Embedding failed after {} retries: {}",
        EMBED_MAX_RETRIES, last_err_msg
    )))
}

// ── Document / item indexing (delegates to rag::index) ───────────────────────

/// Index a single knowledge base document: parse → chunk → embed → store.
///
/// Updates document status to "indexing" then "ready" or "failed".
#[allow(clippy::too_many_arguments)]
pub async fn index_knowledge_document(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    knowledge_base_id: &str,
    document_id: &str,
    source_path: &str,
    mime_type: &str,
    embedding_provider: &str,
    chunk_size: Option<i32>,
    chunk_overlap: Option<i32>,
    separator: Option<String>,
) -> Result<()> {
    axagent_dao::repo::knowledge::update_document_status(db, document_id, "indexing").await?;

    // H6: resolve embedding dimensions from knowledge base config
    let dimensions = axagent_dao::repo::knowledge::get_knowledge_base(db, knowledge_base_id)
        .await
        .ok()
        .and_then(|kb| kb.embedding_dimensions)
        .map(|d| d as usize);

    let result = run_indexing(
        db,
        master_key,
        vector_store,
        knowledge_base_id,
        document_id,
        source_path,
        mime_type,
        embedding_provider,
        chunk_size,
        chunk_overlap,
        separator,
        dimensions,
    )
    .await;

    match result {
        Ok(()) => {
            axagent_dao::repo::knowledge::update_document_status(db, document_id, "ready").await?;
            Ok(())
        },
        Err(e) => {
            // H5: set status to failed so the user can retry
            let _ = axagent_dao::repo::knowledge::update_document_status(db, document_id, "failed")
                .await;
            Err(e)
        },
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_indexing(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    knowledge_base_id: &str,
    document_id: &str,
    source_path: &str,
    mime_type: &str,
    embedding_provider: &str,
    chunk_size: Option<i32>,
    chunk_overlap: Option<i32>,
    separator: Option<String>,
    dimensions: Option<usize>,
) -> Result<()> {
    let is_conversation = source_path.starts_with("conversation://");

    let strategy = if is_conversation {
        let conv_id = source_path.strip_prefix("conversation://").unwrap_or("");
        let text =
            axagent_dao::repo::conversation::get_conversation_archive_text(db, conv_id).await?;

        ChunkStrategy::FromText {
            text,
            chunk_size: chunk_size
                .map(|v| v as usize)
                .unwrap_or(axagent_search::text_chunker::DEFAULT_CHUNK_SIZE),
            overlap: chunk_overlap
                .map(|v| v as usize)
                .unwrap_or(axagent_search::text_chunker::DEFAULT_OVERLAP),
            separator,
        }
    } else {
        ChunkStrategy::ParseAndChunk {
            source_path: source_path.to_string(),
            mime_type: mime_type.to_string(),
            chunk_size: chunk_size
                .map(|v| v as usize)
                .unwrap_or(axagent_search::text_chunker::DEFAULT_CHUNK_SIZE),
            overlap: chunk_overlap
                .map(|v| v as usize)
                .unwrap_or(axagent_search::text_chunker::DEFAULT_OVERLAP),
            separator,
        }
    };

    let chunks = rag::prepare_chunks(document_id, &strategy)?;

    if chunks.is_empty() {
        return Ok(());
    }

    let chunk_texts: Vec<String> = chunks.iter().map(|(_, text, _)| text.clone()).collect();
    let embed_response = generate_embeddings(
        db,
        master_key,
        default_provider_registry(),
        embedding_provider,
        chunk_texts,
        dimensions,
    )
    .await?;

    rag::index(
        vector_store,
        "kb",
        knowledge_base_id,
        document_id,
        "",
        embed_response.embeddings,
        chunks,
    )
    .await
}

/// Index a single memory item: embed content → store in vector DB.
#[allow(clippy::too_many_arguments)]
pub async fn index_memory_item(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    namespace_id: &str,
    item_id: &str,
    content: &str,
    embedding_provider: &str,
    dimensions: Option<usize>,
) -> Result<()> {
    let chunks = rag::prepare_direct_chunk(item_id, content);

    if chunks.is_empty() {
        return Ok(());
    }

    let chunk_texts: Vec<String> = chunks.iter().map(|(_, text, _)| text.clone()).collect();
    let embed_response = generate_embeddings(
        db,
        master_key,
        default_provider_registry(),
        embedding_provider,
        chunk_texts,
        dimensions,
    )
    .await?;

    rag::index(
        vector_store,
        "mem",
        namespace_id,
        item_id,
        content,
        embed_response.embeddings,
        chunks,
    )
    .await
}

/// Index a single wiki note: chunk text → embed → store in vector DB.
///
/// Uses the `FromText` chunk strategy since note content is already plain text.
/// The collection name follows the pattern `wiki_{wiki_id}`.
#[allow(clippy::too_many_arguments)]
pub async fn index_wiki_note(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    wiki_id: &str,
    note_id: &str,
    content: &str,
    embedding_provider: &str,
    dimensions: Option<usize>,
) -> Result<()> {
    let strategy = ChunkStrategy::FromText {
        text: content.to_string(),
        chunk_size: axagent_search::text_chunker::DEFAULT_CHUNK_SIZE,
        overlap: axagent_search::text_chunker::DEFAULT_OVERLAP,
        separator: None,
    };

    let chunks = rag::prepare_chunks(note_id, &strategy)?;

    if chunks.is_empty() {
        return Ok(());
    }

    let chunk_texts: Vec<String> = chunks.iter().map(|(_, text, _)| text.clone()).collect();
    let embed_response = generate_embeddings(
        db,
        master_key,
        default_provider_registry(),
        embedding_provider,
        chunk_texts,
        dimensions,
    )
    .await?;

    // 先删除该笔记的全部旧向量，再写入新 chunk。
    // 分块按 {note_id}_{chunkIndex} upsert，编辑后块数变少时旧高序号 chunk 会永久残留，
    // 导致检索命中已删除内容 —— 必须在写入前整体清理（R1 修复）。
    // 顺序放在 embedding 生成成功之后：失败时保留旧向量（任务会重试），避免笔记瞬间失去全部检索能力。
    let collection = rag::collection_id("wiki", wiki_id);
    let _ = vector_store.delete_document_embeddings(&collection, note_id).await;

    rag::index(vector_store, "wiki", wiki_id, note_id, content, embed_response.embeddings, chunks)
        .await
}

/// 后台批量索引任务参数（避免 8 参数函数触发 clippy::too_many_arguments）。
pub struct WikiBatchIndexingTask {
    pub app: tauri::AppHandle,
    pub db: DatabaseConnection,
    pub master_key: [u8; 32],
    pub vector_store: std::sync::Arc<VectorStore>,
    pub wiki_id: String,
    pub note_ids: Vec<String>,
    pub log_label: &'static str,
    /// 传入时在全部索引完成后额外发一次完成事件（文件夹导入用），payload 含 wikiId/importedCount
    pub completion_event: Option<&'static str>,
}

/// 后台批量索引 wiki 笔记：删旧向量 → index_source → 逐条 emit "wiki-note-indexed"。
///
/// 供 `llm_wiki_ingest` / `llm_wiki_import_folder` / `write_base64_to_file` /
/// `deep_research_topic` 复用，保证所有 LLM 生成页与导入页都进入 RAG 索引（R2 修复）。
/// wiki 未配置 embedding_provider 或查询失败时记录日志并跳过（与旧调用点行为一致）。
pub fn spawn_wiki_note_batch_indexing(
    WikiBatchIndexingTask {
        app,
        db,
        master_key,
        vector_store,
        wiki_id,
        note_ids,
        log_label,
        completion_event,
    }: WikiBatchIndexingTask,
) {
    use tauri::Emitter;

    if note_ids.is_empty() {
        return;
    }

    tokio::spawn(crate::commands::spawn_guard::catch_unwind_logged(log_label, async move {
        let wiki = match axagent_dao::repo::wiki::get_wiki(&db, &wiki_id).await {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("[{log_label}] 获取 wiki {wiki_id} 失败，跳过索引: {e}");
                return;
            },
        };
        if wiki.embedding_provider.is_none() {
            tracing::info!("[{log_label}] wiki {wiki_id} 未配置 embedding_provider，跳过索引");
            return;
        }
        let container = rag::KnowledgeContainer::from_wiki(&wiki);
        for note_id in &note_ids {
            let note = match axagent_dao::repo::note::get_note(&db, note_id).await {
                Ok(n) => n,
                Err(e) => {
                    tracing::warn!("[{log_label}] 获取笔记 {note_id} 失败，跳过索引: {e}");
                    continue;
                },
            };
            let collection = rag::collection_id("wiki", &wiki_id);
            let _ = vector_store.delete_document_embeddings(&collection, note_id).await;

            let index_result = index_source(
                &db,
                &master_key,
                &vector_store,
                &container,
                note_id,
                &note.content,
                None,
                None,
            )
            .await;

            let (success, error_msg) = match &index_result {
                Ok(_) => (true, None),
                Err(e) => {
                    tracing::error!("[{log_label}] wiki 笔记 {note_id} 索引失败: {e}");
                    (false, Some(e.to_string()))
                },
            };
            let _ = app.emit(
                "wiki-note-indexed",
                serde_json::json!({
                    "noteId": note_id,
                    "success": success,
                    "error": error_msg,
                }),
            );
        }

        if let Some(event) = completion_event {
            let _ = app.emit(
                event,
                serde_json::json!({
                    "wikiId": wiki_id,
                    "importedCount": note_ids.len(),
                }),
            );
        }
    }));
}

#[allow(clippy::too_many_arguments)]
pub async fn index_source(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    container: &axagent_search::rag::KnowledgeContainer,
    item_id: &str,
    content: &str,
    source_path: Option<&str>,
    mime_type: Option<&str>,
) -> Result<()> {
    let config = container.source_config();
    let embedding_provider = match &config.embedding_provider {
        Some(p) => p.clone(),
        None => {
            return Err(axagent_harness::core_error::AxAgentError::Provider(format!(
                "{} '{}' has no embedding provider configured",
                container.container_type_str(),
                container.id
            )));
        },
    };
    let dimensions = config.embedding_dimensions.map(|d| d as usize);

    match container.container_type {
        axagent_search::rag::ContainerType::KnowledgeBase => {
            let chunk_size = container.chunk_size;
            let chunk_overlap = container.chunk_overlap;
            let separator = None;

            if let (Some(sp), Some(mt)) = (source_path, mime_type) {
                index_knowledge_document(
                    db,
                    master_key,
                    vector_store,
                    &container.id,
                    item_id,
                    sp,
                    mt,
                    &embedding_provider,
                    chunk_size,
                    chunk_overlap,
                    separator,
                )
                .await
            } else {
                Err(axagent_harness::core_error::AxAgentError::Provider(
                    "KnowledgeBase indexing requires source_path and mime_type".to_string(),
                ))
            }
        },
        axagent_search::rag::ContainerType::Memory => {
            index_memory_item(
                db,
                master_key,
                vector_store,
                &container.id,
                item_id,
                content,
                &embedding_provider,
                dimensions,
            )
            .await
        },
        axagent_search::rag::ContainerType::WikiVault => {
            index_wiki_note(
                db,
                master_key,
                vector_store,
                &container.id,
                item_id,
                content,
                &embedding_provider,
                dimensions,
            )
            .await
        },
    }
}

// ── Search (delegates to rag::search) ────────────────────────────────────────

/// Search knowledge base vectors for relevant content.
pub async fn search_knowledge(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    knowledge_base_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<VectorSearchResult>> {
    rag::search(
        &KnowledgeRAG,
        db,
        master_key,
        vector_store,
        knowledge_base_id,
        query,
        top_k,
        None,
        ProviderEmbedFn,
    )
    .await
}

/// Search knowledge base with optional document ID filter (multi-document collaboration).
/// Paper QA Pipeline 用此函数把检索范围限制在单篇论文内。
pub async fn search_knowledge_with_doc_filter(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    knowledge_base_id: &str,
    query: &str,
    top_k: usize,
    doc_ids: Option<&[String]>,
) -> Result<Vec<VectorSearchResult>> {
    rag::search_with_filter(
        &KnowledgeRAG,
        db,
        master_key,
        vector_store,
        knowledge_base_id,
        query,
        top_k,
        None,
        ProviderEmbedFn,
        doc_ids,
    )
    .await
}

/// Search memory namespace vectors for relevant content.
pub async fn search_memory(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    namespace_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<VectorSearchResult>> {
    // Look up namespace settings for dimensions
    let dims = axagent_dao::repo::memory::get_namespace(db, namespace_id)
        .await
        .ok()
        .and_then(|ns| ns.embedding_dimensions.map(|v| v as usize));
    rag::search(
        &MemoryRAG,
        db,
        master_key,
        vector_store,
        namespace_id,
        query,
        top_k,
        dims,
        ProviderEmbedFn,
    )
    .await
}

// ── Context collection (delegates to rag::collect_rag_context) ───────────────

/// Collect RAG context from all enabled sources for a conversation query.
///
/// Returns a `RagContextResult` with formatted context parts and structured results.
/// RAG query cache: avoids repeated vector searches for the same query
static RAG_CACHE: std::sync::LazyLock<
    tokio::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, RagContextResult)>>,
> = std::sync::LazyLock::new(|| tokio::sync::Mutex::new(std::collections::HashMap::new()));

const RAG_CACHE_TTL_SECS: u64 = 30;

/// 根据 `RerankConfig.backend` 和 `api_key_ref`，从 `CredentialManager` 解析出云端 reranker 的实际 API Key。
///
/// - 本地 backend（`rule` / `cross_encoder` / `pipeline`）直接返回 `None`，无需凭证。
/// - 云端 backend（`cohere` / `jina` / `voyage`）要求 `api_key_ref` 指向已存储的凭证；
///   凭证缺失或类型不匹配时返回 `None`，由下游 `create_rerank_pipeline` 自动降级到 `RuleReranker`。
async fn resolve_rerank_api_key(
    credential_manager: &CredentialManager,
    rerank_config: &axagent_harness::rag_config::RerankConfig,
) -> Option<String> {
    let backend = rerank_config.backend.as_str();
    if !matches!(backend, "cohere" | "jina" | "voyage") {
        return None;
    }
    let key_ref = rerank_config.api_key_ref.as_ref()?;
    match credential_manager.get_credential(key_ref).await {
        Ok(cred) => match cred.credential_type {
            CredentialType::ApiKey { key, .. } => Some(key),
            CredentialType::BearerToken { token } => Some(token),
            _ => {
                tracing::warn!(
                    "Rerank 凭证 '{}' 类型不支持（{:?}），降级到 RuleReranker",
                    key_ref,
                    cred.credential_type
                );
                None
            },
        },
        Err(e) => {
            tracing::warn!("Rerank 凭证 '{}' 加载失败：{}，降级到 RuleReranker", key_ref, e);
            None
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn collect_rag_context(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    kb_ids: &[String],
    mem_ids: &[String],
    wiki_ids: &[String],
    query: &str,
    top_k: usize,
    credential_manager: &Arc<CredentialManager>,
) -> RagContextResult {
    if kb_ids.is_empty() && mem_ids.is_empty() && wiki_ids.is_empty() {
        return RagContextResult {
            context_parts: vec![],
            source_results: vec![],
            graph_context: None,
        };
    }

    // Read pipeline config from global settings
    let pipeline_config = axagent_dao::repo::settings::get_settings(db)
        .await
        .map(|s| s.rag_pipeline_config)
        .unwrap_or_default();

    let use_pipeline = pipeline_config
        .get("query_enhancement")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || pipeline_config
            .get("rerank")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        || pipeline_config
            .get("self_rag")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

    if !use_pipeline {
        // Fast path: no pipeline features enabled, use existing cached search
        let cache_key = format!("{:?}|{:?}|{:?}|{}", kb_ids, mem_ids, wiki_ids, query);
        {
            let cache = RAG_CACHE.lock().await;
            if let Some((timestamp, result)) = cache.get(&cache_key) {
                if timestamp.elapsed().as_secs() < RAG_CACHE_TTL_SECS {
                    return result.clone();
                }
            }
        }

        let result = rag::collect_rag_context(
            db,
            master_key,
            vector_store,
            kb_ids,
            mem_ids,
            wiki_ids,
            query,
            top_k,
            ProviderEmbedFn,
        )
        .await;

        let mut cache = RAG_CACHE.lock().await;
        cache.insert(cache_key, (std::time::Instant::now(), result.clone()));
        cache.retain(|_, (ts, _)| ts.elapsed().as_secs() < 300);
        return result;
    }

    // Pipeline path: build LLM function if query enhancement is enabled
    let qe_enabled = pipeline_config
        .get("query_enhancement")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let rerank_enabled = pipeline_config
        .get("rerank")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let sr_enabled = pipeline_config
        .get("self_rag")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    tracing::info!(
        "RAG pipeline active: enhancement={}, rerank={}, self_rag={}",
        qe_enabled,
        rerank_enabled,
        sr_enabled,
    );

    let llm_fn = if qe_enabled {
        build_rag_llm_fn(db, master_key).await
    } else {
        None
    };

    // Pipeline results are not cached (involve LLM calls whose outputs vary)
    let pipeline_cfg: axagent_harness::types::RAGPipelineConfig =
        serde_json::from_value(pipeline_config.clone()).unwrap_or_default();

    // 解析云端 reranker 的 API Key（本地 backend 返回 None）
    let rerank_api_key = resolve_rerank_api_key(credential_manager, &pipeline_cfg.rerank).await;

    rag::collect_rag_context_with_pipeline(
        db,
        master_key,
        vector_store,
        kb_ids,
        mem_ids,
        wiki_ids,
        query,
        top_k,
        ProviderEmbedFn,
        &pipeline_cfg,
        llm_fn,
        rerank_api_key,
    )
    .await
}

/// 多文档协同版本的 `collect_rag_context`。
///
/// 接受结构化的 `RAGSourceRef` 列表，每个 source 可独立带 `doc_ids` 过滤。
/// 内部根据 pipeline 配置自动路由到 `collect_rag_context_with_filters`
/// 或 `collect_rag_context_with_pipeline_from_refs`。
///
/// `kb_ids` / `wiki_ids` 仅用于知识图谱回链上下文（不参与过滤），
/// 由调用方从 `sources` 中提取后传入；为空则跳过 KG 上下文。
#[allow(clippy::too_many_arguments)]
pub async fn collect_rag_context_with_sources(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    sources: Vec<axagent_search::rag::RAGSourceRef>,
    query: &str,
    top_k: usize,
    credential_manager: &Arc<CredentialManager>,
) -> RagContextResult {
    use axagent_search::rag::RAGSourceType;

    if sources.is_empty() {
        return RagContextResult {
            context_parts: vec![],
            source_results: vec![],
            graph_context: None,
        };
    }

    // 提取 kb_ids / wiki_ids 用于知识图谱回链
    let kb_ids: Vec<String> = sources
        .iter()
        .filter(|s| s.source_type == RAGSourceType::Knowledge)
        .map(|s| s.container_id.clone())
        .collect();
    let wiki_ids: Vec<String> = sources
        .iter()
        .filter(|s| s.source_type == RAGSourceType::Wiki)
        .map(|s| s.container_id.clone())
        .collect();

    // 读取 pipeline 配置
    let pipeline_config = axagent_dao::repo::settings::get_settings(db)
        .await
        .map(|s| s.rag_pipeline_config)
        .unwrap_or_default();

    let use_pipeline = pipeline_config
        .get("query_enhancement")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || pipeline_config
            .get("rerank")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        || pipeline_config
            .get("self_rag")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

    if !use_pipeline {
        // 快速路径：不缓存（sources 可能带 doc_ids 过滤，缓存键复杂）
        return axagent_search::rag::collect_rag_context_with_filters(
            db,
            master_key,
            vector_store,
            sources,
            query,
            top_k,
            ProviderEmbedFn,
            &kb_ids,
            &wiki_ids,
        )
        .await;
    }

    // Pipeline 路径
    let qe_enabled = pipeline_config
        .get("query_enhancement")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let llm_fn = if qe_enabled {
        build_rag_llm_fn(db, master_key).await
    } else {
        None
    };

    let pipeline_cfg: axagent_harness::types::RAGPipelineConfig =
        serde_json::from_value(pipeline_config.clone()).unwrap_or_default();

    let rerank_api_key = resolve_rerank_api_key(credential_manager, &pipeline_cfg.rerank).await;

    axagent_search::rag::collect_rag_context_with_pipeline_from_refs(
        db,
        master_key,
        vector_store,
        sources,
        query,
        top_k,
        ProviderEmbedFn,
        &pipeline_cfg,
        llm_fn,
        rerank_api_key,
        &kb_ids,
        &wiki_ids,
    )
    .await
}
