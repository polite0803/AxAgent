// SPDX-License-Identifier: AGPL-3.0-only

//! 会话压缩核心逻辑（无 HookRunner 运行时依赖）
//! 从 `axagent-runtime-core::compact` 搬迁。

use std::collections::HashSet;

use crate::conversation_model::{ContentBlock, ConversationMessage, MessageRole};
use crate::prompt_provider::{PromptLang, PromptProvider};
use crate::runtime_types::compact::{
    CompactionConfig, CompactionResult, score_message, select_top_messages, should_compact,
};
use crate::runtime_types::session::Session;

fn compact_continuation_preamble(provider: &dyn PromptProvider) -> &'static str {
    provider.get("compact.continuation_preamble", PromptLang::EnUS)
}
fn compact_recent_messages_note(provider: &dyn PromptProvider) -> &'static str {
    provider.get("compact.recent_messages_note", PromptLang::EnUS)
}
fn compact_direct_resume_instruction(provider: &dyn PromptProvider) -> &'static str {
    provider.get("compact.resume_instruction", PromptLang::EnUS)
}

/// Normalizes a compaction summary into user-facing continuation text.
#[must_use]
pub fn format_compact_summary(summary: &str) -> String {
    let without_analysis = strip_tag_block(summary, "analysis");
    let formatted = if let Some(content) = extract_tag_block(&without_analysis, "summary") {
        without_analysis.replace(
            &format!("<summary>{content}</summary>"),
            &format!("Summary:\n{}", content.trim()),
        )
    } else {
        without_analysis
    };

    collapse_blank_lines(&formatted).trim().to_string()
}

/// Builds the synthetic system message used after session compaction.
#[must_use]
pub fn get_compact_continuation_message(
    summary: &str,
    suppress_follow_up_questions: bool,
    recent_messages_preserved: bool,
    provider: &dyn PromptProvider,
) -> String {
    let mut base = compact_continuation_preamble(provider).to_string();
    base.push_str(&format_compact_summary(summary));

    if recent_messages_preserved {
        base.push_str("\n\n");
        base.push_str(compact_recent_messages_note(provider));
    }

    if suppress_follow_up_questions {
        base.push('\n');
        base.push_str(compact_direct_resume_instruction(provider));
    }

    base
}

