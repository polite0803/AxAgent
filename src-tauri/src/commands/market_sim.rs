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

use axagent_market_sim::{
    ExchangeAgent, MarketMakerAgent, MomentumAgent, NoiseAgent, SimConfig, SimKernel, SimResult,
    ValueAgent,
};

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
            30,         // 30bps
            500,        // 500 股/档
            5000,       // 库存上限
            0.1,        // 库存偏移敏感度
            200_000,    // 200μs 刷新间隔
            reference_price,
        )));
    }

    // 动量
    let n_mom = config.momentum_agents.unwrap_or(1);
    for i in 0..n_mom {
        agents.push(Box::new(MomentumAgent::new(
            format!("momentum_{}", i),
            5,               // lookback
            0.003,           // 0.3% 阈值
            200,             // 200 股/次
            2000,            // 持仓上限
            500_000,         // 500μs 检查间隔
            reference_price as f64,
        )));
    }

    // 价值
    let n_val = config.value_agents.unwrap_or(1);
    for i in 0..n_val {
        agents.push(Box::new(ValueAgent::new(
            format!("value_{}", i),
            (reference_price as f64 * 1.02) as i64, // fair_value = 参考价 × 1.02
            30,            // 30bps 阈值
            300,           // 300 股/次
            3000,          // 持仓上限
            1_000_000,     // 1ms 检查间隔
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
        )));
    }

    agents
}

// ── Tauri 命令 ──

/// 运行市场模拟
///
/// 接受模拟请求参数，创建 DES 内核 + Agent，运行后返回统计结果。
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
#[tauri::command]
pub fn market_sim_agent_types() -> Vec<&'static str> {
    vec!["exchange", "market_maker", "momentum", "value", "noise"]
}

/// 返回默认模拟参数建议
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
