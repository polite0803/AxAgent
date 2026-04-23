use crate::AppState;
use axagent_trajectory::{ExecutionResult, ExecutionStrategy, ParallelTask};
use tauri::State;

#[tauri::command]
pub async fn create_parallel_execution(
    state: State<'_, AppState>,
    name: String,
    description: String,
    tasks: Vec<(String, String, String)>,
    strategy: ExecutionStrategy,
    max_parallel: usize,
) -> Result<String, String> {
    let service = state.parallel_execution_service.read().await;
    service
        .create_execution(name, description, tasks, strategy, max_parallel)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_parallel_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<Option<axagent_trajectory::ParallelExecution>, String> {
    let service = state.parallel_execution_service.read().await;
    Ok(service.get_execution(&execution_id).await)
}

#[tauri::command]
pub async fn list_parallel_executions(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_trajectory::ParallelExecution>, String> {
    let service = state.parallel_execution_service.read().await;
    Ok(service.list_executions().await)
}

#[tauri::command]
pub async fn get_next_pending_task(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<Option<ParallelTask>, String> {
    let service = state.parallel_execution_service.read().await;
    Ok(service.get_next_pending_task(&execution_id).await)
}

#[tauri::command]
pub async fn update_task_result(
    state: State<'_, AppState>,
    execution_id: String,
    task_id: String,
    result: String,
) -> Result<(), String> {
    let service = state.parallel_execution_service.write().await;
    service
        .update_task_result(&execution_id, &task_id, result)
        .await
        .ok_or_else(|| "Failed to update task result".to_string())
}

#[tauri::command]
pub async fn update_task_error(
    state: State<'_, AppState>,
    execution_id: String,
    task_id: String,
    error: String,
) -> Result<(), String> {
    let service = state.parallel_execution_service.write().await;
    service
        .update_task_error(&execution_id, &task_id, error)
        .await
        .ok_or_else(|| "Failed to update task error".to_string())
}

#[tauri::command]
pub async fn cancel_parallel_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<(), String> {
    let service = state.parallel_execution_service.write().await;
    service
        .cancel_execution(&execution_id)
        .await
        .ok_or_else(|| "Failed to cancel execution".to_string())
}

#[tauri::command]
pub async fn get_execution_result(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<Option<ExecutionResult>, String> {
    let service = state.parallel_execution_service.read().await;
    Ok(service.get_execution_result(&execution_id).await)
}

#[tauri::command]
pub async fn delete_parallel_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<bool, String> {
    let service = state.parallel_execution_service.write().await;
    Ok(service.delete_execution(&execution_id).await)
}

#[tauri::command]
pub async fn start_parallel_execution(
    state: State<'_, AppState>,
    execution_id: String,
) -> Result<(), String> {
    let service = state.parallel_execution_service.write().await;
    service
        .start_execution(&execution_id)
        .await
        .ok_or_else(|| "Execution not found".to_string())
}
