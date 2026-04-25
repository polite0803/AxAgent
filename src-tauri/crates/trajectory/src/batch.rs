//! Batch trajectory generation and processing

use crate::storage::TrajectoryStorage;
use crate::trajectory::{
    ExportFormat, RLTrainingEntry, RewardSignal, Trajectory, TrajectoryExportOptions,
    TrajectoryOutcome, TrajectoryQuery,
};
use anyhow::Result;
use itertools::Itertools;
use rand::prelude::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub max_concurrent: usize,
    pub quality_threshold: f64,
    pub deduplicate: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
            max_concurrent: 10,
            quality_threshold: 0.3,
            deduplicate: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchResult {
    pub total_processed: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub quality_scores: Vec<f64>,
    pub value_scores: Vec<f64>,
}

impl BatchResult {
    pub fn average_quality(&self) -> f64 {
        if self.quality_scores.is_empty() {
            return 0.0;
        }
        self.quality_scores.iter().sum::<f64>() / self.quality_scores.len() as f64
    }

    pub fn average_value_score(&self) -> f64 {
        if self.value_scores.is_empty() {
            return 0.0;
        }
        self.value_scores.iter().sum::<f64>() / self.value_scores.len() as f64
    }
}

#[derive(Debug, Clone)]
pub enum SamplingStrategy {
    Random,
    TopK(usize),
    Threshold(f64),
    Stratified {
        success_rate: f64,
        partial_rate: f64,
        failure_rate: f64,
    },
    DiversityBased,
}

pub struct BatchProcessor {
    storage: Arc<TrajectoryStorage>,
    #[allow(dead_code)]
    config: BatchConfig,
}

impl BatchProcessor {
    pub fn new(storage: Arc<TrajectoryStorage>, config: BatchConfig) -> Self {
        Self { storage, config }
    }

    pub fn batch_generate<S: AsRef<str>>(&self, session_ids: &[S]) -> Result<Vec<Trajectory>> {
        let storage = &*self.storage;
        let mut trajectories = Vec::new();
        let mut errors = Vec::new();

        for session_id in session_ids {
            match storage.get_session_trajectories(session_id.as_ref()) {
                Ok(trajs) => trajectories.extend(trajs),
                Err(e) => {
                    warn!(
                        "Failed to get trajectories for session {}: {}",
                        session_id.as_ref(),
                        e
                    );
                    errors.push(e);
                }
            }
        }

        info!(
            "Batch generation completed: {} trajectories, {} errors",
            trajectories.len(),
            errors.len()
        );

        Ok(trajectories)
    }

    pub fn filter_by_quality(
        &self,
        trajectories: &[Trajectory],
        threshold: f64,
    ) -> Vec<Trajectory> {
        trajectories
            .iter()
            .filter(|t| t.quality.overall >= threshold)
            .cloned()
            .collect()
    }

