use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent::self_improvement_executor::{SelfImprovementConfig, SelfImprovementExecutor};
use axagent_agent_macro::agent_command;
use axagent_analysis_engine::backtest::{
    BacktestEngine, BacktestResult, BacktestStats, HistoricalAnalysis,
};
use axagent_analysis_engine::backtest_feedback;
use axagent_analysis_engine::evidence_weight::{self, EvidenceWeightReport, EvidenceWeightRequest};
use axagent_analysis_engine::key_levels::{KeyLevelBacktestStats, KeyLevelTracker};
use axagent_analysis_engine::plugin::AnalystPluginManager;
use axagent_analysis_engine::portfolio_monitor::{
    self, CorrelationCell, PortfolioDashboard, StressTestBundle,
};
use axagent_analysis_engine::portfolio_risk::{PortfolioRiskManager, PortfolioRiskMetrics};
use axagent_analysis_engine::position_limits::PositionLimits;
use axagent_analysis_engine::recommender::{self, RecoResponse};
use axagent_analysis_engine::review::{DailyReview, PostCloseReview};
use axagent_analysis_engine::screener::{ScreenCriteria, ScreenResult, StockScreener};
use axagent_analysis_engine::stock_analysis_round::StockAnalysisRound;
use axagent_analysis_engine::trading::{PositionSummary, TradePredictionComparison};
use axagent_astock_data::as_of::{self, AsOfContext};
use axagent_astock_data::batch::{BatchRequest, BatchResult, BatchRunner, MarketBatchQuery};
use axagent_astock_data::fundamentals_report::{FundamentalsAnalyzer, FundamentalsReport};
use axagent_astock_data::{FinancialReport, StockQuote};
use axagent_entities::{
    financial_snapshots, portfolio_holdings, price_alerts, reco_picks, stock_analyses, trades,
    watchlist_items,
};
use axagent_harness::market_data::KLine;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ── What-If 回测命令（结构化参数方案 Phase 5/6）──
// 前端 What-If 面板调用此命令，后端执行 Rhai 引擎确保公式与 DAG 中完全一致。

/// What-If 回测请求参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatIfRequest {
    pub total_score: f64,
    pub dqi_score: f64,
    pub overall_risk: String,
    pub catalyst_level: String,
    pub consensus_score: f64,
    /// 机构痕迹（龙虎榜/大宗交易/北上资金等汇总描述）
    #[serde(default)]
    #[allow(dead_code)]
    pub institutional_trace: String,
    /// DAG 黑板上一次快照（JSON 字符串），包含所有上游节点输出。
    /// 提供时后端自动解构并注入 portfolio-mgr.rhai 所需的所有参数。
    /// 缺失时仅根据 6 个显式参数运行（简化模式）。
    #[serde(default)]
    pub blackboard_snapshot: Option<String>,
}

/// What-If 回测结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatIfResult {
    pub decision: String,
    /// 建议仓位百分比，**语义为 0–100 的整数百分比**（例如 40 表示 40%，不是 0.4）。
    /// 前端展示时直接当作百分比数值使用，切勿再除以 100。
    pub position_pct: f64,
    pub confidence: f64,
    pub risk_level: String,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub reasoning: String,
    /// 决策追溯链（来自 portfolio-mgr.rhai 完整输出）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_trail: Option<Vec<DecisionTrailItem>>,
    /// 技术面否决详情
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technical_veto: Option<TechnicalVetoInfo>,
    /// 模拟门信息
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulation_gate: Option<SimulationGateInfo>,
    /// 模拟门前的原始决策 action
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_sim_action: Option<String>,
    /// 模拟门前的原始仓位
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_sim_position_pct: Option<f64>,
}

/// 决策追溯链节点
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionTrailItem {
    pub rule_id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// 技术面否决信息
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicalVetoInfo {
    pub vetoed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 模拟门信息（S-501~503）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationGateInfo {
    pub vetoed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 模拟门前的原始决策 action
    pub pre_sim_action: String,
    /// 模拟门前的原始仓位
    pub pre_sim_position_pct: f64,
    /// 模拟实测的价格稳定性
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sim_stability: Option<f64>,
    /// 模拟实测的流动性
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sim_liquidity: Option<f64>,
    /// 模拟实测的冲击成本 (bps)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sim_impact: Option<f64>,
}

/// 执行 portfolio-mgr 确定性公式（Rhai 引擎）。
/// 修复 D2: 从文件加载完整 portfolio-mgr.rhai，通过 blackboard_snapshot 提供完整参数。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "执行What-If回测计算")]
#[tauri::command]
pub fn compute_what_if(params: WhatIfRequest) -> Result<WhatIfResult, String> {
    use rhai::{Engine, Scope};

    let mut engine = Engine::new();
    // C4 补充: portfolio-mgr.rhai 因子多、表达式嵌套深，必须放宽 max_expr_depths
    // （默认上限会在 line 518 处触发 "Expression exceeds maximum complexity"，且该错误
    // 是编译期抛出、脚本内 try/catch 无法捕获）。256 为实测下限(48)的 ~5 倍余量。
    engine.set_max_expr_depths(256, 256);
    // 注册脚本依赖的 json_parse/clamp/join，否则 safe_parse/clamp 调用会抛 Function not found
    axagent_harness::rhai_engine::register_common_functions(&mut engine);
    let mut scope = Scope::new();

    // 1. 注入显式参数（用户在前端调整的 6 个核心值）
    scope.push_constant("totalScore", params.total_score);
    scope.push_constant("dqi_score", params.dqi_score);
    scope.push_constant("overall_risk", params.overall_risk.clone());
    scope.push_constant("catalyst_level", params.catalyst_level.clone());
    scope.push_constant("consensusScore", params.consensus_score);

    // 2. 从 blackboard 快照注入完整参数（修复 D2: 从 _raw.portfolio-mgr.input_params 读取）
    if let Some(ref snapshot_json) = params.blackboard_snapshot {
        if let Ok(snapshot) = serde_json::from_str::<serde_json::Value>(snapshot_json) {
            // 优先路径: _raw.portfolio-mgr.input_params（CodeNode 在 DAG 执行时保存的
            // 完整 input_mapping 解析值快照，包含所有上游节点注入的参数）
            // 回退路径: params.portfolio-mgr.input_params（旧版快照）
            let input_params = snapshot
                .pointer("/_raw/portfolio-mgr/input_params")
                .or_else(|| snapshot.pointer("/params/portfolio-mgr/input_params"))
                .or_else(|| {
                    // 无 snapshot 时尝试使用根对象
                    if snapshot.is_object() {
                        Some(&snapshot)
                    } else {
                        None
                    }
                });

            if let Some(params_map) = input_params.and_then(|v| v.as_object()) {
                for (key, val) in params_map {
                    // 跳过已注入的显式字段（前端显式值优先）
                    if key == "totalScore"
                        || key == "dqi_score"
                        || key == "overall_risk"
                        || key == "catalyst_level"
                        || key == "consensusScore"
                    {
                        continue;
                    }
                    // 根据值类型注入 Rhai scope
                    match val {
                        serde_json::Value::Number(n) => {
                            if let Some(f) = n.as_f64() {
                                scope.push_constant(key.as_str(), f);
                            }
                        },
                        serde_json::Value::String(s) => {
                            scope.push_constant(key.as_str(), s.clone());
                        },
                        serde_json::Value::Bool(b) => {
                            scope.push_constant(key.as_str(), *b);
                        },
                        // Map 和 Array 通过 JSON 字符串传递，safe_parse 在 Rhai 中处理
                        serde_json::Value::Object(map) => {
                            if let Ok(json_str) = serde_json::to_string(map) {
                                scope.push_constant(key.as_str(), json_str);
                            }
                        },
                        serde_json::Value::Array(arr) => {
                            if let Ok(json_str) = serde_json::to_string(arr) {
                                scope.push_constant(key.as_str(), json_str);
                            }
                        },
                        serde_json::Value::Null => { /* 不注入，Rhai 中 present()=false */ },
                    }
                }
            }
            // 若 _raw.portfolio-mgr.input_params 不存在，快照可能为旧版。
            // 此时仅 5 个显式参数可用，Rhai 脚本中其他变量 present()=false，走兜底逻辑。
        }
        // 解析失败不报错，静默降级（只有 5 个显式参数可用）
    }

    // 3. 从文件加载完整 portfolio-mgr.rhai 公式
    //    使用 include_str! 编译时嵌入，与 DAG 中实际使用的文件保持同步
    let code = include_str!("portfolio-mgr.rhai");

    // P1-D10: 通过全局 AST 缓存复用编译结果，避免 What-If 回测时重复编译 1373 行脚本。
    // AST 与 Engine 解耦，缓存的 AST 可被当前 Engine（含 common_functions）正确执行。
    let ast = axagent_harness::get_or_compile_ast("portfolio-mgr-whatif", code, &engine).map_err(
        |e| ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("Rhai AST 编译失败: {e}")),
    )?;

    let result: rhai::Dynamic = engine.eval_ast_with_scope(&mut scope, &ast).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("Rhai 执行失败: {e}"))
    })?;

    // 4. 转换结果为 WhatIfResult
    let result_map = result
        .clone()
        .try_cast::<rhai::Map>()
        .ok_or_else(|| ErrorResponse::new(wf_err::INTERNAL).with_detail("Rhai 返回结果不是 map"))?;

    let get_str = |key: &str| -> String {
        result_map.get(key).and_then(|v| v.clone().try_cast::<String>()).unwrap_or_default()
    };
    let get_f64 =
        |key: &str| -> f64 { result_map.get(key).and_then(|v| v.as_float().ok()).unwrap_or(0.0) };
    // 从 map 中递归提取 decision_trail 数组
    let get_decision_trail = |map: &rhai::Map| -> Option<Vec<DecisionTrailItem>> {
        let trail_val = map.get("decision_trail")?;
        let trail_dynamic = trail_val.clone();
        let trail_array: rhai::Dynamic = trail_dynamic;
        let arr = trail_array.into_array().ok()?;
        let items: Vec<DecisionTrailItem> = arr
            .into_iter()
            .filter_map(|item| {
                let item_map = item.try_cast::<rhai::Map>()?;
                Some(DecisionTrailItem {
                    rule_id: item_map
                        .get("rule_id")
                        .and_then(|v| v.clone().try_cast::<String>())
                        .unwrap_or_default(),
                    status: item_map
                        .get("status")
                        .and_then(|v| v.clone().try_cast::<String>())
                        .unwrap_or_default(),
                    detail: item_map.get("detail").and_then(|v| v.clone().try_cast::<String>()),
                    timestamp: item_map
                        .get("timestamp")
                        .and_then(|v| v.clone().try_cast::<String>()),
                })
            })
            .collect();
        if items.is_empty() { None } else { Some(items) }
    };
    let get_technical_veto = |map: &rhai::Map| -> Option<TechnicalVetoInfo> {
        let veto_val = map.get("technical_veto")?;
        let veto_map = veto_val.clone().try_cast::<rhai::Map>()?;
        Some(TechnicalVetoInfo {
            vetoed: veto_map.get("vetoed").and_then(|v| v.as_bool().ok()).unwrap_or(false),
            rule_id: veto_map.get("rule_id").and_then(|v| v.clone().try_cast::<String>()),
            reason: veto_map.get("reason").and_then(|v| v.clone().try_cast::<String>()),
        })
    };
    let get_simulation_gate = |map: &rhai::Map| -> Option<SimulationGateInfo> {
        let gate_val = map.get("simulation_gate")?;
        let gate_map = gate_val.clone().try_cast::<rhai::Map>()?;
        Some(SimulationGateInfo {
            vetoed: gate_map.get("vetoed").and_then(|v| v.as_bool().ok()).unwrap_or(false),
            rule_id: gate_map.get("rule_id").and_then(|v| v.clone().try_cast::<String>()),
            reason: gate_map.get("reason").and_then(|v| v.clone().try_cast::<String>()),
            pre_sim_action: gate_map
                .get("pre_sim_action")
                .and_then(|v| v.clone().try_cast::<String>())
                .unwrap_or_default(),
            pre_sim_position_pct: gate_map
                .get("pre_sim_position_pct")
                .and_then(|v| v.as_float().ok())
                .unwrap_or(0.0),
            sim_stability: gate_map.get("sim_stability").and_then(|v| v.as_float().ok()),
            sim_liquidity: gate_map.get("sim_liquidity").and_then(|v| v.as_float().ok()),
            sim_impact: gate_map.get("sim_impact").and_then(|v| v.as_float().ok()),
        })
    };

    Ok(WhatIfResult {
        decision: get_str("action"),
        position_pct: get_f64("positionPct"),
        confidence: get_f64("confidence"),
        risk_level: get_str("riskLevel"),
        stop_loss_pct: get_f64("stopLossPct"),
        take_profit_pct: get_f64("takeProfitPct"),
        reasoning: get_str("reasoning"),
        decision_trail: get_decision_trail(&result_map),
        technical_veto: get_technical_veto(&result_map),
        simulation_gate: get_simulation_gate(&result_map),
        pre_sim_action: result_map
            .get("pre_sim_action")
            .and_then(|v| v.clone().try_cast::<String>()),
        pre_sim_position_pct: result_map
            .get("pre_sim_position_pct")
            .and_then(|v| v.as_float().ok()),
    })
}

// ── 工具链回放命令（L2 配置参数覆盖回测）──
// 允许用户修改评分权重/估值参数/风控参数后，重算工具链并得到新决策。

/// 工具链回放请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayToolChainRequest {
    pub stock_code: String,
    /// 用户要覆盖的配置参数（key=参数名, value=新值）
    pub config_overrides: std::collections::HashMap<String, serde_json::Value>,
}

/// 工具链回放结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayToolChainResult {
    pub total_score: f64,
    pub score_details: serde_json::Value,
    pub valuation_result: serde_json::Value,
    pub risk_result: serde_json::Value,
    pub decision: WhatIfResult,
    /// 数据源降级标记：为 true 表示本次回放的部分实时数据来自缓存/中性占位，
    /// 评分权重推演仍可运行，但估值/支撑位等依赖实时行情的结果仅供参考。
    pub data_degraded: bool,
}

/// 进程级「最近一次成功获取」的实时数据缓存，供数据源临时不可用时降级回放。
/// key = 股票代码；value = (K线, 实时行情, 缓存时间戳)。
/// 修复 D3: 60 秒 TTL + 最多 1000 条限制，过期条目惰性清理。
#[allow(clippy::type_complexity)]
static LAST_MARKET_DATA: std::sync::LazyLock<
    Mutex<HashMap<String, (Vec<KLine>, StockQuote, Instant)>>,
> = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

const LAST_MARKET_DATA_TTL: Duration = Duration::from_secs(60);
const LAST_MARKET_DATA_CAP: usize = 1000;

/// 构造一个中性占位的行情（价格为 0、PE 为空），用于 quote 获取失败时的降级，
/// 使估值/风险/What-If 仍可基于已有 K 线运行（评分权重推演不受影响）。
///
/// ## D10 风险分析结论（2026-07-12）
/// - `neutral_quote` 仅有 1 个调用点：`replay_tool_chain` 中的 K线成功+行情失败分支。
/// - 该路径下的 `price=0.0` 仅在 downstream `support_score` 计算中作为分子，
///   分母有 `t > 0` 守卫，不会触发除零崩溃，仅导致 support_score 无意义。
/// - 其余 `quote.price` 除法使用（PE计算 / 估值比率）均有 `price > 0.0` 前置守卫。
/// - 结论：**不改 Option**，零值占位是故意设计的降级策略，实际无除零风险。
fn neutral_quote(code: &str) -> StockQuote {
    StockQuote {
        code: code.to_string(),
        name: String::new(),
        price: 0.0,
        pre_close: 0.0,
        open: 0.0,
        high: 0.0,
        low: 0.0,
        volume: 0.0,
        amount: 0.0,
        change_pct: 0.0,
        turnover_rate: 0.0,
        pe: None,
        pb: None,
        total_mv: None,
        circulating_mv: None,
        limit_up: None,
        limit_down: None,
        is_st: false,
        timestamp: String::new(),
    }
}

/// 重新运行工具链（t-scoring / t-valuation / t-risk）带上覆盖的配置参数。
/// 工具本身是纯 Rust 函数（确定性），修改配置参数后会产生不同输出。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "重跑工具链回放配置参数")]
#[tauri::command]
pub async fn replay_tool_chain(
    state: State<'_, AppState>,
    params: ReplayToolChainRequest,
) -> Result<ReplayToolChainResult, String> {
    use axagent_astock_data::{AStockClient, indicators};
    use std::sync::Arc;

    let code = &params.stock_code;
    let client = Arc::new(AStockClient::new());

    // ── 获取实时数据（带降级回退）──
    // 任一数据源失败时，优先使用进程级「最近一次成功」缓存；仅 quote 失败时退化为
    // 中性占位行情，使评分权重推演仍可运行；两者皆失败且无缓存时才返回明确错误。
    let (klines_res, quote_res) =
        tokio::join!(client.get_klines(code, "daily", 120), client.get_quote(code),);

    let (klines, quote, data_degraded) = match (klines_res, quote_res) {
        (Ok(k), Ok(q)) => (k, q, false),
        (Ok(k), Err(_)) => {
            // K 线成功，行情失败 → 中性占位，标记降级（估值/支撑位仅供参考）
            (k, neutral_quote(code), true)
        },
        (Err(_), Ok(q)) => {
            // 行情成功，K 线失败 → 尝试使用缓存的 K 线（检查 TTL）
            let cached = LAST_MARKET_DATA
                .lock()
                .map_err(|_| {
                    ErrorResponse::new(wf_err::INTERNAL).with_detail("K 线缓存锁 poisoned")
                })?
                .get(code)
                .and_then(|(k, _, ts)| {
                    if *ts + LAST_MARKET_DATA_TTL >= Instant::now() {
                        Some(k.clone())
                    } else {
                        None
                    }
                });
            match cached {
                Some(k) => (k, q, true),
                None => {
                    return Err(
                        "Failed to get klines: 数据源暂时不可用且无本地缓存，无法回放".to_string()
                    );
                },
            }
        },
        (Err(_), Err(_)) => {
            // 两者皆失败 → 整组使用缓存（检查 TTL）
            let cached = LAST_MARKET_DATA
                .lock()
                .map_err(|_| {
                    ErrorResponse::new(wf_err::INTERNAL).with_detail("行情缓存锁 poisoned")
                })?
                .get(code)
                .and_then(|(k, q, ts)| {
                    if *ts + LAST_MARKET_DATA_TTL >= Instant::now() {
                        Some((k.clone(), q.clone()))
                    } else {
                        None
                    }
                });
            match cached {
                Some((k, q)) => (k, q, true),
                None => {
                    return Err(
                        "Failed to get market data (klines & quote): 数据源暂时不可用且无本地缓存"
                            .to_string(),
                    );
                },
            }
        },
    };

    // 更新「最近一次成功」缓存（仅当本次确有真实数据，含 TTL + 容量限制）
    if !data_degraded {
        if let Ok(mut cache) = LAST_MARKET_DATA.lock() {
            // 超过容量时清理过期条目
            if cache.len() >= LAST_MARKET_DATA_CAP {
                cache.retain(|_, (_, _, ts)| *ts + LAST_MARKET_DATA_TTL >= Instant::now());
            }
            cache.insert(code.clone(), (klines.clone(), quote.clone(), Instant::now()));
        }
    }

    // ── 从 _template_vars 读取参数（用覆盖值替换默认值） ──
    let tv = |key: &str, default: f64| -> f64 {
        params.config_overrides.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
    };
    let tv_str = |key: &str, default: &str| -> String {
        params.config_overrides.get(key).and_then(|v| v.as_str()).unwrap_or(default).to_string()
    };

    // ── 1. compute_scoring（技术综合评分）──
    let ind = indicators::compute_indicators(code, &klines);

    let w_trend = tv("scoring_trend", 30.0);
    let w_deviation = tv("scoring_deviation", 20.0);
    let w_macd = tv("scoring_macd", 15.0);
    let w_volume = tv("scoring_volume", 15.0);
    let w_rsi = tv("scoring_rsi", 10.0);
    let w_support = tv("scoring_support", 10.0);
    let catalyst_score = tv("catalyst_analyst_score", 50.0);

    let trend_score = match ind.ma_alignment.as_str() {
        "多头排列" => 90.0,
        "弱多头" => 70.0,
        "缠绕/交叉" => 50.0,
        "空头排列" => 30.0,
        _ => 50.0,
    };
    let bias_avg = (ind.bias_ma5.abs() + ind.bias_ma20.abs()) / 2.0;
    let deviation_score = if bias_avg < 2.0 {
        80.0
    } else if bias_avg < 5.0 {
        60.0
    } else if bias_avg < 10.0 {
        40.0
    } else {
        20.0
    };
    let macd_score = match ind.macd_signal.as_str() {
        "金叉" => 90.0,
        "多头运行" => 70.0,
        "死叉" => 30.0,
        "空头运行" => 20.0,
        _ => 50.0,
    };
    let volume_score = match ind.volume_signal.as_str() {
        "放量突破" => 95.0,
        "放量上涨" => 90.0,
        "缩量回调" => 60.0,
        "正常" => 50.0,
        "缩量上涨" => 40.0,
        "放量下跌" => 20.0,
        _ => 50.0,
    };
    let rsi_score = if ind.rsi6 > 80.0 {
        25.0
    } else if ind.rsi6 > 70.0 {
        45.0
    } else if ind.rsi6 > 50.0 {
        75.0
    } else if ind.rsi6 > 30.0 {
        55.0
    } else {
        80.0
    };
    let support_score = if !ind.support_levels.is_empty() && !ind.resistance_levels.is_empty() {
        let d_s = (quote.price - ind.support_levels[0]).abs();
        let d_r = (ind.resistance_levels[0] - quote.price).abs();
        let t = d_s + d_r;
        if t > 0.0 { (d_s / t) * 100.0 } else { 50.0 }
    } else {
        50.0
    };
    let base = trend_score * w_trend / 100.0
        + deviation_score * w_deviation / 100.0
        + macd_score * w_macd / 100.0
        + volume_score * w_volume / 100.0
        + rsi_score * w_rsi / 100.0
        + support_score * w_support / 100.0;
    let total_score = (base * 0.7 + catalyst_score * 0.3).round().clamp(0.0, 100.0);

    let score_details = serde_json::json!({
        "trendScore": trend_score, "deviationScore": deviation_score,
        "macdScore": macd_score, "volumeScore": volume_score,
        "rsiScore": rsi_score, "supportScore": support_score,
        "catalystScore": catalyst_score, "totalScore": total_score,
        "weights": { "trend": w_trend, "deviation": w_deviation, "macd": w_macd,
            "volume": w_volume, "rsi": w_rsi, "support": w_support },
    });

    // ── 2. compute_valuation（简化版：基于 F-Score 和 PE 的基本估值判断）──
    let fscore = tv("fscore_buy_threshold", 7.0) as i64;
    let pe_pct = quote
        .pe
        .as_ref()
        .map(|pe| {
            // 简化版：PE<20 视为低估，PE>40 视为高估
            if *pe < 20.0 {
                20.0
            } else if *pe > 40.0 {
                80.0
            } else {
                50.0
            }
        })
        .unwrap_or(50.0);
    let valuation_result = serde_json::json!({
        "pePercentile": pe_pct, "fscoreThreshold": fscore,
        "valuation": if pe_pct < 30.0 { "undervalued" } else if pe_pct > 70.0 { "overvalued" } else { "fair" },
    });

    // ── 3. compute_portfolio_risk（简化版）──
    let max_dd = tv("risk_max_drawdown_limit", 20.0);
    let hhi_limit = tv("risk_hhi_concentrated", 0.25);
    let kelly_f = tv("kelly_fraction", 0.5);
    let overall_risk = if max_dd > 25.0 {
        "高"
    } else if max_dd > 15.0 {
        "中"
    } else {
        "低"
    };
    let risk_result = serde_json::json!({
        "overallRisk": overall_risk, "maxDrawdownLimit": max_dd,
        "hhiLimit": hhi_limit, "kellyFraction": kelly_f,
    });

    // ── 4. portfolio-mgr 公式 ──
    let what_if = WhatIfRequest {
        total_score,
        dqi_score: 50.0,
        overall_risk: overall_risk.to_string(),
        catalyst_level: tv_str("catalyst_level", "无催化剂"),
        institutional_trace: tv_str("institutional_trace", "无异常"),
        consensus_score: 50.0,
        blackboard_snapshot: None,
    };
    let decision = compute_what_if(what_if)?;

    let _ = client;
    let _ = state;

    Ok(ReplayToolChainResult {
        total_score,
        score_details,
        valuation_result,
        risk_result,
        decision,
        data_degraded,
    })
}
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::State;

