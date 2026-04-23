use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn insight_list(
    app_state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let is = app_state.insight_system.read().unwrap();
    let insights = is.get_insights();
    Ok(insights.iter()
        .filter_map(|i| serde_json::to_value(i).ok())
        .collect())
}

#[tauri::command]
pub async fn insight_get_by_category(
    app_state: State<'_, AppState>,
    category: String,
) -> Result<Vec<serde_json::Value>, String> {
    let cat = match category.as_str() {
        "pattern" => axagent_trajectory::InsightCategory::Pattern,
        "preference" => axagent_trajectory::InsightCategory::Preference,
        "improvement" => axagent_trajectory::InsightCategory::Improvement,
        "warning" => axagent_trajectory::InsightCategory::Warning,
        _ => return Err(format!("Unknown insight category: {}", category)),
    };
    let is = app_state.insight_system.read().unwrap();
    let insights = is.get_insights_by_category(cat);
    Ok(insights.iter()
        .filter_map(|i| serde_json::to_value(i).ok())
        .collect())
}

#[tauri::command]
pub async fn insight_report(
    app_state: State<'_, AppState>,
    session_id: String,
    message_count: Option<usize>,
) -> Result<serde_json::Value, String> {
    let mut is = app_state.insight_system.write().unwrap();
    let report = is.generate_session_report(
        &session_id,
        message_count.unwrap_or(0),
        vec![],
    );
    Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
}