    pub fn sample_for_training(
        &self,
        trajectories: &[Trajectory],
        strategy: SamplingStrategy,
        sample_size: usize,
    ) -> Vec<Trajectory> {
        match strategy {
            SamplingStrategy::Random => {
                let mut rng = rand::thread_rng();
                let mut trajectories: Vec<_> = trajectories.iter().collect();
                trajectories.shuffle(&mut rng);
                trajectories
                    .into_iter()
                    .take(sample_size)
                    .cloned()
                    .collect()
            }
            SamplingStrategy::TopK(k) => {
                let mut trajectories: Vec<_> = trajectories.iter().collect();
                trajectories.sort_by(|a, b| {
                    b.quality
                        .overall
                        .partial_cmp(&a.quality.overall)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                trajectories.into_iter().take(k).cloned().collect()
            }
            SamplingStrategy::Threshold(threshold) => trajectories
                .iter()
                .filter(|t| t.quality.overall >= threshold)
                .cloned()
                .collect(),
            SamplingStrategy::Stratified {
                success_rate,
                partial_rate,
                failure_rate,
            } => {
                let success: Vec<_> = trajectories
                    .iter()
                    .filter(|t| t.outcome == TrajectoryOutcome::Success)
                    .cloned()
                    .collect();
                let partial: Vec<_> = trajectories
                    .iter()
                    .filter(|t| t.outcome == TrajectoryOutcome::Partial)
                    .cloned()
                    .collect();
                let failure: Vec<_> = trajectories
                    .iter()
                    .filter(|t| t.outcome == TrajectoryOutcome::Failure)
                    .cloned()
                    .collect();

                let success_count = (sample_size as f64 * success_rate) as usize;
                let partial_count = (sample_size as f64 * partial_rate) as usize;
                let failure_count = (sample_size as f64 * failure_rate) as usize;

                let mut sampled = Vec::new();
                sampled.extend(success.into_iter().take(success_count));
                sampled.extend(partial.into_iter().take(partial_count));
                sampled.extend(failure.into_iter().take(failure_count));

                sampled
            }
            SamplingStrategy::DiversityBased => self.diversity_sample(trajectories, sample_size),
        }
    }

    fn diversity_sample(&self, trajectories: &[Trajectory], sample_size: usize) -> Vec<Trajectory> {
        let mut selected = Vec::new();
        let mut topics_seen: HashMap<String, usize> = HashMap::new();
        let mut patterns_seen: HashMap<String, usize> = HashMap::new();

        let mut sorted = trajectories.to_vec();
        sorted.sort_by(|a, b| {
            b.value_score
                .partial_cmp(&a.value_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for trajectory in sorted {
            if selected.len() >= sample_size {
                break;
            }

            let topic_key = trajectory
                .topic
                .split_whitespace()
                .take(3)
                .collect::<String>();
            let topic_count = topics_seen.get(&topic_key).copied().unwrap_or(0);

            let mut pattern_overlap = 0;
            for pattern in &trajectory.patterns {
                if patterns_seen.contains_key(pattern) {
                    pattern_overlap += 1;
                }
            }

            if topic_count < 5 && pattern_overlap < 3 {
                selected.push(trajectory.clone());

                *topics_seen.entry(topic_key).or_insert(0) += 1;
                for pattern in &trajectory.patterns {
                    *patterns_seen.entry(pattern.clone()).or_insert(0) += 1;
                }
            }
        }

        selected
    }

    pub fn compute_batch_rewards(
        &self,
        trajectories: &mut [Trajectory],
        reward_weights: &HashMap<String, f64>,
    ) -> Vec<Vec<RewardSignal>> {
        let mut all_rewards = Vec::new();

        for trajectory in trajectories.iter_mut() {
            let rewards = self.compute_trajectory_rewards(trajectory, reward_weights);
            trajectory.rewards.clone_from_slice(&rewards);
            all_rewards.push(rewards);
        }

        all_rewards
    }

    fn compute_trajectory_rewards(
        &self,
        trajectory: &mut Trajectory,
        _weights: &HashMap<String, f64>,
    ) -> Vec<RewardSignal> {
        use crate::trajectory::RewardType;

        let mut rewards = Vec::new();

        for (i, step) in trajectory.steps.iter().enumerate() {
            if step.tool_calls.is_some() {
                rewards.push(RewardSignal {
                    reward_type: RewardType::ToolEfficiency,
                    value: 0.1,
                    step_index: i,
                    timestamp_ms: step.timestamp_ms,
                    metadata: serde_json::json!({
                        "tool_count": step.tool_calls.as_ref().map(|c| c.len()).unwrap_or(0)
                    }),
                });
            }

            if step.reasoning.is_some() {
                rewards.push(RewardSignal {
                    reward_type: RewardType::ReasoningQuality,
                    value: 0.15,
                    step_index: i,
                    timestamp_ms: step.timestamp_ms,
                    metadata: serde_json::json!({}),
                });
            }

            if let Some(results) = &step.tool_results {
                let has_errors = results.iter().any(|r| r.is_error);
                if !has_errors && !results.is_empty() {
                    rewards.push(RewardSignal {
                        reward_type: RewardType::ToolEfficiency,
                        value: 0.2,
                        step_index: i,
                        timestamp_ms: step.timestamp_ms,
                        metadata: serde_json::json!({"success": true}),
                    });
                } else if has_errors {
                    let error_recovery_step = trajectory.steps.get(i + 1..).and_then(|s| {
                        s.iter().position(|next| {
                            next.tool_results.as_ref().is_some_and(|r| !r.is_empty())
                        })
                    });

                    if error_recovery_step.is_some() {
                        rewards.push(RewardSignal {
                            reward_type: RewardType::ErrorRecovery,
                            value: 0.25,
                            step_index: i,
                            timestamp_ms: step.timestamp_ms,
                            metadata: serde_json::json!({"recovered": true}),
                        });
                    }
                }
            }
        }

        let final_reward = match trajectory.outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure => -0.5,
            TrajectoryOutcome::Abandoned => -0.3,
        };

        rewards.push(RewardSignal {
            reward_type: RewardType::TaskCompletion,
            value: final_reward,
            step_index: trajectory.steps.len().saturating_sub(1),
            timestamp_ms: trajectory.steps.last().map(|s| s.timestamp_ms).unwrap_or(0),
            metadata: serde_json::json!({"outcome": format!("{:?}", trajectory.outcome)}),
        });

        rewards
    }

    pub fn export_trajectories(
        &self,
        trajectories: &[Trajectory],
        options: &TrajectoryExportOptions,
    ) -> Result<String> {
        let filtered = self.filter_trajectories(trajectories, options);

        match options.format {
            ExportFormat::Jsonl => {
                let jsonl: Vec<String> = filtered
                    .iter()
                    .map(|t| serde_json::to_string(t).unwrap_or_default())
                    .collect();
                Ok(jsonl.join("\n"))
            }
            ExportFormat::RlTraining => {
                let entries: Vec<RLTrainingEntry> =
                    filtered.iter().map(|t| t.export_as_rl()).collect();
                let jsonl: Vec<String> = entries
                    .iter()
                    .map(|e| serde_json::to_string(e).unwrap_or_default())
                    .collect();
                Ok(jsonl.join("\n"))
            }
            ExportFormat::Compressed => {
                let compressed: Vec<serde_json::Value> = filtered
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "id": t.id,
                            "topic": t.topic,
                            "outcome": t.outcome,
                            "quality": t.quality,
                            "value_score": t.value_score,
                            "step_summaries": t.steps.iter().map(|s| s.content.chars().take(100).collect::<String>()).collect::<Vec<_>>(),
                            "tool_sequence": t.steps.iter().filter_map(|s| s.tool_calls.as_ref().map(|c| c.first().map(|tc| tc.name.clone()).unwrap_or_default())).collect::<Vec<_>>(),
                            "reasoning_summary": if t.steps.iter().any(|s| s.reasoning.is_some()) { "used" } else { "none" }
                        })
                    })
                    .collect();
                Ok(serde_json::to_string_pretty(&compressed)?)
            }
        }
    }