/// 获取当前全局 as-of 降级条目总数(进程级,跨 live/replay)。
/// 缺陷 E 修复:前端 poll 用,实时显示降级数量。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取降级条目总数")]
#[tauri::command]
pub fn get_asof_degradation_count() -> u64 {
    as_of::global_degradation_count()
}

/// 拉取最近 256 条全局降级日志(快照,不清空)。
/// 供前端做"降级详情面板"展示。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取降级日志")]
#[tauri::command]
pub fn get_asof_degradation_log() -> Vec<as_of::DegradationEntry> {
    as_of::peek_global_degradation_report()
}

/// 清空全局降级缓冲(用户从 replay 切回 live 时调用,避免过期条目一直显示)。
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "清空降级日志")]
#[tauri::command]
pub fn clear_asof_degradation_log() {
    as_of::reset_global_degradation_log();
}

/// 搜索股票
///
/// `market` 可选参数用于过滤市场：
/// - `"A"` → 仅 A 股（上交所/深交所/北交所/创业板/科创板）
/// - `"HK"` → 仅港股
/// - `"US"` → 仅美股
/// - `None` → 全市场
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "搜索股票")]
#[tauri::command]
pub async fn search_stock(
    state: State<'_, AppState>,
    keyword: String,
    market: Option<String>,
) -> Result<Vec<axagent_astock_data::StockSearchResult>, String> {
    if keyword.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("keyword 不能为空").into());
    }
    let results = state.astock_client.search_stock(&keyword).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("搜索股票失败: {e}"))
    })?;
    // 按市场过滤（基于 market 描述字段或代码后缀）
    let filtered = match market.as_deref() {
        Some("A") => results
            .into_iter()
            .filter(|r| !r.code.ends_with(".HK") && !r.code.ends_with(".US"))
            .collect(),
        Some("HK") => results.into_iter().filter(|r| r.code.ends_with(".HK")).collect(),
        Some("US") => results.into_iter().filter(|r| r.code.ends_with(".US")).collect(),
        _ => results,
    };
    Ok(filtered)
}

/// 搜索财经新闻
///
/// 复用 AStockClient::search_news 已有链路：多 vendor 路由（eastmoney/akshare/neodata）
/// + 自动去重入库（news_archive 表）+ as-of 模式查本地语料库。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "搜索财经新闻")]
#[tauri::command]
pub async fn search_news(
    state: State<'_, AppState>,
    keyword: String,
    limit: Option<u32>,
) -> Result<Vec<axagent_astock_data::NewsItem>, String> {
    if keyword.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("keyword 不能为空").into());
    }
    let limit = limit.unwrap_or(20).min(100);
    state.astock_client.search_news(&keyword, limit).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("搜索新闻失败: {e}")).to_string()
    })
}

/// 获取社交舆情数据（股吧/雪球热度）
///
/// 返回指定股票在社交平台上的讨论热度、情感倾向等数据。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取社交舆情数据")]
#[tauri::command]
pub async fn get_social_sentiment(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Vec<axagent_astock_data::SocialSentiment>, String> {
    if stock_code.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("stock_code 不能为空").into());
    }
    state.astock_client.get_social_sentiment(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取社交舆情失败: {e}"))
            .to_string()
    })
}

/// 获取实时行情
///
/// spec §4.1: `as_of_date` 非空时,所有 vendor 调用以"截至该日"语义截断,
/// 并在 task_local 中标记 `AsOfContext`,让上层 LLM / 缓存 / 校验能感知。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取实时行情")]
#[tauri::command]
pub async fn get_stock_quote(
    state: State<'_, AppState>,
    stock_code: String,
    as_of_date: Option<String>,
) -> Result<axagent_astock_data::StockQuote, String> {
    // 修复 L-12: 参数非空校验，避免空代码触发无意义的 vendor 请求
    if stock_code.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("stock_code 不能为空").into());
    }
    let as_of_ctx = AsOfContext::parse_optional(as_of_date.as_deref()).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("as_of_date 解析失败: {e}"))
    })?;
    axagent_astock_data::as_of::with_optional_asof(as_of_ctx, async {
        axagent_astock_data::as_of::with_degradation_log(async {
            state.astock_client.get_quote(&stock_code).await.map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("获取实时行情失败: {e}"))
                    .to_string()
            })
        })
        .await
    })
    .await
}

/// 获取K线数据
///
/// spec §4.1: K 线在 as-of 模式下保留 date <= as_of_date 的行(live 模式原样返回)。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取K线数据")]
#[tauri::command]
pub async fn get_stock_kline(
    state: State<'_, AppState>,
    stock_code: String,
    period: String,
    limit: u32,
    as_of_date: Option<String>,
    adj: Option<String>,
) -> Result<Vec<axagent_astock_data::KLine>, String> {
    // 修复 L-12: 参数非空校验，避免空值触发无意义的 vendor 请求
    if stock_code.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("stock_code 不能为空").into());
    }
    if period.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("period 不能为空").into());
    }
    let as_of_ctx = AsOfContext::parse_optional(as_of_date.as_deref()).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("as_of_date 解析失败: {e}"))
    })?;
    let adj_type = match adj.as_deref() {
        None | Some("") | Some("auto") => None,
        Some("none") | Some("forward") | Some("backward") => {
            let parsed: axagent_astock_data::types::AdjType =
                serde_json::from_value(serde_json::Value::String(adj.unwrap())).map_err(|e| {
                    ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("adj 解析失败: {e}"))
                })?;
            Some(parsed)
        },
        Some(other) => {
            return Err(ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("adj 必须是 none/forward/backward/auto, 收到: {other}"))
                .into());
        },
    };
    axagent_astock_data::as_of::with_optional_asof(as_of_ctx, async {
        axagent_astock_data::as_of::with_degradation_log(async {
            state
                .astock_client
                .get_klines_with_adj(&stock_code, &period, limit, adj_type)
                .await
                .map_err(|e| {
                    ErrorResponse::new(wf_err::INTERNAL)
                        .with_detail(format!("获取 K 线数据失败: {e}"))
                        .to_string()
                })
        })
        .await
    })
    .await
}

// ── G1 跨市场数据接入：美股/港股/外汇/基准指数 Tauri 命令 ──

/// 获取国际股票（美股/港股）实时行情
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取国际股票行情")]
#[tauri::command]
pub async fn get_international_stock_quote(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<StockQuote, String> {
    if stock_code.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("stock_code 不能为空").into());
    }
    state.astock_client.get_international_quote(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取国际股票行情失败: {e}"))
            .to_string()
    })
}

/// 获取国际股票（美股/港股）K 线
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取国际股票K线")]
#[tauri::command]
pub async fn get_international_stock_kline(
    state: State<'_, AppState>,
    stock_code: String,
    period: String,
    limit: u32,
) -> Result<Vec<KLine>, String> {
    if stock_code.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("stock_code 不能为空").into());
    }
    state.astock_client.get_international_klines(&stock_code, &period, limit, None).await.map_err(
        |e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("获取国际股票 K 线失败: {e}"))
                .to_string()
        },
    )
}

/// 获取基准指数 K 线（标普 500 / 纳指 / 恒生 / 上证等）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取基准指数K线")]
#[tauri::command]
pub async fn get_benchmark_kline(
    state: State<'_, AppState>,
    benchmark_code: String,
    period: String,
    limit: u32,
) -> Result<Vec<KLine>, String> {
    if benchmark_code.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail("benchmark_code 不能为空")
            .into());
    }
    state.astock_client.get_benchmark_klines(&benchmark_code, &period, limit).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取基准指数 K 线失败: {e}"))
            .to_string()
    })
}

/// 获取外汇 K 线（USD/CNY、HKD/CNY 等）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取外汇K线")]
#[tauri::command]
pub async fn get_forex_kline(
    state: State<'_, AppState>,
    pair: String,
    period: String,
    limit: u32,
) -> Result<Vec<KLine>, String> {
    if pair.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail("pair 不能为空（如 USD/CNY）")
            .into());
    }
    state.astock_client.get_forex_klines(&pair, &period, limit).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取外汇 K 线失败: {e}"))
            .to_string()
    })
}

// ── Phase 2: TradingAgents-CN 优势借鉴 — 批量 + 基本面报告 + 缓存统计 ──

/// 批量请求参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchQuotesRequest {
    pub codes: Vec<String>,
    /// 单只超时（毫秒），默认 8000
    pub per_stock_timeout_ms: Option<u64>,
    /// 总超时（毫秒），默认 30000
    pub total_timeout_ms: Option<u64>,
    /// 允许失败数（0 = 全部必须成功），默认 0
    pub max_failures: Option<usize>,
}

/// 批量获取实时行情（DataFrame 风格）
///
/// 内部并发调 `get_quote`,受 `DomainGate` 限流。
/// 失败股票不阻塞,集中在 `failures` 字段返回。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "批量获取实时行情")]
#[tauri::command]
pub async fn batch_get_quotes(
    state: State<'_, AppState>,
    request: BatchQuotesRequest,
) -> Result<BatchResult<StockQuote>, String> {
    use std::time::Duration;

    let mut req = BatchRequest::new(request.codes);
    if let Some(ms) = request.per_stock_timeout_ms {
        req = req.with_per_stock_timeout(Duration::from_millis(ms));
    }
    if let Some(ms) = request.total_timeout_ms {
        req = req.with_total_timeout(Duration::from_millis(ms));
    }
    if let Some(n) = request.max_failures {
        req = req.with_max_failures(n);
    }

    let client = state.astock_client.clone();
    let runner = BatchRunner::new(client);
    Ok(runner.get_quotes_batch(req).await)
}

/// 批量获取 K 线
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "批量获取K线")]
#[tauri::command]
pub async fn batch_get_klines(
    state: State<'_, AppState>,
    codes: Vec<String>,
    period: String,
    limit: u32,
) -> Result<BatchResult<Vec<axagent_astock_data::KLine>>, String> {
    // 修复 M-DEF-6: 限制批量 codes 数量 <= 50，防止恶意调用方一次性
    // 发起上百个 vendor 请求耗尽信号量池。
    const MAX_CODES: usize = 50;
    if codes.len() > MAX_CODES {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!(
                "batch_get_klines codes 数量 {} 超过上限 {}，请分批调用",
                codes.len(),
                MAX_CODES
            ))
            .into());
    }
    let client = state.astock_client.clone();
    let runner = BatchRunner::new(client);
    Ok(runner.get_klines_batch(codes, &period, limit).await)
}

/// 批量获取财务数据
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "批量获取财务数据")]
#[tauri::command]
pub async fn batch_get_financials(
    state: State<'_, AppState>,
    codes: Vec<String>,
) -> Result<BatchResult<Vec<FinancialReport>>, String> {
    let client = state.astock_client.clone();
    let runner = BatchRunner::new(client);
    Ok(runner.get_financials_batch(codes).await)
}

/// 基本面报告请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsReportRequest {
    pub stock_code: String,
    /// 是否同时返回 Markdown 渲染
    pub include_markdown: Option<bool>,
}

/// 生成基本面分析报告（PE/PB/ROE/同比/估值带/健康度评分）
///
/// 喂给工作流 a-fundamentals 节点使用:
/// 报告把 `FinancialReport` + 实时行情聚合成可读结构 + 0-100 健康度评分。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "生成基本面分析报告")]
#[tauri::command]
pub async fn generate_fundamentals_report(
    state: State<'_, AppState>,
    request: FundamentalsReportRequest,
) -> Result<FundamentalsReportEnvelope, String> {
    let include_md = request.include_markdown.unwrap_or(true);

    // 1. 拉取实时行情
    let quote = state.astock_client.get_quote(&request.stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取实时行情失败: {e}"))
    })?;

    // 2. 拉取财务数据（按时间倒序,首项为最新）
    let financials =
        state.astock_client.get_financials(&request.stock_code).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取财务数据失败: {e}"))
        })?;

    // 3. 生成报告
    let report = FundamentalsAnalyzer::generate(&request.stock_code, &quote, &financials);
    let markdown = if include_md {
        Some(report.to_markdown())
    } else {
        None
    };

    // 4. 拼装返回（含可选 Markdown）
    Ok(FundamentalsReportEnvelope { report, markdown })
}

/// 基本面报告包装（含可选 Markdown）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FundamentalsReportEnvelope {
    pub report: FundamentalsReport,
    pub markdown: Option<String>,
}

/// 仅获取 Markdown 渲染（轻量,不返回结构）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取基本面报告Markdown")]
#[tauri::command]
pub async fn get_fundamentals_report_markdown(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<String, String> {
    let quote = state.astock_client.get_quote(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取实时行情失败: {e}"))
    })?;
    let financials = state.astock_client.get_financials(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取财务数据失败: {e}"))
    })?;
    let report = FundamentalsAnalyzer::generate(&stock_code, &quote, &financials);
    Ok(report.to_markdown())
}

/// 取消分析 — 设置取消令牌让后台任务停止
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "取消股票分析任务")]
#[tauri::command]
pub async fn cancel_stock_analysis(
    state: State<'_, AppState>,
    analysis_id: String,
) -> Result<(), String> {
    let tokens = std::sync::Arc::new(state.agent_cancel_tokens.clone());
    if let Some(token) = tokens.get(&analysis_id) {
        token.store(true, Ordering::Relaxed);
        tracing::info!("cancel_stock_analysis: 已设置取消令牌 {}", analysis_id);
        Ok(())
    } else {
        Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("分析任务不存在或已完成: {}", analysis_id))
            .into())
    }
}

/// 历史分析列表精简 DTO（列表场景专用）
///
/// 与前端 `AnalysisSummary` interface 对齐，仅包含列表渲染必要字段。
/// 排除 `blackboard_snapshot` / `llm_decision_json` / `decision_reasoning` 等大字段，
/// 这些字段在详情页通过 `get_stock_analysis` 单独获取。
/// 收益：单条记录从 KB 级降至百字节级，列表加载性能提升 80%+。
#[derive(Debug, sea_orm::FromQueryResult, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockAnalysisListItem {
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    pub analysis_date: String,
    pub status: String,
    pub decision_action: Option<String>,
    pub decision_position_pct: Option<f64>,
    pub decision_json: Option<String>,
    pub analysis_kind: String,
    pub as_of_date: Option<String>,
    pub parent_analysis_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 历史分析列表
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取历史分析列表")]
#[tauri::command]
pub async fn list_stock_analyses(
    state: State<'_, AppState>,
    limit: u32,
    offset: u32,
) -> Result<Vec<StockAnalysisListItem>, String> {
    // 修复 M-DEF-6: 限制 limit <= 1000，防止恶意大 limit 拖垮 DB / 内存。
    const MAX_LIMIT: u32 = 1000;
    let limit = if limit > MAX_LIMIT {
        tracing::warn!("list_stock_analyses limit={} 超过上限 {}，自动截断", limit, MAX_LIMIT);
        MAX_LIMIT
    } else {
        limit
    };
    // 精简字段查询：仅 SELECT 列表渲染必要列，排除 blackboard_snapshot 等大字段
    use sea_orm::QuerySelect;
    let rows: Vec<StockAnalysisListItem> = stock_analyses::Entity::find()
        .select_only()
        .column(stock_analyses::Column::Id)
        .column(stock_analyses::Column::StockCode)
        .column(stock_analyses::Column::StockName)
        .column(stock_analyses::Column::AnalysisDate)
        .column(stock_analyses::Column::Status)
        .column(stock_analyses::Column::DecisionAction)
        .column(stock_analyses::Column::DecisionPositionPct)
        .column(stock_analyses::Column::DecisionJson)
        .column(stock_analyses::Column::AnalysisKind)
        .column(stock_analyses::Column::AsOfDate)
        .column(stock_analyses::Column::ParentAnalysisId)
        .column(stock_analyses::Column::CreatedAt)
        .column(stock_analyses::Column::UpdatedAt)
        .order_by_desc(stock_analyses::Column::CreatedAt)
        .limit(Some(limit as u64))
        .offset(Some(offset as u64))
        .into_model::<StockAnalysisListItem>()
        .all(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("查询历史分析列表失败: {e}"))
                .to_string()
        })?;
    Ok(rows)
}

/// 获取单个分析详情
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取单个分析详情")]
#[tauri::command]
pub async fn get_stock_analysis(
    state: State<'_, AppState>,
    analysis_id: String,
) -> Result<stock_analyses::Model, String> {
    stock_analyses::Entity::find_by_id(&analysis_id)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询分析详情失败: {e}"))
        })?
        .ok_or_else(|| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("分析记录不存在: {}", analysis_id))
                .to_string()
        })
}

/// 删除历史分析记录
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "删除历史分析记录")]
#[tauri::command]
pub async fn delete_stock_analysis(
    state: State<'_, AppState>,
    analysis_id: String,
) -> Result<(), String> {
    stock_analyses::Entity::delete_by_id(&analysis_id).exec(state.harness.db()).await.map_err(
        |e| ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("删除分析记录失败: {e}")),
    )?;
    Ok(())
}

/// 批量删除历史分析记录
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "批量删除历史分析记录")]
#[tauri::command]
pub async fn batch_delete_stock_analyses(
    state: State<'_, AppState>,
    analysis_ids: Vec<String>,
) -> Result<(), String> {
    // 修复 M-DEF-6: 限制批量删除 ids 数量 <= 100，防止一次性提交过大
    // 事务导致 SQLite 锁定过久 / 内存占用过高。
    const MAX_IDS: usize = 100;
    if analysis_ids.len() > MAX_IDS {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!(
                "batch_delete_stock_analyses ids 数量 {} 超过上限 {}，请分批调用",
                analysis_ids.len(),
                MAX_IDS
            ))
            .into());
    }
    let db = state.harness.db();
    for id in &analysis_ids {
        stock_analyses::Entity::delete_by_id(id).exec(db).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("批量删除分析记录失败: {e}"))
        })?;
    }
    Ok(())
}

/// 重命名历史分析记录
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "重命名历史分析记录")]
#[tauri::command]
pub async fn rename_stock_analysis(
    state: State<'_, AppState>,
    analysis_id: String,
    new_name: String,
) -> Result<(), String> {
    use sea_orm::ActiveModelTrait;
    use sea_orm::Set;
    let mut record: stock_analyses::ActiveModel = stock_analyses::Entity::find_by_id(&analysis_id)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询分析记录失败: {e}"))
        })?
        .ok_or_else(|| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("分析记录不存在: {analysis_id}"))
        })?
        .into();
    record.stock_name = Set(new_name);
    record.update(state.harness.db()).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("更新分析记录失败: {e}"))
    })?;
    Ok(())
}

// ── Watchlist ──

/// 添加自选股
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "添加自选股")]
#[tauri::command]
pub async fn add_to_watchlist(
    state: State<'_, AppState>,
    stock_code: String,
    stock_name: String,
    notes: Option<String>,
) -> Result<watchlist_items::Model, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let model = watchlist_items::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        stock_code: Set(stock_code),
        stock_name: Set(stock_name),
        // R2-Bug-B1 修复: 之前硬编码 Set(None),前端传来的 group 信息全丢
        notes: Set(notes),
        created_at: Set(now),
        updated_at: Set(now),
    };
    model.insert(state.harness.db()).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("添加自选股失败: {e}")).to_string()
    })
}

/// 移除自选股
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "移除自选股")]
#[tauri::command]
pub async fn remove_from_watchlist(state: State<'_, AppState>, id: String) -> Result<(), String> {
    watchlist_items::Entity::delete_by_id(id).exec(state.harness.db()).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("移除自选股失败: {e}"))
    })?;
    Ok(())
}

/// 更新自选股的分组归属
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "更新自选股分组")]
#[tauri::command]
pub async fn watchlist_update_group(
    state: State<'_, AppState>,
    id: String,
    group_name: String,
) -> Result<(), String> {
    let item = watchlist_items::Entity::find_by_id(&id)
        .one(state.harness.db())
        .await
        .map_err(|e| format!("查询自选股失败: {e}"))?
        .ok_or_else(|| format!("自选股 {id} 不存在"))?;

    let mut notes_json: serde_json::Value = item
        .notes
        .as_deref()
        .and_then(|n| serde_json::from_str(n).ok())
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    if let Some(obj) = notes_json.as_object_mut() {
        obj.insert("group".into(), serde_json::Value::String(group_name));
    }

    let mut active: watchlist_items::ActiveModel = item.into();
    active.notes = Set(Some(notes_json.to_string()));
    active.updated_at = Set(chrono::Utc::now().timestamp_millis());
    active.update(state.harness.db()).await.map_err(|e| format!("更新自选股分组失败: {e}"))?;
    Ok(())
}

/// 获取所有分组列表（从 settings 表读取）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取分组列表")]
#[tauri::command]
pub async fn watchlist_list_groups(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    use axagent_entities::settings;
    let setting = settings::Entity::find_by_id("watchlist_groups")
        .one(state.harness.db())
        .await
        .map_err(|e| format!("查询分组设置失败: {e}"))?;

    match setting {
        Some(s) => Ok(serde_json::from_str(&s.value).unwrap_or_default()),
        None => Ok(vec![]),
    }
}

