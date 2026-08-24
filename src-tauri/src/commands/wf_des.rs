// SPDX-License-Identifier: AGPL-3.0-only

//! Walk-Forward + DES 对比集成
//!
//! 用同一策略在两组管道上跑出评分并对比：
//!
//! ```text
//! 策略
//!  ├── WalkForward (历史 K 线) → WalkForwardReport
//!  └── DES 模拟 (Agent-Based)  → SimResult → MetricsReport
//!       └── 对比 → DeviationMetrics
//! ```
//!
//! WalkForward 三条前置条件 2026-07-22 全部满足：
//!   1. 校准通过 (score=3.18)
//!   2. QuantStrategyAgent 已就绪
//!   3. WalkForward 基线评分已建立 (MaCross 5/20: 9 folds, Sharpe=-7.14, MaxDD=10.16%)

use axagent_agent_macro::agent_command;
use serde::{Deserialize, Serialize};

use axagent_market_sim::{
    BEST_PARAMS, ExchangeAgent, MarketMakerAgent, NoiseAgent, SimConfig, SimKernel, SimResult,
    agent::QuantStrategyAgent, types::TradeRecord,
};
use axagent_quant::{
    Bar, EquityPoint, MaCrossStrategy, MetricsReport, Trade, WalkForward, WalkForwardConfig,
    WalkForwardReport,
};

// ── 请求 / 响应类型 ──────────────────────────────────────────────────────────

/// WF + DES 对比请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfDesRequest {
    /// 历史 K 线数据
    pub klines: Vec<Bar>,
    /// Walk-Forward 配置（train/test 窗口大小等）
    pub wf_config: WfDesWfConfig,
    /// DES 模拟配置
    pub des_config: WfDesDesConfig,
    /// 策略名称（当前仅支持 "ma_cross"）
    pub strategy_name: String,
}

/// Walk-Forward 子配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfDesWfConfig {
    pub train_days: i64,
    pub test_days: i64,
    pub step_days: Option<i64>,
    pub risk_free_annual: f64,
}

impl Default for WfDesWfConfig {
    fn default() -> Self {
        Self { train_days: 300, test_days: 100, step_days: None, risk_free_annual: 0.025 }
    }
}

/// DES 模拟子配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfDesDesConfig {
    pub stock_code: String,
    pub reference_price: i64,
    /// DES 模拟时长（纳秒），默认 30s
    pub sim_duration_ns: u64,
    /// 随机种子
    pub seed: u64,
    /// 策略初始资金
    pub initial_cash: f64,
    /// 策略唤醒间隔（纳秒），默认 500μs
    pub wakeup_interval_ns: u64,
}

impl Default for WfDesDesConfig {
    fn default() -> Self {
        Self {
            stock_code: "000001".into(),
            reference_price: 1000,
            sim_duration_ns: 30_000_000_000,
            seed: 42,
            initial_cash: 1_000_000.0,
            wakeup_interval_ns: 500_000,
        }
    }
}

/// WF + DES 对比报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WfDesReport {
    /// Walk-Forward 验证结果
    pub walkforward: WalkForwardReport,
    /// DES 模拟指标
    pub des_metrics: MetricsReport,
    /// DES 原始成交数
    pub des_total_trades: usize,
    /// 偏差指标
    pub deviation: DeviationMetrics,
}

/// 偏差指标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviationMetrics {
    /// Sharpe 偏差（DES - WF）
    pub sharpe_delta: f64,
    /// MaxDD 偏差（DES - WF，百分点）
    pub maxdd_delta: f64,
    /// WinRate 偏差（DES - WF，百分点）
    pub win_rate_delta: f64,
    /// 成交量偏度（DES 总成交额 / WF OOS 总成交额）
    pub volume_ratio: f64,
}

// ── 核心转换 ─────────────────────────────────────────────────────────────────

