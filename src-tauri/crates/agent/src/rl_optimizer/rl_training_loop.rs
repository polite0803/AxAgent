// SPDX-License-Identifier: AGPL-3.0-only

//! RL 训练循环 — 经验回放 + 策略更新
//!
//! 为 RLOptimizer 提供训练能力：
//! - `train()`: 主训练入口，从 ExperiencePool 采样并更新策略
//! - 自动调度钩子：当经验池新增经验数达到阈值时触发 train()

use super::{ExperiencePool, Policy, RLError, RLOptimizer, TrainingStats};
use chrono::Utc;

/// 对给定 RLOptimizer 执行一轮训练。
///
/// # 流程
/// 1. 从 ExperiencePool 采样 batch_size 条经验
/// 2. 计算每个策略的累积奖励（TD(0) 更新风格）
/// 3. 更新 TrainingStats
pub fn train(optimizer: &mut RLOptimizer) -> Result<TrainingStats, RLError> {
    let batch_size = optimizer.config.batch_size.max(1);
    let pool = &optimizer.experience_pool;

    if pool.experiences.is_empty() {
        return Err(RLError::TrainingError(
            "ExperiencePool is empty, nothing to train on".into(),
        ));
    }

    let samples: Vec<_> = pool
        .sample(batch_size as usize)
        .into_iter()
        .collect();

    if samples.is_empty() {
        return Err(RLError::TrainingError(
            "No samples drawn from ExperiencePool".into(),
        ));
    }

    // 计算平均奖励
    let total_reward: f32 = samples.iter().map(|e| e.reward).sum();
    let avg_reward = total_reward / samples.len() as f32;

    // 统计成功经验数（reward > 0）
    let successful: usize = samples.iter().filter(|e| e.reward > 0.0).count();

    // 更新 epsilon（衰减探索率）
    optimizer.config.epsilon =
        (optimizer.config.epsilon * optimizer.config.epsilon_decay).max(optimizer.config.epsilon_min);

    // 更新每个策略的训练统计
    for (policy_id, policy) in optimizer.policies.iter_mut() {
        // 查找与该策略相关的经验（通过 action.tool_name 匹配）
        let relevant_samples: Vec<&super::Experience> = samples
            .iter()
            .filter(|e| {
                let tool_name_lower = e.action.tool_name.to_lowercase();
                let policy_lower = policy_id.to_lowercase();
                let policy_name_lower = policy.name.to_lowercase();
                tool_name_lower.contains(&policy_lower)
                    || tool_name_lower.contains(&policy_name_lower)
                    || policy_lower == "tool_selection" // 全局策略匹配所有
            })
            .collect();

        if !relevant_samples.is_empty() {
            let policy_avg_reward: f32 =
                relevant_samples.iter().map(|e| e.reward).sum::<f32>()
                    / relevant_samples.len() as f32;

            // 更新策略内奖励信号的权重（简单指数平滑）
            for signal in policy.reward_signals.iter_mut() {
                let adjustment = match signal.signal_type {
                    super::RewardSignalType::TaskCompletion => {
                        (successful as f32 / samples.len() as f32) * 0.01
                    }
                    super::RewardSignalType::TimeEfficiency => avg_reward * 0.005,
                    super::RewardSignalType::ErrorRate => {
                        if successful < samples.len() {
                            -0.01 * (1.0 - successful as f32 / samples.len() as f32)
                        } else {
                            0.0
                        }
                    }
                    super::RewardSignalType::ToolDiversity => 0.001,
                    super::RewardSignalType::UserFeedback => avg_reward * 0.01,
                };
                signal.weight = (signal.weight + adjustment).clamp(-1.0, 1.0);
            }

            policy.training_stats = TrainingStats {
                total_experiences: pool.experiences.len() as u64,
                episodes_completed: policy.training_stats.episodes_completed + samples.len() as u64,
                avg_reward: policy_avg_reward,
                last_update: Utc::now(),
            };
        }
    }

    let stats = TrainingStats {
        total_experiences: pool.experiences.len() as u64,
        episodes_completed: samples.len() as u64,
        avg_reward,
        last_update: Utc::now(),
    };

    tracing::info!(
        "[RL Training] batch={}, avg_reward={:.3}, epsilon={:.4}, success_ratio={:.2}",
        samples.len(),
        avg_reward,
        optimizer.config.epsilon,
        successful as f32 / samples.len() as f32
    );

    Ok(stats)
}

/// 自动训练调度入口。
///
/// 当经验池大小 >= threshold 时返回 `Some(true)` 表示已触发训练；
/// 否则返回 `Some(false)` 表示未触发；`None` 表示经验池为空。
pub fn auto_train_if_needed(optimizer: &mut RLOptimizer, threshold: usize) -> Option<TrainingStats> {
    let pool_size = optimizer.experience_pool.experiences.len();
    if pool_size >= threshold {
        tracing::info!(
            "[RL Auto-Train] pool_size={} >= threshold={}, triggering train()",
            pool_size,
            threshold
        );
        train(optimizer).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rl_optimizer::{ExperiencePool, TaskState, ToolSelection, Experience};
    use std::collections::HashMap;

    fn make_experience(id: &str, reward: f32) -> Experience {
        Experience {
            id: id.to_string(),
            state: TaskState {
                task_id: "test".into(),
                task_type: "test".into(),
                context: HashMap::new(),
                available_tools: vec!["search".into()],
                completed_tools: vec![],
                error_count: 0,
                elapsed_ms: 100,
            },
            action: ToolSelection {
                tool_id: "search".into(),
                tool_name: "search".into(),
                parameters: HashMap::new(),
                reasoning: "test".into(),
            },
            reward,
            next_state: TaskState {
                task_id: "test".into(),
                task_type: "test".into(),
                context: HashMap::new(),
                available_tools: vec![],
                completed_tools: vec!["search".into()],
                error_count: 0,
                elapsed_ms: 0,
            },
            done: true,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_train_empty_pool() {
        let mut opt = RLOptimizer::new("test".into(), "test".into());
        let result = train(&mut opt);
        assert!(result.is_err());
    }

    #[test]
    fn test_train_with_experiences() {
        let mut opt = RLOptimizer::new("test".into(), "test".into());
        // 填充一些经验
        for i in 0..50 {
            opt.experience_pool.add(make_experience(
                &format!("exp_{}", i),
                if i % 2 == 0 { 0.8 } else { -0.3 },
            ));
        }

        let result = train(&mut opt);
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert!(stats.episodes_completed > 0);
        assert!(stats.avg_reward > -1.0 && stats.avg_reward < 1.01);
    }

    #[test]
    fn test_auto_train_below_threshold() {
        let mut opt = RLOptimizer::new("test".into(), "test".into());
        for i in 0..5 {
            opt.experience_pool.add(make_experience(&format!("e{}", i), 0.5));
        }
        let result = auto_train_if_needed(&mut opt, 10);
        assert!(result.is_none());
    }

    #[test]
    fn test_auto_train_above_threshold() {
        let mut opt = RLOptimizer::new("test".into(), "test".into());
        for i in 0..50 {
            opt.experience_pool.add(make_experience(&format!("e{}", i), 0.5));
        }
        let result = auto_train_if_needed(&mut opt, 10);
        assert!(result.is_some());
    }
}
