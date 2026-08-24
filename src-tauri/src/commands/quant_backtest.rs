// SPDX-License-Identifier: AGPL-3.0-only

//! 量化回测命令 — 封装 `axagent-quant` 回测引擎的 Tauri IPC 接口。
//!
//! 提供两个命令：
//! - `quant_backtest_run`：加载策略 → 拉取 K 线 → 跑回测引擎 → 落库 → 返回结果
//! - `quant_run_get`：按 runId 读取回测运行记录（含 `resultJson`）
//!
//! ## 数据流
//! ```text
//! 前端 BacktestRunRequest
//!   → 加载 quant_strategies（取 script_source / strategy_type）
//!   → AStockClient.get_klines（拉日 K 线）
//!   → BacktestEngine.run（quant crate）
//!   → MetricsReport::from_backtest_result（绩效）
//!   → [可选] WalkForward::run（反过拟合）
//!   → 写 quant_runs 表（result_json = BacktestResult 序列化）
//!   → 返回 BacktestRunResponse
//! ```

use std::collections::HashMap;

use axagent_astock_data::{AStockClient, types::AdjType};
use axagent_entities::{quant_runs, quant_strategies};
use axagent_harness::market_data::KLine;
use axagent_quant::{
    BacktestConfig, BacktestEngine, BacktestResult, Bar, BollStrategy, MaCrossStrategy,
    MacdStrategy, MatcherConfig as QuantMatcherConfig, MetricsReport, RhaiStrategy, RsiStrategy,
    Strategy, TurtleStrategy, WalkForward, WalkForwardConfig,
    WalkForwardReport as RawWalkForwardReport,
};
use chrono::{NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use uuid::Uuid;

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent_macro::agent_command;

/// 回测运行请求（对齐前端 `BacktestRunRequest`）
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestRunRequest {
    /// 关联 quant_strategies.id
    pub strategy_id: String,
    /// 冗余字段（命令以 DB 记录为准），保留以便对齐前端契约
    pub strategy_type: String,
    /// 股票代码
    pub code: String,
    /// 起始日期（YYYY-MM-DD）
    pub start_date: String,
    /// 截止日期（YYYY-MM-DD）
    pub end_date: String,
    /// 初始资金（元）
    pub initial_cash: f64,
    /// 策略参数（覆盖 DB 默认值）
    pub params: HashMap<String, Value>,
    /// 是否启用 Walk-Forward
    pub walk_forward_enabled: bool,
    /// 显式关闭 Walk-Forward（即便 enabled）
    pub walk_forward_force_off: bool,
    /// 撮合器配置（None 用默认 A 股规则）
    pub matcher_config: Option<MatcherConfigRequest>,
    /// 运行名称
    pub name: Option<String>,
}

/// 撮合器配置请求（camelCase，对齐前端）
///
/// Rust 侧 `axagent_quant::MatcherConfig` 字段为 snake_case 且无 rename，
/// 故命令层用此结构接收前端 JSON，再映射到引擎配置。
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatcherConfigRequest {
    pub commission_rate: f64,
    pub commission_min: f64,
    pub stamp_tax_rate: f64,
    pub slippage_rate: f64,
    pub lot_size: u64,
    pub t1_enforced: bool,
    pub limit_check: bool,
}

impl From<MatcherConfigRequest> for QuantMatcherConfig {
    fn from(m: MatcherConfigRequest) -> Self {
        Self {
            commission_rate: m.commission_rate,
            commission_min: m.commission_min,
            stamp_tax_rate: m.stamp_tax_rate,
            slippage_rate: m.slippage_rate,
            lot_size: m.lot_size,
            t1_enforced: m.t1_enforced,
            limit_check: m.limit_check,
        }
    }
}

/// 回测运行响应（对齐前端 `BacktestRunResponse`）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestRunResponse {
    /// 运行记录（quant_runs 实体，camelCase 序列化）
    pub run: quant_runs::Model,
    /// 完整绩效报告
    pub metrics: MetricsReport,
    /// 信号数
    pub signal_count: usize,
    /// 成交笔数
    pub trade_count: usize,
    /// Walk-Forward 报告（未启用为 null）
    pub walk_forward: Option<WalkForwardReportResponse>,
}

