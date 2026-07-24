// SPDX-License-Identifier: AGPL-3.0-only

//! DeepSeek 原生适配器。
//!
//! DeepSeek 提供与 OpenAI Chat Completions 兼容的端点 (`/v1/chat/completions`)，
//! 因此 chat / chat_stream / embed 委托给 [`OpenAIAdapter`]。
//!
//! 特有能力：`deepseek-reasoner` 模型在响应中返回 `reasoning_content` 字段，
//! 表示深度思考过程。该字段已被 [`OpenAIAdapter`] 的 `extract_thinking` 函数
//! 解析并映射到 harness 的 `thinking` 字段，无需在此重复实现。
//!
//! 本适配器重写：
//! - **`list_models`** — 返回 DeepSeek 官方模型（`deepseek-chat` / `deepseek-reasoner`）。
//! - **`validate_key`** — 使用 DeepSeek 的 base URL 调用 `/models` 端点校验鉴权。

use std::sync::Arc;

use crate::openai::OpenAIAdapter;
use crate::{ProviderAdapter, ProviderRequestContext};
use async_trait::async_trait;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::*;
use futures::Stream;
use std::pin::Pin;

/// DeepSeek 默认 API 端点
const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";

/// DeepSeek 适配器。
///
/// chat / chat_stream / embed 委托给内部 OpenAI 适配器，
/// 因为 DeepSeek 在 `/v1/` 前缀下使用 OpenAI 兼容协议。
/// 模型列表与鉴权校验使用 DeepSeek 官方端点。
pub struct DeepSeekAdapter {
    inner: OpenAIAdapter,
}

impl Default for DeepSeekAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl DeepSeekAdapter {
    pub fn new() -> Self {
        Self { inner: OpenAIAdapter::new() }
    }

    /// 解析 DeepSeek 的有效 base URL。
    fn base_url(ctx: &ProviderRequestContext) -> String {
        ctx.base_url.clone().unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
    }

    /// 构建带代理支持的 HTTP 客户端，委托给内部 OpenAI 适配器。
    #[allow(clippy::result_large_err)]
    fn get_client(&self, ctx: &ProviderRequestContext) -> Result<reqwest::Client> {
        self.inner.get_client(ctx)
    }

    /// 返回 DeepSeek 官方模型列表。
    fn builtin_models(provider_id: &str) -> Vec<Model> {
        vec![
            Model {
                provider_id: provider_id.to_string(),
                model_id: "deepseek-chat".to_string(),
                name: "DeepSeek Chat".to_string(),
                group_name: Some("DeepSeek".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![ModelCapability::TextChat, ModelCapability::FunctionCalling],
                max_tokens: Some(65536),
                max_output_tokens: Some(8192),
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
            Model {
                provider_id: provider_id.to_string(),
                model_id: "deepseek-reasoner".to_string(),
                name: "DeepSeek Reasoner".to_string(),
                group_name: Some("DeepSeek".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![
                    ModelCapability::TextChat,
                    ModelCapability::FunctionCalling,
                    ModelCapability::Reasoning,
                ],
                max_tokens: Some(65536),
                max_output_tokens: Some(32768),
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
        ]
    }
}

#[async_trait]
impl ProviderAdapter for DeepSeekAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: Arc<ChatRequest>,
    ) -> Result<ChatResponse> {
        // 委托给 OpenAI 适配器：reasoning_content 字段已由 extract_thinking 解析
        self.inner.chat(ctx, request).await
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        // 委托给 OpenAI 适配器：reasoning_content 字段已由 extract_thinking 解析
        self.inner.chat_stream(ctx, request, cancel_token)
    }

    /// 返回 DeepSeek 官方模型列表（不调用 API）。
    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        Ok(Self::builtin_models(&ctx.provider_id))
    }

    /// 通过 DeepSeek 的 `/models` 端点校验 API Key 有效性。
    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        let url = format!("{}/models", Self::base_url(ctx));
        let resp = crate::apply_request_headers(
            self.get_client(ctx)?
                .get(&url)
                .header("Authorization", format!("Bearer {}", ctx.api_key)),
            ctx,
        )
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Request failed: {e}")))?;
        let status = resp.status().as_u16();
        Ok(status != 401 && status != 403)
    }

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        self.inner.embed(ctx, request).await
    }
}
