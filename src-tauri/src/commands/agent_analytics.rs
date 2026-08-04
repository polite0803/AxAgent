// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::{CommandError, ErrorCategory};
use agent_macro::agent_command;
use axagent_trajectory::TrajectoryQuery;
use tauri::State;

#[agent_command(domain = agent, safety = Safe, call_mode = StateOnly, description = "轨迹统计")]
#[tauri::command]
pub async fn trajectory_stats(app_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let stats = app_state
        .trajectory_storage
        .get_statistics()
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?;
    Ok(serde_json::to_value(stats)
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?)
}

#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "轨迹列表")]
#[tauri::command]
pub async fn trajectory_list(
    app_state: State<'_, AppState>,
    session_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<serde_json::Value>, String> {
    let query = TrajectoryQuery { session_id, limit: limit.or(Some(20)), ..Default::default() };
    let trajectories = app_state
        .trajectory_storage
        .query_trajectories(&query)
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?;
    Ok(trajectories.iter().filter_map(|t| serde_json::to_value(t).ok()).collect())
}

#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "获取轨迹详情")]
#[tauri::command]
pub async fn get_trajectory_detail(
    app_state: State<'_, AppState>,
    trajectory_id: String,
) -> Result<serde_json::Value, String> {
    let trajectory = app_state
        .trajectory_storage
        .get_trajectory(&trajectory_id)
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?
        .ok_or_else(|| {
            CommandError::new("TRAJECTORY_NOT_FOUND")
                .with_category(ErrorCategory::Validation)
                .with_detail(format!("Trajectory {} not found", trajectory_id))
        })?;
    Ok(serde_json::to_value(trajectory)
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?)
}

#[agent_command(domain = agent, safety = Safe, call_mode = StateOnly, description = "模式统计")]
#[tauri::command]
pub async fn pattern_stats(app_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let pl = app_state.pattern_learner.read().await;
    let stats = pl.get_statistics();
    Ok(serde_json::to_value(stats)
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?)
}

#[agent_command(domain = agent, safety = Safe, call_mode = StateOnly, description = "闭环状态")]
#[tauri::command]
pub async fn closed_loop_status(
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let is_running = app_state.closed_loop_service.is_running();
    let nudge_count = app_state.closed_loop_service.get_nudges().len();
    let pattern_count = app_state.pattern_learner.read().await.get_statistics().total_patterns;
    let insight_count = app_state.insight_system.read().await.get_insights().len();
    Ok(serde_json::json!({
        "closed_loop_running": is_running,
        "nudge_count": nudge_count,
        "pattern_count": pattern_count,
        "insight_count": insight_count,
    }))
}

#[agent_command(domain = agent, safety = Safe, call_mode = StateOnly, description = "强化学习配置")]
#[tauri::command]
pub async fn rl_config(app_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let rl = app_state.rl_engine.read().await;
    Ok(serde_json::json!({
        "config": rl.config(),
        "weights": rl.weights(),
    }))
}

#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "导出训练数据")]
#[tauri::command]
pub async fn rl_export_training_data(
    app_state: State<'_, AppState>,
    min_quality: Option<f64>,
    limit: Option<usize>,
) -> Result<Vec<serde_json::Value>, String> {
    let options = axagent_trajectory::TrajectoryExportOptions {
        format: axagent_trajectory::ExportFormat::RlTraining,
        min_quality: Some(min_quality.unwrap_or(0.3)),
        min_value_score: None,
        outcome_filter: None,
        limit: limit.or(Some(50)),
    };
    let entries = app_state
        .trajectory_storage
        .export_trajectories(&options)
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?;
    Ok(entries.iter().filter_map(|e| serde_json::to_value(e).ok()).collect())
}

#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "计算奖励")]
#[tauri::command]
pub async fn rl_compute_rewards(
    app_state: State<'_, AppState>,
    trajectory_id: String,
) -> Result<serde_json::Value, String> {
    let storage = &app_state.trajectory_storage;
    let mut trajectory = storage
        .get_trajectory(&trajectory_id)
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?
        .ok_or_else(|| {
            CommandError::new("TRAJECTORY_NOT_FOUND")
                .with_category(ErrorCategory::Validation)
                .with_detail(format!("Trajectory {} not found", trajectory_id))
        })?;

    let rl = app_state.rl_engine.read().await;
    let rewards = rl.compute_rewards(&mut trajectory).await;
    let values = rl.estimate_value_function(&trajectory);
    let advantages = if !values.is_empty() {
        rl.compute_advantages(&rewards, &values)
    } else {
        vec![]
    };

    let total_reward: f64 = rewards.iter().map(|r| r.value).sum();

    Ok(serde_json::json!({
        "trajectory_id": trajectory_id,
        "reward_count": rewards.len(),
        "total_reward": total_reward,
        "value_count": values.len(),
        "advantage_count": advantages.len(),
    }))
}

// ---------------------------------------------------------------------------
// 反馈命令（从 agent 模块迁移）
// ---------------------------------------------------------------------------

/// 记录反馈信号用于 RealTimeLearning
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "记录反馈")]
#[tauri::command]
pub async fn record_feedback(
    app_state: State<'_, AppState>,
    feedback_type: String,
    source: String,
    content: String,
) -> Result<(), String> {
    use crate::commands::error::ErrorResponse;
    use crate::commands::error_code::agent as agent_err;

    let ft = match feedback_type.as_str() {
        "success" => axagent_trajectory::FeedbackType::Success,
        "failure" => axagent_trajectory::FeedbackType::Failure,
        "partial" => axagent_trajectory::FeedbackType::Partial,
        "correction" => axagent_trajectory::FeedbackType::Correction,
        _ => {
            return Err(ErrorResponse::new(agent_err::INTERNAL)
                .with_detail(format!("Unknown feedback type: {}", feedback_type))
                .to_string());
        },
    };
    let fs = match source.as_str() {
        "user" => axagent_trajectory::FeedbackSource::User,
        "system" => axagent_trajectory::FeedbackSource::System,
        "self" => axagent_trajectory::FeedbackSource::Self_,
        _ => {
            return Err(ErrorResponse::new(agent_err::INTERNAL)
                .with_detail(format!("Unknown feedback source: {}", source))
                .to_string());
        },
    };

    let mut rl = app_state.realtime_learning.lock().await;
    rl.record_feedback(axagent_trajectory::FeedbackSignal {
        feedback_type: ft,
        source: fs,
        content,
        timestamp: chrono::Utc::now().timestamp_millis(),
        context: None,
    });
    Ok(())
}
