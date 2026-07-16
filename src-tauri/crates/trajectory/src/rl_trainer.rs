// SPDX-License-Identifier: AGPL-3.0-only

use crate::training_env::{EvaluationResult, RewardComputation, TaskDefinition, TrainingEnv};
use crate::trajectory::{TrainingConfig, Trajectory};
use crate::trajectory_compressor::{CompressedTrajectory, TrajectoryCompressor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 注意：本模块的 TrainingEpisode / TrainingReport / RLTrainer 与
// axagent_harness::rl 中的同名类型语义不同（harness 的是通用 trait 契约，
// 本模块的是轨迹训练具体实现，依赖 training_env）。为避免命名冲突，
// 统一加 Trajectory 前缀（AGENTS.md 第 12 条：禁止重复定义）。

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryTrainingEpisode {
    pub episode_id: String,
    pub task: TaskDefinition,
    pub trajectory: Option<CompressedTrajectory>,
    pub reward: Option<RewardComputation>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryTrainingReport {
    pub total_episodes: u32,
    pub passed: u32,
    pub failed: u32,
    pub avg_reward: f64,
    pub episodes: Vec<TrajectoryTrainingEpisode>,
}

pub struct TrajectoryRLTrainer {
    env: TrainingEnv,
    compressor: TrajectoryCompressor,
    episodes: Vec<TrajectoryTrainingEpisode>,
}

impl TrajectoryRLTrainer {
    pub fn new(_config: TrainingConfig, tasks: Vec<TaskDefinition>) -> Self {
        let env = TrainingEnv::new(tasks);
        let compressor = TrajectoryCompressor::new(500);
        Self { env, compressor, episodes: Vec::new() }
    }

    pub fn record_trajectory(&mut self, trajectory: &Trajectory) -> EvaluationResult {
        let result = self.env.evaluate(trajectory);
        let compressed = self.compressor.compress(trajectory);
        let episode = TrajectoryTrainingEpisode {
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
        let compressed: Vec<&CompressedTrajectory> =
            self.episodes.iter().filter_map(|e| e.trajectory.as_ref()).collect();
        let lines: Result<Vec<String>, _> =
            compressed.iter().map(|t| serde_json::to_string(*t)).collect();
        Ok(lines?.join("\n"))
    }

    pub fn report(&self) -> TrajectoryTrainingReport {
        let passed = self.episodes.iter().filter(|e| e.passed).count() as u32;
        let total = self.episodes.len() as u32;
        let avg_reward = if total > 0 {
            self.episodes.iter().filter_map(|e| e.reward.as_ref().map(|r| r.total)).sum::<f64>()
                / total as f64
        } else {
            0.0
        };
        TrajectoryTrainingReport {
            total_episodes: total,
            passed,
            failed: total - passed,
            avg_reward,
            episodes: self.episodes.clone(),
        }
    }
}
