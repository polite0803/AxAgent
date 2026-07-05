// SPDX-License-Identifier: AGPL-3.0-only

//! RL Training 管理命令
//!
//! 提供 RL 训练任务的启停、指标查询、检查点管理。
//! 使用内存状态管理，支持并发训练任务。
//! 后续可扩展为持久化 + 真实 RL 引擎接入。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::command;

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLTrainingConfig {
    pub algorithm: String,
    pub learning_rate: f64,
    pub batch_size: u64,
    pub epochs: u64,
    pub max_steps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub step: u64,
    pub loss: f64,
    pub reward: f64,
    pub policy_loss: f64,
    pub value_loss: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    pub id: String,
    pub name: String,
    pub step: u64,
    pub loss: f64,
    pub reward: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlTrainingState {
    pub training_id: String,
    pub status: String, // "running" | "paused" | "completed" | "failed"
    pub config: RLTrainingConfig,
    pub current_step: u64,
    pub checkpoints: Vec<CheckpointInfo>,
}

// ── Global State ──

static TRAINING_STATE: OnceLock<Mutex<HashMap<String, RlTrainingState>>> = OnceLock::new();

fn training_state() -> &'static Mutex<HashMap<String, RlTrainingState>> {
    TRAINING_STATE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn generate_training_id() -> String {
    format!("training_{}", chrono::Utc::now().timestamp_millis())
}

fn generate_checkpoint_id() -> String {
    format!("ckpt_{}", chrono::Utc::now().timestamp_millis())
}

fn simulate_metrics(step: u64, config: &RLTrainingConfig) -> TrainingMetrics {
    let progress = step as f64 / config.max_steps as f64;
    let base_loss = 2.5 * (-step as f64 * 0.002).exp();
    let noise = (step as f64).sin() * 0.05 + (step as f64 * 0.1).cos() * 0.03;
    TrainingMetrics {
        step,
        loss: (base_loss + noise * 0.5).max(0.01),
        reward: (0.2 + 0.8 * (1.0 - (-progress * 3.0).exp()) + noise * 0.02)
            .min(1.0)
            .max(0.0),
        policy_loss: (base_loss * 0.6 + noise * 0.3).max(0.01),
        value_loss: (base_loss * 0.4 + noise * 0.2).max(0.01),
        timestamp: chrono::Utc::now().timestamp_millis(),
    }
}

// ── Commands ──

#[command]
pub async fn start_rl_training(config: RLTrainingConfig) -> Result<String, String> {
    let training_id = generate_training_id();
    let state = RlTrainingState {
        training_id: training_id.clone(),
        status: "running".into(),
        config,
        current_step: 0,
        checkpoints: Vec::new(),
    };

    let mut store = training_state()
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    store.insert(training_id.clone(), state);
    tracing::info!(target: "rl_training", training_id = %training_id, "RL training started");
    Ok(training_id)
}

#[command]
pub async fn stop_rl_training(training_id: String) -> Result<(), String> {
    let mut store = training_state()
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    match store.get_mut(&training_id) {
        Some(state) => {
            state.status = "paused".into();
            tracing::info!(target: "rl_training", training_id = %training_id, "RL training paused");
            Ok(())
        },
        None => Err(format!("Training session '{}' not found", training_id)),
    }
}

#[command]
pub async fn get_training_metrics(step: u64) -> Result<TrainingMetrics, String> {
    // Use the latest training's config for simulation, or defaults
    let store = training_state()
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    let config = store
        .values()
        .next()
        .map(|s| s.config.clone())
        .unwrap_or(RLTrainingConfig {
            algorithm: "ppo".into(),
            learning_rate: 1e-5,
            batch_size: 64,
            epochs: 10,
            max_steps: 10000,
        });
    drop(store);

    Ok(simulate_metrics(step, &config))
}

#[command]
pub async fn save_checkpoint(
    id: String,
    name: String,
    step: u64,
    loss: f64,
    reward: f64,
    timestamp: i64,
) -> Result<(), String> {
    let mut store = training_state()
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    // Find the training with the matching step
    for state in store.values_mut() {
        if state.current_step <= step {
            state.checkpoints.push(CheckpointInfo {
                id,
                name,
                step,
                loss,
                reward,
                timestamp,
            });
            return Ok(());
        }
    }
    // If no training found, add to a default entry
    let ckpt = CheckpointInfo {
        id,
        name,
        step,
        loss,
        reward,
        timestamp,
    };
    // Store as standalone checkpoint (will be returned by list)
    let ckpt_id = ckpt.id.clone();
    // We use a second static for orphan checkpoints
    CHECKPOINTS
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?
        .push(ckpt);
    tracing::info!(target: "rl_training", checkpoint_id = %ckpt_id, "Checkpoint saved");
    Ok(())
}

static CHECKPOINTS: OnceLock<Mutex<Vec<CheckpointInfo>>> = OnceLock::new();

fn checkpoints() -> &'static Mutex<Vec<CheckpointInfo>> {
    CHECKPOINTS.get_or_init(|| Mutex::new(Vec::new()))
}

#[command]
pub async fn load_checkpoint(checkpoint_id: String) -> Result<(), String> {
    let store = training_state()
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    let found = store
        .values()
        .any(|s| s.checkpoints.iter().any(|c| c.id == checkpoint_id));
    if found {
        tracing::info!(target: "rl_training", checkpoint_id = %checkpoint_id, "Checkpoint loaded");
        Ok(())
    } else {
        // Check orphan checkpoints
        let orphan = checkpoints()
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        if orphan.iter().any(|c| c.id == checkpoint_id) {
            return Ok(());
        }
        Err(format!("Checkpoint '{}' not found", checkpoint_id))
    }
}

#[command]
pub async fn list_checkpoints() -> Result<Vec<CheckpointInfo>, String> {
    let mut all = Vec::new();
    let store = training_state()
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    for state in store.values() {
        all.extend(state.checkpoints.clone());
    }
    drop(store);
    let orphan = checkpoints()
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    all.extend(orphan.clone());
    Ok(all)
}