/// 保存分组列表（写入 settings 表）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "保存分组列表")]
#[tauri::command]
pub async fn watchlist_save_groups(
    state: State<'_, AppState>,
    groups: Vec<String>,
) -> Result<(), String> {
    use axagent_entities::settings;
    let value = serde_json::to_string(&groups).map_err(|e| format!("序列化分组失败: {e}"))?;

    // upsert
    let existing = settings::Entity::find_by_id("watchlist_groups")
        .one(state.harness.db())
        .await
        .map_err(|e| format!("查询设置失败: {e}"))?;

    if let Some(s) = existing {
        let mut active: settings::ActiveModel = s.into();
        active.value = Set(value);
        active.update(state.harness.db()).await.map_err(|e| format!("更新分组设置失败: {e}"))?;
    } else {
        let active =
            settings::ActiveModel { key: Set("watchlist_groups".into()), value: Set(value) };
        active.insert(state.harness.db()).await.map_err(|e| format!("创建分组设置失败: {e}"))?;
    }
    Ok(())
}

/// 自选股列表
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取自选股列表")]
#[tauri::command]
pub async fn list_watchlist(
    state: State<'_, AppState>,
) -> Result<Vec<watchlist_items::Model>, String> {
    watchlist_items::Entity::find()
        .order_by_desc(watchlist_items::Column::CreatedAt)
        .all(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("查询自选股列表失败: {e}"))
                .to_string()
        })
}

/// 提取证据引用链（审计溯源）
///
/// 从指定股票分析的 `decision_json` + `blackboard_snapshot` 中
/// 提取每条决策理由的来源分析师报告引用。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "提取证据引用链")]
#[tauri::command]
pub async fn extract_evidence_citations(
    state: State<'_, AppState>,
    analysis_id: String,
) -> Result<axagent_analysis_engine::evidence_citation::CitationReport, String> {
    use axagent_analysis_engine::evidence_citation::extract_citations;

    let analysis = axagent_entities::stock_analyses::Entity::find_by_id(&analysis_id)
        .one(state.harness.db())
        .await
        .map_err(|e| format!("查询分析记录失败: {e}"))?
        .ok_or_else(|| format!("分析记录 {analysis_id} 不存在"))?;

    let reasoning = analysis.decision_reasoning.unwrap_or_default();
    let snapshot = analysis.blackboard_snapshot.unwrap_or_else(|| "{}".into());

    let mut report = extract_citations(&reasoning, &snapshot);
    report.stock_code = analysis.stock_code;
    report.stock_name = analysis.stock_name;
    report.analysis_date = analysis.analysis_date;
    report.decision_action = analysis.decision_action.unwrap_or_default();
    report.decision_confidence = 0.0; // 从 decision_json 解析

    // 尝试从 decision_json 解析置信度
    if let Some(dj) = &analysis.decision_json {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(dj) {
            if let Some(conf) = parsed.get("confidence").and_then(|v| v.as_f64()) {
                report.decision_confidence = conf;
            }
        }
    }

    Ok(report)
}

// ── 条件单 ──

/// 获取所有条件单
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取条件单列表")]
#[tauri::command]
pub async fn conditional_order_list(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_analysis_engine::conditional_order::ConditionalOrder>, String> {
    use axagent_entities::settings;
    let setting = settings::Entity::find_by_id("conditional_orders")
        .one(state.harness.db())
        .await
        .map_err(|e| format!("查询条件单失败: {e}"))?;

    match setting {
        Some(s) => Ok(serde_json::from_str(&s.value).unwrap_or_default()),
        None => Ok(vec![]),
    }
}

/// 保存条件单列表
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "保存条件单列表")]
#[tauri::command]
pub async fn conditional_order_save(
    state: State<'_, AppState>,
    orders: Vec<axagent_analysis_engine::conditional_order::ConditionalOrder>,
) -> Result<(), String> {
    use axagent_entities::settings;
    let value = serde_json::to_string(&orders).map_err(|e| format!("序列化条件单失败: {e}"))?;

    let existing = settings::Entity::find_by_id("conditional_orders")
        .one(state.harness.db())
        .await
        .map_err(|e| format!("查询设置失败: {e}"))?;

    if let Some(s) = existing {
        let mut active: settings::ActiveModel = s.into();
        active.value = Set(value);
        active.update(state.harness.db()).await.map_err(|e| format!("更新条件单失败: {e}"))?;
    } else {
        let active =
            settings::ActiveModel { key: Set("conditional_orders".into()), value: Set(value) };
        active.insert(state.harness.db()).await.map_err(|e| format!("保存条件单失败: {e}"))?;
    }
    Ok(())
}

/// 评估条件单（测试用）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "评估条件单")]
#[tauri::command]
pub async fn conditional_order_evaluate(
    _stock_code: String,
    _current_price: f64,
    _prev_close: f64,
    _turnover_rate: Option<f64>,
) -> Result<Vec<serde_json::Value>, String> {
    use axagent_analysis_engine::conditional_order::ConditionalOrderEngine;

    // 简化版：本地评估（不依赖 DB），返回匹配结果
    let _engine = ConditionalOrderEngine::new();
    // 这里仅做框架示例，实际评估由后台引擎完成
    Ok(vec![])
}

/// 生成月度投资报告
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "生成月度投资报告")]
#[tauri::command]
pub async fn generate_monthly_report(
    state: State<'_, AppState>,
    year: i32,
    month: u32,
) -> Result<axagent_analysis_engine::monthly_report::MonthlyReport, String> {
    axagent_analysis_engine::monthly_report::generate_monthly_report(
        state.harness.db(),
        year,
        month,
    )
    .await
}

/// 获取情绪深度分析报告
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "分析情绪深度")]
#[tauri::command]
pub async fn analyze_sentiment_depth(
    stock_code: String,
    stock_name: String,
    history_json: String,
) -> Result<axagent_analysis_engine::sentiment_analysis::SentimentReport, String> {
    let history: Vec<axagent_analysis_engine::sentiment_analysis::SentimentSnapshot> =
        serde_json::from_str(&history_json).map_err(|e| format!("解析历史数据失败: {e}"))?;
    Ok(axagent_analysis_engine::sentiment_analysis::analyze_sentiment(
        &stock_code,
        &stock_name,
        &history,
    ))
}

/// 运行组合回测（简化版 — 调用 quant crate）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "运行组合回测")]
#[tauri::command]
pub async fn portfolio_backtest_run(
    _state: State<'_, AppState>,
    config_json: String,
) -> Result<serde_json::Value, String> {
    let config: axagent_quant::PortfolioConfig =
        serde_json::from_str(&config_json).map_err(|e| format!("解析组合配置失败: {e}"))?;
    let _engine = axagent_quant::PortfolioEngine::new(config.clone()).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    // 返回配置校验结果（实际执行需要前端传入K线）
    Ok(serde_json::json!({
        "status": "config_valid",
        "strategies": config.strategies.len(),
        "message": "配置校验通过。回测执行需要前端传入K线数据。",
    }))
}

/// 获取因子分析结果
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取因子分析结果")]
#[tauri::command]
pub async fn factor_analysis_list() -> Result<Vec<serde_json::Value>, String> {
    let registry = axagent_analysis_engine::factor_analysis::FactorRegistry::new();
    let factors = registry.all_factors();
    Ok(factors
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.id,
                "name": f.name,
                "category": format!("{:?}", f.category),
                "higherIsBetter": f.higher_is_better,
                "defaultWeight": f.default_weight,
                "enabled": f.enabled,
            })
        })
        .collect())
}

/// 获取宏观经济数据快照
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取宏观数据快照")]
#[tauri::command]
pub async fn macro_data_snapshot() -> Result<axagent_astock_data::MacroDataSnapshot, String> {
    let client = axagent_astock_data::macro_data::MacroDataClient::new();
    Ok(client.snapshot().await)
}

// ── Portfolio ──

/// 添加持仓
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "添加持仓")]
#[tauri::command]
pub async fn add_portfolio_holding(
    state: State<'_, AppState>,
    stock_code: String,
    stock_name: String,
    shares: f64,
    avg_cost: f64,
) -> Result<portfolio_holdings::Model, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let model = portfolio_holdings::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        stock_code: Set(stock_code),
        stock_name: Set(stock_name),
        shares: Set(shares),
        avg_cost: Set(avg_cost),
        notes: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    model.insert(state.harness.db()).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("添加持仓失败: {e}")).to_string()
    })
}

/// 更新持仓
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "更新持仓")]
#[tauri::command]
pub async fn update_portfolio_holding(
    state: State<'_, AppState>,
    id: String,
    shares: f64,
    avg_cost: f64,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    portfolio_holdings::Entity::update_many()
        .col_expr(portfolio_holdings::Column::Shares, Expr::value(shares))
        .col_expr(portfolio_holdings::Column::AvgCost, Expr::value(avg_cost))
        .col_expr(portfolio_holdings::Column::UpdatedAt, Expr::value(now))
        .filter(portfolio_holdings::Column::Id.eq(id))
        .exec(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("更新持仓失败: {e}"))
        })?;
    Ok(())
}

/// 移除持仓
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "移除持仓")]
#[tauri::command]
pub async fn remove_portfolio_holding(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    portfolio_holdings::Entity::delete_by_id(id).exec(state.harness.db()).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("移除持仓失败: {e}"))
    })?;
    Ok(())
}

/// 持仓列表（含实时盈亏）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "获取持仓列表")]
#[tauri::command]
pub async fn list_portfolio(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let holdings =
        portfolio_holdings::Entity::find().all(state.harness.db()).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询持仓列表失败: {e}"))
        })?;

    let client = state.astock_client.clone();
    let codes: Vec<String> = holdings.iter().map(|h| h.stock_code.clone()).collect();
    let mut quote_tasks = tokio::task::JoinSet::new();
    for code in codes {
        let c = client.clone();
        quote_tasks.spawn(async move {
            let quote = c.get_quote(&code).await.ok();
            (code, quote)
        });
    }
    let mut quotes = std::collections::HashMap::new();
    while let Some(result) = quote_tasks.join_next().await {
        if let Ok((code, quote)) = result {
            quotes.insert(code, quote);
        }
    }

    let enriched: Vec<serde_json::Value> = holdings
        .into_iter()
        .map(|h| {
            let quote = quotes.get(&h.stock_code).and_then(|q| q.as_ref());
            let current_price = quote.map(|q| q.price).unwrap_or(h.avg_cost);
            let market_value = current_price * h.shares;
            let cost_basis = h.avg_cost * h.shares;
            let pnl = market_value - cost_basis;
            let pnl_pct = if cost_basis != 0.0 {
                (pnl / cost_basis) * 100.0
            } else {
                0.0
            };

            serde_json::json!({
                "id": h.id,
                "stockCode": h.stock_code,
                "stockName": h.stock_name,
                "shares": h.shares,
                "avgCost": h.avg_cost,
                "currentPrice": current_price,
                "marketValue": market_value,
                "pnl": pnl,
                "pnlPct": pnl_pct,
                "notes": h.notes,
                "createdAt": h.created_at,
            })
        })
        .collect();
    Ok(enriched)
}

/// 从 settings 表加载估值参数（ValueConfig），仅提取需要的部分
async fn load_value_config(
    db: &sea_orm::DatabaseConnection,
) -> axagent_analysis_engine::decision::ValueConfig {
    if let Ok(Some(v)) = axagent_dao::repo::settings::get_setting(db, "stock_analysis_config").await
    {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&v) {
            if let Some(value_section) = parsed.get("value") {
                if let Ok(cfg) = serde_json::from_value::<
                    axagent_analysis_engine::decision::ValueConfig,
                >(value_section.clone())
                {
                    return cfg;
                }
            }
        }
    }
    axagent_analysis_engine::decision::ValueConfig::default()
}

// ── MCP Stock Data Tools ──

/// 返回 stock data MCP 工具定义列表（供前端 MCP 管理页面注册）
///
/// P2-8: 合并 `axagent_astock_data::mcp_tools::stock_mcp_tools()` 与
/// `axagent_analysis_engine::mcp_tools::industry_chain_mcp_tools()`。
/// G3 产业链工具已从 astock-data 迁回 stock-analysis，需在此合并返回。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取股票MCP工具列表")]
#[tauri::command]
pub async fn get_stock_mcp_tools() -> Result<Vec<serde_json::Value>, String> {
    let mut tools = axagent_astock_data::mcp_tools::stock_mcp_tools();
    tools.extend(axagent_analysis_engine::mcp_tools::industry_chain_mcp_tools());
    Ok(tools)
}

/// 执行 stock data MCP 工具调用
///
/// P2-8: 先尝试 G3 产业链工具（位于 stock-analysis crate），未匹配则回退到 astock-data。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "执行股票MCP工具调用")]
#[tauri::command]
pub async fn execute_stock_mcp_tool(
    state: State<'_, AppState>,
    tool_name: String,
    arguments: serde_json::Value,
) -> Result<String, String> {
    // P2-8: G3 产业链工具优先（纯计算，不依赖 astock_client）
    if axagent_analysis_engine::mcp_tools::is_industry_chain_tool(&tool_name) {
        return axagent_analysis_engine::mcp_tools::execute_industry_chain_tool(
            &tool_name, &arguments,
        );
    }

    // 为 compute_valuation 工具注入估值参数配置
    let arguments = if tool_name == "compute_valuation" {
        let params = get_valuation_params_inner(&state).await;
        let config = serde_json::json!({
            "perpetualGrowth": params.perpetual_growth,
            "discountRate": params.discount_rate,
            "defaultGrowth": params.default_growth,
            "minGrowth": params.min_growth,
            "maxGrowth": params.max_growth,
            "forecastYears": params.forecast_years,
            "bondYield": params.bond_yield,
        });
        let mut args = arguments;
        if let Some(obj) = args.as_object_mut() {
            obj.insert("valuation_config".to_string(), config);
        }
        args
    } else {
        arguments
    };

    axagent_astock_data::mcp_tools::execute_mcp_tool(&state.astock_client, &tool_name, &arguments)
        .await
}

// ── Backtesting ──

/// Bug 6 修复: 把 action → strategy_id 的映射抽成可复用函数,集中维护。
///
/// 之前在 backtest_analysis 里 inline 写死了 5 行 match,既无法测试,
/// 改策略时容易漏改。这里抽成 free function,接受标准化后的 action_token
/// (大写英文,前后空白已 trim),所有未识别的 action 都回退到 "watchlist"。
pub(crate) fn map_action_to_strategy_id(action: &str) -> &'static str {
    // 注意大小写不敏感(既支持中文,也支持 BUY/Hold 等英文)
    match action.trim().to_ascii_uppercase().as_str() {
        "买入" | "BUY" | "增持" | "INCREASE" => "trend",
        "卖出" | "SELL" | "减持" | "REDUCE" => "reversion",
        "持有" | "HOLD" => "value",
        "观望" | "UNCERTAIN" => "capital",
        _ => "watchlist",
    }
}

/// 回测单个分析决策
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "回测单个分析决策")]
#[tauri::command]
pub async fn backtest_analysis(
    state: State<'_, AppState>,
    stock_code: String,
    analysis_date: String,
    decision_action: String,
    decision_confidence: f64,
    holding_days: u32,
    as_of_date: Option<String>,
) -> Result<BacktestResult, String> {
    let ctx = AsOfContext::parse_optional(as_of_date.as_deref())?;
    let result = axagent_astock_data::as_of::with_optional_asof(ctx, async {
        BacktestEngine::backtest_decision(
            &*state.astock_client,
            &stock_code,
            &analysis_date,
            &decision_action,
            decision_confidence,
            holding_days as i64,
            None,
            None,
        )
        .await
    })
    .await?;

    // 写入 strategy_performance 表，让分析回测参与权重进化
    let strategy_id = map_action_to_strategy_id(&decision_action);
    let decision_ms = chrono::NaiveDate::parse_from_str(&analysis_date, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| d.and_utc().timestamp_millis())
        .unwrap_or(0);
    let period = result.time_horizon.as_deref().unwrap_or("short");
    let _ = axagent_analysis_engine::evolution_drift::record_performance(
        state.harness.db(),
        strategy_id,
        period,
        &stock_code,
        "",
        decision_ms,
        chrono::Utc::now().timestamp_millis(),
        result.holding_days as i32,
        result.return_pct,
        if result.was_correct { 1 } else { 0 },
        decision_confidence as i32,
        None,
        None, // agreement_score: 回测无 LLM 对比
    )
    .await
    .map_err(|e| tracing::warn!("[backtest_analysis] record_performance 失败: {e}"))
    .ok();

    Ok(result)
}

/// 批量回测历史分析（已完成的分析）
///
/// `scope`:
/// - `"all"` (默认): 所有 completed 分析(live + replay)
/// - `"live"`: 仅 live 模式分析(实时分析的回测准确率)
/// - `"replay"`: 仅 replay 模式分析(回放分析的真实回测)
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "批量回测历史分析")]
#[tauri::command]
pub async fn backtest_all_history(
    state: State<'_, AppState>,
    holding_days: u32,
    scope: Option<String>,
) -> Result<BacktestStats, String> {
    let scope = scope.unwrap_or_else(|| "all".to_string());

    let mut query =
        stock_analyses::Entity::find().filter(stock_analyses::Column::Status.eq("completed"));
    query = match scope.as_str() {
        "live" => query.filter(stock_analyses::Column::AnalysisKind.eq("live")),
        "replay" => query.filter(stock_analyses::Column::AnalysisKind.eq("replay")),
        _ => query, // "all" 或未知值 = 不过滤
    };
    let analyses = query.all(state.harness.db()).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询历史分析失败: {e}"))
    })?;

    let historical: Vec<HistoricalAnalysis> = analyses
        .iter()
        .map(|a| {
            let confidence = a
                .decision_json
                .as_ref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|v| v.get("confidence").and_then(|c| c.as_f64()))
                .unwrap_or(0.5);
            HistoricalAnalysis {
                stock_code: a.stock_code.clone(),
                analysis_date: a.analysis_date.clone(),
                decision_action: a.decision_action.clone().unwrap_or_else(|| "持有".to_string()),
                decision_confidence: confidence,
                time_horizon: a.decision_time_horizon.clone(),
                expected_holding_days: a.decision_expected_holding_days,
            }
        })
        .collect();

    // 默认持有期参数保留向后兼容，backtest_history 内部会优先使用每条记录的个性化持有期
    let results =
        BacktestEngine::backtest_history(&*state.astock_client, historical, holding_days).await?;
    let stats = BacktestEngine::compute_stats(&results);
    Ok(stats)
}

// ── Replay Sweep (spec §5 Step 8, §9.3) ──

/// 单条 sweep 项：(代码, as-of 截止日, 假设决策, 置信度, 时间维度, 期望持有天数)
#[derive(serde::Deserialize, Debug, Clone)]
pub struct ReplaySweepItem {
    pub stock_code: String,
    pub as_of_date: String,
    pub decision_action: String,
    pub decision_confidence: f64,
    #[serde(default)]
    pub time_horizon: Option<String>,
    #[serde(default)]
    pub expected_holding_days: Option<i64>,
}

/// Sweep 中失败的样本 + 失败原因
#[derive(serde::Serialize, Debug, Clone)]
pub struct ReplaySweepInvalid {
    pub stock_code: String,
    pub as_of_date: String,
    pub reason: String,
}

/// Sweep 结果汇总
#[derive(serde::Serialize, Debug, Clone)]
pub struct ReplaySweepResult {
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
    pub results: Vec<BacktestResult>,
    pub invalid_details: Vec<ReplaySweepInvalid>,
    pub stats: BacktestStats,
}

/// 批量回放回测（Replay Sweep）
///
/// 对给定的 `(stock_code, as_of_date, decision)` 元组逐个调用
/// `BacktestEngine::backtest_decision`，汇总 valid/invalid 统计与 BacktestStats。
///
/// 注意：
/// - `as_of_date` 必须在过去；前端 `DatePicker` 已约束 `disabledDate={d => d > dayjs()}`。
/// - 此命令不读写 DB，只做计算；与 `backtest_all_history` 互为补充。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "批量回放回测")]
#[tauri::command]
pub async fn run_replay_backtest(
    state: State<'_, AppState>,
    items: Vec<ReplaySweepItem>,
    holding_days: u32,
) -> Result<ReplaySweepResult, String> {
    let total = items.len() as u32;
    let mut results: Vec<BacktestResult> = Vec::new();
    let mut invalid_details: Vec<ReplaySweepInvalid> = Vec::new();

    for item in items {
        // 设置 as_of 上下文，保证 backtest_decision 内部 get_klines 获取的是截止日的 K 线
        let ctx = match AsOfContext::parse_optional(Some(item.as_of_date.as_str())) {
            Ok(c) => c,
            Err(e) => {
                invalid_details.push(ReplaySweepInvalid {
                    stock_code: item.stock_code.clone(),
                    as_of_date: item.as_of_date.clone(),
                    reason: format!("as_of 解析失败: {e}"),
                });
                continue;
            },
        };
        let effective_holding = item.expected_holding_days.unwrap_or(holding_days as i64);
        let result = as_of::with_optional_asof(ctx, async {
            BacktestEngine::backtest_decision(
                &*state.astock_client,
                &item.stock_code,
                &item.as_of_date,
                &item.decision_action,
                item.decision_confidence,
                effective_holding,
                item.time_horizon.clone(),
                item.expected_holding_days,
            )
            .await
        })
        .await;

        match result {
            Ok(r) => results.push(r),
            Err(e) => invalid_details.push(ReplaySweepInvalid {
                stock_code: item.stock_code,
                as_of_date: item.as_of_date,
                reason: e,
            }),
        }
    }

    let stats = BacktestEngine::compute_stats(&results);
    Ok(ReplaySweepResult {
        total,
        valid: results.len() as u32,
        invalid: invalid_details.len() as u32,
        results,
        invalid_details,
        stats,
    })
}

// ── Price Alerts ──

