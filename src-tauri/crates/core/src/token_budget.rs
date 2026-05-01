//! 运行时 Token 预算跟踪器
//!
//! 在每次 ReAct 迭代中监控 token 消耗，检测收益递减（模型在无效循环中浪费上下文窗口），
//! 并作出继续/终止决策。移植自 claude-code-main 的 tokenBudget.ts。

use std::time::Instant;

/// 完成阈值：当消耗达到预算的 90% 时触发终止
const COMPLETION_THRESHOLD: f64 = 0.9;

/// 收益递减检测阈值（token）：连续 continuation 的 delta 低于此值时判定为递减
const DIMINISHING_DELTA_THRESHOLD: u64 = 500;

/// 收益递减触发前需要的最小 continuation 次数
const MIN_CONTINUATIONS_FOR_DIMINISHING: u32 = 3;

/// 跟踪跨 continuation 轮次的 token 消耗，检测收益递减。
#[derive(Debug, Clone)]
pub struct TokenBudgetTracker {
    /// Agent 被允许继续的轮次计数
    pub continuation_count: u32,
    /// 自上次检查以来的 token 增量
    pub last_delta_tokens: u64,
    /// 上次检查时的全局轮次 token 总数
    pub last_global_turn_tokens: u64,
    /// 跟踪会话开始时间（用于基于持续时间的决策）
    pub started_at: Instant,
}

impl Default for TokenBudgetTracker {
    fn default() -> Self {
        Self {
            continuation_count: 0,
            last_delta_tokens: 0,
            last_global_turn_tokens: 0,
            started_at: Instant::now(),
        }
    }
}

impl TokenBudgetTracker {
    /// 创建一个新的跟踪器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 重置跟踪器状态（新会话或手动重置时使用）。
    pub fn reset(&mut self) {
        self.continuation_count = 0;
        self.last_delta_tokens = 0;
        self.last_global_turn_tokens = 0;
        self.started_at = Instant::now();
    }

    /// 记录一轮新的 token 消耗（不计入预算检查，仅更新内部状态）。
    pub fn record_tokens(&mut self, global_turn_tokens: u64) {
        self.last_delta_tokens =
            global_turn_tokens.saturating_sub(self.last_global_turn_tokens);
        self.last_global_turn_tokens = global_turn_tokens;
    }
}

/// 每次 token 预算检查返回的决策。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenBudgetDecision {
    /// 继续循环；携带给模型的提示消息
    Continue {
        /// 提示模型 token 使用情况的消息
        nudge_message: String,
        /// 当前 continuation 计数
        continuation_count: u32,
        /// 已使用百分比 (0-100)
        pct_used: u32,
        /// 当前轮次 token 数
        turn_tokens: u64,
        /// 预算上限
        budget: u64,
    },
    /// 停止循环；可选附带完成事件
    Stop {
        completion_event: Option<BudgetCompletionEvent>,
    },
}

/// 预算耗尽时的完成事件信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetCompletionEvent {
    pub continuation_count: u32,
    pub pct_used: u32,
    pub turn_tokens: u64,
    pub budget: u64,
    /// 是否因收益递减而终止
    pub diminishing_returns: bool,
    /// 跟踪会话持续时间（毫秒）
    pub duration_ms: u64,
}

impl TokenBudgetTracker {
    /// 执行一次预算检查。
    ///
    /// # 参数
    /// - `budget`: 可选的预算上限（None 表示无预算限制，返回 Stop）
    /// - `global_turn_tokens`: 当前全局轮次 token 总数
    ///
    /// # 决策逻辑
    /// 1. 无预算或预算为 0 → 立即停止
    /// 2. 检测收益递减（连续 3+ 次 continuation 且 delta < 500 tokens）→ 停止
    /// 3. 消耗 < 90% 预算 → 继续
    /// 4. 消耗 >= 90% 预算 → 停止
    pub fn check(
        &mut self,
        budget: Option<u64>,
        global_turn_tokens: u64,
    ) -> TokenBudgetDecision {
        let Some(budget) = budget else {
            return TokenBudgetDecision::Stop {
                completion_event: None,
            };
        };

        if budget == 0 {
            return TokenBudgetDecision::Stop {
                completion_event: None,
            };
        }

        let turn_tokens = global_turn_tokens;
        let pct = ((turn_tokens as f64 / budget as f64) * 100.0).round() as u32;
        let delta = global_turn_tokens.saturating_sub(self.last_global_turn_tokens);

        // 检测收益递减：连续多次 continuation 但每次增量很小
        let is_diminishing = self.continuation_count >= MIN_CONTINUATIONS_FOR_DIMINISHING
            && delta < DIMINISHING_DELTA_THRESHOLD
            && self.last_delta_tokens < DIMINISHING_DELTA_THRESHOLD;

        // 未达到阈值且非递减 → 继续
        if !is_diminishing
            && (turn_tokens as f64) < budget as f64 * COMPLETION_THRESHOLD
        {
            self.continuation_count += 1;
            self.last_delta_tokens = delta;
            self.last_global_turn_tokens = global_turn_tokens;

            return TokenBudgetDecision::Continue {
                nudge_message: build_nudge_message(pct, turn_tokens, budget),
                continuation_count: self.continuation_count,
                pct_used: pct,
                turn_tokens,
                budget,
            };
        }

        // 达到阈值或检测到递减 → 停止
        if is_diminishing || self.continuation_count > 0 {
            return TokenBudgetDecision::Stop {
                completion_event: Some(BudgetCompletionEvent {
                    continuation_count: self.continuation_count,
                    pct_used: pct,
                    turn_tokens,
                    budget,
                    diminishing_returns: is_diminishing,
                    duration_ms: self.started_at.elapsed().as_millis() as u64,
                }),
            };
        }

        TokenBudgetDecision::Stop {
            completion_event: None,
        }
    }
}

