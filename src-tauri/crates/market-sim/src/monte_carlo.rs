//! 蒙特卡洛模拟引擎 —— 多路径 × 多场景 × 聚合评分。
//!
//! ## 用法
//!
//! 1. 配置场景列表（Normal / Bull / Bear / FlashCrash / HighVol）
//! 2. 为每个场景指定路径数
//! 3. 运行 → 每路径产生一个 SimResult
//! 4. 聚合 → 输出 RobustnessReport（含跨场景胜率、最差回撤、一致性评分）

use serde::{Deserialize, Serialize};

use crate::config::SimConfig;
use crate::kernel::SimKernel;
use crate::oracle::{BaselineOracle, DriftOracle, EventOracle, Oracle};
use crate::types::*;

// ── 场景类型 ──

/// 市场情景标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioType {
    /// 正常市场
    Normal,
    /// 牛市（温和上涨）
    Bull,
    /// 熊市（持续下跌）
    Bear,
    /// 闪崩（暴跌后反弹）
    FlashCrash,
    /// 高波动
    HighVolatility,
}

impl ScenarioType {
    pub fn label(&self) -> &str {
        match self {
            ScenarioType::Normal => "正常",
            ScenarioType::Bull => "牛市",
            ScenarioType::Bear => "熊市",
            ScenarioType::FlashCrash => "闪崩",
            ScenarioType::HighVolatility => "高波动",
        }
    }
}

/// 单场景配置
#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    pub scenario: ScenarioType,
    pub paths: usize,
}

// ── 聚合结果 ──

/// 单路径结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    pub seed: u64,
    pub total_events: u64,
    pub total_trades: u64,
    pub final_mid_price: Option<f64>,
    pub wall_clock_ms: u64,
}

/// 单场景聚合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario: ScenarioType,
    pub label: String,
    pub paths: usize,
    pub avg_total_trades: f64,
    pub avg_final_mid_price: Option<f64>,
    pub price_change_pct: Option<f64>, // (avg_final - ref) / ref
    pub path_results: Vec<PathResult>,
}

/// 鲁棒性报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobustnessReport {
    pub stock_code: String,
    pub reference_price: Price,
    pub total_paths: usize,
    pub scenario_results: Vec<ScenarioResult>,
    /// 策略生存率：在所有场景中都有正收益的路径比例
    pub survival_rate: f64,
    /// 场景一致性：各场景之间终止价格的标准差 / 均值
    pub consistency_score: f64,
    /// 最佳/最差场景
    pub best_scenario: String,
    pub worst_scenario: String,
}

// ── 蒙特卡洛引擎 ──

pub struct MonteCarloEngine {
    pub config: SimConfig,
    pub scenarios: Vec<ScenarioConfig>,
    /// Agent 构建函数（接受 seed + oracle，返回 Agent 列表）
    pub agent_builder: Box<dyn Fn(u64) -> Vec<Box<dyn crate::agent::traits::SimAgent>> + Send>,
}

impl MonteCarloEngine {
    /// 根据场景类型创建对应的 Oracle
    ///
    /// 修复 P1-1: 增加 seed 参数，调用 with_seed 系列构造方法。
    /// 蒙特卡洛的每条路径需要独立但可复现的 Oracle，原实现调用 ::new()
    /// 使用 StdRng::from_entropy() 初始化 RNG，导致即使蒙特卡洛的 seed
    /// 参数相同，每次运行结果也不同。现统一改为 with_seed(seed)，
    /// seed 由调用方按 (base_seed + path_index) 派生传入。
    fn make_oracle(&self, scenario: ScenarioType, seed: u64) -> Box<dyn Oracle> {
        let ref_price = self.config.reference_price;
        match scenario {
            ScenarioType::Normal => Box::new(BaselineOracle::with_seed(ref_price, 15, seed)),
            ScenarioType::Bull => Box::new(DriftOracle::with_seed(ref_price, 30, 20, seed)),
            ScenarioType::Bear => Box::new(DriftOracle::with_seed(ref_price, -50, 40, seed)),
            ScenarioType::FlashCrash => Box::new(EventOracle::flash_crash_with_seed(
                ref_price,
                60_000_000,
                120_000_000,
                seed,
            )),
            ScenarioType::HighVolatility => {
                Box::new(EventOracle::high_volatility_with_seed(ref_price, seed))
            },
        }
    }

    pub fn new(
        config: SimConfig,
        agent_builder: impl Fn(u64) -> Vec<Box<dyn crate::agent::traits::SimAgent>> + Send + 'static,
    ) -> Self {
        Self {
            scenarios: vec![ScenarioConfig { scenario: ScenarioType::Normal, paths: 30 }],
            config,
            agent_builder: Box::new(agent_builder),
        }
    }

    /// 设置场景列表
    pub fn with_scenarios(mut self, scenarios: Vec<ScenarioConfig>) -> Self {
        self.scenarios = scenarios;
        self
    }

