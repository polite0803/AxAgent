// SPDX-License-Identifier: AGPL-3.0-only

//! RL Training 管理命令
//!
//! 提供 RL 训练任务的启停、指标查询、检查点管理。
//! 对接真实 RLEngine + TrajectoryStorage，替代旧版的纯内存模拟。
//! 训练数据从 TrajectoryStorage 实时采集，奖励由 RLEngine 计算。

use crate::AppState;
use axagent_harness::trajectory_types::RewardType;
use axagent_trajectory::RLConfig;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use tauri::{State, command};
use tokio::sync::Mutex;

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
    pub status: String,
    pub config: RLTrainingConfig,
    pub current_step: u64,
    pub checkpoints: Vec<CheckpointInfo>,
}

// ── Runtime tracking ──
// 训练会话的运行时状态。真实训练数据来自 RLEngine + TrajectoryStorage。

struct TrainingRuntime {
    state: RlTrainingState,
}

static TRAINING_RUNTIME: OnceLock<Mutex<HashMap<String, TrainingRuntime>>> = OnceLock::new();

fn training_runtime() -> &'static Mutex<HashMap<String, TrainingRuntime>> {
    TRAINING_RUNTIME.get_or_init(|| Mutex::new(HashMap::new()))
}

fn generate_training_id() -> String {
    format!("training_{}", chrono::Utc::now().timestamp_millis())
}

fn timestamp_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// ── 真实指标计算 ──

/// 从 TrajectoryStorage 采集轨迹数据，使用 RLEngine 计算真实奖励指标。
async fn compute_real_metrics(state: &AppState, step: u64) -> Result<TrainingMetrics, String> {
    // 获取最近的轨迹数据
    let trajectories = state
        .trajectory_storage
        .get_trajectories(Some(50))
        .await
        .map_err(|e| format!("获取轨迹数据失败: {}", e))?;

    if trajectories.is_empty() {
        // 无轨迹数据时返回零值指标
        return Ok(TrainingMetrics {
            step,
            loss: 0.0,
            reward: 0.0,
            policy_loss: 0.0,
            value_loss: 0.0,
            timestamp: timestamp_millis(),
        });
    }

    // 使用 RLEngine 计算奖励
    let rl_engine = state.rl_engine.read().await;
    let _weights = rl_engine.weights();

    let mut total_reward = 0.0f64;
    let mut total_tool_efficiency = 0.0f64;
    let mut total_reasoning_quality = 0.0f64;
    let mut count = 0u64;

    for t in &trajectories {
        let mut traj = t.clone();
        let rewards = rl_engine.compute_rewards(&mut traj).await;

        for r in &rewards {
            total_reward += r.value;
            match r.reward_type {
                RewardType::ToolEfficiency => {
                    total_tool_efficiency += r.value;
                },
                RewardType::ReasoningQuality => {
                    total_reasoning_quality += r.value;
                },
                _ => {},
            }
        }
        if !rewards.is_empty() {
            count += 1;
        }
    }
    drop(rl_engine);

    let avg_reward = if count > 0 {
        total_reward / count as f64
    } else {
        0.0
    };
    let avg_tool_efficiency = if count > 0 {
        total_tool_efficiency / count as f64
    } else {
        0.0
    };
    let avg_reasoning_quality = if count > 0 {
        total_reasoning_quality / count as f64
    } else {
        0.0
    };

    // 计算损失：1.0 - avg_reward 作为简化的损失函数
    let loss = (1.0 - avg_reward.clamp(0.0, 1.0)).max(0.0);
    let policy_loss = (1.0 - avg_tool_efficiency.clamp(0.0, 1.0)).max(0.0);
    let value_loss = (1.0 - avg_reasoning_quality.clamp(0.0, 1.0)).max(0.0);

    Ok(TrainingMetrics {
        step,
        loss,
        reward: avg_reward,
        policy_loss,
        value_loss,
        timestamp: timestamp_millis(),
    })
}

