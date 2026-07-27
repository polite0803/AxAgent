// SPDX-License-Identifier: AGPL-3.0-only
//! G14 DojoSDK 工具集 — Tauri 命令层 + DojoSdkExecutor 实现
//!
//! 对接 astock-data 的 `DojoSdkExecutor` trait，将 6 个 DojoSDK 工具路由到
//! quant / stock-analysis / tools 等具体 crate。
//!
//! ## 工具清单
//!
//! - `sector_precomputed_sector_alpha_factors_daily` — 行业 alpha 因子日频数据
//! - `dojo_run_quant_backtest` — 量化策略回测（封装 quant crate）
//! - `dojo_get_skill_content` — 获取 SKILL 内容（走 SkillPromptCache）
//! - `dojo_list_skills` — 列出所有可用 SKILL
//! - `dojo_get_paper_portfolio` — 获取模拟观察组合详情
//! - `dojo_list_market_mainlines` — 列出最近 N 天市场主线
//!
//! ## 启动注册
//!
//! 在 `init::services` 中调用 `register_dojo_sdk_executor(Box::new(DojoSdkExecutorImpl::new(state)))`
//! 完成注册。注册后所有 MCP 工具调用 `dojo_*` / `sector_precomputed_*` 都会路由到这里。

use crate::AppState;
use axagent_astock_data::mcp_tools::DojoSdkExecutor;
use axagent_harness::plan_types::{
    Phase, PhaseStatus, PlannedTask, ReplanAction, ReplanReason, TaskStatus,
};
use axagent_harness::strategy_contract::Bar;
use axagent_quant::{
    BacktestConfig, BacktestEngine, BollStrategy, MaCrossStrategy, MacdStrategy, MatcherConfig,
    RsiStrategy, TurtleStrategy,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tauri::State;
use tokio::sync::Mutex;

/// DojoSdkExecutor 的实现
///
/// 持有 astock_client 与可选 db 连接，路由 6 个 DojoSDK 工具到具体 crate。
/// db 通过 `set_db` 在启动后期注入（init::services 完成数据库初始化后）。
pub struct DojoSdkExecutorImpl {
    /// 同步 RwLock，避免 set_db 需要 async（注册阶段尚未进入 runtime）
    db: std::sync::RwLock<Option<sea_orm::DatabaseConnection>>,
    astock_client: Arc<axagent_astock_data::AStockClient>,
}

impl DojoSdkExecutorImpl {
    /// 创建新的执行器实例
    pub fn new(astock_client: Arc<axagent_astock_data::AStockClient>) -> Self {
        Self { db: std::sync::RwLock::new(None), astock_client }
    }

    /// 在启动后注入数据库连接（同步，init::services 中调用）
    ///
    /// 注意：使用 std::sync::RwLock 而非 tokio::sync::RwLock，因为：
    /// 1. set_db 只在启动时调用一次，不会跨 await
    /// 2. get_db 内部不会长时间持有锁（仅 clone 后立即释放）
    /// 3. 简化注册流程，避免 async 复杂性
    pub fn set_db(&self, db: sea_orm::DatabaseConnection) {
        let mut guard = self.db.write().expect("db RwLock poisoned");
        *guard = Some(db);
    }

    /// 获取数据库连接（如果已注入）
    fn get_db(&self) -> Option<sea_orm::DatabaseConnection> {
        self.db.read().ok()?.clone()
    }

    /// 运行量化策略回测
    async fn execute_quant_backtest(&self, arguments: &Value) -> Result<String, String> {
        let stock_code = arguments["stock_code"]
            .as_str()
            .ok_or_else(|| "stock_code 参数缺失".to_string())?
            .to_string();
        let strategy_name =
            arguments["strategy"].as_str().ok_or_else(|| "strategy 参数缺失".to_string())?;
        let start_date = arguments["start_date"]
            .as_str()
            .ok_or_else(|| "start_date 参数缺失".to_string())?
            .to_string();
        let end_date = arguments["end_date"]
            .as_str()
            .ok_or_else(|| "end_date 参数缺失".to_string())?
            .to_string();
        let initial_capital = arguments["initial_capital"].as_f64().unwrap_or(100_000.0);

        // 拉取 K 线（前复权）
        let klines = self
            .astock_client
            .get_klines(&stock_code, "daily", 500)
            .await
            .map_err(|e| format!("拉取 K 线失败: {e}"))?;

        if klines.is_empty() {
            return Err(format!("股票 {stock_code} 无 K 线数据"));
        }

        // 转换为 quant::Bar（Bar 类型已下沉到 harness::strategy_contract）
        let bars: Vec<Bar> = klines
            .iter()
            .filter(|k| {
                k.date.as_str() >= start_date.as_str() && k.date.as_str() <= end_date.as_str()
            })
            .map(|k| Bar::from_kline(stock_code.clone(), k))
            .collect();

        if bars.is_empty() {
            return Err(format!(
                "在 {start_date} ~ {end_date} 范围内无 K 线数据（共 {} 根，需检查日期范围）",
                klines.len()
            ));
        }

        // 构造回测配置
        let config = BacktestConfig {
            initial_cash: initial_capital,
            matcher: MatcherConfig::default(),
            start_date: Some(start_date.clone()),
            end_date: Some(end_date.clone()),
            codes: vec![stock_code.clone()],
        };

        let engine = BacktestEngine::new(config);

        // 根据策略名构造策略实例
        let result = match strategy_name {
            "ma_cross" => {
                let fast = arguments["params"]["fast"].as_u64().unwrap_or(5) as usize;
                let slow = arguments["params"]["slow"].as_u64().unwrap_or(20) as usize;
                let mut strategy = MaCrossStrategy::new(fast, slow);
                engine.run(&mut strategy, bars).await
            },
            "macd" => {
                let fast = arguments["params"]["fast"].as_u64().unwrap_or(12) as usize;
                let slow = arguments["params"]["slow"].as_u64().unwrap_or(26) as usize;
                let signal = arguments["params"]["signal"].as_u64().unwrap_or(9) as usize;
                let mut strategy = MacdStrategy::new(fast, slow, signal);
                engine.run(&mut strategy, bars).await
            },
            "rsi" => {
                let period = arguments["params"]["period"].as_u64().unwrap_or(14) as usize;
                let overbought = arguments["params"]["overbought"].as_f64().unwrap_or(70.0);
                let oversold = arguments["params"]["oversold"].as_f64().unwrap_or(30.0);
                let mut strategy = RsiStrategy::new(period, overbought, oversold).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
                engine.run(&mut strategy, bars).await
            },
            "boll" => {
                let period = arguments["params"]["period"].as_u64().unwrap_or(20) as usize;
                let stddev = arguments["params"]["stddev"].as_f64().unwrap_or(2.0);
                let mut strategy = BollStrategy::new(period, stddev);
                engine.run(&mut strategy, bars).await
            },
            "turtle" => {
                let entry = arguments["params"]["entry"].as_u64().unwrap_or(20) as usize;
                let exit = arguments["params"]["exit"].as_u64().unwrap_or(10) as usize;
                let atr_period = arguments["params"]["atr_period"].as_u64().unwrap_or(20) as usize;
                let atr_multiplier = arguments["params"]["atr_multiplier"].as_f64().unwrap_or(2.0);
                let mut strategy = TurtleStrategy::new(entry, exit, atr_period, atr_multiplier);
                engine.run(&mut strategy, bars).await
            },
            _ => return Err(format!("未知策略: {strategy_name}")),
        };

        match result {
            Ok(bt) => {
                let response = json!({
                    "status": "ok",
                    "stock_code": stock_code,
                    "strategy": strategy_name,
                    "start_date": start_date,
                    "end_date": end_date,
                    "initial_capital": initial_capital,
                    "final_equity": bt.final_equity,
                    "total_return_pct": bt.total_return * 100.0,
                    "annualized_return_pct": bt.annualized_return * 100.0,
                    "sharpe": bt.sharpe,
                    "max_drawdown_pct": bt.max_drawdown_pct,
                    "win_rate": bt.win_rate,
                    "total_trades": bt.total_trades,
                    "winning_trades": bt.winning_trades,
                    "losing_trades": bt.losing_trades,
                    "duration_ms": bt.duration_ms,
                    "trade_count": bt.trades.len(),
                    "equity_curve_points": bt.equity_curve.len(),
                });
                serde_json::to_string(&response).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            Err(e) => Err(format!("回测失败: {e}")),
        }
    }

    /// 获取模拟观察组合详情
    async fn execute_get_paper_portfolio(&self, arguments: &Value) -> Result<String, String> {
        let portfolio_id = arguments["portfolio_id"]
            .as_str()
            .ok_or_else(|| "portfolio_id 参数缺失".to_string())?
            .to_string();

        let db = self.get_db().ok_or_else(|| "数据库未初始化".to_string())?;

        let detail = axagent_stock_analysis::paper_portfolio::get_portfolio_detail(
            &db,
            &*self.astock_client,
            &portfolio_id,
        )
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

        match detail {
            Some(d) => {
                let response = json!({
                    "status": "ok",
                    "portfolio": d.portfolio,
                    "positions": d.positions,
                    "summary": d.summary,
                });
                serde_json::to_string(&response).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            None => Err(format!("组合 {portfolio_id} 不存在")),
        }
    }

    /// 列出最近 N 天市场主线
    async fn execute_list_market_mainlines(&self, arguments: &Value) -> Result<String, String> {
        let days = arguments["days"].as_u64().unwrap_or(7) as usize;
        let category = arguments["category"].as_str().map(String::from);

        let db = self.get_db().ok_or_else(|| "数据库未初始化".to_string())?;

        let mainlines = if let Some(cat) = &category {
            axagent_stock_analysis::market_mainline::list_mainlines_by_category(&db, cat)
                .await
                .map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?
        } else {
            axagent_stock_analysis::market_mainline::list_recent_mainlines(&db, days)
                .await
                .map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?
        };

        let response = json!({
            "status": "ok",
            "count": mainlines.len(),
            "days": days,
            "category": category,
            "mainlines": mainlines,
        });
        serde_json::to_string(&response).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
    }
}

// ── G19 Plan 三件套工具实现 ──────────────────────────────────────────────
//
// 复用 axagent_agent::HierarchicalPlanner，通过全局 PLANS_REGISTRY 管理 plan_id → planner。
// 三件套：
// - dojo_create_plan: 创建分层执行计划
// - dojo_execute_plan: 控制执行（start/pause/resume/cancel/progress/next_tasks/complete_task/fail_task）
// - dojo_revise_plan: 重规划（Retry/Skip/Insert/Remove/Reorder/AddPhase/ModifyTask + Rollback）
//
// P2-9: 引入 TTL 清理机制，避免长期运行导致 PLANS_REGISTRY 内存膨胀。
// - 每个 PlanEntry 记录 last_accessed_at（create/execute/revise 时更新）
// - 默认 TTL = 24 小时，超期未访问的计划会被 cleanup_expired_plans 删除
// - 在 init::services 中通过 tokio::spawn 启动后台定时清理任务

/// 默认 TTL：24 小时（单位：秒）
pub const PLAN_DEFAULT_TTL_SECS: u64 = 24 * 60 * 60;

/// 清理任务执行间隔：1 小时
pub const PLAN_CLEANUP_INTERVAL_SECS: u64 = 60 * 60;

/// Plan Registry 表项：planner + 最后访问时间
struct PlanEntry {
    planner: Arc<Mutex<axagent_agent::HierarchicalPlanner>>,
    last_accessed_at: std::time::Instant,
}

/// 全局 Plan Registry：plan_id → PlanEntry
#[allow(clippy::type_complexity)]
static PLANS_REGISTRY: LazyLock<Arc<Mutex<HashMap<String, PlanEntry>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

/// 清理过期的 plan（由后台定时任务调用）
///
/// 删除 `last_accessed_at` 距今超过 `ttl_secs` 的所有 plan。
/// 返回被清理的 plan_id 列表。
pub async fn cleanup_expired_plans(ttl_secs: u64) -> Vec<String> {
    let ttl = std::time::Duration::from_secs(ttl_secs);
    let now = std::time::Instant::now();
    let mut registry = PLANS_REGISTRY.lock().await;
    let expired_ids: Vec<String> = registry
        .iter()
        .filter_map(|(id, entry)| {
            if now.duration_since(entry.last_accessed_at) > ttl {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();
    for id in &expired_ids {
        registry.remove(id);
    }
    if !expired_ids.is_empty() {
        tracing::info!(expired_count = expired_ids.len(), ttl_secs, "[G19 TTL] 清理过期 plan");
    }
    expired_ids
}

/// 启动 plan TTL 清理后台任务（在 init::services 中调用一次）
///
/// 接受 shutdown_token 以便应用关闭时优雅退出，与项目其它后台任务保持一致。
///
/// 使用 `tauri::async_runtime::spawn` 而非 `tokio::spawn`，因为本函数在
/// `start_background_services`（setup hook 同步上下文）中调用，此时可能没有
/// tokio runtime。`tauri::async_runtime::spawn` 会自动选择合适的运行时。
pub fn spawn_plan_ttl_cleanup(shutdown_token: tokio_util::sync::CancellationToken) {
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(PLAN_CLEANUP_INTERVAL_SECS);
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("[G19 TTL] 收到关闭信号，停止 plan TTL 清理任务");
                    break;
                },
                _ = tokio::time::sleep(interval) => {
                    // 静默清理，失败仅记录日志
                    let expired = cleanup_expired_plans(PLAN_DEFAULT_TTL_SECS).await;
                    if !expired.is_empty() {
                        tracing::debug!(
                            expired_ids = ?expired,
                            "[G19 TTL] 后台清理任务完成"
                        );
                    }
                },
            }
        }
    });
}

/// 生成 plan_id（短 UUID）
fn generate_plan_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string().split('-').next().unwrap_or("plan").to_string()
}

