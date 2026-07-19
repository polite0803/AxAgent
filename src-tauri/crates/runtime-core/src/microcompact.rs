// SPDX-License-Identifier: AGPL-3.0-only

//! L2 Microcompact 工具结果去重
//!
//! 在每次发送请求给 LLM 之前,扫描历史消息中的 ToolResult blocks:
//! - 第一次出现的 ToolResult 完整保留
//! - 内容指纹重复的 ToolResult 替换为占位符
//!
//! 这是 L1 Snip(单条截断)和 L3 Auto Compact(整体摘要)之间的中间层,
//! 用于控制"重复调用同一工具"对上下文窗口的占用,不调用 LLM,不修改 session 自身。
//!
//! 配置项复用 `axagent_kit::unified_config::CacheSettings` 中的
//! `tool_result_cache_max_entries` / `tool_result_cache_ttl_secs`。
//!
//! 移植自 claude-code-main 的 microcompact 设计。

use std::collections::HashMap;
use std::hash::Hasher;

use crate::session::{ContentBlock, ConversationMessage};

/// L2 Microcompact 配置。
#[derive(Debug, Clone, Copy)]
pub struct MicrocompactConfig {
    /// 是否启用。
    pub enabled: bool,
    /// 缓存最多保留多少条 ToolResult 指纹(超出后不再去重,避免内存膨胀)。
    pub max_entries: usize,
    /// 用于计算指纹的输出前缀字符数。
    /// 用前缀 + 总长度做指纹,避免哈希整个大字符串。
    pub fingerprint_prefix_chars: usize,
}

impl Default for MicrocompactConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            // 与 kit/unified_config.rs::CacheSettings::tool_result_cache_max_entries 对齐
            max_entries: 500,
            fingerprint_prefix_chars: 1000,
        }
    }
}

impl MicrocompactConfig {
    /// 完全关闭(用于测试或显式禁用)。
    #[must_use]
    pub const fn disabled() -> Self {
        Self { enabled: false, max_entries: 0, fingerprint_prefix_chars: 0 }
    }
}

/// 对消息列表中的 ToolResult 做去重处理。
///
/// 此函数不修改原始消息,返回新的消息向量。
/// 仅 ToolResult blocks 受影响,Text / ToolUse blocks 原样保留。
#[must_use]
pub fn microcompact_messages(
    messages: &[ConversationMessage],
    config: &MicrocompactConfig,
) -> Vec<ConversationMessage> {
    if !config.enabled || config.max_entries == 0 {
        return messages.to_vec();
    }
    let mut seen: HashMap<u64, ()> = HashMap::with_capacity(config.max_entries);
    messages
        .iter()
        .map(|msg| {
            let needs_dedup = msg.blocks.iter().any(|b| match b {
                ContentBlock::ToolResult { is_error: false, output, .. } => {
                    output.chars().count() > config.fingerprint_prefix_chars
                }
                _ => false,
            });
            if !needs_dedup {
                return msg.clone();
            }
            let mut new_msg = msg.clone();
            for block in &mut new_msg.blocks {
                if let ContentBlock::ToolResult { output, is_error, .. } = block {
                    // 错误结果不去重(LLM 需要看到失败信息)
                    if *is_error {
                        continue;
                    }
                    // 只对超过指纹前缀长度的结果去重(短结果保留)
                    if output.chars().count() <= config.fingerprint_prefix_chars {
                        continue;
                    }
                    let fingerprint = fingerprint_output(output, config.fingerprint_prefix_chars);
                    if seen.contains_key(&fingerprint) {
                        let original_chars = output.chars().count();
                        *output = format!(
                            "[... duplicated tool result, {original_chars} chars, see earlier occurrence ...]"
                        );
                    } else {
                        if seen.len() < config.max_entries {
                            seen.insert(fingerprint, ());
                        }
                    }
                }
            }
            new_msg
        })
        .collect()
}

