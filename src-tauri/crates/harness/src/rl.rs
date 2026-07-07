// SPDX-License-Identifier: AGPL-3.0-only

//! RL 契约 — RLEngine + RLTrainer traits + DTOs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLConfig {
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub exploration_rate: f64,
    pub batch_size: usize,
}
impl Default for RLConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            discount_factor: 0.99,
            exploration_rate: 0.1,
            batch_size: 32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardWeights {
    pub task_completion: f64,
    pub efficiency: f64,
    pub code_quality: f64,
    pub user_satisfaction: f64,
}
impl Default for RewardWeights {
    fn default() -> Self {
        Self {
            task_completion: 1.0,
            efficiency: 0.5,
            code_quality: 0.3,
            user_satisfaction: 0.8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEpisode {
    pub episode_id: String,
    pub steps: Vec<TrainingStep>,
    pub total_reward: f64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStep {
    pub observation: String,
    pub action: String,
    pub reward: f64,
    pub next_observation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReport {
    pub episodes_trained: usize,
    pub avg_reward: f64,
    pub max_reward: f64,
    pub total_steps: usize,
    pub duration_secs: f64,
}

#[async_trait]
pub trait RLEngine: Send + Sync {
    async fn compute_rewards(&self, episodes: &[TrainingEpisode]) -> Result<Vec<f64>, String>;
    async fn compute_advantages(&self, rewards: &[f64]) -> Vec<f64>;
    async fn reset(&self);
}

#[async_trait]
pub trait RLTrainer: Send + Sync {
    async fn train_episode(&self, episode: TrainingEpisode) -> Result<TrainingReport, String>;
    async fn get_progress(&self) -> Result<TrainingReport, String>;
}

#[derive(Default)]
pub struct NoopRLEngine;
#[async_trait]
impl RLEngine for NoopRLEngine {
    async fn compute_rewards(&self, _episodes: &[TrainingEpisode]) -> Result<Vec<f64>, String> {
        Ok(vec![0.0])
    }
    async fn compute_advantages(&self, _rewards: &[f64]) -> Vec<f64> {
        vec![0.0]
    }
    async fn reset(&self) {}
}

#[derive(Default)]
pub struct NoopRLTrainer;
#[async_trait]
impl RLTrainer for NoopRLTrainer {
    async fn train_episode(&self, _episode: TrainingEpisode) -> Result<TrainingReport, String> {
        Err("RL trainer not configured".to_string())
    }
    async fn get_progress(&self) -> Result<TrainingReport, String> {
        Ok(TrainingReport {
            episodes_trained: 0,
            avg_reward: 0.0,
            max_reward: 0.0,
            total_steps: 0,
            duration_secs: 0.0,
        })
    }
}
