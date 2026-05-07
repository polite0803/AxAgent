use crate::rl_optimizer::{
    Experience, RLConfig, RLError, RLOptimizer, TaskState, ToolSelection, TrainingStats,
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

    pub fn train(&mut self) -> Result<TrainingStats, RLError> {
        let batch_size = self.config.batch_size as usize;
        let experiences = self.optimizer.experience_pool.sample(batch_size);

        if experiences.is_empty() {
            return Err(RLError::TrainingError("No experiences to train on".to_string()));
        }

        let mut total_reward = 0.0f32;
        for experience in &experiences {
            total_reward += experience.reward;
        }

        let avg_reward = total_reward / experiences.len() as f32;

        Ok(TrainingStats {
            total_experiences: self.optimizer.experience_pool.experiences.len() as u64,
            episodes_completed: experiences.len() as u64,
            avg_reward,
            last_update: chrono::Utc::now(),
        })
    }

    pub fn update_tool_selection_policy(
        &mut self,
        policy_id: &str,
        experiences: &[&Experience],
    ) -> Result<(), RLError> {
        let _ = policy_id;
        let _ = experiences;
        Ok(())
    }

    pub fn evaluate_policy(&self, _policy_id: &str, test_states: &[TaskState]) -> Vec<f32> {
        let mut rewards = Vec::new();

        for state in test_states {
            if let Ok(action) = self.optimizer.select_tool(state) {
                let reward = self.calculate_reward(&action);
                rewards.push(reward);
            }
        }

        rewards
    }

    fn calculate_reward(&self, _action: &ToolSelection) -> f32 {
        1.0
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
    fn test_rl_trainer_evaluate_policy() {
        let optimizer = RLOptimizer::new("opt1".to_string(), "Optimizer".to_string());
        let trainer = RLtrainer::new(optimizer);
        let states = vec![make_task_state(), make_task_state()];
        let rewards = trainer.evaluate_policy("p1", &states);
        assert_eq!(rewards.len(), 2);
        for r in &rewards {
            assert!((*r - 1.0).abs() < 0.001);
        }
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
