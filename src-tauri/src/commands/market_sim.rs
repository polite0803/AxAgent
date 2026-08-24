// SPDX-License-Identifier: AGPL-3.0-only

//! 市场模拟命令 — 封装 `axagent-market-sim` SIM 内核的 Tauri IPC 接口。
//!
//! 允许前端/工作流在分析流程中运行多 Agent 市场模拟并读取结果。
//!
//! ## 使用方式
//!
//! ```typescript
//! const result = await invoke<SimRunResult>("market_sim_run", {
//!   request: {
//!     stockCode: "000001",
//!     referencePrice: 1000,
//!     maxSimTimeNs: 50_000_000,
//!     agents: ["exchange", "market_maker", "momentum", "noise", "value"]
//!   }
//! });
//! ```

use serde::{Deserialize, Serialize};

use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent_macro::agent_command;
use axagent_market_sim::{
    ExchangeAgent, MarketMakerAgent, MomentumAgent, NoiseAgent, SimConfig, SimKernel, SimResult,
    ValueAgent,
    agent::QuantStrategyAgent,
    monte_carlo::{MonteCarloEngine, ScenarioConfig, ScenarioType},
};
use axagent_quant::{BollStrategy, MaCrossStrategy, MacdStrategy, RsiStrategy, TurtleStrategy};

/// 前端传入的模拟请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimRunRequest {
    /// 股票代码
    pub stock_code: String,
    /// 参考价格（分）
    pub reference_price: i64,
    /// 最大模拟时间（纳秒），默认 50ms
    pub max_sim_time_ns: Option<u64>,
    /// 默认延迟（纳秒），默认 100ns
    pub default_latency_ns: Option<u64>,
    /// 随机种子，默认 42
    pub seed: Option<u64>,
    /// 模拟 Agent 配置——不传则使用默认组合
    pub agent_config: Option<AgentConfig>,
    /// 启用追踪日志
    pub trace: Option<bool>,
}

/// Agent 组合配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    /// 做市商数量
    pub market_makers: Option<u32>,
    /// 动量 Agent 数量
    pub momentum_agents: Option<u32>,
    /// 价值 Agent 数量
    pub value_agents: Option<u32>,
    /// 噪声 Agent 数量
    pub noise_agents: Option<u32>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            market_makers: Some(1),
            momentum_agents: Some(1),
            value_agents: Some(1),
            noise_agents: Some(2),
        }
    }
}

/// 模拟运行结果（转传到前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimRunResult {
    pub stock_code: String,
    pub reference_price: i64,
    pub total_events: u64,
    pub wall_clock_ms: u64,
    pub sim_time_ns: u64,
    pub final_mid_price: Option<f64>,
    pub agent_count: usize,
    pub stats: SimRunStats,
}

/// 轻量级统计（回传前端用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimRunStats {
    pub total_trades: u64,
    pub total_orders: u64,
    pub max_queue_depth: usize,
}

impl From<SimResult> for SimRunResult {
    fn from(sr: SimResult) -> Self {
        Self {
            stock_code: sr.stock_code,
            reference_price: sr.reference_price,
            total_events: sr.total_events,
            wall_clock_ms: sr.wall_clock_ms,
            sim_time_ns: sr.sim_time_ns,
            final_mid_price: sr.final_mid_price,
            agent_count: sr.stats.agent_count,
            stats: SimRunStats {
                total_trades: sr.stats.total_trades,
                total_orders: sr.stats.total_orders,
                max_queue_depth: sr.stats.max_queue_depth,
            },
        }
    }
}

// ── 辅助：使用默认参数创建 Agent 组合 ──

fn build_default_agents(
    reference_price: i64,
    config: &AgentConfig,
) -> Vec<Box<dyn axagent_market_sim::SimAgent>> {
    let mut agents: Vec<Box<dyn axagent_market_sim::SimAgent>> = Vec::new();

    // 交易所（始终需要）
    agents.push(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));

    // 做市商
    let n_mm = config.market_makers.unwrap_or(1);
    for i in 0..n_mm {
        agents.push(Box::new(MarketMakerAgent::new(
            format!("mm_{}", i),
            30,      // 30bps
            500,     // 500 股/档
            5000,    // 库存上限
            0.1,     // 库存偏移敏感度
            200_000, // 200μs 刷新间隔
            reference_price,
        )));
    }

    // 动量
    let n_mom = config.momentum_agents.unwrap_or(1);
    for i in 0..n_mom {
        agents.push(Box::new(MomentumAgent::new(
            format!("momentum_{}", i),
            5,       // lookback
            0.003,   // 0.3% 阈值
            200,     // 200 股/次
            2000,    // 持仓上限
            500_000, // 500μs 检查间隔
            reference_price as f64,
        )));
    }

    // 价值
    let n_val = config.value_agents.unwrap_or(1);
    for i in 0..n_val {
        agents.push(Box::new(ValueAgent::new(
            format!("value_{}", i),
            (reference_price as f64 * 1.02) as i64, // fair_value = 参考价 × 1.02
            30,                                     // 30bps 阈值
            300,                                    // 300 股/次
            3000,                                   // 持仓上限
            1_000_000,                              // 1ms 检查间隔
        )));
    }

    // 噪声
    let n_noise = config.noise_agents.unwrap_or(2);
    for i in 0..n_noise {
        agents.push(Box::new(NoiseAgent::new(
            format!("noise_{}", i),
            300_000 + i as u64 * 100_000, // 300-500μs 间隔（错开）
            0.3,                          // 30% 下单概率
            50,                           // 最大 50 股/单
            30,                           // 30bps 噪声
            reference_price,
            42 + i as u64, // seed
        )));
    }

    agents
}

