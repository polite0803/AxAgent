// SPDX-License-Identifier: AGPL-3.0-only

//! 任务「不丢失、不重复」能力 — 恢复引导。
//!
//! 启动时扫描 `background_tasks` 中未完成（pending / running）的任务，重新入队。
//! - `bash` 类型：`attempt += 1` 后重跑（临时失败重试）。
//! - `agent` 类型：从 `resume_from`（断点位置）续跑；无断点则复位为 pending。
//!
//! Tauri 命令入口 `restore_pending_tasks`（手动触发，调试用），启动钩子
//! 对应实现由 `init/services.rs` 调用，通过传入 `DatabaseConnection` 与
//! 可选的 `AppHandle` 发送事件，保持本模块与 `AppState` 解耦。

use axagent_entities::background_tasks;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tauri::{Emitter, State};
use tracing::info;

use crate::AppState;

/// 未完成任务将重新入队的可见状态集合
const INCOMPLETE: &[&str] = &["pending", "running"];

/// Tauri 命令：手动触发恢复引导（调试/运维用）。
#[tauri::command]
pub async fn restore_pending_tasks(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let db = state.harness.db().clone();
    restore_incomplete_tasks(&db, Some(&app_handle)).await.map_err(|e| {
        crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Retryable,
        )
        .to_string()
    })
}

/// 恢复引导核心逻辑：扫描未完成任务 → 置回 pending（attempt 自增）。
///
/// 幂等：每个任务原子地复位为 pending；重复调用不会重复计数。
/// 返回被恢复的任务 id 列表。
pub async fn restore_incomplete_tasks(
    db: &sea_orm::DatabaseConnection,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<Vec<String>, sea_orm::DbErr> {
    let tasks = background_tasks::Entity::find()
        .filter(background_tasks::Column::Status.is_in(INCOMPLETE.to_vec()))
        .all(db)
        .await?;

    let mut recovered = Vec::new();
    for task in tasks {
        let task_id = task.id.clone();
        let next_attempt = task.attempt.saturating_add(1);
        // agent 任务无断点则从零开始；有断点保留 resume_from 供续跑。
        let resume_from = if task.task_type == "agent" {
            task.resume_from.clone()
        } else {
            None
        };
        let mut am: background_tasks::ActiveModel = task.into();
        let now = chrono::Utc::now().timestamp();
        am.status = Set("pending".to_string());
        am.resume_from = Set(resume_from);
        am.attempt = Set(next_attempt);
        am.finished_at = Set(None);
        am.updated_at = Set(now);
        am.update(db).await?;
        recovered.push(task_id);
    }

    if !recovered.is_empty() {
        info!("[scheduler.restore] 恢复 {} 个未完成任务: {:?}", recovered.len(), recovered);
        if let Some(app) = app_handle {
            let _ = app.emit("background-task:restored", &recovered);
        }
    } else {
        info!("[scheduler.restore] 无不完整后台任务，无需恢复");
    }
    Ok(recovered)
}
