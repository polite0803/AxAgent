// SPDX-License-Identifier: AGPL-3.0-only

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
    /// 压缩后保留的最小 token 数（固定基础值）
    pub min_tokens: u64,
    /// 压缩后保留的包含文本块的最小消息数
    pub min_text_block_messages: usize,
    /// 压缩后保留的最大 token 数（硬上限）
    pub max_tokens: u64,
    /// 是否启用会话记忆压缩
    pub enabled: bool,
    /// 自适应压缩模式：true = 根据历史使用率动态调整阈值
    #[serde(default)]
    pub adaptive: bool,
    /// 自适应因子（0.5~2.0）：越高→保留越多上下文
    #[serde(default = "default_adaptive_factor")]
    pub adaptive_factor: f64,
    /// 最近 N 次估算的 token 使用率历史
    #[serde(skip)]
    pub usage_history: Vec<f64>,
}

fn default_adaptive_factor() -> f64 {
    1.0
}

impl Default for SessionMemoryCompactConfig {
    fn default() -> Self {
        Self {
            min_tokens: 10_000,
            min_text_block_messages: 5,
            max_tokens: 40_000,
            enabled: true,
            adaptive: false,
            adaptive_factor: 1.0,
            usage_history: Vec::new(),
        }
    }
}

impl SessionMemoryCompactConfig {
    /// 记录一次 token 使用率观察值。
    /// ratio = 当前会话 token 数 / 模型最大上下文窗口
    pub fn record_usage(&mut self, current_tokens: u64, max_window: u64) {
        if max_window == 0 {
            return;
        }
        let ratio = (current_tokens as f64 / max_window as f64).clamp(0.0, 1.0);
        self.usage_history.push(ratio);
        // 保留最近 10 次记录
        if self.usage_history.len() > 10 {
            self.usage_history.remove(0);
        }
    }

    /// 计算动态压缩阈值。
    /// 基于历史使用率趋势：如果使用率持续升高 → 提高压缩力度（降低阈值）
    /// 如果使用率持续偏低 → 降低压缩力度（提高阈值）
    pub fn effective_max_tokens(&self) -> u64 {
        if !self.adaptive || self.usage_history.len() < 3 {
            return self.max_tokens;
        }

        let avg_usage: f64 =
            self.usage_history.iter().copied().sum::<f64>() / self.usage_history.len() as f64;

        // 趋势：最近 3 次 vs 全部历史
        let recent: f64 = self
            .usage_history
            .iter()
            .rev()
            .take(3)
            .copied()
            .sum::<f64>()
            / 3.0;
        let trend = recent - avg_usage; // 正值 = 使用率在上升

        // 基础压缩比例：基于平均使用率
        // 使用率 20% → 保留 80%, 使用率 80% → 保留 40%
        let base_compression = 1.0 - (avg_usage * 0.5);

        // 趋势调整：上升趋势 → 额外压缩 10%, 下降趋势 → 放松 10%
        let trend_adjust = if trend > 0.05 {
            -0.1 // 使用率上升，需要更激进压缩
        } else if trend < -0.05 {
            0.1 // 使用率下降，可保留更多
        } else {
            0.0
        };

        let effective_ratio =
            (base_compression + trend_adjust).clamp(0.3, 0.95) * self.adaptive_factor;

        let result = (self.max_tokens as f64 * effective_ratio) as u64;
        // 确保不低于 min_tokens 的 120%
        let min_allowed = (self.min_tokens as f64 * 1.2) as u64;
        result.max(min_allowed).min(self.max_tokens)
    }

