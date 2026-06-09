use crate::rl_optimizer::{
    Experience, PolicyType, RLConfig, RLError, RLOptimizer, RewardSignal, RewardSignalType,
    TaskState, ToolSelection, TrainingStats,
};
use std::collections::HashMap;

pub struct RLtrainer {
    optimizer: RLOptimizer,
    config: RLConfig,
}

impl RLtrainer {
    pub fn new(optimizer: RLOptimizer) -> Self {
        Self {
            optimizer,
            config: RLConfig::default(),
        }
    }

    /// 执行一轮训练：从经验池采样 → 计算工具级奖励 → 更新策略权重
    pub fn train(&mut self) -> Result<TrainingStats, RLError> {
        let batch_size = self.config.batch_size as usize;
        let experiences = self.optimizer.experience_pool.sample(batch_size);

        if experiences.is_empty() {
            return Err(RLError::TrainingError("No experiences to train on".to_string()));
        }

        // 按工具分组统计奖励
        let mut tool_rewards: HashMap<String, (f32, usize)> = HashMap::new();
        let mut total_reward = 0.0f32;

        for exp in &experiences {
            let tool_key = format!("{}:{}", exp.action.tool_name, exp.action.tool_id);
            let entry = tool_rewards.entry(tool_key).or_insert((0.0, 0));
            entry.0 += exp.reward;
            entry.1 += 1;
            total_reward += exp.reward;
        }

        let avg_reward = total_reward / experiences.len() as f32;
        let episodes_completed = experiences.iter().filter(|e| e.done).count() as u64;

        // 找到表现最好的工具
        let best_tool = tool_rewards
            .iter()
            .max_by(|(_, (ra, ca)), (_, (rb, cb))| {
                let avg_a = ra / *ca as f32;
                let avg_b = rb / *cb as f32;
                avg_a
                    .partial_cmp(&avg_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(name, _)| name.clone());

        // 更新策略的奖励信号权重（基于本次训练结果）
        let policy_id = "tool_selection";
        if let Some(policy) = self.optimizer.policies.get_mut(policy_id) {
            // 更新或创建工具奖励信号
            for (tool_key, (reward_sum, count)) in &tool_rewards {
                let avg_tool_reward = *reward_sum / *count as f32;
                let tool_name = tool_key.split(':').next().unwrap_or(tool_key);

                if let Some(existing) = policy
                    .reward_signals
                    .iter_mut()
                    .find(|s| s.name == tool_name)
                {
                    // 移动平均更新权重
                    existing.weight = existing.weight * 0.7 + avg_tool_reward * 0.3;
                } else if policy.reward_signals.len() < 20 {
                    policy.reward_signals.push(RewardSignal {
                        name: tool_name.to_string(),
                        weight: avg_tool_reward,
                        signal_type: RewardSignalType::TaskCompletion,
                    });
                }
            }

            // 更新训练统计
            policy.training_stats = TrainingStats {
                total_experiences: self.optimizer.experience_pool.experiences.len() as u64,
                episodes_completed: policy.training_stats.episodes_completed + episodes_completed,
                avg_reward,
                last_update: chrono::Utc::now(),
            };
        } else if let Some(best) = best_tool {
            // 自动创建工具选择策略
            let tool_name = best.split(':').next().unwrap_or(&best).to_string();
            let mut policy = crate::rl_optimizer::Policy {
                id: "tool_selection".to_string(),
                name: "Tool Selection Policy".to_string(),
                policy_type: PolicyType::ToolSelection,
                model_id: "rl-v1".to_string(),
                reward_signals: Vec::new(),
                training_stats: TrainingStats {
                    total_experiences: self.optimizer.experience_pool.experiences.len() as u64,
                    episodes_completed,
                    avg_reward,
                    last_update: chrono::Utc::now(),
                },
            };
            policy.reward_signals.push(RewardSignal {
                name: tool_name,
                weight: avg_reward,
                signal_type: RewardSignalType::TaskCompletion,
            });
            self.optimizer.add_policy(policy);
        }

        Ok(TrainingStats {
            total_experiences: self.optimizer.experience_pool.experiences.len() as u64,
            episodes_completed,
            avg_reward,
            last_update: chrono::Utc::now(),
        })
    }

    /// 基于经验数据更新指定策略的工具选择权重
    pub fn update_tool_selection_policy(
        &mut self,
        policy_id: &str,
        experiences: &[&Experience],
    ) -> Result<(), RLError> {
        let policy = self
            .optimizer
            .policies
            .get_mut(policy_id)
            .ok_or_else(|| RLError::PolicyNotFound(policy_id.to_string()))?;

        // 按工具分组统计
        let mut tool_stats: HashMap<String, (f32, usize, usize)> = HashMap::new();
        for exp in experiences {
            let key = exp.action.tool_name.clone();
            let entry = tool_stats.entry(key).or_insert((0.0, 0, 0));
            entry.0 += exp.reward;
            entry.1 += 1; // total uses
            if exp.reward > 0.0 {
                entry.2 += 1; // successful uses
            }
        }

        // 清除旧的工具信号，用新的替换
        policy.reward_signals.retain(|s| {
            !matches!(s.signal_type, RewardSignalType::TaskCompletion)
                && !matches!(s.signal_type, RewardSignalType::ToolDiversity)
        });

        for (tool, (reward_sum, total, successes)) in &tool_stats {
            let avg = *reward_sum / *total as f32;
            let success_rate = *successes as f32 / *total as f32;
            policy.reward_signals.push(RewardSignal {
                name: tool.clone(),
                weight: avg.clamp(0.0, 1.0),
                signal_type: RewardSignalType::TaskCompletion,
            });
            // 也添加成功率信号
            policy.reward_signals.push(RewardSignal {
                name: format!("{}_success", tool),
                weight: success_rate,
                signal_type: RewardSignalType::ToolDiversity,
            });
        }

        policy.training_stats.last_update = chrono::Utc::now();
        Ok(())
    }

    /// 评估策略在测试状态集上的表现
    pub fn evaluate_policy(&self, policy_id: &str, test_states: &[TaskState]) -> Vec<f32> {
        let mut rewards = Vec::new();
        let policy = match self.optimizer.policies.get(policy_id) {
            Some(p) => p,
            None => return rewards,
        };

        for state in test_states {
            // 为每个可用工具分配奖励值
            for tool in &state.available_tools {
                let reward = policy
                    .reward_signals
                    .iter()
                    .find(|s| s.name == *tool)
                    .map(|s| s.weight)
                    .unwrap_or(0.3); // 默认权重
                rewards.push(reward);
            }
        }

        rewards
    }

    /// 计算动作奖励：基于参数完整性 + 推理质量 + 策略匹配度
    pub fn calculate_reward(&self, action: &ToolSelection) -> f32 {
        let mut reward = 0.5f32; // 基础奖励

        if !action.parameters.is_empty() {
            reward += 0.2;
        }
        if !action.reasoning.is_empty() {
            reward += 0.3;
        }

        // 如果策略中有该工具的记录，加入策略权重
        if let Some(policy) = self.optimizer.policies.get("tool_selection")
            && let Some(signal) = policy
                .reward_signals
                .iter()
                .find(|s| s.name == action.tool_name)
        {
            reward = reward * 0.6 + signal.weight * 0.4;
        }

        reward.clamp(0.0, 1.0)
    }

    pub fn get_optimizer(&self) -> &RLOptimizer {
        &self.optimizer
    }

    pub fn get_mut_optimizer(&mut self) -> &mut RLOptimizer {
        &mut self.optimizer
    }
}

pub struct ExperienceCollector {
    current_experience: Option<Experience>,
    experience_buffer: Vec<Experience>,
}

impl ExperienceCollector {
    pub fn new() -> Self {
        Self {
            current_experience: None,
            experience_buffer: Vec::new(),
        }
    }

    pub fn start_episode(&mut self, state: TaskState) {
        self.current_experience = Some(Experience {
            id: uuid::Uuid::new_v4().to_string(),
            state,
            action: ToolSelection {
                tool_id: String::new(),
                tool_name: String::new(),
                parameters: HashMap::new(),
                reasoning: String::new(),
            },
            reward: 0.0,
            next_state: TaskState {
                task_id: String::new(),
                task_type: String::new(),
                context: HashMap::new(),
                available_tools: Vec::new(),
                completed_tools: Vec::new(),
                error_count: 0,
                elapsed_ms: 0,
            },
            done: false,
            timestamp: chrono::Utc::now(),
        });
    }

    pub fn record_action(&mut self, action: ToolSelection) {
        if let Some(ref mut exp) = self.current_experience {
            exp.action = action;
        }
    }

    pub fn record_reward(&mut self, reward: f32) {
        if let Some(ref mut exp) = self.current_experience {
            exp.reward += reward;
        }
    }

    pub fn end_episode(&mut self, next_state: TaskState, done: bool) {
        if let Some(mut exp) = self.current_experience.take() {
            exp.next_state = next_state;
            exp.done = done;
            self.experience_buffer.push(exp);
        }
    }

    pub fn get_experiences(&self) -> Vec<Experience> {
        self.experience_buffer.clone()
    }

    pub fn clear(&mut self) {
        self.experience_buffer.clear();
    }
}

impl Default for ExperienceCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task_state() -> TaskState {
        TaskState {
            task_id: "task_1".to_string(),
            task_type: "code_generation".to_string(),
            context: HashMap::new(),
            available_tools: vec!["tool_a".to_string(), "tool_b".to_string()],
            completed_tools: vec![],
            error_count: 0,
            elapsed_ms: 0,
        }
    }

    fn make_tool_selection() -> ToolSelection {
        ToolSelection {
            tool_id: "tool_a".to_string(),
            tool_name: "Tool A".to_string(),
            parameters: HashMap::new(),
            reasoning: "best fit".to_string(),
        }
    }

    #[test]
    fn test_rl_trainer_new() {
        let optimizer = RLOptimizer::new("opt1".to_string(), "Optimizer".to_string());
        let trainer = RLtrainer::new(optimizer);
        assert_eq!(trainer.get_optimizer().id, "opt1");
    }

    #[test]
    fn test_rl_trainer_train_no_experiences() {
        let optimizer = RLOptimizer::new("opt1".to_string(), "Optimizer".to_string());
        let mut trainer = RLtrainer::new(optimizer);
        let result = trainer.train();
        assert!(result.is_err());
        match result.unwrap_err() {
            RLError::TrainingError(msg) => assert!(msg.contains("No experiences")),
            _ => panic!("Expected TrainingError"),
        }
    }

    #[test]
    fn test_rl_trainer_train_with_experiences() {
        let mut optimizer = RLOptimizer::new("opt1".to_string(), "Optimizer".to_string());
        for i in 0..5 {
            optimizer.record_experience(Experience {
                id: format!("exp_{}", i),
                state: make_task_state(),
                action: make_tool_selection(),
                reward: 1.0,
                next_state: make_task_state(),
                done: false,
                timestamp: chrono::Utc::now(),
            });
        }
        let mut trainer = RLtrainer::new(optimizer);
        let result = trainer.train();
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert!(stats.total_experiences >= 5);
    }

    #[test]
    fn test_rl_trainer_get_mut_optimizer() {
        let optimizer = RLOptimizer::new("opt1".to_string(), "Optimizer".to_string());
        let mut trainer = RLtrainer::new(optimizer);
        trainer.get_mut_optimizer().record_experience(Experience {
            id: "exp_1".to_string(),
            state: make_task_state(),
            action: make_tool_selection(),
            reward: 0.5,
            next_state: make_task_state(),
            done: false,
            timestamp: chrono::Utc::now(),
        });
        assert_eq!(trainer.get_optimizer().experience_pool.experiences.len(), 1);
    }

    #[test]
    fn test_rl_trainer_update_tool_selection_policy() {
        let optimizer = RLOptimizer::new("opt1".to_string(), "Optimizer".to_string());
        let mut trainer = RLtrainer::new(optimizer);
        let exp = Experience {
            id: "exp_1".to_string(),
            state: make_task_state(),
            action: make_tool_selection(),
            reward: 1.0,
            next_state: make_task_state(),
            done: false,
            timestamp: chrono::Utc::now(),
        };
        let result = trainer.update_tool_selection_policy("p1", &[&exp]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rl_trainer_evaluate_policy_with_reward_signals() {
        let mut optimizer = RLOptimizer::new("opt1".to_string(), "Optimizer".to_string());
        let mut policy = crate::rl_optimizer::Policy {
            id: "p1".to_string(),
            name: "Test Policy".to_string(),
            policy_type: PolicyType::ToolSelection,
            model_id: "rl-v1".to_string(),
            reward_signals: vec![RewardSignal {
                name: "tool_a".to_string(),
                weight: 0.8,
                signal_type: RewardSignalType::TaskCompletion,
            }],
            training_stats: TrainingStats {
                total_experiences: 0,
                episodes_completed: 0,
                avg_reward: 0.0,
                last_update: chrono::Utc::now(),
            },
        };
        optimizer.add_policy(policy);
        let trainer = RLtrainer::new(optimizer);
        let states = vec![make_task_state()];
        let rewards = trainer.evaluate_policy("p1", &states);
        // tool_a has weight 0.8, but the state has tool_a and tool_b
        // tool_b gets default 0.3
        assert_eq!(rewards.len(), 2);
        assert!((rewards[0] - 0.8).abs() < 0.001);
        assert!((rewards[1] - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_experience_collector_new() {
        let collector = ExperienceCollector::new();
        assert!(collector.get_experiences().is_empty());
    }

    #[test]
    fn test_experience_collector_start_episode() {
        let mut collector = ExperienceCollector::new();
        collector.start_episode(make_task_state());
        assert!(collector.current_experience.is_some());
    }

    #[test]
    fn test_experience_collector_record_action() {
        let mut collector = ExperienceCollector::new();
        collector.start_episode(make_task_state());
        collector.record_action(make_tool_selection());
        let exp = collector.current_experience.as_ref().unwrap();
        assert_eq!(exp.action.tool_id, "tool_a");
    }

    #[test]
    fn test_experience_collector_record_reward() {
        let mut collector = ExperienceCollector::new();
        collector.start_episode(make_task_state());
        collector.record_reward(0.5);
        collector.record_reward(0.3);
        let exp = collector.current_experience.as_ref().unwrap();
        assert!((exp.reward - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_experience_collector_end_episode() {
        let mut collector = ExperienceCollector::new();
        collector.start_episode(make_task_state());
        collector.record_action(make_tool_selection());
        collector.record_reward(1.0);
        let next_state = make_task_state();
        collector.end_episode(next_state, true);
        assert!(collector.current_experience.is_none());
        let experiences = collector.get_experiences();
        assert_eq!(experiences.len(), 1);
        assert!(experiences[0].done);
        assert!((experiences[0].reward - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_experience_collector_clear() {
        let mut collector = ExperienceCollector::new();
        collector.start_episode(make_task_state());
        collector.end_episode(make_task_state(), true);
        assert_eq!(collector.get_experiences().len(), 1);
        collector.clear();
        assert!(collector.get_experiences().is_empty());
    }

    #[test]
    fn test_experience_collector_multiple_episodes() {
        let mut collector = ExperienceCollector::new();
        for i in 0..3 {
            collector.start_episode(make_task_state());
            collector.record_reward(i as f32);
            collector.end_episode(make_task_state(), i == 2);
        }
        let experiences = collector.get_experiences();
        assert_eq!(experiences.len(), 3);
        assert!(!experiences[0].done);
        assert!(!experiences[1].done);
        assert!(experiences[2].done);
    }

    #[test]
    fn test_experience_collector_record_without_start() {
        let mut collector = ExperienceCollector::new();
        collector.record_action(make_tool_selection());
        collector.record_reward(1.0);
        assert!(collector.current_experience.is_none());
    }

    #[test]
    fn test_experience_collector_default() {
        let collector = ExperienceCollector::default();
        assert!(collector.get_experiences().is_empty());
    }
}
