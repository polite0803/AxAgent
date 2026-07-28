// SPDX-License-Identifier: AGPL-3.0-only

//! 自改进循环通用执行器
//!
//! 在基座层提供与业务无关的回合制迭代策略：执行 → 评估 → 收敛/逃逸判定 → 决策。
//! 业务层通过实现 [`axagent_harness::SelfImprovingRound`] 注入领域评估逻辑，
//! 复用本执行器的最大轮数限制、连续无进展逃逸、收敛阈值等通用机制。
//!
//! 架构分层：
//! - `harness` crate 定义 trait + DTO（权威）
//! - 本模块（`agent` crate）实现通用执行器
//! - 业务 crate（AxInvest/AxOPC/AxSim 等）实现 `SelfImprovingRound` trait

use axagent_harness::self_improving_loop::{
    LoopError, NextAction, RoundEvaluation, RoundResult, SelfImprovingRound,
};
use serde::{Deserialize, Serialize};

/// 自改进循环配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementConfig {
    /// 最大执行轮数（默认 5）
    pub max_rounds: u32,
    /// 收敛阈值（默认 0.85），评估分数高于此值直接 Accept
    pub convergence_threshold: f64,
    /// 连续无进展多少次后 Escalate（默认 3）
    pub escalate_threshold: u32,
}

impl Default for SelfImprovementConfig {
    fn default() -> Self {
        Self { max_rounds: 5, convergence_threshold: 0.85, escalate_threshold: 3 }
    }
}

impl SelfImprovementConfig {
    pub fn new(max_rounds: u32, convergence_threshold: f64, escalate_threshold: u32) -> Self {
        Self {
            max_rounds: max_rounds.max(1),
            convergence_threshold: convergence_threshold.clamp(0.0, 1.0),
            escalate_threshold: escalate_threshold.max(1),
        }
    }
}

/// 自改进循环的最终输出
#[derive(Debug, Clone)]
pub struct FinalOutput {
    /// 最终采纳的输出文本
    pub text: String,
    /// 总执行轮数
    pub total_rounds: u32,
    /// 最终一轮的评估
    pub final_evaluation: RoundEvaluation,
    /// 所有回合的历史记录（按时间顺序）
    pub round_history: Vec<RoundResult>,
}

/// 通用自改进执行器
///
/// 业务无关：只关心 trait 对象 + 配置。收敛检测、无进展逃逸、最大轮数限制
/// 全部由本结构体统一管理；业务层实现 `SelfImprovingRound` 即可复用。
pub struct SelfImprovementExecutor {
    inner: Box<dyn SelfImprovingRound>,
    config: SelfImprovementConfig,
    round_history: Vec<RoundResult>,
    no_progress_count: u32,
}

impl SelfImprovementExecutor {
    pub fn new(inner: Box<dyn SelfImprovingRound>, config: SelfImprovementConfig) -> Self {
        Self { inner, config, round_history: Vec::new(), no_progress_count: 0 }
    }

