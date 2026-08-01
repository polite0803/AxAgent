// SPDX-License-Identifier: AGPL-3.0-only

//! llama.cpp server 本地推理适配器。
//!
//! 连接本机运行的 [llama.cpp `llama-server`](https://github.com/ggml-org/llama.cpp)。
//! llama-server 提供 OpenAI 兼容的 `/v1/*` 端点（chat / models / embeddings），
//! 因此 chat、流式、模型列表、鉴权探测、embedding 全部委托给 [`OpenAIAdapter`]。
//!
//! 典型用途：本地 embedding 模型（如 BAAI bge-m3 的 GGUF 量化版），
//! 也可作为任意 GGUF 模型的本地推理后端（无需 API key，鉴权头被服务端忽略）。
//!
//! 运行状态查看与启停管理不在此 adapter 中实现，
//! 由 `axagent` 应用层 `commands/local_model.rs` 提供（探测 /health /props /v1/models，
//! 以及子进程托管启动 / 停止）。

use std::sync::Arc;

use crate::openai::OpenAIAdapter;
use crate::{ProviderAdapter, ProviderRequestContext};
use async_trait::async_trait;
use axagent_harness::core_error::Result;
use axagent_harness::types::*;
use futures::Stream;
use std::pin::Pin;

/// Provider adapter for local llama.cpp `llama-server`.
///
/// llama-server 的 `/v1/*` 端点与 OpenAI 协议兼容，全部能力委托内层
/// `OpenAIAdapter`。模型类型识别走 `detect_model_type`（bge-m3 等已被识别为
/// `ModelType::Embedding`）。
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
}

#[async_trait]
impl ProviderAdapter for LlamaCppAdapter {
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
        cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        self.inner.chat_stream(ctx, request, cancel_token)
    }

    /// 模型列表直接走 OpenAI 兼容 `/v1/models`。
    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        self.inner.list_models(ctx).await
    }

    /// llama.cpp 不需要 API key：探测 `/v1/models` 成功即视为可达。
    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        self.inner.validate_key(ctx).await
    }

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        self.inner.embed(ctx, request).await
    }
}
