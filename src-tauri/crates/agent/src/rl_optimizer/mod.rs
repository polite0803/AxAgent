pub mod experience;
pub mod policy;
pub mod trainer;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLOptimizer {
    pub id: String,
    pub name: String,
    pub policies: HashMap<String, Policy>,
    pub experience_pool: ExperiencePool,
    pub config: RLConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLConfig {
    pub learning_rate: f32,
    pub batch_size: u32,
    pub gamma: f32,
    pub epsilon: f32,
    pub epsilon_decay: f32,
    pub epsilon_min: f32,
}

impl Default for RLConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            batch_size: 32,
            gamma: 0.99,
            epsilon: 1.0,
            epsilon_decay: 0.995,
            epsilon_min: 0.01,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperiencePool {
    pub experiences: Vec<Experience>,
    pub max_size: usize,
}

impl ExperiencePool {
    pub fn new(max_size: usize) -> Self {
        Self {
            experiences: Vec::new(),
            max_size,
        }
    }

    pub fn add(&mut self, experience: Experience) {
        if self.experiences.len() >= self.max_size {
            self.experiences.remove(0);
        }
        self.experiences.push(experience);
    }

    pub fn sample(&self, batch_size: usize) -> Vec<&Experience> {
        let len = self.experiences.len();
        if len == 0 {
            return vec![];
        }
        let batch_size = batch_size.min(len);
        let mut indices: Vec<usize> = (0..len).collect();
        for i in 0..batch_size {
            let j = i + (fastrand::usize(..(len - i)));
            indices.swap(i, j);
        }
        indices
            .into_iter()
            .take(batch_size)
            .map(|i| &self.experiences[i])
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub state: TaskState,
    pub action: ToolSelection,
    pub reward: f32,
    pub next_state: TaskState,
    pub done: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub task_id: String,
    pub task_type: String,
    pub context: HashMap<String, serde_json::Value>,
    pub available_tools: Vec<String>,
    pub completed_tools: Vec<String>,
    pub error_count: u32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelection {
    pub tool_id: String,
    pub tool_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub policy_type: PolicyType,
    pub model_id: String,
    pub reward_signals: Vec<RewardSignal>,
    pub training_stats: TrainingStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyType {
    ToolSelection,
    TaskDecomposition,
    ErrorRecovery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSignal {
    pub name: String,
    pub weight: f32,
    pub signal_type: RewardSignalType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardSignalType {
    TaskCompletion,
    TimeEfficiency,
    ErrorRate,
    ToolDiversity,
    UserFeedback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStats {
    pub total_experiences: u64,
    pub episodes_completed: u64,
    pub avg_reward: f32,
    pub last_update: chrono::DateTime<chrono::Utc>,
}

impl RLOptimizer {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            policies: HashMap::new(),
            experience_pool: ExperiencePool::new(10000),
            config: RLConfig::default(),
        }
    }

    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.insert(policy.id.clone(), policy);
    }

    pub fn record_experience(&mut self, experience: Experience) {
        self.experience_pool.add(experience);
    }

    pub fn select_tool(&self, _state: &TaskState) -> Result<ToolSelection, RLError> {
        Ok(ToolSelection {
            tool_id: "default_tool".to_string(),
            tool_name: "Default Tool".to_string(),
            parameters: HashMap::new(),
            reasoning: "RL policy selection".to_string(),
        })
    }

    pub fn get_policy_stats(&self, policy_id: &str) -> Option<TrainingStats> {
        self.policies
            .get(policy_id)
            .map(|p| p.training_stats.clone())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RLError {
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),
    #[error("Training error: {0}")]
    TrainingError(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}
