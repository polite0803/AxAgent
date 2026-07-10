// SPDX-License-Identifier: AGPL-3.0-only

//! 会话压缩类型 — 从 `axagent-runtime-core::compact` 搬迁的纯数据定义。

use crate::PromptProvider;
use crate::conversation_model::{ContentBlock, ConversationMessage, MessageRole};
use crate::runtime_types::session::Session;

/// 控制会话何时以及如何压缩的阈值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionConfig {
    pub preserve_recent_messages: usize,
    pub max_estimated_tokens: usize,
    /// 是否在压缩期间提取每轮摘要。
    pub enable_turn_summaries: bool,
    /// 是否在评分消息时应用基于距离的相关性衰减。
    pub enable_distance_decay: bool,
    /// 是否在检测到任务边界后自动清理上下文。
    pub enable_task_boundary_cleanup: bool,
    /// 消息的最大年龄（从末尾往前），超过后将被激进裁剪。
    pub max_turn_age: Option<usize>,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            preserve_recent_messages: 12,
            max_estimated_tokens: 80_000,
            enable_turn_summaries: true,
            enable_distance_decay: true,
            enable_task_boundary_cleanup: true,
            max_turn_age: Some(50),
        }
    }
}

/// 紧急压缩配置：熔断器触发后的超激进模式。
#[must_use]
pub fn emergency_compaction_config() -> CompactionConfig {
    CompactionConfig {
        preserve_recent_messages: 1,
        max_estimated_tokens: 5_000,
        enable_turn_summaries: true,
        enable_distance_decay: true,
        enable_task_boundary_cleanup: true,
        max_turn_age: Some(5),
    }
}

/// 压缩结果：摘要 + 保留尾部消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionResult {
    pub summary: String,
    pub formatted_summary: String,
    pub compacted_session: Session,
    pub removed_message_count: usize,
}

/// 粗略估算会话 transcript 的 token 占用。
#[must_use]
pub fn estimate_session_tokens(session: &Session) -> usize {
    session.messages.iter().map(estimate_message_tokens).sum()
}

/// 估算单条消息的 token 数（以 4 字符 ≈ 1 token 估算）。
#[must_use]
pub fn estimate_message_tokens(message: &ConversationMessage) -> usize {
    message
        .blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => text.len() / 4 + 1,
            ContentBlock::ToolUse { name, input, .. } => (name.len() + input.len()) / 4 + 1,
            ContentBlock::ToolResult { tool_name, output, .. } => {
                (tool_name.len() + output.len()) / 4 + 1
            },
        })
        .sum()
}

/// 判断会话是否超过压缩预算阈值。
#[must_use]
pub fn should_compact(
    session: &Session,
    config: CompactionConfig,
    provider: &dyn PromptProvider,
) -> bool {
    let start = if session.messages.first().is_some_and(|msg| has_compacted_summary(msg, provider))
    {
        1
    } else {
        0
    };
    let compactable = &session.messages[start..];

    compactable.len() > config.preserve_recent_messages
        && compactable.iter().map(estimate_message_tokens).sum::<usize>()
            >= config.max_estimated_tokens
}

fn has_compacted_summary(message: &ConversationMessage, provider: &dyn PromptProvider) -> bool {
    message.role == MessageRole::System
        && first_text_block(message)
            .is_some_and(|text| text.starts_with(compact_continuation_preamble(provider)))
}

fn first_text_block(message: &ConversationMessage) -> Option<&str> {
    message.blocks.iter().find_map(|block| match block {
        ContentBlock::Text { text } => Some(text.as_str()),
        _ => None,
    })
}

fn compact_continuation_preamble(provider: &dyn PromptProvider) -> &'static str {
    provider.get("compact.continuation_preamble", crate::prompt_provider::PromptLang::EnUS)
}

// ── 消息重要性评分（从 runtime-core message_importance 搬迁）──

/// 消息重要性分数 (0-100)
#[must_use]
pub fn score_message(msg: &ConversationMessage) -> u32 {
    let mut score = 50;

    if msg.role == MessageRole::User {
        score += 20;
    }

    let has_tool_use = msg.blocks.iter().any(|b| matches!(b, ContentBlock::ToolUse { .. }));
    if has_tool_use {
        score += 15;
    }

    let has_error =
        msg.blocks.iter().any(|b| matches!(b, ContentBlock::ToolResult { is_error: true, .. }));
    if has_error {
        score -= 10;
    }

    let text_len: usize = msg
        .blocks
        .iter()
        .map(|b| match b {
            ContentBlock::Text { text } => text.len(),
            _ => 0,
        })
        .sum();
    if text_len > 500 {
        score += 10;
    }
    if text_len < 20 {
        score -= 5;
    }

    score.clamp(0, 100)
}

/// 选择保留的消息：按重要性排序，保留 top N 条。
/// 返回值为原始索引列表，按原始顺序排序。
#[must_use]
pub fn select_top_messages(messages: &[ConversationMessage], keep_count: usize) -> Vec<usize> {
    let actual_keep = keep_count.min(messages.len());
    if actual_keep == 0 || messages.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(usize, u32)> =
        messages.iter().enumerate().map(|(i, msg)| (i, score_message(msg))).collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.1));
    let mut indices: Vec<usize> = scored.iter().take(actual_keep).map(|(i, _)| *i).collect();
    indices.sort();
    indices
}
