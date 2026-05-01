use crate::session::{ContentBlock, ConversationMessage, MessageRole, Session};

const COMPACT_CONTINUATION_PREAMBLE: &str =
    "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\n";
const COMPACT_RECENT_MESSAGES_NOTE: &str = "Recent messages are preserved verbatim.";
const COMPACT_DIRECT_RESUME_INSTRUCTION: &str = "Continue the conversation from where it left off without asking the user any further questions. Resume directly — do not acknowledge the summary, do not recap what was happening, and do not preface with continuation text.";

/// Thresholds controlling when and how a session is compacted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionConfig {
    pub preserve_recent_messages: usize,
    pub max_estimated_tokens: usize,
    /// Whether to extract per-turn summaries during compaction.
    pub enable_turn_summaries: bool,
    /// Whether to apply distance-based relevance decay when scoring messages.
    pub enable_distance_decay: bool,
    /// Whether to automatically clean up context after a task boundary is detected.
    pub enable_task_boundary_cleanup: bool,
    /// Maximum age (in turns from the end) before messages are aggressively pruned.
    pub max_turn_age: Option<usize>,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            // Preserve 12 recent messages (up from 4) to keep enough context
            // for multi-step agent workflows. 4 messages could lose critical
            // decisions or file paths from just one tool-call round.
            preserve_recent_messages: 12,
            // Raise threshold to 80K tokens (from 10K). Modern LLMs have
            // 128K-200K context windows; 10K was too aggressive and caused
            // premature compaction that lost important context.
            max_estimated_tokens: 80_000,
            enable_turn_summaries: true,
            enable_distance_decay: true,
            enable_task_boundary_cleanup: true,
            max_turn_age: Some(50),
        }
    }
}

/// Result of compacting a session into a summary plus preserved tail messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionResult {
    pub summary: String,
    pub formatted_summary: String,
    pub compacted_session: Session,
    pub removed_message_count: usize,
}

/// Roughly estimates the token footprint of the current session transcript.
#[must_use]
pub fn estimate_session_tokens(session: &Session) -> usize {
    session.messages.iter().map(estimate_message_tokens).sum()
}

/// Returns `true` when the session exceeds the configured compaction budget.
#[must_use]
pub fn should_compact(session: &Session, config: CompactionConfig) -> bool {
    let start = compacted_summary_prefix_len(session);
    let compactable = &session.messages[start..];

    compactable.len() > config.preserve_recent_messages
        && compactable
            .iter()
            .map(estimate_message_tokens)
            .sum::<usize>()
            >= config.max_estimated_tokens
}

/// 使用多层阈值系统判断是否需要压缩（增强版）。
///
/// 与 `should_compact` 不同，此函数：
/// - 考虑有效上下文窗口大小
/// - 使用四层阈值（warning / auto_compact / error / blocking_limit）
/// - 返回详细的阈值状态而非简单布尔值
///
/// # 参数
/// - `session`: 当前会话
/// - `effective_window`: 模型的有效上下文窗口 token 数
#[must_use]
pub fn evaluate_compact_threshold(
    session: &Session,
    effective_window: u64,
) -> crate::compact_thresholds::CompactThresholdState {
    crate::compact_thresholds::CompactThresholdState::compute(session, effective_window)
}

/// 获取建议的压缩配置，根据当前阈值状态自动调整激进程度。
///
/// 越接近上下文窗口限制，配置越激进（保留更少的最近消息）。
#[must_use]
pub fn adaptive_compaction_config(session: &Session, effective_window: u64) -> CompactionConfig {
    crate::compact_thresholds::recommended_compaction_config(session, effective_window)
}

