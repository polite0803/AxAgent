//! Token 预算预测 — API 调用前预估 token 消耗，主动触发压缩
//!
//! 策略：
//! - 每轮调用前预估输入 token 数
//! - 超过阈值（默认 80% 上下文窗口）时主动触发压缩
//! - 根据历史 token 增长率预测何时需要压缩

use crate::session::ConversationMessage;

/// 一次 token 使用快照，用于历史趋势分析
#[derive(Debug, Clone)]
pub struct TokenSnapshot {
    /// 快照时间戳（毫秒）
    pub timestamp_ms: u64,
    /// 当前预估输入 token 数
    pub input_tokens: u32,
    /// 当前预估总 token 数
    pub total_tokens: u32,
}

/// 预算评估决策
#[derive(Debug, Clone)]
pub enum BudgetDecision {
    /// 正常，无需任何操作
    Proceed,
    /// 建议压缩（当前用量超过压缩阈值）
    CompactRecommended { current_pct: u32 },
    /// 强制压缩（即将超出上下文窗口）
    CompactRequired { current_pct: u32 },
}

/// Token 预算预测器
///
/// 在每次 API 调用前预估 token 消耗，当用量超过阈值时主动触发压缩，
/// 避免因上下文窗口溢出导致 API 调用失败。
#[derive(Debug, Clone)]
pub struct TokenBudgetPredictor {
    /// 上下文窗口大小（token 数）
    context_window: u32,
    /// 压缩阈值（占上下文窗口百分比，0-100）
    compact_threshold_pct: u32,
    /// 历史 token 使用记录（最多保留 20 条）
    history: Vec<TokenSnapshot>,
}

impl TokenBudgetPredictor {
    /// 创建指定上下文窗口大小的预测器
    pub fn new(context_window: u32) -> Self {
        Self {
            context_window,
            compact_threshold_pct: 80,
            history: Vec::new(),
        }
    }

    /// 预估消息列表的 token 数（简化的字符/4 估算法）
    ///
    /// 这是一个快速估算方法，不需要实际调用 tokenizer。
    /// 对于英文文本，1 token ≈ 4 字符；对于中文，实际系数不同，
    /// 但作为预算预警已足够准确。
    pub fn estimate_tokens(messages: &[ConversationMessage]) -> u32 {
        let mut total = 0u32;
        for msg in messages {
            // 角色标记开销
            total += 4;
            for block in &msg.blocks {
                total += match block {
                    crate::session::ContentBlock::Text { text } => {
                        (text.len() as u32 / 4).max(1)
                    }
                    crate::session::ContentBlock::ToolUse { name, input, .. } => {
                        8 + (name.len() as u32 / 4) + (input.len() as u32 / 4)
                    }
                    crate::session::ContentBlock::ToolResult { output, .. } => {
                        4 + (output.len() as u32 / 4)
                    }
                };
            }
        }
        total
    }

    /// 评估当前 token 预算并返回决策
    ///
    /// 同时将当前快照记录到历史中，用于后续趋势分析。
    pub fn evaluate(&mut self, messages: &[ConversationMessage]) -> BudgetDecision {
        let estimated = Self::estimate_tokens(messages);
        let pct = ((estimated as f64 / self.context_window as f64) * 100.0) as u32;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.history.push(TokenSnapshot {
            timestamp_ms: now,
            input_tokens: estimated,
            total_tokens: estimated,
        });

        // 只保留最近 20 条记录
        if self.history.len() > 20 {
            self.history.remove(0);
        }

        if pct >= 95 {
            BudgetDecision::CompactRequired { current_pct: pct }
        } else if pct >= self.compact_threshold_pct {
            BudgetDecision::CompactRecommended { current_pct: pct }
        } else {
            BudgetDecision::Proceed
        }
    }

