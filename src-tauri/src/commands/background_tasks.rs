// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::task as task_err;
use crate::commands::spawn_guard::panic_message;
use agent_macro::agent_command;
use axagent_entities::background_tasks;
use chrono::Utc;
use futures::FutureExt;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, State};
use tracing::warn;

/// 对传给 shell 的命令参数做基础转义，防止命令注入。
/// 仅允许字母数字、空格和常见安全字符，拒绝包含 shell 元字符的命令。
fn validate_command(cmd: &str) -> Result<(), String> {
    // 危险字符黑名单：管道、重定向、命令分隔符、命令替换、变量展开、单引号（纵深防御）
    const DANGEROUS_CHARS: &[char] =
        &[';', '&', '|', '`', '$', '(', ')', '<', '>', '\n', '\r', '\''];
    for ch in DANGEROUS_CHARS {
        if cmd.contains(*ch) {
            warn!("background_tasks: 命令包含危险字符 '{}', 已拒绝: {}", ch, cmd);
            return Err(serde_json::to_string(
                &ErrorResponse::new(task_err::DANGEROUS_COMMAND)
                    .with_detail(format!("命令包含危险字符 '{}', 已拒绝: {}", ch, cmd)),
            )
            .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e)));
        }
    }
    Ok(())
}

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
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| {
            serde_json::to_string(&ErrorResponse::new(task_err::NOT_FOUND))
                .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))
        })?;
    let mut new_output = task.output.clone();
    new_output.push_str(text);
    if !text.ends_with('\n') {
        new_output.push('\n');
    }
    let mut am: background_tasks::ActiveModel = task.into();
    am.output = Set(new_output);
    am.updated_at = Set(now);
    am.update(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
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
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| {
            serde_json::to_string(&ErrorResponse::new(task_err::NOT_FOUND))
                .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))
        })?;
    let mut am: background_tasks::ActiveModel = task.into();
    am.status = Set(status.to_string());
    am.updated_at = Set(now);
    if let Some(code) = exit_code {
        am.exit_code = Set(Some(code));
    }
    if status == "completed" || status == "failed" || status == "stopped" {
        am.finished_at = Set(Some(now));
    }
    am.update(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(())
}