    /// 运行完整的自改进循环
    ///
    /// 流程：执行 → 评估 → 收敛检测 → 无进展逃逸 → 决策（Accept/Refine/Redirect/Escalate）
    pub async fn run(&mut self, task: &str) -> Result<FinalOutput, LoopError> {
        let mut prev_evaluation: Option<RoundEvaluation> = None;
        let mut best_score = 0.0_f64;

        for round in 1..=self.config.max_rounds {
            // 1. 执行一轮
            let mut result = self
                .inner
                .execute_round(task, prev_evaluation.as_ref())
                .await
                .map_err(LoopError::Internal)?;
            result.round = round;

            // 2. 自我评估
            let evaluation =
                self.inner.evaluate_round(task, &result).await.map_err(LoopError::Internal)?;
            result.evaluation = Some(evaluation.clone());

            self.round_history.push(result);

            // 3. 收敛检测：分数达到阈值直接接受
            if evaluation.score >= self.config.convergence_threshold {
                return Ok(self.build_final_output(round, evaluation));
            }

            // 4. 连续无进展检测
            if evaluation.score > best_score {
                best_score = evaluation.score;
                self.no_progress_count = 0;
            } else {
                self.no_progress_count += 1;
            }

            if self.no_progress_count >= self.config.escalate_threshold {
                return Err(LoopError::Escalated(format!(
                    "No score improvement for {} rounds (best: {best_score})",
                    self.no_progress_count
                )));
            }

            // 5. 决策下一轮动作
            let last_result = self
                .round_history
                .last()
                .expect("round_history should have at least one entry after push");
            match self
                .inner
                .decide_next(task, last_result, &evaluation)
                .await
                .map_err(LoopError::Internal)?
            {
                NextAction::Accept => {
                    return Ok(self.build_final_output(round, evaluation));
                },
                NextAction::Refine { direction } => {
                    // 把 direction 写入 next_direction，与 gaps 一起通过
                    // prev_evaluation 传递给下一轮 execute_round。
                    // 必须 continue，否则末尾的 `prev_evaluation = Some(evaluation)`
                    // 会用无 direction 的 evaluation 覆盖此处构造的值。
                    let mut eval_with_direction = evaluation.clone();
                    eval_with_direction.next_direction = Some(direction);
                    prev_evaluation = Some(eval_with_direction);
                    continue;
                },
                NextAction::Redirect { .. } => {
                    // 换方向：重置 prev_evaluation，让下一轮从零开始感知
                    // （next_direction 也随之丢失，符合 Redirect 语义）
                    prev_evaluation = None;
                    continue;
                },
                NextAction::Escalate { message } => {
                    return Err(LoopError::Escalated(message));
                },
            }
        }

        Err(LoopError::MaxRoundsExceeded(self.config.max_rounds))
    }

    /// 构造最终输出（提取最后一轮的 output）
    fn build_final_output(&self, total_rounds: u32, evaluation: RoundEvaluation) -> FinalOutput {
        let text = self.round_history.last().map(|r| r.output.clone()).unwrap_or_default();
        FinalOutput {
            text,
            total_rounds,
            final_evaluation: evaluation,
            round_history: self.round_history.clone(),
        }
    }

