use crate::AppState;
use axagent_core::repo::scheduled_task as db_repo;
use axagent_trajectory::{ScheduledTask, TaskDefinition, TaskRunResult, TaskType};
use tauri::State;

fn task_to_entity(task: &ScheduledTask) -> axagent_core::entity::scheduled_tasks::ActiveModel {
    let task_type = task.task_type.as_str().to_string();
    let status = match task.status {
        axagent_trajectory::ScheduledTaskStatus::Active => "active".to_string(),
        axagent_trajectory::ScheduledTaskStatus::Paused => "paused".to_string(),
        axagent_trajectory::ScheduledTaskStatus::Disabled => "disabled".to_string(),
    };
    let config = serde_json::to_string(&task.config).unwrap_or_default();
    let schedule_config = serde_json::to_string(&task.schedule_config).unwrap_or_default();
    let last_result = task
        .last_result
        .as_ref()
        .and_then(|r| serde_json::to_string(r).ok());

    axagent_core::entity::scheduled_tasks::ActiveModel {
        id: sea_orm::Set(task.id.clone()),
        name: sea_orm::Set(task.name.clone()),
        description: sea_orm::Set(task.description.clone()),
        task_type: sea_orm::Set(task_type),
        workflow_id: sea_orm::Set(task.workflow_id.clone()),
        cron_expression: sea_orm::Set(Some(schedule_config)),
        interval_seconds: sea_orm::Set(task.schedule_config.interval_seconds.map(|v| v as i64)),
        next_run_at: sea_orm::Set(task.next_run_at.timestamp_millis()),
        last_run_at: sea_orm::Set(task.last_run_at.map(|dt| dt.timestamp_millis())),
        last_result: sea_orm::Set(last_result),
        status: sea_orm::Set(status),
        config: sea_orm::Set(config),
        created_at: sea_orm::Set(task.created_at.timestamp_millis()),
        updated_at: sea_orm::Set(task.updated_at.timestamp_millis()),
    }
}

fn entity_to_task(model: &axagent_core::entity::scheduled_tasks::Model) -> ScheduledTask {
    let task_type = match model.task_type.as_str() {
        "daily_summary" => TaskType::DailySummary,
        "backup" => TaskType::Backup,
        "cleanup" => TaskType::Cleanup,
        "custom" => TaskType::Custom,
        "health_check" => TaskType::HealthCheck,
        "data_sync" => TaskType::DataSync,
        "workflow" => TaskType::Workflow,
        _ => TaskType::Custom,
    };

    let status = match model.status.as_str() {
        "active" => axagent_trajectory::ScheduledTaskStatus::Active,
        "paused" => axagent_trajectory::ScheduledTaskStatus::Paused,
        "disabled" => axagent_trajectory::ScheduledTaskStatus::Disabled,
        _ => axagent_trajectory::ScheduledTaskStatus::Active,
    };

    let config: axagent_trajectory::TaskConfig =
        serde_json::from_str(&model.config).unwrap_or_default();
    let schedule_config: axagent_trajectory::ScheduleConfig = model
        .cron_expression
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let last_result: Option<TaskRunResult> = model
        .last_result
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());

    ScheduledTask {
        id: model.id.clone(),
        name: model.name.clone(),
        description: model.description.clone(),
        task_type,
        workflow_id: model.workflow_id.clone(),
        schedule_config,
        next_run_at: chrono::DateTime::from_timestamp_millis(model.next_run_at)
            .unwrap_or_else(chrono::Utc::now),
        last_run_at: model
            .last_run_at
            .and_then(chrono::DateTime::from_timestamp_millis),
        last_result,
        status,
        config,
        created_at: chrono::DateTime::from_timestamp_millis(model.created_at)
            .unwrap_or_else(chrono::Utc::now),
        updated_at: chrono::DateTime::from_timestamp_millis(model.updated_at)
            .unwrap_or_else(chrono::Utc::now),
    }
}

#[tauri::command]
pub async fn get_scheduled_task_templates(
) -> Result<Vec<axagent_trajectory::TaskTemplateInfo>, String> {
    Ok(axagent_trajectory::TaskTemplate::all_templates())
}

