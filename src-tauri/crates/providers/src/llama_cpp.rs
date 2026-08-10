// SPDX-License-Identifier: AGPL-3.0-only

//! llama.cpp server 本地推理适配器。
//!
//! 连接本机运行的 [llama.cpp `llama-server`](https://github.com/ggml-org/llama.cpp)。
//! llama-server 提供 OpenAI 兼容的 `/v1/*` 端点（chat / models / embeddings），
//! 但在鉴权和响应格式上存在细微差异，因此本适配器覆写关键方法。
//!
//! 典型用途：本地 embedding 模型（如 BAAI bge-m3 的 GGUF 量化版），
//! 也可作为任意 GGUF 模型的本地推理后端（无需 API key，鉴权头被服务端忽略）。
//!
//! 运行状态查看与启停管理不在此 adapter 中实现，
//! 由 `axagent` 应用层 `commands/local_model.rs` 提供（探测 /health /props /v1/models，
//! 以及子进程托管启动 / 停止）。

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::openai::OpenAIAdapter;
use crate::{ProviderAdapter, ProviderRequestContext};
use async_trait::async_trait;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::*;
use futures::Stream;
use serde::Deserialize;

/// Provider adapter for local llama.cpp `llama-server`.
///
/// llama-server 的 `/v1/*` 端点与 OpenAI 协议兼容，但有以下差异：
/// 1. **鉴权**: 不需要 API key，使用 `/health` 端点探测可达性
/// 2. **模型列表**: `/v1/models` 返回的 meta 字段用 `snake_case`，模型元数据在 `meta` 子对象
/// 3. **流式**: 不支持 `stream_options.include_usage`，委托给 OpenAI adapter 时该字段会被忽略
pub struct LlamaCppAdapter {
    inner: OpenAIAdapter,
}

impl Default for LlamaCppAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LlamaCppAdapter {
    pub fn new() -> Self {
        Self { inner: OpenAIAdapter::new() }
    }

    /// 构建 llama-server 的根 URL（去掉可能的 `/v1` 后缀，因为健康检查在根路径）。
    fn root_url(ctx: &ProviderRequestContext) -> String {
        let base = ctx.base_url.clone().unwrap_or_else(|| "http://127.0.0.1:8091/v1".to_string());
        base.trim_end_matches('/').trim_end_matches("/v1").to_string()
    }

    /// 构建带 `/v1` 的 API 基础 URL。
    fn api_url(ctx: &ProviderRequestContext) -> String {
        let base = ctx.base_url.clone().unwrap_or_else(|| "http://127.0.0.1:8091/v1".to_string());
        let trimmed = base.trim_end_matches('/');
        if trimmed.ends_with("/v1") {
            trimmed.to_string()
        } else {
            format!("{trimmed}/v1")
        }
    }

    /// 获取 HTTP 客户端（支持代理配置）。
    fn get_client(&self, ctx: &ProviderRequestContext) -> Result<reqwest::Client> {
        match &ctx.proxy_config {
            Some(c) if c.proxy_type.as_deref() != Some("none") => crate::build_http_client(Some(c)),
            _ => crate::build_default_http_client(),
        }
    }

    /// llama.cpp 特定的模型类型检测（基于模型 ID 关键字匹配）。
    fn detect_llama_model_type(model_id: &str) -> ModelType {
        let lower = model_id.to_lowercase();
        if lower.contains("embed")
            || lower.contains("bge")
            || lower.contains("e5")
            || lower.contains("gte")
            || lower.contains("nomic-embed")
            || lower.contains("all-minilm")
            || lower.contains("m3e")
            || lower.contains("jina-embedding")
        {
            ModelType::Embedding
        } else if lower.contains("whisper")
            || lower.contains("speech")
            || lower.contains("tts")
            || lower.contains("voice")
        {
            ModelType::Voice
        } else {
            ModelType::Chat
        }
    }

