// SPDX-License-Identifier: AGPL-3.0-only

//! Conversation model DTOs — dependency-inversion boundary for agent ↔ runtime-core.
//!
//! These DTOs mirror `axagent_runtime_core` types for `ConversationMessage`,
//! `ContentBlock`, `TokenUsage`, etc., allowing the agent crate to reference
//! them without depending on `axagent-runtime-core`.
//!
//! Field layouts intentionally match the runtime-core counterparts so that
//! dependents can migrate with minimal friction.  When the runtime-core types
//! evolve, these DTOs must be updated together.

use serde::{Deserialize, Serialize};

// ── MessageRole ──────────────────────────────────────────────────────────────
//
// Re-export the canonical MessageRole from harness::types so we avoid two
// identical enums that differ only in crate path.

pub use crate::types::MessageRole;

// ── ContentBlock ─────────────────────────────────────────────────────────────

/// Mirrors `axagent_runtime_core::session::ContentBlock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        tool_use_id: String,
        tool_name: String,
        output: String,
        is_error: bool,
    },
}

// ── ConversationMessage ──────────────────────────────────────────────────────

/// Mirrors `axagent_runtime_core::session::ConversationMessage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub blocks: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

// ── TokenUsage ───────────────────────────────────────────────────────────────

/// Mirrors `axagent_runtime_core::usage::TokenUsage`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
    #[serde(default)]
    pub cache_miss_input_tokens: Option<u32>,
}

impl TokenUsage {
    /// Total tokens consumed = input + output.
    #[must_use]
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

// ── SessionInfo (minimal DTO) ────────────────────────────────────────────────

/// Minimal session info that agent needs — full Session stays in runtime-core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub token_usage: Option<TokenUsage>,
}

// ── TurnSummary ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub turn_id: String,
    pub session_id: String,
    pub summary: String,
    pub tool_calls: Vec<String>,
    pub token_usage: TokenUsage,
}
