use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn trajectory_stats(
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let stats = app_state.trajectory_storage.get_statistics()
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(stats).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn trajectory_list(
    app_state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<serde_json::Value>, String> {
    let trajectories = app_state.trajectory_storage.get_trajectories(limit.or(Some(20)))
        .map_err(|e| e.to_string())?;
    Ok(trajectories.iter()
        .filter_map(|t| serde_json::to_value(t).ok())
        .collect())
}

#[tauri::command]
pub async fn pattern_stats(
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pl = app_state.pattern_learner.read().unwrap();
    let stats = pl.get_statistics();
    Ok(serde_json::to_value(stats).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn closed_loop_status(
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let is_running = app_state.closed_loop_service.is_running();
    let nudge_count = app_state.closed_loop_service.get_nudges().len();
    let pattern_count = app_state.pattern_learner.read().unwrap().get_statistics().total_patterns;
    let insight_count = app_state.insight_system.read().unwrap().get_insights().len();
    Ok(serde_json::json!({
        "closed_loop_running": is_running,
        "nudge_count": nudge_count,
        "pattern_count": pattern_count,
        "insight_count": insight_count,
    }))
}

#[tauri::command]
pub async fn rl_config(
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let rl = app_state.rl_engine.read().unwrap();
    Ok(serde_json::json!({
        "config": rl.config(),
        "weights": rl.weights(),
    }))
}

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
    let entries = app_state.trajectory_storage.export_trajectories(&options)
        .map_err(|e| e.to_string())?;
    Ok(entries.iter().filter_map(|e| serde_json::to_value(e).ok()).collect())
}

#[tauri::command]
pub async fn rl_compute_rewards(
    app_state: State<'_, AppState>,
    trajectory_id: String,
) -> Result<serde_json::Value, String> {
    let storage = &app_state.trajectory_storage;
    let mut trajectory = storage.get_trajectory(&trajectory_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trajectory {} not found", trajectory_id))?;

    let rl = app_state.rl_engine.read().unwrap();
    let rewards = rl.compute_rewards(&mut trajectory);
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