/// 从 JSON 构造 Phase（自动生成 ID）
fn phase_from_json(phase_json: &Value, phase_idx: usize) -> Result<Phase, String> {
    let name =
        phase_json["name"].as_str().ok_or_else(|| "phase.name 缺失".to_string())?.to_string();
    let description = phase_json["description"].as_str().unwrap_or("").to_string();
    let dependencies: Vec<String> = phase_json["dependencies"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    // 兼容整数索引（1=第一Phase）与字符串 ID
                    if let Some(n) = v.as_u64() {
                        Some(format!("phase_{}", n))
                    } else {
                        v.as_str().map(String::from)
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let phase_id = format!("phase_{}", phase_idx + 1);
    let tasks_json =
        phase_json["tasks"].as_array().ok_or_else(|| format!("phase '{name}' 缺少 tasks 数组"))?;

    let mut tasks = Vec::with_capacity(tasks_json.len());
    for (ti, tj) in tasks_json.iter().enumerate() {
        let task = task_from_json(tj, ti)?;
        tasks.push(task);
    }

    Ok(Phase { id: phase_id, name, description, tasks, dependencies, status: PhaseStatus::Pending })
}

/// 从 JSON 构造 PlannedTask（自动生成 ID）
fn task_from_json(task_json: &Value, task_idx: usize) -> Result<PlannedTask, String> {
    let description = task_json["description"]
        .as_str()
        .ok_or_else(|| "task.description 缺失".to_string())?
        .to_string();
    let action_type = task_json["action_type"].as_str().unwrap_or("agent").to_string();
    let parameters = task_json["parameters"].clone();
    let dependencies: Vec<String> = task_json["dependencies"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    if let Some(n) = v.as_u64() {
                        Some(format!("task_{}", n))
                    } else {
                        v.as_str().map(String::from)
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let max_retries = task_json["max_retries"].as_u64().unwrap_or(3) as u32;
    let assigned_role = task_json["assigned_role"].as_str().map(String::from);

    Ok(PlannedTask {
        id: format!("task_{}", task_idx + 1),
        description,
        action_type,
        parameters,
        dependencies,
        status: TaskStatus::Pending,
        result: None,
        error: None,
        retry_count: 0,
        max_retries,
        assigned_role,
        compensation: None,
    })
}

/// 创建分层执行计划
async fn execute_create_plan(arguments: &Value) -> Result<String, String> {
    let goal = arguments["goal"].as_str().ok_or_else(|| "goal 参数缺失".to_string())?.to_string();
    let phases_json =
        arguments["phases"].as_array().ok_or_else(|| "phases 参数缺失或非数组".to_string())?;

    if phases_json.is_empty() {
        return Err("phases 不能为空".to_string());
    }

    let mut phases = Vec::with_capacity(phases_json.len());
    for (i, pj) in phases_json.iter().enumerate() {
        phases.push(phase_from_json(pj, i)?);
    }

    let mut planner = axagent_agent::HierarchicalPlanner::new();
    let plan_ref = planner.create_plan(&goal, phases);
    let plan_id = generate_plan_id();
    let plan_snapshot = plan_ref.clone();

    let planner_arc = Arc::new(Mutex::new(planner));
    PLANS_REGISTRY.lock().await.insert(
        plan_id.clone(),
        PlanEntry { planner: planner_arc, last_accessed_at: std::time::Instant::now() },
    );

    let response = json!({
        "status": "ok",
        "plan_id": plan_id,
        "plan": plan_snapshot,
        "message": "计划已创建。调用 dojo_execute_plan(action='start') 开始执行。",
    });
    serde_json::to_string(&response).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 执行计划控制
async fn execute_execute_plan(arguments: &Value) -> Result<String, String> {
    let plan_id =
        arguments["plan_id"].as_str().ok_or_else(|| "plan_id 参数缺失".to_string())?.to_string();
    let action = arguments["action"].as_str().unwrap_or("start").to_string();

    // P2-9: 在持锁期间一并更新 last_accessed_at，避免双重锁
    let planner_arc = {
        let mut registry = PLANS_REGISTRY.lock().await;
        let entry = registry
            .get_mut(&plan_id)
            .ok_or_else(|| format!("计划 {plan_id} 不存在（可能已过期或未创建）"))?;
        entry.last_accessed_at = std::time::Instant::now();
        entry.planner.clone()
    };

    let mut planner = planner_arc.lock().await;

    let response = match action.as_str() {
        "start" => {
            planner.start_execution()?;
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            let next_tasks: Vec<&PlannedTask> = planner.get_next_executable_tasks();
            json!({
                "status": "ok",
                "action": "start",
                "plan_status": plan.status,
                "next_tasks": next_tasks.iter().map(|t| task_brief(t)).collect::<Vec<_>>(),
            })
        },
        "pause" => {
            planner.pause_execution()?;
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            json!({
                "status": "ok",
                "action": "pause",
                "plan_status": plan.status,
            })
        },
        "resume" => {
            planner.resume_execution()?;
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            let next_tasks: Vec<&PlannedTask> = planner.get_next_executable_tasks();
            json!({
                "status": "ok",
                "action": "resume",
                "plan_status": plan.status,
                "next_tasks": next_tasks.iter().map(|t| task_brief(t)).collect::<Vec<_>>(),
            })
        },
        "cancel" => {
            planner.cancel_execution()?;
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            json!({
                "status": "ok",
                "action": "cancel",
                "plan_status": plan.status,
            })
        },
        "progress" => {
            let progress = planner.get_progress();
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            json!({
                "status": "ok",
                "action": "progress",
                "plan_status": plan.status,
                "progress": progress,
            })
        },
        "next_tasks" => {
            let next_tasks: Vec<&PlannedTask> = planner.get_next_executable_tasks();
            json!({
                "status": "ok",
                "action": "next_tasks",
                "next_tasks": next_tasks.iter().map(|t| task_brief(t)).collect::<Vec<_>>(),
            })
        },
        "complete_task" => {
            let task_id = arguments["task_id"]
                .as_str()
                .ok_or_else(|| "complete_task 需要 task_id 参数".to_string())?
                .to_string();
            let result =
                arguments.get("result").cloned().unwrap_or_else(|| json!({"status": "done"}));
            planner.mark_task_started(&task_id)?;
            planner.mark_task_completed(&task_id, result)?;
            let progress = planner.get_progress();
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            let next_tasks: Vec<&PlannedTask> = planner.get_next_executable_tasks();
            json!({
                "status": "ok",
                "action": "complete_task",
                "task_id": task_id,
                "plan_status": plan.status,
                "progress": progress,
                "next_tasks": next_tasks.iter().map(|t| task_brief(t)).collect::<Vec<_>>(),
            })
        },
        "fail_task" => {
            let task_id = arguments["task_id"]
                .as_str()
                .ok_or_else(|| "fail_task 需要 task_id 参数".to_string())?
                .to_string();
            let error = arguments["error"].as_str().unwrap_or("未知错误").to_string();
            // 若任务未启动则先标记 started（HierarchicalPlanner 要求）
            let _ = planner.mark_task_started(&task_id);
            planner.mark_task_failed(&task_id, &error)?;
            let progress = planner.get_progress();
            let plan = planner.get_plan().ok_or("计划不存在")?.clone();
            json!({
                "status": "ok",
                "action": "fail_task",
                "task_id": task_id,
                "error": error,
                "plan_status": plan.status,
                "progress": progress,
            })
        },
        other => {
            return Err(format!(
                "未知 action: {other}（支持 start/pause/resume/cancel/progress/next_tasks/complete_task/fail_task）"
            ));
        },
    };

    serde_json::to_string(&response).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 重规划（修订计划）
async fn execute_revise_plan(arguments: &Value) -> Result<String, String> {
    let plan_id =
        arguments["plan_id"].as_str().ok_or_else(|| "plan_id 参数缺失".to_string())?.to_string();

    // P2-9: 在持锁期间一并更新 last_accessed_at，避免双重锁
    let planner_arc = {
        let mut registry = PLANS_REGISTRY.lock().await;
        let entry = registry.get_mut(&plan_id).ok_or_else(|| format!("计划 {plan_id} 不存在"))?;
        entry.last_accessed_at = std::time::Instant::now();
        entry.planner.clone()
    };

    let mut planner = planner_arc.lock().await;

    // 回滚模式
    if let Some(version) = arguments["rollback_to_version"].as_u64() {
        let restored_plan = planner.rollback(version as u32)?;
        let response = json!({
            "status": "ok",
            "action": "rollback",
            "rollback_to_version": version,
            "plan": restored_plan,
        });
        return serde_json::to_string(&response).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        });
    }

    // 重规划模式
    let reason_str = arguments["reason"].as_str().ok_or_else(|| "reason 参数缺失".to_string())?;
    let actions_json =
        arguments["actions"].as_array().ok_or_else(|| "actions 参数缺失或非数组".to_string())?;

    let reason = parse_replan_reason(reason_str, arguments)?;
    let actions = parse_replan_actions(actions_json)?;

    let record = planner.replan(reason, actions)?;
    let plan = planner.get_plan().ok_or("计划不存在")?.clone();
    let progress = planner.get_progress();

    let response = json!({
        "status": "ok",
        "action": "revise",
        "plan": plan,
        "progress": progress,
        "record": record,
        "version_history_count": planner.get_plan_versions().len(),
    });
    serde_json::to_string(&response).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 解析 ReplanReason
fn parse_replan_reason(reason_str: &str, args: &Value) -> Result<ReplanReason, String> {
    match reason_str {
        "StepFailed" => {
            let task_id = args["task_id"]
                .as_str()
                .or_else(|| args["details"]["task_id"].as_str())
                .ok_or_else(|| "StepFailed 需要 task_id 参数".to_string())?
                .to_string();
            let error = args["error"]
                .as_str()
                .or_else(|| args["details"]["error"].as_str())
                .unwrap_or("unknown error")
                .to_string();
            Ok(ReplanReason::StepFailed { task_id, error })
        },
        "NewDependencyDiscovered" => {
            let task_id = args["task_id"]
                .as_str()
                .or_else(|| args["details"]["task_id"].as_str())
                .ok_or_else(|| "NewDependencyDiscovered 需要 task_id 参数".to_string())?
                .to_string();
            let dependency = args["dependency"]
                .as_str()
                .or_else(|| args["details"]["dependency"].as_str())
                .ok_or_else(|| "NewDependencyDiscovered 需要 dependency 参数".to_string())?
                .to_string();
            Ok(ReplanReason::NewDependencyDiscovered { task_id, dependency })
        },
        "GoalChanged" => {
            let old_goal = args["old_goal"]
                .as_str()
                .or_else(|| args["details"]["old_goal"].as_str())
                .unwrap_or("")
                .to_string();
            let new_goal = args["new_goal"]
                .as_str()
                .or_else(|| args["details"]["new_goal"].as_str())
                .unwrap_or("")
                .to_string();
            Ok(ReplanReason::GoalChanged { old_goal, new_goal })
        },
        "ResourceConstraint" => {
            let constraint = args["constraint"]
                .as_str()
                .or_else(|| args["details"]["constraint"].as_str())
                .unwrap_or("unspecified constraint")
                .to_string();
            Ok(ReplanReason::ResourceConstraint { constraint })
        },
        "ManualIntervention" => {
            let reason = args["detail"]
                .as_str()
                .or_else(|| args["details"]["reason"].as_str())
                .unwrap_or("manual intervention")
                .to_string();
            Ok(ReplanReason::ManualIntervention { reason })
        },
        other => Err(format!(
            "未知 reason: {other}（支持 StepFailed/NewDependencyDiscovered/GoalChanged/ResourceConstraint/ManualIntervention）"
        )),
    }
}

/// 解析 ReplanAction 数组
fn parse_replan_actions(actions_json: &[Value]) -> Result<Vec<ReplanAction>, String> {
    let mut actions = Vec::with_capacity(actions_json.len());
    for aj in actions_json {
        let action_type = aj["type"].as_str().ok_or_else(|| "action.type 缺失".to_string())?;
        let action = match action_type {
            "Retry" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "Retry 需要 task_id".to_string())?
                    .to_string();
                let modified_parameters = aj.get("modified_parameters").cloned();
                ReplanAction::Retry { task_id, modified_parameters }
            },
            "Skip" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "Skip 需要 task_id".to_string())?
                    .to_string();
                let reason = aj["reason"].as_str().unwrap_or("").to_string();
                ReplanAction::Skip { task_id, reason }
            },
            "Insert" => {
                let phase_id = aj["phase_id"]
                    .as_str()
                    .ok_or_else(|| "Insert 需要 phase_id".to_string())?
                    .to_string();
                let task_json =
                    aj.get("task").ok_or_else(|| "Insert 需要 task 定义".to_string())?;
                let task = task_from_json(task_json, 0)?;
                let position = aj["position"].as_u64().unwrap_or(0) as usize;
                ReplanAction::Insert { phase_id, task, position }
            },
            "Remove" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "Remove 需要 task_id".to_string())?
                    .to_string();
                let reason = aj["reason"].as_str().unwrap_or("").to_string();
                ReplanAction::Remove { task_id, reason }
            },
            "Reorder" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "Reorder 需要 task_id".to_string())?
                    .to_string();
                let new_position =
                    aj["new_position"].as_u64().or_else(|| aj["position"].as_u64()).unwrap_or(0)
                        as usize;
                ReplanAction::Reorder { task_id, new_position }
            },
            "AddPhase" => {
                let phase_json =
                    aj.get("phase").ok_or_else(|| "AddPhase 需要 phase 定义".to_string())?;
                let phase = phase_from_json(phase_json, 0)?;
                let position = aj["position"].as_u64().unwrap_or(0) as usize;
                ReplanAction::AddPhase { phase, position }
            },
            "ModifyTask" => {
                let task_id = aj["task_id"]
                    .as_str()
                    .ok_or_else(|| "ModifyTask 需要 task_id".to_string())?
                    .to_string();
                let modifications = aj["modifications"].clone();
                ReplanAction::ModifyTask { task_id, modifications }
            },
            other => return Err(format!("未知 action type: {other}")),
        };
        actions.push(action);
    }
    Ok(actions)
}