/// 创建价格告警
///
/// v203 后参数语义：
/// - `condition` 仍接收老值（above/below/change_up/change_down/volume_spike），命令内部映射到 6 类 alert_type
/// - `target_price` 在 price 类告警是绝对价，在 change_pct 类是百分比，在 turnover_rate 类是换手率
/// - 新增 `alert_type` + `condition_type` + `threshold` 三列同步写入，新代码读新列
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "创建价格告警")]
#[tauri::command]
pub async fn create_price_alert(
    state: State<'_, AppState>,
    stock_code: String,
    stock_name: String,
    condition: String,
    target_price: f64,
) -> Result<price_alerts::Model, String> {
    use axagent_analysis_engine::alert_mapping::{
        condition_type_for, legacy_condition_to_alert_type,
    };

    // 映射老 condition 到 6 类 alert_type（未知值保守视为 take_profit）
    let alert_type = legacy_condition_to_alert_type(&condition)
        .unwrap_or(axagent_analysis_engine::alert_mapping::alert_types::TAKE_PROFIT);
    let condition_type = condition_type_for(alert_type);
    let threshold = target_price;

    let now = chrono::Utc::now().timestamp_millis();
    let model = price_alerts::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        stock_code: Set(stock_code.clone()),
        stock_name: Set(stock_name.clone()),
        // 老字段同步写入，兼容旧代码读取
        condition: Set(condition.clone()),
        target_price: Set(target_price),
        // 新字段（v203）
        alert_type: Set(Some(alert_type.to_string())),
        condition_type: Set(Some(condition_type.to_string())),
        threshold: Set(Some(threshold)),
        is_triggered: Set(0),
        triggered_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let inserted = model.insert(state.harness.db()).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("创建价格告警失败: {e}"))
            .to_string()
    })?;

    // P0: 同步加入 RealtimeMonitor 配置，即时生效（无需重启）
    if let Some(monitor) = state.stock_monitor.get() {
        use axagent_analysis_engine::monitor::MonitorConfig;
        // 用 alert_type 直接构造 MonitorConfig 的 6 个 Option 字段
        let mut config = MonitorConfig {
            stock_code: stock_code.clone(),
            stock_name: stock_name.clone(),
            stop_loss: None,
            take_profit: None,
            resistance_break: None,
            support_break: None,
            change_pct_alert: None,
            turnover_rate_alert: None,
            enabled: true,
        };
        match alert_type {
            axagent_analysis_engine::alert_mapping::alert_types::STOP_LOSS => {
                config.stop_loss = Some(threshold);
            },
            axagent_analysis_engine::alert_mapping::alert_types::TAKE_PROFIT => {
                config.take_profit = Some(threshold);
            },
            axagent_analysis_engine::alert_mapping::alert_types::RESISTANCE => {
                config.resistance_break = Some(threshold);
            },
            axagent_analysis_engine::alert_mapping::alert_types::SUPPORT => {
                config.support_break = Some(threshold);
            },
            axagent_analysis_engine::alert_mapping::alert_types::CHANGE => {
                config.change_pct_alert = Some(threshold);
            },
            axagent_analysis_engine::alert_mapping::alert_types::VOLUME => {
                config.turnover_rate_alert = Some(threshold);
            },
            _ => {},
        }
        monitor.add_config(config).await;
        tracing::info!(
            "[create_price_alert] 已加入实时监控: {} {} {} → alert_type={} threshold={:.2}",
            stock_code,
            stock_name,
            condition,
            alert_type,
            threshold
        );
    } else {
        tracing::warn!("[create_price_alert] RealtimeMonitor 未初始化，告警仅写入 DB，不会触发");
    }

    Ok(inserted)
}

/// 查询价格告警列表
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "查询价格告警列表")]
#[tauri::command]
pub async fn list_price_alerts(
    state: State<'_, AppState>,
) -> Result<Vec<price_alerts::Model>, String> {
    price_alerts::Entity::find()
        .order_by_desc(price_alerts::Column::CreatedAt)
        .all(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("查询价格告警失败: {e}"))
                .to_string()
        })
}

/// 删除价格告警
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "删除价格告警")]
#[tauri::command]
pub async fn delete_price_alert(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // 先查出 stock_code，以便从 RealtimeMonitor 移除监控
    let alert =
        price_alerts::Entity::find_by_id(&id).one(state.harness.db()).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询价格告警失败: {e}"))
        })?;

    price_alerts::Entity::delete_by_id(id).exec(state.harness.db()).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("删除价格告警失败: {e}"))
    })?;

    // P0: 同步从 RealtimeMonitor 移除监控
    if let (Some(monitor), Some(alert)) = (state.stock_monitor.get(), alert) {
        monitor.remove_config(&alert.stock_code).await;
        tracing::info!("[delete_price_alert] 已从实时监控移除: {}", alert.stock_code);
    }

    Ok(())
}

/// P1-2: 加入实时行情监控（前端订阅 stock-quote-update 事件接收推送）。
///
/// @param stock_codes 要监控的股票代码列表
/// @param priority "active"（2s 轮询，用户当前查看）或 "background"（10s 轮询，仅监控）
/// @param replace 是否替换当前监控列表（true=替换，false=追加）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "加入行情监控")]
#[tauri::command]
pub async fn watch_stock_quotes(
    state: State<'_, AppState>,
    stock_codes: Vec<String>,
    priority: Option<String>,
    replace: Option<bool>,
) -> Result<(), String> {
    use axagent_astock_data::realtime_quote::WatchPriority;

    let watcher = state.quote_watcher.get().ok_or_else(|| "实时行情监视器未初始化".to_string())?;

    let pri = match priority.as_deref().unwrap_or("active") {
        "background" | "bg" => WatchPriority::Background,
        _ => WatchPriority::Active,
    };

    if replace.unwrap_or(false) {
        // 替换模式：先清空当前监控列表
        let current = watcher.watched_stocks().await;
        for code in &current {
            watcher.unwatch(code).await;
        }
    }

    let codes: Vec<&str> = stock_codes.iter().map(|s| s.as_str()).collect();
    watcher.watch_many(&codes, pri).await;

    tracing::info!(
        "[watch_stock_quotes] 已加入 {} 只股票监控 (priority={:?})",
        stock_codes.len(),
        pri
    );
    Ok(())
}

/// P1-2: 移除实时行情监控
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "移除行情监控")]
#[tauri::command]
pub async fn unwatch_stock_quotes(
    state: State<'_, AppState>,
    stock_codes: Vec<String>,
) -> Result<(), String> {
    let watcher = state.quote_watcher.get().ok_or_else(|| "实时行情监视器未初始化".to_string())?;

    for code in &stock_codes {
        watcher.unwatch(code).await;
    }

    tracing::info!("[unwatch_stock_quotes] 已移除 {} 只股票监控", stock_codes.len());
    Ok(())
}

/// P1-2: 查询当前监控的股票列表
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "查询当前监控股票")]
#[tauri::command]
pub async fn list_watched_quotes(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let watcher = state.quote_watcher.get().ok_or_else(|| "实时行情监视器未初始化".to_string())?;

    Ok(watcher.watched_stocks().await)
}

/// P1-2: 设置单只股票的监控优先级
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "设置监控优先级")]
#[tauri::command]
pub async fn set_quote_watch_priority(
    state: State<'_, AppState>,
    stock_code: String,
    priority: String,
) -> Result<(), String> {
    use axagent_astock_data::realtime_quote::WatchPriority;

    let watcher = state.quote_watcher.get().ok_or_else(|| "实时行情监视器未初始化".to_string())?;

    let pri = match priority.as_str() {
        "background" | "bg" => WatchPriority::Background,
        _ => WatchPriority::Active,
    };

    watcher.set_priority(&stock_code, pri).await;
    Ok(())
}

// ── 自定义分析师插件 ──

/// 列出所有自定义分析师插件
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出自定义分析师插件")]
#[tauri::command]
pub async fn list_custom_analysts()
-> Result<Vec<axagent_analysis_engine::plugin::CustomAnalyst>, String> {
    let mgr = AnalystPluginManager::new("agency_experts/stock-analysis");
    Ok(mgr.discover_custom_analysts())
}

/// 生成股票分析 HTML 报告
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "生成股票分析HTML报告")]
#[tauri::command]
pub async fn generate_stock_report(
    state: State<'_, AppState>,
    analysis_id: String,
) -> Result<String, String> {
    let record = stock_analyses::Entity::find_by_id(&analysis_id)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询分析记录失败: {e}"))
        })?
        .ok_or_else(|| ErrorResponse::new(wf_err::INTERNAL).with_detail("分析记录不存在"))?;

    // 生成报告路径
    let reports_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("AxInvest")
        .join("reports");
    std::fs::create_dir_all(&reports_dir).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建报告目录失败: {e}"))
    })?;

    let filename = format!("{}_{}.html", record.stock_code, record.analysis_date.replace('-', ""));
    let filepath = reports_dir.join(&filename);

    // 获取行情和K线数据
    let quote = state.astock_client.get_quote(&record.stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取行情失败: {e}"))
    })?;

    let klines =
        state.astock_client.get_klines(&record.stock_code, "daily", 120).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取 K 线失败: {e}"))
        })?;

    // 计算技术指标和客观评分
    let indicators =
        axagent_astock_data::indicators::compute_indicators(&record.stock_code, &klines);
    let mut score =
        axagent_analysis_engine::scoring::ScoringEngine::score(&indicators, quote.price, None);
    let pe = quote.pe.unwrap_or(0.0);
    let pb = quote.pb.unwrap_or(0.0);
    let roe = state
        .astock_client
        .get_financials(&record.stock_code)
        .await
        .ok()
        .and_then(|f| f.first().and_then(|r| r.roe));
    axagent_analysis_engine::scoring::ScoringEngine::apply_fundamental_adjustment(
        &mut score, pe, pb, roe,
    );
    axagent_analysis_engine::scoring::ScoringEngine::apply_industry_adjustment(
        &mut score, pe, None, pb, None,
    );

    let quote_json = serde_json::to_string(&quote).unwrap_or_default();
    let score_json = serde_json::to_string(&score).unwrap_or_default();
    let decision_json = record.decision_json.clone().unwrap_or_default();

    // 从 blackboard_snapshot 恢复分析师报告（仅提取 report.* 条目）
    // 注：snapshot 是 JSON 对象，value 可能是字符串（来自工作流结果）或嵌套对象
    // （来自 key_levels API 追加），用 Value 解析兼容两种情况。
    let bb_value: serde_json::Value = record
        .blackboard_snapshot
        .as_ref()
        .and_then(|snap| serde_json::from_str(snap).ok())
        .unwrap_or(serde_json::Value::Object(Default::default()));

    // 辅助：从 Value 中取字符串（空值视为缺失）
    let bb_str =
        |k: &str| -> String { bb_value.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string() };

    // 分析师报告：所有 report.* 前缀的键
    let analyst_reports: std::collections::HashMap<String, String> = bb_value
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter(|(k, _)| k.starts_with("report."))
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let value_assessment_json = bb_str("value.assessment");

    let html = axagent_analysis_engine::report::generate_html_report(
        &record.stock_code,
        &record.stock_name,
        &record.analysis_date,
        &quote_json,
        &indicators,
        &score_json,
        &analyst_reports,
        &decision_json,
        "",
        "",
        &value_assessment_json,
        &bb_str("raw.block_trades"),
        &bb_str("raw.institutional_visits"),
        &bb_str("market.index_quotes"),
        &bb_str("raw.peers"),
        &bb_str("raw.option_pcr"),
    );

    std::fs::write(&filepath, &html).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("写入报告文件失败: {e}"))
    })?;

    Ok(filepath.to_string_lossy().to_string())
}

// ── 手动交易日志 ──

/// 记录一笔交易
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "记录交易")]
#[tauri::command]
pub async fn record_trade(
    state: State<'_, AppState>,
    stock_code: String,
    stock_name: String,
    direction: String,
    price: f64,
    quantity: i32,
    trade_date: String,
    trade_time: String,
    notes: Option<String>,
    analysis_id: Option<String>,
) -> Result<trades::Model, String> {
    let engine = state.trading_engine.read().await;
    engine
        .execute_trade(
            &stock_code,
            &stock_name,
            &direction,
            price,
            quantity,
            &trade_date,
            &trade_time,
            notes.as_deref(),
            analysis_id.as_deref(),
        )
        .await
}

/// 获取交易历史
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取交易历史")]
#[tauri::command]
pub async fn list_trades(
    state: State<'_, AppState>,
    stock_code: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<trades::Model>, String> {
    let engine = state.trading_engine.read().await;
    engine.get_trades(stock_code.as_deref(), limit.unwrap_or(50)).await
}

/// 获取持仓汇总（交易日志驱动的成本跟踪）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "获取持仓汇总")]
#[tauri::command]
pub async fn get_trade_positions(
    state: State<'_, AppState>,
) -> Result<Vec<PositionSummary>, String> {
    let engine = state.trading_engine.read().await;
    engine.get_positions().await
}

/// 开启 / 关闭交易功能
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "开关交易功能")]
#[tauri::command]
pub async fn toggle_trading_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    tracing::info!("Trading system {}abled", if enabled { "en" } else { "dis" });
    axagent_dao::repo::settings::set_setting(
        state.harness.db(),
        "trading_enabled",
        &enabled.to_string(),
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("切换交易功能失败: {e}"))
            .to_string()
    })
}

/// 获取最近分析记录（用于 Dashboard）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取最近分析记录")]
#[tauri::command]
#[allow(dead_code)] // 暂未在 frontend 调起，预留给 Dashboard "历史" 区块
pub async fn get_recent_analyses(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
    let rows = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::Status.eq("completed"))
        .order_by_desc(stock_analyses::Column::CreatedAt)
        .limit(limit.unwrap_or(5) as u64)
        .all(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询最近分析记录失败: {e}"))
        })?;
    let result: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "stockCode": r.stock_code,
                "stockName": r.stock_name,
                "decisionAction": r.decision_action,
                "analysisDate": r.analysis_date,
                "status": r.status,
            })
        })
        .collect();
    Ok(result)
}

/// 校验交易（提交前预览）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "校验交易")]
#[tauri::command]
pub async fn validate_trade(
    state: State<'_, AppState>,
    stock_code: String,
    direction: String,
    quantity: i32,
    price: f64,
) -> Result<serde_json::Value, String> {
    let engine = state.trading_engine.read().await;
    let result = engine.validate_trade(&stock_code, &direction, quantity, price).await;
    Ok(serde_json::json!({
        "valid": result.valid,
        "errors": result.errors,
        "warnings": result.warnings,
    }))
}

/// 对比实际交易出场价与最近分析预测价位
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "对比交易与分析预测")]
#[tauri::command]
pub async fn compare_trade_with_analysis(
    state: State<'_, AppState>,
    trade_id: String,
) -> Result<TradePredictionComparison, String> {
    let trade = trades::Entity::find_by_id(&trade_id)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询交易记录失败: {e}"))
        })?
        .ok_or_else(|| ErrorResponse::new(wf_err::INTERNAL).with_detail("交易记录不存在"))?;

    let engine = state.trading_engine.read().await;
    engine.compare_trade_vs_prediction(&trade).await
}

// ── Key Levels Commands ──

/// 回测关键价位命中率
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "回测关键价位命中率")]
#[tauri::command]
pub async fn backtest_key_levels(
    state: State<'_, AppState>,
    lookback_days: u32,
) -> Result<KeyLevelBacktestStats, String> {
    let tracker =
        KeyLevelTracker::new(Arc::new(state.harness.db().clone()), state.astock_client.clone());
    tracker.backtest_key_levels(lookback_days).await
}

// ── Screen Commands ──

/// 从自选股中筛选(自选股为空或 DB 异常时回退到 FALLBACK_STOCKS 池)
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "筛选股票")]
#[tauri::command]
pub async fn screen_stocks(
    state: State<'_, AppState>,
    criteria: ScreenCriteria,
) -> Result<Vec<ScreenResult>, String> {
    let watchlist: Vec<(String, String)> =
        match axagent_entities::watchlist_items::Entity::find().all(state.harness.db()).await {
            Ok(rows) => rows.iter().map(|w| (w.stock_code.clone(), w.stock_name.clone())).collect(),
            Err(e) => {
                tracing::warn!("screen_stocks: 读自选股失败,改用 FALLBACK 池: {}", e);
                Vec::new()
            },
        };

    StockScreener::screen_watchlist(&state.astock_client, &watchlist, &criteria).await
}

/// 从全市场发现热门候选标的
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "发现热门候选标的")]
#[tauri::command]
pub async fn discover_stock_candidates(
    state: State<'_, AppState>,
) -> Result<Vec<ScreenResult>, String> {
    StockScreener::discover_candidates(&state.astock_client).await
}

// ── Calendar Commands ──

/// 获取市场状态
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取市场状态")]
#[tauri::command]
pub async fn get_market_status() -> Result<serde_json::Value, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let date = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Utc::now().date_naive());
    Ok(serde_json::json!({
        "isTradingDay": axagent_astock_data::calendar::is_trading_day(&date),
        "isTradingTime": axagent_astock_data::calendar::is_trading_time(),
        "status": axagent_astock_data::calendar::next_trading_time_desc(),
    }))
}

/// 从东方财富 API 刷新交易日历
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "刷新交易日历")]
#[tauri::command]
pub async fn refresh_trading_calendar() -> Result<Vec<String>, String> {
    axagent_astock_data::calendar::fetch_holiday_calendar().await
}

// ── Review Commands ──

/// 生成每日收盘复盘报告
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "生成每日复盘报告")]
#[tauri::command]
pub async fn generate_daily_review(state: State<'_, AppState>) -> Result<DailyReview, String> {
    let watchlist: Vec<(String, String)> = axagent_entities::watchlist_items::Entity::find()
        .all(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询自选股失败: {e}"))
        })?
        .iter()
        .map(|w| (w.stock_code.clone(), w.stock_name.clone()))
        .collect();

    // 查询当日已触发的价格告警
    let triggered_alerts_result = price_alerts::Entity::find()
        .filter(price_alerts::Column::IsTriggered.eq(true))
        .all(state.harness.db())
        .await;

    let mut triggered_alerts: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    if let Ok(alerts) = triggered_alerts_result {
        for alert in alerts {
            let desc = format!(
                "{}触发: 价格{}{:.2}(目标{:.2})",
                alert.condition,
                if alert.condition == "above" {
                    "≥"
                } else {
                    "≤"
                },
                state
                    .astock_client
                    .get_quote(&alert.stock_code)
                    .await
                    .map(|q| q.price)
                    .unwrap_or(0.0),
                alert.target_price
            );
            triggered_alerts.entry(alert.stock_code).or_default().push(desc);
        }
    }

    PostCloseReview::generate(
        &*state.astock_client,
        &watchlist,
        &triggered_alerts,
        state.harness.db(),
    )
    .await
}

// ── Scoring Weights Optimization ──

/// 基于回测结果优化评分权重
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "优化评分权重")]
#[tauri::command]
pub async fn optimize_scoring_weights(
    state: State<'_, AppState>,
) -> Result<axagent_analysis_engine::decision::ScoringWeights, String> {
    axagent_analysis_engine::backtest::optimize_weights(&*state.astock_client, state.harness.db())
        .await
}

/// 荐股策略历史回测（两组对比）
///
/// 1. 从 reco_picks 表读取最近一次荐股的真实推荐记录（synthetic=0）作为正向样本
/// 2. 从同次荐股的候选池快照中，减去正向样本，得到负向样本（漏推荐的股票）
/// 3. 两组分别跑策略信号历史回溯
/// 4. 输出对比结果
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "荐股策略历史回测")]
#[tauri::command]
pub async fn backtest_reco_strategies(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<axagent_analysis_engine::backtest_strategy::BacktestComparisonResponse, String> {
    let ctx = AsOfContext::parse_optional(as_of_date.as_deref())?;
    axagent_astock_data::as_of::with_optional_asof(ctx, async {
        backtest_reco_strategies_inner(&state).await
    })
    .await
}

async fn backtest_reco_strategies_inner(
    state: &State<'_, AppState>,
) -> Result<axagent_analysis_engine::backtest_strategy::BacktestComparisonResponse, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    // 1. 找最近一次荐股记录的 generated_at
    // P1 修复(2026-08-01): 排除 serenity-screening 写入的候选行（style='serenity'，
    // seed_pool_json 为 candidate 对象数组，非 [[code,name]] 快照格式）——
    // 若其晚于最近一次智能荐股，会污染回测的候选池解析与正负样本划分。
    let latest = reco_picks::Entity::find()
        .filter(reco_picks::Column::Style.ne("serenity"))
        .order_by_desc(reco_picks::Column::GeneratedAt)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询荐股记录失败: {e}"))
        })?;

    let latest_run = match latest {
        Some(r) => r,
        None => {
            return Err(ErrorResponse::new(wf_err::INTERNAL)
                .with_detail("暂无荐股记录。请先打开荐股面板获取推荐后再运行回测。")
                .into());
        },
    };
    let run_ts = latest_run.generated_at;

    // 2. 读取该次运行的所有推荐记录
    let all_picks = reco_picks::Entity::find()
        .filter(reco_picks::Column::GeneratedAt.eq(&run_ts))
        .all(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询荐股记录失败: {e}"))
        })?;

    if all_picks.is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail("荐股记录为空，无法回测")
            .into());
    }

    // 3. 解析候选池快照（从任一记录的 seed_pool_json 字段）
    let seed_pool_json =
        all_picks.first().and_then(|p| p.seed_pool_json.as_deref()).unwrap_or("[]");

    let seed_pool: Vec<Vec<String>> = serde_json::from_str(seed_pool_json).unwrap_or_default();

    // 4. 分离正向/负向样本
    // 正向 = synthetic=0 的 picks（被策略真实命中的推荐）
    // 负向 = 候选池中 - 正向（但注意：候选池可能有重复，用 HashSet 去重）
    let positive_set: std::collections::HashSet<String> =
        all_picks.iter().filter(|p| p.synthetic == 0).map(|p| p.stock_code.clone()).collect();

    let positive_stocks: Vec<(String, String)> = all_picks
        .iter()
        .filter(|p| p.synthetic == 0)
        .map(|p| (p.stock_code.clone(), p.stock_name.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    // 负向：候选池中的股票 - 正向样本
    let negative_stocks: Vec<(String, String)> = seed_pool
        .into_iter()
        .filter(|pair| pair.len() >= 2)
        .filter(|pair| !positive_set.contains(&pair[0]))
        .map(|pair| (pair[0].clone(), pair[1].clone()))
        .collect();

    // 5. 跑回测
    let bt_result = axagent_analysis_engine::backtest_strategy::backtest_two_groups(
        state.astock_client.clone(),
        &positive_stocks,
        &negative_stocks,
    )
    .await?;

    // 6. 更新信号质量缓存（供给侧反馈 → 荐股读取）
    axagent_analysis_engine::backtest_strategy::update_signal_quality_cache(
        &bt_result.positive.strategies,
    );

    Ok(bt_result)
}

/// 根据回测结果自动调整荐股策略权重
/// 预览荐股策略权重调整（只读，不写入）
///
/// 返回新旧权重对比，供用户确认后再调 apply。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "预览荐股策略权重调整")]
#[tauri::command]
pub async fn preview_adjust_reco_weights(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<serde_json::Value, String> {
    use axagent_analysis_engine::backtest_strategy::adjust_strategy_weights;
    use axagent_entities::workflow_template;
    use std::collections::BTreeMap;

    let db = state.harness.db();

    // 1. 读取模板已有权重
    let tmpl = workflow_template::Entity::find_by_id("stock-analysis")
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("读取模板失败: {e}"))
        })?
        .ok_or_else(|| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail("stock-analysis 模板不存在")
        })?;

    let existing: BTreeMap<String, f64> = tmpl
        .variables
        .as_deref()
        .and_then(|v| serde_json::from_str::<Vec<serde_json::Value>>(v).ok())
        .and_then(|vars| {
            // find 闭包不能用 ?，改写成显式检查
            let mut found = None;
            for v in &vars {
                if let Some(name) = v.get("name").and_then(|n| n.as_str()) {
                    if name == "reco_strategy_weights" {
                        if let Some(val) = v.get("value").and_then(|val| val.as_object().cloned()) {
                            found = Some(val);
                        }
                        break;
                    }
                }
            }
            found
        })
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
                .collect::<BTreeMap<String, f64>>()
        })
        .unwrap_or_default();

    // 2. 跑回测
    let ctx = AsOfContext::parse_optional(as_of_date.as_deref())?;
    let bt_result = axagent_astock_data::as_of::with_optional_asof(ctx, async {
        backtest_reco_strategies_inner(&state).await
    })
    .await?;

    // 3. 计算新权重
    let new_weights = adjust_strategy_weights(&bt_result.positive.strategies, Some(&existing))?;

    // 4. 构建 diff
    let mut diff: Vec<serde_json::Value> = Vec::new();
    for (sid, new_w) in &new_weights {
        let old_w = existing.get(sid).copied().unwrap_or(1.0);
        if (old_w - new_w).abs() > 0.001 {
            diff.push(serde_json::json!({
                "strategyId": sid,
                "oldWeight": (old_w * 100.0).round() / 100.0,
                "newWeight": (new_w * 100.0).round() / 100.0,
                "delta": ((new_w - old_w) * 100.0).round() / 100.0,
            }));
        }
    }

    Ok(serde_json::json!({
        "totalStrategies": bt_result.positive.strategies.len(),
        "changed": diff.len(),
        "weights": diff,
    }))
}