/// Compacts a session by summarizing older messages and preserving the recent tail.
#[must_use]
pub fn compact_session(
    session: &Session,
    config: CompactionConfig,
    provider: &dyn PromptProvider,
) -> CompactionResult {
    if !should_compact(session, config, provider) {
        return CompactionResult {
            summary: String::new(),
            formatted_summary: String::new(),
            compacted_session: session.clone(),
            removed_message_count: 0,
        };
    }

    let existing_summary =
        session.messages.first().and_then(|m| extract_existing_compacted_summary(m, provider));
    let compacted_prefix_len = usize::from(existing_summary.is_some());
    let raw_keep_from = session.messages.len().saturating_sub(config.preserve_recent_messages);
    // Ensure we do not split a tool-use / tool-result pair at the compaction
    // boundary. If the first preserved message is a user message whose first
    // block is a ToolResult, the assistant message with the matching ToolUse
    // was slated for removal — that produces an orphaned tool role message on
    // the OpenAI-compat path (400: tool message must follow assistant with
    // tool_calls). Walk the boundary back until we start at a safe point.
    let keep_from = {
        let mut k = raw_keep_from;
        // If the first preserved message is a tool-result turn, ensure its
        // paired assistant tool-use turn is preserved too. Without this fix,
        // the OpenAI-compat adapter sends an orphaned 'tool' role message
        // with no preceding assistant 'tool_calls', which providers reject
        // with a 400. We walk back only if the immediately preceding message
        // is NOT an assistant message that contains a ToolUse block (i.e. the
        // pair is actually broken at the boundary).
        loop {
            if k == 0 || k <= compacted_prefix_len {
                break;
            }
            let first_preserved = &session.messages[k];
            let starts_with_tool_result = first_preserved
                .blocks
                .first()
                .is_some_and(|b| matches!(b, ContentBlock::ToolResult { .. }));
            if !starts_with_tool_result {
                break;
            }
            // Check the message just before the current boundary.
            let preceding = &session.messages[k - 1];
            let preceding_has_tool_use =
                preceding.blocks.iter().any(|b| matches!(b, ContentBlock::ToolUse { .. }));
            if preceding_has_tool_use {
                // Pair is intact — walk back one more to include the assistant turn.
                k = k.saturating_sub(1);
                break;
            }
            // Preceding message has no ToolUse but we have a ToolResult —
            // this is already an orphaned pair; walk back to try to fix it.
            k = k.saturating_sub(1);
        }
        k
    };
    // ── 消息重要性评分：智能选择保留关键消息 ──
    // 仅在可移除范围足够大（>10 条）时启用重要性评分。
    // 小范围直接全量移除，避免评分粒度过粗导致压缩效果不佳。
    let removable_len = keep_from.saturating_sub(compacted_prefix_len);
    let enable_importance = removable_len > 10;

    let actual_removed: Vec<ConversationMessage>;
    let preserved: Vec<ConversationMessage>;

    if enable_importance {
        // 使用重要性评分选择 top 70% 的高分消息作为额外保留
        let importance_indices =
            select_top_messages(&session.messages, (session.messages.len() as f64 * 0.7) as usize);
        let importance_set: HashSet<usize> = importance_indices.iter().copied().collect();

        // 从可移除范围中分离：高分消息保留，低分消息移除
        let removable_range = &session.messages[compacted_prefix_len..keep_from];
        let mut removed: Vec<ConversationMessage> = Vec::new();
        let mut kept: Vec<ConversationMessage> = Vec::new();

        for (offset, msg) in removable_range.iter().enumerate() {
            let global_idx = compacted_prefix_len + offset;
            if importance_set.contains(&global_idx) {
                kept.push(msg.clone());
            } else {
                removed.push(msg.clone());
            }
        }

        // 安全检查：当可移除范围非空但所有消息都被重要性评分保留时，
        // 强制移除评分最低的一条消息，确保压缩始终有实际效果
        if removed.is_empty() && !removable_range.is_empty() {
            let lowest_offset = removable_range
                .iter()
                .enumerate()
                .min_by_key(|(_, msg)| score_message(msg))
                .map(|(offset, _)| offset)
                .expect("removable_range is non-empty, min_by_key always returns Some");
            removed.clear();
            kept.clear();
            for (offset, msg) in removable_range.iter().enumerate() {
                if offset == lowest_offset {
                    removed.push(msg.clone());
                } else {
                    kept.push(msg.clone());
                }
            }
        }

        // 构建保留集合：重要性保留 + 尾部消息
        kept.extend(session.messages[keep_from..].iter().cloned());
        actual_removed = removed;
        preserved = kept;
    } else {
        // 小会话：保持原有行为，全量移除可移除范围
        actual_removed = session.messages[compacted_prefix_len..keep_from].to_vec();
        preserved = session.messages[keep_from..].to_vec();
    }
    let summary =
        merge_compact_summaries(existing_summary.as_deref(), &summarize_messages(&actual_removed));
    let formatted_summary = format_compact_summary(&summary);
    let continuation =
        get_compact_continuation_message(&summary, true, !preserved.is_empty(), provider);

    let mut compacted_messages = vec![ConversationMessage {
        role: MessageRole::System,
        blocks: vec![ContentBlock::Text { text: continuation }],
        usage: None,
    }];
    compacted_messages.extend(preserved);

    let mut compacted_session = session.clone();
    compacted_session.messages = compacted_messages;
    compacted_session.record_compaction(summary.clone(), actual_removed.len());

    CompactionResult {
        summary,
        formatted_summary,
        compacted_session,
        removed_message_count: actual_removed.len(),
    }
}