/// 将 DES 成交记录转换为 quant crate 的权益曲线 + Trade 列表 + MetricsReport
fn sim_trades_to_metrics(
    sim_trades: &[TradeRecord],
    strategy_agent_id: &str,
    stock_code: &str,
    initial_cash: f64,
) -> (Vec<EquityPoint>, Vec<Trade>, MetricsReport) {
    let mut cash = initial_cash;
    let mut position: u64 = 0;
    let mut cost_basis = 0.0;
    let mut equity_points: Vec<EquityPoint> = Vec::new();
    let mut trades: Vec<Trade> = Vec::new();

    // 筛选策略 Agent 的成交
    let mut agent_trades: Vec<&TradeRecord> = sim_trades
        .iter()
        .filter(|t| t.buyer_agent_id == strategy_agent_id || t.seller_agent_id == strategy_agent_id)
        .collect();
    agent_trades.sort_by_key(|t| t.timestamp);

    /// 将顺序索引映射为日历日期（每 28 天进一个月，避免 chrono 解析失败）
    fn idx_to_date(idx: u32) -> String {
        let m = 1 + idx / 28;
        let d = 1 + idx % 28;
        format!("2024-{:02}-{:02}", m.min(12), d.min(28))
    }

    for (trade_idx, trade) in agent_trades.into_iter().enumerate() {
        let trade_idx = trade_idx as u32;
        let is_buy = trade.buyer_agent_id == strategy_agent_id;
        let price = trade.price as f64;
        let qty = trade.quantity;
        let amount = qty as f64 * price;

        if is_buy {
            cash -= amount;
            // 更新持仓均价
            let total_cost = cost_basis * position as f64;
            position += qty;
            cost_basis = if position > 0 {
                (total_cost + amount) / position as f64
            } else {
                0.0
            };
        } else {
            cash += amount;
            let sell_qty = qty.min(position);
            let realized_pnl = if sell_qty > 0 && cost_basis > 0.0 {
                sell_qty as f64 * (price - cost_basis)
            } else {
                0.0
            };
            position = position.saturating_sub(sell_qty);
            trades.push(Trade {
                code: stock_code.to_string(),
                side: axagent_quant::Side::Short,
                quantity: sell_qty,
                price,
                amount,
                commission: 0.0,
                stamp_tax: 0.0,
                slippage: 0.0,
                timestamp: idx_to_date(trade_idx),
                reason: "des_sim".into(),
                realized_pnl,
            });
        }

        if is_buy {
            trades.push(Trade {
                code: stock_code.to_string(),
                side: axagent_quant::Side::Long,
                quantity: qty,
                price,
                amount,
                commission: 0.0,
                stamp_tax: 0.0,
                slippage: 0.0,
                timestamp: idx_to_date(trade_idx),
                reason: "des_sim".into(),
                realized_pnl: 0.0,
            });
        }

        let date = idx_to_date(trade_idx);
        let pos_value = position as f64 * price;
        equity_points.push(EquityPoint {
            date,
            equity: cash + pos_value,
            cash,
            position_value: pos_value,
        });
    }

    let metrics = if equity_points.len() >= 2 {
        MetricsReport::from_equity_curve(
            &equity_points,
            &trades,
            0.025,
            244.0, // A 股年化交易日
        )
    } else {
        MetricsReport { total_trades: trades.len(), ..Default::default() }
    };

    (equity_points, trades, metrics)
}

// ── 策略工厂 ──────────────────────────────────────────────────────────────────

fn build_strategy(name: &str) -> Result<Box<dyn axagent_quant::Strategy>, String> {
    match name {
        "ma_cross" => Ok(Box::new(MaCrossStrategy::new(5, 20))),
        other => Err(format!("未知策略: {}，当前仅支持 ma_cross", other)),
    }
}

// ── DES 模拟运行 ──────────────────────────────────────────────────────────────