/// 应用荐股策略权重调整（用户确认后）
///
/// 如果传了 `weights` 则只应用其中列出的策略；不传则全部应用（向后兼容）。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "应用荐股策略权重调整")]
#[tauri::command]
pub async fn apply_reco_weights(
    state: State<'_, AppState>,
    weights: Option<Vec<serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::workflow_template;
    use sea_orm::sea_query::Expr;
    use sea_orm::{EntityTrait, QueryFilter};
    use std::collections::BTreeMap;

    let db = state.harness.db();

    // 1. 读取模板
    let tmpl = workflow_template::Entity::find_by_id("stock-analysis")
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("读取模板失败: {e}"))
        })?
        .ok_or_else(|| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail("stock-analysis 模板不存在")
        })?;

    let mut vars: Vec<serde_json::Value> =
        tmpl.variables.as_deref().and_then(|v| serde_json::from_str(v).ok()).unwrap_or_default();

    // 2. 构建要写入的 weight map
    let weight_map: BTreeMap<String, f64> = match weights {
        Some(list) => list
            .into_iter()
            .filter_map(|v| {
                let sid = v.get("strategyId")?.as_str()?.to_string();
                let w = v.get("weight")?.as_f64()?;
                Some((sid, w))
            })
            .collect(),
        None => {
            // 没有指定 weights → 跑一次完整回测再全部应用（向后兼容）
            return Err(ErrorResponse::new(wf_err::INTERNAL)
                .with_detail("请先调用 preview_adjust_reco_weights 获取建议权重再确认应用")
                .into());
        },
    };

    if weight_map.is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail("未选中任何权重调整项")
            .into());
    }

    // 3. 更新或创建 reco_strategy_weights
    let mut found = false;
    let weights_value = serde_json::to_value(&weight_map).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("序列化权重失败: {e}"))
    })?;
    for v in &mut vars {
        if v.get("name").and_then(|n| n.as_str()) == Some("reco_strategy_weights") {
            if let Some(obj) = v.as_object_mut() {
                if let Some(existing) = obj.get("value").and_then(|e| e.as_object()) {
                    let mut merged = existing.clone();
                    for (k, w) in &weight_map {
                        merged.insert(k.clone(), serde_json::json!(w));
                    }
                    obj.insert("value".into(), serde_json::json!(merged));
                } else {
                    obj.insert("value".into(), weights_value.clone());
                }
                found = true;
            }
            break;
        }
    }
    if !found {
        vars.push(serde_json::json!({
            "name": "reco_strategy_weights",
            "var_type": "object",
            "value": weights_value,
            "description": "荐股策略权重（用户确认后应用）",
            "is_secret": false,
        }));
    }

    // 4. 写回 DB
    let vars_str = serde_json::to_string(&vars).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("序列化模板变量失败: {e}"))
    })?;
    workflow_template::Entity::update_many()
        .col_expr(workflow_template::Column::Variables, Expr::value(vars_str))
        .filter(workflow_template::Column::Id.eq("stock-analysis"))
        .exec(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("写入模板变量失败: {e}"))
        })?;

    Ok(serde_json::json!({
        "applied": weight_map.len(),
        "weights": weight_map,
    }))
}

// ── RecoSignalTimeline ──

/// 获取指定策略的历史信号明细
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取策略历史信号明细")]
#[tauri::command]
pub async fn get_reco_signal_history(
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<Vec<axagent_analysis_engine::backtest_strategy::StrategySignalResult>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    // 1. 找最近一次荐股记录
    // P1 修复(2026-08-01): 排除 serenity 候选行（style='serenity'），
    // 只取 recommend_stocks 的策略推荐（seed_pool_json 为 [[code,name]] 快照格式）。
    let latest = reco_picks::Entity::find()
        .filter(reco_picks::Column::Style.ne("serenity"))
        .order_by_desc(reco_picks::Column::GeneratedAt)
        .one(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询荐股记录失败: {e}"))
        })?;

    let latest_run = match latest {
        Some(r) => r,
        None => {
            return Err(ErrorResponse::new(wf_err::INTERNAL)
                .with_detail("暂无荐股记录。请先打开荐股面板获取推荐。")
                .into());
        },
    };
    let run_ts = latest_run.generated_at;

    // 2. 读取该次运行的所有推荐记录
    let all_picks = reco_picks::Entity::find()
        .filter(reco_picks::Column::GeneratedAt.eq(&run_ts))
        .all(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询荐股记录失败: {e}"))
        })?;

    // 3. 构建股票列表（从推荐记录 + 候选池去重）
    let seed_pool_json =
        all_picks.first().and_then(|p| p.seed_pool_json.as_deref()).unwrap_or("[]");
    let seed_pool: Vec<Vec<String>> = serde_json::from_str(seed_pool_json).unwrap_or_default();

    use std::collections::BTreeSet;
    let mut all_stocks: BTreeSet<(String, String)> = BTreeSet::new();
    for p in &all_picks {
        all_stocks.insert((p.stock_code.clone(), p.stock_name.clone()));
    }
    for pair in &seed_pool {
        if pair.len() >= 2 {
            all_stocks.insert((pair[0].clone(), pair[1].clone()));
        }
    }
    let stock_list: Vec<(String, String)> = all_stocks.into_iter().collect();

    // 4. 调用信号历史
    axagent_analysis_engine::backtest_strategy::run_signal_history(
        state.astock_client.clone(),
        &strategy_id,
        Some(&stock_list),
    )
    .await
}

// ── Portfolio Risk ──

/// 获取组合风险指标
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "获取组合风险指标")]
#[tauri::command]
pub async fn get_portfolio_risk(
    state: State<'_, AppState>,
) -> Result<PortfolioRiskMetrics, String> {
    let engine = state.trading_engine.read().await;
    let positions = engine.get_positions().await?;
    Ok(PortfolioRiskManager::compute_from_positions(&positions))
}

// ── R2 组合监控 ──

/// 拉取最近一次组合监控快照（按 as_of_date 时间旅行）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "获取组合监控仪表盘")]
#[tauri::command]
pub async fn get_portfolio_dashboard(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<PortfolioDashboard, String> {
    let as_of = as_of_date.as_deref();
    let mut dashboard = portfolio_monitor::get_dashboard(state.harness.db(), as_of).await?;
    // 当天实时数据叠加：当前持仓/总市值（历史快照保留）
    if as_of.is_none() {
        let engine = state.trading_engine.read().await;
        let positions = engine.get_positions().await?;
        let (top, _sector, max_sec) = portfolio_monitor::compute_concentration(&positions);
        let n = positions.len();
        dashboard.top_concentration_pct = top;
        dashboard.positions = positions.clone();
        dashboard.total_market_value =
            positions.iter().map(|p| p.market_value.unwrap_or(0.0)).sum();
        dashboard.total_pnl = positions.iter().map(|p| p.unrealized_pnl.unwrap_or(0.0)).sum();
        let cost: f64 = positions.iter().map(|p| p.avg_cost * p.total_shares as f64).sum();
        dashboard.total_pnl_pct = if cost > 0.0 {
            (dashboard.total_pnl / cost) * 100.0
        } else {
            0.0
        };
        dashboard.risk_level = portfolio_monitor::compute_risk_level(top, max_sec, n);
        dashboard.diversification_score =
            portfolio_monitor::compute_diversification_score(n, top, max_sec);
        dashboard.concentration_warning =
            portfolio_monitor::compute_concentration_warning(top, max_sec, n);
        dashboard.sector_exposure = portfolio_monitor::compute_concentration(&positions).1;
        // 实时 stress test
        dashboard.stress_test =
            portfolio_monitor::run_all_scenarios(&positions, &dashboard.sector_exposure);
        dashboard.snapshot_at = chrono::Utc::now().timestamp_millis();
    }
    Ok(dashboard)
}

/// 立即刷新组合监控快照（写 portfolio_metrics_daily + correlation_snapshot）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "刷新组合监控快照")]
#[tauri::command]
pub async fn refresh_portfolio_metrics(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<serde_json::Value, String> {
    let engine = state.trading_engine.read().await;
    let positions = engine.get_positions().await?;
    drop(engine);
    let as_of = as_of_date.as_deref();

    let (id, count) = portfolio_monitor::refresh_metrics(
        state.harness.db(),
        &positions,
        &PositionLimits::default(),
        None,
        None,
        None,
        as_of,
    )
    .await?;

    let corr_count = portfolio_monitor::refresh_correlation(
        state.harness.db(),
        &*state.astock_client,
        &positions,
        60,
        as_of,
    )
    .await?;

    Ok(serde_json::json!({
        "metricsId": id,
        "positionsSnapshotted": count,
        "correlationPairsWritten": corr_count,
        "asOfDate": as_of,
    }))
}

/// 拉取最近一次两两相关性快照（按 as_of_date 时间旅行）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "获取组合相关性快照")]
#[tauri::command]
pub async fn get_portfolio_correlations(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<Vec<CorrelationCell>, String> {
    portfolio_monitor::get_correlation_snapshot(state.harness.db(), as_of_date.as_deref()).await
}

/// 压测（无 DB 副作用，纯计算）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "运行组合压力测试")]
#[tauri::command]
pub async fn run_portfolio_stress_test(
    state: State<'_, AppState>,
) -> Result<StressTestBundle, String> {
    let engine = state.trading_engine.read().await;
    let positions = engine.get_positions().await?;
    let (top, sector, _max) = portfolio_monitor::compute_concentration(&positions);
    let _ = top;
    Ok(portfolio_monitor::run_all_scenarios(&positions, &sector))
}

/// 校验能否新开仓（position_limits）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "校验新开仓限制")]
#[tauri::command]
pub async fn check_position_limits(
    state: State<'_, AppState>,
    stock_code: String,
    proposed_shares: i32,
    proposed_price: f64,
) -> Result<serde_json::Value, String> {
    let _ = stock_code; // sector lookup not used yet; keep on signature for forward-compat
    let engine = state.trading_engine.read().await;
    let positions = engine.get_positions().await?;
    let total_mv: f64 = positions.iter().map(|p| p.market_value.unwrap_or(0.0)).sum();
    let (top, sector_exposures, _max_sec) = portfolio_monitor::compute_concentration(&positions);
    let _ = top;
    let sector_pairs: Vec<(String, f64)> = sector_exposures.into_iter().collect();
    let limits = PositionLimits::default();
    let new_position_value = proposed_shares as f64 * proposed_price;
    let res = limits.check_new_position(
        new_position_value,
        total_mv,
        positions.len(),
        None,
        &sector_pairs,
    );
    match res {
        Ok(()) => Ok(serde_json::json!({
            "ok": true,
            "maxSingleStockPct": limits.max_single_stock_pct,
            "maxTotalPositions": limits.max_total_positions,
            "maxSectorExposurePct": limits.max_sector_exposure_pct,
            "newPositionValue": new_position_value,
        })),
        Err(reason) => Ok(serde_json::json!({
            "ok": false,
            "reason": reason,
            "maxSingleStockPct": limits.max_single_stock_pct,
            "maxTotalPositions": limits.max_total_positions,
            "maxSectorExposurePct": limits.max_sector_exposure_pct,
            "newPositionValue": new_position_value,
        })),
    }
}

// ── Value Investing ──

/// 获取巴菲特式价值投资评估
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取价值投资评估")]
#[tauri::command]
pub async fn get_value_assessment(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<axagent_analysis_engine::value::ValueAssessment, String> {
    let client = &state.astock_client;
    let quote = client.get_quote(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取实时行情失败: {e}"))
    })?;
    let financials = client.get_financials(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取财务数据失败: {e}"))
    })?;
    let shares = quote.total_mv.and_then(|mv| {
        if quote.price > 0.0 {
            Some(mv / quote.price / 1_0000_0000.0)
        } else {
            None
        }
    });
    let value_config = load_value_config(state.harness.db()).await;
    Ok(match shares {
        Some(s) if s > 0.0 => axagent_analysis_engine::value::ValueEngine::assess(
            quote.price,
            &financials,
            s,
            Some(&value_config),
        ),
        _ => axagent_analysis_engine::value::ValueEngine::assess_no_shares(
            quote.price,
            &financials,
            Some(&value_config),
        ),
    })
}

/// 计算巴菲特式价值投资综合指标（DCF + F-Score + 护城河量化 + 安全边际 + 所有者收益）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "计算价值投资综合指标")]
#[tauri::command]
pub async fn compute_value_metrics(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<axagent_analysis_engine::value_investing::ValueMetrics, String> {
    let quote = state.astock_client.get_quote(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取实时行情失败: {e}"))
    })?;
    let financials = state.astock_client.get_financials(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取财务数据失败: {e}"))
    })?;
    let total_shares = quote.total_mv.and_then(|mv| {
        if quote.price > 0.0 {
            Some(mv / quote.price / 1_0000_0000.0)
        } else {
            None
        }
    });
    let value_config = load_value_config(state.harness.db()).await;
    Ok(axagent_analysis_engine::value_investing::ValueInvestingEngine::compute(
        &stock_code,
        quote.price,
        total_shares,
        &financials,
        quote.pe,
        quote.pb,
        Some(&value_config),
    ))
}

// ── Position Limits ──

/// 获取全局仓位限制配置
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取全局仓位限制配置")]
#[tauri::command]
pub async fn get_position_limits() -> Result<PositionLimits, String> {
    Ok(PositionLimits::default())
}

// ── 新增数据源命令 ──

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取研究报告")]
#[tauri::command]
pub async fn get_stock_research_reports(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Vec<axagent_astock_data::ResearchReport>, String> {
    state.astock_client.get_research_reports(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取研究报告失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取一致预期EPS")]
#[tauri::command]
pub async fn get_stock_consensus_eps(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Option<axagent_astock_data::ConsensusEPS>, String> {
    state.astock_client.get_consensus_eps(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取一致预期 EPS 失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取概念板块")]
#[tauri::command]
pub async fn get_stock_concept_blocks(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Option<axagent_astock_data::ConceptBlocks>, String> {
    state.astock_client.get_concept_blocks(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取概念板块失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取公司公告")]
#[tauri::command]
pub async fn get_stock_announcements(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Vec<axagent_astock_data::Announcement>, String> {
    state.astock_client.get_announcements(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取公告失败: {e}")).to_string()
    })
}

/// 财报披露日历(R3-B):
///
/// 复用 `get_announcements` vendor 链路(优先 cninfo),按标题归类成
/// preliminary / express / formal / shareholders_meeting,过滤其它类。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取财报披露日历")]
#[tauri::command]
pub async fn get_earnings_calendar(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Vec<axagent_astock_data::EarningsEvent>, String> {
    state.astock_client.get_earnings_calendar(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取财报日历失败: {e}"))
            .to_string()
    })
}

/// 估值带(R3-C):
///
/// - years: 回溯窗口(默认 5 年);内部按 EOD 快照表统计 PE/PB/PS 的 5/10/25/50/75/90/95
///   分位 + 当前分位。
/// - 数据来源:本机 `financial_snapshots` 表(DB),表为空时返回 verdict = "insufficient"。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "计算估值带")]
#[tauri::command]
pub async fn compute_valuation_band(
    state: State<'_, AppState>,
    stock_code: String,
    years: Option<u32>,
) -> Result<axagent_astock_data::ValuationBand, String> {
    use axagent_astock_data::valuation_band::FinancialSnapshotLike;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let years = years.unwrap_or(5);
    let since_date = chrono::Local::now()
        .date_naive()
        .checked_sub_signed(chrono::Duration::days(365 * years as i64))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "0000-00-00".to_string());

    let db = state.harness.db();
    let stock_code_c = stock_code.clone();
    let since_date_c = since_date.clone();
    let historical: Vec<financial_snapshots::Model> = financial_snapshots::Entity::find()
        .filter(financial_snapshots::Column::StockCode.eq(stock_code_c.clone()))
        .filter(financial_snapshots::Column::SnapshotDate.gte(since_date_c.clone()))
        .order_by_asc(financial_snapshots::Column::SnapshotDate)
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询财务快照失败: {e}"))
        })?;

    // 把 ORM Model 转换为本地 struct 实现 trait
    struct SnapAdapter {
        date: String,
        pe: Option<f64>,
        pb: Option<f64>,
        ps: Option<f64>,
    }
    impl FinancialSnapshotLike for SnapAdapter {
        fn snapshot_date(&self) -> &str {
            &self.date
        }
        fn pe_ttm(&self) -> Option<f64> {
            self.pe
        }
        fn pb(&self) -> Option<f64> {
            self.pb
        }
        fn ps_ttm(&self) -> Option<f64> {
            self.ps
        }
    }
    let samples: Vec<SnapAdapter> = historical
        .into_iter()
        .map(|m| SnapAdapter { date: m.snapshot_date, pe: m.pe_ttm, pb: m.pb, ps: m.ps_ttm })
        .collect();

    let band = axagent_astock_data::valuation_band::compute_valuation_band(
        &stock_code,
        &samples,
        None, // 不传 current,让 UI 调用方自行叠加最新值
    );
    Ok(band)
}

/// 列估值快照原始行(R3-C 辅助):返回 financial_snapshots 表中某只股票在区间内的全部快照。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列估值快照原始行")]
#[tauri::command]
pub async fn list_financial_snapshots(
    state: State<'_, AppState>,
    stock_code: String,
    start: Option<String>,
    end: Option<String>,
) -> Result<Vec<financial_snapshots::Model>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
    let mut q = financial_snapshots::Entity::find()
        .filter(financial_snapshots::Column::StockCode.eq(stock_code.clone()));
    if let Some(s) = start {
        q = q.filter(financial_snapshots::Column::SnapshotDate.gte(s));
    }
    if let Some(e) = end {
        q = q.filter(financial_snapshots::Column::SnapshotDate.lte(e));
    }
    let rows = q
        .order_by_asc(financial_snapshots::Column::SnapshotDate)
        .all(state.harness.db())
        .await
        .map_err(|err| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询财务快照失败: {err}"))
        })?;
    Ok(rows)
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取热门股票")]
#[tauri::command]
pub async fn get_hot_stocks(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_astock_data::HotStock>, String> {
    state.astock_client.get_hot_stocks().await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取热门股票失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取行业排名")]
#[tauri::command]
pub async fn get_industry_ranking(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_astock_data::IndustryRank>, String> {
    state.astock_client.get_industry_ranking().await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取行业排名失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "搜索概念板块")]
#[tauri::command]
pub async fn search_concept_boards(
    state: State<'_, AppState>,
    keyword: String,
) -> Result<Vec<axagent_astock_data::ConceptBoard>, String> {
    if keyword.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("keyword 不能为空").into());
    }
    state.astock_client.search_concept_boards(&keyword).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("搜索概念板块失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取板块成分股")]
#[tauri::command]
pub async fn get_concept_board_members(
    state: State<'_, AppState>,
    board_code: String,
) -> Result<Vec<axagent_astock_data::BoardMember>, String> {
    if board_code.trim().is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL).with_detail("board_code 不能为空").into());
    }
    state.astock_client.get_concept_board_members(&board_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取板块成分股失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取财联社快讯")]
#[tauri::command]
pub async fn get_cls_flash(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_astock_data::ClsFlashItem>, String> {
    state.astock_client.get_cls_flash().await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取财联社快讯失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取龙虎榜")]
#[tauri::command]
pub async fn get_market_dragon_tiger(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_astock_data::MarketDragonTiger>, String> {
    state.astock_client.get_market_dragon_tiger().await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取龙虎榜失败: {e}")).to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取北向资金")]
#[tauri::command]
pub async fn get_north_bound_flow(
    state: State<'_, AppState>,
) -> Result<Option<axagent_astock_data::NorthBoundFlow>, String> {
    Ok(state.astock_client.get_north_bound_flow().await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("获取北向资金失败: {e}"))
    })?)
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取指数行情")]
#[tauri::command]
pub async fn get_index_quotes(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_astock_data::IndexQuote>, String> {
    state.astock_client.get_index_quotes().await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取指数行情失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取同行业对比")]
#[tauri::command]
pub async fn get_stock_peers(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Vec<axagent_astock_data::PeerComparison>, String> {
    state.astock_client.get_peers(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取同行业对比失败: {e}"))
            .to_string()
    })
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取期权PCR")]
#[tauri::command]
pub async fn get_stock_option_pcr(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Option<axagent_astock_data::OptionPCR>, String> {
    state.astock_client.get_option_pcr(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取期权 PCR 失败: {e}"))
            .to_string()
    })
}

// ── CronJob 定时任务（基于上游 CronJobStore + 持久化）──

use axagent_runtime_core::{CronJob, CronJobStatus};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJobResponse {
    id: String,
    name: String,
    description: String,
    schedule: String,
    status: String,
    recurring: bool,
    run_count: u32,
    last_run_at: Option<i64>,
    next_run_at: Option<i64>,
}

impl From<&CronJob> for CronJobResponse {
    fn from(j: &CronJob) -> Self {
        Self {
            id: j.id.clone(),
            name: j.name.clone(),
            description: j.description.clone(),
            schedule: j.schedule.clone(),
            status: format!("{:?}", j.status).to_lowercase(),
            recurring: j.recurring,
            run_count: j.run_count,
            last_run_at: j.last_run_at,
            next_run_at: j.next_run_at,
        }
    }
}

/// 创建股票定时分析任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "创建股票定时分析任务")]
#[tauri::command]
pub async fn create_stock_cron(
    state: State<'_, AppState>,
    stock_code: String,
    stock_name: String,
    cron_expression: String,
) -> Result<CronJobResponse, String> {
    let id = format!(
        "stock-{}-{}",
        stock_code,
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x")
    );
    let prompt = format!("对 {} ({}) 执行完整股票分析", stock_code, stock_name);
    let desc = format!("定时分析 {}", stock_code);
    let job = CronJob::new(&id, &cron_expression, &prompt, &desc)
        .with_workflow_id("stock-analysis".to_string())
        .with_task_type("stock-analysis");
    state.cron_job_store.add(job.clone()).await;
    Ok(CronJobResponse::from(&job))
}

/// 列出所有股票定时分析任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出股票定时分析任务")]
#[tauri::command]
pub async fn list_stock_crons(state: State<'_, AppState>) -> Result<Vec<CronJobResponse>, String> {
    let jobs = state.cron_job_store.list().await;
    Ok(jobs
        .iter()
        .filter(|j| j.task_type.as_deref() == Some("stock-analysis"))
        .map(CronJobResponse::from)
        .collect())
}

/// 启停定时任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "开关股票定时任务")]
#[tauri::command]
pub async fn toggle_stock_cron(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .cron_job_store
        .set_status(
            &id,
            if enabled {
                CronJobStatus::Active
            } else {
                CronJobStatus::Paused
            },
        )
        .await;
    Ok(())
}

/// 删除定时任务
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "删除股票定时任务")]
#[tauri::command]
pub async fn delete_stock_cron(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.cron_job_store.remove(&id).await;
    Ok(())
}

// ── P1-1: 持仓定时扫描 ──

/// 创建持仓自动扫描定时任务
///
/// 定时扫描所有持仓股，自动执行完整分析并携带持仓上下文。
/// task_type = "portfolio-scan"
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "创建持仓扫描定时任务")]
#[tauri::command]
pub async fn create_portfolio_scan_cron(
    state: State<'_, AppState>,
    cron_expression: String,
    enabled: Option<bool>,
) -> Result<CronJobResponse, String> {
    let id =
        format!("pfscan-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
    let mut job = CronJob::new(
        &id,
        &cron_expression,
        "持仓自动扫描",
        "定时扫描持仓列表，对每只持仓股执行完整分析，关联持仓上下文",
    )
    .with_task_type("portfolio-scan");
    if !enabled.unwrap_or(true) {
        job.status = CronJobStatus::Paused;
    }
    state.cron_job_store.add(job.clone()).await;
    Ok(CronJobResponse::from(&job))
}

/// 列出所有持仓扫描定时任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出持仓扫描定时任务")]
#[tauri::command]
pub async fn list_portfolio_scan_crons(
    state: State<'_, AppState>,
) -> Result<Vec<CronJobResponse>, String> {
    let jobs = state.cron_job_store.list().await;
    Ok(jobs
        .iter()
        .filter(|j| j.task_type.as_deref() == Some("portfolio-scan"))
        .map(CronJobResponse::from)
        .collect())
}

/// 启停持仓扫描定时任务
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description = "开关持仓扫描定时任务")]
#[tauri::command]
pub async fn toggle_portfolio_scan_cron(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .cron_job_store
        .set_status(
            &id,
            if enabled {
                CronJobStatus::Active
            } else {
                CronJobStatus::Paused
            },
        )
        .await;
    Ok(())
}

/// 删除持仓扫描定时任务
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "删除持仓扫描定时任务")]
#[tauri::command]
pub async fn delete_portfolio_scan_cron(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.cron_job_store.remove(&id).await;
    Ok(())
}

/// 检查指定数据源的连接可用性
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "检查数据源连接可用性")]
#[tauri::command]
pub async fn check_vendor_health(state: State<'_, AppState>, vendor: String) -> Result<(), String> {
    // 对需要 token/密钥的 vendor，先从数据库加载凭据到内存
    if vendor == "xueqiu" || vendor == "iwencai" || vendor == "neodata" {
        let template = axagent_entities::workflow_template::Entity::find_by_id("stock-analysis")
            .one(state.harness.db())
            .await
            .map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询模板失败: {e}"))
            })?;
        if let Some(t) = template {
            let vars = extract_template_vars(&t);
            for (name, value) in &vars {
                if name == "vendor_xueqiu_token" {
                    if let serde_json::Value::String(token) = value {
                        if !token.is_empty() {
                            if let Some(ref xq) = state.astock_client.xq_token {
                                *xq.write().await = token.clone();
                            }
                        }
                    }
                }
                if name == "vendor_iwencai_key" {
                    if let serde_json::Value::String(key) = value {
                        if !key.is_empty() {
                            *state.astock_client.iwencai_key.write().await = key.clone();
                        }
                    }
                }
                if name == "vendor_neodata_token" {
                    if let serde_json::Value::String(token) = value {
                        if !token.is_empty() {
                            if let Some(ref nd) = state.astock_client.neodata_token {
                                *nd.write().await = token.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    state.astock_client.check_vendor_health(&vendor).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("检查数据源健康度失败: {e}"))
            .to_string()
    })
}

/// 获取所有数据源的实时健康状态
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取所有数据源健康状态")]
#[tauri::command]
pub async fn get_vendor_health_all(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_astock_data::vendor_health::VendorHealth>, String> {
    Ok(state.astock_client.health_tracker.get_all_health().await)
}

/// P3-B5(F): 获取 vendor fallback 日志，用于前端调试"为什么 X 数据用了 Y vendor"
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取数据源Fallback日志")]
#[tauri::command]
pub async fn get_vendor_fallback_log(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_astock_data::vendor_health::FallbackRecord>, String> {
    Ok(state.astock_client.health_tracker.get_fallback_log().await)
}

/// P3-B5(B): 手动设置 vendor 状态（healthy/degraded/disabled）
/// 用于前端设置页"vendor 健康面板"手动启停 vendor
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "设置数据源状态")]
#[tauri::command]
pub async fn set_vendor_status(
    state: State<'_, AppState>,
    vendor: String,
    status: String,
) -> Result<(), String> {
    use axagent_astock_data::vendor_health::VendorStatus;

    let st = match status.as_str() {
        "healthy" => VendorStatus::Healthy,
        "degraded" => VendorStatus::Degraded,
        "disabled" => VendorStatus::Disabled,
        other => {
            return Err(ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!(
                    "无效的 vendor 状态: {other}（可选: healthy/degraded/disabled）"
                ))
                .to_string());
        },
    };

    state.astock_client.health_tracker.set_vendor_status(&vendor, st).await;
    tracing::info!("[P3-B5] vendor '{vendor}' 状态已手动设置为 '{status}'");
    Ok(())
}

/// 将 NeoData token 保存到 Python 脚本缓存文件
///
/// 调用 WorkBuddy 的 connect_cloud_service 获取新 token 后，
/// 由前端或自动化流程触发此命令将 token 写入脚本缓存。
/// 后续所有 NeoData 查询自动使用该 token（无需在设置页手动粘贴）。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "保存NeoData token")]
#[tauri::command]
pub async fn save_neodata_token(state: State<'_, AppState>, token: String) -> Result<(), String> {
    if token.is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail("NeoData token 不能为空")
            .into());
    }
    // 1) 写入 Python 脚本缓存
    axagent_astock_data::vendors::neodata::save_token_to_cache(&token).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("保存 NeoData token 到脚本缓存失败: {e}"))
    })?;

    // 2) 同时写入共享内存（立即生效，无需重启）
    if let Some(ref nd) = state.astock_client.neodata_token {
        *nd.write().await = token.clone();
    }

    // 3) 持久化到数据库（设置页下次加载时自动读取）
    use axagent_entities::workflow_template;
    use sea_orm::EntityTrait;
    if let Some(t) = workflow_template::Entity::find_by_id("stock-analysis")
        .one(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询模板失败: {e}"))
        })?
    {
        let mut vars = t
            .variables
            .as_ref()
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
            .unwrap_or_default();
        let token_val = serde_json::json!({
            "name": "vendor_neodata_token",
            "is_secret": true,
            "defaultValue": null,
            "value": token,
            "type": "string",
        });
        // 替换或新增
        if let Some(pos) = vars
            .iter()
            .position(|v| v.get("name").and_then(|n| n.as_str()) == Some("vendor_neodata_token"))
        {
            vars[pos] = token_val;
        } else {
            vars.push(token_val);
        }
        let json_str = serde_json::to_string(&vars).unwrap_or_default();
        use axagent_entities::workflow_template::ActiveModel;
        use sea_orm::ActiveModelTrait;
        let mut am: ActiveModel = t.into();
        am.variables = sea_orm::ActiveValue::Set(Some(json_str));
        am.update(state.harness.db()).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("持久化 NeoData token 失败: {e}"))
        })?;
    }

    Ok(())
}

