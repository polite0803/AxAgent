// SPDX-License-Identifier: AGPL-3.0-only

//! FeedbackOrchestrator — 反馈驱动自动优化调度器
//!
//! 监听用户反馈事件（FeedbackRecord 写入后触发），根据反馈模式自动决策：
//! - 负面反馈（rating <= 2）累积到阈值 → 触发 RL 训练
//! - 正向反馈（rating >= 4）累积时 → 触发技能进化评估
//! - 定期检查经验池大小，超过阈值触发训练

use std::sync::atomic::{AtomicUsize, Ordering};

/// 反馈聚合后的动作指令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorAction {
    /// 无操作。
    None,
    /// 应触发 RL 训练（负面反馈累积到阈值）。
    TriggerRLTraining {
        reason: String,
        negative_count: usize,
    },
    /// 应触发技能进化（正向反馈累积到阈值）。
    TriggerSkillEvolution {
        reason: String,
        positive_count: usize,
    },
    /// 经验池大小检查触发训练。
    TriggerPoolSizeCheck { pool_size: usize },
}

/// 反馈驱动的自动优化调度器。
///
/// 核心职责：
/// - 累积正面/负面反馈计数
/// - 达到阈值时触发对应的优化动作
/// - 支持可选定时检查
/// - 桥接 tracer FeedbackRecord → orchestrator（process_feedback_record）
pub struct FeedbackOrchestrator {
    /// 负反馈阈值（默认 5）
    negative_threshold: usize,
    /// 正反馈阈值（默认 10）
    positive_threshold: usize,
    /// 负面反馈累计（评级 1-2）
    negative_count: AtomicUsize,
    /// 正面反馈累计（评级 4-5）
    positive_count: AtomicUsize,
    /// 总反馈数
    total_feedback: AtomicUsize,
    /// 经验池规模检查阈值（默认 100）
    pool_size_threshold: usize,
    /// 已处理的反馈记录 trace_id 集合（用于去重）
    seen_traces: std::sync::Mutex<std::collections::HashSet<String>>,
}

impl FeedbackOrchestrator {
    pub fn new() -> Self {
        Self {
            negative_threshold: 5,
            positive_threshold: 10,
            negative_count: AtomicUsize::new(0),
            positive_count: AtomicUsize::new(0),
            total_feedback: AtomicUsize::new(0),
            pool_size_threshold: 100,
            seen_traces: std::sync::Mutex::new(std::collections::HashSet::new()),
        }
    }

    /// 自定义阈值。
    pub fn with_thresholds(
        mut self,
        negative_threshold: usize,
        positive_threshold: usize,
        pool_size_threshold: usize,
    ) -> Self {
        self.negative_threshold = negative_threshold;
        self.positive_threshold = positive_threshold;
        self.pool_size_threshold = pool_size_threshold;
        self
    }

    /// 记录一条用户反馈，返回建议动作。
    ///
    /// rating: 1-5（1 最差，5 最好）
    pub fn record_feedback(&self, rating: u8) -> OrchestratorAction {
        let rating = rating.clamp(1, 5);
        self.total_feedback.fetch_add(1, Ordering::Relaxed);

        match rating {
            1 | 2 => {
                let prev = self.negative_count.fetch_add(1, Ordering::Relaxed) + 1;
                tracing::info!(
                    "[FeedbackOrchestrator] negative feedback #{}, total negative={}",
                    self.total_feedback.load(Ordering::Relaxed),
                    prev
                );

                if prev >= self.negative_threshold {
                    // 达到负反馈阈值，重置计数器并触发 RL 训练
                    self.negative_count.store(0, Ordering::Relaxed);
                    OrchestratorAction::TriggerRLTraining {
                        reason: axagent_core::i18n::fmt_msg_with(
                            axagent_core::i18n::I18nKey::AgentNegativeFeedbackThreshold,
                            &prev.to_string(),
                            &self.negative_threshold.to_string(),
                        ),
                        negative_count: prev,
                    }
                } else {
                    OrchestratorAction::None
                }
            },
            4 | 5 => {
                let prev = self.positive_count.fetch_add(1, Ordering::Relaxed) + 1;
                tracing::info!(
                    "[FeedbackOrchestrator] positive feedback #{}, total positive={}",
                    self.total_feedback.load(Ordering::Relaxed),
                    prev
                );

                if prev >= self.positive_threshold {
                    self.positive_count.store(0, Ordering::Relaxed);
                    OrchestratorAction::TriggerSkillEvolution {
                        reason: axagent_core::i18n::fmt_msg_with(
                            axagent_core::i18n::I18nKey::AgentPositiveFeedbackThreshold,
                            &prev.to_string(),
                            &self.positive_threshold.to_string(),
                        ),
                        positive_count: prev,
                    }
                } else {
                    OrchestratorAction::None
                }
            },
            _ => OrchestratorAction::None, // rating == 3 中性，不触发任何动作
        }
    }

    /// 检查经验池是否超过阈值（供定时调用）。
    pub fn check_pool_size(&self, pool_size: usize) -> OrchestratorAction {
        if pool_size >= self.pool_size_threshold {
            OrchestratorAction::TriggerPoolSizeCheck { pool_size }
        } else {
            OrchestratorAction::None
        }
    }