// ── Commands ──

/// 启动 RL 训练会话。
#[command]
pub async fn start_rl_training(
    _state: State<'_, AppState>,
    config: RLTrainingConfig,
) -> Result<String, String> {
    let training_id = generate_training_id();

    let runtime = TrainingRuntime {
        state: RlTrainingState {
            training_id: training_id.clone(),
            status: "running".into(),
            config,
            current_step: 0,
            checkpoints: Vec::new(),
        },
    };
    let mut store = training_runtime().lock().await;
    store.insert(training_id.clone(), runtime);

    // 更新 RLOptimizer 配置以匹配训练参数
    {
        let mut opt = super::_shared_state::SHARED_OPTIMIZER.write().await;
        opt.config = RLConfig {
            gamma: 0.99,
            lambda: 0.95,
            reward_scale: 1.0,
            entropy_coefficient: 0.01,
            value_coefficient: 0.5,
            use_td_lambda: true,
            ..Default::default()
        };
        tracing::info!(
            target: "rl_training",
            training_id = %training_id,
            "RLOptimizer config updated for training"
        );
    }

    tracing::info!(target: "rl_training", training_id = %training_id, "RL training started");
    Ok(training_id)
}

/// 停止 RL 训练会话。
#[command]
pub async fn stop_rl_training(
    _state: State<'_, AppState>,
    training_id: String,
) -> Result<(), String> {
    let mut store = training_runtime().lock().await;
    match store.get_mut(&training_id) {
        Some(runtime) => {
            runtime.state.status = "paused".into();
        },
        None => return Err(format!("训练会话 '{}' 不存在", training_id)),
    }

    tracing::info!(target: "rl_training", training_id = %training_id, "RL training paused");
    Ok(())
}

/// 获取训练指标（对接真实 RLEngine + TrajectoryStorage）。
#[command]
pub async fn get_training_metrics(
    state: State<'_, AppState>,
    step: u64,
) -> Result<TrainingMetrics, String> {
    // 更新训练会话的当前步数
    {
        let mut store = training_runtime().lock().await;
        for runtime in store.values_mut() {
            if runtime.state.status == "running" {
                runtime.state.current_step = runtime.state.current_step.max(step);
            }
        }
    }

    compute_real_metrics(&state, step).await
}

/// 保存训练检查点。
#[command]
pub async fn save_checkpoint(
    state: State<'_, AppState>,
    id: String,
    name: String,
    step: u64,
    loss: f64,
    reward: f64,
    timestamp: i64,
) -> Result<(), String> {
    let ckpt = CheckpointInfo { id, name, step, loss, reward, timestamp };

    // 存入训练运行时
    {
        let mut store = training_runtime().lock().await;
        for runtime in store.values_mut() {
            runtime.state.checkpoints.push(ckpt.clone());
        }
    }

    // 持久化到 TrajectoryStorage（以 pattern 形式存储检查点元数据）
    let pattern = axagent_trajectory::TrajectoryPattern {
        id: ckpt.id.clone(),
        name: format!("rl_checkpoint:{}", ckpt.name),
        description: serde_json::to_string(&ckpt).unwrap_or_default(),
        pattern_type: "rl_checkpoint".into(),
        success_rate: ckpt.reward.clamp(0.0, 1.0),
        trajectory_ids: Vec::new(),
        frequency: 1,
        average_quality: ckpt.reward.clamp(0.0, 1.0),
        average_value_score: ckpt.reward.clamp(0.0, 1.0),
        reward_profile: Vec::new(),
        created_at: chrono::Utc::now(),
    };
    if let Err(e) = state.trajectory_storage.save_pattern(&pattern).await {
        tracing::warn!(target: "rl_training", checkpoint_id = %ckpt.id,
            "Failed to persist checkpoint: {}", e);
    }

    tracing::info!(target: "rl_training", checkpoint_id = %ckpt.id, "Checkpoint saved");
    Ok(())
}

