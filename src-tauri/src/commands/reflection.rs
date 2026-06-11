// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent::{Reflection, TaskExecutionRecord};
use chrono::Utc;
use tauri::State;

#[tauri::command]
pub async fn reflect_on_task(
    state: State<'_, AppState>,
    task_id: String,
    task_description: String,
    success: bool,
    error: Option<String>,
    tools_used: Vec<String>,
    iterations: usize,
    duration_ms: u64,
) -> Result<Reflection, String> {
    let now = Utc::now();
    let end = now;
    let start = now - chrono::Duration::milliseconds(duration_ms as i64);

    let mut record = TaskExecutionRecord::new(task_id, task_description, start, end);
    record.compute_duration();
    record = record
        .with_success(success)
        .with_tools(tools_used)
        .with_iterations(iterations);
    if let Some(e) = error {
        record = record.with_error(e);
    }

    let reflection = state.reflector.reflect(&record).await;
    Ok(reflection)
}

#[tauri::command]
pub async fn get_reflection_history(state: State<'_, AppState>) -> Result<Vec<Reflection>, String> {
    let history = state.reflector.get_history().await;
    Ok(history)
}

#[tauri::command]
pub async fn clear_reflection_history(state: State<'_, AppState>) -> Result<(), String> {
    state.reflector.clear_history().await;
    Ok(())
}

#[tauri::command]
pub async fn get_reflection_insights(
    state: State<'_, AppState>,
    category: Option<String>,
) -> Result<Vec<axagent_agent::Insight>, String> {
    let generator = state.reflector.get_insight_generator();
    let category_enum = category.and_then(|c| match c.as_str() {
        "error_pattern" => Some(axagent_agent::InsightCategory::ErrorPattern),
        "success_pattern" => Some(axagent_agent::InsightCategory::SuccessPattern),
        "optimization" => Some(axagent_agent::InsightCategory::Optimization),
        "knowledge" => Some(axagent_agent::InsightCategory::Knowledge),
        "workflow" => Some(axagent_agent::InsightCategory::Workflow),
        "tool_usage" => Some(axagent_agent::InsightCategory::ToolUsage),
        _ => None,
    });
    let insights = generator.get_insights(category_enum).await;
    Ok(insights)
}

#[tauri::command]
pub async fn search_reflection_insights(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<axagent_agent::Insight>, String> {
    let generator = state.reflector.get_insight_generator();
    let insights = generator.search_insights(&query).await;
    Ok(insights)
}

#[tauri::command]
pub async fn get_reflection_insight_stats(
    state: State<'_, AppState>,
) -> Result<axagent_agent::InsightStats, String> {
    let generator = state.reflector.get_insight_generator();
    let stats = generator.get_stats().await;
    Ok(stats)
}