    /// 预测 token 增长率（token/秒）
    ///
    /// 基于历史首尾快照的时间差和 token 增量计算。
    pub fn predicted_growth_rate(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }
        let first = &self.history[0];
        let last = &self.history.last().unwrap();
        let dt = (last.timestamp_ms - first.timestamp_ms).max(1) as f64 / 1000.0;
        let dtokens = (last.total_tokens as f64 - first.total_tokens as f64).max(0.0);
        dtokens / dt.max(1.0)
    }

    /// 预估还需要多少轮对话才需要压缩
    ///
    /// 返回 `None` 表示当前增长率不足以触发压缩（或历史数据不足）。
    pub fn rounds_until_compact(&self) -> Option<u32> {
        let rate = self.predicted_growth_rate();
        if rate <= 0.0 {
            return None;
        }
        let last = self.history.last()?;
        let remaining =
            (self.context_window as f64 * 0.8) - last.total_tokens as f64;
        if remaining <= 0.0 {
            return Some(0);
        }
        Some((remaining / rate).ceil() as u32)
    }
}

impl Default for TokenBudgetPredictor {
    fn default() -> Self {
        Self::new(200_000) // Claude 默认 200K 上下文窗口
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ContentBlock;
    use crate::session::MessageRole;

    #[test]
    fn estimate_empty_is_zero() {
        assert_eq!(TokenBudgetPredictor::estimate_tokens(&[]), 0);
    }

    #[test]
    fn estimate_text_message() {
        let msg = ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "hello world".to_string(),
            }],
            usage: None,
        };
        // 4 (角色) + "hello world" 11 字符 / 4 = 2 → max(1) → 2 → total: 4 + 2 = 6
        let tokens = TokenBudgetPredictor::estimate_tokens(&[msg]);
        assert!(tokens > 0);
    }

    #[test]
    fn compact_recommended_over_threshold() {
        let mut predictor = TokenBudgetPredictor::new(100);
        // 构造一个超过 80% 阈值的消息
        let msg = ConversationMessage {
            role: crate::session::MessageRole::User,
            blocks: vec![crate::session::ContentBlock::Text {
                text: "x".repeat(400), // 400 / 4 ≈ 100 + 4 角色 = 104 > 100
            }],
            usage: None,
        };
        let decision = predictor.evaluate(&[msg]);
        assert!(matches!(
            decision,
            BudgetDecision::CompactRequired { .. } | BudgetDecision::CompactRecommended { .. }
        ));
    }

    #[test]
    fn proceed_when_under_threshold() {
        let mut predictor = TokenBudgetPredictor::new(100_000);
        let msg = ConversationMessage {
            role: crate::session::MessageRole::User,
            blocks: vec![crate::session::ContentBlock::Text {
                text: "hello".to_string(),
            }],
            usage: None,
        };
        let decision = predictor.evaluate(&[msg]);
        assert!(matches!(decision, BudgetDecision::Proceed));
    }

    #[test]
    fn growth_rate_zero_with_insufficient_data() {
        let predictor = TokenBudgetPredictor::new(200_000);
        assert_eq!(predictor.predicted_growth_rate(), 0.0);
    }

    #[test]
    fn growth_rate_with_two_snapshots() {
        let mut predictor = TokenBudgetPredictor::new(200_000);
        // 手动构造两个时间间隔为 1000ms、token 相差 100 的快照
        predictor.history.push(TokenSnapshot {
            timestamp_ms: 1000,
            input_tokens: 100,
            total_tokens: 100,
        });
        predictor.history.push(TokenSnapshot {
            timestamp_ms: 2000,
            input_tokens: 200,
            total_tokens: 200,
        });
        let rate = predictor.predicted_growth_rate();
        // 100 token / 1 秒 = 100 token/s
        assert!(rate > 0.0);
    }

    #[test]
    fn rounds_until_compact_without_data() {
        let predictor = TokenBudgetPredictor::new(200_000);
        assert_eq!(predictor.rounds_until_compact(), None);
    }

    #[test]
    fn history_capped_at_20() {
        let mut predictor = TokenBudgetPredictor::new(100_000);
        let msg = ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "a".to_string(),
            }],
            usage: None,
        };
        for _ in 0..25 {
            predictor.evaluate(&[msg.clone()]);
        }
        assert!(predictor.history.len() <= 20);
    }
}
