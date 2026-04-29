use crate::training_env::{EvaluationResult, RewardComputation, TaskDefinition, TrainingEnv};
use crate::trajectory::{Trajectory, TrainingConfig};
use crate::trajectory_compressor::{CompressedTrajectory, TrajectoryCompressor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEpisode {
    pub episode_id: String,
    pub task: TaskDefinition,
    pub trajectory: Option<CompressedTrajectory>,
    pub reward: Option<RewardComputation>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReport {
    pub total_episodes: u32,
    pub passed: u32,
    pub failed: u32,
    pub avg_reward: f64,
    pub episodes: Vec<TrainingEpisode>,
}

pub struct RLTrainer {
    env: TrainingEnv,
    compressor: TrajectoryCompressor,
    episodes: Vec<TrainingEpisode>,
}

impl RLTrainer {
    pub fn new(_config: TrainingConfig, tasks: Vec<TaskDefinition>) -> Self {
        let env = TrainingEnv::new(tasks);
        let compressor = TrajectoryCompressor::new(500);
        Self {
            env,
            compressor,
            episodes: Vec::new(),
        }
    }

    pub fn record_trajectory(&mut self, trajectory: &Trajectory) -> EvaluationResult {
        let result = self.env.evaluate(trajectory);
        let compressed = self.compressor.compress(trajectory);
        let episode = TrainingEpisode {
            episode_id: uuid::Uuid::new_v4().to_string(),
            task: TaskDefinition {
                id: trajectory.topic.clone(),
                prompt: String::new(),
                expected_outcome: None,
                difficulty: 0.5,
                category: "general".to_string(),
                metadata: HashMap::new(),
            },
            trajectory: Some(compressed),
            reward: Some(result.reward.clone()),
            passed: result.passed,
        };
        self.episodes.push(episode);
        result
    }

    pub fn export_jsonl(&self) -> Result<String, serde_json::Error> {
        let compressed: Vec<&CompressedTrajectory> = self
            .episodes
            .iter()
            .filter_map(|e| e.trajectory.as_ref())
            .collect();
        let lines: Result<Vec<String>, _> = compressed
            .iter()
            .map(|t| serde_json::to_string(*t))
            .collect();
        Ok(lines?.join("\n"))
    }

    pub fn report(&self) -> TrainingReport {
        let passed = self.episodes.iter().filter(|e| e.passed).count() as u32;
        let total = self.episodes.len() as u32;
        let avg_reward = if total > 0 {
            self.episodes
                .iter()
                .filter_map(|e| e.reward.as_ref().map(|r| r.total))
                .sum::<f64>()
                / total as f64
        } else {
            0.0
        };
        TrainingReport {
            total_episodes: total,
            passed,
            failed: total - passed,
            avg_reward,
            episodes: self.episodes.clone(),
        }
    }
}
