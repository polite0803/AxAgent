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

// ── 历史消息回灌（PLAN-codex-parity P0-3 跨进程上下文重建） ────────────────────

/// 将 DB 消息行（`messages.parts` JSON + `content`）转换为运行时
/// `ConversationMessage` 序列，供进程重启后重建 `Session.messages`。
///
/// 规则：
/// - `System` 行跳过（system prompt 由运行时另行构建，历史 system 行不参与）；
/// - `Assistant` 行拆分为 `[assistant(text + tool_use)]` + `[tool(tool_result)]`：
///   DB 把一个 turn 的 assistant 块与 tool 观察合并进同一条消息的 parts，
///   而生产 `run_turn` 的消息形状是 assistant(tool_use) → tool(tool_result)
///   相邻两条（Anthropic 等供应商要求 tool_result 不能混在 assistant 消息里）；
/// - `parts` 缺失或解析失败时回退用 `content` 文本（用户消息不写 parts）；
/// - 空 parts + 空 content → 返回空（调用方自然跳过）。
pub fn history_to_conversation_messages(
    role: MessageRole,
    content: &str,
    parts: Option<&str>,
) -> Vec<ConversationMessage> {
    use crate::types::ContentBlock as HistoryBlock;

    if matches!(role, MessageRole::System) {
        return Vec::new();
    }

    let parsed: Option<Vec<HistoryBlock>> =
        parts.filter(|p| !p.trim().is_empty()).and_then(|p| serde_json::from_str(p).ok());

    let blocks: Vec<ContentBlock> = match parsed {
        Some(list) if !list.is_empty() => list
            .into_iter()
            .map(|block| match block {
                HistoryBlock::Text { text } => ContentBlock::Text { text },
                HistoryBlock::ToolUse { id, name, input } => {
                    ContentBlock::ToolUse { id, name, input }
                },
                HistoryBlock::ToolResult { tool_use_id, tool_name, output, is_error } => {
                    ContentBlock::ToolResult { tool_use_id, tool_name, output, is_error }
                },
            })
            .collect(),
        _ => {
            if content.trim().is_empty() {
                Vec::new()
            } else {
                vec![ContentBlock::Text { text: content.to_string() }]
            }
        },
    };

    if blocks.is_empty() {
        return Vec::new();
    }

    if matches!(role, MessageRole::Assistant) {
        let mut assistant_blocks = Vec::new();
        let mut tool_results = Vec::new();
        for block in blocks {
            match block {
                ContentBlock::ToolResult { .. } => tool_results.push(block),
                other => assistant_blocks.push(other),
            }
        }
        let mut out = Vec::with_capacity(2);
        if !assistant_blocks.is_empty() {
            out.push(ConversationMessage {
                role: MessageRole::Assistant,
                blocks: assistant_blocks,
                usage: None,
            });
        }
        if !tool_results.is_empty() {
            out.push(ConversationMessage {
                role: MessageRole::Tool,
                blocks: tool_results,
                usage: None,
            });
        }
        return out;
    }

    vec![ConversationMessage { role, blocks, usage: None }]
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
    #[serde(alias = "input_tokens")]
    pub input_tokens: u32,
    #[serde(alias = "output_tokens")]
    pub output_tokens: u32,
    #[serde(alias = "cache_creation_input_tokens")]
    pub cache_creation_input_tokens: u32,
    #[serde(alias = "cache_read_input_tokens")]
    pub cache_read_input_tokens: u32,
    #[serde(default, alias = "cache_miss_input_tokens")]
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
    #[serde(alias = "session_id")]
    pub session_id: String,
    #[serde(alias = "user_id")]
    pub user_id: String,
    pub title: Option<String>,
    #[serde(alias = "created_at")]
    pub created_at: i64,
    #[serde(alias = "updated_at")]
    pub updated_at: i64,
    #[serde(alias = "token_usage")]
    pub token_usage: Option<TokenUsage>,
}

#[cfg(test)]
mod history_tests {
    use super::*;

    #[test]
    fn assistant_parts_split_into_assistant_and_tool_messages() {
        let parts = r#"[
            {"type":"text","text":"开始查文件"},
            {"type":"tool_use","id":"t1","name":"Read","input":"{\"path\":\"a.rs\"}"},
            {"type":"tool_result","toolUseId":"t1","toolName":"Read","output":"ok","isError":false}
        ]"#;
        let out =
            history_to_conversation_messages(MessageRole::Assistant, "开始查文件", Some(parts));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].role, MessageRole::Assistant);
        assert_eq!(out[0].blocks.len(), 2);
        assert!(matches!(&out[0].blocks[0], ContentBlock::Text { .. }));
        assert!(matches!(&out[0].blocks[1], ContentBlock::ToolUse { .. }));
        assert_eq!(out[1].role, MessageRole::Tool);
        assert!(matches!(&out[1].blocks[0], ContentBlock::ToolResult { .. }));
    }

    #[test]
    fn user_message_without_parts_falls_back_to_content() {
        let out = history_to_conversation_messages(MessageRole::User, "你好", None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].role, MessageRole::User);
        assert_eq!(out[0].blocks, vec![ContentBlock::Text { text: "你好".to_string() }]);
    }

    #[test]
    fn system_rows_are_skipped() {
        let out = history_to_conversation_messages(MessageRole::System, "sys", None);
        assert!(out.is_empty());
    }

    #[test]
    fn empty_content_without_parts_is_skipped() {
        let out = history_to_conversation_messages(MessageRole::User, "   ", None);
        assert!(out.is_empty());
    }

    #[test]
    fn invalid_parts_json_falls_back_to_content() {
        let out =
            history_to_conversation_messages(MessageRole::Assistant, "fallback", Some("not-json"));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].blocks, vec![ContentBlock::Text { text: "fallback".to_string() }]);
    }
}