/// 计算 ToolResult 输出的指纹(基于前缀 + 总字符数)。
fn fingerprint_output(s: &str, prefix_chars: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let prefix: String = s.chars().take(prefix_chars).collect();
    hasher.write(prefix.as_bytes());
    hasher.write_usize(s.chars().count());
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ConversationMessageExt;

    fn make_tool_result_message(output: &str) -> ConversationMessage {
        ConversationMessageExt::tool_result("toolu_1", "test_tool", output, false)
    }

    fn make_error_result_message(output: &str) -> ConversationMessage {
        ConversationMessageExt::tool_result("toolu_1", "test_tool", output, true)
    }

    fn long_output() -> String {
        "X".repeat(2000)
    }

    #[test]
    fn test_disabled_returns_clone() {
        let msg = make_tool_result_message(&long_output());
        let config = MicrocompactConfig::disabled();
        let result = microcompact_messages(std::slice::from_ref(&msg), &config);
        assert_eq!(result[0], msg);
    }

    #[test]
    fn test_first_occurrence_preserved() {
        let msg = make_tool_result_message(&long_output());
        let config = MicrocompactConfig::default();
        let result = microcompact_messages(std::slice::from_ref(&msg), &config);
        assert_eq!(result[0], msg);
    }

    #[test]
    fn test_duplicate_replaced_with_placeholder() {
        let output = long_output();
        let msg1 = make_tool_result_message(&output);
        let msg2 = make_tool_result_message(&output);
        let config = MicrocompactConfig::default();
        let result = microcompact_messages(&[msg1.clone(), msg2], &config);
        // 第一条完整保留
        assert_eq!(result[0], msg1);
        // 第二条被替换为占位符
        if let ContentBlock::ToolResult { output, .. } = &result[1].blocks[0] {
            assert!(output.contains("duplicated tool result"));
            assert!(output.contains("see earlier occurrence"));
        } else {
            panic!("expected ToolResult");
        }
    }

    #[test]
    fn test_short_results_not_deduplicated() {
        // 短结果(< fingerprint_prefix_chars=1000)不去重
        let msg1 = make_tool_result_message("short");
        let msg2 = make_tool_result_message("short");
        let config = MicrocompactConfig::default();
        let result = microcompact_messages(&[msg1.clone(), msg2.clone()], &config);
        assert_eq!(result[0], msg1);
        assert_eq!(result[1], msg2);
    }

    #[test]
    fn test_error_results_not_deduplicated() {
        let output = long_output();
        let msg1 = make_error_result_message(&output);
        let msg2 = make_error_result_message(&output);
        let config = MicrocompactConfig::default();
        let result = microcompact_messages(&[msg1.clone(), msg2.clone()], &config);
        // 错误结果保留完整内容
        assert_eq!(result[0], msg1);
        assert_eq!(result[1], msg2);
    }

    #[test]
    fn test_max_entries_limit() {
        // max_entries=1 时,只缓存第一个;后续不同 fingerprint 的输出不去重
        let mut messages = vec![];
        for i in 0..3 {
            // 每个 output 前缀不同(用不同字符填前 2000 字符,确保 fingerprint 不同)
            let prefix_char = match i {
                0 => 'A',
                1 => 'B',
                2 => 'C',
                _ => 'Z',
            };
            let output = format!("{}{}", prefix_char.to_string().repeat(2000), i);
            messages.push(make_tool_result_message(&output));
        }
        let config =
            MicrocompactConfig { enabled: true, max_entries: 1, fingerprint_prefix_chars: 1000 };
        let result = microcompact_messages(&messages, &config);
        // 第一条保留(进入缓存)
        if let ContentBlock::ToolResult { output, .. } = &result[0].blocks[0] {
            assert!(!output.contains("duplicated"));
        } else {
            panic!("expected ToolResult");
        }
        // 第二、三条:因 max_entries=1 不进入缓存,但 fingerprint 不同所以也不会被去重
        for r in &result[1..] {
            if let ContentBlock::ToolResult { output, .. } = &r.blocks[0] {
                assert!(!output.contains("duplicated"));
            } else {
                panic!("expected ToolResult");
            }
        }
    }
}
