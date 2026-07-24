// SPDX-License-Identifier: AGPL-3.0-only

//! 文心一言（百度千帆 ERNIE）原生适配器。
//!
//! 文心一言通过百度千帆 v2 的 OpenAI 兼容端点
//! (`https://qianfan.baidubce.com/v2/chat/completions`)
//! 提供服务，因此 chat / chat_stream / embed 委托给 [`OpenAIAdapter`]。
//!
//! 文心一言无特殊思考字段。
//!
//! 本适配器重写：
//! - **`list_models`** — 返回文心一言官方模型（ernie 系列）。
//! - **`validate_key`** — 使用文心一言的 base URL 调用 `/models` 端点校验鉴权。

use std::sync::Arc;

use crate::openai::OpenAIAdapter;
use crate::{ProviderAdapter, ProviderRequestContext};
use async_trait::async_trait;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::*;
use futures::Stream;
use std::pin::Pin;

/// 百度千帆 v2 默认 API 端点
const DEFAULT_BASE_URL: &str = "https://qianfan.baidubce.com/v2";

/// 文心一言适配器。
///
/// chat / chat_stream / embed 委托给内部 OpenAI 适配器，
/// 因为千帆 v2 在 `/v2` 前缀下使用 OpenAI 兼容协议。
/// 模型列表与鉴权校验使用文心一言官方端点。
pub struct WenxinAdapter {
    inner: OpenAIAdapter,
}

impl Default for WenxinAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl WenxinAdapter {
    pub fn new() -> Self {
        Self { inner: OpenAIAdapter::new() }
    }

    /// 解析文心一言的有效 base URL。
    fn base_url(ctx: &ProviderRequestContext) -> String {
        ctx.base_url.clone().unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
    }

    /// 构建带代理支持的 HTTP 客户端，委托给内部 OpenAI 适配器。
    #[allow(clippy::result_large_err)]
    fn get_client(&self, ctx: &ProviderRequestContext) -> Result<reqwest::Client> {
        self.inner.get_client(ctx)
    }

    /// 返回文心一言官方模型列表。
    fn builtin_models(provider_id: &str) -> Vec<Model> {
        vec![
            Model {
                provider_id: provider_id.to_string(),
                model_id: "ernie-4.0-8k".to_string(),
                name: "ERNIE 4.0 8K".to_string(),
                group_name: Some("ERNIE".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![ModelCapability::TextChat, ModelCapability::FunctionCalling],
                max_tokens: Some(8192),
                max_output_tokens: None,
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
            Model {
                provider_id: provider_id.to_string(),
                model_id: "ernie-4.0-turbo-8k".to_string(),
                name: "ERNIE 4.0 Turbo 8K".to_string(),
                group_name: Some("ERNIE".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![ModelCapability::TextChat, ModelCapability::FunctionCalling],
                max_tokens: Some(8192),
                max_output_tokens: None,
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
            Model {
                provider_id: provider_id.to_string(),
                model_id: "ernie-3.5-8k".to_string(),
                name: "ERNIE 3.5 8K".to_string(),
                group_name: Some("ERNIE".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![ModelCapability::TextChat, ModelCapability::FunctionCalling],
                max_tokens: Some(8192),
                max_output_tokens: None,
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
            Model {
                provider_id: provider_id.to_string(),
                model_id: "ernie-speed-128k".to_string(),
                name: "ERNIE Speed 128K".to_string(),
                group_name: Some("ERNIE".to_string()),
                model_type: ModelType::Chat,
                capabilities: vec![ModelCapability::TextChat, ModelCapability::FunctionCalling],
                max_tokens: Some(131072),
                max_output_tokens: None,
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            },
        ]
    }
}

#[async_trait]
impl ProviderAdapter for WenxinAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: Arc<ChatRequest>,
    ) -> Result<ChatResponse> {
        self.inner.chat(ctx, request).await
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        self.inner.chat_stream(ctx, request, cancel_token)
    }

    /// 返回文心一言官方模型列表（不调用 API）。
    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        Ok(Self::builtin_models(&ctx.provider_id))
    }

    /// 通过文心一言的 `/models` 端点校验 API Key 有效性。
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