/// 任务简略信息（用于响应返回）
fn task_brief(t: &PlannedTask) -> Value {
    json!({
        "id": t.id,
        "description": t.description,
        "action_type": t.action_type,
        "status": format!("{:?}", t.status),
        "assigned_role": t.assigned_role,
        "dependencies": t.dependencies,
        "retry_count": t.retry_count,
        "max_retries": t.max_retries,
    })
}

#[async_trait::async_trait]
impl DojoSdkExecutor for DojoSdkExecutorImpl {
    async fn execute(&self, tool_name: &str, arguments: &Value) -> Result<String, String> {
        tracing::info!("[DojoSDK] 执行工具: {tool_name}, args: {arguments}");
        match tool_name {
            "sector_precomputed_sector_alpha_factors_daily" => {
                execute_sector_alpha_factors(arguments).await
            },
            "dojo_run_quant_backtest" => self.execute_quant_backtest(arguments).await,
            "dojo_get_skill_content" => execute_get_skill_content(arguments),
            "dojo_list_skills" => execute_list_skills(arguments),
            "dojo_get_paper_portfolio" => self.execute_get_paper_portfolio(arguments).await,
            "dojo_list_market_mainlines" => self.execute_list_market_mainlines(arguments).await,
            // G19 Plan 三件套
            "dojo_create_plan" => execute_create_plan(arguments).await,
            "dojo_execute_plan" => execute_execute_plan(arguments).await,
            "dojo_revise_plan" => execute_revise_plan(arguments).await,
            _ => Err(format!("未知 DojoSDK 工具: {tool_name}")),
        }
    }
}

