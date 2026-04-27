use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelectionPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub model_id: String,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
    pub reward_signals: Vec<RewardSignal>,
    pub training_config: TrainingConfig,
    pub q_values: HashMap<String, f32>,
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
pub struct TrainingConfig {
    pub learning_rate: f32,
    pub batch_size: u32,
    pub epochs: u32,
    pub gradient_clip: f32,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            batch_size: 32,
            epochs: 10,
            gradient_clip: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDecompositionPolicy {
    pub id: String,
    pub decomposition_type: DecompositionType,
    pub max_depth: u32,
    pub min_task_size: u32,
    pub learned_patterns: Vec<DecompositionPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecompositionType {
    Sequential,
    Parallel,
    Hierarchical,
    Conditional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionPattern {
    pub task_signature: String,
    pub subtasks: Vec<SubtaskSpec>,
    pub success_rate: f32,
    pub avg_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskSpec {
    pub name: String,
    pub description: String,
    pub tools_required: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryPolicy {
    pub id: String,
    pub error_categories: Vec<ErrorCategory>,
    pub recovery_strategies: HashMap<String, RecoveryStrategy>,
    pub learned_heuristics: Vec<ErrorHeuristic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCategory {
    Timeout,
    RateLimit,
    InvalidInput,
    ToolFailure,
    NetworkError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStrategy {
    pub strategy_type: StrategyType,
    pub max_retries: u32,
    pub backoff_multiplier: f32,
    pub fallback_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    Retry,
    AlternativeTool,
    SimplifyTask,
    RequestUserInput,
    SkipTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHeuristic {
    pub error_pattern: String,
    pub recommended_strategy: String,
    pub success_rate: f32,
    pub usage_count: u32,
}

impl ToolSelectionPolicy {
    pub fn new(id: String, name: String, model_id: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            model_id,
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 2048,
            reward_signals: Vec::new(),
            training_config: TrainingConfig::default(),
            q_values: HashMap::new(),
        }
    }

    pub fn update_q_value(&mut self, state_action: &str, reward: f32, next_max_q: f32) {
        let learning_rate = self.training_config.learning_rate;
        let gamma = 0.99;

        let current_q = self.q_values.get(state_action).copied().unwrap_or(0.0);
        let new_q = current_q + learning_rate * (reward + gamma * next_max_q - current_q);
        self.q_values.insert(state_action.to_string(), new_q);
    }

    pub fn get_best_action(&self, state: &str) -> Option<String> {
        self.q_values
            .iter()
            .filter(|(k, _)| k.starts_with(state))
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(k, _)| k.split(':').nth(1).unwrap_or("").to_string())
    }
}