    /// 处理来自 tracer 的 FeedbackRecord（桥接 tracer → orchestrator）。
    ///
    /// 解析字符串格式的 rating，去重（同 trace_id 多次提交只计首次），
    /// 然后委托给 `record_feedback`。
    ///
    /// 返回解析后的 rating 值和对应的 OrchestratorAction。
    /// 若 rating 解析失败或已处理过该 trace，返回 `None`。
    pub fn process_feedback_record(
        &self,
        trace_id: &str,
        rating_str: &str,
    ) -> Option<(u8, OrchestratorAction)> {
        // 去重：同 trace_id 只处理一次
        {
            let mut seen = self.seen_traces.lock().unwrap();
            if !seen.insert(trace_id.to_string()) {
                tracing::debug!("[FeedbackOrchestrator] skipping duplicate trace_id={}", trace_id);
                return None;
            }
        }

        // 解析 rating 字符串
        let rating: u8 = rating_str.parse().ok()?;
        let rating = rating.clamp(1, 5);

        let action = self.record_feedback(rating);
        Some((rating, action))
    }

    /// 批量处理 FeedbackRecord 列表（从 tracer 的 FEEDBACK_STORAGE 拉取后调用）。
    ///
    /// 返回触发的动作列表（仅包含 Trigger* 变体，不含 None）。
    pub fn process_feedback_batch(
        &self,
        records: &[FeedbackRecordInput],
    ) -> Vec<FeedbackBatchResult> {
        let mut results = Vec::new();
        for rec in records {
            if let Some((rating, action)) = self.process_feedback_record(&rec.trace_id, &rec.rating)
            {
                results.push(FeedbackBatchResult {
                    trace_id: rec.trace_id.clone(),
                    rating,
                    action,
                    comment: rec.comment.clone(),
                });
            }
        }
        results
    }

    /// 获取已处理 trace_id 数量（去重计数）。
    pub fn seen_count(&self) -> usize {
        self.seen_traces.lock().unwrap().len()
    }

    /// 重置负反馈计数器（RL 训练完成后调用）。
    pub fn reset_negatives(&self) {
        self.negative_count.store(0, Ordering::Relaxed);
    }

    /// 重置正反馈计数器（技能进化完成后调用）。
    pub fn reset_positives(&self) {
        self.positive_count.store(0, Ordering::Relaxed);
    }

    /// 获取当前统计信息。
    pub fn stats(&self) -> OrchestratorStats {
        OrchestratorStats {
            negative_count: self.negative_count.load(Ordering::Relaxed),
            positive_count: self.positive_count.load(Ordering::Relaxed),
            total_feedback: self.total_feedback.load(Ordering::Relaxed),
            negative_threshold: self.negative_threshold,
            positive_threshold: self.positive_threshold,
            pool_size_threshold: self.pool_size_threshold,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrchestratorStats {
    pub negative_count: usize,
    pub positive_count: usize,
    pub total_feedback: usize,
    pub negative_threshold: usize,
    pub positive_threshold: usize,
    pub pool_size_threshold: usize,
}

impl Default for FeedbackOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// 基于反馈评级的分类（供外部使用）。
pub fn classify_feedback_rating(rating: u8) -> FeedbackCategory {
    match rating.clamp(1, 5) {
        1 | 2 => FeedbackCategory::Negative,
        3 => FeedbackCategory::Neutral,
        4 | 5 => FeedbackCategory::Positive,
        _ => FeedbackCategory::Neutral,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackCategory {
    Negative,
    Neutral,
    Positive,
}

/// tracer 反馈记录的输入表示（桥梁类型，避免循环依赖）。
#[derive(Debug, Clone)]
pub struct FeedbackRecordInput {
    pub trace_id: String,
    pub rating: String,
    pub comment: Option<String>,
}

/// 批量处理反馈后的单条结果。
#[derive(Debug, Clone)]
pub struct FeedbackBatchResult {
    pub trace_id: String,
    pub rating: u8,
    pub action: OrchestratorAction,
    pub comment: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negative_threshold_triggers_rl() {
        let orchestrator = FeedbackOrchestrator::new();
        // rating=1 五次
        for _ in 0..4 {
            assert_eq!(orchestrator.record_feedback(1), OrchestratorAction::None);
        }
        let action = orchestrator.record_feedback(1);
        assert_eq!(
            action,
            OrchestratorAction::TriggerRLTraining {
                reason: "累积 5 条负面反馈（评级 1-2），已达到阈值 5".to_string(),
                negative_count: 5,
            }
        );
    }

    #[test]
    fn test_positive_threshold_triggers_evolution() {
        let orchestrator = FeedbackOrchestrator::new();
        for _ in 0..9 {
            assert_eq!(orchestrator.record_feedback(4), OrchestratorAction::None);
        }
        let action = orchestrator.record_feedback(5);
        assert_eq!(
            action,
            OrchestratorAction::TriggerSkillEvolution {
                reason: "累积 10 条正向反馈（评级 4-5），已达到阈值 10".to_string(),
                positive_count: 10,
            }
        );
    }

    #[test]
    fn test_neutral_no_trigger() {
        let orchestrator = FeedbackOrchestrator::new();
        for _ in 0..20 {
            assert_eq!(orchestrator.record_feedback(3), OrchestratorAction::None);
        }
    }

    #[test]
    fn test_pool_size_check() {
        let orchestrator = FeedbackOrchestrator::new();
        assert_eq!(
            orchestrator.check_pool_size(150),
            OrchestratorAction::TriggerPoolSizeCheck { pool_size: 150 }
        );
        assert_eq!(orchestrator.check_pool_size(50), OrchestratorAction::None);
    }
}