#[tauri::command]
pub async fn create_scheduled_task(
    state: State<'_, AppState>,
    name: String,
    description: String,
    task_type: TaskType,
    schedule_config: axagent_trajectory::ScheduleConfig,
    workflow_id: Option<String>,
) -> Result<String, String> {
    let service = state.scheduled_task_service.read().await;
    let interval_hours = if let Some(seconds) = schedule_config.interval_seconds {
        seconds / 3600
    } else {
        24
    };
    let next_run = chrono::Utc::now() + chrono::Duration::hours(interval_hours as i64);
    let mut task = ScheduledTask::new(name, description, task_type, next_run)
        .with_schedule_config(schedule_config);
    if let Some(wf_id) = workflow_id {
        task = task.with_workflow_id(wf_id);
    }
    let task_id = service.create_task(task).await.map_err(|e| e.to_string())?;

    let saved_task = service.get_task(&task_id).await.unwrap();
    let entity = task_to_entity(&saved_task);
    db_repo::upsert_scheduled_task(&state.sea_db, entity)
        .await
        .map_err(|e| e.to_string())?;

    Ok(task_id)
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
    let task_id = service
        .create_backup_task(name, description, interval_hours)
        .await
        .map_err(|e| e.to_string())?;

    let saved_task = service.get_task(&task_id).await.unwrap();
    let entity = task_to_entity(&saved_task);
    db_repo::upsert_scheduled_task(&state.sea_db, entity)
        .await
        .map_err(|e| e.to_string())?;

    Ok(task_id)
}

#[tauri::command]
pub async fn create_cleanup_task(
    state: State<'_, AppState>,
    name: String,
    description: String,
    interval_hours: u64,
) -> Result<String, String> {
    let service = state.scheduled_task_service.read().await;
    let task_id = service
        .create_cleanup_task(name, description, interval_hours)
        .await
        .map_err(|e| e.to_string())?;

    let saved_task = service.get_task(&task_id).await.unwrap();
    let entity = task_to_entity(&saved_task);
    db_repo::upsert_scheduled_task(&state.sea_db, entity)
        .await
        .map_err(|e| e.to_string())?;

    Ok(task_id)
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
pub async fn list_due_tasks(state: State<'_, AppState>) -> Result<Vec<ScheduledTask>, String> {
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
        .update_task(&task_id, task.clone())
        .await
        .ok_or_else(|| "Task not found".to_string())?;

    let entity = task_to_entity(&task);
    db_repo::upsert_scheduled_task(&state.sea_db, entity)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<bool, String> {
    let service = state.scheduled_task_service.write().await;
    let result = service.delete_task(&task_id).await;

    if result {
        db_repo::delete_scheduled_task(&state.sea_db, &task_id)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(result)
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
        .ok_or_else(|| "Task not found".to_string())?;

    if let Some(task) = service.get_task(&task_id).await {
        let entity = task_to_entity(&task);
        db_repo::upsert_scheduled_task(&state.sea_db, entity)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
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
        .ok_or_else(|| "Task not found".to_string())?;

    if let Some(task) = service.get_task(&task_id).await {
        let entity = task_to_entity(&task);
        db_repo::upsert_scheduled_task(&state.sea_db, entity)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
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
    service.record_execution(&task_id, result.clone()).await;

    if let Some(task) = service.get_task(&task_id).await {
        let entity = task_to_entity(&task);
        db_repo::upsert_scheduled_task(&state.sea_db, entity)
            .await
            .map_err(|e| e.to_string())?;
    }

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

#[tauri::command]
pub async fn execute_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<TaskRunResult, String> {
    let service = state.scheduled_task_service.read().await;
    service
        .execute_task(&task_id)
        .await
        .ok_or_else(|| "Task not found".to_string())
}

#[tauri::command]
pub async fn load_scheduled_tasks_from_db(state: State<'_, AppState>) -> Result<usize, String> {
    load_tasks_from_db_internal(&state.sea_db, &state.scheduled_task_service).await
}

pub async fn load_tasks_from_db_internal(
    sea_db: &sea_orm::DatabaseConnection,
    scheduled_task_service: &std::sync::Arc<
        tokio::sync::RwLock<axagent_trajectory::ScheduledTaskService>,
    >,
) -> Result<usize, String> {
    let db_models = db_repo::list_scheduled_tasks(sea_db)
        .await
        .map_err(|e| e.to_string())?;

    let service = scheduled_task_service.write().await;
    let mut count = 0;

    for model in db_models {
        let task = entity_to_task(&model);
        if service.add_task(task).await.is_ok() {
            count += 1;
        }
    }

    tracing::info!("[scheduled_task] Loaded {} tasks from database", count);
    Ok(count)
}
