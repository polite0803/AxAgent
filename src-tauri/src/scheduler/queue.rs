// SPDX-License-Identifier: AGPL-3.0-only

//! 异步排队与优先级调度 — 运行中重排序。
//!
//! 设计依据 `docs/夜间长时自主任务运行-详细设计.md` ③：
//! - 调度排序策略已落在 `CronJobStore::list_due`（`priority desc, estimate asc,
//!   created_at asc`，见 `runtime-core/cron_job.rs`）。
//! - 本模块补充 **运行中重排序** 能力：`set_task_priority` 命令可在任务入队后
//!   调整 `priority`，下次 `list_due` 轮询即按新优先级拾取。

use axagent_runtime_core::cron_job::{CronJobPriority, CronJobStore};
use tracing::{info, warn};

/// 合法的优先级字符串（与 CronJobPriority serde 命名一致）
const VALID: &[&str] = &["low", "medium", "high", "batch"];

/// 运行中重排序：更新 CronJob 的优先级。
///
/// - 优先级非法时返回错误（不静默吞掉，避免误配写入库）。
/// - 优先级合法时更新内存 + 持久化。
pub async fn set_task_priority(
    store: &CronJobStore,
    job_id: &str,
    priority: &str,
) -> Result<String, String> {
    if !VALID.contains(&priority.to_ascii_lowercase().as_str()) {
        return Err(format!("非法优先级 '{}'，仅支持: {}", priority, VALID.join(" / ")));
    }
    if let Some(job) = store.get(job_id).await {
        let p = priority.to_ascii_lowercase();
        store.update(job_id, |j| j.priority = p.clone()).await;
        info!("[scheduler.queue] 任务 '{}' 优先级 → {}", job.name, p);
        Ok(p)
    } else {
        warn!("[scheduler.queue] 任务 '{}' 不存在，无法重排序", job_id);
        Err(format!("任务不存在: {}", job_id))
    }
}

/// 工具函数：将字符串优先级归一化为合法枚举，供调度方校验。
///
/// 不合法时回退为默认 Medium（与 CronJob serde 默认一致），保证不 panic。
pub fn parse_priority(p: &str) -> CronJobPriority {
    match p.to_ascii_lowercase().as_str() {
        "low" => CronJobPriority::Low,
        "medium" => CronJobPriority::Medium,
        "high" => CronJobPriority::High,
        "batch" => CronJobPriority::Batch,
        _ => CronJobPriority::Medium,
    }
}