/// Walk-Forward 响应（对齐前端 `WalkForwardReport`）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardReportResponse {
    pub folds: Vec<WalkForwardFoldResponse>,
    /// 样本外权益序列
    pub oos_equity: Vec<f64>,
    pub stability_score: f64,
    pub overfit_window_count: usize,
    pub aggregated_test_sharpe: f64,
}

/// 单 fold 响应（对齐前端 `WalkForwardFold`）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardFoldResponse {
    pub train_bars_count: usize,
    pub test_bars_count: usize,
    pub fold_index: usize,
    pub train_start: String,
    pub train_end: String,
    pub test_start: String,
    pub test_end: String,
    pub train_sharpe: f64,
    pub test_sharpe: f64,
    pub best_params: Option<Value>,
}

/// 运行回测
///
/// 流程：插入 pending → 加载策略 → 拉 K 线 → 跑引擎 → 落库（completed/failed）
#[agent_command(domain = quant, safety = Caution, call_mode = StateInput, description = "执行量化回测")]
#[tauri::command]
pub async fn quant_backtest_run(
    state: State<'_, AppState>,
    request: BacktestRunRequest,
) -> Result<BacktestRunResponse, String> {
    let db = state.harness.db();
    let started_at = Utc::now().timestamp();

    // 1. 加载策略定义（以 DB 为准）
    let strategy_model = quant_strategies::Entity::find_by_id(&request.strategy_id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询策略失败: {e}"))
        })?
        .ok_or_else(|| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("策略不存在: {}", request.strategy_id))
        })?;

    // 2. 插入 pending 记录
    let config_json = serde_json::to_string(&request).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("序列化配置失败: {e}"))
    })?;
    let run_id = Uuid::new_v4().to_string();
    let pending = quant_runs::ActiveModel {
        id: Set(run_id.clone()),
        strategy_id: Set(request.strategy_id.clone()),
        name: Set(request.name.clone()),
        start_date: Set(request.start_date.clone()),
        end_date: Set(request.end_date.clone()),
        initial_cash: Set(request.initial_cash),
        config_json: Set(config_json),
        status: Set("pending".to_string()),
        walk_forward_enabled: Set(if request.walk_forward_enabled { 1 } else { 0 }),
        started_at: Set(started_at),
        ..Default::default()
    };
    let pending = pending.insert(db).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("插入回测记录失败: {e}"))
    })?;

    // 3. 执行回测
    match execute_backtest(&request, &strategy_model, &state.astock_client).await {
        Ok((result, metrics, walk_forward_rust)) => {
            let finished_at = Utc::now().timestamp();
            let result_json = serde_json::to_string(&result).map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("序列化结果失败: {e}"))
            })?;
            let walk_forward = walk_forward_rust.as_ref().map(map_wf_report);

            let mut am: quant_runs::ActiveModel = pending.into();
            am.status = Set("completed".to_string());
            am.result_json = Set(Some(result_json));
            am.finished_at = Set(Some(finished_at));
            if let Some(wf) = &walk_forward_rust {
                am.walk_forward_overfit_warning = Set(Some(if wf.overfit_warning { 1 } else { 0 }));
                am.walk_forward_stability_score = Set(Some(wf.stability_score));
            }
            let model = am.update(db).await.map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("更新回测记录失败: {e}"))
            })?;

            Ok(BacktestRunResponse {
                run: model,
                metrics,
                signal_count: result.signals.len(),
                trade_count: result.trades.len(),
                walk_forward,
            })
        },
        Err(err) => {
            let finished_at = Utc::now().timestamp();
            let mut am: quant_runs::ActiveModel = pending.into();
            am.status = Set("failed".to_string());
            am.error_message = Set(Some(err.clone()));
            am.finished_at = Set(Some(finished_at));
            // 失败状态落库即使出错也不覆盖原始错误，只记录日志避免僵尸记录
            if let Err(db_err) = am.update(db).await {
                tracing::error!(
                    "[quant_backtest] 回测失败后更新状态为 failed 失败: {db_err} (原始错误: {err})"
                );
            }
            Err(err)
        },
    }
}

