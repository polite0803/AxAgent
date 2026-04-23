use crate::AppState;
use axagent_trajectory::{ScheduledTask, TaskDefinition, TaskRunResult, TaskType};
use tauri::State;

#[tauri::command]
pub async fn create_scheduled_task(
    state: State<'_, AppState>,
    name: String,
    description: String,
    task_type: TaskType,
    interval_hours: Option<u64>,
) -> Result<String, String> {
    let service = state.scheduled_task_service.read().await;
    let next_run = chrono::Utc::now() + chrono::Duration::hours(interval_hours.unwrap_or(24) as i64);
    let mut task = ScheduledTask::new(name, description, task_type, next_run);
    if let Some(hours) = interval_hours {
        task = task.with_interval(hours * 3600);
    }
    service.create_task(task).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_daily_summary_task(
    state: State<'_, AppState>,
    name: String,
    description: String,
    hour: u32,
    minute: u32,
) -> Result<String, String> {
    let service = state.scheduled_task_service.read().await;
    service
        .create_daily_summary_task(name, description, hour, minute)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_backup_task(
    state: State<'_, AppState>,
    name: String,
    description: String,
    interval_hours: u64,
) -> Result<String, String> {
    let service = state.scheduled_task_service.read().await;
    service
        .create_backup_task(name, description, interval_hours)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_cleanup_task(
    state: State<'_, AppState>,
    name: String,
    description: String,
    interval_hours: u64,
) -> Result<String, String> {
    let service = state.scheduled_task_service.read().await;
    service
        .create_cleanup_task(name, description, interval_hours)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Option<ScheduledTask>, String> {
    let service = state.scheduled_task_service.read().await;
    Ok(service.get_task(&task_id).await)
}

#[tauri::command]
pub async fn list_scheduled_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<ScheduledTask>, String> {
    let service = state.scheduled_task_service.read().await;
    Ok(service.list_tasks().await)
}

#[tauri::command]
pub async fn list_due_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<ScheduledTask>, String> {
    let service = state.scheduled_task_service.read().await;
    Ok(service.list_due_tasks().await)
}

#[tauri::command]
pub async fn update_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
    task: ScheduledTask,
) -> Result<(), String> {
    let service = state.scheduled_task_service.write().await;
    service
        .update_task(&task_id, task)
        .await
        .ok_or_else(|| "Task not found".to_string())
}

#[tauri::command]
pub async fn delete_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<bool, String> {
    let service = state.scheduled_task_service.write().await;
    Ok(service.delete_task(&task_id).await)
}

#[tauri::command]
pub async fn pause_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    let service = state.scheduled_task_service.write().await;
    service
        .pause_task(&task_id)
        .await
        .ok_or_else(|| "Task not found".to_string())
}

#[tauri::command]
pub async fn resume_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    let service = state.scheduled_task_service.write().await;
    service
        .resume_task(&task_id)
        .await
        .ok_or_else(|| "Task not found".to_string())
}

#[tauri::command]
pub async fn record_task_execution(
    state: State<'_, AppState>,
    task_id: String,
    success: bool,
    output: Option<String>,
    error: Option<String>,
    duration_ms: u64,
) -> Result<(), String> {
    let result = if success {
        TaskRunResult::success(output.unwrap_or_default(), duration_ms)
    } else {
        TaskRunResult::failure(error.unwrap_or_default(), duration_ms)
    };
    let service = state.scheduled_task_service.write().await;
    service.record_execution(&task_id, result).await;
    Ok(())
}

#[tauri::command]
pub async fn get_task_execution_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<TaskRunResult>, String> {
    let service = state.scheduled_task_service.read().await;
    Ok(service.get_execution_history(limit).await)
}

#[tauri::command]
pub async fn get_next_scheduled_time(
    state: State<'_, AppState>,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    let service = state.scheduled_task_service.read().await;
    Ok(service.get_next_scheduled_time().await)
}

#[tauri::command]
pub async fn register_task_definition(
    state: State<'_, AppState>,
    name: String,
    task_type: TaskType,
    prompt_template: String,
) -> Result<String, String> {
    let definition = TaskDefinition::new(name, task_type, prompt_template);
    let service = state.scheduled_task_service.write().await;
    service.register_task_definition(definition).await;
    Ok("OK".to_string())
}
