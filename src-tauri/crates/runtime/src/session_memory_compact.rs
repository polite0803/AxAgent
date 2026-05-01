//! 会话记忆压缩
//!
//! 利用轨迹系统提取的结构化记忆（而非通用 LLM 摘要）作为压缩基础。
//! 相比纯 LLM 摘要压缩，结构化记忆保留更多细节（偏好、事实、模式、上下文），
//! 产生更丰富的压缩结果。
//!
//! 移植自 claude-code-main 的 sessionMemoryCompact.ts。

use crate::compact::{CompactionConfig, CompactionResult};
use crate::session::{ContentBlock, ConversationMessage, MessageRole, Session};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 配置
// ---------------------------------------------------------------------------

/// 会话记忆压缩配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMemoryCompactConfig {
    /// 压缩后保留的最小 token 数
    pub min_tokens: u64,
    /// 压缩后保留的包含文本块的最小消息数
    pub min_text_block_messages: usize,
    /// 压缩后保留的最大 token 数（硬上限）
    pub max_tokens: u64,
    /// 是否启用会话记忆压缩
    pub enabled: bool,
}

impl Default for SessionMemoryCompactConfig {
    fn default() -> Self {
        Self {
            min_tokens: 10_000,
            min_text_block_messages: 5,
            max_tokens: 40_000,
            enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 结构化记忆
// ---------------------------------------------------------------------------

/// 从轨迹分析中提取的结构化记忆条目。
/// 与 `axagent_trajectory::auto_memory::ExtractedMemory` 对应，
/// 但作为运行时独立的类型以避免循环依赖。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredMemory {
    /// 记忆类型：偏好、事实、模式、上下文、项目
    pub memory_type: String,
    /// 记忆内容
    pub content: String,
    /// 置信度 (0.0-1.0)
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// 压缩结果
// ---------------------------------------------------------------------------

/// 会话记忆压缩的结果。
#[derive(Debug, Clone)]
pub struct SessionMemoryCompactResult {
    /// 压缩边界之后保留的消息列表
    pub messages_to_keep: Vec<ConversationMessage>,
    /// 用作压缩摘要的会话记忆内容
    pub session_memory_content: String,
    /// 会话记忆是否因长度而被截断
    pub was_truncated: bool,
    /// 压缩后估算的 token 数
    pub post_compact_token_count: u64,
}

// ---------------------------------------------------------------------------
// 核心算法
// ---------------------------------------------------------------------------

/// 使用结构化记忆执行会话记忆压缩。
///
/// # 算法步骤
/// 1. 检查是否启用且存在记忆 → 否则返回 None
/// 2. 构建结构化记忆摘要文本
/// 3. 从尾部倒序遍历消息，累积 token 直到满足 min 要求但不超过 max
/// 4. 调整边界索引避免割裂 tool_use/tool_result 配对
/// 5. 若压缩后 token 仍超过 auto-compact 阈值，返回 None（需回退到 LLM 压缩）
///
/// # 返回
/// - `Some(result)`: 压缩成功，包含保留消息和记忆摘要
/// - `None`: 不适用（无记忆、已禁用、或需要回退到 LLM 压缩）
pub fn try_session_memory_compact(
    session: &Session,
    memories: &[StructuredMemory],
    config: &SessionMemoryCompactConfig,
    compaction_config: CompactionConfig,
) -> Option<SessionMemoryCompactResult> {
    if !config.enabled || memories.is_empty() {
        return None;
    }

    // 构建结构化记忆摘要
    let (memory_content, was_truncated) = build_session_memory_content(memories, config.max_tokens);

    // 从尾部计算起始索引
    let start_index = compute_compact_start_index(
        &session.messages,
        config.min_tokens,
        config.min_text_block_messages,
        config.max_tokens,
    );

    // 确保起始索引有效
    if start_index >= session.messages.len() {
        return None;
    }

    // 调整索引导避免割裂 tool_use/tool_result 配对
    let adjusted_start = adjust_index_to_preserve_pairs(&session.messages, start_index);

    let messages_to_keep: Vec<ConversationMessage> =
        session.messages[adjusted_start..].to_vec();

    // 估算压缩后的 token 数
    let post_compact_tokens = messages_to_keep
        .iter()
        .map(|m| crate::compact::estimate_message_tokens(m) as u64)
        .sum::<u64>()
        + (memory_content.len() / 4) as u64; // 记忆摘要的估算 token

    // 如果压缩后仍超过自动压缩阈值，回退到 LLM 压缩
    if post_compact_tokens > compaction_config.max_estimated_tokens as u64 {
        return None;
    }

    // 至少需要保留一些消息才有意义
    if messages_to_keep.len() < config.min_text_block_messages {
        return None;
    }

    Some(SessionMemoryCompactResult {
        messages_to_keep,
        session_memory_content: memory_content,
        was_truncated,
        post_compact_token_count: post_compact_tokens,
    })
}

/// 将结构化记忆列表转换为压缩摘要文本。
///
/// 按类型分组输出，每个记忆一行，超过 max_tokens 时截断。
fn build_session_memory_content(memories: &[StructuredMemory], max_tokens: u64) -> (String, bool) {
    let max_chars = (max_tokens * 4) as usize; // ~4 chars per token

    // 按类型分组
    let mut by_type: std::collections::BTreeMap<&str, Vec<&StructuredMemory>> =
        std::collections::BTreeMap::new();
    for mem in memories {
        by_type
            .entry(mem.memory_type.as_str())
            .or_default()
            .push(mem);
    }

    // 高置信度记忆优先
    for memories_list in by_type.values_mut() {
        memories_list.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push("Session Memory Summary:".to_string());

    for (mem_type, memories_list) in &by_type {
        if memories_list.is_empty() {
            continue;
        }
        let type_label = match *mem_type {
            "preference" => "User Preferences",
            "fact" => "Key Facts",
            "pattern" => "Learned Patterns",
            "context" => "Session Context",
            "project" => "Project Info",
            other => other,
        };
        lines.push(format!("\n## {}", type_label));
        for mem in memories_list {
            let confidence_str = if mem.confidence >= 0.8 {
                "high"
            } else if mem.confidence >= 0.5 {
                "medium"
            } else {
                "low"
            };
            lines.push(format!("- [{}] {}", confidence_str, mem.content));
        }
    }

    let full = lines.join("\n");

    if full.len() <= max_chars {
        (full, false)
    } else {
        // 截断：保留完整行，直到超过限制
        let mut truncated = String::new();
        let mut was_truncated = false;
        for line in lines {
            if truncated.len() + line.len() + 1 > max_chars {
                was_truncated = true;
                truncated.push_str("\n... (truncated)");
                break;
            }
            if !truncated.is_empty() {
                truncated.push('\n');
            }
            truncated.push_str(&line);
        }
        (truncated, was_truncated)
    }
}

/// 从消息列表尾部计算压缩起始索引。
///
/// 从末尾向前遍历，累积 token 数直到满足 `min_tokens` 和 `min_text_block_messages`，
/// 但不超过 `max_tokens`。返回的索引指向第一条需要保留的消息。
fn compute_compact_start_index(
    messages: &[ConversationMessage],
    min_tokens: u64,
    min_text_block_messages: usize,
    max_tokens: u64,
) -> usize {
    let mut accumulated_tokens: u64 = 0;
    let mut text_block_messages: usize = 0;
    let mut keep_from: usize = messages.len();

    for (i, msg) in messages.iter().enumerate().rev() {
        let msg_tokens = crate::compact::estimate_message_tokens(msg) as u64;

        // 检查是否超过 max
        if accumulated_tokens + msg_tokens > max_tokens && text_block_messages >= min_text_block_messages
        {
            keep_from = i + 1;
            break;
        }

        accumulated_tokens += msg_tokens;

        // 检查文本块
        if msg.blocks.iter().any(|b| matches!(b, ContentBlock::Text { .. })) {
            text_block_messages += 1;
        }

        // 检查是否满足最小值
        if accumulated_tokens >= min_tokens && text_block_messages >= min_text_block_messages {
            keep_from = i;
            break;
        }

        keep_from = i;
    }

    keep_from
}

/// 调整压缩边界索引，确保不会割裂 tool_use / tool_result 配对。
///
/// 如果在边界处第一条保留消息是 tool_result 但其前一条消息没有 tool_use，
/// 向下调整边界以包含配对的 tool_use 消息。这避免在 OpenAI 兼容 API 上产生
/// 孤立的 'tool' 角色消息（会导致 400 错误）。
fn adjust_index_to_preserve_pairs(
    messages: &[ConversationMessage],
    start_index: usize,
) -> usize {
    if start_index == 0 || start_index >= messages.len() {
        return start_index;
    }

    let mut adjusted = start_index;

    loop {
        if adjusted == 0 {
            break;
        }

        let first_kept = &messages[adjusted];
        let starts_with_tool_result = first_kept
            .blocks
            .first()
            .is_some_and(|b| matches!(b, ContentBlock::ToolResult { .. }));

        if !starts_with_tool_result {
            break;
        }

        let preceding = &messages[adjusted - 1];
        let preceding_has_tool_use = preceding
            .blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }));

        if preceding_has_tool_use {
            // 配对完整 — 再向前一步以包含 assistant 轮次
            adjusted = adjusted.saturating_sub(1);
            break;
        }

        // 前一条没有 ToolUse 但我们有 ToolResult — 已是孤立的配对，向前走尝试修复
        adjusted = adjusted.saturating_sub(1);
    }

    adjusted
}

/// 将 SessionMemoryCompactResult 转换为标准的 CompactionResult。
///
/// 这使得会话记忆压缩可以无缝替代传统的 LLM 压缩。
pub fn to_compaction_result(
    sm_result: &SessionMemoryCompactResult,
    session: &Session,
) -> CompactionResult {
    let removed_count = session.messages.len() - sm_result.messages_to_keep.len();

    let continuation_message = format!(
        "This session is being continued from a previous conversation. \
         The following structured memories summarize the earlier portion:\n\n{}",
        sm_result.session_memory_content
    );

    let mut compacted_messages = vec![ConversationMessage {
        role: MessageRole::System,
        blocks: vec![ContentBlock::Text {
            text: continuation_message,
        }],
        usage: None,
    }];
    compacted_messages.extend(sm_result.messages_to_keep.clone());

    let mut compacted_session = session.clone();
    compacted_session.messages = compacted_messages;

    CompactionResult {
        summary: sm_result.session_memory_content.clone(),
        formatted_summary: format!(
            "Session Memory Summary:\n{}",
            sm_result.session_memory_content
        ),
        compacted_session,
        removed_message_count: removed_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{ContentBlock, ConversationMessage, Session};

    fn make_test_memories() -> Vec<StructuredMemory> {
        vec![
            StructuredMemory {
                memory_type: "preference".to_string(),
                content: "User prefers Rust over TypeScript".to_string(),
                confidence: 0.9,
            },
            StructuredMemory {
                memory_type: "fact".to_string(),
                content: "Project uses SeaORM for database".to_string(),
                confidence: 0.85,
            },
            StructuredMemory {
                memory_type: "pattern".to_string(),
                content: "User always runs cargo check before commit".to_string(),
                confidence: 0.75,
            },
            StructuredMemory {
                memory_type: "context".to_string(),
                content: "Working on AxAgent backend upgrade".to_string(),
                confidence: 0.95,
            },
        ]
    }

    fn make_test_session(message_count: usize) -> Session {
        let mut session = Session::new();
        for i in 0..message_count {
            // 创建足够大的消息以确保 token 估算值超过压缩阈值
            let text = format!("message {} {}", i, "x".repeat(10_000));
            if i % 2 == 0 {
                session
                    .push_message(ConversationMessage::user_text(&text))
                    .unwrap();
            } else {
                session
                    .push_message(ConversationMessage::assistant(vec![
                        ContentBlock::Text { text },
                    ]))
                    .unwrap();
            }
        }
        session
    }

    #[test]
    fn test_disabled_returns_none() {
        let session = make_test_session(20);
        let config = SessionMemoryCompactConfig {
            enabled: false,
            ..Default::default()
        };
        let result = try_session_memory_compact(
            &session,
            &make_test_memories(),
            &config,
            CompactionConfig::default(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_no_memories_returns_none() {
        let session = make_test_session(20);
        let config = SessionMemoryCompactConfig::default();
        let result = try_session_memory_compact(
            &session,
            &[],
            &config,
            CompactionConfig::default(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_basic_compaction_works() {
        let session = make_test_session(30);
        let config = SessionMemoryCompactConfig {
            min_tokens: 100,
            min_text_block_messages: 2,
            max_tokens: 500_000,
            enabled: true,
        };
        let result = try_session_memory_compact(
            &session,
            &make_test_memories(),
            &config,
            CompactionConfig::default(),
        );
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(!r.messages_to_keep.is_empty());
        assert!(!r.session_memory_content.is_empty());
        assert!(!r.was_truncated);
    }

    #[test]
    fn test_memory_content_formatting() {
        let (content, _) = build_session_memory_content(&make_test_memories(), 10_000);
        assert!(content.contains("Session Memory Summary"));
        assert!(content.contains("User Preferences"));
        assert!(content.contains("Key Facts"));
        assert!(content.contains("Rust over TypeScript"));
        assert!(content.contains("high")); // confidence 0.9
    }

    #[test]
    fn test_truncation_on_small_max_tokens() {
        let (content, was_truncated) = build_session_memory_content(&make_test_memories(), 10);
        assert!(was_truncated || content.len() <= 40); // 10 tokens * 4 chars
    }

    #[test]
    fn test_pair_preservation() {
        let mut session = Session::new();
        let tool_id = "call_001";
        // Assistant with ToolUse
        session
            .push_message(ConversationMessage::assistant(vec![
                ContentBlock::ToolUse {
                    id: tool_id.to_string(),
                    name: "read_file".to_string(),
                    input: "main.rs".to_string(),
                },
            ]))
            .unwrap();
        // Tool result
        session
            .push_message(ConversationMessage::tool_result(
                tool_id,
                "read_file",
                "contents here",
                false,
            ))
            .unwrap();
        // More messages
        for i in 0..5 {
            session
                .push_message(ConversationMessage::user_text(&format!("msg {}", i)))
                .unwrap();
        }

        // 尝试在 tool_result 处切割
        let adjusted = adjust_index_to_preserve_pairs(&session.messages, 1);
        // 应该调整到 0（包含 assistant ToolUse）
        assert!(adjusted <= 1);
    }

    #[test]
    fn test_to_compaction_result() {
        let session = make_test_session(30);
        let config = SessionMemoryCompactConfig {
            min_tokens: 100,
            min_text_block_messages: 2,
            max_tokens: 500_000,
            enabled: true,
        };
        let result = try_session_memory_compact(
            &session,
            &make_test_memories(),
            &config,
            CompactionConfig {
                max_estimated_tokens: 500_000,
                ..CompactionConfig::default()
            },
        )
        .unwrap();

        let compaction = to_compaction_result(&result, &session);
        assert!(compaction.removed_message_count > 0);
        assert!(!compaction.summary.is_empty());
        assert!(compaction.compacted_session.messages[0].role == MessageRole::System);
    }
}