/// 按 runId 读取回测运行记录
#[agent_command(domain = quant, safety = Safe, call_mode = StateInput, description = "获取回测运行记录")]
#[tauri::command]
pub async fn quant_run_get(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<quant_runs::Model, String> {
    let db = state.harness.db();
    quant_runs::Entity::find_by_id(&run_id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询回测记录失败: {e}"))
        })?
        .ok_or_else(|| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("回测记录不存在: {}", run_id))
                .to_string()
        })
}

/// 核心执行：拉数据 + 跑引擎 + 可选 Walk-Forward
async fn execute_backtest(
    request: &BacktestRunRequest,
    strategy_model: &quant_strategies::Model,
    client: &std::sync::Arc<AStockClient>,
) -> Result<(BacktestResult, MetricsReport, Option<RawWalkForwardReport>), String> {
    // 1. 构造策略实例
    let mut strategy = build_strategy(strategy_model, &request.params)?;

    // 2. 拉取日 K 线（根据日期范围估算所需数量，避免硬编码）
    //    估算方式：日期范围天数 × 250/365（交易日比例）+ 20% 缓冲
    let kline_count = {
        let start = NaiveDate::parse_from_str(&request.start_date, "%Y-%m-%d").ok();
        let end = NaiveDate::parse_from_str(&request.end_date, "%Y-%m-%d").ok();
        match (start, end) {
            (Some(s), Some(e)) => {
                let total_days = (e - s).num_days().max(30) as f64;
                let estimated = (total_days * 250.0 / 365.0 * 1.2).ceil() as u32;
                estimated.clamp(250, 8000)
            },
            _ => 2000u32,
        }
    };
    // 复用 AppState 中共享的 AStockClient（连接池 + 缓存共享），避免每次请求新建
    tracing::debug!("[quant_backtest] 复用共享 AStockClient");
    // 同时尝试获取实时行情（获取涨跌停/ST 标记，用于回测撮合）
    let quote_info = client.get_quote(&request.code).await.ok();
    let (limit_up, limit_down, is_st) = match quote_info {
        Some(ref q) => (q.limit_up, q.limit_down, q.is_st),
        None => (None, None, false),
    };

    // 修复 C2: 回测必须使用前复权数据，否则遇分红/送转标的价格断层，虚假信号、收益与回撤失真
    let klines = client
        .get_klines_with_adj(&request.code, "daily", kline_count, Some(AdjType::Forward))
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("拉取前复权K线失败(limit={kline_count}): {e}"))
        })?;
    if klines.is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("未获取到 {} 的K线数据", request.code))
            .to_string());
    }
    let bars: Vec<Bar> = klines
        .into_iter()
        .map(|k| kline_to_bar(k, &request.code, limit_up, limit_down, is_st))
        .collect();

    // 3. 构造回测配置
    let matcher = request.matcher_config.clone().map(QuantMatcherConfig::from).unwrap_or_default();
    let config = BacktestConfig {
        initial_cash: request.initial_cash,
        matcher,
        start_date: Some(request.start_date.clone()),
        end_date: Some(request.end_date.clone()),
        codes: vec![request.code.clone()],
    };

    // 4. 跑主回测
    let engine = BacktestEngine::new(config);
    let bars_for_wf = bars.clone();
    let result = engine.run(strategy.as_mut(), bars).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("回测执行失败: {e}"))
    })?;
    let metrics = MetricsReport::from_backtest_result(&result, 0.025);

    // 5. 可选 Walk-Forward 验证（默认开启，forceOff 可显式关闭）
    let walk_forward = if request.walk_forward_enabled && !request.walk_forward_force_off {
        let wf = WalkForward::new(WalkForwardConfig::default());
        let sm = strategy_model.clone();
        let params = request.params.clone();
        let strategy_name = sm.name.clone();
        // 修复 P0-T4: 改 Result 传播，避免 panic 拖垮 Tauri 进程。
        // 之前依赖"factory 内构造不应失败"的假设并不严格；
        // 改为在 factory 内返回错误，WalkForward::run 接受 Result<_, _>。
        let report = wf
            .run(
                move |_| {
                    build_strategy(&sm, &params).map_err(|e| {
                        ErrorResponse::new(wf_err::INTERNAL)
                            .with_detail(format!(
                                "WalkForward 策略构造失败 (name={strategy_name}): {e}"
                            ))
                            .to_string()
                    })
                },
                bars_for_wf,
            )
            .await
            .map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("Walk-Forward 验证失败: {e}"))
            })?;
        Some(report)
    } else {
        None
    };

    Ok((result, metrics, walk_forward))
}