    /// 访问历史回合记录（用于超轮数降级时取最后一轮输出）
    pub fn round_history(&self) -> &[RoundResult] {
        &self.round_history
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::self_improving_loop::{RoundEvaluation, RoundResult, RoundStep};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// 测试用的可编程 SelfImprovingRound 实现
    ///
    /// 通过预设的分数序列 + 行为决策驱动执行器迭代，验证收敛/逃逸/超轮数三条路径。
    struct MockRound {
        /// 每轮返回的分数序列（按调用顺序消费）
        scores: Arc<tokio::sync::Mutex<Vec<f64>>>,
        /// 每轮的输出文本
        outputs: Vec<String>,
        /// decide_next 返回的动作（默认 Refine）
        next_action: NextAction,
        /// execute_round 调用计数
        execute_count: Arc<AtomicU32>,
        /// evaluate_round 调用计数
        evaluate_count: Arc<AtomicU32>,
    }

    impl MockRound {
        fn new(scores: Vec<f64>, outputs: Vec<String>) -> Self {
            Self {
                scores: Arc::new(tokio::sync::Mutex::new(scores)),
                outputs,
                next_action: NextAction::Refine { direction: "test direction".to_string() },
                execute_count: Arc::new(AtomicU32::new(0)),
                evaluate_count: Arc::new(AtomicU32::new(0)),
            }
        }
    }

    #[async_trait::async_trait]
    impl SelfImprovingRound for MockRound {
        async fn execute_round(
            &mut self,
            _task: &str,
            _prev_evaluation: Option<&RoundEvaluation>,
        ) -> Result<RoundResult, Box<dyn std::error::Error + Send>> {
            self.execute_count.fetch_add(1, Ordering::SeqCst);
            let idx = (self.execute_count.load(Ordering::SeqCst) - 1) as usize;
            let output = self.outputs.get(idx).cloned().unwrap_or_else(|| format!("output-{idx}"));
            Ok(RoundResult {
                round: 0,
                output,
                evaluation: None,
                trace: vec![RoundStep {
                    index: 0,
                    kind: "think".to_string(),
                    summary: "mock step".to_string(),
                    tokens_used: 100,
                }],
            })
        }

        async fn evaluate_round(
            &self,
            _task: &str,
            _result: &RoundResult,
        ) -> Result<RoundEvaluation, Box<dyn std::error::Error + Send>> {
            self.evaluate_count.fetch_add(1, Ordering::SeqCst);
            let mut scores = self.scores.lock().await;
            let score = scores.first().copied().unwrap_or(0.5);
            if !scores.is_empty() {
                scores.remove(0);
            }
            Ok(RoundEvaluation {
                score,
                confidence: 0.9,
                gaps: vec!["gap1".to_string()],
                strengths: vec![],
                raw_assessment: format!("score={score}"),
                next_direction: None,
            })
        }

        async fn decide_next(
            &self,
            _task: &str,
            _result: &RoundResult,
            _evaluation: &RoundEvaluation,
        ) -> Result<NextAction, Box<dyn std::error::Error + Send>> {
            Ok(self.next_action.clone())
        }
    }

    #[tokio::test]
    async fn test_converges_on_high_score() {
        // 第一轮就达到 0.9，应立即收敛
        let mock = MockRound::new(vec![0.9], vec!["high quality output".to_string()]);
        let exec_count = mock.execute_count.clone();
        let mut executor =
            SelfImprovementExecutor::new(Box::new(mock), SelfImprovementConfig::default());

        let result = executor.run("test task").await.expect("should converge");
        assert_eq!(result.total_rounds, 1);
        assert_eq!(result.text, "high quality output");
        assert!(result.final_evaluation.score >= 0.85);
        assert_eq!(exec_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_exceeds_max_rounds() {
        // 分数始终低于阈值且无 Refine 改进，应达到 max_rounds 后报错
        let config = SelfImprovementConfig::new(3, 0.95, 10);
        let mock = MockRound::new(
            vec![0.3, 0.4, 0.5],
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        let mut executor = SelfImprovementExecutor::new(Box::new(mock), config);

        let err = executor.run("test").await.expect_err("should exceed max rounds");
        match err {
            LoopError::MaxRoundsExceeded(n) => assert_eq!(n, 3),
            other => panic!("expected MaxRoundsExceeded, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_escalates_on_no_progress() {
        // 分数持续不增长，应在 escalate_threshold 次后 Escalate
        let config = SelfImprovementConfig::new(10, 0.95, 3);
        // 分数序列：0.5, 0.5, 0.5 — 第一轮 best=0.5；第二轮 score=0.5 不大于 0.5，no_progress=1
        // 第三轮 score=0.5 不大于 0.5，no_progress=2；第四轮同理 no_progress=3 触发
        let mock = MockRound::new(vec![0.5, 0.5, 0.5, 0.5], vec!["x".to_string()]);
        let mut executor = SelfImprovementExecutor::new(Box::new(mock), config);

        let err = executor.run("test").await.expect_err("should escalate");
        match err {
            LoopError::Escalated(msg) => assert!(msg.contains("No score improvement")),
            other => panic!("expected Escalated, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_accept_action_terminates() {
        // decide_next 返回 Accept 时应立即结束
        let mut mock = MockRound::new(vec![0.3], vec!["accept me".to_string()]);
        mock.next_action = NextAction::Accept;
        let mut executor =
            SelfImprovementExecutor::new(Box::new(mock), SelfImprovementConfig::default());

        let result = executor.run("test").await.expect("should accept");
        assert_eq!(result.total_rounds, 1);
        assert_eq!(result.text, "accept me");
    }

    #[tokio::test]
    async fn test_escalate_action_terminates() {
        let mut mock = MockRound::new(vec![0.3], vec!["escalate me".to_string()]);
        mock.next_action = NextAction::Escalate { message: "human needed".to_string() };
        let mut executor =
            SelfImprovementExecutor::new(Box::new(mock), SelfImprovementConfig::default());

        let err = executor.run("test").await.expect_err("should escalate");
        match err {
            LoopError::Escalated(msg) => assert_eq!(msg, "human needed"),
            other => panic!("expected Escalated, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_redirect_resets_progress() {
        // Redirect 重置 prev_evaluation：验证下一轮 execute_round 收到的
        // prev_evaluation 是 None（而不是上一轮的 evaluation）。
        // 通过自定义 mock 在 execute_round 中观察 prev_evaluation 是否为 None。
        let observed_prev_evaluation_is_none = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed_clone = observed_prev_evaluation_is_none.clone();

        struct RedirectObserverRound {
            call_count: AtomicU32,
            observed: Arc<std::sync::atomic::AtomicBool>,
        }

        #[async_trait::async_trait]
        impl SelfImprovingRound for RedirectObserverRound {
            async fn execute_round(
                &mut self,
                _task: &str,
                prev_evaluation: Option<&RoundEvaluation>,
            ) -> Result<RoundResult, Box<dyn std::error::Error + Send>> {
                let n = self.call_count.fetch_add(1, Ordering::SeqCst);
                // 第 2 次调用（n==1）发生在 Redirect 之后，prev_evaluation 必须是 None
                if n == 1 {
                    self.observed.store(prev_evaluation.is_none(), Ordering::SeqCst);
                }
                Ok(RoundResult {
                    round: 0,
                    output: format!("round-{n}"),
                    evaluation: None,
                    trace: vec![],
                })
            }

            async fn evaluate_round(
                &self,
                _task: &str,
                _result: &RoundResult,
            ) -> Result<RoundEvaluation, Box<dyn std::error::Error + Send>> {
                Ok(RoundEvaluation {
                    score: 0.4,
                    confidence: 0.7,
                    gaps: vec![],
                    strengths: vec![],
                    raw_assessment: String::new(),
                    next_direction: None,
                })
            }

            async fn decide_next(
                &self,
                _task: &str,
                _result: &RoundResult,
                _evaluation: &RoundEvaluation,
            ) -> Result<NextAction, Box<dyn std::error::Error + Send>> {
                // 第 1 轮 Redirect，第 2 轮 Accept（结束循环）
                let n = self.call_count.load(Ordering::SeqCst);
                if n <= 1 {
                    Ok(NextAction::Redirect { reason: "wrong direction".to_string() })
                } else {
                    Ok(NextAction::Accept)
                }
            }
        }

        let mock =
            RedirectObserverRound { call_count: AtomicU32::new(0), observed: observed_clone };
        let mut executor =
            SelfImprovementExecutor::new(Box::new(mock), SelfImprovementConfig::default());

        // 第 1 轮：execute → evaluate(0.4) → decide_next=Redirect → prev_evaluation=None
        // 第 2 轮：execute (prev=None) → evaluate(0.4) → decide_next=Accept → 结束
        let _ = executor.run("test").await.expect("should accept on round 2");

        assert!(
            observed_prev_evaluation_is_none.load(Ordering::SeqCst),
            "Redirect 后下一轮 execute_round 的 prev_evaluation 必须为 None"
        );
    }

    #[tokio::test]
    async fn test_round_history_tracking() {
        let mock = MockRound::new(vec![0.9], vec!["first".to_string()]);
        let mut executor =
            SelfImprovementExecutor::new(Box::new(mock), SelfImprovementConfig::default());

        let result = executor.run("test").await.expect("should converge");
        assert_eq!(result.round_history.len(), 1);
        assert_eq!(result.round_history[0].output, "first");
        assert!(result.round_history[0].evaluation.is_some());
    }

    #[test]
    fn test_config_validation() {
        let config = SelfImprovementConfig::new(0, 1.5, 0);
        assert_eq!(config.max_rounds, 1);
        assert_eq!(config.convergence_threshold, 1.0);
        assert_eq!(config.escalate_threshold, 1);

        let config = SelfImprovementConfig::new(5, 0.85, 3);
        assert_eq!(config.max_rounds, 5);
        assert_eq!(config.convergence_threshold, 0.85);
        assert_eq!(config.escalate_threshold, 3);
    }
}