// ── Tauri 命令 ──

/// 运行市场模拟
///
/// 接受模拟请求参数，创建 DES 内核 + Agent，运行后返回统计结果。
#[agent_command(domain = market_sim, safety = Caution, call_mode = StateInput, description = "运行市场模拟")]
#[tauri::command]
pub fn market_sim_run(request: SimRunRequest) -> Result<SimRunResult, String> {
    let config = SimConfig {
        max_time_ns: request.max_sim_time_ns.unwrap_or(50_000_000),
        seed: request.seed.unwrap_or(42),
        stock_code: request.stock_code.clone(),
        reference_price: request.reference_price,
        tick_size: 1,
        default_latency_ns: request.default_latency_ns.unwrap_or(100),
        trace: request.trace.unwrap_or(false),
    };

    let agent_cfg = request.agent_config.unwrap_or_default();
    let agents = build_default_agents(request.reference_price, &agent_cfg);

    let mut kernel = SimKernel::new(config);
    for agent in agents {
        kernel.register(agent);
    }

    match kernel.run() {
        Ok(result) => Ok(SimRunResult::from(result)),
        Err(e) => Err(format!("市场模拟失败: {}", e)),
    }
}

/// 返回市场模拟支持的 Agent 类型列表
#[agent_command(domain = market_sim, safety = Safe, call_mode = StateInput, description = "获取支持的 Agent 类型")]
#[tauri::command]
pub fn market_sim_agent_types() -> Vec<&'static str> {
    vec!["exchange", "market_maker", "momentum", "value", "noise"]
}

/// 返回默认模拟参数建议
#[agent_command(domain = market_sim, safety = Safe, call_mode = StateInput, description = "获取默认模拟参数")]
#[tauri::command]
pub fn market_sim_defaults() -> serde_json::Value {
    serde_json::json!({
        "maxSimTimeNs": 50_000_000,
        "defaultLatencyNs": 100,
        "referencePrice": 1000,
        "agentConfig": {
            "marketMakers": 1,
            "momentumAgents": 1,
            "valueAgents": 1,
            "noiseAgents": 2
        }
    })
}

/// 蒙特卡洛多场景模拟请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McSimRequest {
    pub stock_code: String,
    pub reference_price: i64,
    /// 最大模拟时间（纳秒），默认 50ms
    pub max_sim_time_ns: Option<u64>,
    /// 随机种子，默认 42
    pub seed: Option<u64>,
    /// 场景列表
    pub scenarios: Vec<McScenarioSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McScenarioSpec {
    pub scenario: String,
    pub paths: u32,
}

/// 蒙特卡洛模拟结果（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McSimResult {
    pub stock_code: String,
    pub reference_price: i64,
    pub total_paths: usize,
    pub survival_rate: f64,
    pub consistency_score: f64,
    pub best_scenario: String,
    pub worst_scenario: String,
    pub scenario_results: Vec<McScenarioResultItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McScenarioResultItem {
    pub scenario: String,
    pub label: String,
    pub paths: usize,
    pub avg_total_trades: f64,
    pub avg_final_mid_price: Option<f64>,
    pub price_change_pct: Option<f64>,
}

