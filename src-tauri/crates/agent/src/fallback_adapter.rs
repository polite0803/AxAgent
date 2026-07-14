// SPDX-License-Identifier: AGPL-3.0-only

//! Provider 降级适配器 — 主适配器失败时自动尝试备用适配器。
//!
//! 用法：在 agent_query 中创建 FallbackProviderAdapter，传入主适配器和备用适配器列表。
//! 当 `chat()` 调用失败时，会按顺序尝试备用适配器（使用各自的 ProviderRequestContext），
//! 直到成功或全部失败。

use async_trait::async_trait;
use axagent_harness::core_error::Result;
use axagent_harness::types::*;
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// 备用适配器条目：适配器 + 其专属请求上下文（含独立 API key、base URL 等）
struct FallbackEntry {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
}

/// 带 fallback 的 ProviderAdapter 包装器。
/// 主适配器调用失败时，按顺序尝试备用适配器。
pub struct FallbackProviderAdapter {
    primary: Arc<dyn ProviderAdapter>,
    fallbacks: Vec<FallbackEntry>,
}

impl FallbackProviderAdapter {
    pub fn new(
        primary: Arc<dyn ProviderAdapter>,
        fallback_adapters: Vec<Arc<dyn ProviderAdapter>>,
        fallback_contexts: Vec<ProviderRequestContext>,
    ) -> Self {
        assert_eq!(
            fallback_adapters.len(),
            fallback_contexts.len(),
            "FallbackProviderAdapter: adapters and contexts must have the same length"
        );
        let fallbacks = fallback_adapters
            .into_iter()
            .zip(fallback_contexts)
            .map(|(adapter, ctx)| FallbackEntry { adapter, ctx })
            .collect();
        Self { primary, fallbacks }
    }

    /// 备用适配器数量
    pub fn fallback_count(&self) -> usize {
        self.fallbacks.len()
    }
}

#[async_trait]
impl ProviderAdapter for FallbackProviderAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: Arc<ChatRequest>,
    ) -> Result<ChatResponse> {
        match self.primary.chat(ctx, request.clone()).await {
            Ok(resp) => Ok(resp),
            Err(primary_err) => {
                tracing::warn!(
                    "Provider fallback: primary adapter failed ({}), trying {} fallback(s)",
                    primary_err,
                    self.fallbacks.len()
                );
                for (i, entry) in self.fallbacks.iter().enumerate() {
                    match entry.adapter.chat(&entry.ctx, request.clone()).await {
                        Ok(resp) => {
                            tracing::warn!(
                                "Provider fallback: using fallback #{} after primary error: {}",
                                i + 1,
                                primary_err
                            );
                            return Ok(resp);
                        },
                        Err(fb_err) => {
                            tracing::warn!("Fallback #{} also failed: {}", i + 1, fb_err);
                        },
                    }
                }
                Err(primary_err)
            },
        }
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<Arc<AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        // 流式调用不支持 fallback（流已消费后无法重试）
        self.primary.chat_stream(ctx, request, cancel_token)
    }

    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        self.primary.list_models(ctx).await
    }

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        self.primary.embed(ctx, request).await
    }
}