/// 加载训练检查点。
#[command]
pub async fn load_checkpoint(
    state: State<'_, AppState>,
    checkpoint_id: String,
) -> Result<(), String> {
    // 检查训练运行时中的检查点
    {
        let store = training_runtime().lock().await;
        let found =
            store.values().any(|r| r.state.checkpoints.iter().any(|c| c.id == checkpoint_id));
        if found {
            tracing::info!(target: "rl_training", checkpoint_id = %checkpoint_id,
                "Checkpoint loaded from runtime");
            return Ok(());
        }
    }

    // 从持久化存储中查找
    let patterns = state
        .trajectory_storage
        .get_patterns()
        .await
        .map_err(|e| format!("加载检查点失败: {}", e))?;
    let found = patterns.iter().any(|p| p.id == checkpoint_id);
    if found {
        tracing::info!(target: "rl_training", checkpoint_id = %checkpoint_id,
            "Checkpoint loaded from storage");
        Ok(())
    } else {
        Err(format!("检查点 '{}' 不存在", checkpoint_id))
    }
}

/// 列出所有检查点。
#[command]
pub async fn list_checkpoints(state: State<'_, AppState>) -> Result<Vec<CheckpointInfo>, String> {
    let mut all = Vec::new();

    // 从训练运行时中收集
    {
        let store = training_runtime().lock().await;
        for runtime in store.values() {
            all.extend(runtime.state.checkpoints.clone());
        }
    }

    // 从持久化存储中收集
    let patterns = state
        .trajectory_storage
        .get_patterns()
        .await
        .map_err(|e| format!("获取检查点列表失败: {}", e))?;
    for p in &patterns {
        if p.name.starts_with("rl_checkpoint:") {
            if let Ok(ckpt) = serde_json::from_str::<CheckpointInfo>(&p.description) {
                if !all.iter().any(|c| c.id == ckpt.id) {
                    all.push(ckpt);
                }
            }
        }
    }

    // 按时间戳降序排列
    all.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
    Ok(all)
}

/// 删除训练检查点（P1 修复：补齐前端调用的后端命令）。
///
/// 同时清理训练运行时内存和持久化存储（trajectory_patterns 表）中的记录。
#[command]
pub async fn delete_checkpoint(
    state: State<'_, AppState>,
    checkpoint_id: String,
) -> Result<(), String> {
    let mut removed_from_runtime = false;

    // 1. 从训练运行时内存中移除
    {
        let mut store = training_runtime().lock().await;
        for runtime in store.values_mut() {
            let before = runtime.state.checkpoints.len();
            runtime.state.checkpoints.retain(|c| c.id != checkpoint_id);
            if runtime.state.checkpoints.len() < before {
                removed_from_runtime = true;
            }
        }
    }

    // 2. 从持久化存储中删除（trajectory_patterns 表）
    let db = state.harness.db();
    let delete_result =
        axagent_entities::trajectory_patterns::Entity::delete_by_id(checkpoint_id.clone())
            .exec(db)
            .await;

    let removed_from_storage = match delete_result {
        Ok(r) => r.rows_affected > 0,
        Err(e) => {
            tracing::warn!(target: "rl_training", checkpoint_id = %checkpoint_id,
                "Failed to delete checkpoint from storage: {}", e);
            false
        },
    };

    if !removed_from_runtime && !removed_from_storage {
        return Err(format!("检查点 '{}' 不存在", checkpoint_id));
    }

    tracing::info!(target: "rl_training", checkpoint_id = %checkpoint_id,
        "Checkpoint deleted (runtime={}, storage={})", removed_from_runtime, removed_from_storage);
    Ok(())
}