/// 运行蒙特卡洛多场景模拟
#[agent_command(domain = market_sim, safety = Caution, call_mode = StateInput, description = "运行蒙特卡洛模拟")]
#[tauri::command]
pub fn market_sim_run_mc(request: McSimRequest) -> Result<McSimResult, String> {
    let ref_price = request.reference_price;
    let stock_code = request.stock_code.clone();
    let max_time_ns = request.max_sim_time_ns.unwrap_or(50_000_000);
    let seed = request.seed.unwrap_or(42);
    let scenarios = request.scenarios.clone();

    let default_agents = move |_seed: u64| -> Vec<Box<dyn axagent_market_sim::SimAgent>> {
        vec![
            Box::new(ExchangeAgent::with_tick_size("exchange", 1)),
            Box::new(MarketMakerAgent::new("mm", 50, 500, 5000, 0.1, 200_000, ref_price)),
            Box::new(MomentumAgent::new(
                "momentum",
                5,
                0.003,
                200,
                2000,
                500_000,
                ref_price as f64,
            )),
            Box::new(ValueAgent::new(
                "value",
                (ref_price as f64 * 1.02) as i64,
                30,
                300,
                3000,
                1_000_000,
            )),
            Box::new(NoiseAgent::new("noise", 300_000, 0.3, 50, 30, ref_price, _seed)),
        ]
    };

    let config = SimConfig {
        max_time_ns,
        seed,
        stock_code: stock_code.clone(),
        reference_price: ref_price,
        tick_size: 1,
        ..Default::default()
    };

    let mut engine = MonteCarloEngine::new(config, default_agents);
    engine.scenarios = scenarios
        .iter()
        .map(|s| {
            let scenario_type = match s.scenario.as_str() {
                "bull" => ScenarioType::Bull,
                "bear" => ScenarioType::Bear,
                "flash_crash" => ScenarioType::FlashCrash,
                "high_vol" => ScenarioType::HighVolatility,
                _ => ScenarioType::Normal,
            };
            ScenarioConfig { scenario: scenario_type, paths: s.paths as usize }
        })
        .collect();

    let report = engine.run();

    Ok(McSimResult {
        stock_code: report.stock_code,
        reference_price: report.reference_price,
        total_paths: report.total_paths,
        survival_rate: (report.survival_rate * 1000.0).round() / 10.0,
        consistency_score: report.consistency_score,
        best_scenario: report.best_scenario,
        worst_scenario: report.worst_scenario,
        scenario_results: report
            .scenario_results
            .into_iter()
            .map(|sr| McScenarioResultItem {
                scenario: format!("{:?}", sr.scenario),
                label: sr.label,
                paths: sr.paths,
                avg_total_trades: (sr.avg_total_trades * 10.0).round() / 10.0,
                avg_final_mid_price: sr.avg_final_mid_price.map(|p| (p * 100.0).round() / 100.0),
                price_change_pct: sr.price_change_pct.map(|p| (p * 100.0).round() / 100.0),
            })
            .collect(),
    })
}

/// 量化策略模拟请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuantSimRequest {
    pub stock_code: String,
    pub reference_price: i64,
    pub strategy_name: String,
    pub max_sim_time_ms: Option<u64>,
    pub seed: Option<u64>,
}

/// 量化策略模拟结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuantSimRunResult {
    pub total_events: u64,
    pub total_trades: u64,
    pub final_mid_price: Option<f64>,
    pub wall_clock_ms: u64,
    pub strategy_name: String,
}

/// 运行量化策略模拟（在 DES 市场环境中运行 quant crate 策略）
#[agent_command(domain = market_sim, safety = Caution, call_mode = StateInput, description = "运行量化策略模拟")]
#[tauri::command]
pub fn market_sim_run_strategy(request: QuantSimRequest) -> Result<QuantSimRunResult, String> {
    let strategy: Box<dyn axagent_quant::Strategy> = match request.strategy_name.as_str() {
        "ma_cross" => Box::new(MaCrossStrategy::new(5, 20)),
        "macd" => Box::new(MacdStrategy::new(12, 26, 9)),
        "rsi" => Box::new(RsiStrategy::new(14, 70.0, 30.0).map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建 RSI 策略失败: {e}"))
        })?),
        "boll" => Box::new(BollStrategy::new(20, 2.0)),
        "turtle" => Box::new(TurtleStrategy::new(20, 10, 20, 2.0)),
        _ => {
            return Err(ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("未知策略: {}", request.strategy_name))
                .to_string());
        },
    };

    let config = SimConfig {
        max_time_ns: request.max_sim_time_ms.unwrap_or(500) * 1_000_000,
        seed: request.seed.unwrap_or(42),
        stock_code: request.stock_code.clone(),
        reference_price: request.reference_price,
        tick_size: 1,
        ..Default::default()
    };

    let mut kernel = SimKernel::new(config);
    kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
    kernel.register(Box::new(MarketMakerAgent::new(
        "mm",
        50,
        500,
        5000,
        0.1,
        200_000,
        request.reference_price,
    )));
    kernel.register(Box::new(NoiseAgent::new(
        "noise",
        300_000,
        0.3,
        50,
        30,
        request.reference_price,
        request.seed.unwrap_or(42),
    )));

    let quant_agent = QuantStrategyAgent::new(
        "strategy",
        strategy,
        &request.stock_code,
        request.reference_price,
        100_000.0, // 10 万初始资金
        500_000,   // 500μs 唤醒间隔
    );
    kernel.register(Box::new(quant_agent));

    match kernel.run() {
        Ok(result) => Ok(QuantSimRunResult {
            total_events: result.total_events,
            total_trades: result.stats.total_trades,
            final_mid_price: result.final_mid_price,
            wall_clock_ms: result.wall_clock_ms,
            strategy_name: request.strategy_name,
        }),
        Err(e) => Err(format!("策略模拟失败: {}", e)),
    }
}
