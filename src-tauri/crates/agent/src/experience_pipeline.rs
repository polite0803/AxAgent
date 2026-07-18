// SPDX-License-Identifier: AGPL-3.0-only

//! ExperiencePipeline — 反馈→优化自动桥接
//!
//! 负责将 Reflection 产出和用户反馈自动转换为 Experience 并写入
//! RL Optimizer 的 ExperiencePool，打通反馈层到优化层的数据流。
//!
//! 数据流：Reflection → ExperiencePipeline → RL Optimizer ExperiencePool
//!        FeedbackRecord → ExperiencePipeline → RL Optimizer ExperiencePool

use crate::reflector::Reflection;
use crate::rl_optimizer::{Experience, RLOptimizer, TaskState, ThresholdScheduler, ToolSelection};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 将上游反馈/反思信号转换为 RL 训练经验的管道。
pub struct ExperiencePipeline {
    rl_optimizer: Arc<RwLock<RLOptimizer>>,
    /// 统计已处理的 Reflection 数量
    reflections_processed: u64,
    /// 统计已处理的反馈数量
    feedback_processed: u64,
    /// 有状态阈值调度器（追踪增量，触发训练）
    train_scheduler: ThresholdScheduler,
}

impl ExperiencePipeline {
    pub fn new(rl_optimizer: Arc<RwLock<RLOptimizer>>, auto_train_threshold: usize) -> Self {
        let threshold = if auto_train_threshold == 0 {
            100
        } else {
            auto_train_threshold
        };

        Self {
            rl_optimizer,
            reflections_processed: 0,
            feedback_processed: 0,
            train_scheduler: ThresholdScheduler::new(threshold, threshold * 10),
        }
    }

    /// 将 Reflection 转换为 Experience 并写入 ExperiencePool。
    ///
    /// 转换规则：
    /// - quality_score → reward（归一化到 -1..1）
    /// - task_id → state.task_id
    /// - error_patterns / reusable_patterns → state.context
    /// - improvement_suggestions → action.reasoning
    pub async fn process_reflection(&mut self, reflection: &Reflection) -> Experience {
        // 将 quality_score (1-10) 映射为 reward (-1.0 .. 1.0)
        let reward = (reflection.quality_score as f32 - 5.5) / 5.0;

        let mut context = HashMap::new();
        context.insert("error_patterns".to_string(), serde_json::json!(reflection.error_patterns));
        context.insert(
            "reusable_patterns".to_string(),
            serde_json::json!(reflection.reusable_patterns),
        );
        if let Some(ref metrics) = reflection.quality_metrics {
            context.insert(
                "quality_metrics".to_string(),
                serde_json::json!({
                    "overall_score": metrics.overall_weighted_score,
                    "task_success": metrics.task_success_score,
                    "tool_efficiency": metrics.tool_efficiency_score,
                }),
            );
        }

        let state = TaskState {
            task_id: reflection.task_id.clone(),
            task_type: "reflection".to_string(),
            context,
            available_tools: Vec::new(),
            completed_tools: Vec::new(),
            error_count: reflection.error_patterns.len() as u32,
            elapsed_ms: 0,
        };

        let next_state = TaskState {
            task_id: reflection.task_id.clone(),
            task_type: "reflection".to_string(),
            context: HashMap::new(),
            available_tools: Vec::new(),
            completed_tools: Vec::new(),
            error_count: 0,
            elapsed_ms: 0,
        };

        let reasoning = if reflection.improvement_suggestions.is_empty() {
            reflection.overall_summary.clone()
        } else {
            reflection.improvement_suggestions.join("; ")
        };

        let experience = Experience {
            id: uuid::Uuid::new_v4().to_string(),
            state,
            action: ToolSelection {
                tool_id: "reflection".to_string(),
                tool_name: "Reflection Learning".to_string(),
                parameters: HashMap::new(),
                reasoning,
            },
            reward,
            next_state,
            done: reward > 0.3,
            timestamp: reflection.timestamp,
        };

        {
            let mut opt = self.rl_optimizer.write().await;
            opt.record_experience(experience.clone());
        }

        self.reflections_processed += 1;
        tracing::info!(
            "[ExperiencePipeline] processed reflection #{}, reward={:.2}",
            self.reflections_processed,
            reward
        );

        // 检查是否需要自动触发训练
        self.check_auto_train().await;

        experience
    }

