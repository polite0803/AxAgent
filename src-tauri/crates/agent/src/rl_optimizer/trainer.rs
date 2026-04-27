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
            return Err(RLError::TrainingError(
                "No experiences to train on".to_string(),
            ));
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