#[agent_command(domain = system, safety = Caution, call_mode = StateInput, description = "创建后台任务")]
#[allow(clippy::too_many_arguments)]
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
    let db = state.harness.db().clone();
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
    background_tasks::Entity::insert(model).exec(&db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    if task_type == "bash" {
        if let Some(cmd) = command {
            // 安全校验：拒绝包含 shell 元字符的命令，防止命令注入
            validate_command(&cmd)?;
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
                // === 1. Drop guard：任何路径退出都把 status 收敛到终态 ===
                // 业务完成前 finished=false；任何 panic / 早退 / 早 return
                // 都会触发 drop() 兜底：把 status 强制落为 failed 并 emit
                // `background-task:failed` 事件，防止任务卡在 running。
                struct TaskGuard {
                    db: sea_orm::DatabaseConnection,
                    task_id: String,
                    app: tauri::AppHandle,
                    finished: AtomicBool,
                }
                impl Drop for TaskGuard {
                    fn drop(&mut self) {
                        if self.finished.load(Ordering::Acquire) {
                            return;
                        }
                        // drop 路径上不能 .await，用 fire-and-forget spawn 兜底写库
                        let db = self.db.clone();
                        let tid = self.task_id.clone();
                        let app = self.app.clone();
                        tokio::spawn(async move {
                            if let Err(e) = update_status(&db, &tid, "failed", Some(-1)).await {
                                warn!("TaskGuard drop 兜底更新 status 失败: {}", e);
                            }
                            let _ = app.emit("background-task:failed", &tid);
                        });
                    }
                }
                let guard = TaskGuard {
                    db: db1.clone(),
                    task_id: tid1.clone(),
                    app: app.clone(),
                    finished: AtomicBool::new(false),
                };

                // === 2. catch_unwind 包裹主体 ===
                let result = AssertUnwindSafe(async {
                    if let Err(e) = update_status(&db1, &tid1, "running", None).await {
                        warn!("更新任务状态失败: {}", e);
                    }
                    let mut cmd_builder =
                        tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" });
                    cmd_builder
                        .arg(if cfg!(windows) { "/C" } else { "-c" })
                        .arg(&cmd)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped());
                    // Windows: 隐藏控制台窗口
                    #[cfg(windows)]
                    axagent_kit::utils::hide_window(cmd_builder.as_std_mut());
                    let mut child = match cmd_builder.spawn() {
                        Ok(c) => c,
                        Err(e) => {
                            if let Err(e2) =
                                append_output(&db2, &tid2, &format!("启动失败: {}", e)).await
                            {
                                warn!("追加输出失败: {}", e2);
                            }
                            // Drop guard 会兜底更新 status=failed + emit failed 事件
                            return;
                        },
                    };
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();
                    let db_o = db3.clone();
                    let tid_o = tid3.clone();
                    // stdout_task: 子 spawn panic 只会丢部分输出
                    // 用 catch_unwind 兜住, 不污染外层状态机
                    let stdout_task = tokio::spawn(async move {
                        let inner = AssertUnwindSafe(async {
                            if let Some(mut reader) = stdout {
                                use tokio::io::AsyncBufReadExt;
                                let mut lines = tokio::io::BufReader::new(&mut reader).lines();
                                while let Ok(Some(line)) = lines.next_line().await {
                                    if let Err(e) = append_output(&db_o, &tid_o, &line).await {
                                        warn!("追加 stdout 输出失败: {}", e);
                                    }
                                }
                            }
                        })
                        .catch_unwind()
                        .await;
                        if let Err(p) = inner {
                            tracing::error!(
                                "[spawn_background_task] stdout_task PANIC task={}: {}",
                                tid_o,
                                panic_message(&p)
                            );
                        }
                    });
                    let db_e = db3.clone();
                    let tid_e = tid5.clone();
                    let stderr_task = tokio::spawn(async move {
                        let inner = AssertUnwindSafe(async {
                            if let Some(mut reader) = stderr {
                                use tokio::io::AsyncBufReadExt;
                                let mut lines = tokio::io::BufReader::new(&mut reader).lines();
                                while let Ok(Some(line)) = lines.next_line().await {
                                    if let Err(e) =
                                        append_output(&db_e, &tid_e, &format!("[stderr] {}", line))
                                            .await
                                    {
                                        warn!("追加 stderr 输出失败: {}", e);
                                    }
                                }
                            }
                        })
                        .catch_unwind()
                        .await;
                        if let Err(p) = inner {
                            tracing::error!(
                                "[spawn_background_task] stderr_task PANIC task={}: {}",
                                tid_e,
                                panic_message(&p)
                            );
                        }
                    });
                    let status = child.wait().await;
                    let _ = stdout_task.await;
                    let _ = stderr_task.await;
                    match status {
                        Ok(exit) => {
                            let code = exit.code().unwrap_or(-1);
                            if exit.success() {
                                if let Err(e) = append_output(
                                    &db3,
                                    &tid4,
                                    &format!("\n--- 完成 (exit: {}) ---", code),
                                )
                                .await
                                {
                                    tracing::warn!("后台任务追加输出失败 task_id={}: {}", tid4, e);
                                }
                                if let Err(e) =
                                    update_status(&db3, &tid4, "completed", Some(code)).await
                                {
                                    tracing::warn!(
                                        "后台任务状态更新为 completed 失败 task_id={}: {}",
                                        tid4,
                                        e
                                    );
                                }
                            } else {
                                if let Err(e) = append_output(
                                    &db3,
                                    &tid4,
                                    &format!("\n--- 失败 (exit: {}) ---", code),
                                )
                                .await
                                {
                                    tracing::warn!("后台任务追加输出失败 task_id={}: {}", tid4, e);
                                }
                                if let Err(e) =
                                    update_status(&db3, &tid4, "failed", Some(code)).await
                                {
                                    tracing::warn!(
                                        "后台任务状态更新为 failed 失败 task_id={}: {}",
                                        tid4,
                                        e
                                    );
                                }
                            }
                        },
                        Err(e) => {
                            if let Err(ee) =
                                append_output(&db3, &tid4, &format!("\n--- 执行错误: {} ---", e))
                                    .await
                            {
                                tracing::warn!("后台任务追加输出失败 task_id={}: {}", tid4, ee);
                            }
                            // Drop guard 兜底覆盖 status=failed
                            return;
                        },
                    }
                    let _ = app.emit("background-task:updated", &tid4);
                    // === 3. 业务正常完成, 通知 Drop guard 跳过兜底 ===
                    guard.finished.store(true, Ordering::Release);
                })
                .catch_unwind()
                .await;

                if let Err(panic) = result {
                    // Drop guard 在 outer scope 末尾 drop, 会自动把 status 落为 failed
                    tracing::error!(
                        "[spawn_background_task] PANIC task={}: {}",
                        tid1,
                        panic_message(&panic)
                    );
                }
                // guard 在此 drop —— finished=false 时触发兜底
            });
        }
    } else if task_type == "agent" {
        if let Err(e) = update_status(&db, &id, "running", None).await {
            tracing::warn!("后台任务启动状态更新为 running 失败 task_id={}: {}", id, e);
        }
    }
    let _ = app_handle.emit("background-task:created", &id);
    Ok(id)
}

#[agent_command(domain = system, safety = Safe, call_mode = StateOnly, description = "列出后台任务")]
#[tauri::command]
pub async fn list_background_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<BackgroundTaskInfo>, String> {
    let tasks = background_tasks::Entity::find()
        .order_by_desc(background_tasks::Column::CreatedAt)
        .all(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    Ok(tasks.into_iter().map(Into::into).collect())
}

#[agent_command(domain = system, safety = Safe, call_mode = StateInput, description = "获取后台任务输出")]
#[tauri::command]
pub async fn get_background_task_output(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<BackgroundTaskInfo, String> {
    let task = background_tasks::Entity::find_by_id(&task_id)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| {
            serde_json::to_string(&ErrorResponse::new(task_err::NOT_FOUND))
                .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))
        })?;
    Ok(task.into())
}

#[agent_command(domain = system, safety = Caution, call_mode = StateInput, description = "停止后台任务")]
#[tauri::command]
pub async fn stop_background_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    let task = background_tasks::Entity::find_by_id(&task_id)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .ok_or_else(|| {
            serde_json::to_string(&ErrorResponse::new(task_err::NOT_FOUND))
                .unwrap_or_else(|e| format!("{{\"error\":\"serialization failed: {}\"}}", e))
        })?;
    if task.status == "running" || task.status == "pending" {
        update_status(state.harness.db(), &task_id, "stopped", None).await?;
    }
    Ok(())
}