/// 执行每日快照采集：遍历 SNAPSHOT_METHODS，将全市场/个股数据存入 DiskCache
///
/// 调用一次即可采集当日快照；as-of 模式下的 NoHistoricalSemantic 方法会优先查快照缓存。
/// 建议在每日收盘后（15:30 以后）通过 cron 调用。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "采集每日快照")]
#[tauri::command]
pub async fn sweep_daily_snapshots(state: State<'_, AppState>) -> Result<String, String> {
    use axagent_astock_data::daily_snapshot::{PER_STOCK_METHODS, SNAPSHOT_METHODS};
    use chrono::Local;

    let date = Local::now().format("%Y-%m-%d").to_string();
    let client = &state.astock_client;
    let mut market_count = 0u32;
    let mut stock_count = 0u32;

    // 获取自选股列表作为个股遍历的候选池
    let watchlist_codes: Vec<String> = axagent_entities::watchlist_items::Entity::find()
        .all(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("读取自选股失败: {e}"))
        })?
        .into_iter()
        .map(|w| w.stock_code)
        .collect();

    // 遍历所有快照方法
    for method in SNAPSHOT_METHODS {
        if PER_STOCK_METHODS.contains(method) {
            // 个股级方法：遍历自选股逐只采集
            for code in &watchlist_codes {
                let json = match *method {
                    "get_money_flow" => match client.get_money_flow(code).await {
                        Ok(Some(r)) => serde_json::to_string(&r).unwrap_or_default(),
                        _ => continue,
                    },
                    "get_north_bound_holding" => match client.get_north_bound_holding(code).await {
                        Ok(Some(r)) => serde_json::to_string(&r).unwrap_or_default(),
                        _ => continue,
                    },
                    "get_margin_data" => match client.get_margin_data(code).await {
                        Ok(Some(r)) => serde_json::to_string(&r).unwrap_or_default(),
                        _ => continue,
                    },
                    _ => continue,
                };
                client.set_stock_daily_snapshot(method, code, &date, &json);
                stock_count += 1;
            }
        } else {
            // 全市场方法
            let json = match *method {
                "get_hot_stocks" => match client.get_hot_stocks().await {
                    Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
                    _ => continue,
                },
                "get_industry_ranking" => match client.get_industry_ranking().await {
                    Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
                    _ => continue,
                },
                "get_cls_flash" => match client.get_cls_flash().await {
                    Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
                    _ => continue,
                },
                "get_stock_concept_blocks" => {
                    // 概念板块需要个股参数，遍历自选股
                    for code in &watchlist_codes {
                        match client.get_concept_blocks(code).await {
                            Ok(Some(r)) => {
                                let json = serde_json::to_string(&r).unwrap_or_default();
                                client.set_stock_daily_snapshot(method, code, &date, &json);
                                stock_count += 1;
                            },
                            _ => continue,
                        }
                    }
                    continue;
                },
                "search_stock" => {
                    // 不需要采集全市场搜索快照，跳过
                    continue;
                },
                "get_sector_info" => {
                    // 行业分类是个股维度，遍历自选股
                    for code in &watchlist_codes {
                        match client.get_sector_info(code).await {
                            Ok(Some(r)) => {
                                let json = serde_json::to_string(&r).unwrap_or_default();
                                client.set_stock_daily_snapshot(method, code, &date, &json);
                                stock_count += 1;
                            },
                            _ => continue,
                        }
                    }
                    continue;
                },
                "get_stock_announcements" => {
                    // 公告是逐只个股的，遍历自选股
                    for code in &watchlist_codes {
                        match client.get_announcements(code).await {
                            Ok(r) if !r.is_empty() => {
                                let json = serde_json::to_string(&r).unwrap_or_default();
                                client.set_stock_daily_snapshot(method, code, &date, &json);
                                stock_count += 1;
                            },
                            _ => continue,
                        }
                    }
                    continue;
                },
                "get_index_quotes" => match client.get_index_quotes().await {
                    Ok(r) => serde_json::to_string(&r).unwrap_or_default(),
                    _ => continue,
                },
                _ => continue,
            };
            if !json.is_empty() {
                client.set_daily_snapshot(method, &date, &json);
                market_count += 1;
            }
        }
    }

    Ok(format!("快照采集完成: 全市场 {} 项, 个股 {} 条", market_count, stock_count))
}

/// 拉取智能荐股结果（按周期）
///
/// 前端传 period 序列化为 [Period] 枚举（"short" | "mid" | "long"）
/// 可选 `as_of_date` 触发时间旅行模式：as_of_date 之前的数据用于回测，
/// 之后的数据被严格屏蔽。
/// 响应见 [RecoResponse]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "智能荐股")]
#[tauri::command]
pub async fn recommend_stocks(
    state: State<'_, AppState>,
    period: axagent_analysis_engine::recommender::Period,
    as_of_date: Option<String>,
) -> Result<RecoResponse, String> {
    // 解析 as_of_date；非法/未来 → 4xx-style 错误
    let as_of_ctx = AsOfContext::parse_optional(as_of_date.as_deref())?;

    // 读取 workflow template 变量用于 vendor 启用检测
    let template = axagent_entities::workflow_template::Entity::find_by_id("stock-analysis")
        .one(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询模板失败: {e}"))
        })?;

    let vars: Vec<(String, serde_json::Value)> = match template {
        Some(t) => extract_template_vars(&t),
        None => Vec::new(),
    };

    // state.astock_client 已是 Arc<AStockClient>，直接 clone Arc 即可
    let client: std::sync::Arc<_> = state.astock_client.clone();
    let response = if let Some(ctx) = as_of_ctx {
        axagent_astock_data::as_of::AS_OF
            .scope(Some(ctx), async {
                recommender::recommend_stocks(client, period, &vars, None).await
            })
            .await
    } else {
        recommender::recommend_stocks(client, period, &vars, None).await
    }?;

    // ── 持久化荐股结果（仅 live 模式） ──
    if as_of_date.is_none() {
        let generated_at = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();
        let created_at = generated_at.clone();

        // 构建策略权重快照（用于回溯某次荐股时的权重配置）
        let strategy_weights_json: Option<String> = {
            let vars_clone: Vec<(String, serde_json::Value)> = vars.clone();
            vars_clone.iter().find(|(k, _)| k == "reco_strategy_weights").and_then(|(_, v)| {
                if v.is_object() {
                    Some(v.to_string())
                } else {
                    None
                }
            })
        };

        // 构建候选池快照（用于回测的负向样本）
        // P3 修复(2026-08-01): 直接复用 recommend_stocks 扫描实际使用的池
        // （流动性过滤后 seed），不再二次 build_seed_pool ——
        // 旧逻辑浪费 get_hot_stocks + get_industry_ranking 两次请求，
        // 且两次构建间数据变化会导致快照与真实扫描池不一致（preseed 模式更严重）。
        let seed_pool_json =
            response.seed_pool_snapshot.clone().unwrap_or_else(|| "[]".to_string());

        for picks in response.picks.values() {
            for pick in picks {
                use sea_orm::ActiveModelTrait;
                // 序列化完整 pick 到 pick_data —— get_cached_recommendation 会
                // 读这一列还原 cache,与实时拉取结果 schema 完全等价。
                let pick_data = serde_json::to_string(pick).ok();
                let am = reco_picks::ActiveModel {
                    id: sea_orm::Set(uuid::Uuid::new_v4().to_string()),
                    generated_at: sea_orm::Set(generated_at.clone()),
                    period: sea_orm::Set(pick.period.as_str().to_string()),
                    stock_code: sea_orm::Set(pick.stock_code.clone()),
                    stock_name: sea_orm::Set(pick.stock_name.clone()),
                    style: sea_orm::Set(pick.style.as_str().to_string()),
                    confidence: sea_orm::Set(pick.confidence as i32),
                    synthetic: sea_orm::Set(if pick.synthetic { 1 } else { 0 }),
                    seed_pool_json: sea_orm::Set(Some(seed_pool_json.clone())),
                    strategy_weights_json: sea_orm::Set(strategy_weights_json.clone()),
                    pick_data: sea_orm::Set(pick_data),
                    created_at: sea_orm::Set(created_at.clone()),
                };
                // P2 修复(2026-08-01): 插入失败不再静默吞错，记 warn 日志便于排查
                // （此前 `let _ = insert(...)` 失败无感知，前端表现为"无缓存"）
                if let Err(e) = am.insert(state.harness.db()).await {
                    tracing::warn!(
                        "[recommend_stocks] 写入 reco_picks 失败 ({} {} {}): {}",
                        pick.period.as_str(),
                        pick.stock_code,
                        pick.style.as_str(),
                        e
                    );
                }
            }
        }
    }

    Ok(response)
}

/// 读取最近一次 live 荐股结果(缓存) —— 智能荐股页打开时优先调此命令,
/// 避免每次打开都触发一次新的后端推荐任务。
///
/// 返回值:
/// - `Some(RecoResponse)` —— 缓存存在,直接展示
/// - `None` —— 该 period 还没有历史荐股(可引导用户点"刷新"实时拉取)
///
/// 行为:
/// 1. 查 reco_picks 表:按 period 过滤,取最新 generated_at 对应的所有行
/// 2. 反序列化每行 pick_data → RecoPick
/// 3. 按 style 分组,组装 RecoResponse,mode = "cached"
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "读取缓存荐股结果")]
#[tauri::command]
pub async fn get_cached_recommendation(
    state: State<'_, AppState>,
    period: axagent_analysis_engine::recommender::Period,
) -> Result<Option<RecoResponse>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

    let db = state.harness.db();
    let period_str = period.as_str();

    // 1) 找该 period 最新的 generated_at
    // 用 ORDER BY + LIMIT 1 拿最新一次,避免聚合复杂 SQL
    // P1 修复(2026-08-01): 排除 serenity 候选行（style='serenity'，period 固定 'mid'，
    // 会抢占 mid 的缓存并让前端 STYLE_KEYS 匹配不上而显示空）。
    let latest = reco_picks::Entity::find()
        .filter(reco_picks::Column::Period.eq(period_str))
        .filter(reco_picks::Column::Style.ne("serenity"))
        .filter(reco_picks::Column::PickData.is_not_null()) // 跳过 v007 之前的旧行
        .order_by_desc(reco_picks::Column::GeneratedAt)
        .limit(1)
        .one(db)
        .await
        .map_err(|e| ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询荐股记录失败: {e}")))?;

    let Some(latest_row) = latest else {
        return Ok(None);
    };

    // 2) 取同 generated_at 的所有行
    let rows = reco_picks::Entity::find()
        .filter(reco_picks::Column::Period.eq(period_str))
        .filter(reco_picks::Column::GeneratedAt.eq(&latest_row.generated_at))
        .filter(reco_picks::Column::PickData.is_not_null())
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询荐股记录失败: {e}"))
        })?;

    if rows.is_empty() {
        return Ok(None);
    }

    // 3) 反序列化 + 按 style 分组
    use std::collections::{BTreeMap, HashMap};
    let mut picks_map: BTreeMap<
        axagent_analysis_engine::recommender::Style,
        Vec<axagent_analysis_engine::recommender::RecoPick>,
    > = BTreeMap::new();

    for row in &rows {
        let Some(ref pd) = row.pick_data else {
            continue;
        };
        let Ok(pick) = serde_json::from_str::<axagent_analysis_engine::recommender::RecoPick>(pd)
        else {
            continue;
        };
        picks_map.entry(pick.style).or_default().push(pick);
    }

    if picks_map.is_empty() {
        return Ok(None);
    }

    // 4) 估算 seed pool size:从同 generated_at 的任意行读 seed_pool_json(若有)
    let raw_seed_pool_size: usize = rows
        .iter()
        .find_map(|r| {
            r.seed_pool_json.as_ref().and_then(|s| {
                let parsed: Option<Vec<Vec<String>>> = serde_json::from_str(s).ok();
                parsed.map(|v| v.len())
            })
        })
        .unwrap_or(0);

    // 5) 解析 generated_at 为毫秒时间戳(ISO 8601 字符串 → timestamp)
    let generated_at_ms = parse_iso8601_to_millis(&latest_row.generated_at).unwrap_or(0);

    Ok(Some(RecoResponse {
        period,
        picks: picks_map,
        disabled_styles: vec![],
        degraded_styles: vec![],
        degraded_reasons: HashMap::new(),
        generated_at: generated_at_ms,
        raw_seed_pool_size,
        as_of_date: None,
        mode: "cached".to_string(),
        error_detail: None,
        // 缓存还原路径无需携带 seed 快照（serde skip 不传给前端；表内已有）
        seed_pool_snapshot: None,
    }))
}

/// 把 ISO 8601 字符串（如 "2026-06-23T10:24:47.123"）解析为毫秒时间戳。
/// 解析失败返回 None（不抛错，避免缓存展示被一个坏数据阻断）。
fn parse_iso8601_to_millis(s: &str) -> Option<i64> {
    use chrono::TimeZone;
    // 兼容两种常见格式：带毫秒 / 不带毫秒
    let formats = ["%Y-%m-%dT%H:%M:%S%.3f", "%Y-%m-%dT%H:%M:%S"];
    for fmt in formats {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            // 用本地时区解释（recommend_stocks 也是用 chrono::Local::now() 生成）
            if let chrono::LocalResult::Single(dt) = chrono::Local.from_local_datetime(&naive) {
                return Some(dt.timestamp_millis());
            }
        }
    }
    None
}

/// 失效荐股缓存（设置页保存 vendor 后由前端调用）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "失效荐股缓存")]
#[tauri::command]
pub fn invalidate_recommendation_cache() {
    recommender::invalidate_cache();
}

/// 个股最近一次分析摘要 — 用于荐股面板等场景展示"上次分析结论"
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestAnalysisSummary {
    pub analysis_id: String,
    pub analysis_date: String,
    pub decision_action: String, // BUY / HOLD / SELL / uncertain
    pub decision_position_pct: Option<f64>,
    pub confidence: Option<i32>, // 加权置信度 0-100，从 decision_json 提取
    pub status: String,          // completed / running / failed
    pub outcome: Option<String>, // win / loss / pending
    pub decision_time_horizon: Option<String>,
    pub decision_expected_holding_days: Option<i64>,
}

