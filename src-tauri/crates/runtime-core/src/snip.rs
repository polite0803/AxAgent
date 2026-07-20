// SPDX-License-Identifier: AGPL-3.0-only

//! L1 Snip 轻量裁剪
//!
//! 在每次发送请求给 LLM 之前,对超长的 ToolResult 输出做就地截断:
//! 保留头部 + 尾部,中间用占位符 `[... snipped N chars ...]` 替换。
//!
//! 与 L3 Auto Compact(整体摘要)和 L4 Reactive Compact(错误后压缩)不同,
//! L1 Snip 不调用 LLM,不修改 session 自身,仅作用于"即将发给 LLM 的请求副本",
//! 用于控制单条工具结果对上下文窗口的占用。
//!
//! 移植自 claude-code-main 的 snipToolResults 设计。

use crate::session::{ContentBlock, ConversationMessage};

/// L1 Snip 配置。
#[derive(Debug, Clone, Copy)]
pub struct SnipConfig {
    /// 是否启用 snip。关闭时原样返回消息。
    pub enabled: bool,
    /// 单条 ToolResult 输出超过此字符数时触发 snip。
    pub max_tool_result_chars: usize,
    /// 触发 snip 时保留头部的字符数。
    pub preserve_head_chars: usize,
    /// 触发 snip 时保留尾部的字符数。
    pub preserve_tail_chars: usize,
}

impl Default for SnipConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            // 默认阈值 ~30K 字符(约 7.5K tokens),覆盖大多数工具输出场景
            max_tool_result_chars: 30_000,
            preserve_head_chars: 12_000,
            preserve_tail_chars: 8_000,
        }
    }
}

impl SnipConfig {
    /// 完全关闭 snip(用于测试或显式禁用)。
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            max_tool_result_chars: usize::MAX,
            preserve_head_chars: 0,
            preserve_tail_chars: 0,
        }
    }
}

/// 对消息列表中的 ToolResult 做就地 snip 处理。
///
/// 此函数不修改原始消息,返回新的消息向量。
/// 仅当 ToolResult 的 `output` 字段超过阈值时才截断。
#[must_use]
pub fn snip_tool_results(
    messages: &[ConversationMessage],
    config: &SnipConfig,
) -> Vec<ConversationMessage> {
    if !config.enabled {
        return messages.to_vec();
    }
    messages.iter().map(|msg| snip_message(msg, config)).collect()
}

/// 对单条消息中的 ToolResult blocks 做 snip。
fn snip_message(msg: &ConversationMessage, config: &SnipConfig) -> ConversationMessage {
    let needs_snip = msg.blocks.iter().any(|b| match b {
        ContentBlock::ToolResult { output, .. } => {
            output.chars().count() > config.max_tool_result_chars
        },
        _ => false,
    });
    if !needs_snip {
        return msg.clone();
    }
    let mut new_msg = msg.clone();
    for block in &mut new_msg.blocks {
        if let ContentBlock::ToolResult { output, .. } = block {
            let len = output.chars().count();
            if len > config.max_tool_result_chars {
                *output = snip_string(output, config);
            }
        }
    }
    new_msg
}

/// 对单个字符串做 head + tail 截断,中间替换为占位符。
fn snip_string(s: &str, config: &SnipConfig) -> String {
    let chars: Vec<char> = s.chars().collect();
    let total = chars.len();
    let head = config.preserve_head_chars.min(total);
    let tail = config.preserve_tail_chars.min(total.saturating_sub(head));
    let snipped = total.saturating_sub(head + tail);
    let head_str: String = chars[..head].iter().collect();
    let tail_str: String = chars[total - tail..].iter().collect();
    format!("{head_str}\n[... snipped {snipped} chars ...]\n{tail_str}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ConversationMessageExt;

    fn make_tool_result_message(output: &str) -> ConversationMessage {
        ConversationMessageExt::tool_result("toolu_1", "test_tool", output, false)
    }

    #[test]
    fn test_disabled_config_returns_clone() {
        let msg = make_tool_result_message(&"x".repeat(100_000));
        let config = SnipConfig::disabled();
        let result = snip_tool_results(std::slice::from_ref(&msg), &config);
        assert_eq!(result[0], msg);
    }

    #[test]
    fn test_short_output_unchanged() {
        let msg = make_tool_result_message("short output");
        let config = SnipConfig::default();
        let result = snip_tool_results(std::slice::from_ref(&msg), &config);
        assert_eq!(result[0], msg);
    }

    #[test]
    fn test_long_output_snipped() {
        let long_output = format!("{}{}", "H".repeat(50_000), "T".repeat(50_000));
        let msg = make_tool_result_message(&long_output);
        let config = SnipConfig::default();
        let result = snip_tool_results(&[msg], &config);
        if let ContentBlock::ToolResult { output, .. } = &result[0].blocks[0] {
            // snip 后应明显短于原 100K 字符
            assert!(output.chars().count() < 50_000);
            assert!(output.contains("[... snipped "));
            assert!(output.contains("chars ...]"));
            // 头尾应被保留
            assert!(output.starts_with('H'));
            assert!(output.ends_with('T'));
        } else {
            panic!("expected ToolResult block");
        }
    }

    #[test]
    fn test_mixed_blocks_only_snips_long_ones() {
        let short_msg = make_tool_result_message("short");
        let long_msg = make_tool_result_message(&"x".repeat(50_000));
        let config = SnipConfig::default();
        let result = snip_tool_results(&[short_msg.clone(), long_msg], &config);
        assert_eq!(result[0], short_msg);
        if let ContentBlock::ToolResult { output, .. } = &result[1].blocks[0] {
            assert!(output.contains("[... snipped "));
        } else {
            panic!("expected ToolResult");
        }
    }

    #[test]
    fn test_snip_string_preserves_head_and_tail() {
        let s = "ABCDEFGH".repeat(10_000); // 80K chars
        let config = SnipConfig::default();
        let snipped = snip_string(&s, &config);
        assert!(snipped.starts_with("ABCDEFGH"));
        assert!(snipped.ends_with("ABCDEFGH"));
        assert!(snipped.contains("[... snipped "));
    }

    #[test]
    fn test_unicode_safe() {
        // 中文字符 3 字节,确保按 char 而非 byte 截断
        let s = "你".repeat(50_000);
        let msg = make_tool_result_message(&s);
        let config = SnipConfig::default();
        let result = snip_tool_results(&[msg], &config);
        if let ContentBlock::ToolResult { output, .. } = &result[0].blocks[0] {
            // 不应在多字节字符中间截断(不会出现 invalid char)
            assert!(output.contains("[... snipped "));
        } else {
            panic!("expected ToolResult");
        }
    }
}
