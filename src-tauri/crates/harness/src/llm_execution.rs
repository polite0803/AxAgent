// SPDX-License-Identifier: AGPL-3.0-only

//! LLM execution service — dependency inversion boundary for LLM calling.
//!
//! `axagent-runtime-core` implements this trait; `axagent-orchestrator` and
//! `axagent-agent` consume it without depending on `runtime-core`.

use async_trait::async_trait;
use std::sync::Arc;

use crate::provider::{ProviderAdapter, ProviderRequestContext};

/// Configuration for an LLM call.
///
/// Mirrors `runtime_core::LlmCallConfig` at the harness level so that
/// orchestrator/agent crates can configure LLM calls without depending on
/// `runtime-core`.
#[derive(Clone, Default)]
pub struct LlmCallConfig {
    /// Whether to enable strict mode
    pub strict_mode: bool,
    /// Maximum context tokens (None = unlimited)
    pub max_context_tokens: Option<u32>,
    /// Reserved output tokens
    pub reserved_output_tokens: Option<u32>,
    /// Session identifier for audit
    pub session_id: Option<String>,
}

/// Result of an LLM call.
#[derive(Clone)]
pub struct LlmCallResult {
    /// The text content of the response
    pub content: String,
}

/// Service for executing LLM calls through harness abstractions.
///
/// Implementations wrap `runtime_core::execute_llm()` and bridge the
/// concrete `LlmCallConfig` / `ChatRequest` types.
#[async_trait]
pub trait LlmExecutionService: Send + Sync {
    /// Execute an LLM call with the given parameters.
    ///
    /// `messages` is a JSON-serialized chat request payload.
    async fn execute(
        &self,
        adapter: &(dyn ProviderAdapter + '_),
        ctx: &ProviderRequestContext,
        messages: serde_json::Value,
        config: &LlmCallConfig,
    ) -> Result<LlmCallResult, String>;
}

/// Arc wrapper for convenience.
pub type SharedLlmExecutionService = Arc<dyn LlmExecutionService>;
