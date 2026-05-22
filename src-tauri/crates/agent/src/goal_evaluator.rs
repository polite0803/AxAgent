use crate::reasoning_state::ReasoningContext;
use crate::thought_chain::ThoughtChain;

/// 目标评估结果
#[derive(Debug, Clone)]
pub struct GoalEvaluation {
    /// 目标是否已达成
    pub achieved: bool,
    /// 置信度 0.0-1.0
    pub confidence: f32,
    /// 评估理由
    pub reason: String,
    /// 缺失的子目标
    pub missing: Vec<String>,
}

/// 目标达成评估器
///
/// 在进入 Synthesizing 阶段前评估：
/// 1. 快速检查：sub_goals 是否有对应的已完成步骤
/// 2. 基本指标：验证步骤数、失败步骤数、观察结果
pub struct GoalEvaluator {
    /// 连续未达成次数（防无限重试）
    consecutive_not_achieved: usize,
    /// 最大允许的连续未达成次数（超过后强制进入 Synthesizing）
    max_not_achieved: usize,
}

impl GoalEvaluator {
    pub fn new(max_not_achieved: usize) -> Self {
        Self {
            consecutive_not_achieved: 0,
            max_not_achieved,
        }
    }

    /// 评估目标是否已达成
    ///
    /// 返回 `GoalEvaluation`，调用方根据结果决定是否进入 Synthesizing。
    pub fn evaluate(&mut self, chain: &ThoughtChain, context: &ReasoningContext) -> GoalEvaluation {
        let total_steps = chain.steps.len();
        let _verified_steps = chain.steps.iter().filter(|s| s.is_verified).count();
        let failed_steps = chain
            .steps
            .iter()
            .filter(|s| {
                s.is_verified
                    && s.observation
                        .as_deref()
                        .map(|o| o.contains("Error"))
                        .unwrap_or(false)
            })
            .count();
        let completed_steps = chain
            .steps
            .iter()
            .filter(|s| s.is_verified && s.observation.is_some())
            .count();

        // 检查 sub_goals 覆盖率
        let sub_goals = &context.sub_goals;
        let missing_goals: Vec<String> = if !sub_goals.is_empty() {
            sub_goals
                .iter()
                .filter(|goal| {
                    !chain.steps.iter().any(|s| {
                        s.is_verified && s.reasoning.to_lowercase().contains(&goal.to_lowercase())
                    })
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        let goal_coverage = if sub_goals.is_empty() {
            1.0
        } else {
            1.0 - (missing_goals.len() as f32 / sub_goals.len() as f32)
        };

        // 综合判断
        let has_completed_steps = completed_steps > 0;
        let failure_ratio = if total_steps > 0 {
            failed_steps as f32 / total_steps as f32
        } else {
            0.0
        };
        let no_sub_goals = sub_goals.is_empty();

        // 安全检查：连续多次未达成，强制放行，防止无限重试
        if self.consecutive_not_achieved >= self.max_not_achieved {
            return GoalEvaluation {
                achieved: true,
                confidence: 0.5,
                reason: format!(
                    "连续 {} 次评估未达成，强制进入综合阶段",
                    self.consecutive_not_achieved
                ),
                missing: Vec::new(),
            };
        }

        if !has_completed_steps {
            self.consecutive_not_achieved += 1;
            return GoalEvaluation {
                achieved: false,
                confidence: 0.2,
                reason: "尚未完成任何已验证步骤".to_string(),
                missing: if no_sub_goals {
                    vec!["(无子目标)".to_string()]
                } else {
                    sub_goals.clone()
                },
            };
        }

        if failure_ratio > 0.5 && completed_steps < 3 {
            self.consecutive_not_achieved += 1;
            return GoalEvaluation {
                achieved: false,
                confidence: 0.3,
                reason: format!(
                    "失败率过高 ({:.0}%)，仅 {} 个步骤完成",
                    failure_ratio * 100.0,
                    completed_steps
                ),
                missing: missing_goals,
            };
        }

        if goal_coverage < 0.5 && !no_sub_goals {
            self.consecutive_not_achieved += 1;
            return GoalEvaluation {
                achieved: false,
                confidence: goal_coverage,
                reason: format!(
                    "子目标覆盖率仅 {:.0}%，缺失: {}",
                    goal_coverage * 100.0,
                    missing_goals.join(", ")
                ),
                missing: missing_goals,
            };
        }

        self.consecutive_not_achieved = 0;
        GoalEvaluation {
            achieved: true,
            confidence: goal_coverage.min(0.95),
            reason: format!(
                "目标基本达成: {} 个已验证步骤, {} 个子目标已完成",
                completed_steps,
                sub_goals.len().saturating_sub(missing_goals.len())
            ),
            missing: missing_goals,
        }
    }

    /// 重置计数
    pub fn reset(&mut self) {
        self.consecutive_not_achieved = 0;
    }
}

impl Default for GoalEvaluator {
    fn default() -> Self {
        Self::new(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning_state::{ReasoningContext, ReasoningState};
    use crate::thought_chain::{ThoughtChain, ThoughtStep};

    #[test]
    fn test_empty_chain_not_achieved() {
        let chain = ThoughtChain::new();
        let context = ReasoningContext::new("test goal");
        let mut evaluator = GoalEvaluator::new(3);

        let result = evaluator.evaluate(&chain, &context);
        assert!(!result.achieved);
    }

    #[test]
    fn test_verified_steps_achieved() {
        let mut chain = ThoughtChain::new();
        let mut step = ThoughtStep::new(ReasoningState::Acting, "did something".to_string());
        step.is_verified = true;
        step.observation = Some("success".to_string());
        chain.add_step(step);

        let context = ReasoningContext::new("test");
        let mut evaluator = GoalEvaluator::new(3);

        let result = evaluator.evaluate(&chain, &context);
        assert!(result.achieved);
    }

    #[test]
    fn test_consecutive_not_achieved_force_through() {
        let chain = ThoughtChain::new();
        let context = ReasoningContext::new("impossible goal");
        let mut evaluator = GoalEvaluator::new(1);

        // 第一次 — 未达成
        let r1 = evaluator.evaluate(&chain, &context);
        assert!(!r1.achieved);
        // 第二次 — consecutive=1 >= max=1，强制通过
        let r2 = evaluator.evaluate(&chain, &context);
        assert!(r2.achieved);
        assert!(r2.confidence < 0.6);
    }

    #[test]
    fn test_reset() {
        let mut evaluator = GoalEvaluator::new(2);
        let chain = ThoughtChain::new();
        let context = ReasoningContext::new("test");

        evaluator.evaluate(&chain, &context);
        evaluator.reset();
        // reset 后计数归零
        let result = evaluator.evaluate(&chain, &context);
        assert!(!result.achieved); // 不是强制通过
    }
}
