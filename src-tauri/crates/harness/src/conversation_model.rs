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

/// Session 别名（与 SessionInfo 同构，重构兼容）。
pub type Session = SessionInfo;

/// 会话压缩配置。
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub max_tokens: usize,
    pub strategy: String,
}

/// 会话压缩结果。
#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub compacted: bool,
    pub tokens_saved: usize,
}

/// 运行时错误。
#[derive(Debug, Clone)]
pub enum RuntimeError {
    Timeout,
    TokenLimitExceeded { limit: usize, actual: usize },
    InvalidRequest(String),
    ApiError(String),
    Internal(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::Timeout => write!(f, "request timed out"),
            RuntimeError::TokenLimitExceeded { limit, actual } => {
                write!(f, "token limit exceeded: {actual}/{limit}")
            }
            RuntimeError::InvalidRequest(msg) => write!(f, "invalid request: {msg}"),
            RuntimeError::ApiError(msg) => write!(f, "API error: {msg}"),
            RuntimeError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// 权限模式（与 runtime-core::permissions::PermissionMode 同构）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionMode {
    ReadOnly,
    Prompt,
    WorkspaceWrite,
    DangerFullAccess,
    Allow,
}

impl PermissionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionMode::ReadOnly => "read_only",
            PermissionMode::Prompt => "prompt",
            PermissionMode::WorkspaceWrite => "workspace_write",
            PermissionMode::DangerFullAccess => "danger_full_access",
            PermissionMode::Allow => "allow",
        }
    }
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