    /// 将用户反馈（来自 FeedbackRecord）转换为 Experience 并写入 ExperiencePool。
    ///
    /// 用户 rating (1-5) 映射为 reward：
    /// - 1 → -1.0
    /// - 2 → -0.5
    /// - 3 →  0.0
    /// - 4 →  0.5
    /// - 5 →  1.0
    pub async fn process_feedback(
        &mut self,
        trace_id: &str,
        rating: u8,
        comment: Option<&str>,
    ) -> Experience {
        let rating = rating.clamp(1, 5);
        let reward = match rating {
            1 => -1.0,
            2 => -0.5,
            3 => 0.0,
            4 => 0.5,
            5 => 1.0,
            _ => 0.0, // unreachable
        };

        let mut context = HashMap::new();
        context.insert("rating".to_string(), serde_json::json!(rating));
        if let Some(c) = comment {
            context.insert("comment".to_string(), serde_json::json!(c));
        }

        let state = TaskState {
            task_id: format!("feedback:{}", trace_id),
            task_type: "user_feedback".to_string(),
            context,
            available_tools: Vec::new(),
            completed_tools: Vec::new(),
            error_count: if rating <= 2 { 1 } else { 0 },
            elapsed_ms: 0,
        };

        let next_state = TaskState {
            task_id: format!("feedback:{}", trace_id),
            task_type: "user_feedback".to_string(),
            context: HashMap::new(),
            available_tools: Vec::new(),
            completed_tools: Vec::new(),
            error_count: 0,
            elapsed_ms: 0,
        };

        let reasoning = format!(
            "User rated trace {} as {}/5{}",
            trace_id,
            rating,
            comment.map(|c| format!(": {}", c)).unwrap_or_default()
        );

        let experience = Experience {
            id: uuid::Uuid::new_v4().to_string(),
            state,
            action: ToolSelection {
                tool_id: "user_feedback".to_string(),
                tool_name: "User Feedback".to_string(),
                parameters: HashMap::new(),
                reasoning,
            },
            reward,
            next_state,
            done: true,
            timestamp: chrono::Utc::now(),
        };

        {
            let mut opt = self.rl_optimizer.write().await;
            opt.record_experience(experience.clone());
        }

        self.feedback_processed += 1;
        tracing::info!(
            "[ExperiencePipeline] processed feedback #{}, rating={}, reward={:.2}",
            self.feedback_processed,
            rating,
            reward
        );

        // 检查是否需要自动触发训练
        self.check_auto_train().await;

        experience
    }

    /// 自动训练调度：委托给 ThresholdScheduler，当增量或池大小达到阈值时触发 train()。
    async fn check_auto_train(&mut self) {
        let mut opt = self.rl_optimizer.write().await;
        match self.train_scheduler.check_and_train(&mut opt) {
            Some(Ok(stats)) => {
                tracing::info!(
                    "[ExperiencePipeline] auto-train completed: train_count={}, episodes={}, avg_reward={:.3}",
                    self.train_scheduler.train_count(),
                    stats.episodes_completed,
                    stats.avg_reward
                );
            },
            Some(Err(e)) => {
                tracing::warn!("[ExperiencePipeline] auto-train failed: {}", e);
            },
            None => { /* 未达到阈值，不触发 */ },
        }
    }

    /// 基于反馈评分构造简化 Reflection 并调用 process_reflection。
    ///
    /// 作为 `Reflector::reflect()`（目前零调用）的轻量替代：
    /// 用反馈评分反向构造 Reflection 字段，让 Reflection→Experience
    /// 数据链路至少在后端跑通，为后续接入完整 Reflector 保留数据格式兼容。
    pub async fn bridge_feedback_to_reflection(
        &mut self,
        trace_id: &str,
        rating: u8,
        comment: Option<&str>,
    ) {
        let quality_score = match rating {
            1 | 2 => 3u8,
            3 => 5u8,
            4 => 7u8,
            5 => 9u8,
            _ => 5u8,
        };
        let error_patterns = if rating <= 2 {
            vec![format!("low_rating_from_feedback:{}", trace_id)]
        } else {
            Vec::new()
        };
        let improvement_suggestions =
            comment.map(|c| vec![format!("feedback_suggestion: {}", c)]).unwrap_or_default();
        let overall_summary =
            format!("Feedback-based reflection for trace {} (rating={}/5)", trace_id, rating);

        let reflection = crate::reflector::Reflection {
            task_id: format!("feedback:{}", trace_id),
            timestamp: chrono::Utc::now(),
            quality_score,
            quality_analysis: overall_summary.clone(),
            efficiency_analysis: String::new(),
            error_patterns,
            reusable_patterns: Vec::new(),
            knowledge_suggestions: Vec::new(),
            improvement_suggestions,
            overall_summary,
            quality_metrics: None,
            metadata: None,
        };

        self.process_reflection(&reflection).await;
    }

