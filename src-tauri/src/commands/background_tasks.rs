use crate::AppState;
use axagent_core::entity::background_tasks;
use chrono::Utc;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTaskInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub task_type: String,
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub status: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub conversation_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

impl From<background_tasks::Model> for BackgroundTaskInfo {
    fn from(m: background_tasks::Model) -> Self {
        Self {
            id: m.id,
            title: m.title,
            description: m.description,
            task_type: m.task_type,
            command: m.command,
            prompt: m.prompt,
            status: m.status,
            output: m.output,
            exit_code: m.exit_code,
            conversation_id: m.conversation_id,
            created_at: m.created_at,
            updated_at: m.updated_at,
            finished_at: m.finished_at,
        }
    }
}

async fn append_output(db: &DatabaseConnection, task_id: &str, text: &str) -> Result<(), String> {
    let now = Utc::now().timestamp_millis();
    let task = background_tasks::Entity::find_by_id(task_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "任务未找到".to_string())?;
    let mut new_output = task.output.clone();
    new_output.push_str(text);
    if !text.ends_with('\n') {
        new_output.push('\n');
    }
    let mut am: background_tasks::ActiveModel = task.into();
    am.output = Set(new_output);
    am.updated_at = Set(now);
    am.update(db).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn update_status(
    db: &DatabaseConnection,
    task_id: &str,
    status: &str,
    exit_code: Option<i32>,
) -> Result<(), String> {
    let now = Utc::now().timestamp_millis();
    let task = background_tasks::Entity::find_by_id(task_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "任务未找到".to_string())?;
    let mut am: background_tasks::ActiveModel = task.into();
    am.status = Set(status.to_string());
    am.updated_at = Set(now);
    if let Some(code) = exit_code {
        am.exit_code = Set(Some(code));
    }
    if status == "completed" || status == "failed" || status == "stopped" {
        am.finished_at = Set(Some(now));
    }
    am.update(db).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn spawn_background_task(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    title: String,
    task_type: String,
    command: Option<String>,
    prompt: Option<String>,
    description: Option<String>,
) -> Result<String, String> {
    let db = state.sea_db.clone();
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().timestamp_millis();

    let model = background_tasks::ActiveModel {
        id: Set(id.clone()),
        title: Set(title.clone()),
        description: Set(description.unwrap_or_default()),
        task_type: Set(task_type.clone()),
        command: Set(command.clone()),
        prompt: Set(prompt.clone()),
        status: Set("pending".to_string()),
        output: Set(String::new()),
        exit_code: Set(None),
        conversation_id: Set(None),
        created_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        finished_at: Set(None),
    };
    background_tasks::Entity::insert(model)
        .exec(&db)
        .await
        .map_err(|e| e.to_string())?;

    if task_type == "bash" {
        if let Some(cmd) = command {
            let db1 = db.clone();
            let db2 = db.clone();
            let db3 = db.clone();
            let tid1 = id.clone();
            let tid2 = id.clone();
            let tid3 = id.clone();
            let tid4 = id.clone();
            let tid5 = id.clone();
            let app = app_handle.clone();
            tokio::spawn(async move {
                let _ = update_status(&db1, &tid1, "running", None).await;
                let mut child =
                    match tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
                        .arg(if cfg!(windows) { "/C" } else { "-c" })
                        .arg(&cmd)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .spawn()
                    {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = append_output(&db2, &tid2, &format!("启动失败: {}", e)).await;
                            let _ = update_status(&db2, &tid2, "failed", Some(-1)).await;
                            let _ = app.emit("background-task:updated", &tid2);
                            return;
                        },
                    };
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                let db_o = db3.clone();
                let tid_o = tid3.clone();
                let stdout_task = tokio::spawn(async move {
                    if let Some(mut reader) = stdout {
                        use tokio::io::AsyncBufReadExt;
                        let mut lines = tokio::io::BufReader::new(&mut reader).lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ = append_output(&db_o, &tid_o, &line).await;
                        }
                    }
                });
                let db_e = db3.clone();
                let tid_e = tid5.clone();
                let stderr_task = tokio::spawn(async move {
                    if let Some(mut reader) = stderr {
                        use tokio::io::AsyncBufReadExt;
                        let mut lines = tokio::io::BufReader::new(&mut reader).lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ =
                                append_output(&db_e, &tid_e, &format!("[stderr] {}", line)).await;
                        }
                    }
                });
                let status = child.wait().await;
                let _ = stdout_task.await;
                let _ = stderr_task.await;
                match status {
                    Ok(exit) => {
                        let code = exit.code().unwrap_or(-1);
                        if exit.success() {
                            let _ = append_output(
                                &db3,
                                &tid4,
                                &format!("\n--- 完成 (exit: {}) ---", code),
                            )
                            .await;
                            let _ = update_status(&db3, &tid4, "completed", Some(code)).await;
                        } else {
                            let _ = append_output(
                                &db3,
                                &tid4,
                                &format!("\n--- 失败 (exit: {}) ---", code),
                            )
                            .await;
                            let _ = update_status(&db3, &tid4, "failed", Some(code)).await;
                        }
                    },
                    Err(e) => {
                        let _ =
                            append_output(&db3, &tid4, &format!("\n--- 执行错误: {} ---", e)).await;
                        let _ = update_status(&db3, &tid4, "failed", Some(-1)).await;
                    },
                }
                let _ = app.emit("background-task:updated", &tid4);
            });
        }
    } else if task_type == "agent" {
        let _ = update_status(&db, &id, "running", None).await;
    }
    let _ = app_handle.emit("background-task:created", &id);
    Ok(id)
}

#[tauri::command]
pub async fn list_background_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<BackgroundTaskInfo>, String> {
    let tasks = background_tasks::Entity::find()
        .order_by_desc(background_tasks::Column::CreatedAt)
        .all(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tasks.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn get_background_task_output(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<BackgroundTaskInfo, String> {
    let task = background_tasks::Entity::find_by_id(&task_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "任务未找到".to_string())?;
    Ok(task.into())
}

#[tauri::command]
pub async fn stop_background_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    let task = background_tasks::Entity::find_by_id(&task_id)
        .one(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "任务未找到".to_string())?;
    if task.status == "running" || task.status == "pending" {
        update_status(&state.sea_db, &task_id, "stopped", None).await?;
    }
    Ok(())
}
