// SPDX-License-Identifier: AGPL-3.0-only

//! Conversation model DTOs — Authoritative source of shared data types.
//!
//! These are the **canonical** definitions of `ConversationMessage`,
//! `ContentBlock`, `TokenUsage`, etc.  Downstream crates (runtime-core, agent)
//! MUST `pub use axagent_harness::*` instead of repeating the definitions.
//!
//! Field layouts are the single source of truth.  When the business model
//! evolves, change them here first.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── MessageRole ──────────────────────────────────────────────────────────────
//
// Re-export the canonical MessageRole from harness::types so we avoid two
// identical enums that differ only in crate path.

pub use crate::types::MessageRole;

// ── ContentBlock ─────────────────────────────────────────────────────────────

/// Authoritative definition of a content block (text / tool-use / tool-result).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: String },
    ToolResult { tool_use_id: String, tool_name: String, output: String, is_error: bool },
}

// ── ConversationMessage ──────────────────────────────────────────────────────

/// Authoritative definition of a conversation message (role + content + optional usage).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub blocks: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

// ── TokenUsage ───────────────────────────────────────────────────────────────

/// Authoritative definition of per-turn / per-session token counters.
///
/// Field semantics match runtime-core conventions (DeepSeek-style fields).
/// - `input_tokens`, `output_tokens`: provider-chargeable totals
/// - `cache_creation_input_tokens`: prompt caching write tokens
/// - `cache_read_input_tokens`: prompt caching hit tokens
/// - `cache_miss_input_tokens`: optional true miss value (DeepSeek-specific)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
    #[serde(default)]
    pub cache_miss_input_tokens: Option<u32>,
}

impl TokenUsage {
    /// Total tokens consumed = input + output + cache_creation + cache_read.
    ///
    /// This is the canonical aggregate across the entire codebase.  Downstream
    /// crates must NOT redefine this method.
    #[must_use]
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
    }
}

// ── SessionInfo (minimal DTO) ────────────────────────────────────────────────

/// Minimal session info that agent needs — full Session stays in runtime-core.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub token_usage: Option<TokenUsage>,
}