    /// 运行蒙特卡洛模拟
    pub fn run(&mut self) -> RobustnessReport {
        let ref_price = self.config.reference_price;
        let stock_code = self.config.stock_code.clone();

        let mut scenario_results = Vec::new();

        for sc in &self.scenarios {
            let mut path_results = Vec::with_capacity(sc.paths);
            let base_seed = self.config.seed;

            for path_idx in 0..sc.paths {
                let seed = base_seed + path_idx as u64;

                // 修复 C4.7: 每次循环重新创建 Oracle，避免跨路径状态污染
                // （原实现 Oracle 在 path 循环外创建，signal_at 修改内部 last_time，
                // 后续路径的 dt_days 计算会基于上次的状态，导致场景价格漂移）
                // 修复 P1-1: 传入 seed 让 Oracle 可复现（seed = base_seed + path_idx），
                // 每条路径独立但相同 seed 重跑结果一致。
                let mut oracle = self.make_oracle(sc.scenario, seed);

                // 修复 C4.7: 使用固定时间 signal_at(1_000_000_000)，避免时间累加
                // （原实现使用 (path_idx + 1) * 1_000_000_000，时间累加导致 dt_days 越来越大）
                let oracle_signal = oracle.signal_at(1_000_000_000);
                let scenario_price = oracle_signal.fundamental_value;

                let mut cfg = self.config.clone();
                cfg.seed = seed;
                cfg.reference_price = scenario_price;

                let mut kernel = SimKernel::new(cfg);
                let agents = (self.agent_builder)(seed);
                for agent in agents {
                    kernel.register(agent);
                }

                match kernel.run() {
                    Ok(result) => {
                        path_results.push(PathResult {
                            seed,
                            total_events: result.total_events,
                            total_trades: result.stats.total_trades,
                            final_mid_price: result.final_mid_price,
                            wall_clock_ms: result.wall_clock_ms,
                        });
                    },
                    Err(_) => {
                        // 路径失败，记录空结果
                        path_results.push(PathResult {
                            seed,
                            total_events: 0,
                            total_trades: 0,
                            final_mid_price: None,
                            wall_clock_ms: 0,
                        });
                    },
                }
            }

            let avg_trades =
                path_results.iter().map(|p| p.total_trades as f64).sum::<f64>() / sc.paths as f64;

            let valid_prices: Vec<f64> =
                path_results.iter().filter_map(|p| p.final_mid_price).collect();

            let avg_price = if valid_prices.is_empty() {
                None
            } else {
                Some(valid_prices.iter().sum::<f64>() / valid_prices.len() as f64)
            };

            let price_change = avg_price.map(|p| (p - ref_price as f64) / ref_price as f64 * 100.0);

            scenario_results.push(ScenarioResult {
                scenario: sc.scenario,
                label: sc.scenario.label().to_string(),
                paths: sc.paths,
                avg_total_trades: avg_trades,
                avg_final_mid_price: avg_price,
                price_change_pct: price_change,
                path_results,
            });
        }

        // 计算跨场景指标
        let valid_changes: Vec<f64> =
            scenario_results.iter().filter_map(|s| s.price_change_pct).collect();

        let survival_rate = if !valid_changes.is_empty() {
            let positive = valid_changes.iter().filter(|&&c| c > 0.0).count();
            positive as f64 / valid_changes.len() as f64
        } else {
            0.0
        };

        let consistency_score = if valid_changes.len() >= 2 {
            let mean = valid_changes.iter().sum::<f64>() / valid_changes.len() as f64;
            let variance = valid_changes.iter().map(|c| (c - mean).powi(2)).sum::<f64>()
                / valid_changes.len() as f64;
            let stddev = variance.sqrt();
            if mean.abs() > 0.001 {
                stddev / mean.abs()
            } else {
                0.0
            }
        } else {
            0.0
        };

        let best = scenario_results
            .iter()
            .max_by(|a, b| {
                a.price_change_pct
                    .unwrap_or(0.0)
                    .partial_cmp(&b.price_change_pct.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.label.clone())
            .unwrap_or_default();

        let worst = scenario_results
            .iter()
            .min_by(|a, b| {
                a.price_change_pct
                    .unwrap_or(0.0)
                    .partial_cmp(&b.price_change_pct.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.label.clone())
            .unwrap_or_default();

        RobustnessReport {
            stock_code,
            reference_price: ref_price,
            total_paths: self.scenarios.iter().map(|s| s.paths).sum(),
            scenario_results,
            survival_rate,
            consistency_score: (consistency_score * 100.0).round() / 100.0,
            best_scenario: best,
            worst_scenario: worst,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{ExchangeAgent, MarketMakerAgent, NoiseAgent};

    #[test]
    fn test_monte_carlo_basic() {
        let config = SimConfig {
            max_time_ns: 5_000_000, // 5ms 快速运行
            stock_code: "000001".to_string(),
            reference_price: 1000,
            default_latency_ns: 100,
            seed: 42,
            ..Default::default()
        };

        let mut engine = MonteCarloEngine::new(config, |seed| {
            let price = 1000;
            vec![
                Box::new(ExchangeAgent::with_tick_size("exchange", 1)),
                Box::new(MarketMakerAgent::new("mm", 50, 500, 5000, 0.1, 200_000, price)),
                Box::new(NoiseAgent::new("noise", 300_000, 0.3, 50, 30, price, seed)),
            ]
        });

        engine.scenarios = vec![
            ScenarioConfig { scenario: ScenarioType::Normal, paths: 5 },
            ScenarioConfig { scenario: ScenarioType::Bull, paths: 5 },
        ];

        let report = engine.run();

        assert_eq!(report.total_paths, 10);
        assert_eq!(report.scenario_results.len(), 2);
        assert!(report.survival_rate >= 0.0);
        assert!(report.consistency_score >= 0.0);
    }
}
