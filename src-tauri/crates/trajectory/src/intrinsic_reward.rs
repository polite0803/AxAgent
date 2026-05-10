//! Intrinsic motivation and curiosity-driven exploration
//!
//! Provides intrinsic reward signals based on novelty, learning progress,
//! and information gain to encourage exploration beyond extrinsic rewards.

use crate::trajectory::Trajectory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoveltyEstimator {
    pub state_embedding_counts: HashMap<String, u32>,
    pub total_states: u32,
}

impl NoveltyEstimator {
    pub fn new() -> Self {
        Self {
            state_embedding_counts: HashMap::new(),
            total_states: 0,
        }
    }

    pub fn compute_novelty(&mut self, state_key: &str) -> f64 {
        let count = self
            .state_embedding_counts
            .entry(state_key.to_string())
            .or_insert(0);
        let novelty = 1.0 / (1.0 + *count as f64);
        *count += 1;
        self.total_states += 1;
        novelty
    }

    pub fn reset(&mut self) {
        self.state_embedding_counts.clear();
        self.total_states = 0;
    }
}

impl Default for NoveltyEstimator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressTracker {
    pub performance_history: Vec<f64>,
    pub window_size: usize,
}

impl ProgressTracker {
    pub fn new(window_size: usize) -> Self {
        Self {
            performance_history: Vec::new(),
            window_size,
        }
    }

    pub fn record(&mut self, score: f64) {
        self.performance_history.push(score);
    }

    pub fn compute_learning_progress(&self) -> f64 {
        if self.performance_history.len() < self.window_size * 2 {
            return 0.0;
        }

        let recent_start = self
            .performance_history
            .len()
            .saturating_sub(self.window_size);
        let older_end = recent_start;
        let older_start = older_end.saturating_sub(self.window_size);

        let recent: Vec<f64> = self.performance_history[recent_start..].to_vec();
        let older: Vec<f64> = self.performance_history[older_start..older_end].to_vec();

        let recent_mean = recent.iter().sum::<f64>() / recent.len() as f64;
        let older_mean = older.iter().sum::<f64>() / older.len() as f64;

        let progress = recent_mean - older_mean;
        progress.max(0.0)
    }