fn summarize_messages(messages: &[ConversationMessage]) -> String {
    let user_messages = messages.iter().filter(|message| message.role == MessageRole::User).count();
    let assistant_messages =
        messages.iter().filter(|message| message.role == MessageRole::Assistant).count();
    let tool_messages = messages.iter().filter(|message| message.role == MessageRole::Tool).count();

    let mut tool_names = messages
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolUse { name, .. } => Some(name.as_str()),
            ContentBlock::ToolResult { tool_name, .. } => Some(tool_name.as_str()),
            ContentBlock::Text { .. } => None,
        })
        .collect::<Vec<_>>();
    tool_names.sort_unstable();
    tool_names.dedup();

    let mut lines = vec![
        "<summary>".to_string(),
        "Conversation summary:".to_string(),
        format!(
            "- Scope: {} earlier messages compacted (user={}, assistant={}, tool={}).",
            messages.len(),
            user_messages,
            assistant_messages,
            tool_messages
        ),
    ];

    if !tool_names.is_empty() {
        lines.push(format!("- Tools mentioned: {}.", tool_names.join(", ")));
    }

    let recent_user_requests = collect_recent_role_summaries(messages, MessageRole::User, 3);
    if !recent_user_requests.is_empty() {
        lines.push("- Recent user requests:".to_string());
        lines.extend(recent_user_requests.into_iter().map(|request| format!("  - {request}")));
    }

    let pending_work = infer_pending_work(messages);
    if !pending_work.is_empty() {
        lines.push("- Pending work:".to_string());
        lines.extend(pending_work.into_iter().map(|item| format!("  - {item}")));
    }

    let key_files = collect_key_files(messages);
    if !key_files.is_empty() {
        lines.push(format!("- Key files referenced: {}.", key_files.join(", ")));
    }

    if let Some(current_work) = infer_current_work(messages) {
        lines.push(format!("- Current work: {current_work}"));
    }

    lines.push("- Key timeline:".to_string());
    for message in messages {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        let content = message.blocks.iter().map(summarize_block).collect::<Vec<_>>().join(" | ");
        lines.push(format!("  - {role}: {content}"));
    }
    lines.push("</summary>".to_string());
    lines.join("\n")
}

fn merge_compact_summaries(existing_summary: Option<&str>, new_summary: &str) -> String {
    let Some(existing_summary) = existing_summary else {
        return new_summary.to_string();
    };

    let previous_highlights = extract_summary_highlights(existing_summary);
    let new_formatted_summary = format_compact_summary(new_summary);
    let new_highlights = extract_summary_highlights(&new_formatted_summary);
    let new_timeline = extract_summary_timeline(&new_formatted_summary);

    let mut lines = vec!["<summary>".to_string(), "Conversation summary:".to_string()];

    if !previous_highlights.is_empty() {
        lines.push("- Previously compacted context:".to_string());
        lines.extend(previous_highlights.into_iter().map(|line| format!("  {line}")));
    }

    if !new_highlights.is_empty() {
        lines.push("- Newly compacted context:".to_string());
        lines.extend(new_highlights.into_iter().map(|line| format!("  {line}")));
    }

    if !new_timeline.is_empty() {
        lines.push("- Key timeline:".to_string());
        lines.extend(new_timeline.into_iter().map(|line| format!("  {line}")));
    }

    lines.push("</summary>".to_string());
    lines.join("\n")
}

fn summarize_block(block: &ContentBlock) -> String {
    let raw = match block {
        ContentBlock::Text { text } => text.clone(),
        ContentBlock::ToolUse { name, input, .. } => format!("tool_use {name}({input})"),
        ContentBlock::ToolResult { tool_name, output, is_error, .. } => {
            format!("tool_result {tool_name}: {}{output}", if *is_error { "error " } else { "" })
        },
    };
    // Truncate to 500 chars (up from 160) to preserve more useful context
    // such as file paths, error messages, and key results.
    truncate_summary(&raw, 500)
}