/// 根据策略实体 + 运行时参数构造策略实例
fn build_strategy(
    model: &quant_strategies::Model,
    params: &HashMap<String, Value>,
) -> Result<Box<dyn Strategy>, String> {
    let f64_param = |key: &str, default: f64| -> f64 {
        params.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
    };
    let usize_param = |key: &str, default: usize| -> usize {
        params.get(key).and_then(|v| v.as_u64()).map(|x| x as usize).unwrap_or(default)
    };

    match model.strategy_type.as_str() {
        "rhai" => {
            let script = model.script_source.clone().ok_or_else(|| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("rhai 策略 {} 缺少 script_source", model.name))
            })?;
            let s = RhaiStrategy::from_script(model.name.clone(), script).map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("编译 rhai 策略失败: {e}"))
            })?;
            Ok(Box::new(s))
        },
        // builtin 或未知类型：按 name 关键字分发到内置策略
        _ => {
            let name = model.name.to_lowercase();
            if name.contains("rsi") {
                let s = RsiStrategy::new(
                    usize_param("period", 14),
                    f64_param("overbought", 70.0),
                    f64_param("oversold", 30.0),
                )
                .map_err(|e| {
                    ErrorResponse::new(wf_err::INTERNAL)
                        .with_detail(format!("RSI 策略参数错误: {e}"))
                })?;
                Ok(Box::new(s))
            } else if name.contains("boll") || name.contains("布林") {
                Ok(Box::new(BollStrategy::new(usize_param("period", 20), f64_param("stddev", 2.0))))
            } else if name.contains("macd") {
                Ok(Box::new(MacdStrategy::new(
                    usize_param("fast", 12),
                    usize_param("slow", 26),
                    usize_param("signal", 9),
                )))
            } else if name.contains("turtle") || name.contains("海龟") {
                Ok(Box::new(TurtleStrategy::new(
                    usize_param("entryPeriod", 20),
                    usize_param("exitPeriod", 10),
                    usize_param("atrPeriod", 20),
                    f64_param("atrMultiplier", 2.0),
                )))
            } else if name.contains("ma") || name.contains("金叉") {
                Ok(Box::new(MaCrossStrategy::new(
                    usize_param("shortPeriod", 5),
                    usize_param("longPeriod", 20),
                )))
            } else {
                // 默认 RSI
                tracing::warn!(
                    "Unknown builtin strategy '{}', defaulting to RSI(14,70,30)",
                    model.name
                );
                let s = RsiStrategy::new(14, 70.0, 30.0).map_err(|e| {
                    ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("{e}"))
                })?;
                Ok(Box::new(s))
            }
        },
    }
}