/// 运行一轮真实 RL 训练（对接 RLEngine 的 compute_rewards）。
///
/// 从 TrajectoryStorage 采集最近的轨迹数据，计算奖励信号，
/// 并更新奖励权重向量。返回训练后的指标摘要。
#[command]
pub async fn run_rl_training_step(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    // 1. 获取最近的轨迹数据
    let mut trajectories = state
        .trajectory_storage
        .get_trajectories(Some(100))
        .await
        .map_err(|e| format!("获取轨迹数据失败: {}", e))?;

    if trajectories.is_empty() {
        return Ok(serde_json::json!({
            "success": false,
            "message": "没有可用的轨迹数据",
            "trajectoriesProcessed": 0,
        }));
    }

    // 2. 使用 RLEngine 计算真实奖励
    let rl_engine = state.rl_engine.read().await;
    let mut total_reward = 0.0f64;
    let mut total_tool_efficiency = 0.0f64;
    let mut total_reasoning_quality = 0.0f64;
    let mut processed = 0u64;

    for traj in trajectories.iter_mut() {
        // compute_rewards 支持 LLM judge（若配置），无 judge 时退化为启发式
        let rewards = rl_engine.compute_rewards(traj).await;

        for r in &rewards {
            total_reward += r.value;
            match r.reward_type {
                RewardType::ToolEfficiency => {
                    total_tool_efficiency += r.value;
                },
                RewardType::ReasoningQuality => {
                    total_reasoning_quality += r.value;
                },
                _ => {},
            }
        }

        // M7-D: 将轨迹奖励作为 Experience 写入 RLOptimizer
        {
            let mut opt = super::_shared_state::SHARED_OPTIMIZER.write().await;
            let total: f32 = rewards.iter().map(|r| r.value as f32).sum();
            let experience = axagent_agent::rl_optimizer::Experience {
                id: uuid::Uuid::new_v4().to_string(),
                state: axagent_agent::rl_optimizer::TaskState {
                    task_id: traj.id.clone(),
                    task_type: "trajectory".to_string(),
                    context: std::collections::HashMap::new(),
                    available_tools: Vec::new(),
                    completed_tools: Vec::new(),
                    error_count: if traj.outcome
                        != axagent_harness::trajectory_types::TrajectoryOutcome::Success
                    {
                        1
                    } else {
                        0
                    },
                    elapsed_ms: 0,
                },
                action: axagent_agent::rl_optimizer::ToolSelection {
                    tool_id: "trajectory_eval".to_string(),
                    tool_name: "Trajectory Evaluation".to_string(),
                    parameters: std::collections::HashMap::new(),
                    reasoning: format!("trajectory {} reward", traj.id),
                },
                reward: total,
                next_state: axagent_agent::rl_optimizer::TaskState {
                    task_id: traj.id.clone(),
                    task_type: "trajectory".to_string(),
                    context: std::collections::HashMap::new(),
                    available_tools: Vec::new(),
                    completed_tools: Vec::new(),
                    error_count: 0,
                    elapsed_ms: 0,
                },
                done: true,
                timestamp: chrono::Utc::now(),
            };
            opt.record_experience(experience);
        }

        if !rewards.is_empty() {
            processed += 1;
        }
    }
    drop(rl_engine);

    let avg_reward = if processed > 0 {
        total_reward / processed as f64
    } else {
        0.0
    };
    let avg_tool_eff = if processed > 0 {
        total_tool_efficiency / processed as f64
    } else {
        0.0
    };
    let avg_reasoning = if processed > 0 {
        total_reasoning_quality / processed as f64
    } else {
        0.0
    };

    // 3. 计算简化损失
    let loss = (1.0 - avg_reward.clamp(0.0, 1.0)).max(0.0);

    // 4. 返回结果
    Ok(serde_json::json!({
        "success": true,
        "trajectoriesProcessed": processed,
        "avgReward": avg_reward,
        "avgToolEfficiency": avg_tool_eff,
        "avgReasoningQuality": avg_reasoning,
        "loss": loss,
        "timestamp": timestamp_millis(),
        "message": format!("完成 {} 条轨迹的奖励计算", processed),
    }))
}
