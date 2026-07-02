// SPDX-License-Identifier: AGPL-3.0-only

//! 公共多模式 Provider 适配器（Multi-Mode Adapter）
//!
//! P2-7 修复：原 HermesAdapter / OpenClawAdapter 各自实现一遍 `chat` / `chat_stream` /
//! `list_models` / `validate_key` / `embed` 的 5 路模式分发（`ApiMode::ChatCompletions`
//! / `CodexResponses` / `AnthropicMessages`），共 200+ 行重复代码。
//!
//! 本模块提供 `MultiModeAdapter` 通用结构，把模式解析 + 路由逻辑下沉到一处，
//! HermesAdapter / OpenClawAdapter 仅保留各自特有的 jobs API 逻辑。
//!
//! P2-8 修复：原 `hermes_request` / `openclaw_request` 使用 `Result<String>` 风格，
//! 与本 crate 其他 adapter 统一返回 `AxAgentError::Provider(...)` 的约定不一致。
//! 统一为 `Result<String>` + 内部用 `AxAgentError::Provider` 包裹。

use async_trait::async_trait;
use axagent_core::error::{AxAgentError, Result};
use axagent_harness::ProviderAdapter;
use axagent_harness::ProviderRequestContext;
use axagent_harness::types::*;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

use crate::anthropic::AnthropicAdapter;
use crate::openai::OpenAIAdapter;
use crate::openai_responses::OpenAIResponsesAdapter;

/// Multi-Mode 适配器解析出的 API 协议模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiMode {
    /// OpenAI Chat Completions 协议（`/v1/chat/completions`）
    ChatCompletions,
    /// OpenAI Codex Responses 协议（`/v1/responses`）
    CodexResponses,
    /// Anthropic Messages 协议（`/v1/messages`）
    AnthropicMessages,
}

impl ApiMode {
    /// 从 base_url 启发式推断协议模式。
    pub fn detect_from_url(url: &str) -> Self {
        let url = url.trim_end_matches('/').to_lowercase();
        if url.contains("/anthropic") || url.contains("/v1/messages") {
            Self::AnthropicMessages
        } else if url.contains("/responses") || url.contains("/v1/responses") {
            Self::CodexResponses
        } else {
            Self::ChatCompletions
        }
    }

    /// 解析 ProviderRequestContext 中的协议提示（api_mode 优先，api_path 兜底，base_url 最后）。
    pub fn resolve(ctx: &ProviderRequestContext) -> Self {
        if let Some(mode) = ctx.api_mode.as_deref() {
            match mode.to_lowercase().as_str() {
                "chat_completions" | "chatcompletions" => return Self::ChatCompletions,
                "codex_responses" | "responses" | "openai_responses" => {
                    return Self::CodexResponses;
                },
                "anthropic_messages" | "anthropic" | "messages" => {
                    return Self::AnthropicMessages;
                },
                _ => {},
            }
        }

        ctx.api_path
            .as_deref()
            .map(|p| {
                if p.contains("anthropic") || p.contains("/messages") {
                    Self::AnthropicMessages
                } else if p.contains("responses") {
                    Self::CodexResponses
                } else {
                    Self::ChatCompletions
                }
            })
            .unwrap_or_else(|| {
                let base = ctx.base_url.as_deref().unwrap_or("");
                Self::detect_from_url(base)
            })
    }
}

/// 多模式 Provider 适配器：内部路由到 OpenAI / Anthropic / Codex-Responses 三种具体 adapter。
///
/// 通过 `Arc` 共享内部子 adapter，避免每次 `clone` 复制整个对象。
/// 内部子 adapter 不是 `Clone`（依赖 `reqwest::Client` 等非 Clone 字段）。
pub struct MultiModeAdapter {
    chat_completions: Arc<OpenAIAdapter>,
    codex_responses: Arc<OpenAIResponsesAdapter>,
    anthropic: Arc<AnthropicAdapter>,
}

impl MultiModeAdapter {
    pub fn new() -> Self {
        Self {
            chat_completions: Arc::new(OpenAIAdapter::new()),
            codex_responses: Arc::new(OpenAIResponsesAdapter::new()),
            anthropic: Arc::new(AnthropicAdapter::new()),
        }
    }

    pub async fn chat_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        mode: ApiMode,
    ) -> Result<ChatResponse> {
        match mode {
            ApiMode::ChatCompletions => self.chat_completions.chat(ctx, request).await,
            ApiMode::CodexResponses => self.codex_responses.chat(ctx, request).await,
            ApiMode::AnthropicMessages => self.anthropic.chat(ctx, request).await,
        }
    }

    pub fn chat_stream_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
        mode: ApiMode,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        match mode {
            ApiMode::ChatCompletions => {
                self.chat_completions
                    .chat_stream(ctx, request, cancel_token)
            },
            ApiMode::CodexResponses => self.codex_responses.chat_stream(ctx, request, cancel_token),
            ApiMode::AnthropicMessages => self.anthropic.chat_stream(ctx, request, cancel_token),
        }
    }

    pub async fn list_models_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        mode: ApiMode,
    ) -> Result<Vec<Model>> {
        match mode {
            ApiMode::ChatCompletions => self.chat_completions.list_models(ctx).await,
            ApiMode::CodexResponses => self.codex_responses.list_models(ctx).await,
            ApiMode::AnthropicMessages => self.anthropic.list_models(ctx).await,
        }
    }

    pub async fn validate_key_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        mode: ApiMode,
    ) -> Result<bool> {
        match mode {
            ApiMode::ChatCompletions => self.chat_completions.validate_key(ctx).await,
            ApiMode::CodexResponses => self.codex_responses.validate_key(ctx).await,
            ApiMode::AnthropicMessages => self.anthropic.validate_key(ctx).await,
        }
    }

    pub async fn embed_with_mode(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
        mode: ApiMode,
    ) -> Result<EmbedResponse> {
        match mode {
            ApiMode::ChatCompletions => self.chat_completions.embed(ctx, request).await,
            ApiMode::CodexResponses => self.codex_responses.embed(ctx, request).await,
            ApiMode::AnthropicMessages => Err(AxAgentError::Provider(
                "Embed endpoint is not supported in anthropic_messages mode".to_string(),
            )),
        }
    }

    /// 暴露 codex_responses 内部 adapter，用于调用
    /// `get_response` / `delete_response` 等 ProviderAdapter trait 中按模式分派不到的方法。
    pub fn codex_responses(&self) -> &OpenAIResponsesAdapter {
        &self.codex_responses
    }
}

impl Default for MultiModeAdapter {
    fn default() -> Self {
        Self::new()
    }
}
