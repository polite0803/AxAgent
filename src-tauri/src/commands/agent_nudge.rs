use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn nudge_list(
    app_state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let ns = app_state.nudge_service.lock().await;
    let pending = ns.get_pending_nudges(&session_id);
    Ok(pending
        .iter()
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
pub async fn nudge_stats(app_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let ns = app_state.nudge_service.lock().await;
    let stats = ns.get_nudge_stats();
    serde_json::to_value(stats).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn nudge_closed_loop_list(
    app_state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let nudges = app_state.closed_loop_service.get_nudges();
    Ok(nudges
        .iter()
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

#[tauri::command]
pub async fn skill_find_similar(
    app_state: State<'_, AppState>,
    topic: String,
) -> Result<Vec<serde_json::Value>, String> {
    let closed_loop = app_state.closed_loop_service.clone();
    let similar = closed_loop
        .find_similar_skills(&topic)
        .map_err(|e| e.to_string())?;
    Ok(similar
        .iter()
        .filter_map(|s| serde_json::to_value(s).ok())
        .collect())
}

#[tauri::command]
pub async fn skill_upgrade_propose(
    app_state: State<'_, AppState>,
    skill_id: String,
    _task_description: String,
) -> Result<Option<serde_json::Value>, String> {
    let closed_loop = app_state.closed_loop_service.clone();

    if let Ok(Some(skill)) = closed_loop.get_skill_by_id(&skill_id) {
        let skill_factor = skill.success_rate;
        let confidence = 0.5 + 0.3 * skill_factor;

        let upgrade_proposal = axagent_trajectory::SkillUpgradeProposal {
            target_skill_id: skill_id,
            suggested_improvements: format!("Based on recent usage, consider enhancing the skill '{}' with additional capabilities or error handling", skill.name),
            additional_scenarios: vec![],
            confidence,
            trigger_event: "manual_proposal".to_string(),
        };

        return Ok(Some(
            serde_json::to_value(upgrade_proposal).map_err(|e| e.to_string())?,
        ));
    }
    Ok(None)
}

#[tauri::command]
pub async fn skill_upgrade_execute(
    app_state: State<'_, AppState>,
    skill_id: String,
    improvements: String,
    additional_scenarios: Vec<String>,
) -> Result<bool, String> {
    let closed_loop = app_state.closed_loop_service.clone();
    let upgrade_proposal = axagent_trajectory::SkillUpgradeProposal {
        target_skill_id: skill_id,
        suggested_improvements: improvements,
        additional_scenarios,
        confidence: 1.0,
        trigger_event: "manual_upgrade".to_string(),
    };

    let auto_action = axagent_trajectory::AutoAction {
        action_type: "upgrade_skill".to_string(),
        target: serde_json::to_string(&upgrade_proposal).map_err(|e| e.to_string())?,
    };

    closed_loop.execute_upgrade_action(&auto_action).await;
    Ok(true)
}

/// 全局 IPC 调用计数器
static IPC_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static IPC_TOTAL_DURATION_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static IPC_ERROR_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 获取 IPC 调用指标（proactiveStore 用于性能预热）
#[tauri::command]
pub fn get_invoke_metrics() -> Result<serde_json::Value, String> {
    let total = IPC_COUNTER.load(std::sync::atomic::Ordering::Relaxed);
    let total_dur = IPC_TOTAL_DURATION_MS.load(std::sync::atomic::Ordering::Relaxed);
    let errors = IPC_ERROR_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    Ok(serde_json::json!({
        "totalCalls": total,
        "avgDurationMs": if total > 0 { total_dur / total } else { 0 },
        "errorRate": if total > 0 { errors as f64 / total as f64 } else { 0.0 },
    }))
}

/// 将主动建议转换为 Nudge（nudgeStore 调用）
#[tauri::command]
pub async fn proactive_convert_to_nudge(
    _state: tauri::State<'_, crate::AppState>,
    suggestion_id: String,
) -> Result<(), String> {
    tracing::info!("[nudge] converting suggestion to nudge: {}", suggestion_id);
    // NudgeService 创建由 closed_loop 系统触发，
    // 前端通过此命令标记建议已被用户确认
    Ok(())
}