fn collect_recent_role_summaries(
    messages: &[ConversationMessage],
    role: MessageRole,
    limit: usize,
) -> Vec<String> {
    messages
        .iter()
        .filter(|message| message.role == role)
        .rev()
        .filter_map(|message| first_text_block(message))
        .take(limit)
        .map(|text| truncate_summary(text, 500))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn infer_pending_work(messages: &[ConversationMessage]) -> Vec<String> {
    messages
        .iter()
        .rev()
        .filter_map(first_text_block)
        .filter(|text| {
            let lowered = text.to_ascii_lowercase();
            lowered.contains("todo")
                || lowered.contains("next")
                || lowered.contains("pending")
                || lowered.contains("follow up")
                || lowered.contains("remaining")
        })
        .take(3)
        .map(|text| truncate_summary(text, 500))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn collect_key_files(messages: &[ConversationMessage]) -> Vec<String> {
    let mut files = messages
        .iter()
        .flat_map(|message| message.blocks.iter())
        .map(|block| match block {
            ContentBlock::Text { text } => text.as_str(),
            ContentBlock::ToolUse { input, .. } => input.as_str(),
            ContentBlock::ToolResult { output, .. } => output.as_str(),
        })
        .flat_map(extract_file_candidates)
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files.into_iter().take(8).collect()
}

fn infer_current_work(messages: &[ConversationMessage]) -> Option<String> {
    messages
        .iter()
        .rev()
        .filter_map(first_text_block)
        .find(|text| !text.trim().is_empty())
        .map(|text| truncate_summary(text, 200))
}

fn first_text_block(message: &ConversationMessage) -> Option<&str> {
    message.blocks.iter().find_map(|block| match block {
        ContentBlock::Text { text } if !text.trim().is_empty() => Some(text.as_str()),
        ContentBlock::ToolUse { .. }
        | ContentBlock::ToolResult { .. }
        | ContentBlock::Text { .. } => None,
    })
}

fn has_interesting_extension(candidate: &str) -> bool {
    std::path::Path::new(candidate)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["rs", "ts", "tsx", "js", "json", "md"]
                .iter()
                .any(|expected| extension.eq_ignore_ascii_case(expected))
        })
}

fn extract_file_candidates(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .filter_map(|token| {
            let candidate = token.trim_matches(|char: char| {
                matches!(char, ',' | '.' | ':' | ';' | ')' | '(' | '"' | '\'' | '`')
            });
            if candidate.contains('/') && has_interesting_extension(candidate) {
                Some(candidate.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn truncate_summary(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }
    let mut truncated = content.chars().take(max_chars).collect::<String>();
    truncated.push('…');
    truncated
}

fn extract_tag_block(content: &str, tag: &str) -> Option<String> {
    let start = format!("<{tag}>");
    let end = format!("</{tag}>");
    let start_index = content.find(&start)? + start.len();
    let end_index = content[start_index..].find(&end)? + start_index;
    Some(content[start_index..end_index].to_string())
}

fn strip_tag_block(content: &str, tag: &str) -> String {
    let start = format!("<{tag}>");
    let end = format!("</{tag}>");
    if let (Some(start_index), Some(end_index_rel)) = (content.find(&start), content.find(&end)) {
        let end_index = end_index_rel + end.len();
        let mut stripped = String::new();
        stripped.push_str(&content[..start_index]);
        stripped.push_str(&content[end_index..]);
        stripped
    } else {
        content.to_string()
    }
}

fn collapse_blank_lines(content: &str) -> String {
    let mut result = String::new();
    let mut last_blank = false;
    for line in content.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && last_blank {
            continue;
        }
        result.push_str(line);
        result.push('\n');
        last_blank = is_blank;
    }
    result
}

fn extract_existing_compacted_summary(
    message: &ConversationMessage,
    provider: &dyn PromptProvider,
) -> Option<String> {
    if message.role != MessageRole::System {
        return None;
    }

    let text = first_text_block(message)?;
    let summary = text.strip_prefix(compact_continuation_preamble(provider))?;
    let summary = summary
        .split_once(&("\n\n".to_string() + compact_recent_messages_note(provider)))
        .map_or(summary, |(value, _)| value);
    let summary = summary
        .split_once(&("\n".to_string() + compact_direct_resume_instruction(provider)))
        .map_or(summary, |(value, _)| value);
    Some(summary.trim().to_string())
}

fn extract_summary_highlights(summary: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut in_timeline = false;

    for line in format_compact_summary(summary).lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() || trimmed == "Summary:" || trimmed == "Conversation summary:" {
            continue;
        }
        if trimmed == "- Key timeline:" {
            in_timeline = true;
            continue;
        }
        if in_timeline {
            continue;
        }
        lines.push(trimmed.to_string());
    }

    lines
}

fn extract_summary_timeline(summary: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut in_timeline = false;

    for line in format_compact_summary(summary).lines() {
        let trimmed = line.trim_end();
        if trimmed == "- Key timeline:" {
            in_timeline = true;
            continue;
        }
        if !in_timeline {
            continue;
        }
        if trimmed.is_empty() {
            break;
        }
        lines.push(trimmed.to_string());
    }

    lines
}

/// compacted summaries.
#[must_use]
pub fn summarize_turn(messages: &[ConversationMessage]) -> String {
    let mut parts: Vec<String> = Vec::new();

    for message in messages {
        match message.role {
            MessageRole::User => {
                if let Some(text) = first_text_block(message) {
                    let short = text.chars().take(200).collect::<String>();
                    if !short.trim().is_empty() {
                        parts.push(format!("User: {}", short));
                    }
                }
            },
            MessageRole::Assistant => {
                let tool_uses: Vec<&str> = message
                    .blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::ToolUse { name, .. } => Some(name.as_str()),
                        _ => None,
                    })
                    .collect();
                if !tool_uses.is_empty() {
                    parts.push(format!("Used: {}", tool_uses.join(", ")));
                } else if let Some(text) = first_text_block(message) {
                    let short = text.chars().take(150).collect::<String>();
                    if !short.trim().is_empty() {
                        parts.push(short);
                    }
                }
            },
            MessageRole::Tool => {
                for block in &message.blocks {
                    if let ContentBlock::ToolResult { tool_name, output, is_error, .. } = block {
                        let status = if *is_error { "failed" } else { "ok" };
                        let output_short = output.chars().take(80).collect::<String>();
                        parts.push(format!("{tool_name}: {status} ({output_short})"));
                    }
                }
            },
            MessageRole::System => {},
        }
    }

    if parts.is_empty() {
        "(empty turn)".to_string()
    } else {
        parts.join(" | ")
    }
}