/// KLine → Bar（注入股票代码，补全撮合所需字段）
/// 修复 D4-B: 通过 quote 数据提供涨跌停/ST 标记
fn kline_to_bar(
    k: KLine,
    code: &str,
    limit_up: Option<f64>,
    limit_down: Option<f64>,
    is_st: bool,
) -> Bar {
    Bar {
        date: k.date,
        code: code.to_string(),
        open: k.open,
        high: k.high,
        low: k.low,
        close: k.close,
        volume: k.volume,
        amount: k.amount,
        turnover_rate: k.turnover_rate,
        adj_factor: k.adj_factor,
        limit_up,
        limit_down,
        is_st,
    }
}

// ── 策略列表 ──

/// 列出所有量化策略（内置 + Rhai 注册）
#[agent_command(domain = quant, safety = Safe, call_mode = StateInput, description = "列出量化策略")]
#[tauri::command]
pub async fn quant_strategies_list(
    state: State<'_, AppState>,
) -> Result<Vec<quant_strategies::Model>, String> {
    use sea_orm::EntityTrait;
    quant_strategies::Entity::find().all(state.harness.db()).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("查询策略列表失败: {e}"))
            .to_string()
    })
}

/// 注册 Rhai 策略
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRhaiRequest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub script_source: String,
    pub params: HashMap<String, Value>,
    pub walk_forward_enabled: bool,
    pub upsert: bool,
}

#[agent_command(domain = quant, safety = Caution, call_mode = StateInput, description = "注册 Rhai 策略")]
#[tauri::command]
pub async fn quant_strategy_register_rhai(
    state: State<'_, AppState>,
    request: RegisterRhaiRequest,
) -> Result<quant_strategies::Model, String> {
    use sea_orm::ActiveModelTrait;
    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;
    use uuid::Uuid;

    let db = state.harness.db();
    let now = Utc::now().timestamp();
    let params_json = serde_json::to_string(&request.params).unwrap_or_else(|_| "{}".to_string());

    // upsert 模式：按 name 查找已有记录
    if request.upsert {
        if let Some(existing) = quant_strategies::Entity::find()
            .filter(quant_strategies::Column::Name.eq(&request.name))
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询已有策略失败: {e}"))
            })?
        {
            let mut am: quant_strategies::ActiveModel = existing.into();
            am.version = Set(request.version.clone());
            am.description = Set(request.description.clone());
            am.script_source = Set(Some(request.script_source.clone()));
            am.params_json = Set(Some(params_json));
            am.walk_forward_enabled = Set(if request.walk_forward_enabled { 1 } else { 0 });
            am.updated_at = Set(now);
            return am.update(db).await.map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("更新策略失败: {e}"))
                    .to_string()
            });
        }
    }

    // 新建
    let new_id = Uuid::new_v4().to_string();
    let model = quant_strategies::ActiveModel {
        id: Set(new_id),
        name: Set(request.name),
        version: Set(request.version),
        strategy_type: Set("rhai".to_string()),
        description: Set(request.description),
        script_source: Set(Some(request.script_source)),
        params_json: Set(Some(params_json)),
        walk_forward_enabled: Set(if request.walk_forward_enabled { 1 } else { 0 }),
        created_at: Set(now),
        updated_at: Set(now),
    };
    model.insert(db).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("插入策略失败: {e}")).to_string()
    })
}

// ── 指标对比 ──

/// 多 run 指标对比结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunWithMetricsResponse {
    pub run: quant_runs::Model,
    pub strategy_name: String,
    pub metrics: Option<MetricsReport>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsCompareResponse {
    pub runs: Vec<RunWithMetricsResponse>,
    /// metric_name → run_id（各指标最佳者）
    pub best_by: HashMap<String, String>,
}

