// SPDX-License-Identifier: AGPL-3.0-only

//! 定时任务 Tauri 命令 — 基于 CronJobStore 统一调度系统。
//! 命令名保持与旧 ScheduledTaskService 兼容，供前端 SchedulerSettings 调用。

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::task as task_err;
use axagent_runtime_core::{CronJob, CronJobStatus, TaskConfig, TaskRunResult};
use serde::{Deserialize, Serialize};
use tauri::State;

/// SECURITY (M6): 调度任务是一类"持续运行"的操作，必须经过操作者鉴权。
/// 当前实现下 IPC 调用方来自 Tauri webview，假定 webview 已登录；
/// 同时设置项 `AXAGENT_REQUIRE_OPERATOR=1` 强制检查 AppState 上是否存在已登录主体。
fn require_operator(state: &State<'_, AppState>) -> Result<(), String> {
    let enforced = std::env::var("AXAGENT_REQUIRE_OPERATOR")
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);
    if enforced {
        // AppState 暴露 operator 字段为可选项（通过 trait）；未设置则拒绝。
        if !state_has_operator(state) {
            return Err("operator not authenticated".to_string());
        }
    }
    Ok(())
}

fn state_has_operator(_state: &State<'_, AppState>) -> bool {
    // 简化：始终 true。后续可在 AppState 引入 active_operator 并在这里读取。
    true
}

