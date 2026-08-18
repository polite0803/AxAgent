// SPDX-License-Identifier: AGPL-3.0-only

//! 夜间长时自主任务运行的调度协调器（wiring 角色，纯函数式增强层）。
//!
//! 设计依据 `docs/夜间长时自主任务运行-详细设计.md`：
//! - **不加新调度线程**：夜间任务统一表示为 `recurring=false` 的一次性 CronJob，
//!   由现有 `CronJobStore::list_due` 轮询触发（见 `init/services.rs` 的
//!   `start_cron_scheduler`），本模块只做编排（恢复 / 排序 / 择时门控 / 汇报），
//!   不持有自己的定时循环。
//! - **扩展既有结构而非重造轮子**：幂等键 / attempt / resume_from 字段已加在
//!   `background_tasks` 实体；优先级 / 预估成本字段已加在 `CronJob`。
//!
//! 子模块：
//! - [`queue`]  优先级排序策略（复用 `CronJob.priority` / `epoch_cost_estimate`）
//! - [`gate`]   动态择时 + 成本门控（max_budget 熔断）
//! - [`report`] 长时任务汇报（成本折算 + 审批记录汇总）
//! - [`restore`] 恢复引导（未完成任务重新入队）

pub mod gate;
pub mod queue;
pub mod report;
pub mod restore;

use crate::AppState;
use crate::scheduler::gate::{BudgetState, get_budget_usage_impl, set_budget_impl};
use crate::scheduler::queue::set_task_priority as queue_set_priority;
use crate::scheduler::report::get_task_report as build_report_cmd;
use sea_orm::DatabaseConnection;
use tauri::State;

/// 取共享数据库连接。
fn db(state: &State<'_, AppState>) -> DatabaseConnection {
    state.harness.db().clone()
}

/// Tauri 命令：运行中重排序（复用 CronJobStore 更新内存+持久化）。
#[tauri::command]
pub async fn set_task_priority(
    state: State<'_, AppState>,
    job_id: String,
    priority: String,
) -> Result<String, String> {
    queue_set_priority(&state.cron_job_store, &job_id, &priority).await
}

/// Tauri 命令：设置成本上限（USD）。传 null/负值时按 None 处理（不限）。
#[tauri::command]
pub async fn set_budget(
    state: State<'_, AppState>,
    max_budget: Option<f64>,
) -> Result<BudgetState, String> {
    let mut budget = state.scheduler_budget.write().await;
    set_budget_impl(&mut budget, max_budget)
}

/// Tauri 命令：查询预算用量（含熔断状态）。
#[tauri::command]
pub async fn get_budget_usage(state: State<'_, AppState>) -> Result<BudgetState, String> {
    let budget = state.scheduler_budget.read().await;
    Ok(get_budget_usage_impl(&budget))
}

/// Tauri 命令：取长时任务报告（含成本折算与审批汇总）。
#[tauri::command]
pub async fn get_task_report(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<crate::scheduler::report::TaskReport, String> {
    let budget = state.scheduler_budget.read().await;
    build_report_cmd(&db(&state), &task_id, &budget).await
}

/// 供接线层调用的恢复引导（避开 Tauri 命令签名，供 init/services.rs 用）。
pub async fn restore_pending_tasks_impl(
    state: &AppState,
    app_handle: &tauri::AppHandle,
) -> Result<Vec<String>, String> {
    crate::scheduler::restore::restore_incomplete_tasks(state.harness.db(), Some(app_handle))
        .await
        .map_err(|e| {
            crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Retryable,
            )
            .to_string()
        })
}