fn run_des_simulation(
    strategy: Box<dyn axagent_quant::Strategy>,
    des: &WfDesDesConfig,
) -> Result<SimResult, String> {
    let config = SimConfig {
        max_time_ns: des.sim_duration_ns,
        seed: des.seed,
        stock_code: des.stock_code.clone(),
        reference_price: des.reference_price,
        tick_size: 1,
        ..Default::default()
    };

    let mut kernel = SimKernel::new(config);

    // 使用校准后的最佳参数配置做市商和噪声
    kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
    kernel.register(Box::new(MarketMakerAgent::new(
        "mm",
        BEST_PARAMS.mm_spread_bps,
        BEST_PARAMS.mm_quote_size,
        5000,
        0.1,
        200_000,
        des.reference_price,
    )));
    kernel.register(Box::new(NoiseAgent::new(
        "noise",
        300_000,
        BEST_PARAMS.noise_act_prob,
        50,
        BEST_PARAMS.noise_price_noise_bps,
        des.reference_price,
        des.seed,
    )));

    let quant_agent = QuantStrategyAgent::new(
        "strategy",
        strategy,
        &des.stock_code,
        des.reference_price,
        des.initial_cash,
        des.wakeup_interval_ns,
    );
    kernel.register(Box::new(quant_agent));

    kernel.run().map_err(|e| format!("DES 模拟失败: {}", e))
}

// ── Tauri 命令 ────────────────────────────────────────────────────────────────

/// 运行 Walk-Forward + DES 对比
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description =  "运行Walk-Forward与DES对比")]
#[tauri::command]
pub fn wf_des_integration(request: WfDesRequest) -> Result<WfDesReport, String> {
    if request.klines.is_empty() {
        return Err("K 线数据为空".to_string());
    }

    // 1. 构建策略
    let strategy_name = request.strategy_name.clone();
    build_strategy(&strategy_name)?; // 验证策略名称有效

    // 2. 运行 Walk-Forward
    let wf_config = WalkForwardConfig {
        train_days: request.wf_config.train_days,
        test_days: request.wf_config.test_days,
        step_days: request.wf_config.step_days,
        risk_free_annual: request.wf_config.risk_free_annual,
        ..Default::default()
    };
    let wf = WalkForward::new(wf_config);

    let rt =
        tokio::runtime::Runtime::new().map_err(|e| format!("创建 tokio runtime 失败: {}", e))?;

    let wf_report = rt
        .block_on(wf.run(
            |_| build_strategy(&strategy_name).map(|s| s as Box<dyn axagent_quant::Strategy>),
            request.klines,
        ))
        .map_err(|e| format!("Walk-Forward 失败: {}", e))?;

    // 3. 运行 DES 模拟
    let strategy = build_strategy(&strategy_name)?;
    let des_result = run_des_simulation(strategy, &request.des_config)?;

    // 4. 转换 DES 结果为可比较指标
    let (_des_eq, _des_trades, des_metrics) = sim_trades_to_metrics(
        &des_result.trades,
        "strategy",
        &request.des_config.stock_code,
        request.des_config.initial_cash,
    );

    // 5. 计算偏差
    let wf_sharpe = wf_report.aggregated_oos_metrics.sharpe;
    let des_sharpe = des_metrics.sharpe;
    let wf_maxdd = wf_report.aggregated_oos_metrics.max_drawdown_pct;
    let des_maxdd = des_metrics.max_drawdown_pct;
    let wf_win = wf_report.aggregated_oos_metrics.win_rate;
    let des_win = des_metrics.win_rate;

    // WF OOS 总成交额（近似）
    let wf_volume: f64 =
        wf_report.windows.iter().flat_map(|w| w.test_result.trades.iter()).map(|t| t.amount).sum();
    let des_volume: f64 =
        des_metrics.total_trades as f64 * request.des_config.reference_price as f64;

    let deviation = DeviationMetrics {
        sharpe_delta: des_sharpe - wf_sharpe,
        maxdd_delta: (des_maxdd - wf_maxdd) * 100.0,
        win_rate_delta: (des_win - wf_win) * 100.0,
        volume_ratio: if wf_volume > 0.0 {
            des_volume / wf_volume
        } else {
            0.0
        },
    };

    Ok(WfDesReport {
        walkforward: wf_report,
        des_metrics,
        des_total_trades: des_result.trades.len(),
        deviation,
    })
}
