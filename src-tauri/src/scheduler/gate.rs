// SPDX-License-Identifier: AGPL-3.0-only

//! 动态择时执行（成本感知）+ 成本门控。
//!
//! 设计依据 `docs/夜间长时自主任务运行-详细设计.md` ⑧：
//! - **时段价格感知**：价格低时提前启动排队任务，价格高时让非紧急任务进入延迟/闲时窗口。
//! - **成本上限**：用户设 `max_budget`；累计超限 → 停止新任务（熔断）+ 通知管理员。
//! - 采用**分钟级二次采样**探价，不做实时常驻值守。
//!
//! 本模块是纯函数式协调层，只做判定，不做定时；探价频率复用 cron tick。

use axagent_runtime_core::cron_job::CronJobPriority;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ── 预算状态 ──────────────────────────────────

/// 内存中的预算额度（不落库，重启后由运维重新设置）。
/// 如需持久化，可后续迁移到 `app_config` 或独立 `budget_settings` 表。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetState {
    /// 成本上限（USD）。None = 不限。
    pub max_budget: Option<f64>,
    /// 已累计成本（USD）。
    pub spent: f64,
    /// 是否已熔断。
    pub tripped: bool,
}

impl BudgetState {
    /// 记录一笔新增成本并检查是否超限。
    /// - 未超限返回 `Ok(ShouldRun)`
    /// - 超限则熔断并返回 `Ok(Tripped)`
    /// - 成本非法返回 `Err`
    pub fn record_spend(&mut self, cost_usd: f64) -> Result<bool, String> {
        if cost_usd < 0.0 {
            return Err("成本不能为负".to_string());
        }
        self.spent += cost_usd;
        if let Some(max) = self.max_budget {
            if self.spent > max {
                self.tripped = true;
                info!("[scheduler.gate] 成本熔断：spent={:.4} > max={:.4}", self.spent, max);
                return Ok(true);
            }
        }
        Ok(self.tripped)
    }

    /// 是否允许启动新任务（未熔断）。
    pub fn can_run(&self) -> bool {
        !self.tripped
    }
}

// ── 择时判定 ──────────────────────────────────

/// 时段价格档位。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PriceLevel {
    /// 低价时段：适合启动排队任务 / 批量任务
    Low,
    /// 常规时段
    Normal,
    /// 高价时段：非紧急任务应延迟
    High,
}

/// 每分钟级的价格采样结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplePoint {
    pub ts_millis: i64,
    pub price_level: PriceLevel,
    /// 若超限则为 true（熔断信号）
    pub tripped: bool,
}

/// 判定当前时段价格档位。
///
/// 简化模型：以当前小时判断（0-6 为低价闲时，22-24 为低价，白天高峰为高价）。
/// 该阈值可后续下沉为配置，这里先满足「低则提前启动、高则延迟」的核心诉求。
pub fn price_level(hour_local: u8) -> PriceLevel {
    match hour_local {
        0..=5 => PriceLevel::Low,     // 午夜低价
        6..=11 => PriceLevel::Normal, // 上午
        12..=21 => PriceLevel::High,  // 白天高峰/夜间提前
        22..=23 => PriceLevel::Low,   // 深夜低价
        _ => PriceLevel::Normal,
    }
}

/// 综合门控判定：成本 + 择时。
///
/// 返回决策：
/// - `None` = 当前不应启动（熔断或高价时段且非紧急）。
/// - `Some(())` = 可以启动。
pub fn should_run(level: PriceLevel, job_priority: CronJobPriority, budget: &BudgetState) -> bool {
    if !budget.can_run() {
        return false;
    }
    // 高价时段仅放行紧急（high）任务；批量任务高峰一律不取。
    if level == PriceLevel::High {
        return job_priority == CronJobPriority::High;
    }
    true
}

// ── Tauri 命令辅助实现（budget 状态注入由调用方持有） ──────────

/// 设置成本上限（USD）。None 表示不限。
pub fn set_budget_impl(
    budget: &mut BudgetState,
    max_budget: Option<f64>,
) -> Result<BudgetState, String> {
    if max_budget.is_some_and(|m| m < 0.0) {
        return Err("max_budget 不能为负".to_string());
    }
    budget.max_budget = max_budget;
    budget.tripped = false; // 重设上限后解除熔断，允许重新评估
    info!("[scheduler.gate] 成本上限 → {:?} USD", budget.max_budget);
    Ok(budget.clone())
}

/// 查询预算用量。
pub fn get_budget_usage_impl(budget: &BudgetState) -> BudgetState {
    budget.clone()
}

/// 供 future：将一次探价写入 cron_job_history 成本段（当前仅打日志占位）。
/// 预留接口，未来可把成本落库。
#[allow(dead_code)]
pub async fn record_sample_to_db(
    _db: &DatabaseConnection,
    _task: &axagent_runtime_core::CronJob,
    sample: &SamplePoint,
) -> Result<(), String> {
    warn!("[scheduler.gate] (占位) 探价落库未启用: {:?}", sample.price_level);
    Ok(())
}