/// `position` is 0-indexed from the end (0 = most recent, N = furthest back).
#[must_use]
pub fn decay_weight(position: usize, base_weight: f64, decay_factor: f64) -> f64 {
    if decay_factor <= 0.0 || decay_factor >= 1.0 {
        return base_weight;
    }
    base_weight * decay_factor.powi(position as i32)
}

/// Returns `Some(index)` of the first message of the new task, or `None`
/// if no clear boundary is found.
#[must_use]
pub fn detect_task_boundary(messages: &[ConversationMessage]) -> Option<usize> {
    if messages.len() < 4 {
        return None;
    }

    let completion_markers = [
        "thanks",
        "thank you",
        "done",
        "looks good",
        "lgtm",
        "works",
        "working",
        "perfect",
        "great",
        "awesome",
        "completed",
        "resolved",
        "fixed",
    ];

    let new_task_markers = [
        "now let's",
        "next,",
        "can you also",
        "additionally",
        "separately",
        "another thing",
        "new task",
        "moving on",
        "also,",
        "one more",
        "by the way",
    ];

    // Search from newest backwards for completion markers followed by new task
    for i in (1..messages.len()).rev() {
        if messages[i].role == MessageRole::User
            && let Some(text) = first_text_block(&messages[i])
        {
            let lowered = text.to_lowercase();
            // Check if this is a new task request
            if new_task_markers.iter().any(|m| lowered.contains(m)) {
                return Some(i);
            }
        }
        // Check if the previous message pair signals completion
        if i > 0
            && messages[i - 1].role == MessageRole::User
            && let Some(text) = first_text_block(&messages[i - 1])
        {
            let lowered = text.to_lowercase();
            if completion_markers.iter().any(|m| lowered.contains(m)) {
                // Found completion — check if next message is a new task
                if i < messages.len()
                    && messages[i].role == MessageRole::User
                    && let Some(next_text) = first_text_block(&messages[i])
                {
                    let next_lower = next_text.to_lowercase();
                    if new_task_markers.iter().any(|m| next_lower.contains(m))
                        || !completion_markers.iter().any(|m| next_lower.contains(m))
                    {
                        return Some(i);
                    }
                }
            }
        }
    }

    None
}

/// Clean up messages before a detected task boundary.
///
/// When a task boundary is found at `boundary_index`, messages before that
/// index can be replaced with a compact summary, reducing context bloat
/// from completed tasks.
///
/// Returns the number of messages that should be compacted (pre-boundary count).
#[must_use]
pub fn cleanup_task_boundary(messages: &[ConversationMessage]) -> Option<usize> {
    detect_task_boundary(messages)
}
