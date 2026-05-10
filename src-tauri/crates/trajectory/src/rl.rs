//! RL reward signal computation module
//!
//! Provides research-grade reinforcement learning reward computation including:
//! - Multi-dimensional reward signals
//! - Reward shaping
//! - Temporal difference learning
//! - Policy gradient estimation

#![allow(clippy::unwrap_used)]

use crate::trajectory::{
    MessageRole, RewardSignal, RewardType, Trajectory, TrajectoryOutcome, TrajectoryStep,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLConfig {
    pub gamma: f64,
    pub lambda: f64,
    pub reward_scale: f64,
    pub entropy_coefficient: f64,
    pub value_coefficient: f64,
    pub use_td_lambda: bool,
}

impl Default for RLConfig {
    fn default() -> Self {
        Self {
            gamma: 0.99,
            lambda: 0.95,
            reward_scale: 1.0,
            entropy_coefficient: 0.01,
            value_coefficient: 0.5,
            use_td_lambda: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardWeights {
    pub task_completion: f64,
    pub tool_efficiency: f64,
    pub reasoning_quality: f64,
    pub error_recovery: f64,
    pub user_feedback: f64,
    pub pattern_match: f64,
}

impl Default for RewardWeights {
    fn default() -> Self {
        Self {
            task_completion: 0.4,
            tool_efficiency: 0.2,
            reasoning_quality: 0.15,
            error_recovery: 0.15,
            user_feedback: 0.05,
            pattern_match: 0.05,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RLState {
    pub values: Vec<f64>,
    pub advantages: Vec<f64>,
    pub returns: Vec<f64>,
    pub policy_logits: Vec<f64>,
}

impl RLState {
    pub fn new(steps: usize) -> Self {
        Self {
            values: vec![0.0; steps],
            advantages: vec![0.0; steps],
            returns: vec![0.0; steps],
            policy_logits: vec![0.0; steps],
        }
    }
}

pub type LlmJudgeFuture<'a> = Pin<Box<dyn Future<Output = Result<f64, String>> + Send + 'a>>;

pub trait LlmJudge: Send + Sync {
    fn evaluate_reasoning(&self, reasoning: &str, context: &str) -> LlmJudgeFuture<'_>;
    fn evaluate_tool_efficiency(
        &self,
        tool_name: &str,
        args: &str,
        result: &str,
    ) -> LlmJudgeFuture<'_>;
}

pub struct DefaultLlmJudge;

impl LlmJudge for DefaultLlmJudge {
    fn evaluate_reasoning(&self, reasoning: &str, _context: &str) -> LlmJudgeFuture<'_> {
        let length_score = (reasoning.len() as f64 / 500.0).min(1.0) * 0.2;

        let structure_indicators = [
            "first",
            "then",
            "next",
            "finally",
            "because",
            "therefore",
            "however",
        ];
        let structure_count = structure_indicators
            .iter()
            .filter(|ind| reasoning.to_lowercase().contains(*ind))
            .count() as f64;
        let structure_score = (structure_count / 7.0).min(1.0) * 0.4;

        let has_alternatives = reasoning.to_lowercase().contains("alternative")
            || reasoning.to_lowercase().contains("option")
            || reasoning.to_lowercase().contains("could");
        let alternatives_score = if has_alternatives { 0.2 } else { 0.0 };

        let has_reflection = reasoning.to_lowercase().contains("should")
            || reasoning.to_lowercase().contains("consider")
            || reasoning.to_lowercase().contains("might");
        let reflection_score = if has_reflection { 0.2 } else { 0.0 };

        let score = (length_score + structure_score + alternatives_score + reflection_score)
            .clamp(0.0, 1.0);
        Box::pin(async move { Ok(score) })
    }

    fn evaluate_tool_efficiency(
        &self,
        tool_name: &str,
        args: &str,
        result: &str,
    ) -> LlmJudgeFuture<'_> {
        let name_relevance = if tool_name.is_empty() { 0.3 } else { 0.5 };
        let args_quality = if args.is_empty() {
            0.3
        } else {
            (args.len() as f64 / 200.0).min(1.0) * 0.3
        };
        let result_quality = if result.is_empty() { 0.2 } else { 0.5 };
        let score = (name_relevance + args_quality + result_quality).clamp(0.0, 1.0);
        Box::pin(async move { Ok(score) })
    }
}

pub struct RLEngine {
    config: RLConfig,
    weights: RewardWeights,
    llm_judge: Option<Box<dyn LlmJudge>>,
}

impl RLEngine {
    pub fn new(config: RLConfig, weights: RewardWeights) -> Self {
        Self {
            config,
            weights,
            llm_judge: None,
        }
    }

    pub fn with_llm_judge(mut self, judge: Box<dyn LlmJudge>) -> Self {
        self.llm_judge = Some(judge);
        self
    }

    pub fn set_llm_judge(&mut self, judge: Box<dyn LlmJudge>) {
        self.llm_judge = Some(judge);
    }

    pub fn config(&self) -> &RLConfig {
        &self.config
    }

    pub fn weights(&self) -> &RewardWeights {
        &self.weights
    }

    pub fn llm_judge(&self) -> Option<&dyn LlmJudge> {
        self.llm_judge.as_ref().map(|j| j.as_ref())
    }

    pub fn compute_rewards(&self, trajectory: &mut Trajectory) -> Vec<RewardSignal> {
        let mut rewards = Vec::new();
        let steps = &trajectory.steps;

        for (i, step) in steps.iter().enumerate() {
            let mut step_rewards = Vec::new();

            if let Some(ref tool_calls) = step.tool_calls {
                let efficiency = self.compute_tool_efficiency(step, tool_calls.len());
                step_rewards.push(RewardSignal {
                    reward_type: RewardType::ToolEfficiency,
                    value: efficiency * self.weights.tool_efficiency * self.config.reward_scale,
                    step_index: i,
                    timestamp_ms: step.timestamp_ms,
                    metadata: serde_json::json!({
                        "tool_count": tool_calls.len(),
                        "raw_efficiency": efficiency
                    }),
                });
            }

            if let Some(ref reasoning) = step.reasoning {
                let quality = self.compute_reasoning_quality(reasoning);
                step_rewards.push(RewardSignal {
                    reward_type: RewardType::ReasoningQuality,
                    value: quality * self.weights.reasoning_quality * self.config.reward_scale,
                    step_index: i,
                    timestamp_ms: step.timestamp_ms,
                    metadata: serde_json::json!({
                        "reasoning_length": reasoning.len(),
                        "quality_score": quality
                    }),
                });
            }

            if let Some(ref results) = step.tool_results {
                let error_recovery = self.compute_error_recovery(steps, i, results);
                if error_recovery > 0.0 {
                    step_rewards.push(RewardSignal {
                        reward_type: RewardType::ErrorRecovery,
                        value: error_recovery
                            * self.weights.error_recovery
                            * self.config.reward_scale,
                        step_index: i,
                        timestamp_ms: step.timestamp_ms,
                        metadata: serde_json::json!({"recovered": true}),
                    });
                }
            }

            for pattern in &trajectory.patterns {
                step_rewards.push(RewardSignal {
                    reward_type: RewardType::PatternMatch,
                    value: 0.05 * self.weights.pattern_match * self.config.reward_scale,
                    step_index: i,
                    timestamp_ms: step.timestamp_ms,
                    metadata: serde_json::json!({"pattern": pattern}),
                });
            }

            rewards.extend(step_rewards);
        }

        let final_reward = self.compute_final_reward(trajectory);
        rewards.push(RewardSignal {
            reward_type: RewardType::TaskCompletion,
            value: final_reward * self.weights.task_completion * self.config.reward_scale,
            step_index: steps.len().saturating_sub(1),
            timestamp_ms: steps.last().map(|s| s.timestamp_ms).unwrap_or(0),
            metadata: serde_json::json!({
                "outcome": format!("{:?}", trajectory.outcome),
                "final": true
            }),
        });

        trajectory.rewards = rewards.clone();
        rewards
    }

    pub async fn compute_rewards_v2(&self, trajectory: &mut Trajectory) -> Vec<RewardSignal> {
        let mut rewards = Vec::new();
        let steps = &trajectory.steps;

        for (i, step) in steps.iter().enumerate() {
            let mut step_rewards = Vec::new();

            if let Some(ref tool_calls) = step.tool_calls {
                let heuristic_efficiency = self.compute_tool_efficiency(step, tool_calls.len());

                if let Some(ref judge) = self.llm_judge {
                    let mut llm_scores = Vec::new();
                    for tc in tool_calls {
                        let result_str = step
                            .tool_results
                            .as_ref()
                            .and_then(|results| {
                                results
                                    .iter()
                                    .find(|r| r.tool_use_id == tc.id)
                                    .map(|r| r.output.as_str())
                            })
                            .unwrap_or("");
                        match judge
                            .evaluate_tool_efficiency(&tc.name, &tc.arguments, result_str)
                            .await
                        {
                            Ok(score) => llm_scores.push(score),
                            Err(_) => llm_scores.push(heuristic_efficiency),
                        }
                    }
                    let llm_avg = if llm_scores.is_empty() {
                        heuristic_efficiency
                    } else {
                        llm_scores.iter().sum::<f64>() / llm_scores.len() as f64
                    };

                    step_rewards.push(RewardSignal {
                        reward_type: RewardType::LlmToolEfficiency,
                        value: llm_avg * self.weights.tool_efficiency * self.config.reward_scale,
                        step_index: i,
                        timestamp_ms: step.timestamp_ms,
                        metadata: serde_json::json!({
                            "tool_count": tool_calls.len(),
                            "llm_score": llm_avg,
                            "heuristic_score": heuristic_efficiency
                        }),
                    });

                    step_rewards.push(RewardSignal {
                        reward_type: RewardType::ToolEfficiency,
                        value: heuristic_efficiency
                            * self.weights.tool_efficiency
                            * self.config.reward_scale,
                        step_index: i,
                        timestamp_ms: step.timestamp_ms,
                        metadata: serde_json::json!({
                            "tool_count": tool_calls.len(),
                            "raw_efficiency": heuristic_efficiency
                        }),
                    });
                } else {
                    step_rewards.push(RewardSignal {
                        reward_type: RewardType::ToolEfficiency,
                        value: heuristic_efficiency
                            * self.weights.tool_efficiency
                            * self.config.reward_scale,
                        step_index: i,
                        timestamp_ms: step.timestamp_ms,
                        metadata: serde_json::json!({
                            "tool_count": tool_calls.len(),
                            "raw_efficiency": heuristic_efficiency
                        }),
                    });
                }
            }

            if let Some(ref reasoning) = step.reasoning {
                let heuristic_quality = self.compute_reasoning_quality(reasoning);

                if let Some(ref judge) = self.llm_judge {
                    let context = step.content.as_str();
                    if let Ok(llm_score) = judge.evaluate_reasoning(reasoning, context).await {
                        step_rewards.push(RewardSignal {
                            reward_type: RewardType::LlmReasoningQuality,
                            value: llm_score
                                * self.weights.reasoning_quality
                                * self.config.reward_scale,
                            step_index: i,
                            timestamp_ms: step.timestamp_ms,
                            metadata: serde_json::json!({
                                "reasoning_length": reasoning.len(),
                                "llm_score": llm_score,
                                "heuristic_score": heuristic_quality
                            }),
                        });
                    }

                    step_rewards.push(RewardSignal {
                        reward_type: RewardType::ReasoningQuality,
                        value: heuristic_quality
                            * self.weights.reasoning_quality
                            * self.config.reward_scale,
                        step_index: i,
                        timestamp_ms: step.timestamp_ms,
                        metadata: serde_json::json!({
                            "reasoning_length": reasoning.len(),
                            "quality_score": heuristic_quality
                        }),
                    });
                } else {
                    step_rewards.push(RewardSignal {
                        reward_type: RewardType::ReasoningQuality,
                        value: heuristic_quality
                            * self.weights.reasoning_quality
                            * self.config.reward_scale,
                        step_index: i,
                        timestamp_ms: step.timestamp_ms,
                        metadata: serde_json::json!({
                            "reasoning_length": reasoning.len(),
                            "quality_score": heuristic_quality
                        }),
                    });
                }
            }

            if let Some(ref results) = step.tool_results {
                let error_recovery = self.compute_error_recovery(steps, i, results);
                if error_recovery > 0.0 {
                    step_rewards.push(RewardSignal {
                        reward_type: RewardType::ErrorRecovery,
                        value: error_recovery
                            * self.weights.error_recovery
                            * self.config.reward_scale,
                        step_index: i,
                        timestamp_ms: step.timestamp_ms,
                        metadata: serde_json::json!({"recovered": true}),
                    });
                }
            }

            for pattern in &trajectory.patterns {
                step_rewards.push(RewardSignal {
                    reward_type: RewardType::PatternMatch,
                    value: 0.05 * self.weights.pattern_match * self.config.reward_scale,
                    step_index: i,
                    timestamp_ms: step.timestamp_ms,
                    metadata: serde_json::json!({"pattern": pattern}),
                });
            }

            rewards.extend(step_rewards);
        }

        let final_reward = self.compute_final_reward(trajectory);
        rewards.push(RewardSignal {
            reward_type: RewardType::TaskCompletion,
            value: final_reward * self.weights.task_completion * self.config.reward_scale,
            step_index: steps.len().saturating_sub(1),
            timestamp_ms: steps.last().map(|s| s.timestamp_ms).unwrap_or(0),
            metadata: serde_json::json!({
                "outcome": format!("{:?}", trajectory.outcome),
                "final": true
            }),
        });

        trajectory.rewards = rewards.clone();
        rewards
    }

    fn compute_tool_efficiency(&self, step: &TrajectoryStep, tool_count: usize) -> f64 {
        let base = if tool_count == 0 {
            0.5
        } else {
            1.0 / tool_count as f64
        };

        let success_rate = step
            .tool_results
            .as_ref()
            .map(|results| {
                let successful = results.iter().filter(|r| !r.is_error).count();
                successful as f64 / results.len().max(1) as f64
            })
            .unwrap_or(0.5);

        base * 0.3 + success_rate * 0.7
    }

    fn compute_reasoning_quality(&self, reasoning: &str) -> f64 {
        let length_score = (reasoning.len() as f64 / 500.0).min(1.0) * 0.2;

        let structure_indicators = [
            "first",
            "then",
            "next",
            "finally",
            "because",
            "therefore",
            "however",
        ]
        .iter()
        .filter(|ind| reasoning.to_lowercase().contains(*ind))
        .count() as f64;
        let structure_score = (structure_indicators / 7.0).min(1.0) * 0.4;

        let has_alternatives = reasoning.to_lowercase().contains("alternative")
            || reasoning.to_lowercase().contains("option")
            || reasoning.to_lowercase().contains("could");
        let alternatives_score = if has_alternatives { 0.2 } else { 0.0 };

        let has_reflection = reasoning.to_lowercase().contains("should")
            || reasoning.to_lowercase().contains("consider")
            || reasoning.to_lowercase().contains("might");
        let reflection_score = if has_reflection { 0.2 } else { 0.0 };

        (length_score + structure_score + alternatives_score + reflection_score).clamp(0.0, 1.0)
    }

    fn compute_error_recovery(
        &self,
        steps: &[TrajectoryStep],
        current_idx: usize,
        results: &[crate::trajectory::ToolResult],
    ) -> f64 {
        let has_errors = results.iter().any(|r| r.is_error);
        if !has_errors {
            return 0.0;
        }

        let next_steps = &steps[current_idx + 1..];
        let has_recovery = next_steps.iter().any(|s| {
            s.tool_results
                .as_ref()
                .map(|r| r.iter().any(|tr| !tr.is_error))
                .unwrap_or(false)
        });

        if has_recovery {
            let steps_to_recovery = next_steps
                .iter()
                .position(|s| {
                    s.tool_results
                        .as_ref()
                        .map(|r| r.iter().any(|tr| !tr.is_error))
                        .unwrap_or(false)
                })
                .unwrap_or(0);

            (1.0 / (steps_to_recovery as f64 + 1.0)).min(1.0)
        } else {
            0.0
        }
    }

    fn compute_final_reward(&self, trajectory: &Trajectory) -> f64 {
        match trajectory.outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => {
                let completion_ratio = trajectory.quality.task_completion;
                0.3 + completion_ratio * 0.4
            },
            TrajectoryOutcome::Failure => {
                let error_count = trajectory
                    .steps
                    .iter()
                    .filter(|s| {
                        s.tool_results
                            .as_ref()
                            .map(|r| r.iter().any(|tr| tr.is_error))
                            .unwrap_or(false)
                    })
                    .count();
                (-0.5f64).max(-1.0 + error_count as f64 * 0.1)
            },
            TrajectoryOutcome::Abandoned => -0.2,
        }
    }

    pub fn compute_advantages(&self, rewards: &[RewardSignal], values: &[f64]) -> Vec<f64> {
        let mut advantages = vec![0.0; values.len()];

        if self.config.use_td_lambda {
            let (td_errors, _) = self.compute_td_lambda(rewards, values);
            advantages = td_errors;
        } else {
            let mut cumulative = 0.0;
            for t in (0..values.len()).rev() {
                let reward_t = rewards.get(t).map(|r| r.value).unwrap_or(0.0);
                let next_value = values.get(t + 1).copied().unwrap_or(0.0);
                let td_error = reward_t + self.config.gamma * next_value - values[t];
                cumulative = td_error + self.config.gamma * self.config.lambda * cumulative;
                advantages[t] = cumulative;
            }
        }

        let std = Self::standard_deviation(&advantages);
        if std > 0.0 {
            advantages.iter_mut().for_each(|a| *a /= std);
        }

        advantages
    }

    fn compute_td_lambda(&self, rewards: &[RewardSignal], values: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let mut td_errors = vec![0.0; values.len()];
        let mut returns = vec![0.0; values.len()];

        let mut lambda_return = 0.0;
        for t in (0..values.len()).rev() {
            let reward_t = rewards.get(t).map(|r| r.value).unwrap_or(0.0);
            let next_value = values.get(t + 1).copied().unwrap_or(0.0);
            let delta = reward_t + self.config.gamma * next_value - values[t];
            lambda_return = delta + self.config.gamma * self.config.lambda * lambda_return;
            td_errors[t] = lambda_return;
            returns[t] = lambda_return + values[t];
        }

        (td_errors, returns)
    }

    fn standard_deviation(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        variance.sqrt()
    }

    pub fn estimate_value_function(&self, trajectory: &Trajectory) -> Vec<f64> {
        let steps = trajectory.steps.len();
        let mut values = vec![0.0; steps];

        for (i, _) in trajectory.steps.iter().enumerate() {
            let future_rewards: f64 = trajectory
                .rewards
                .iter()
                .filter(|r| r.step_index >= i)
                .map(|r| r.value)
                .sum();

            let discount_factor = self.config.gamma.powi((steps - i) as i32);
            values[i] = future_rewards * discount_factor;
        }

        values
    }

    pub fn compute_policy_gradient(
        &self,
        trajectory: &Trajectory,
        advantages: &[f64],
    ) -> HashMap<String, f64> {
        let mut gradients = HashMap::new();

        for (i, step) in trajectory.steps.iter().enumerate() {
            let advantage = advantages.get(i).copied().unwrap_or(0.0);

            if step.tool_calls.is_some() {
                *gradients.entry("tool_usage".to_string()).or_insert(0.0) += advantage;
            }

            if step.reasoning.is_some() {
                *gradients.entry("reasoning".to_string()).or_insert(0.0) += advantage * 0.5;
            }

            if step.role == MessageRole::User {
                *gradients
                    .entry("user_engagement".to_string())
                    .or_insert(0.0) += advantage * 0.3;
            }
        }

        gradients
    }

    pub fn shape_rewards(&self, rewards: &mut [RewardSignal]) {
        for reward in rewards.iter_mut() {
            let shaping_bonus = match reward.reward_type {
                RewardType::TaskCompletion => 0.0,
                RewardType::ToolEfficiency => {
                    if reward.value > 0.1 {
                        self.config.entropy_coefficient * 0.1
                    } else {
                        0.0
                    }
                },
                RewardType::ReasoningQuality => self.config.entropy_coefficient * 0.05,
                RewardType::ErrorRecovery => 0.2 * self.config.entropy_coefficient,
                RewardType::UserFeedback => 0.1 * self.config.entropy_coefficient,
                RewardType::PatternMatch => 0.05 * self.config.entropy_coefficient,
                RewardType::LlmReasoningQuality => self.config.entropy_coefficient * 0.08,
                RewardType::LlmToolEfficiency => self.config.entropy_coefficient * 0.06,
            };

            reward.value += shaping_bonus;
        }
    }
}

pub struct RewardNormalizer {
    running_mean: f64,
    running_var: f64,
    count: u64,
    epsilon: f64,
}

impl RewardNormalizer {
    pub fn new() -> Self {
        Self {
            running_mean: 0.0,
            running_var: 1.0,
            count: 0,
            epsilon: 1e-8,
        }
    }

    pub fn normalize(&mut self, rewards: &mut [RewardSignal]) {
        if rewards.is_empty() {
            return;
        }

        let values: Vec<f64> = rewards.iter().map(|r| r.value).collect();
        let batch_mean = values.iter().sum::<f64>() / values.len() as f64;
        let _batch_var =
            values.iter().map(|v| (v - batch_mean).powi(2)).sum::<f64>() / values.len() as f64;

        self.count += 1;
        let delta = batch_mean - self.running_mean;
        self.running_mean += delta / self.count as f64;
        self.running_var += delta * (batch_mean - self.running_mean);

        let std = (self.running_var / (self.count - 1).max(1) as f64 + self.epsilon).sqrt();

        for reward in rewards.iter_mut() {
            reward.value = (reward.value - self.running_mean) / std;
        }
    }
}

impl Default for RewardNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trajectory::ToolCall;

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
                tool_results: Some(vec![crate::trajectory::ToolResult {
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
    fn test_compute_rewards_success() {
        let config = RLConfig::default();
        let weights = RewardWeights::default();
        let engine = RLEngine::new(config, weights);

        let mut trajectory = create_test_trajectory(TrajectoryOutcome::Success);
        let rewards = engine.compute_rewards(&mut trajectory);

        assert!(!rewards.is_empty());
        let final_reward = rewards
            .iter()
            .find(|r| r.reward_type == RewardType::TaskCompletion)
            .unwrap();
        assert!(final_reward.value > 0.0);
    }

    #[test]
    fn test_compute_rewards_failure() {
        let config = RLConfig::default();
        let weights = RewardWeights::default();
        let engine = RLEngine::new(config, weights);

        let mut trajectory = create_test_trajectory(TrajectoryOutcome::Failure);
        let rewards = engine.compute_rewards(&mut trajectory);

        let final_reward = rewards
            .iter()
            .find(|r| r.reward_type == RewardType::TaskCompletion)
            .unwrap();
        assert!(final_reward.value < 0.0);
    }

    #[test]
    fn test_reward_normalization() {
        let mut normalizer = RewardNormalizer::new();
        let mut rewards = vec![
            RewardSignal {
                reward_type: RewardType::ToolEfficiency,
                value: 0.5,
                step_index: 0,
                timestamp_ms: 1000,
                metadata: serde_json::json!({}),
            },
            RewardSignal {
                reward_type: RewardType::ToolEfficiency,
                value: 1.0,
                step_index: 1,
                timestamp_ms: 2000,
                metadata: serde_json::json!({}),
            },
        ];

        normalizer.normalize(&mut rewards);

        let mean = rewards.iter().map(|r| r.value).sum::<f64>() / rewards.len() as f64;
        assert!((mean).abs() < 0.01);
    }
}