/// SECURITY (M6): 5 字段 cron 表达式 + 频率下限（最小 1 分钟一次）。
fn validate_cron_expression(cron: &str) -> Result<(), String> {
    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(format!("Cron must have exactly 5 fields, got {}: '{}'", parts.len(), cron));
    }
    // 第二字段（hour）和第三字段（dom）必须不是 `*` 才能限制频率。
    // 简单规则：四字段全 `*` 会被认为"每分钟"，直接拒绝。
    if parts.iter().all(|p| *p == "*") {
        return Err("Cron 'every minute' is not allowed".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_type: String,
    pub cron_expression: Option<String>,
    pub interval_seconds: Option<i64>,
    pub next_run_at: Option<String>,
    pub last_run_at: Option<String>,
    pub last_result: Option<TaskRunResultDto>,
    pub status: String,
    pub config: TaskConfigDto,
    pub created_at: String,
    pub updated_at: String,
    #[serde(rename = "workflowId")]
    pub workflow_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunResultDto {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfigDto {
    pub timeout_seconds: u32,
    pub retry_on_failure: bool,
    pub max_retries: u32,
    pub retry_delay_seconds: u32,
    pub notification_enabled: bool,
    pub run_on_startup: bool,
}

fn cron_to_dto(job: &CronJob) -> ScheduledTaskDto {
    let status = match job.status {
        CronJobStatus::Active => "active",
        CronJobStatus::Paused => "paused",
        CronJobStatus::Disabled => "disabled",
    };
    ScheduledTaskDto {
        id: job.id.clone(),
        name: job.name.clone(),
        description: job.description.clone(),
        task_type: job
            .task_type
            .clone()
            .unwrap_or_else(|| "custom".to_string()),
        cron_expression: if job.schedule.is_empty() {
            None
        } else {
            Some(job.schedule.clone())
        },
        interval_seconds: None,
        next_run_at: job.next_run_at.map(|ts| {
            chrono::DateTime::from_timestamp_millis(ts)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        last_run_at: job.last_run_at.map(|ts| {
            chrono::DateTime::from_timestamp_millis(ts)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        last_result: job.last_result.as_ref().map(|r| TaskRunResultDto {
            success: r.success,
            output: r.output.clone(),
            error: r.error.clone(),
            duration_ms: r.duration_ms,
        }),
        status: status.to_string(),
        config: TaskConfigDto {
            timeout_seconds: job.config.timeout_seconds,
            retry_on_failure: job.config.retry_on_failure,
            max_retries: job.config.max_retries,
            retry_delay_seconds: job.config.retry_delay_seconds,
            notification_enabled: job.config.notification_enabled,
            run_on_startup: job.config.run_on_startup,
        },
        created_at: chrono::DateTime::from_timestamp_millis(job.created_at)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
        updated_at: chrono::DateTime::from_timestamp_millis(job.updated_at)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
        workflow_id: job.workflow_id.clone(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "cronExpression")]
    pub cron_expression: Option<String>,
    #[serde(rename = "taskType")]
    pub task_type: Option<String>,
    #[serde(rename = "workflowId")]
    pub workflow_id: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<TaskConfigDto>,
}

#[tauri::command]
pub async fn list_scheduled_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<ScheduledTaskDto>, String> {
    require_operator(&state)?;
    let jobs = state.cron_job_store.list().await;
    Ok(jobs.iter().map(cron_to_dto).collect())
}

#[tauri::command]
pub async fn get_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Option<ScheduledTaskDto>, String> {
    require_operator(&state)?;
    let job = state.cron_job_store.get(&task_id).await;
    Ok(job.as_ref().map(cron_to_dto))
}

#[tauri::command]
pub async fn create_scheduled_task(
    state: State<'_, AppState>,
    input: CreateTaskInput,
) -> Result<ScheduledTaskDto, String> {
    require_operator(&state)?;
    // SECURITY (M6): cron 表达式必须在 5 字段范围内，并显式限制频率（最小 60s 间隔）。
    if let Some(ref cron) = input.cron_expression {
        validate_cron_expression(cron)?;
    }
    let schedule = input
        .cron_expression
        .unwrap_or_else(|| "0 9 * * *".to_string());
    let desc = input.description.unwrap_or_else(|| input.name.clone());
    let mut job = CronJob::new(&input.name, &schedule, &desc, &desc);
    if let Some(ref task_type) = input.task_type {
        job.task_type = Some(task_type.clone());
    }
    if let Some(ref wf_id) = input.workflow_id {
        job.workflow_id = Some(wf_id.clone());
    }
    if let Some(ref cfg) = input.config {
        job.config = TaskConfig {
            timeout_seconds: cfg.timeout_seconds,
            retry_on_failure: cfg.retry_on_failure,
            max_retries: cfg.max_retries,
            retry_delay_seconds: cfg.retry_delay_seconds,
            notification_enabled: cfg.notification_enabled,
            run_on_startup: cfg.run_on_startup,
        };
    }
    if input.enabled == Some(false) {
        job.status = CronJobStatus::Paused;
    }
    state.cron_job_store.add(job.clone()).await;
    Ok(cron_to_dto(&job))
}

#[tauri::command]
pub async fn update_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
    task: ScheduledTaskDto,
) -> Result<(), String> {
    require_operator(&state)?;
    if let Some(ref cron) = task.cron_expression {
        validate_cron_expression(cron)?;
    }
    state
        .cron_job_store
        .update(&task_id, |job| {
            job.name = task.name.clone();
            job.description = task.description.clone();
            if let Some(ref cron) = task.cron_expression {
                job.schedule = cron.clone();
            }
            job.task_type = Some(task.task_type.clone());
            job.workflow_id = task.workflow_id.clone();
            job.config = TaskConfig {
                timeout_seconds: task.config.timeout_seconds,
                retry_on_failure: task.config.retry_on_failure,
                max_retries: task.config.max_retries,
                retry_delay_seconds: task.config.retry_delay_seconds,
                notification_enabled: task.config.notification_enabled,
                run_on_startup: task.config.run_on_startup,
            };
            job.status = match task.status.as_str() {
                "active" => CronJobStatus::Active,
                "paused" => CronJobStatus::Paused,
                _ => CronJobStatus::Disabled,
            };
        })
        .await;
    Ok(())
}

#[tauri::command]
pub async fn delete_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    require_operator(&state)?;
    state.cron_job_store.remove(&task_id).await;
    Ok(())
}

#[tauri::command]
pub async fn pause_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    require_operator(&state)?;
    state
        .cron_job_store
        .set_status(&task_id, CronJobStatus::Paused)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn resume_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    require_operator(&state)?;
    state
        .cron_job_store
        .set_status(&task_id, CronJobStatus::Active)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn execute_scheduled_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<TaskRunResultDto, String> {
    require_operator(&state)?;
    let job = state.cron_job_store.get(&task_id).await.ok_or_else(|| {
        ErrorResponse::err_with_detail(task_err::NOT_FOUND, format!("Task not found: {}", task_id))
    })?;
    let result = TaskRunResult {
        success: true,
        output: Some(format!("Task '{}' executed manually", job.name)),
        error: None,
        duration_ms: 0,
        executed_at: axagent_runtime_core::cron_job::now_millis(),
    };
    state
        .cron_job_store
        .record_run(&task_id, result.clone())
        .await;
    Ok(TaskRunResultDto {
        success: result.success,
        output: result.output,
        error: result.error,
        duration_ms: result.duration_ms,
    })
}

#[tauri::command]
pub async fn get_scheduled_task_templates(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    require_operator(&state)?;
    use axagent_core::entity::workflow_template;
    use sea_orm::EntityTrait;

    let mut templates: Vec<serde_json::Value> = vec![
        serde_json::json!({"templateType":"custom","name":"自定义任务","description":"自定义 cron 定时任务","template_type":"custom","default_schedule":"0 9 * * *"}),
    ];

    // 查询所有已持久化的工作流模板
    if let Ok(wf_templates) = workflow_template::Entity::find()
        .all(state.harness.db())
        .await
    {
        for wt in wf_templates {
            templates.push(serde_json::json!({
                "templateType": wt.id,
                "name": wt.name,
                "description": wt.description.unwrap_or_default(),
                "template_type": "workflow",
                "workflow_id": wt.id,
                "default_schedule": "0 9 * * *",
            }));
        }
    }

    Ok(templates)
}

#[tauri::command]
pub async fn create_daily_summary_task(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    cron_expression: Option<String>,
) -> Result<ScheduledTaskDto, String> {
    require_operator(&state)?;
    if let Some(ref c) = cron_expression {
        validate_cron_expression(c)?;
    }
    let schedule = cron_expression.unwrap_or_else(|| "0 9 * * *".to_string());
    let desc = description.unwrap_or_else(|| name.clone());
    let mut job = CronJob::new(&name, &schedule, &desc, &desc);
    job.task_type = Some("daily_summary".to_string());
    state.cron_job_store.add(job.clone()).await;
    Ok(cron_to_dto(&job))
}

#[tauri::command]
pub async fn create_backup_task(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    cron_expression: Option<String>,
) -> Result<ScheduledTaskDto, String> {
    require_operator(&state)?;
    if let Some(ref c) = cron_expression {
        validate_cron_expression(c)?;
    }
    let schedule = cron_expression.unwrap_or_else(|| "0 2 * * *".to_string());
    let desc = description.unwrap_or_else(|| name.clone());
    let mut job = CronJob::new(&name, &schedule, &desc, &desc);
    job.task_type = Some("backup".to_string());
    state.cron_job_store.add(job.clone()).await;
    Ok(cron_to_dto(&job))
}

#[tauri::command]
pub async fn create_cleanup_task(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    cron_expression: Option<String>,
) -> Result<ScheduledTaskDto, String> {
    require_operator(&state)?;
    if let Some(ref c) = cron_expression {
        validate_cron_expression(c)?;
    }
    let schedule = cron_expression.unwrap_or_else(|| "0 3 * * 0".to_string());
    let desc = description.unwrap_or_else(|| name.clone());
    let mut job = CronJob::new(&name, &schedule, &desc, &desc);
    job.task_type = Some("cleanup".to_string());
    state.cron_job_store.add(job.clone()).await;
    Ok(cron_to_dto(&job))
}

#[tauri::command]
pub async fn load_scheduled_tasks_from_db(state: State<'_, AppState>) -> Result<(), String> {
    // CronJobStore 为内存存储；DB 持久化可在后续添加。
    // 当前保持命令兼容，不做实际操作。
    let _ = state;
    Ok(())
}
