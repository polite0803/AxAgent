// SPDX-License-Identifier: AGPL-3.0-only

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_selection_policy_new() {
        let policy = ToolSelectionPolicy::new(
            "p1".to_string(),
            "Test Policy".to_string(),
            "model-1".to_string(),
        );
        assert_eq!(policy.id, "p1");
        assert_eq!(policy.name, "Test Policy");
        assert_eq!(policy.model_id, "model-1");
        assert!((policy.temperature - 0.7).abs() < 0.001);
        assert!((policy.top_p - 0.9).abs() < 0.001);
        assert_eq!(policy.max_tokens, 2048);
        assert!(policy.reward_signals.is_empty());
        assert!(policy.q_values.is_empty());
    }

    #[test]
    fn test_tool_selection_policy_update_q_value_new() {
        let mut policy =
            ToolSelectionPolicy::new("p1".to_string(), "Test".to_string(), "m1".to_string());
        policy.update_q_value("state1:action1", 1.0, 0.5);
        assert!(policy.q_values.contains_key("state1:action1"));
        let q = policy.q_values.get("state1:action1").unwrap();
        assert!(*q > 0.0);
    }

    #[test]
    fn test_tool_selection_policy_update_q_value_existing() {
        let mut policy =
            ToolSelectionPolicy::new("p1".to_string(), "Test".to_string(), "m1".to_string());
        policy.training_config.learning_rate = 0.1;
        policy.update_q_value("state1:action1", 1.0, 0.5);
        let first_q = *policy.q_values.get("state1:action1").unwrap();
        policy.update_q_value("state1:action1", 0.5, 0.3);
        let second_q = *policy.q_values.get("state1:action1").unwrap();
        assert!((second_q - first_q).abs() > 0.001);
    }

    #[test]
    fn test_tool_selection_policy_get_best_action() {
        let mut policy =
            ToolSelectionPolicy::new("p1".to_string(), "Test".to_string(), "m1".to_string());
        policy.q_values.insert("state1:action_a".to_string(), 0.8);
        policy.q_values.insert("state1:action_b".to_string(), 0.3);
        let best = policy.get_best_action("state1");
        assert!(best.is_some());
        assert_eq!(best.unwrap(), "action_a");
    }

    #[test]
    fn test_tool_selection_policy_get_best_action_no_match() {
        let mut policy =
            ToolSelectionPolicy::new("p1".to_string(), "Test".to_string(), "m1".to_string());
        policy.q_values.insert("state1:action_a".to_string(), 0.8);
        let best = policy.get_best_action("state2");
        assert!(best.is_none());
    }

    #[test]
    fn test_training_config_default() {
        let config = TrainingConfig::default();
        assert!((config.learning_rate - 0.001).abs() < 0.0001);
        assert_eq!(config.batch_size, 32);
        assert_eq!(config.epochs, 10);
        assert!((config.gradient_clip - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_reward_signal_type_variants() {
        let types = vec![
            RewardSignalType::TaskCompletion,
            RewardSignalType::TimeEfficiency,
            RewardSignalType::ErrorRate,
            RewardSignalType::ToolDiversity,
            RewardSignalType::UserFeedback,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let de: RewardSignalType = serde_json::from_str(&json).unwrap();
            assert_eq!(de, t);
        }
    }

    #[test]
    fn test_decomposition_type_variants() {
        let types = vec![
            DecompositionType::Sequential,
            DecompositionType::Parallel,
            DecompositionType::Hierarchical,
            DecompositionType::Conditional,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let de: DecompositionType = serde_json::from_str(&json).unwrap();
            assert_eq!(de, t);
        }
    }

    #[test]
    fn test_error_category_variants() {
        let cats = vec![
            ErrorCategory::Timeout,
            ErrorCategory::RateLimit,
            ErrorCategory::InvalidInput,
            ErrorCategory::ToolFailure,
            ErrorCategory::NetworkError,
        ];
        for c in cats {
            let json = serde_json::to_string(&c).unwrap();
            let de: ErrorCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(de, c);
        }
    }

    #[test]
    fn test_strategy_type_variants() {
        let types = vec![
            StrategyType::Retry,
            StrategyType::AlternativeTool,
            StrategyType::SimplifyTask,
            StrategyType::RequestUserInput,
            StrategyType::SkipTask,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let de: StrategyType = serde_json::from_str(&json).unwrap();
            assert_eq!(de, t);
        }
    }

    #[test]
    fn test_task_decomposition_policy_serialization() {
        let policy = TaskDecompositionPolicy {
            id: "tdp1".to_string(),
            decomposition_type: DecompositionType::Hierarchical,
            max_depth: 3,
            min_task_size: 1,
            learned_patterns: vec![],
        };
        let json = serde_json::to_string(&policy).unwrap();
        let de: TaskDecompositionPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(de.id, "tdp1");
        assert_eq!(de.max_depth, 3);
    }

    #[test]
    fn test_error_recovery_policy_serialization() {
        let policy = ErrorRecoveryPolicy {
            id: "erp1".to_string(),
            error_categories: vec![ErrorCategory::Timeout],
            recovery_strategies: HashMap::new(),
            learned_heuristics: vec![],
        };
        let json = serde_json::to_string(&policy).unwrap();
        let de: ErrorRecoveryPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(de.id, "erp1");
    }

    #[test]
    fn test_subtask_spec() {
        let spec = SubtaskSpec {
            name: "sub1".to_string(),
            description: "A subtask".to_string(),
            tools_required: vec!["tool1".to_string()],
            dependencies: vec![],
        };
        let json = serde_json::to_string(&spec).unwrap();
        let de: SubtaskSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(de.name, "sub1");
        assert_eq!(de.tools_required.len(), 1);
    }

    #[test]
    fn test_recovery_strategy() {
        let strategy = RecoveryStrategy {
            strategy_type: StrategyType::Retry,
            max_retries: 3,
            backoff_multiplier: 2.0,
            fallback_action: Some("skip".to_string()),
        };
        let json = serde_json::to_string(&strategy).unwrap();
        let de: RecoveryStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(de.max_retries, 3);
        assert_eq!(de.strategy_type, StrategyType::Retry);
    }

    #[test]
    fn test_error_heuristic() {
        let heuristic = ErrorHeuristic {
            error_pattern: "timeout".to_string(),
            recommended_strategy: "retry".to_string(),
            success_rate: 0.85,
            usage_count: 10,
        };
        let json = serde_json::to_string(&heuristic).unwrap();
        let de: ErrorHeuristic = serde_json::from_str(&json).unwrap();
        assert!((de.success_rate - 0.85).abs() < 0.001);
        assert_eq!(de.usage_count, 10);
    }
}