/// 查询个股最近一次已完成分析的决策摘要
///
/// 若 `as_of_date` 不为 None 则只返回到该日期为止的分析（时间旅行兼容）。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "查询个股最近分析摘要")]
#[tauri::command]
pub async fn get_latest_analysis_for_stock(
    state: tauri::State<'_, AppState>,
    stock_code: String,
    as_of_date: Option<String>,
) -> Result<Option<LatestAnalysisSummary>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

    let db = state.harness.db();
    let mut query = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::StockCode.eq(&stock_code))
        .filter(stock_analyses::Column::Status.eq("completed"));

    // 时间旅行模式：只返回截止日之前的分析
    if let Some(ref cutoff) = as_of_date {
        query = query.filter(stock_analyses::Column::AnalysisDate.lte(cutoff));
    }

    let row =
        query.order_by_desc(stock_analyses::Column::CreatedAt).limit(1).one(db).await.map_err(
            |e| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("查询 stock_analyses 失败: {e}"))
            },
        )?;

    let Some(model) = row else {
        return Ok(None);
    };

    // 从 decision_json 提取 confidence
    let confidence: Option<i32> = model.decision_json.as_ref().and_then(|raw| {
        serde_json::from_str::<serde_json::Value>(raw).ok().and_then(|v| {
            v.get("confidence")
                .or_else(|| v.get("weighted_confidence"))
                .and_then(|c| c.as_i64())
                .map(|i| i as i32)
        })
    });

    Ok(Some(LatestAnalysisSummary {
        analysis_id: model.id,
        analysis_date: model.analysis_date,
        decision_action: model.decision_action.unwrap_or_else(|| "uncertain".into()),
        decision_position_pct: model.decision_position_pct,
        confidence,
        status: model.status,
        outcome: model.outcome,
        decision_time_horizon: model.decision_time_horizon,
        decision_expected_holding_days: model.decision_expected_holding_days,
    }))
}

/// 批量查询多只个股的最近分析摘要
///
/// 一次 SQL 查询返回 HashMap，key 为 stock_code。
/// `as_of_date` 语义同 `get_latest_analysis_for_stock`。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "批量查询个股最近分析摘要")]
#[tauri::command]
pub async fn get_latest_analyses_for_stocks(
    state: tauri::State<'_, AppState>,
    stock_codes: Vec<String>,
    as_of_date: Option<String>,
) -> Result<std::collections::HashMap<String, Option<LatestAnalysisSummary>>, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let db = state.harness.db();
    let mut result: std::collections::HashMap<String, Option<LatestAnalysisSummary>> =
        std::collections::HashMap::new();

    // 批量查询：循环查询每只 stock_code，利用连接池和 SQLite 的行级缓存，40 只以内足够快
    for code in &stock_codes {
        let mut query = stock_analyses::Entity::find()
            .filter(stock_analyses::Column::StockCode.eq(code))
            .filter(stock_analyses::Column::Status.eq("completed"));

        if let Some(ref cutoff) = as_of_date {
            query = query.filter(stock_analyses::Column::AnalysisDate.lte(cutoff));
        }

        let row =
            query.order_by_desc(stock_analyses::Column::CreatedAt).limit(1).one(db).await.map_err(
                |e| {
                    ErrorResponse::new(wf_err::INTERNAL)
                        .with_detail(format!("批量查询 stock_analyses({code}) 失败: {e}"))
                },
            )?;

        let summary = row.map(|model| {
            let confidence: Option<i32> = model.decision_json.as_ref().and_then(|raw| {
                serde_json::from_str::<serde_json::Value>(raw).ok().and_then(|v| {
                    v.get("confidence")
                        .or_else(|| v.get("weighted_confidence"))
                        .and_then(|c| c.as_i64())
                        .map(|i| i as i32)
                })
            });

            LatestAnalysisSummary {
                analysis_id: model.id,
                analysis_date: model.analysis_date,
                decision_action: model.decision_action.unwrap_or_else(|| "uncertain".into()),
                decision_position_pct: model.decision_position_pct,
                confidence,
                status: model.status,
                outcome: model.outcome,
                decision_time_horizon: model.decision_time_horizon,
                decision_expected_holding_days: model.decision_expected_holding_days,
            }
        });

        result.insert(code.clone(), summary);
    }

    Ok(result)
}

/// 从 workflow_template 实体提取 (name, value) 列表
fn extract_template_vars(
    t: &axagent_entities::workflow_template::Model,
) -> Vec<(String, serde_json::Value)> {
    use axagent_harness::workflow_types::Variable;
    let raw = match t.variables.as_ref() {
        Some(s) => s,
        None => return Vec::new(),
    };
    match serde_json::from_str::<Vec<Variable>>(raw) {
        Ok(vs) => vs.into_iter().map(|v| (v.name, v.value)).collect(),
        Err(_) => Vec::new(),
    }
}

// ── 自选股自动扫描定时任务 ──

/// 创建自选股自动分析定时任务
///
/// 到点时遍历用户自选股列表，对每只股票执行 `run_single_stock_analysis`。
/// 后端 CronExecutor 通过 `task_type = "watchlist-scan"` 路由。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "创建自选股扫描定时任务")]
#[tauri::command]
pub async fn create_watchlist_scan_cron(
    state: State<'_, AppState>,
    cron_expression: String,
    enabled: Option<bool>,
) -> Result<CronJobResponse, String> {
    let id =
        format!("wlscan-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
    let mut job = CronJob::new(
        &id,
        &cron_expression,
        "自选股自动扫描",
        "定时扫描自选股列表，对每只股票执行完整分析工作流",
    )
    .with_task_type("watchlist-scan");
    if !enabled.unwrap_or(true) {
        job.status = CronJobStatus::Paused;
    }
    state.cron_job_store.add(job.clone()).await;
    Ok(CronJobResponse::from(&job))
}

/// 列出所有自选股扫描定时任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出自选股扫描定时任务")]
#[tauri::command]
pub async fn list_watchlist_scan_crons(
    state: State<'_, AppState>,
) -> Result<Vec<CronJobResponse>, String> {
    let jobs = state.cron_job_store.list().await;
    Ok(jobs
        .iter()
        .filter(|j| j.task_type.as_deref() == Some("watchlist-scan"))
        .map(CronJobResponse::from)
        .collect())
}

/// 启停自选股扫描定时任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "开关自选股扫描定时任务")]
#[tauri::command]
pub async fn toggle_watchlist_scan_cron(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .cron_job_store
        .set_status(
            &id,
            if enabled {
                CronJobStatus::Active
            } else {
                CronJobStatus::Paused
            },
        )
        .await;
    Ok(())
}

/// 删除自选股扫描定时任务
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "删除自选股扫描定时任务")]
#[tauri::command]
pub async fn delete_watchlist_scan_cron(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.cron_job_store.remove(&id).await;
    Ok(())
}

/// 创建决策校验+反思复盘定时任务
///
/// 每天扫描 30 天前的分析结果，判定 win/loss。
/// loss 自动触发 `run_reflection_workflow`（嵌套原股票分析工作流的 as-of 重放 + hindsight 注入）。
///
/// 参数：
/// - `cron_expression`: cron 表达式，默认 "0 6 * * *"
/// - `min_confidence_threshold`: 触发反思的最低置信度（0=全部触发）
/// - `reflection_depth`: "light"(简要) 或 "deep"(详细推理链)
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "创建决策校验定时任务")]
#[tauri::command]
pub async fn create_validate_decisions_cron(
    state: State<'_, AppState>,
    cron_expression: Option<String>,
    min_confidence_threshold: Option<i32>,
    reflection_depth: Option<String>,
    enabled: Option<bool>,
) -> Result<CronJobResponse, String> {
    let id = format!("vldec-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
    let expr = cron_expression.unwrap_or_else(|| "0 6 * * *".to_string());
    let threshold = min_confidence_threshold.unwrap_or(0);
    let depth = reflection_depth.unwrap_or_else(|| "light".to_string());
    let desc = format!(
        "扫描30天前的分析结果判定win/loss，loss自动触发反思工作流（阈值:{}, 深度:{})",
        threshold, depth
    );
    let mut job =
        CronJob::new(&id, &expr, "决策校验 + 反思复盘", &desc).with_task_type("validate-decisions");
    if !enabled.unwrap_or(true) {
        job.status = CronJobStatus::Paused;
    }
    state.cron_job_store.add(job.clone()).await;
    Ok(CronJobResponse::from(&job))
}

/// 列出所有决策校验定时任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出决策校验定时任务")]
#[tauri::command]
pub async fn list_validate_decisions_crons(
    state: State<'_, AppState>,
) -> Result<Vec<CronJobResponse>, String> {
    let jobs = state.cron_job_store.list().await;
    Ok(jobs
        .iter()
        .filter(|j| j.task_type.as_deref() == Some("validate-decisions"))
        .map(CronJobResponse::from)
        .collect())
}

/// 启停决策校验定时任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "开关决策校验定时任务")]
#[tauri::command]
pub async fn toggle_validate_decisions_cron(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .cron_job_store
        .set_status(
            &id,
            if enabled {
                CronJobStatus::Active
            } else {
                CronJobStatus::Paused
            },
        )
        .await;
    Ok(())
}

/// 删除决策校验定时任务
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "删除决策校验定时任务")]
#[tauri::command]
pub async fn delete_validate_decisions_cron(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.cron_job_store.remove(&id).await;
    Ok(())
}

/// 创建批量反思定时任务（D1 借鉴：定期 resolve pending reflections）
///
/// 收市后 18:00 执行: `0 18 * * *`
/// 每个 pending row 到达持仓期后自动 resolve，无需手动触发。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "创建批量反思定时任务")]
#[tauri::command]
pub async fn create_batch_reflection_cron(
    state: State<'_, AppState>,
    cron_expression: Option<String>,
    enabled: Option<bool>,
) -> Result<CronJobResponse, String> {
    let id =
        format!("batchref-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));
    let expr = cron_expression.unwrap_or_else(|| "0 18 * * *".to_string());
    let mut job = CronJob::new(
        &id,
        &expr,
        "批量反思复盘",
        "扫描所有 pending reflection row，到达持仓期的自动 resolve",
    )
    .with_task_type("batch-reflection");
    if !enabled.unwrap_or(true) {
        job.status = CronJobStatus::Paused;
    }
    state.cron_job_store.add(job.clone()).await;
    Ok(CronJobResponse::from(&job))
}

/// 列出所有批量反思定时任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出批量反思定时任务")]
#[tauri::command]
pub async fn list_batch_reflection_crons(
    state: State<'_, AppState>,
) -> Result<Vec<CronJobResponse>, String> {
    let jobs = state.cron_job_store.list().await;
    Ok(jobs
        .iter()
        .filter(|j| j.task_type.as_deref() == Some("batch-reflection"))
        .map(CronJobResponse::from)
        .collect())
}

/// 启停批量反思定时任务
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "开关批量反思定时任务")]
#[tauri::command]
pub async fn toggle_batch_reflection_cron(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .cron_job_store
        .set_status(
            &id,
            if enabled {
                CronJobStatus::Active
            } else {
                CronJobStatus::Paused
            },
        )
        .await;
    Ok(())
}

/// 删除批量反思定时任务
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "删除批量反思定时任务")]
#[tauri::command]
pub async fn delete_batch_reflection_cron(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.cron_job_store.remove(&id).await;
    Ok(())
}

/// 查询反思复盘记录列表
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "查询反思复盘记录列表")]
#[tauri::command]
pub async fn list_reflections(
    state: State<'_, AppState>,
    stock_code: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    use axagent_entities::stock_reflections;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let db = state.harness.db();
    let mut query = stock_reflections::Entity::find()
        .order_by(stock_reflections::Column::CreatedAt, sea_orm::Order::Desc);
    if let Some(ref code) = stock_code {
        query = query.filter(stock_reflections::Column::StockCode.eq(code));
    }
    let items = query.all(db).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询反思记录失败: {e}"))
    })?;
    let limit = limit.unwrap_or(50) as usize;
    let result: Vec<serde_json::Value> = items
        .into_iter()
        .take(limit)
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "stockCode": r.stock_code,
                "stockName": r.stock_name,
                "originalAnalysisId": r.original_analysis_id,
                "asOfDate": r.as_of_date,
                "hindsightDate": r.hindsight_date,
                "minConfidenceThreshold": r.min_confidence_threshold,
                "reflectionDepth": r.reflection_depth,
                "actualOutcome": r.actual_outcome,
                "whatWentWrong": r.what_went_wrong,
                "missedSignals": r.missed_signals,
                "fixForFuture": r.fix_for_future,
                "decisionJson": r.decision_json,
                "blackboardSnapshot": r.blackboard_snapshot,
                "status": r.status,
                "createdAt": r.created_at,
            })
        })
        .collect();
    Ok(result)
}

/// 删除单条反思记录
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "删除反思记录")]
#[tauri::command]
pub async fn delete_reflection(
    state: State<'_, AppState>,
    reflection_id: String,
) -> Result<(), String> {
    use axagent_entities::stock_reflections;
    use sea_orm::EntityTrait;
    stock_reflections::Entity::delete_by_id(&reflection_id)
        .exec(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("删除反思记录失败: {e}"))
        })?;
    Ok(())
}

/// 手动触发反思复盘工作流（在前端复盘 tab 点击"开始反思"时调用）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "手动触发反思复盘")]
#[tauri::command]
pub async fn run_reflection_now(
    state: State<'_, AppState>,
    stock_code: String,
    stock_name: String,
    as_of_date: String,
    actual_outcome: String,
    reflection_depth: Option<String>,
) -> Result<String, String> {
    let db = state.harness.db();
    let client = &state.astock_client;
    let engine = &state.work_engine;
    let vs = &state.vector_store;
    let mk = state.harness.master_key();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let depth = reflection_depth.unwrap_or_else(|| "light".to_string());

    // Bug 3 修复: 前端表单未让用户填 stockName(避免冗余输入),
    // 后端必须用 stock_code 反查股票名,确保反思历史 / RAG 索引都有正确名称。
    // 失败时回退到原值(空字符串),但不阻塞流程。
    let resolved_name = if stock_name.trim().is_empty() {
        match client.get_quote(&stock_code).await {
            Ok(q) if !q.name.is_empty() => q.name,
            Ok(_) => stock_code.clone(),
            Err(e) => {
                tracing::warn!(
                    "[run_reflection_now] 无法获取 {} 名称: {e},使用代码占位",
                    stock_code
                );
                stock_code.clone()
            },
        }
    } else {
        stock_name.clone()
    };

    crate::commands::stock_workflow::run_reflection_workflow(
        db,
        client,
        engine,
        vs,
        mk,
        &stock_code,
        &resolved_name,
        "", // original_analysis_id — 手动触发时无原始决策,run_reflection_workflow 已处理跳过
        &actual_outcome,
        // v008 (C3 借鉴): 4 个结构化 outcome 变量
        // 手动反思场景: 用户没传 raw/alpha,留 None 走 fallback 显示 "n/a"
        None,
        None,
        None,
        None,
        &as_of_date,
        &today,
        0u8, // min_confidence_threshold — 手动触发时全量
        &depth,
        // [B2/B3 借鉴] 手动反思场景无 B1 阶段落盘的 pending row,传 None 走 INSERT 路径
        None,
        // [方向3] 手动反思也持久化 trajectory，为 ExperiencePipeline 提供数据源
        Some(&state.trajectory_storage),
    )
    .await
}

/// 获取某只股票最近未处理的参数调整建议
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取参数调整建议")]
#[tauri::command]
pub async fn list_param_suggestions(
    state: State<'_, AppState>,
    stock_code: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    use axagent_entities::stock_reflections;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let db = state.harness.db();
    let mut query = stock_reflections::Entity::find()
        .filter(stock_reflections::Column::Status.eq("completed"))
        .order_by(stock_reflections::Column::CreatedAt, sea_orm::Order::Desc);
    if let Some(ref code) = stock_code {
        query = query.filter(stock_reflections::Column::StockCode.eq(code));
    }
    let items = query.all(db).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询参数建议失败: {e}"))
    })?;

    let result: Vec<serde_json::Value> = items
        .into_iter()
        .filter_map(|r| {
            let dj = r.decision_json.as_deref()?;
            let parsed: serde_json::Value = serde_json::from_str(dj).ok()?;
            let suggestions = parsed.get("params_suggestion")?;
            if suggestions.as_array()?.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "reflectionId": r.id,
                "stockCode": r.stock_code,
                "stockName": r.stock_name,
                "asOfDate": r.as_of_date,
                "createdAt": r.created_at,
                "suggestions": suggestions,
            }))
        })
        .take(20)
        .collect();
    Ok(result)
}

/// 应用用户选中的参数调整建议到 stock-analysis 模板变量
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "应用参数调整建议")]
#[tauri::command]
pub async fn apply_param_suggestions(
    state: State<'_, AppState>,
    updates: Vec<serde_json::Value>,
) -> Result<(), String> {
    use axagent_entities::workflow_template;
    use sea_orm::sea_query::Expr;
    use sea_orm::{EntityTrait, QueryFilter};

    let db = state.harness.db();

    // 1. 读取 stock-analysis 模板的 variables
    let tmpl = workflow_template::Entity::find_by_id("stock-analysis")
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("读取模板失败: {e}"))
        })?
        .ok_or_else(|| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail("stock-analysis 模板不存在")
        })?;

    let mut vars: Vec<serde_json::Value> =
        tmpl.variables.as_deref().and_then(|v| serde_json::from_str(v).ok()).unwrap_or_default();

    // 2. 逐个更新
    for update in &updates {
        let param_name = update
            .get("param")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorResponse::new(wf_err::INTERNAL).with_detail("缺少 param"))?;
        let new_value = update
            .get("value")
            .ok_or_else(|| ErrorResponse::new(wf_err::INTERNAL).with_detail("缺少 value"))?;

        // 找到匹配的变量并更新 value
        let mut found = false;
        for v in &mut vars {
            if v.get("name").and_then(|n| n.as_str()) == Some(param_name) {
                if let Some(val_field) = v.as_object_mut() {
                    // 只允许修改 number 类型的变量，跳过 secret
                    if val_field.get("var_type").and_then(|t| t.as_str()) == Some("number")
                        && val_field.get("is_secret") != Some(&serde_json::Value::Bool(true))
                    {
                        val_field.insert("value".into(), new_value.clone());
                        found = true;
                    }
                }
                break;
            }
        }
        if !found {
            tracing::warn!("[param_suggestions] 参数 {param_name} 不存在或不可修改，跳过");
        }
    }

    // 3. 持久化
    let vars_json = serde_json::to_string(&vars).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("序列化失败: {e}"))
    })?;
    let now = chrono::Utc::now().timestamp_millis();
    workflow_template::Entity::update_many()
        .col_expr(workflow_template::Column::Variables, Expr::value(vars_json))
        .col_expr(workflow_template::Column::UpdatedAt, Expr::value(now))
        .filter(workflow_template::Column::Id.eq("stock-analysis"))
        .exec(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("更新模板变量失败: {e}"))
        })?;

    tracing::info!("[param_suggestions] 已应用 {} 项参数调整到 stock-analysis 模板", updates.len());

    // 4. 同时触发策略权重重新计算，使 params_suggestion 间接影响荐股权重
    let _ = axagent_analysis_engine::evolution_drift::recalc_and_persist(db, "manual", None, None)
        .await
        .map(|(written, _)| {
            tracing::info!("[param_suggestions] 参数调整触发策略权重重算，{written} 项更新");
        })
        .map_err(|e| tracing::warn!("[param_suggestions] 策略权重重算失败: {e}"));

    Ok(())
}

// ── Path 1: WFO 参数校准 ──
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "校准组合管理参数")]
#[tauri::command]
pub async fn calibrate_portfolio_mgr_params(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use axagent_analysis_engine::portfolio_formula::{
        PortfolioMgrParamSet, score_param_set, try_parse_param_suggestion,
    };
    use axagent_entities::stock_reflections;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let db = state.harness.db();

    let reflections = stock_reflections::Entity::find()
        .filter(stock_reflections::Column::Status.eq("completed"))
        .filter(stock_reflections::Column::ParameterSuggestionsJson.is_not_null())
        .order_by(stock_reflections::Column::CreatedAt, sea_orm::Order::Desc)
        .all(db)
        .await
        .map_err(|e| format!("查询反思失败: {e}"))?;

    let suggestions: Vec<(String, Option<PortfolioMgrParamSet>)> = reflections
        .iter()
        .filter_map(|r| {
            let verdict = r.verdict.as_deref()?.to_string();
            let params_json = r.parameter_suggestions_json.as_deref()?;
            let params = try_parse_param_suggestion(params_json);
            Some((verdict, params))
        })
        .collect();

    if suggestions.is_empty() {
        return Ok(serde_json::json!({
            "bestParams": null, "grid": [],
            "totalReflections": reflections.len(), "parsedSuggestions": 0,
            "message": "没有可解析的参数建议，请先生成反思（reflection）"
        }));
    }

    let grid = PortfolioMgrParamSet::default_grid();
    let mut results: Vec<serde_json::Value> = grid
        .iter()
        .map(|params| {
            let score = score_param_set(params, &suggestions);
            serde_json::json!({
                "buyThreshold": params.buy_threshold,
                "increaseThreshold": params.increase_threshold,
                "holdThreshold": params.hold_threshold,
                "watchThreshold": params.watch_threshold,
                "reduceThreshold": params.reduce_threshold,
                "capExtreme": params.cap_extreme,
                "capHigh": params.cap_high,
                "capMid": params.cap_mid,
                "score": (score * 10000.0).round() / 100.0,
            })
        })
        .collect();

    results.sort_by(|a, b| {
        b.get("score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .partial_cmp(&a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(serde_json::json!({
        "bestParams": results.first(),
        "grid": results,
        "totalReflections": reflections.len(),
        "parsedSuggestions": suggestions.len(),
    }))
} // ── R1 复盘→进化：EvolutionDriftPanel 命令 ──

/// 查询进化漂移仪表盘（前端 EvolutionDriftPanel 主页用）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "查询进化漂移仪表盘")]
#[tauri::command]
pub async fn get_evolution_drift_dashboard(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<axagent_analysis_engine::evolution_drift::EvolutionDriftDashboard, String> {
    let db = state.harness.db();
    axagent_analysis_engine::evolution_drift::get_dashboard(db, as_of_date.as_deref()).await
}

/// 拉取某条 (strategy, period) 的权重时间线
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取权重时间线")]
#[tauri::command]
pub async fn get_evolution_drift_timeline(
    state: State<'_, AppState>,
    strategy_id: String,
    period: String,
    limit: Option<u32>,
) -> Result<Vec<axagent_analysis_engine::evolution_drift::TimelinePoint>, String> {
    let db = state.harness.db();
    axagent_analysis_engine::evolution_drift::get_timeline(
        db,
        &strategy_id,
        &period,
        limit.unwrap_or(60),
    )
    .await
}

/// 拉取近期决策一致性分数趋势（Phase 3: 双视角一致性趋势图）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取一致性分数历史")]
#[tauri::command]
pub async fn get_agreement_score_history(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    use axagent_entities::strategy_performance;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let limit = limit.unwrap_or(50) as u64;
    let rows = strategy_performance::Entity::find()
        .filter(strategy_performance::Column::AgreementScore.is_not_null())
        .order_by_desc(strategy_performance::Column::CreatedAt)
        .limit(limit)
        .all(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("查询 agreement_score 历史失败: {e}"))
        })?;

    let result: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "exitAt": r.exit_at,
                "agreementScore": r.agreement_score.unwrap_or(0),
                "stockCode": r.stock_code,
                "stockName": r.stock_name,
                "returnPct": r.return_pct,
                "wasCorrect": r.was_correct,
            })
        })
        .collect();
    Ok(result)
}