    /// 根据模型 ID 推断能力标签。
    fn infer_capabilities(model_type: &ModelType, model_id: &str) -> Vec<ModelCapability> {
        let mut caps = match model_type {
            ModelType::Chat => vec![ModelCapability::TextChat],
            ModelType::Embedding => vec![],
            ModelType::Voice => vec![ModelCapability::RealtimeVoice],
        };

        let lower = model_id.to_lowercase();
        if lower.contains("vision") || lower.contains("multimodal") || lower.contains("qwen-vl") {
            caps.push(ModelCapability::Vision);
        }
        if lower.starts_with("deepseek-r1")
            || lower.starts_with("qwen3")
            || lower.contains("thinking")
            || lower.contains("reasoning")
        {
            caps.push(ModelCapability::Reasoning);
        }
        caps
    }
}

/// llama-server `/v1/models` 响应结构。
/// 与标准 OpenAI 不同，llama-server 在每个模型条目里嵌套了 `meta` 子对象。
#[derive(Deserialize)]
struct LlamaModelsResponse {
    data: Vec<LlamaModelEntry>,
}

#[derive(Deserialize)]
struct LlamaModelEntry {
    id: String,
    #[serde(default)]
    meta: Option<LlamaModelMeta>,
}

#[derive(Deserialize, Default)]
#[allow(dead_code)]
struct LlamaModelMeta {
    #[serde(default)]
    n_embd: Option<u64>,
    #[serde(default)]
    n_ctx: Option<u64>,
    #[serde(default)]
    n_ctx_train: Option<u64>,
    #[serde(default)]
    n_params: Option<u64>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    ftype: Option<String>,
    #[serde(default)]
    n_vocab: Option<u64>,
}

#[async_trait]
impl ProviderAdapter for LlamaCppAdapter {
    /// 非流式 chat — llama-server 完全兼容 OpenAI chat/completions，直接委托。
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: Arc<ChatRequest>,
    ) -> Result<ChatResponse> {
        self.inner.chat(ctx, request).await
    }

    /// 流式 chat — llama-server 的 SSE 格式与 OpenAI 兼容，直接委托。
    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        self.inner.chat_stream(ctx, request, cancel_token)
    }

    /// 模型列表：覆写以解析 llama-server 特有的 `meta` 字段格式。
    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        let url = format!("{}/models", Self::api_url(ctx));

        let client = self.get_client(ctx)?;
        let resp = crate::apply_request_headers(
            client
                .get(&url)
                .timeout(Duration::from_secs(5))
                .header("Authorization", format!("Bearer {}", ctx.api_key)),
            ctx,
        )
        .send()
        .await
        .map_err(|e| {
            AxAgentError::Provider(format!("llama.cpp list_models request failed: {e}"))
        })?;

        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Provider(format!("llama.cpp list_models error {s}: {t}")));
        }

        let body =
            resp.text().await.map_err(|e| AxAgentError::Provider(format!("Read error: {e}")))?;

        // 解析 llama-server 格式: { data: [{ id, meta: {...} }] }
        let parsed: LlamaModelsResponse = serde_json::from_str(&body)
            .map_err(|e| AxAgentError::Provider(format!("llama.cpp models parse error: {e}")))?;

        let models = parsed
            .data
            .into_iter()
            .map(|entry| {
                let model_type = Self::detect_llama_model_type(&entry.id);
                let caps = Self::infer_capabilities(&model_type, &entry.id);
                let max_tokens = entry.meta.as_ref().and_then(|m| m.n_ctx).map(|ctx| ctx as u32);

                Model {
                    provider_id: ctx.provider_id.clone(),
                    model_id: entry.id.clone(),
                    name: entry.id.clone(),
                    group_name: None,
                    model_type,
                    capabilities: caps,
                    max_tokens,
                    max_output_tokens: None,
                    enabled: true,
                    param_overrides: None,
                    input_price_per_mtok: None,
                    output_price_per_mtok: None,
                }
            })
            .collect();

        Ok(models)
    }

    /// llama.cpp 不需要 API key：使用 `/health` 端点探测可达性，而非 `/v1/models`。
    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        let url = format!("{}/health", Self::root_url(ctx));
        let client = self.get_client(ctx)?;

        match client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => {
                tracing::debug!(
                    "[llama_cpp] health check failed: {e}, provider_id={}",
                    ctx.provider_id
                );
                Ok(false)
            },
        }
    }

    /// Embedding — llama-server 的 embeddings 端点与 OpenAI 兼容，直接委托。
    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        self.inner.embed(ctx, request).await
    }
}