#[agent_command(domain = quant, safety = Safe, call_mode = StateInput, description = "对比回测指标")]
#[tauri::command]
pub async fn quant_metrics_compare(
    state: State<'_, AppState>,
    run_ids: Vec<String>,
) -> Result<MetricsCompareResponse, String> {
    use sea_orm::EntityTrait;

    let db = state.harness.db();
    let mut runs = Vec::with_capacity(run_ids.len());
    for rid in run_ids {
        let run = quant_runs::Entity::find_by_id(&rid).one(db).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询 run {rid} 失败: {e}"))
        })?;
        if let Some(r) = run {
            let strategy_name = {
                let sm = quant_strategies::Entity::find_by_id(&r.strategy_id)
                    .one(db)
                    .await
                    .map_err(|e| {
                        ErrorResponse::new(wf_err::INTERNAL)
                            .with_detail(format!("查询策略 {rid} 失败: {e}"))
                    })?;
                sm.map(|s| s.name).unwrap_or_default()
            };
            let metrics = r.result_json.as_ref().and_then(|json| {
                serde_json::from_str::<BacktestResult>(json)
                    .ok()
                    .map(|br| MetricsReport::from_backtest_result(&br, 0.025))
            });
            runs.push(RunWithMetricsResponse {
                run: r,
                strategy_name,
                metrics,
                error_message: None,
            });
        }
    }

    // 计算各指标最佳者
    let mut best_by: HashMap<String, String> = HashMap::new();
    // 年化收益率最高 → best sharpe / totalReturn
    let mut best_sharpe = f64::NEG_INFINITY;
    let mut best_total_return = f64::NEG_INFINITY;
    let mut best_sortino = f64::NEG_INFINITY;
    let mut best_profit_factor = f64::NEG_INFINITY;
    let mut best_max_drawdown = f64::INFINITY; // 回撤越小越好
    for r in &runs {
        if let Some(ref m) = r.metrics {
            if m.sharpe > best_sharpe {
                best_sharpe = m.sharpe;
                best_by.insert("sharpe".into(), r.run.id.clone());
            }
            if m.total_return > best_total_return {
                best_total_return = m.total_return;
                best_by.insert("totalReturn".into(), r.run.id.clone());
            }
            if m.sortino > best_sortino {
                best_sortino = m.sortino;
                best_by.insert("sortino".into(), r.run.id.clone());
            }
            if m.profit_factor > best_profit_factor {
                best_profit_factor = m.profit_factor;
                best_by.insert("profitFactor".into(), r.run.id.clone());
            }
            if m.max_drawdown < best_max_drawdown {
                best_max_drawdown = m.max_drawdown;
                best_by.insert("maxDrawdown".into(), r.run.id.clone());
            }
        }
    }

    Ok(MetricsCompareResponse { runs, best_by })
}

/// 原始 WalkForwardReport → 响应结构（对齐前端契约）
fn map_wf_report(r: &RawWalkForwardReport) -> WalkForwardReportResponse {
    let folds = r
        .windows
        .iter()
        .map(|w| WalkForwardFoldResponse {
            train_bars_count: w.fold.train_bars_count,
            test_bars_count: w.fold.test_bars_count,
            fold_index: w.fold.fold_idx,
            train_start: w.fold.train_start.clone(),
            train_end: w.fold.train_end.clone(),
            test_start: w.fold.test_start.clone(),
            test_end: w.fold.test_end.clone(),
            train_sharpe: w.train_metrics.sharpe,
            test_sharpe: w.test_metrics.sharpe,
            best_params: w
                .best_params
                .as_ref()
                .map(|v| serde_json::to_value(v).unwrap_or(Value::Null)),
        })
        .collect();
    WalkForwardReportResponse {
        folds,
        oos_equity: r.aggregated_oos_equity.iter().map(|e| e.equity).collect(),
        stability_score: r.stability_score,
        overfit_window_count: r.overfit_window_count,
        aggregated_test_sharpe: r.aggregated_oos_metrics.sharpe,
    }
}
