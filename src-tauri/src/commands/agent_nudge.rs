use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn nudge_list(
    app_state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let ns = app_state.nudge_service.lock().await;
    let pending = ns.get_pending_nudges(&session_id);
    Ok(pending.iter()
        .filter_map(|n| serde_json::to_value(n).ok())
        .collect())
}

#[tauri::command]
pub async fn nudge_dismiss(
    app_state: State<'_, AppState>,
    nudge_id: String,
) -> Result<bool, String> {
    let mut ns = app_state.nudge_service.lock().await;
    Ok(ns.take_nudge_action(&nudge_id, axagent_trajectory::NudgeAction::Dismissed))
}

#[tauri::command]
pub async fn nudge_snooze(
    app_state: State<'_, AppState>,
    nudge_id: String,
    until: i64,
) -> Result<bool, String> {
    let mut ns = app_state.nudge_service.lock().await;
    Ok(ns.snooze_nudge(&nudge_id, until))
}

#[tauri::command]
pub async fn nudge_execute(
    app_state: State<'_, AppState>,
    nudge_id: String,
) -> Result<bool, String> {
    let mut ns = app_state.nudge_service.lock().await;
    Ok(ns.take_nudge_action(&nudge_id, axagent_trajectory::NudgeAction::AddedToMemory))
}

#[tauri::command]
pub async fn nudge_stats(
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ns = app_state.nudge_service.lock().await;
    let stats = ns.get_nudge_stats();
    Ok(serde_json::to_value(stats).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn nudge_closed_loop_list(
    app_state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let nudges = app_state.closed_loop_service.get_nudges();
    Ok(nudges.iter()
        .filter_map(|n| serde_json::to_value(n).ok())
        .collect())
}

#[tauri::command]
pub async fn nudge_closed_loop_acknowledge(
    app_state: State<'_, AppState>,
    nudge_id: String,
) -> Result<(), String> {
    app_state.closed_loop_service.acknowledge_nudge(&nudge_id);
    Ok(())
}
