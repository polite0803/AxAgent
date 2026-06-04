

pub use axagent_harness::trajectory_types::{PrmLlmProvider, RewardCategory, StepReward};

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

use crate::trajectory::{Trajectory, TrajectoryOutcome};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRewardConfig {
    pub step_weight: f64,
    pub outcome_weight: f64,
    pub coherence_window: usize,
    pub min_step_reward: f64,
}

impl Default for ProcessRewardConfig {
    fn default() -> Self {
        Self {
            step_weight: 0.6,
            outcome_weight: 0.4,
            coherence_window: 3,
            min_step_reward: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRewardResult {
    pub step_rewards: Vec<StepReward>,
    pub aggregate_reward: f64,
    pub outcome_reward: f64,
    pub weighted_reward: f64,
}

pub struct DefaultPrmProvider {
    #[allow(dead_code)]
    task_context: String,
}

impl DefaultPrmProvider {
    pub fn new(task_context: &str) -> Self {
        Self {
            task_context: task_context.to_string(),
        }
    }

    fn evaluate_correctness(&self, content: &str) -> f64 {
        let mut score: f64 = 0.3;

        let deduction_patterns = [
            "therefore",
            "thus",
            "because",
            "since",
            "hence",
            "so",
            "consequently",
            "it follows",
        ];
        for pattern in &deduction_patterns {
            if content.to_lowercase().contains(pattern) {
                score += 0.15;
                break;
            }
        }

        let factual_indicators = [
            "result", "output", "value", "found", "returned", "equals", "is", "are", "was", "were",
        ];
        let factual_count = factual_indicators
            .iter()
            .filter(|p| content.to_lowercase().contains(*p))
            .count();
        score += (factual_count as f64 * 0.05).min(0.3);

        let contradiction_patterns = ["cannot be", "impossible", "contradicts", "inconsistent"];
        for pattern in &contradiction_patterns {
            if content.to_lowercase().contains(pattern) {
                score -= 0.1;
            }
        }

        score.clamp(0.0, 1.0)
    }

    fn evaluate_coherence(&self, content: &str, previous_steps: &[String]) -> f64 {
        if previous_steps.is_empty() {
            return 0.6;
        }

        let mut score: f64 = 0.5;

        let content_lower = content.to_lowercase();
        let content_words: Vec<&str> = content_lower.split_whitespace().collect();

        let window = previous_steps.iter().rev().take(3).collect::<Vec<_>>();

        let mut shared_concepts = 0;
        let mut total_concepts = 0;

        for prev in &window {
            let prev_lower = prev.to_lowercase();
            let prev_words: Vec<&str> = prev_lower.split_whitespace().collect();
            total_concepts += prev_words.len();

            for word in &content_words {
                if prev_words.contains(word) && word.len() > 3 {
                    shared_concepts += 1;
                }
            }
        }

        if total_concepts > 0 {
            let overlap_ratio = shared_concepts as f64 / total_concepts as f64;
            score += overlap_ratio * 0.3;
        }

        let transition_words = [
            "then",
            "next",
            "after",
            "now",
            "following",
            "based on",
            "using",
            "with this",
            "from the",
        ];
        for tw in &transition_words {
            if content_lower.contains(tw) {
                score += 0.1;
                break;
            }
        }

        let abrupt_patterns = ["however", "but", "on the other hand", "wait", "actually"];
        for pattern in &abrupt_patterns {
            if content_lower.contains(pattern) {
                score -= 0.05;
            }
        }

        score.clamp(0.0, 1.0)
    }

    fn evaluate_completeness(&self, content: &str, context: &str) -> f64 {
        let mut score: f64 = 0.3;

        let context_lower = context.to_lowercase();
        let content_lower = content.to_lowercase();

        let context_keywords: Vec<&str> = context_lower
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();

        if !context_keywords.is_empty() {
            let covered = context_keywords
                .iter()
                .filter(|kw| content_lower.contains(*kw))
                .count();
            let coverage = covered as f64 / context_keywords.len() as f64;
            score += coverage * 0.4;
        }

        if content.len() > 50 {
            score += 0.1;
        }
        if content.len() > 100 {
            score += 0.1;
        }

        let completeness_indicators = [
            "all",
            "every",
            "each",
            "complete",
            "fully",
            "entire",
            "comprehensive",
        ];
        for indicator in &completeness_indicators {
            if content_lower.contains(indicator) {
                score += 0.05;
                break;
            }
        }

        let incomplete_indicators = ["todo", "tbd", "pending", "not yet", "incomplete", "..."];
        for indicator in &incomplete_indicators {
            if content_lower.contains(indicator) {
                score -= 0.15;
            }
        }

        score.clamp(0.0, 1.0)
    }

    fn evaluate_efficiency(&self, content: &str, previous_steps: &[String]) -> f64 {
        let mut score: f64 = 0.7;

        let content_lower = content.to_lowercase();

        let redundant_patterns = [
            "same as before",
            "repeat",
            "again",
            "re-do",
            "redo",
            "retry",
        ];
        for pattern in &redundant_patterns {
            if content_lower.contains(pattern) {
                score -= 0.2;
            }
        }

        if !previous_steps.is_empty() {
            let prev_contents: Vec<&str> = previous_steps
                .iter()
                .rev()
                .take(5)
                .map(|s| s.as_str())
                .collect();

            for prev in &prev_contents {
                let similarity = Self::text_similarity(content, prev);
                if similarity > 0.8 {
                    score -= 0.3;
                } else if similarity > 0.6 {
                    score -= 0.1;
                }
            }
        }

        if content.len() > 500 {
            let word_count = content.split_whitespace().count();
            let density = if word_count > 0 {
                let unique_words = content
                    .split_whitespace()
                    .collect::<std::collections::HashSet<_>>()
                    .len();
                unique_words as f64 / word_count as f64
            } else {
                0.5
            };
            if density < 0.3 {
                score -= 0.15;
            }
        }

        let efficiency_indicators = ["directly", "simply", "concise", "optimal", "efficient"];
        for indicator in &efficiency_indicators {
            if content_lower.contains(indicator) {
                score += 0.1;
                break;
            }
        }

        score.clamp(0.0, 1.0)
    }

    fn evaluate_safety(&self, content: &str) -> f64 {
        let mut score: f64 = 1.0;

        let dangerous_patterns = [
            "rm -rf",
            "delete all",
            "drop table",
            "truncate",
            "format",
            "shutdown",
            "sudo",
            "chmod 777",
            "curl | sh",
            "wget | bash",
            "eval(",
            "exec(",
            "system(",
            "unsafe",
            "unwrap()",
            "force",
            "override",
            "bypass",
        ];

        let content_lower = content.to_lowercase();
        for pattern in &dangerous_patterns {
            if content_lower.contains(pattern) {
                score -= 0.3;
            }
        }

        let caution_indicators = [
            "check",
            "verify",
            "validate",
            "confirm",
            "ensure",
            "carefully",
            "safely",
            "backup",
        ];
        for indicator in &caution_indicators {
            if content_lower.contains(indicator) {
                score += 0.05;
            }
        }

        score.clamp(0.0, 1.0)
    }

    fn text_similarity(a: &str, b: &str) -> f64 {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();
        let a_words: std::collections::HashSet<&str> = a_lower.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b_lower.split_whitespace().collect();

        if a_words.is_empty() && b_words.is_empty() {
            return 1.0;
        }
        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let intersection = a_words.intersection(&b_words).count();
        let union = a_words.union(&b_words).count();

        if union == 0 {
            return 0.0;
        }

        intersection as f64 / union as f64
    }
}

impl PrmLlmProvider for DefaultPrmProvider {
    fn evaluate_step(
        &self,
        step_content: &str,
        context: &str,
        previous_steps: &[String],
    ) -> Pin<Box<dyn Future<Output = Result<StepReward, String>> + Send + '_>> {
        let correctness = self.evaluate_correctness(step_content);
        let coherence = self.evaluate_coherence(step_content, previous_steps);
        let completeness = self.evaluate_completeness(step_content, context);
        let efficiency = self.evaluate_efficiency(step_content, previous_steps);
        let safety = self.evaluate_safety(step_content);

        let categories = vec![
            (RewardCategory::Correctness, correctness),
            (RewardCategory::Coherence, coherence),
            (RewardCategory::Completeness, completeness),
            (RewardCategory::Efficiency, efficiency),
            (RewardCategory::Safety, safety),
        ];

        let reward: f64 = categories
            .iter()
            .map(|(cat, score)| cat.weight() * score)
            .sum();

        let reasoning = format!(
            "correctness={:.2} coherence={:.2} completeness={:.2} efficiency={:.2} safety={:.2}",
            correctness, coherence, completeness, efficiency, safety
        );

        let step_index = 0;

        Box::pin(async move {
            Ok(StepReward {
                step_index,
                reward,
                reasoning,
                categories,
            })
        })
    }
}

pub struct ProcessRewardModel {
    config: ProcessRewardConfig,
    provider: Option<Box<dyn PrmLlmProvider>>,
}

impl ProcessRewardModel {
    pub fn new(config: ProcessRewardConfig) -> Self {
        Self {
            config,
            provider: None,
        }
    }

    pub fn with_provider(mut self, provider: Box<dyn PrmLlmProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn set_provider(&mut self, provider: Box<dyn PrmLlmProvider>) {
        self.provider = Some(provider);
    }

    pub fn has_provider(&self) -> bool {
        self.provider.is_some()
    }

    pub fn with_default_provider(mut self, task_context: &str) -> Self {
        self.provider = Some(Box::new(DefaultPrmProvider::new(task_context)));
        self
    }

    pub async fn compute_trajectory_rewards(&self, trajectory: &Trajectory) -> ProcessRewardResult {
        let mut step_rewards = Vec::with_capacity(trajectory.steps.len());

        let provider: &dyn PrmLlmProvider = self
            .provider
            .as_ref()
            .map(|p| p.as_ref())
            .unwrap_or_else(|| {
                static DEFAULT: DefaultPrmProvider = DefaultPrmProvider {
                    task_context: String::new(),
                };
                &DEFAULT
            });

        let mut previous_contents: Vec<String> = Vec::new();

        for (i, step) in trajectory.steps.iter().enumerate() {
            let content = &step.content;
            let context = &trajectory.topic;

            let reward = provider
                .evaluate_step(content, context, &previous_contents)
                .await;

            let step_reward = match reward {
                Ok(mut sr) => {
                    sr.step_index = i;
                    sr.reward = sr.reward.max(self.config.min_step_reward);
                    sr
                },
                Err(e) => StepReward {
                    step_index: i,
                    reward: self.config.min_step_reward,
                    reasoning: format!("evaluation error: {}", e),
                    categories: vec![],
                },
            };

            previous_contents.push(content.clone());

            if previous_contents.len() > self.config.coherence_window {
                let drain_count = previous_contents.len() - self.config.coherence_window;
                previous_contents.drain(..drain_count);
            }

            step_rewards.push(step_reward);
        }

        let aggregate_reward = if !step_rewards.is_empty() {
            step_rewards.iter().map(|sr| sr.reward).sum::<f64>() / step_rewards.len() as f64
        } else {
            0.0
        };

        let outcome_reward = match trajectory.outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure => 0.0,
            TrajectoryOutcome::Abandoned => 0.1,
        };

        let weighted_reward = aggregate_reward * self.config.step_weight
            + outcome_reward * self.config.outcome_weight;

        ProcessRewardResult {
            step_rewards,
            aggregate_reward,
            outcome_reward,
            weighted_reward,
        }
    }
}

impl Default for ProcessRewardModel {
    fn default() -> Self {
        Self::new(ProcessRewardConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trajectory::{MessageRole, TrajectoryStep};

    fn make_test_trajectory(
        steps: Vec<(&str, Option<&str>)>,
        outcome: TrajectoryOutcome,
    ) -> Trajectory {
        let trajectory_steps: Vec<TrajectoryStep> = steps
            .iter()
            .enumerate()
            .map(|(i, (content, reasoning))| TrajectoryStep {
                timestamp_ms: (i as u64 + 1) * 1000,
                role: if i == 0 {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                },
                content: content.to_string(),
                reasoning: reasoning.map(|r| r.to_string()),
                tool_calls: None,
                tool_results: None,
            })
            .collect();

        Trajectory::new(
            "session_1".into(),
            "user_1".into(),
            "Implement a sorting algorithm".into(),
            "Test trajectory for PRM".into(),
            outcome,
            5000,
            trajectory_steps,
        )
    }

    #[test]
    fn test_reward_category_weights() {
        let total: f64 = [
            RewardCategory::Correctness,
            RewardCategory::Coherence,
            RewardCategory::Completeness,
            RewardCategory::Efficiency,
            RewardCategory::Safety,
        ]
        .iter()
        .map(|c| c.weight())
        .sum();
        assert!((total - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reward_category_labels() {
        assert_eq!(RewardCategory::Correctness.label(), "correctness");
        assert_eq!(RewardCategory::Coherence.label(), "coherence");
        assert_eq!(RewardCategory::Completeness.label(), "completeness");
        assert_eq!(RewardCategory::Efficiency.label(), "efficiency");
        assert_eq!(RewardCategory::Safety.label(), "safety");
    }

    #[test]
    fn test_process_reward_config_default() {
        let config = ProcessRewardConfig::default();
        assert!((config.step_weight - 0.6).abs() < f64::EPSILON);
        assert!((config.outcome_weight - 0.4).abs() < f64::EPSILON);
        assert_eq!(config.coherence_window, 3);
        assert!((config.min_step_reward - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_prm_correctness_deduction() {
        let provider = DefaultPrmProvider::new("test task");
        let score =
            provider.evaluate_correctness("Therefore the result is 42 because we computed it");
        assert!(score > 0.3);
    }

    #[test]
    fn test_default_prm_correctness_no_deduction() {
        let provider = DefaultPrmProvider::new("test task");
        let score = provider.evaluate_correctness("random text without logic");
        assert!(score <= 0.5);
    }

    #[test]
    fn test_default_prm_coherence_with_context() {
        let provider = DefaultPrmProvider::new("test task");
        let prev = vec!["We read the file and found the data".to_string()];
        let score =
            provider.evaluate_coherence("Using the data from the file, we process it", &prev);
        assert!(score > 0.5);
    }

    #[test]
    fn test_default_prm_coherence_no_context() {
        let provider = DefaultPrmProvider::new("test task");
        let score = provider.evaluate_coherence("some step content", &[]);
        assert!((score - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_prm_completeness() {
        let provider = DefaultPrmProvider::new("sorting algorithm");
        let score = provider.evaluate_completeness(
            "We implement the sorting algorithm by comparing elements",
            "sorting algorithm",
        );
        assert!(score > 0.3);
    }

    #[test]
    fn test_default_prm_completeness_incomplete() {
        let provider = DefaultPrmProvider::new("test task");
        let score = provider.evaluate_completeness("TODO: implement this later", "test task");
        assert!(score < 0.5);
    }

    #[test]
    fn test_default_prm_efficiency_redundant() {
        let provider = DefaultPrmProvider::new("test task");
        let prev = vec!["Read the file and extract data".to_string()];
        let score = provider.evaluate_efficiency("Read the file and extract data again", &prev);
        assert!(score < 0.7);
    }

    #[test]
    fn test_default_prm_efficiency_clean() {
        let provider = DefaultPrmProvider::new("test task");
        let score = provider.evaluate_efficiency("Process the extracted data concisely", &[]);
        assert!(score > 0.5);
    }

    #[test]
    fn test_default_prm_safety_dangerous() {
        let provider = DefaultPrmProvider::new("test task");
        let score = provider.evaluate_safety("Run rm -rf / to clean up");
        assert!(score < 1.0);
    }

    #[test]
    fn test_default_prm_safety_safe() {
        let provider = DefaultPrmProvider::new("test task");
        let score = provider.evaluate_safety("Verify the output and check the result safely");
        assert!(score > 0.9);
    }

    #[test]
    fn test_text_similarity_identical() {
        let sim = DefaultPrmProvider::text_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_text_similarity_different() {
        let sim = DefaultPrmProvider::text_similarity("hello world", "foo bar baz");
        assert!(sim < 0.3);
    }

    #[test]
    fn test_text_similarity_empty() {
        let sim = DefaultPrmProvider::text_similarity("", "");
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_default_prm_provider_evaluate_step() {
        let provider = DefaultPrmProvider::new("sorting algorithm");
        let result = provider
            .evaluate_step(
                "Therefore we sort the array by comparing adjacent elements",
                "sorting algorithm",
                &[],
            )
            .await
            .unwrap();
        assert!(result.reward > 0.0);
        assert_eq!(result.categories.len(), 5);
        assert!(result.reasoning.contains("correctness="));
    }

    #[tokio::test]
    async fn test_process_reward_model_success_trajectory() {
        let model =
            ProcessRewardModel::default().with_default_provider("Implement a sorting algorithm");
        let trajectory = make_test_trajectory(
            vec![
                ("I need to implement a sorting algorithm", Some("Planning the approach")),
                (
                    "Therefore I will use quicksort because it is efficient",
                    Some("Choosing algorithm"),
                ),
                ("Now I implement the partition function using the pivot", Some("Implementing")),
                ("I verify the output by checking the sorted result", Some("Verifying")),
            ],
            TrajectoryOutcome::Success,
        );
        let result = model.compute_trajectory_rewards(&trajectory).await;
        assert!(!result.step_rewards.is_empty());
        assert!(result.aggregate_reward > 0.0);
        assert!((result.outcome_reward - 1.0).abs() < f64::EPSILON);
        assert!(result.weighted_reward > 0.0);
    }

    #[tokio::test]
    async fn test_process_reward_model_failure_trajectory() {
        let model =
            ProcessRewardModel::default().with_default_provider("Implement a sorting algorithm");
        let trajectory = make_test_trajectory(
            vec![
                ("I will try something", None),
                ("rm -rf / to clean up", None),
            ],
            TrajectoryOutcome::Failure,
        );
        let result = model.compute_trajectory_rewards(&trajectory).await;
        assert!(!result.step_rewards.is_empty());
        assert!((result.outcome_reward - 0.0).abs() < f64::EPSILON);
        let safety_step = &result.step_rewards[1];
        let safety_score = safety_step
            .categories
            .iter()
            .find(|(c, _)| *c == RewardCategory::Safety)
            .map(|(_, s)| *s)
            .unwrap_or(1.0);
        assert!(safety_score < 1.0);
    }

    #[tokio::test]
    async fn test_process_reward_model_empty_trajectory() {
        let model = ProcessRewardModel::default();
        let trajectory = make_test_trajectory(vec![], TrajectoryOutcome::Success);
        let result = model.compute_trajectory_rewards(&trajectory).await;
        assert!(result.step_rewards.is_empty());
        assert!((result.aggregate_reward - 0.0).abs() < f64::EPSILON);
        assert!((result.outcome_reward - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_process_reward_model_weighted_reward() {
        let config = ProcessRewardConfig {
            step_weight: 0.7,
            outcome_weight: 0.3,
            ..ProcessRewardConfig::default()
        };
        let model = ProcessRewardModel::new(config).with_default_provider("test task");
        let trajectory = make_test_trajectory(
            vec![("Therefore we compute the result", Some("reasoning"))],
            TrajectoryOutcome::Success,
        );
        let result = model.compute_trajectory_rewards(&trajectory).await;
        let expected = result.aggregate_reward * 0.7 + result.outcome_reward * 0.3;
        assert!((result.weighted_reward - expected).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_process_reward_model_min_step_reward() {
        let config = ProcessRewardConfig {
            min_step_reward: 0.5,
            ..ProcessRewardConfig::default()
        };
        let model = ProcessRewardModel::new(config).with_default_provider("test task");
        let trajectory = make_test_trajectory(vec![("x", None)], TrajectoryOutcome::Success);
        let result = model.compute_trajectory_rewards(&trajectory).await;
        for sr in &result.step_rewards {
            assert!(sr.reward >= 0.5);
        }
    }

    #[test]
    fn test_step_reward_serialization() {
        let sr = StepReward {
            step_index: 2,
            reward: 0.85,
            reasoning: "high quality step".into(),
            categories: vec![
                (RewardCategory::Correctness, 0.9),
                (RewardCategory::Coherence, 0.8),
            ],
        };
        let json = serde_json::to_string(&sr).unwrap();
        assert!(json.contains("Correctness"));
        let deserialized: StepReward = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.step_index, 2);
        assert!((deserialized.reward - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_process_reward_result_serialization() {
        let result = ProcessRewardResult {
            step_rewards: vec![StepReward {
                step_index: 0,
                reward: 0.7,
                reasoning: "test".into(),
                categories: vec![(RewardCategory::Correctness, 0.7)],
            }],
            aggregate_reward: 0.7,
            outcome_reward: 1.0,
            weighted_reward: 0.82,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ProcessRewardResult = serde_json::from_str(&json).unwrap();
        assert!((deserialized.aggregate_reward - 0.7).abs() < f64::EPSILON);
        assert!((deserialized.outcome_reward - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_process_reward_model_abandoned_trajectory() {
        let model = ProcessRewardModel::default().with_default_provider("test task");
        let trajectory =
            make_test_trajectory(vec![("Starting task", None)], TrajectoryOutcome::Abandoned);
        let result = model.compute_trajectory_rewards(&trajectory).await;
        assert!((result.outcome_reward - 0.1).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_process_reward_model_partial_trajectory() {
        let model = ProcessRewardModel::default().with_default_provider("test task");
        let trajectory = make_test_trajectory(
            vec![("Partial work done", Some("thinking"))],
            TrajectoryOutcome::Partial,
        );
        let result = model.compute_trajectory_rewards(&trajectory).await;
        assert!((result.outcome_reward - 0.5).abs() < f64::EPSILON);
    }

    struct MockPrmProvider {
        fixed_reward: f64,
    }

    impl PrmLlmProvider for MockPrmProvider {
        fn evaluate_step(
            &self,
            _step_content: &str,
            _context: &str,
            _previous_steps: &[String],
        ) -> Pin<Box<dyn Future<Output = Result<StepReward, String>> + Send + '_>> {
            let reward = self.fixed_reward;
            Box::pin(async move {
                Ok(StepReward {
                    step_index: 0,
                    reward,
                    reasoning: "mock evaluation".into(),
                    categories: vec![
                        (RewardCategory::Correctness, reward),
                        (RewardCategory::Coherence, reward),
                        (RewardCategory::Completeness, reward),
                        (RewardCategory::Efficiency, reward),
                        (RewardCategory::Safety, reward),
                    ],
                })
            })
        }
    }

    #[tokio::test]
    async fn test_process_reward_model_with_custom_provider() {
        let mut model = ProcessRewardModel::new(ProcessRewardConfig::default());
        model.set_provider(Box::new(MockPrmProvider { fixed_reward: 0.8 }));
        let trajectory = make_test_trajectory(
            vec![("Step 1", None), ("Step 2", None), ("Step 3", None)],
            TrajectoryOutcome::Success,
        );
        let result = model.compute_trajectory_rewards(&trajectory).await;
        assert!((result.aggregate_reward - 0.8).abs() < f64::EPSILON);
        assert_eq!(result.step_rewards.len(), 3);
    }
}