/// 构建给模型的 token 消耗提示消息。
fn build_nudge_message(pct_used: u32, turn_tokens: u64, budget: u64) -> String {
    let remaining = budget.saturating_sub(turn_tokens);
    format!(
        "Token usage: {pct_used}% of budget used ({turn_tokens}/{budget} tokens). \
         {remaining} tokens remaining. Continue efficiently or wrap up soon.",
        pct_used = pct_used,
        turn_tokens = turn_tokens,
        budget = budget,
        remaining = remaining,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_budget_returns_stop() {
        let mut tracker = TokenBudgetTracker::new();
        let decision = tracker.check(None, 100);
        assert!(matches!(decision, TokenBudgetDecision::Stop { completion_event: None }));
    }

    #[test]
    fn test_zero_budget_returns_stop() {
        let mut tracker = TokenBudgetTracker::new();
        let decision = tracker.check(Some(0), 0);
        assert!(matches!(decision, TokenBudgetDecision::Stop { completion_event: None }));
    }

    #[test]
    fn test_below_threshold_returns_continue() {
        let mut tracker = TokenBudgetTracker::new();
        // 1000 tokens out of 100_000 budget = 1%, well below 90%
        let decision = tracker.check(Some(100_000), 1_000);
        assert!(matches!(decision, TokenBudgetDecision::Continue { .. }));
        if let TokenBudgetDecision::Continue { pct_used, .. } = decision {
            assert_eq!(pct_used, 1);
        }
    }

    #[test]
    fn test_above_threshold_returns_stop() {
        let mut tracker = TokenBudgetTracker::new();
        // 95_000 tokens out of 100_000 budget = 95%, above 90%
        let decision = tracker.check(Some(100_000), 95_000);
        assert!(matches!(decision, TokenBudgetDecision::Stop { .. }));
        if let TokenBudgetDecision::Stop { completion_event: Some(event) } = decision {
            assert_eq!(event.pct_used, 95);
            assert!(!event.diminishing_returns);
        }
    }

    #[test]
    fn test_diminishing_returns_detection() {
        let mut tracker = TokenBudgetTracker::new();

        // 第一次 continuation：5000 tokens → Continue
        let decision1 = tracker.check(Some(100_000), 5_000);
        assert!(matches!(decision1, TokenBudgetDecision::Continue { .. }));

        // 第二次 continuation：5200 tokens (delta 200) → Continue
        let decision2 = tracker.check(Some(100_000), 5_200);
        assert!(matches!(decision2, TokenBudgetDecision::Continue { .. }));

        // 第三次 continuation：5400 tokens (delta 200) → Continue
        // (continuation_count 此时为 2, 不满足 >= 3 的条件)
        let decision3 = tracker.check(Some(100_000), 5_400);
        assert!(matches!(decision3, TokenBudgetDecision::Continue { .. }));

        // 第四次 continuation：5600 tokens (delta 200, last_delta 200)
        // continuation_count=3 >= 3, delta < 500, last_delta < 500 → diminishing!
        let decision4 = tracker.check(Some(100_000), 5_600);
        assert!(
            matches!(decision4, TokenBudgetDecision::Stop {
                completion_event: Some(BudgetCompletionEvent { diminishing_returns: true, .. })
            }),
            "应该在连续 3 次 continuation 后（第 4 次调用）检测到收益递减"
        );
    }

    #[test]
    fn test_diminishing_not_triggered_early() {
        let mut tracker = TokenBudgetTracker::new();

        // 第一次：大增量
        tracker.check(Some(100_000), 10_000);
        // 第二次：小增量，但 continuation_count 只有 1，不触发
        let decision = tracker.check(Some(100_000), 10_300);
        assert!(matches!(decision, TokenBudgetDecision::Continue { .. }));
    }

    #[test]
    fn test_reset() {
        let mut tracker = TokenBudgetTracker::new();
        tracker.check(Some(100_000), 5_000);
        tracker.check(Some(100_000), 5_200);
        assert_eq!(tracker.continuation_count, 2);

        tracker.reset();
        assert_eq!(tracker.continuation_count, 0);
        assert_eq!(tracker.last_delta_tokens, 0);
        assert_eq!(tracker.last_global_turn_tokens, 0);
    }

    #[test]
    fn test_nudge_message_format() {
        let msg = build_nudge_message(50, 50000, 100000);
        assert!(msg.contains("50%"));
        assert!(msg.contains("50000"));
        assert!(msg.contains("100000"));
    }
}
