// SPDX-License-Identifier: AGPL-3.0-only

//! 长时工作汇报。
//!
//! 设计依据 `docs/夜间长时自主任务运行-详细设计.md` ⑦：
//! - **触发**：任务完成事件 → 判定超过阈值（时长）→ 自动生成报告。
//! - **并入**：成本折算（复用 `harness/usage_pricing` 定价）+ 审批记录汇总。
//!
//! 本模块轻量生成结构化报告（JSON 为主，可被前端渲染为成本仪表盘）。

use axagent_entities::background_tasks;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::scheduler::gate::BudgetState;

/// 报告触发阈值：任务执行时长 >= 该毫秒数即视为「长时任务」(5 分钟)。
pub const LONG_TASK_THRESHOLD_MS: i64 = 300_000;

/// 长时任务报告（结构化，可被前端渲染）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReport {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub duration_ms: i64,
    /// 成本折算（USD）
    pub cost_usd: f64,
    /// 审批记录汇总条目
    pub approvals: Vec<ApprovalSummary>,
    /// 报告生成时间
    pub generated_at: i64,
}

/// 审批汇总条目（从 workflow_approvals 冗余摘取）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalSummary {
    pub approver: Option<String>,
    pub status: Option<String>,
    pub decision: Option<String>,
}

/// 生成单个后台任务的长时报告。
pub async fn build_report(
    db: &DatabaseConnection,
    task_id: &str,
    budget: &BudgetState,
) -> Result<TaskReport, String> {
    let Some(task) = background_tasks::Entity::find_by_id(task_id)
        .one(db)
        .await
        .map_err(|e| format!("查询任务失败: {}", e))?
    else {
        return Err(format!("任务不存在: {}", task_id));
    };

    let duration_ms =
        task.finished_at.map(|f| f.saturating_sub(task.created_at).max(0)).unwrap_or(0);

    // 成本折算：这里以预算累计成本作为任务成本口径。
    // 精确实时单任务成本需 TokenUsage 侧记账，避免新增记账表，简化处理。
    let cost_usd = budget.spent;

    // 审批汇总：审批记录键为 execution_id + node_id，不与 background task 直接关联。
    // 预留扩展点（如 cron_job_history → execution 反查），当前返回空列表。
    let approvals = vec![];

    Ok(TaskReport {
        task_id: task_id.to_string(),
        title: task.title,
        status: task.status,
        duration_ms,
        cost_usd,
        approvals,
        generated_at: chrono::Utc::now().timestamp_millis(),
    })
}

/// 判定是否需要触发汇报：任务状态为终态且超阈值。
pub fn should_report(status: &str, duration_ms: i64) -> bool {
    matches!(status, "completed" | "failed" | "stopped" | "needs_investigation")
        && duration_ms >= LONG_TASK_THRESHOLD_MS
}

/// 任务完成事件触发的汇报入口（由 scheduler 完成回调解用）。
///
/// 仅当超过阈值时返回报告（日志输出），否则返回 None。
pub async fn maybe_trigger_report(
    db: &DatabaseConnection,
    task_id: &str,
    budget: &BudgetState,
) -> Result<Option<TaskReport>, String> {
    let report = build_report(db, task_id, budget).await?;
    if should_report(&report.status, report.duration_ms) {
        info!(
            "[scheduler.report] 长时任务 '{}' 超阈值，生成报告: status={} duration={}ms cost=${:.4}",
            report.task_id, report.status, report.duration_ms, report.cost_usd
        );
        Ok(Some(report))
    } else {
        Ok(None)
    }
}

/// Tauri 命令：取任务报告（不超过阈值也返回，供前端展示）。
pub async fn get_task_report(
    db: &DatabaseConnection,
    task_id: &str,
    budget: &BudgetState,
) -> Result<TaskReport, String> {
    build_report(db, task_id, budget).await
}