/// 智能压缩：优先尝试会话记忆压缩，失败时回退到传统 LLM 压缩。
///
/// 此函数将 session_memory_compact 和传统的 compact_session 串联起来：
/// 1. 如果有结构化记忆可用，先尝试 session_memory_compact
/// 2. 如果记忆压缩成功，返回其结果
/// 3. 如果记忆压缩不适用或失败，回退到传统 compact_session
///
/// # 参数
/// - `session`: 要压缩的会话
/// - `config`: 基础压缩配置
/// - `memories`: 从轨迹系统提取的结构化记忆（可为空）
#[must_use]
pub fn smart_compact(
    session: &Session,
    config: CompactionConfig,
    memories: &[crate::session_memory_compact::StructuredMemory],
) -> CompactionResult {
    // 尝试会话记忆压缩
    let sm_config = crate::session_memory_compact::SessionMemoryCompactConfig::default();
    if let Some(sm_result) = crate::session_memory_compact::try_session_memory_compact(
        session, memories, &sm_config, config,
    ) {
        return crate::session_memory_compact::to_compaction_result(&sm_result, session);
    }

    // 回退到传统 LLM 压缩
    compact_session(session, config)
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
) -> String {
    let mut base = format!(
        "{COMPACT_CONTINUATION_PREAMBLE}{}",
        format_compact_summary(summary)
    );

    if recent_messages_preserved {
        base.push_str("\n\n");
        base.push_str(COMPACT_RECENT_MESSAGES_NOTE);
    }

    if suppress_follow_up_questions {
        base.push('\n');
        base.push_str(COMPACT_DIRECT_RESUME_INSTRUCTION);
    }

    base
}

/// Compacts a session by summarizing older messages and preserving the recent tail.
#[must_use]
pub fn compact_session(session: &Session, config: CompactionConfig) -> CompactionResult {
    if !should_compact(session, config) {
        return CompactionResult {
            summary: String::new(),
            formatted_summary: String::new(),
            compacted_session: session.clone(),
            removed_message_count: 0,
        };
    }

    let existing_summary = session
        .messages
        .first()
        .and_then(extract_existing_compacted_summary);
    let compacted_prefix_len = usize::from(existing_summary.is_some());
    let raw_keep_from = session
        .messages
        .len()
        .saturating_sub(config.preserve_recent_messages);
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
            let preceding_has_tool_use = preceding
                .blocks
                .iter()
                .any(|b| matches!(b, ContentBlock::ToolUse { .. }));
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
    let removed = &session.messages[compacted_prefix_len..keep_from];
    let preserved = session.messages[keep_from..].to_vec();
    let summary =
        merge_compact_summaries(existing_summary.as_deref(), &summarize_messages(removed));
    let formatted_summary = format_compact_summary(&summary);
    let continuation = get_compact_continuation_message(&summary, true, !preserved.is_empty());

    let mut compacted_messages = vec![ConversationMessage {
        role: MessageRole::System,
        blocks: vec![ContentBlock::Text { text: continuation }],
        usage: None,
    }];
    compacted_messages.extend(preserved);

    let mut compacted_session = session.clone();
    compacted_session.messages = compacted_messages;
    compacted_session.record_compaction(summary.clone(), removed.len());

    CompactionResult {
        summary,
        formatted_summary,
        compacted_session,
        removed_message_count: removed.len(),
    }
}

fn compacted_summary_prefix_len(session: &Session) -> usize {
    usize::from(
        session
            .messages
            .first()
            .and_then(extract_existing_compacted_summary)
            .is_some(),
    )
}

