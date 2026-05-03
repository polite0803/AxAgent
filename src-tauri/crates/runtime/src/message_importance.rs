//! 消息重要性评分 — 在上下文压缩时评估每条消息的保留优先级
//!
//! 核心思路：
//! - 用户消息权重最高（用户意图不可丢失）
//! - 包含工具调用的消息次重要（记录了实际操作）
//! - 包含错误的工具结果降低优先级（可丢弃的错误重试）
//! - 文本长度影响信息密度评估

use crate::session::{ContentBlock, ConversationMessage, MessageRole};

/// 消息重要性分数 (0-100)
///
/// 评分规则：
/// - 基础分 50
/// - 用户消息 +20（用户意图最关键）
/// - 包含工具调用 +15（记录实际操作步骤）
/// - 工具结果包含错误 -10（错误重试信息可丢弃）
/// - 文本长度 > 500 字符 +10（可能包含重要上下文）
/// - 文本长度 < 20 字符 -5（信息量过低）
#[must_use]
pub fn score_message(msg: &ConversationMessage) -> u32 {
    let mut score = 50; // 基础分

    // 用户消息更重要
    if msg.role == MessageRole::User {
        score += 20;
    }

    // 包含工具调用的消息更重要
    let has_tool_use = msg
        .blocks
        .iter()
        .any(|b| matches!(b, ContentBlock::ToolUse { .. }));
    if has_tool_use {
        score += 15;
    }

    // 包含错误的工具结果减分（可丢弃）
    let has_error = msg
        .blocks
        .iter()
        .any(|b| matches!(b, ContentBlock::ToolResult { is_error: true, .. }));
    if has_error {
        score -= 10;
    }

    // 纯文本内容长度影响
    let text_len: usize = msg
        .blocks
        .iter()
        .map(|b| match b {
            ContentBlock::Text { text } => text.len(),
            _ => 0,
        })
        .sum();
    if text_len > 500 {
        score += 10; // 长消息可能包含重要信息
    }
    if text_len < 20 {
        score -= 5; // 太短的消息信息量低
    }

    score.clamp(0, 100)
}

/// 选择保留的消息：按重要性排序，保留 top N 条
///
/// 返回值为原始索引列表（已按原始顺序排序），表示应保留的消息位置。
/// 优先保留得分高的消息，同等分数时保留位置靠前的。
#[must_use]
pub fn select_top_messages(messages: &[ConversationMessage], keep_count: usize) -> Vec<usize> {
    let actual_keep = keep_count.min(messages.len());
    if actual_keep == 0 || messages.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(usize, u32)> = messages
        .iter()
        .enumerate()
        .map(|(i, msg)| (i, score_message(msg)))
        .collect();

    // 按分数降序排列（稳定排序保证同分时保持原序）
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    // 取前 N 条，恢复原始顺序
    let mut indices: Vec<usize> = scored.iter().take(actual_keep).map(|(i, _)| *i).collect();
    indices.sort();
    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{ContentBlock, ConversationMessage, MessageRole};

    #[test]
    fn user_messages_score_higher() {
        let user_msg = ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "帮我分析这个bug".into(),
            }],
            usage: None,
        };
        let assistant_msg = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::Text {
                text: "好的".into(),
            }],
            usage: None,
        };
        assert!(score_message(&user_msg) > score_message(&assistant_msg));
    }

    #[test]
    fn tool_use_scores_higher() {
        let plain = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::Text { text: "ok".into() }],
            usage: None,
        };
        let with_tool = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::ToolUse {
                id: "1".into(),
                name: "Read".into(),
                input: "{}".into(),
            }],
            usage: None,
        };
        assert!(score_message(&with_tool) > score_message(&plain));
    }

    #[test]
    fn error_tool_results_score_lower() {
        let ok_result = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "1".into(),
                tool_name: "bash".into(),
                output: "success".into(),
                is_error: false,
            }],
            usage: None,
        };
        let error_result = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "1".into(),
                tool_name: "bash".into(),
                output: "command not found".into(),
                is_error: true,
            }],
            usage: None,
        };
        assert!(score_message(&ok_result) > score_message(&error_result));
    }

    #[test]
    fn long_text_scores_higher_than_short() {
        let long_msg = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::Text {
                text: "x".repeat(600),
            }],
            usage: None,
        };
        let short_msg = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::Text { text: "ok".into() }],
            usage: None,
        };
        assert!(score_message(&long_msg) > score_message(&short_msg));
    }

    #[test]
    fn score_clamped_to_zero() {
        // 构造一个评分可能为负的消息：极短 + 错误工具结果
        let msg = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![
                ContentBlock::Text {
                    text: "x".into(), // 极短文本
                },
                ContentBlock::ToolResult {
                    tool_use_id: "1".into(),
                    tool_name: "bash".into(),
                    output: "error".into(),
                    is_error: true, // 错误
                },
            ],
            usage: None,
        };
        // 基础 50 - 5(短文本) - 10(错误) = 35，不应为负
        let score = score_message(&msg);
        assert!(score <= 100);
        assert!(score >= 35); // 验证计算正确
    }

    #[test]
    fn select_top_messages_respects_count() {
        let messages = vec![
            ConversationMessage::user_text("重要问题"),
            ConversationMessage::user_text("ok"),
            ConversationMessage::user_text("另外一个重要问题"),
            ConversationMessage::user_text("嗯"),
        ];
        // 保留前 2 条
        let indices = select_top_messages(&messages, 2);
        assert_eq!(indices.len(), 2);
        // 应该按原始顺序排列
        assert!(indices.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn select_top_messages_empty_input() {
        let indices = select_top_messages(&[], 5);
        assert!(indices.is_empty());
    }

    #[test]
    fn select_top_messages_keep_count_exceeds_len() {
        let messages = vec![ConversationMessage::user_text("hi")];
        let indices = select_top_messages(&messages, 10);
        assert_eq!(indices.len(), 1);
    }
}