    /// 获取有效 min_tokens（自适应时动态调节）
    pub fn effective_min_tokens(&self) -> u64 {
        if !self.adaptive || self.usage_history.is_empty() {
            return self.min_tokens;
        }
        let avg_usage: f64 =
            self.usage_history.iter().copied().sum::<f64>() / self.usage_history.len() as f64;
        // 使用率高时提高 min_tokens（保留更多上下文），使用率低时降低
        let adjustment = 1.0 + (avg_usage - 0.3).clamp(-0.3, 0.3);
        let result = (self.min_tokens as f64 * adjustment) as u64;
        result.max(self.min_tokens / 2).min(self.min_tokens * 2)
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

    // ── 自适应阈值 ──
    let effective_max = config.effective_max_tokens();
    let effective_min = config.effective_min_tokens();

    // 构建结构化记忆摘要
    let (memory_content, was_truncated) = build_session_memory_content(memories, effective_max);

    // 从尾部计算起始索引（使用自适应阈值）
    let start_index = compute_compact_start_index(
        &session.messages,
        effective_min,
        config.min_text_block_messages,
        effective_max,
    );

    // 确保起始索引有效
    if start_index >= session.messages.len() {
        return None;
    }

    // 调整索引导避免割裂 tool_use/tool_result 配对
    let adjusted_start = adjust_index_to_preserve_pairs(&session.messages, start_index);

    let messages_to_keep: Vec<ConversationMessage> = session.messages[adjusted_start..].to_vec();

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
        if accumulated_tokens + msg_tokens > max_tokens
            && text_block_messages >= min_text_block_messages
        {
            keep_from = i + 1;
            break;
        }

        accumulated_tokens += msg_tokens;

        // 检查文本块
        if msg
            .blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::Text { .. }))
        {
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
fn adjust_index_to_preserve_pairs(messages: &[ConversationMessage], start_index: usize) -> usize {
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
        formatted_summary: format!("Session Memory Summary:\n{}", sm_result.session_memory_content),
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
                    .push_message(ConversationMessage::assistant(vec![ContentBlock::Text { text }]))
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
        let result =
            try_session_memory_compact(&session, &[], &config, CompactionConfig::default());
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
            adaptive: false,
            adaptive_factor: 1.0,
            usage_history: Vec::new(),
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
            .push_message(ConversationMessage::assistant(vec![ContentBlock::ToolUse {
                id: tool_id.to_string(),
                name: "read_file".to_string(),
                input: "main.rs".to_string(),
            }]))
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
            adaptive: false,
            adaptive_factor: 1.0,
            usage_history: Vec::new(),
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

    // ── 自适应压缩测试 ──

    #[test]
    fn test_adaptive_config_defaults() {
        let config = SessionMemoryCompactConfig::default();
        assert!(!config.adaptive);
        assert!((config.adaptive_factor - 1.0).abs() < f64::EPSILON);
        assert!(config.usage_history.is_empty());
    }

    #[test]
    fn test_record_usage_and_effective() {
        let mut config = SessionMemoryCompactConfig::default();
        config.adaptive = true;
        config.max_tokens = 100_000;
        config.min_tokens = 10_000;

        // 记录低使用率
        config.record_usage(10_000, 128_000); // ~8%
        assert_eq!(config.usage_history.len(), 1);

        // 自适应未生效时（history < 3），effective = base
        let eff_max = config.effective_max_tokens();
        assert_eq!(eff_max, 100_000); // 默认值，history 不足 3

        // 记录足够多的观察值
        config.record_usage(12_000, 128_000);
        config.record_usage(15_000, 128_000);
        assert_eq!(config.usage_history.len(), 3);

        // 低使用率 → 应该保留更多
        let eff_max_after = config.effective_max_tokens();
        assert!(eff_max_after <= 100_000); // 应该 <= max
        assert!(eff_max_after >= 12_000); // 应该 >= min*1.2
    }

    #[test]
    fn test_high_usage_causes_more_compression() {
        let mut config = SessionMemoryCompactConfig::default();
        config.adaptive = true;
        config.max_tokens = 100_000;
        config.min_tokens = 10_000;

        // 模拟高使用率
        config.record_usage(100_000, 128_000); // ~78%
        config.record_usage(110_000, 128_000);
        config.record_usage(120_000, 128_000);
        config.record_usage(115_000, 128_000);

        let eff_max = config.effective_max_tokens();
        // 高使用率 → 更激进的压缩 → significantly < 100_000
        assert!(eff_max < 80_000, "expected heavy compression, got {eff_max}");
    }

    #[test]
    fn test_rising_trend_triggers_tighter_compression() {
        let mut config = SessionMemoryCompactConfig::default();
        config.adaptive = true;
        config.max_tokens = 100_000;
        config.min_tokens = 10_000;

        // 模拟使用率快速上升趋势
        config.record_usage(10_000, 128_000);
        config.record_usage(50_000, 128_000);
        config.record_usage(90_000, 128_000);
        config.record_usage(110_000, 128_000);

        let eff_max = config.effective_max_tokens();
        // 持续上升 → 应该压缩
        assert!(eff_max < 100_000);
    }

    #[test]
    fn test_adaptive_disabled_uses_base_values() {
        let mut config = SessionMemoryCompactConfig::default();
        config.adaptive = false; // 显式禁用
        config.max_tokens = 100_000;
        config.min_tokens = 10_000;

        // 即使有历史数据也不影响
        config.record_usage(110_000, 128_000);
        config.record_usage(120_000, 128_000);
        config.record_usage(130_000, 128_000);

        let eff_max = config.effective_max_tokens();
        assert_eq!(eff_max, 100_000); // 禁用时返回 base max
    }

    #[test]
    fn test_effective_min_tokens_scaling() {
        let mut config = SessionMemoryCompactConfig::default();
        config.adaptive = true;
        config.min_tokens = 10_000;

        // 高使用率 → min_tokens 上浮
        config.record_usage(100_000, 128_000);
        config.record_usage(110_000, 128_000);
        config.record_usage(120_000, 128_000);

        let eff_min = config.effective_min_tokens();
        assert!(eff_min > 10_000, "expected higher min_tokens under high usage, got {eff_min}");

        // 低使用率 → min_tokens 下降
        let mut config2 = SessionMemoryCompactConfig::default();
        config2.adaptive = true;
        config2.min_tokens = 10_000;
        config2.record_usage(10_000, 128_000);
        config2.record_usage(12_000, 128_000);
        config2.record_usage(8_000, 128_000);

        let eff_min2 = config2.effective_min_tokens();
        assert!(
            eff_min2 <= 15_000,
            "expected moderate min_tokens under low usage, got {eff_min2}"
        );
    }

    #[test]
    fn test_usage_history_trimming() {
        let mut config = SessionMemoryCompactConfig::default();
        for i in 1..=15 {
            config.record_usage(i * 5_000, 128_000);
        }
        // 最多保留 10 条
        assert!(config.usage_history.len() <= 10);
    }
}