fn summarize_messages(messages: &[ConversationMessage]) -> String {
    let user_messages = messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .count();
    let assistant_messages = messages
        .iter()
        .filter(|message| message.role == MessageRole::Assistant)
        .count();
    let tool_messages = messages
        .iter()
        .filter(|message| message.role == MessageRole::Tool)
        .count();

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
        lines.extend(
            recent_user_requests
                .into_iter()
                .map(|request| format!("  - {request}")),
        );
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
        let content = message
            .blocks
            .iter()
            .map(summarize_block)
            .collect::<Vec<_>>()
            .join(" | ");
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
        lines.extend(
            previous_highlights
                .into_iter()
                .map(|line| format!("  {line}")),
        );
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
        ContentBlock::ToolResult {
            tool_name,
            output,
            is_error,
            ..
        } => format!(
            "tool_result {tool_name}: {}{output}",
            if *is_error { "error " } else { "" }
        ),
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

/// Estimate tokens for a single conversation message.
///
/// Uses character-based heuristics: ~4 chars per token for text,
/// with per-block computation for structured content.
#[must_use]
pub fn estimate_message_tokens(message: &ConversationMessage) -> usize {
    message
        .blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => text.len() / 4 + 1,
            ContentBlock::ToolUse { name, input, .. } => (name.len() + input.len()) / 4 + 1,
            ContentBlock::ToolResult {
                tool_name, output, ..
            } => (tool_name.len() + output.len()) / 4 + 1,
        })
        .sum()
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

fn extract_existing_compacted_summary(message: &ConversationMessage) -> Option<String> {
    if message.role != MessageRole::System {
        return None;
    }

    let text = first_text_block(message)?;
    let summary = text.strip_prefix(COMPACT_CONTINUATION_PREAMBLE)?;
    let summary = summary
        .split_once(&format!("\n\n{COMPACT_RECENT_MESSAGES_NOTE}"))
        .map_or(summary, |(value, _)| value);
    let summary = summary
        .split_once(&format!("\n{COMPACT_DIRECT_RESUME_INSTRUCTION}"))
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

/// Extract a concise summary of a single conversation turn.
///
/// Scans the message blocks for user intent, tool usage, and results,
/// producing a short one-line description suitable for inclusion in
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
            }
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
            }
            MessageRole::Tool => {
                for block in &message.blocks {
                    if let ContentBlock::ToolResult {
                        tool_name,
                        output,
                        is_error,
                        ..
                    } = block
                    {
                        let status = if *is_error { "failed" } else { "ok" };
                        let output_short = output.chars().take(80).collect::<String>();
                        parts.push(format!("{tool_name}: {status} ({output_short})"));
                    }
                }
            }
            MessageRole::System => {}
        }
    }

    if parts.is_empty() {
        "(empty turn)".to_string()
    } else {
        parts.join(" | ")
    }
}

/// Compute a relevance decay weight based on distance from the current turn.
///
/// Messages closer to the current turn receive higher weight.
/// Uses exponential decay: `weight = base_weight * decay_factor^(distance)`
///
/// `position` is 0-indexed from the end (0 = most recent, N = furthest back).
#[must_use]
pub fn decay_weight(position: usize, base_weight: f64, decay_factor: f64) -> f64 {
    if decay_factor <= 0.0 || decay_factor >= 1.0 {
        return base_weight;
    }
    base_weight * decay_factor.powi(position as i32)
}