    fn filter_trajectories(
        &self,
        trajectories: &[Trajectory],
        options: &TrajectoryExportOptions,
    ) -> Vec<Trajectory> {
        let mut filtered = trajectories.to_vec();

        if let Some(min_quality) = options.min_quality {
            filtered.retain(|t| t.quality.overall >= min_quality);
        }

        if let Some(min_value) = options.min_value_score {
            filtered.retain(|t| t.value_score >= min_value);
        }

        if let Some(outcome) = &options.outcome_filter {
            filtered.retain(|t| &t.outcome == outcome);
        }

        if let Some(limit) = options.limit {
            filtered.truncate(limit);
        }

        filtered
    }

    pub fn analyze_batch(&self, trajectories: &[Trajectory]) -> BatchAnalysis {
        let total = trajectories.len();

        let outcome_counts: HashMap<TrajectoryOutcome, usize> = trajectories
            .iter()
            .into_group_map_by(|t| t.outcome)
            .into_iter()
            .map(|(outcome, group)| (outcome, group.len()))
            .collect();

        let quality_scores: Vec<f64> = trajectories.iter().map(|t| t.quality.overall).collect();
        let value_scores: Vec<f64> = trajectories.iter().map(|t| t.value_score).collect();

        let avg_quality = if quality_scores.is_empty() {
            0.0
        } else {
            quality_scores.iter().sum::<f64>() / total as f64
        };

        let avg_value = if value_scores.is_empty() {
            0.0
        } else {
            value_scores.iter().sum::<f64>() / total as f64
        };

        let mut pattern_counts: HashMap<String, usize> = HashMap::new();
        for t in trajectories {
            for p in &t.patterns {
                *pattern_counts.entry(p.clone()).or_insert(0) += 1;
            }
        }

        let mut pattern_vec: Vec<_> = pattern_counts.into_iter().collect();
        pattern_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let top_patterns: Vec<_> = pattern_vec
            .into_iter()
            .take(10)
            .map(|(name, count)| PatternStat { name, count })
            .collect();

        BatchAnalysis {
            total,
            outcome_counts,
            avg_quality,
            avg_value,
            top_patterns,
            quality_distribution: self.compute_quality_distribution(&quality_scores),
        }
    }

    fn compute_quality_distribution(&self, scores: &[f64]) -> QualityDistribution {
        let mut excellent = 0;
        let mut good = 0;
        let mut fair = 0;
        let mut poor = 0;

        for &score in scores {
            match score {
                s if s >= 0.8 => excellent += 1,
                s if s >= 0.6 => good += 1,
                s if s >= 0.4 => fair += 1,
                _ => poor += 1,
            }
        }

        let total = scores.len().max(1) as f64;
        QualityDistribution {
            excellent: excellent as f64 / total,
            good: good as f64 / total,
            fair: fair as f64 / total,
            poor: poor as f64 / total,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PatternStat {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QualityDistribution {
    pub excellent: f64,
    pub good: f64,
    pub fair: f64,
    pub poor: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchAnalysis {
    pub total: usize,
    pub outcome_counts: HashMap<TrajectoryOutcome, usize>,
    pub avg_quality: f64,
    pub avg_value: f64,
    pub top_patterns: Vec<PatternStat>,
    pub quality_distribution: QualityDistribution,
}

impl TrajectoryQuery {
    pub fn execute(&self, storage: &TrajectoryStorage) -> Result<Vec<Trajectory>> {
        storage.query_trajectories(self)
    }
}