// ── 工具实现（独立函数） ─────────────────────────────────────────────────

/// 行业 alpha 因子日频数据（占位实现）
///
/// 真实场景下应调用专门的因子计算服务（如 quant crate 的因子库或外部数据源）。
/// 当前返回占位数据结构，包含 6 类标准因子，便于上层 LLM 理解数据格式。
async fn execute_sector_alpha_factors(arguments: &Value) -> Result<String, String> {
    let start_date = arguments["start_date"].as_str().unwrap_or("2026-06-01").to_string();
    let end_date = arguments["end_date"].as_str().unwrap_or("2026-07-26").to_string();
    let industry = arguments["industry"].as_str().unwrap_or("全部").to_string();

    let requested_factors: Vec<String> = arguments["factors"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_else(|| {
            vec![
                "size".into(),
                "value".into(),
                "momentum".into(),
                "reversal".into(),
                "volatility".into(),
                "liquidity".into(),
            ]
        });

    // 占位数据：返回标准格式，便于上层 LLM 消费
    let response = json!({
        "status": "ok",
        "data_source": "precomputed_sector_alpha_factors_daily",
        "query": {
            "start_date": start_date,
            "end_date": end_date,
            "industry": industry,
            "factors": requested_factors,
        },
        "note": "G14 占位实现：真实因子数据需接入因子计算服务。当前返回数据结构示例。",
        "industries": [
            {
                "industry": "银行",
                "factor_values": {
                    "size": 0.32,
                    "value": 0.18,
                    "momentum": -0.05,
                    "reversal": 0.02,
                    "volatility": 0.08,
                    "liquidity": 0.21
                }
            },
            {
                "industry": "半导体",
                "factor_values": {
                    "size": 0.45,
                    "value": -0.12,
                    "momentum": 0.38,
                    "reversal": -0.15,
                    "volatility": 0.42,
                    "liquidity": 0.35
                }
            }
        ]
    });
    serde_json::to_string(&response).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取 SKILL 内容
fn execute_get_skill_content(arguments: &Value) -> Result<String, String> {
    let skill_name =
        arguments["skill_name"].as_str().ok_or_else(|| "skill_name 参数缺失".to_string())?;

    match axagent_tools::tools::skill::SkillPromptCache::get_skill_prompt(skill_name) {
        Some(content) => {
            let response = json!({
                "status": "ok",
                "skill_name": skill_name,
                "content": content,
                "content_length": content.len(),
            });
            serde_json::to_string(&response).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        None => Err(format!("SKILL '{skill_name}' 不存在或未加载")),
    }
}

/// 列出所有可用 SKILL
fn execute_list_skills(arguments: &Value) -> Result<String, String> {
    let include_external = arguments["include_external"].as_bool().unwrap_or(true);

    let cached_skills = axagent_tools::tools::skill::SkillPromptCache::list_cached_skills();

    let skills: Vec<Value> = cached_skills
        .iter()
        .filter(|name| {
            if include_external {
                true
            } else {
                // 只保留 axagent 自己的 skills（排除 claude/trae/codebuddy 等外部目录）
                let self_source = axagent_kit::skill_dirs::self_source_kind();
                let self_dir = dirs::home_dir()
                    .map(|h| h.join(format!(".{self_source}")).join("skills"))
                    .unwrap_or_default();
                let skill_dir = self_dir.join(name);
                skill_dir.exists()
            }
        })
        .map(|name| {
            json!({
                "name": name,
                "source_kind": if include_external { "any" } else { axagent_kit::skill_dirs::self_source_kind() },
            })
        })
        .collect();

    let response = json!({
        "status": "ok",
        "count": skills.len(),
        "skills": skills,
        "include_external": include_external,
    });
    serde_json::to_string(&response).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── Tauri 命令层（前端可直接调用，不经过 MCP 协议） ───────────────────────

/// 执行 DojoSDK 工具（前端 IPC 入口）
#[tauri::command]
pub async fn dojo_sdk_execute_tool(
    state: State<'_, AppState>,
    tool_name: String,
    arguments: Value,
) -> Result<String, String> {
    if !axagent_astock_data::mcp_tools::is_dojo_sdk_tool(&tool_name) {
        return Err(format!("非 DojoSDK 工具: {tool_name}"));
    }

    // 直接调用 astock-data 的 execute_mcp_tool（它会路由到 DojoSdkExecutor）
    axagent_astock_data::mcp_tools::execute_mcp_tool(&state.astock_client, &tool_name, &arguments)
        .await
}

/// 列出所有 DojoSDK 工具的元数据
#[tauri::command]
pub async fn dojo_sdk_list_tools() -> Result<Vec<Value>, String> {
    let all_tools = axagent_astock_data::mcp_tools::stock_mcp_tools();
    let dojo_tools: Vec<Value> = all_tools
        .into_iter()
        .filter(|t| {
            t["name"]
                .as_str()
                .map(axagent_astock_data::mcp_tools::is_dojo_sdk_tool)
                .unwrap_or(false)
        })
        .collect();
    Ok(dojo_tools)
}

/// 检查 DojoSdkExecutor 是否已注册
#[tauri::command]
pub async fn dojo_sdk_is_ready() -> Result<bool, String> {
    Ok(axagent_astock_data::mcp_tools::has_dojo_sdk_executor())
}