/// Detect task boundaries in a sequence of messages and return the index
/// after which earlier messages can be safely cleaned up.
///
/// A task boundary is detected when:
/// - A user message signals task completion (e.g. "thanks", "done", "looks good")
/// - A significant gap in conversation context is detected
/// - A new, distinct task request begins
///
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
        if messages[i].role == MessageRole::User {
            if let Some(text) = first_text_block(&messages[i]) {
                let lowered = text.to_lowercase();
                // Check if this is a new task request
                if new_task_markers.iter().any(|m| lowered.contains(m)) {
                    return Some(i);
                }
            }
        }
        // Check if the previous message pair signals completion
        if i > 0 && messages[i - 1].role == MessageRole::User {
            if let Some(text) = first_text_block(&messages[i - 1]) {
                let lowered = text.to_lowercase();
                if completion_markers.iter().any(|m| lowered.contains(m)) {
                    // Found completion — check if next message is a new task
                    if i < messages.len() && messages[i].role == MessageRole::User {
                        if let Some(next_text) = first_text_block(&messages[i]) {
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
    detect_task_boundary(messages).map(|boundary| boundary)
}

#[cfg(test)]
mod tests {
    use super::{
        collect_key_files, compact_session, format_compact_summary,
        get_compact_continuation_message, infer_pending_work, should_compact, CompactionConfig,
    };
    use crate::session::{ContentBlock, ConversationMessage, MessageRole, Session};

    #[test]
    fn formats_compact_summary_like_upstream() {
        let summary = "<analysis>scratch</analysis>\n<summary>Kept work</summary>";
        assert_eq!(format_compact_summary(summary), "Summary:\nKept work");
    }

    #[test]
    fn leaves_small_sessions_unchanged() {
        let mut session = Session::new();
        session.messages = vec![ConversationMessage::user_text("hello")];

        let result = compact_session(&session, CompactionConfig::default());
        assert_eq!(result.removed_message_count, 0);
        assert_eq!(result.compacted_session, session);
        assert!(result.summary.is_empty());
        assert!(result.formatted_summary.is_empty());
    }

    #[test]
    fn compacts_older_messages_into_a_system_summary() {
        let mut session = Session::new();
        session.messages = vec![
            ConversationMessage::user_text("one ".repeat(200)),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "two ".repeat(200),
            }]),
            ConversationMessage::tool_result("1", "bash", "ok ".repeat(200), false),
            ConversationMessage {
                role: MessageRole::Assistant,
                blocks: vec![ContentBlock::Text {
                    text: "recent".to_string(),
                }],
                usage: None,
            },
        ];

        let result = compact_session(
            &session,
            CompactionConfig {
                preserve_recent_messages: 2,
                max_estimated_tokens: 1,
                ..Default::default()
            },
        );
        // one extra message to avoid an orphaned tool result at the boundary.
        // messages[1] (assistant) must be kept along with messages[2] (tool result).
        assert!(
            result.removed_message_count <= 2,
            "expected at most 2 removed, got {}",
            result.removed_message_count
        );
        assert_eq!(
            result.compacted_session.messages[0].role,
            MessageRole::System
        );
        assert!(matches!(
            &result.compacted_session.messages[0].blocks[0],
            ContentBlock::Text { text } if text.contains("Summary:")
        ));
        assert!(result.formatted_summary.contains("Scope:"));
        assert!(result.formatted_summary.contains("Key timeline:"));
        assert!(should_compact(
            &session,
            CompactionConfig {
                preserve_recent_messages: 2,
                max_estimated_tokens: 1,
                ..Default::default()
            },
        ));
        // Note: with the tool-use/tool-result boundary guard the compacted session
        // may preserve one extra message at the boundary, so token reduction is
        // not guaranteed for small sessions. The invariant that matters is that
        // the removed_message_count is non-zero (something was compacted).
        assert!(
            result.removed_message_count > 0,
            "compaction must remove at least one message"
        );
    }

    #[test]
    fn keeps_previous_compacted_context_when_compacting_again() {
        let mut initial_session = Session::new();
        initial_session.messages = vec![
            ConversationMessage::user_text("Investigate rust/crates/runtime/src/compact.rs"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "I will inspect the compact flow.".to_string(),
            }]),
            ConversationMessage::user_text("Also update rust/crates/runtime/src/conversation.rs"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Next: preserve prior summary context during auto compact.".to_string(),
            }]),
        ];
        let config = CompactionConfig {
            preserve_recent_messages: 2,
            max_estimated_tokens: 1,
            ..Default::default()
        };

        let first = compact_session(&initial_session, config);
        let mut follow_up_messages = first.compacted_session.messages.clone();
        follow_up_messages.extend([
            ConversationMessage::user_text("Please add regression tests for compaction."),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Working on regression coverage now.".to_string(),
            }]),
        ]);

        let mut second_session = Session::new();
        second_session.messages = follow_up_messages;
        let second = compact_session(&second_session, config);

        assert!(second
            .formatted_summary
            .contains("Previously compacted context:"));
        assert!(second
            .formatted_summary
            .contains("Scope: 2 earlier messages compacted"));
        assert!(second
            .formatted_summary
            .contains("Newly compacted context:"));
        assert!(second
            .formatted_summary
            .contains("Also update rust/crates/runtime/src/conversation.rs"));
        assert!(matches!(
            &second.compacted_session.messages[0].blocks[0],
            ContentBlock::Text { text }
                if text.contains("Previously compacted context:")
                    && text.contains("Newly compacted context:")
        ));
        assert!(matches!(
            &second.compacted_session.messages[1].blocks[0],
            ContentBlock::Text { text } if text.contains("Please add regression tests for compaction.")
        ));
    }

    #[test]
    fn ignores_existing_compacted_summary_when_deciding_to_recompact() {
        let summary = "<summary>Conversation summary:\n- Scope: earlier work preserved.\n- Key timeline:\n  - user: large preserved context\n</summary>";
        let mut session = Session::new();
        session.messages = vec![
            ConversationMessage {
                role: MessageRole::System,
                blocks: vec![ContentBlock::Text {
                    text: get_compact_continuation_message(summary, true, true),
                }],
                usage: None,
            },
            ConversationMessage::user_text("tiny"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "recent".to_string(),
            }]),
        ];

        assert!(!should_compact(
            &session,
            CompactionConfig {
                preserve_recent_messages: 2,
                max_estimated_tokens: 1,
                ..Default::default()
            }
        ));
    }

    #[test]
    fn truncates_long_blocks_in_summary() {
        let summary = super::summarize_block(&ContentBlock::Text {
            text: "x".repeat(600),
        });
        assert!(summary.ends_with('…'));
        assert!(summary.chars().count() <= 501);
    }

    #[test]
    fn extracts_key_files_from_message_content() {
        let files = collect_key_files(&[ConversationMessage::user_text(
            "Update rust/crates/runtime/src/compact.rs and rust/crates/rusty-claude-cli/src/main.rs next.",
        )]);
        assert!(files.contains(&"rust/crates/runtime/src/compact.rs".to_string()));
        assert!(files.contains(&"rust/crates/rusty-claude-cli/src/main.rs".to_string()));
    }

    /// Regression: compaction must not split an assistant(ToolUse) /
    /// user(ToolResult) pair at the boundary. An orphaned tool-result message
    /// without the preceding assistant `tool_calls` causes a 400 on the
    /// OpenAI-compat path (gaebal-gajae repro 2026-04-09).
    #[test]
    fn compaction_does_not_split_tool_use_tool_result_pair() {
        use crate::session::{ContentBlock, Session};

        let tool_id = "call_abc";
        let mut session = Session::default();
        // Turn 1: user prompt
        session
            .push_message(ConversationMessage::user_text("Search for files"))
            .unwrap();
        // Turn 2: assistant calls a tool
        session
            .push_message(ConversationMessage::assistant(vec![
                ContentBlock::ToolUse {
                    id: tool_id.to_string(),
                    name: "search".to_string(),
                    input: "{\"q\":\"*.rs\"}".to_string(),
                },
            ]))
            .unwrap();
        // Turn 3: tool result
        session
            .push_message(ConversationMessage::tool_result(
                tool_id,
                "search",
                "found 5 files",
                false,
            ))
            .unwrap();
        // Turn 4: assistant final response
        session
            .push_message(ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Done.".to_string(),
            }]))
            .unwrap();

        // Compact preserving only 1 recent message — without the fix this
        // would cut the boundary so that the tool result (turn 3) is first,
        // without its preceding assistant tool_calls (turn 2).
        let config = CompactionConfig {
            preserve_recent_messages: 1,
            ..CompactionConfig::default()
        };
        let result = compact_session(&session, config);
        // After compaction, no two consecutive messages should have the pattern
        // tool_result immediately following a non-assistant message (i.e. an
        // orphaned tool result without a preceding assistant ToolUse).
        let messages = &result.compacted_session.messages;
        for i in 1..messages.len() {
            let curr_is_tool_result = messages[i]
                .blocks
                .first()
                .is_some_and(|b| matches!(b, ContentBlock::ToolResult { .. }));
            if curr_is_tool_result {
                let prev_has_tool_use = messages[i - 1]
                    .blocks
                    .iter()
                    .any(|b| matches!(b, ContentBlock::ToolUse { .. }));
                assert!(
                    prev_has_tool_use,
                    "message[{}] is a ToolResult but message[{}] has no ToolUse: {:?}",
                    i,
                    i - 1,
                    &messages[i - 1].blocks
                );
            }
        }
    }

    #[test]
    fn infers_pending_work_from_recent_messages() {
        let pending = infer_pending_work(&[
            ConversationMessage::user_text("done"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Next: update tests and follow up on remaining CLI polish.".to_string(),
            }]),
        ]);
        assert_eq!(pending.len(), 1);
        assert!(pending[0].contains("Next: update tests"));
    }

    #[test]
    fn test_summarize_turn_simple() {
        let messages = vec![
            ConversationMessage::user_text("Fix the bug in main.rs"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "I'll fix that.".to_string(),
            }]),
        ];
        let summary = super::summarize_turn(&messages);
        assert!(summary.contains("Fix the bug"));
        assert!(summary.contains("I'll fix that"));
    }

    #[test]
    fn test_summarize_turn_with_tools() {
        let messages = vec![
            ConversationMessage::user_text("Read the file"),
            ConversationMessage {
                role: MessageRole::Assistant,
                blocks: vec![ContentBlock::ToolUse {
                    id: "1".to_string(),
                    name: "read_file".to_string(),
                    input: "main.rs".to_string(),
                }],
                usage: None,
            },
            ConversationMessage::tool_result("1", "read_file", "file contents", false),
        ];
        let summary = super::summarize_turn(&messages);
        assert!(summary.contains("Used: read_file"));
        assert!(summary.contains("read_file: ok"));
    }

    #[test]
    fn test_decay_weight_values() {
        let w0 = super::decay_weight(0, 1.0, 0.9);
        let w1 = super::decay_weight(1, 1.0, 0.9);
        let w5 = super::decay_weight(5, 1.0, 0.9);
        assert!((w0 - 1.0).abs() < 0.001);
        assert!((w1 - 0.9).abs() < 0.001);
        assert!(w5 < w1);
        assert!(w5 > 0.4);
    }

    #[test]
    fn test_decay_weight_invalid_factor() {
        let w = super::decay_weight(3, 1.0, 1.5);
        assert!((w - 1.0).abs() < 0.001);
        let w2 = super::decay_weight(3, 1.0, 0.0);
        assert!((w2 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_detect_task_boundary_finds_transition() {
        let messages = vec![
            ConversationMessage::user_text("Fix the bug"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Done".to_string(),
            }]),
            ConversationMessage::user_text("Thanks, looks good!"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "You're welcome".to_string(),
            }]),
            ConversationMessage::user_text("Now let's add a new feature"),
        ];
        let boundary = super::detect_task_boundary(&messages);
        assert_eq!(boundary, Some(4));
    }

    #[test]
    fn test_detect_task_boundary_short_conversation() {
        let messages = vec![
            ConversationMessage::user_text("Hi"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }]),
        ];
        let boundary = super::detect_task_boundary(&messages);
        assert_eq!(boundary, None);
    }

    #[test]
    fn test_cleanup_task_boundary_returns_count() {
        let messages = vec![
            ConversationMessage::user_text("do task A"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "done A".to_string(),
            }]),
            ConversationMessage::user_text("thanks, looks good"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "welcome".to_string(),
            }]),
            ConversationMessage::user_text("Now let's do task B"),
        ];
        let cleanup = super::cleanup_task_boundary(&messages);
        assert_eq!(cleanup, Some(4));
    }
}