/// 手动触发权重重算（用户在前端 EvolutionDriftPanel 点"立即重算"时使用）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "手动重算策略权重")]
#[tauri::command]
pub async fn manual_recalc_strategy_weights(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<serde_json::Value, String> {
    let db = state.harness.db();
    let (written, new_weights) = axagent_analysis_engine::evolution_drift::recalc_and_persist(
        db,
        "manual",
        None,
        as_of_date.as_deref(),
    )
    .await?;
    // 同时返回当前生效的 weights 便于前端 refresh
    let flat: Vec<(String, String, f64)> =
        new_weights.into_iter().map(|((s, p), w)| (s, p, w)).collect();
    Ok(serde_json::json!({
        "written": written,
        "currentWeights": flat,
    }))
}

/// 把"当前生效的策略权重"组装成 reco_strategy_weights JSON,
/// 由前端 recommendStocks 时一并传给模板 vars。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取荐股策略权重")]
#[tauri::command]
pub async fn get_reco_strategy_weights(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.harness.db();
    let weights = axagent_analysis_engine::evolution_drift::load_current_weights(db).await?;
    // 转成 JSON 对象 {"trend_short": 1.2, ...}
    let mut obj = serde_json::Map::new();
    for ((s, p), w) in weights {
        let key = format!("{s}_{p}");
        obj.insert(key, serde_json::json!(w));
    }
    Ok(serde_json::Value::Object(obj))
}

// ─── P2-6: RealtimeMonitor T+0 自动重跑配置 ───

/// 查询 T+0 配置
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "查询T+0配置")]
#[tauri::command]
pub async fn get_t0_config(
    state: State<'_, AppState>,
) -> Result<axagent_analysis_engine::monitor::TZeroConfig, String> {
    let monitor = state.stock_monitor.get().ok_or_else(|| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail("RealtimeMonitor 未初始化")
    })?;
    Ok(monitor.t0_config().await)
}

/// 更新 T+0 配置
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "更新T+0配置")]
#[tauri::command]
pub async fn set_t0_config(
    state: State<'_, AppState>,
    config: axagent_analysis_engine::monitor::TZeroConfig,
) -> Result<(), String> {
    let monitor = state.stock_monitor.get().ok_or_else(|| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail("RealtimeMonitor 未初始化")
    })?;
    monitor.set_t0_config(config).await;
    Ok(())
}

// ── P0-1: 证据质量驱动的决策权重 ──

/// 计算证据质量驱动的分析师权重（结合市场环境、时间维度、历史表现）
///
/// 前端分析页面生成 consensus 时调用此命令替代简单阈值投票。
/// 传入市场环境信息、分析师报告、历史权重，返回每个分析师的最终权重和共识结果。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "计算证据质量权重")]
#[tauri::command]
pub fn compute_evidence_weights(
    request: EvidenceWeightRequest,
) -> Result<EvidenceWeightReport, String> {
    Ok(evidence_weight::compute_evidence_weights(request))
}

// ── P0-2: 回测→Prompt 优化 ──

/// 分析回测结果，生成分析师反馈报告
///
/// 输入回测参与记录，输出每位分析师的表现分析、趋势判断和 Prompt 调整建议。
/// 前端 BacktestPanel 完成回测后调用此命令，展示分析和调整建议。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "分析回测反馈")]
#[tauri::command]
pub fn analyze_backtest_feedback(
    participations: Vec<backtest_feedback::AnalysisParticipation>,
) -> Result<backtest_feedback::BacktestFeedbackReport, String> {
    let input = backtest_feedback::FeedbackInput { participations };
    Ok(backtest_feedback::analyze_backtest_feedback(input))
}

// ── P2-1: NLU 意图解析 ──

/// 解析自然语言分析意图
///
/// 输入"调研茅台短线""分析宁德时代中线"等自然语言，返回结构化分析请求参数。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "解析自然语言分析意图")]
#[tauri::command]
pub fn parse_analysis_intent(
    input: String,
) -> Result<axagent_analysis_engine::intent_parser::ParsedIntent, String> {
    Ok(axagent_analysis_engine::intent_parser::parse_analysis_intent(&input))
}

// ── P1-2: VLM 截图导入持仓 ──

/// 解析 VLM 截图识别结果，返回结构化持仓数据
///
/// 前端将截图发送给 vision 模型后，将 VLM 的文本输出传入此命令进行结构化解析。
/// VLM 调用由前端通过现有 conversation 系统完成（复用已有的 LLM provider 适配层）。
///
/// 使用流程：
/// 1. 前端选择截图 → 调用 conversation 系统发送给 vision 模型
/// 2. 前端将 VLM 返回的文本传入本命令
/// 3. 后端解析并返回结构化持仓列表
/// 4. 前端确认后逐条调用 add_portfolio_holding
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "解析VLM持仓截图")]
#[tauri::command]
pub fn parse_vlm_portfolio_screenshot(
    raw_vlm_output: String,
) -> Result<axagent_analysis_engine::vlm_import::VlmParseResult, String> {
    Ok(axagent_analysis_engine::vlm_import::parse_vlm_output(&raw_vlm_output))
}

/// 解析 CSV 交易记录文件（不写入数据库，仅预览）
///
/// 支持 通达信/东方财富/通用 CSV 格式，自动识别中文列名。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "解析CSV交易记录")]
#[tauri::command]
pub fn parse_trades_csv(
    file_path: String,
) -> Result<Vec<axagent_analysis_engine::trade_import::ImportRow>, String> {
    axagent_analysis_engine::trade_import::parse_csv(&file_path)
}

/// 批量导入交易记录到数据库
///
/// 内部调用 `batch_import_trades`：写入 trades 表 + 同步更新 portfolio_holdings。
/// 支持查重（同股票+同方向+同日期+同价格+同数量跳过）。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "批量导入交易记录")]
#[tauri::command]
pub async fn import_trades(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<axagent_analysis_engine::trade_import::ImportSummary, String> {
    let rows = axagent_analysis_engine::trade_import::parse_csv(&file_path)?;
    let db = state.harness.db();
    axagent_analysis_engine::trade_import::batch_import_trades(db, &rows).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("批量导入交易失败: {e}"))
            .to_string()
    })
}

/// 批量导入 VLM 识别的持仓
///
/// 一步完成：解析 VLM 输出 → 批量写入 portfolio_holdings
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "批量导入VLM持仓")]
#[tauri::command]
pub async fn import_portfolio_from_vlm(
    state: State<'_, AppState>,
    raw_vlm_output: String,
    replace_existing: Option<bool>,
) -> Result<serde_json::Value, String> {
    use axagent_analysis_engine::vlm_import::{holdings_to_import_params, parse_vlm_output};
    use axagent_entities::portfolio_holdings;
    use sea_orm::{EntityTrait, Set};

    let parsed = parse_vlm_output(&raw_vlm_output);
    if !parsed.success {
        return Ok(serde_json::json!({
            "success": false,
            "error": parsed.error,
            "holdings": [],
        }));
    }

    let db = state.harness.db();

    // 可选：清除旧持仓
    if replace_existing.unwrap_or(false) {
        let _ = portfolio_holdings::Entity::delete_many().exec(db).await;
    }

    let params = holdings_to_import_params(&parsed.holdings);
    let mut imported = Vec::new();
    let mut errors = Vec::new();

    for p in &params {
        let now_ms = chrono::Utc::now().timestamp_millis();
        match portfolio_holdings::Entity::insert(portfolio_holdings::ActiveModel {
            id: Set(uuid::Uuid::new_v4().to_string()),
            stock_code: Set(p.stock_code.clone()),
            stock_name: Set(p.stock_name.clone()),
            shares: Set(p.shares),
            avg_cost: Set(p.avg_cost),
            notes: Set(None),
            created_at: Set(now_ms),
            updated_at: Set(now_ms),
        })
        .exec(db)
        .await
        {
            Ok(_) => imported.push(p.stock_code.clone()),
            Err(e) => errors.push(format!("{}: {}", p.stock_code, e)),
        }
    }

    Ok(serde_json::json!({
        "success": errors.is_empty() || !imported.is_empty(),
        "imported": imported.len(),
        "failed": errors.len(),
        "stockCodes": imported,
        "errors": if errors.is_empty() { None } else { Some(errors) },
    }))
}

// ── P3: 快速回测模式（采样+持有期模拟）──
//
// 借鉴 TradingAgents 的轻量回测设计：
// 在指定日期范围内按 N 日间隔采样，对每个采样日运行完整分析，
// 然后模拟持有 M 个交易日后计算收益率。
//
// 这是 full backtest_analysis 的精简版本：
// - 不需要 user interaction（自动采样+计算）
// - 只返回统计摘要，不逐笔记录

/// 快速回测请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickBacktestRequest {
    pub stock_code: String,
    /// 开始日期 YYYY-MM-DD
    pub start_date: String,
    /// 结束日期 YYYY-MM-DD
    pub end_date: String,
    /// 采样间隔（交易日），默认 10
    pub sample_interval: Option<u32>,
    /// 持有期（交易日），默认 20
    pub hold_days: Option<u32>,
    /// as-of 模式可选
    pub as_of_date: Option<String>,
}

/// 单次采样结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickBacktestSample {
    pub analysis_date: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub return_pct: f64,
    pub was_correct: bool,
    pub decision_action: String,
    pub decision_confidence: f64,
}

/// 快速回测结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickBacktestResult {
    pub stock_code: String,
    pub total_samples: usize,
    pub correct_count: usize,
    pub accuracy_pct: f64,
    pub avg_return_pct: f64,
    pub win_rate: f64,
    pub samples: Vec<QuickBacktestSample>,
    pub error: Option<String>,
}

/// 快速回测：采样运行 + 持有期模拟
///
/// 使用 as-of 模式回放历史数据，在每个采样日运行分析，然后查看持有期后的价格表现。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "快速回测")]
#[tauri::command]
pub async fn quick_backtest(
    state: State<'_, AppState>,
    request: QuickBacktestRequest,
) -> Result<QuickBacktestResult, String> {
    use axagent_astock_data::as_of;
    use chrono::NaiveDate;

    let stock_code = request.stock_code.clone();
    let sample_interval = request.sample_interval.unwrap_or(10).max(1);
    let hold_days = request.hold_days.unwrap_or(20).max(1);
    let start_date = NaiveDate::parse_from_str(&request.start_date, "%Y-%m-%d").map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("无效的开始日期: {e}"))
    })?;
    let end_date = NaiveDate::parse_from_str(&request.end_date, "%Y-%m-%d").map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("无效的结束日期: {e}"))
    })?;

    // 生成采样日期列表（按间隔采样）
    let mut sample_dates: Vec<String> = Vec::new();
    let mut current = start_date;
    while current <= end_date {
        // 粗略判断是否为交易日（跳过周末）
        if current.weekday().num_days_from_monday() < 5 {
            let date_str = current.format("%Y-%m-%d").to_string();
            sample_dates.push(date_str);
        }
        // 跳过 N 天
        for _ in 0..sample_interval {
            current = current.succ_opt().unwrap_or(current);
        }
    }

    // 限制采样数量以避免过长时间
    let max_samples = 50;
    if sample_dates.len() > max_samples {
        sample_dates.truncate(max_samples);
    }

    let mut samples = Vec::with_capacity(sample_dates.len());
    let mut correct_count = 0usize;
    let mut total_return = 0.0f64;

    for (i, analysis_date) in sample_dates.iter().enumerate() {
        // 获取采样日行情作为 entry price（使用请求级的 as-of 上下文）
        let entry_ctx = AsOfContext::parse_optional(request.as_of_date.as_deref())?;
        let entry_price = as_of::with_optional_asof(entry_ctx, async {
            let client = &state.astock_client;
            let klines = client.get_klines(&stock_code, "daily", 1).await.map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("获取 K 线失败({analysis_date}): {e}"))
            })?;
            Ok::<f64, String>(klines.first().map(|k| k.close).unwrap_or(0.0))
        })
        .await?;

        // 计算持有期后的 exit date
        let exit_date_str = {
            let base = NaiveDate::parse_from_str(analysis_date, "%Y-%m-%d").unwrap_or_default();
            let mut exit = base;
            let mut days_forward = 0;
            while days_forward < hold_days {
                exit = exit.succ_opt().unwrap_or(exit);
                if exit.weekday().num_days_from_monday() < 5 {
                    days_forward += 1;
                }
            }
            exit.format("%Y-%m-%d").to_string()
        };

        // 使用 as-of 模式获取退出日行情（fix D1: 以前使用同一个 as_of 上下文，无视 exit_date）
        let exit_ctx = AsOfContext::parse_optional(Some(&exit_date_str))?;
        let exit_price = as_of::with_optional_asof(exit_ctx, async {
            let client = &state.astock_client;
            let klines = client.get_klines(&stock_code, "daily", 1).await.map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("获取退出日 K 线失败({exit_date_str}): {e}"))
            })?;
            Ok::<f64, String>(klines.first().map(|k| k.close).unwrap_or(0.0))
        })
        .await?;

        let return_pct = if entry_price > 0.0 {
            ((exit_price - entry_price) / entry_price) * 100.0
        } else {
            0.0
        };

        let was_correct = return_pct > 0.0;
        if was_correct {
            correct_count += 1;
        }
        total_return += return_pct;

        let sample_result = QuickBacktestSample {
            analysis_date: analysis_date.clone(),
            entry_price,
            exit_price,
            return_pct,
            was_correct,
            decision_action: if return_pct > 0.0 {
                "买入".into()
            } else {
                "卖出/持有".into()
            },
            decision_confidence: 50.0f64.min(50.0 + return_pct.abs()),
        };

        samples.push(sample_result);

        // 给后端喘息，避免请求过密
        if i < sample_dates.len() - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    let total = samples.len();
    let accuracy_pct = if total > 0 {
        (correct_count as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let avg_return_pct = if total > 0 {
        total_return / total as f64
    } else {
        0.0
    };

    Ok(QuickBacktestResult {
        stock_code,
        total_samples: total,
        correct_count,
        accuracy_pct,
        avg_return_pct,
        win_rate: accuracy_pct,
        samples,
        error: None,
    })
}

// ── 以下为插件探测发现的阻断性缺失命令修复 ──

/// 列出所有可用的股票分析工具名称（用于 Agent 配置页工具选择列表）
///
/// P2-8: 合并 G3 产业链工具（来自 axagent_analysis_engine::mcp_tools）。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "列出股票分析工具")]
#[tauri::command]
pub async fn list_stock_tools() -> Result<Vec<String>, String> {
    let mut tools = axagent_astock_data::mcp_tools::stock_mcp_tools();
    tools.extend(axagent_analysis_engine::mcp_tools::industry_chain_mcp_tools());
    let names: Vec<String> = tools
        .into_iter()
        .filter_map(|t| t.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();
    Ok(names)
}

/// 获取限售解禁时间表
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取限售解禁时间表")]
#[tauri::command]
pub async fn get_lockup_schedule(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Vec<axagent_astock_data::types::LockupSchedule>, String> {
    state.astock_client.get_lockup_schedule(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取解禁数据失败: {e}"))
            .to_string()
    })
}

/// 获取分红记录
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取分红记录")]
#[tauri::command]
pub async fn get_dividend_records(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Vec<axagent_astock_data::types::DividendRecord>, String> {
    state.astock_client.get_dividend_records(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取分红数据失败: {e}"))
            .to_string()
    })
}

/// 获取股票财务数据（对比面板使用）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取股票财务数据")]
#[tauri::command]
pub async fn get_stock_financials(
    state: State<'_, AppState>,
    stock_code: String,
) -> Result<Vec<axagent_harness::market_data::FinancialReport>, String> {
    state.astock_client.get_financials(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("获取财务数据失败: {e}"))
            .to_string()
    })
}

/// 演化漂移重算（EvolutionDriftPanel 重算按钮）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "演化漂移重算")]
#[tauri::command]
pub async fn stock_evolution_recalc(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<serde_json::Value, String> {
    let db = state.harness.db();
    let (written, new_weights) = axagent_analysis_engine::evolution_drift::recalc_and_persist(
        db,
        "manual",
        None,
        as_of_date.as_deref(),
    )
    .await?;
    let flat: Vec<(String, String, f64)> =
        new_weights.into_iter().map(|((s, p), w)| (s, p, w)).collect();
    Ok(serde_json::json!({
        "written": written,
        "currentWeights": flat,
    }))
}

// ── 自改进分析循环（Loop Engineering 股票域闭环）──
//
// 对接上游 harness::SelfImprovingRound + agent::SelfImprovementExecutor，
// 让本地股票业务复用上游"执行 → 自评估 → 识别不足 → 注入改进提示 → 重新生成"
// 的回合制迭代策略，实现功能性闭环。
//   - trait + DTO 在 axagent-harness（foundation）
//   - 通用执行器在 axagent-agent（consumer）
//   - 领域实现 StockAnalysisRound 在 axagent-stock-analysis（implementor）
//   - 命令注册在 src/commands（wiring）

/// 运行自改进股票分析循环
///
/// 在 Loop Engineering 基础设施上执行多轮股票分析：
/// 1. `task` — 分析任务，如 "分析 600519.SH"
/// 2. `max_rounds` — 最大改进轮数（默认 3）
/// 3. 每轮自动补全薄弱维度，评估质量，决定是否继续改进
///
/// 返回最终分析报告 + 评估分数 + 轮次信息。
/// 即使中途出错也会返回已完成的回合记录（`partial: true`），便于前端展示部分结果。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "运行自改进股票分析循环")]
#[tauri::command]
pub async fn run_self_improving_stock_analysis(
    state: State<'_, AppState>,
    task: String,
    max_rounds: Option<u32>,
) -> Result<serde_json::Value, String> {
    let client = state.astock_client.clone();
    let round = StockAnalysisRound::new(client);
    let config = SelfImprovementConfig::new(
        max_rounds.unwrap_or(3),
        0.80, // 收敛阈值：评估分数高于此值直接 Accept
        3,    // 连续无进展多少次后 Escalate
    );
    let mut executor = SelfImprovementExecutor::new(Box::new(round), config);

    match executor.run(&task).await {
        Ok(output) => Ok(serde_json::json!({
            "text": output.text,
            "totalRounds": output.total_rounds,
            "finalScore": output.final_evaluation.score,
            "confidence": output.final_evaluation.confidence,
            "strengths": output.final_evaluation.strengths,
            "gaps": output.final_evaluation.gaps,
        })),
        Err(e) => {
            // 即使出错也尝试返回已有的 round_history
            let partial = executor.round_history();
            if let Some(last) = partial.last() {
                Ok(serde_json::json!({
                    "text": last.output.clone(),
                    "totalRounds": partial.len(),
                    "finalScore": last.evaluation.as_ref().map(|e| e.score),
                    "error": e.to_string(),
                    "partial": true,
                }))
            } else {
                Err(ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("自改进分析失败: {e}"))
                    .to_string())
            }
        },
    }
}

// ── 估值参数配置命令 ──

/// 估值参数配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuationParams {
    /// 永续增长率（默认 0.03 = 3%）
    pub perpetual_growth: f64,
    /// 折现率（默认 0.10 = 10%）
    pub discount_rate: f64,
    /// 默认增长率（无数据时使用，默认 0.08 = 8%）
    pub default_growth: f64,
    /// 最小增长率（默认 0.02 = 2%）
    pub min_growth: f64,
    /// 最大增长率（默认 0.30 = 30%）
    pub max_growth: f64,
    /// 预测年数（默认 5 年）
    pub forecast_years: i32,
    /// 格雷厄姆公式中的 AAA 企业债收益率基准（默认 4.4）
    pub bond_yield: f64,
}

impl Default for ValuationParams {
    fn default() -> Self {
        Self {
            perpetual_growth: 0.03,
            discount_rate: 0.10,
            default_growth: 0.08,
            min_growth: 0.02,
            max_growth: 0.30,
            forecast_years: 5,
            bond_yield: 4.4,
        }
    }
}

const VALUATION_PARAMS_KEY: &str = "valuation_params";

/// 从数据库加载估值参数，失败时返回默认值
/// 可供其他模块（如 init/services.rs 中的工具回调）复用
pub async fn load_valuation_params(db: &sea_orm::DatabaseConnection) -> ValuationParams {
    match axagent_dao::repo::settings::get_setting(db, VALUATION_PARAMS_KEY).await {
        Ok(Some(json_str)) => {
            serde_json::from_str(&json_str).unwrap_or_else(|_| ValuationParams::default())
        },
        Ok(None) => ValuationParams::default(),
        Err(_) => ValuationParams::default(),
    }
}

/// 为 compute_valuation 工具注入估值配置
/// 如果是 compute_valuation 工具，从数据库加载估值参数并注入到 arguments 中
pub async fn inject_valuation_config_for_tool(
    tool_name: &str,
    db: &sea_orm::DatabaseConnection,
    arguments: serde_json::Value,
) -> serde_json::Value {
    if tool_name == "compute_valuation" {
        let params = load_valuation_params(db).await;
        let config = serde_json::json!({
            "perpetualGrowth": params.perpetual_growth,
            "discountRate": params.discount_rate,
            "defaultGrowth": params.default_growth,
            "minGrowth": params.min_growth,
            "maxGrowth": params.max_growth,
            "forecastYears": params.forecast_years,
            "bondYield": params.bond_yield,
        });
        let mut args = arguments;
        if let Some(obj) = args.as_object_mut() {
            obj.insert("valuation_config".to_string(), config);
        }
        args
    } else {
        arguments
    }
}

#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取估值参数配置")]
#[tauri::command]
pub async fn get_valuation_params(state: State<'_, AppState>) -> Result<ValuationParams, String> {
    let params = get_valuation_params_inner(&state).await;
    Ok(params)
}

/// 内部函数：从数据库加载估值参数，失败时返回默认值
async fn get_valuation_params_inner(state: &State<'_, AppState>) -> ValuationParams {
    let db = state.harness.db();
    match axagent_dao::repo::settings::get_setting(db, VALUATION_PARAMS_KEY).await {
        Ok(Some(json_str)) => {
            serde_json::from_str(&json_str).unwrap_or_else(|_| ValuationParams::default())
        },
        Ok(None) => ValuationParams::default(),
        Err(_) => ValuationParams::default(),
    }
}

#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "保存估值参数配置")]
#[tauri::command]
pub async fn save_valuation_params(
    state: State<'_, AppState>,
    params: ValuationParams,
) -> Result<(), String> {
    let db = state.harness.db();
    let json_str =
        serde_json::to_string(&params).map_err(|e| format!("序列化估值参数失败: {}", e))?;
    axagent_dao::repo::settings::set_setting(db, VALUATION_PARAMS_KEY, &json_str)
        .await
        .map_err(|e| format!("保存估值参数失败: {}", e))
}