    pub fn stats(&self) -> PipelineStats {
        let pool_size = self
            .rl_optimizer
            .try_read()
            .map(|opt| opt.experience_pool.experiences.len())
            .unwrap_or(0);

        PipelineStats {
            reflections_processed: self.reflections_processed,
            feedback_processed: self.feedback_processed,
            pool_size,
            train_count: self.train_scheduler.train_count(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub reflections_processed: u64,
    pub feedback_processed: u64,
    pub pool_size: usize,
    pub train_count: u64,
}

#[cfg(test)]
mod tests {
    //! P1-5 修复:补齐 ExperiencePipeline 单元测试。
    //! 覆盖 process_reflection / process_feedback / bridge_feedback_to_reflection / stats 主路径。

    use super::*;
    use crate::reflector::Reflection;

    fn make_pipeline(threshold: usize) -> ExperiencePipeline {
        let optimizer = Arc::new(RwLock::new(RLOptimizer::new(
            "test_optimizer".to_string(),
            "Test RL Optimizer".to_string(),
        )));
        ExperiencePipeline::new(optimizer, threshold)
    }

    fn make_reflection(quality_score: u8, task_id: &str) -> Reflection {
        Reflection {
            task_id: task_id.to_string(),
            timestamp: chrono::Utc::now(),
            quality_score,
            quality_analysis: String::new(),
            efficiency_analysis: String::new(),
            error_patterns: vec!["timeout".to_string()],
            reusable_patterns: vec!["search->read".to_string()],
            knowledge_suggestions: Vec::new(),
            improvement_suggestions: vec!["add retry".to_string()],
            overall_summary: "test summary".to_string(),
            quality_metrics: None,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn test_process_reflection_writes_to_pool() {
        let mut pipeline = make_pipeline(100);
        let reflection = make_reflection(8, "task-1");

        let experience = pipeline.process_reflection(&reflection).await;

        // 验证 reward 映射 (quality_score=8 → (8-5.5)/5 = 0.5)
        assert!((experience.reward - 0.5).abs() < 0.01);
        assert_eq!(experience.state.task_id, "task-1");
        assert_eq!(experience.action.tool_id, "reflection");

        // 验证 ExperiencePool 已写入
        let stats = pipeline.stats();
        assert_eq!(stats.reflections_processed, 1);
        assert_eq!(stats.pool_size, 1);
    }

    #[tokio::test]
    async fn test_process_feedback_reward_mapping() {
        let mut pipeline = make_pipeline(100);

        // rating 1 → -1.0
        let exp1 = pipeline.process_feedback("trace-1", 1, None).await;
        assert!((exp1.reward - (-1.0)).abs() < 0.01);

        // rating 3 → 0.0
        let exp3 = pipeline.process_feedback("trace-3", 3, None).await;
        assert!(exp3.reward.abs() < 0.01);

        // rating 5 → 1.0
        let exp5 = pipeline.process_feedback("trace-5", 5, Some("great")).await;
        assert!((exp5.reward - 1.0).abs() < 0.01);
        assert!(exp5.action.reasoning.contains("great"));

        // rating 越界(0 / 9)应被 clamp 到 [1,5]
        let exp_clamp = pipeline.process_feedback("trace-clamp", 0, None).await;
        // rating 0 → clamp 到 1 → reward = -1.0
        assert!((exp_clamp.reward - (-1.0)).abs() < 0.01);

        let stats = pipeline.stats();
        assert_eq!(stats.feedback_processed, 4);
        assert_eq!(stats.pool_size, 4);
    }

    #[tokio::test]
    async fn test_bridge_feedback_to_reflection() {
        let mut pipeline = make_pipeline(100);

        // rating=2 → quality_score=3,reward = (3-5.5)/5 = -0.5
        pipeline.bridge_feedback_to_reflection("trace-low", 2, None).await;
        let stats = pipeline.stats();
        assert_eq!(stats.reflections_processed, 1);
        assert_eq!(stats.pool_size, 1);

        // rating=5 → quality_score=9,reward = (9-5.5)/5 = 0.7
        pipeline.bridge_feedback_to_reflection("trace-high", 5, Some("nice")).await;
        let stats = pipeline.stats();
        assert_eq!(stats.reflections_processed, 2);
        assert_eq!(stats.pool_size, 2);
    }

    #[tokio::test]
    async fn test_auto_train_triggers_at_threshold() {
        // 阈值设为 3,3 条经验后应触发训练
        let mut pipeline = make_pipeline(3);

        for i in 0..3 {
            let r = make_reflection(7, &format!("task-{i}"));
            pipeline.process_reflection(&r).await;
        }

        let stats = pipeline.stats();
        // 训练可能成功或失败(取决于 RLOptimizer 内部),但 train_count 应 >=1
        assert!(
            stats.train_count >= 1,
            "auto-train should have triggered at threshold, train_count={}",
            stats.train_count
        );
    }

    #[tokio::test]
    async fn test_stats_initial_state() {
        let pipeline = make_pipeline(100);
        let stats = pipeline.stats();
        assert_eq!(stats.reflections_processed, 0);
        assert_eq!(stats.feedback_processed, 0);
        assert_eq!(stats.pool_size, 0);
        assert_eq!(stats.train_count, 0);
    }

    #[tokio::test]
    async fn test_threshold_zero_defaults_to_100() {
        // threshold=0 应自动回退到 100,不会立即触发训练
        let mut pipeline = make_pipeline(0);
        let r = make_reflection(7, "task-1");
        pipeline.process_reflection(&r).await;
        let stats = pipeline.stats();
        assert_eq!(stats.train_count, 0, "should not trigger train with single experience");
    }
}
