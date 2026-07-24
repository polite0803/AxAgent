// SPDX-License-Identifier: AGPL-3.0-only

//! 智谱 GLM 原生适配器。
//!
//! 智谱 GLM 通过 BigModel 开放平台
//! (`https://open.bigmodel.cn/api/paas/v4/chat/completions`)
//! 提供 OpenAI 兼容的 Chat Completions 端点，
//! 因此 chat / chat_stream / embed 委托给 [`OpenAIAdapter`]。
//!
//! 特有能力：glm-4 系列模型在响应中返回 `thinking` 字段，表示思考过程。
//! 该字段已被 [`OpenAIAdapter`] 的 `extract_thinking` 函数解析并映射到
//! harness 的 `thinking` 字段，无需在此重复实现。
//!
//! 本适配器重写：
//! - **`list_models`** — 返回智谱 GLM 官方模型。
//! - **`validate_key`** — 使用智谱 GLM 的 base URL 调用 `/models` 端点校验鉴权。

use std::sync::Arc;

use crate::openai::OpenAIAdapter;
use crate::{ProviderAdapter, ProviderRequestContext};
use async_trait::async_trait;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::*;
use futures::Stream;
use std::pin::Pin;

/// 智谱 GLM BigModel 开放平台默认端点
const DEFAULT_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

/// 智谱 GLM 适配器。
///
/// chat / chat_stream / embed 委托给内部 OpenAI 适配器，
/// 因为 BigModel 在 `/api/paas/v4` 前缀下使用 OpenAI 兼容协议。
/// 模型列表与鉴权校验使用智谱 GLM 官方端点。
pub struct GlmAdapter {
    inner: OpenAIAdapter,
}

impl Default for GlmAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl GlmAdapter {
    pub fn new() -> Self {
        Self { inner: OpenAIAdapter::new() }
    }

    /// 解析智谱 GLM 的有效 base URL。
    fn base_url(ctx: &ProviderRequestContext) -> String {
        ctx.base_url.clone().unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
    }

    /// 构建带代理支持的 HTTP 客户端，委托给内部 OpenAI 适配器。
    #[allow(clippy::result_large_err)]
    fn get_client(&self, ctx: &ProviderRequestContext) -> Result<reqwest::Client> {
        self.inner.get_client(ctx)
    }

    /// 返回智谱 GLM 官方模型列表。
    fn builtin_models(provider_id: &str) -> Vec<Model> {
        vec![
            Model {
                provider_id: provider_id.to_string(),
                model_id: "glm-4".to_string(),
                name: "GLM-4".to_string(),
                group_name: Some("GLM".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![
                    ModelCapability::TextChat,
                    ModelCapability::FunctionCalling,
                    ModelCapability::Vision,
                ],
                max_tokens: Some(131072),
                max_output_tokens: Some(4096),
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
            Model {
                provider_id: provider_id.to_string(),
                model_id: "glm-4-plus".to_string(),
                name: "GLM-4 Plus".to_string(),
                group_name: Some("GLM".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![
                    ModelCapability::TextChat,
                    ModelCapability::FunctionCalling,
                    ModelCapability::Vision,
                ],
                max_tokens: Some(131072),
                max_output_tokens: Some(4096),
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
            Model {
                provider_id: provider_id.to_string(),
                model_id: "glm-4-air".to_string(),
                name: "GLM-4 Air".to_string(),
                group_name: Some("GLM".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![ModelCapability::TextChat, ModelCapability::FunctionCalling],
                max_tokens: Some(131072),
                max_output_tokens: Some(4096),
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
            Model {
                provider_id: provider_id.to_string(),
                model_id: "glm-4-flash".to_string(),
                name: "GLM-4 Flash".to_string(),
                group_name: Some("GLM".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![ModelCapability::TextChat, ModelCapability::FunctionCalling],
                max_tokens: Some(131072),
                max_output_tokens: Some(4096),
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
        ]
    }
}

#[async_trait]
impl ProviderAdapter for GlmAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: Arc<ChatRequest>,
    ) -> Result<ChatResponse> {
        // 委托给 OpenAI 适配器：thinking 字段已由 extract_thinking 解析
        self.inner.chat(ctx, request).await
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        // 委托给 OpenAI 适配器：thinking 字段已由 extract_thinking 解析
        self.inner.chat_stream(ctx, request, cancel_token)
    }

    /// 返回智谱 GLM 官方模型列表（不调用 API）。
    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        Ok(Self::builtin_models(&ctx.provider_id))
    }

    /// 通过智谱 GLM 的 `/models` 端点校验 API Key 有效性。
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