    pub fn reset(&mut self) {
        self.performance_history.clear();
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new(10)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoGainCalculator {
    pub knowledge_state: HashMap<String, f64>,
}

impl InfoGainCalculator {
    pub fn new() -> Self {
        Self {
            knowledge_state: HashMap::new(),
        }
    }

    pub fn compute_information_gain(
        &self,
        old_state: &HashMap<String, f64>,
        new_state: &HashMap<String, f64>,
    ) -> f64 {
        let all_keys: std::collections::HashSet<String> =
            old_state.keys().chain(new_state.keys()).cloned().collect();

        let mut total_gain = 0.0;

        for key in &all_keys {
            let p = old_state.get(key).copied().unwrap_or(0.0);
            let q = new_state.get(key).copied().unwrap_or(0.0);

            total_gain += (p - q).abs();
        }

        total_gain
    }

    pub fn update_knowledge(&mut self, new_state: &HashMap<String, f64>) -> f64 {
        let gain = self.compute_information_gain(&self.knowledge_state, new_state);
        self.knowledge_state = new_state.clone();
        gain
    }

    pub fn reset(&mut self) {
        self.knowledge_state.clear();
    }
}

impl Default for InfoGainCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrinsicMotivationConfig {
    pub novelty_weight: f64,
    pub progress_weight: f64,
    pub info_gain_weight: f64,
}

impl Default for IntrinsicMotivationConfig {
    fn default() -> Self {
        Self {
            novelty_weight: 0.4,
            progress_weight: 0.35,
            info_gain_weight: 0.25,
        }
    }
}

pub struct IntrinsicMotivationEngine {
    novelty_estimator: NoveltyEstimator,
    progress_tracker: ProgressTracker,
    info_gain_calculator: InfoGainCalculator,
    config: IntrinsicMotivationConfig,
}

impl IntrinsicMotivationEngine {
    pub fn new(config: IntrinsicMotivationConfig) -> Self {
        Self {
            novelty_estimator: NoveltyEstimator::new(),
            progress_tracker: ProgressTracker::new(10),
            info_gain_calculator: InfoGainCalculator::new(),
            config,
        }
    }

    pub fn compute_intrinsic_reward(&mut self, trajectory: &Trajectory) -> f64 {
        let tool_sequence_key = Self::extract_tool_sequence_key(trajectory);
        let novelty = self.novelty_estimator.compute_novelty(&tool_sequence_key);

        self.progress_tracker.record(trajectory.quality.overall);
        let progress = self.progress_tracker.compute_learning_progress();

        let knowledge = Self::extract_knowledge_state(trajectory);
        let info_gain = self.info_gain_calculator.update_knowledge(&knowledge);

        let reward = novelty * self.config.novelty_weight
            + progress * self.config.progress_weight
            + info_gain * self.config.info_gain_weight;

        reward.clamp(0.0, 1.0)
    }

    pub fn has_provider(&self) -> bool {
        true
    }

    fn extract_tool_sequence_key(trajectory: &Trajectory) -> String {
        let tool_names: Vec<String> = trajectory
            .steps
            .iter()
            .filter_map(|s| {
                s.tool_calls
                    .as_ref()
                    .map(|calls| calls.iter().map(|c| c.name.clone()).collect::<Vec<_>>())
            })
            .flatten()
            .collect();
        tool_names.join("->")
    }

    fn extract_knowledge_state(trajectory: &Trajectory) -> HashMap<String, f64> {
        let mut state = HashMap::new();

        let tool_count = trajectory
            .steps
            .iter()
            .filter(|s| s.tool_calls.is_some())
            .count() as f64;
        state.insert("tool_diversity".to_string(), tool_count / 10.0);

        let reasoning_ratio = if !trajectory.steps.is_empty() {
            trajectory
                .steps
                .iter()
                .filter(|s| s.reasoning.is_some())
                .count() as f64
                / trajectory.steps.len() as f64
        } else {
            0.0
        };
        state.insert("reasoning_ratio".to_string(), reasoning_ratio);

        state.insert("quality_score".to_string(), trajectory.quality.overall);

        let error_rate = if !trajectory.steps.is_empty() {
            trajectory
                .steps
                .iter()
                .filter(|s| {
                    s.tool_results
                        .as_ref()
                        .map(|r| r.iter().any(|tr| tr.is_error))
                        .unwrap_or(false)
                })
                .count() as f64
                / trajectory.steps.len() as f64
        } else {
            0.0
        };
        state.insert("error_rate".to_string(), error_rate);

        state
    }

    pub fn novelty_estimator(&self) -> &NoveltyEstimator {
        &self.novelty_estimator
    }

    pub fn progress_tracker(&self) -> &ProgressTracker {
        &self.progress_tracker
    }

    pub fn info_gain_calculator(&self) -> &InfoGainCalculator {
        &self.info_gain_calculator
    }

    pub fn config(&self) -> &IntrinsicMotivationConfig {
        &self.config
    }
}

impl Default for IntrinsicMotivationEngine {
    fn default() -> Self {
        Self::new(IntrinsicMotivationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trajectory::{MessageRole, ToolCall, ToolResult, TrajectoryOutcome, TrajectoryStep};

    fn create_test_trajectory(outcome: TrajectoryOutcome) -> Trajectory {
        let steps = vec![
            TrajectoryStep {
                timestamp_ms: 1000,
                role: MessageRole::User,
                content: "Help me fix this bug".to_string(),
                reasoning: None,
                tool_calls: None,
                tool_results: None,
            },
            TrajectoryStep {
                timestamp_ms: 2000,
                role: MessageRole::Assistant,
                content: "I'll analyze the code".to_string(),
                reasoning: Some("First I need to understand the issue".to_string()),
                tool_calls: Some(vec![ToolCall {
                    id: "call_1".to_string(),
                    name: "read_file".to_string(),
                    arguments: "{}".to_string(),
                }]),
                tool_results: Some(vec![ToolResult {
                    tool_use_id: "call_1".to_string(),
                    tool_name: "read_file".to_string(),
                    output: "file content".to_string(),
                    is_error: false,
                }]),
            },
        ];

        Trajectory::new(
            "session_1".to_string(),
            "user_1".to_string(),
            "Bug fixing".to_string(),
            "Fixed the bug".to_string(),
            outcome,
            5000,
            steps,
        )
    }

    #[test]
    fn test_novelty_estimator_first_visit() {
        let mut estimator = NoveltyEstimator::new();
        let novelty = estimator.compute_novelty("state_a");
        assert!((novelty - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_novelty_estimator_decreases_with_visits() {
        let mut estimator = NoveltyEstimator::new();
        let first = estimator.compute_novelty("state_a");
        let second = estimator.compute_novelty("state_a");
        let third = estimator.compute_novelty("state_a");
        assert!(first > second);
        assert!(second > third);
    }

    #[test]
    fn test_novelty_estimator_independent_keys() {
        let mut estimator = NoveltyEstimator::new();
        let first_a = estimator.compute_novelty("state_a");
        let first_b = estimator.compute_novelty("state_b");
        assert!((first_a - first_b).abs() < 1e-6);
    }

    #[test]
    fn test_novelty_estimator_reset() {
        let mut estimator = NoveltyEstimator::new();
        estimator.compute_novelty("state_a");
        estimator.compute_novelty("state_a");
        estimator.reset();
        let after_reset = estimator.compute_novelty("state_a");
        assert!((after_reset - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_progress_tracker_insufficient_data() {
        let tracker = ProgressTracker::new(3);
        assert!((tracker.compute_learning_progress() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_progress_tracker_detects_improvement() {
        let mut tracker = ProgressTracker::new(3);
        for _ in 0..3 {
            tracker.record(0.2);
        }
        for _ in 0..3 {
            tracker.record(0.8);
        }
        let progress = tracker.compute_learning_progress();
        assert!(progress > 0.0);
    }

    #[test]
    fn test_progress_tracker_no_regression_reward() {
        let mut tracker = ProgressTracker::new(3);
        for _ in 0..3 {
            tracker.record(0.8);
        }
        for _ in 0..3 {
            tracker.record(0.2);
        }
        let progress = tracker.compute_learning_progress();
        assert!((progress - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_info_gain_identical_states() {
        let calc = InfoGainCalculator::new();
        let state: HashMap<String, f64> = vec![("a".to_string(), 0.5), ("b".to_string(), 0.3)]
            .into_iter()
            .collect();
        let gain = calc.compute_information_gain(&state, &state);
        assert!(gain.abs() < 1e-6);
    }

    #[test]
    fn test_info_gain_different_states() {
        let calc = InfoGainCalculator::new();
        let old: HashMap<String, f64> = vec![("a".to_string(), 0.5)].into_iter().collect();
        let new: HashMap<String, f64> = vec![("a".to_string(), 0.9)].into_iter().collect();
        let gain = calc.compute_information_gain(&old, &new);
        assert!(gain > 0.0);
    }

    #[test]
    fn test_info_gain_update_knowledge() {
        let mut calc = InfoGainCalculator::new();
        let state: HashMap<String, f64> = vec![("x".to_string(), 0.7)].into_iter().collect();
        let gain = calc.update_knowledge(&state);
        assert!(gain > 0.0);
        assert_eq!(calc.knowledge_state.get("x"), Some(&0.7));
    }

    #[test]
    fn test_intrinsic_motivation_engine_success_trajectory() {
        let mut engine = IntrinsicMotivationEngine::default();
        let trajectory = create_test_trajectory(TrajectoryOutcome::Success);
        let reward = engine.compute_intrinsic_reward(&trajectory);
        assert!(reward > 0.0);
        assert!(reward <= 1.0);
    }

    #[test]
    fn test_intrinsic_motivation_engine_novelty_decreases() {
        let mut engine = IntrinsicMotivationEngine::default();
        let trajectory = create_test_trajectory(TrajectoryOutcome::Success);
        let reward1 = engine.compute_intrinsic_reward(&trajectory);
        let reward2 = engine.compute_intrinsic_reward(&trajectory);
        assert!(reward1 >= reward2);
    }

    #[test]
    fn test_intrinsic_motivation_config_custom_weights() {
        let config = IntrinsicMotivationConfig {
            novelty_weight: 1.0,
            progress_weight: 0.0,
            info_gain_weight: 0.0,
        };
        let mut engine = IntrinsicMotivationEngine::new(config);
        let trajectory = create_test_trajectory(TrajectoryOutcome::Success);
        let reward = engine.compute_intrinsic_reward(&trajectory);
        assert!(reward > 0.0);
    }
}
