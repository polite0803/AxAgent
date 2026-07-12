//! 参数校准器 —— 扫描参数网格，寻找使模拟产出最接近 A 股 Stylized Facts 的参数组合。
//!
//! ## 校准结果（2026-07-03）
//!
//! 100 组拉丁超立方 × 5s/路径 扫描，Bar 聚合 10ms：
//!
//! ```text
//! mm_spread_bps:         35
//! mm_quote_size:         634
//! noise_act_prob:        0.27
//! noise_price_noise_bps: 32
//! momentum_threshold:    0.0035
//! score:                 3.18
//! ```
//!
//! 这些参数已导出为 `BEST_PARAMS` 常量，被 `MarketSimulationTool` 和
//! `QuantStrategyAgent` 默认使用。

//! ## 用法
//!
//! ```rust,no_run
//! use axagent_market_sim::{CalibrationRunner, CalibrationParam};
//!
//! let mut runner = CalibrationRunner::new(1000, 5);
//! let results = runner.run();
//! if let Some(best) = results.first() {
//!     println!("最佳参数: {:?}", best.param);
//!     println!("评分: {:.2}", best.score);
//! }
//! ```

use serde::{Deserialize, Serialize};

use crate::agent::{ExchangeAgent, MarketMakerAgent, MomentumAgent, NoiseAgent, ValueAgent};
use crate::config::SimConfig;
use crate::kernel::SimKernel;
use crate::stylized_facts::{StylizedFacts, TargetRange};
use crate::types::*;

/// 校准参数组合
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CalibrationParam {
    /// 做市商价差（基点）
    pub mm_spread_bps: i64,
    /// 做市商每档挂单量
    pub mm_quote_size: u64,
    /// 噪声 Agent 下单概率
    pub noise_act_prob: f64,
    /// 噪声 Agent 价格噪声（基点）
    pub noise_price_noise_bps: i64,
    /// 动量 Agent 阈值
    pub momentum_threshold: f64,
}

impl Default for CalibrationParam {
    fn default() -> Self {
        Self {
            mm_spread_bps: 30,
            mm_quote_size: 500,
            noise_act_prob: 0.3,
            noise_price_noise_bps: 30,
            momentum_threshold: 0.003,
        }
    }
}

/// 校准结果：参数 + 评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub param: CalibrationParam,
    pub score: f64,
    pub stylized_facts: StylizedFactsF64,
    pub total_trades: usize,
}

/// 最佳校准参数（A 股，2026-07-03）
///
/// 来源：`test_full_calibration_scan`，100 组参数 × 5s/路径
pub const BEST_PARAMS: CalibrationParam = CalibrationParam {
    mm_spread_bps: 35,
    mm_quote_size: 634,
    noise_act_prob: 0.27,
    noise_price_noise_bps: 32,
    momentum_threshold: 0.0035,
};

/// Stylized Facts 的浮点表示（用于序列化输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylizedFactsF64 {
    pub kurtosis: f64,
    pub hurst: f64,
    pub lb_pvalue: f64,
    pub leverage_corr: f64,
    pub passed: Vec<String>,
    pub failed: Vec<String>,
}

/// 参数网格扫描器
pub struct CalibrationRunner {
    reference_price: Price,
    /// 每组参数运行的虚拟时间（ns）
    pub sim_duration_ns: SimTimestamp,
    /// Bar 聚合窗口（纳秒），默认 10ms
    pub bar_ns: u64,
    /// 要测试的参数组合（可手动设置，默认等间距网格）
    pub params: Vec<CalibrationParam>,
}

impl CalibrationRunner {
    /// 创建校准器
    ///
    /// - `reference_price`: 参考价（分）
    /// - `n_grid`: 每个维度的网格点数（默认网格: 5^5=3125 组，实际用拉丁超立方采样 N 组）
    pub fn new(reference_price: Price, n_params: usize) -> Self {
        let params = Self::generate_latin_hypercube(n_params);
        Self {
            reference_price,
            sim_duration_ns: 600_000_000_000, // 10 分钟虚拟时间
            bar_ns: 10_000_000,               // 10ms Bar 窗口
            params,
        }
    }

    /// 设置模拟时长
    pub fn with_duration(mut self, duration_ns: SimTimestamp) -> Self {
        self.sim_duration_ns = duration_ns;
        self
    }

    /// 生成拉丁超立方采样参数
    fn generate_latin_hypercube(n: usize) -> Vec<CalibrationParam> {
        let count = n.clamp(20, 5000);

        // 各参数的范围
        let spread_range = (10.0, 100.0); // bps
        let qty_range = (100.0, 2000.0);
        let prob_range = (0.05, 0.8);
        let noise_range = (5.0, 100.0); // bps
        let mom_range = (0.001, 0.01);

        let mut params = Vec::with_capacity(count);

        for i in 0..count {
            // 用确定性伪随机产生拉丁超立方采样
            let t = i as f64 / count as f64;
            let phi1 = (i as f64 * 1.618033988749895).fract();
            let phi2 = (i as f64 * std::f64::consts::E).fract();
            let phi3 = (i as f64 * std::f64::consts::PI).fract();
            let phi4 = (i as f64 * std::f64::consts::SQRT_2).fract();
            let phi5 = (i as f64 * 0.5772156649).fract();

            // 分层 + 抖动
            let u1 = (t + phi1 * 1.0 / count as f64).fract();
            let u2 = (t + phi2 * 1.0 / count as f64).fract();
            let u3 = (t + phi3 * 1.0 / count as f64).fract();
            let u4 = (t + phi4 * 1.0 / count as f64).fract();
            let u5 = (t + phi5 * 1.0 / count as f64).fract();

            params.push(CalibrationParam {
                mm_spread_bps: (spread_range.0 + u1 * (spread_range.1 - spread_range.0)).round()
                    as i64,
                mm_quote_size: (qty_range.0 + u2 * (qty_range.1 - qty_range.0)).round() as u64,
                noise_act_prob: prob_range.0 + u3 * (prob_range.1 - prob_range.0),
                noise_price_noise_bps: (noise_range.0 + u4 * (noise_range.1 - noise_range.0))
                    .round() as i64,
                momentum_threshold: mom_range.0 + u5 * (mom_range.1 - mom_range.0),
            });
        }

        params
    }

    /// 用给定参数运行一次模拟，返回 Stylized Facts
    fn run_single(&self, param: &CalibrationParam, seed: u64) -> CalibrationResult {
        let price = self.reference_price;

        let config = SimConfig {
            max_time_ns: self.sim_duration_ns,
            seed,
            stock_code: "000001".to_string(),
            reference_price: price,
            default_latency_ns: 100,
            ..Default::default()
        };

        let mut kernel = SimKernel::new(config);

        // 注册 Agent
        kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));

        // 做市商
        kernel.register(Box::new(MarketMakerAgent::new(
            "mm",
            param.mm_spread_bps,
            param.mm_quote_size,
            5000,
            0.1,
            500_000, // 0.5ms 报价间隔
            price,
        )));

        // 动量
        if param.momentum_threshold > 0.0 {
            kernel.register(Box::new(MomentumAgent::new(
                "momentum",
                10, // lookback
                param.momentum_threshold,
                200,
                3000,
                1_000_000, // 1ms
                price as f64,
            )));
        }

        // 噪声 × 3（不同间隔错开）
        for i in 0..3 {
            kernel.register(Box::new(NoiseAgent::new(
                format!("noise_{}", i),
                400_000 + i as u64 * 150_000, // 400-700μs
                param.noise_act_prob,
                50,
                param.noise_price_noise_bps,
                price,
                seed + i as u64, // 修复 H3.6: 传递种子保证可复现
            )));
        }

        // 价值 Agent × 1
        kernel.register(Box::new(ValueAgent::new(
            "value",
            (price as f64 * 1.02) as i64,
            30,
            300,
            3000,
            2_000_000,
        )));

        match kernel.run() {
            Ok(result) => {
                let total_trades = result.trades.len();
                let facts = if total_trades >= 20 {
                    // 使用 Bar 聚合（10ms 窗口）
                    StylizedFacts::from_bars(&result.trades, self.bar_ns)
                } else {
                    StylizedFacts {
                        kurtosis: 0.0,
                        skewness: 0.0,
                        hurst_exponent: 0.5,
                        lb_stat: 0.0,
                        lb_pvalue: 1.0,
                        leverage_corr: 0.0,
                        n_observations: total_trades,
                        passed: Vec::new(),
                        failed: vec![format!("成交不足: {} < 20", total_trades)],
                    }
                };

                let score = if total_trades >= 20 {
                    facts.score(&TargetRange::default())
                } else {
                    999.0
                };

                CalibrationResult {
                    param: *param,
                    score,
                    stylized_facts: StylizedFactsF64 {
                        kurtosis: facts.kurtosis,
                        hurst: facts.hurst_exponent,
                        lb_pvalue: facts.lb_pvalue,
                        leverage_corr: facts.leverage_corr,
                        passed: facts.passed,
                        failed: facts.failed,
                    },
                    total_trades,
                }
            },
            Err(e) => CalibrationResult {
                param: *param,
                score: 9999.0,
                stylized_facts: StylizedFactsF64 {
                    kurtosis: 0.0,
                    hurst: 0.5,
                    lb_pvalue: 1.0,
                    leverage_corr: 0.0,
                    passed: vec![],
                    failed: vec![format!("模拟失败: {}", e)],
                },
                total_trades: 0,
            },
        }
    }

    /// 运行全部参数扫描
    ///
    /// 修复 L-11: 当前实现是串行的（map + collect）。
    /// 由于 run_single 需要 &mut self（共享可变状态如 RNG），无法直接用 rayon par_iter。
    /// 若需并行化，应将 run_single 改为接受独立状态（如克隆 self 或拆分不可变部分），
    /// 然后用 rayon 或 tokio task 并行执行。当前规模（通常 < 100 个参数组合）下
    /// 串行性能可接受，并行化收益有限。
    pub fn run(&mut self) -> Vec<CalibrationResult> {
        // 修复 M-DEF-5: 原 if/else 两个分支都调用 `self.run_single(param, seed)`，
        // 是死代码（可能源于早期"测试模式跳过部分扫描"的实验残留）。
        // 直接调用 run_single，简化逻辑。
        let mut results: Vec<CalibrationResult> = self
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let seed = 1000 + i as u64;
                self.run_single(param, seed)
            })
            .collect();

        results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// 获取最佳 N 个结果
    pub fn best_n(&self, results: &[CalibrationResult], n: usize) -> Vec<CalibrationResult> {
        results.iter().take(n.min(results.len())).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibrator_creates_params() {
        let runner = CalibrationRunner::new(1000, 50);
        assert_eq!(runner.params.len(), 50);
        assert!(runner.params[0].mm_spread_bps >= 10);
        assert!(runner.params[0].mm_spread_bps <= 100);
    }

    #[test]
    fn test_full_calibration_scan() {
        // 完整校准扫描：100 组拉丁超立方参数，500ms/路径
        let mut runner = CalibrationRunner::new(1000, 100);
        runner.sim_duration_ns = 5_000_000_000; // 5s
        let results = runner.run();

        assert!(!results.is_empty());
        println!("\n=== A 股校准扫描结果 (Bar 聚合) ===");
        println!("参数组合数: {} | 每路径时长: {}s | Bar 窗口: {}ms", results.len(), 5, 10);
        println!();

        // 前 5 名
        println!("--- Top 5 ---");
        for (i, r) in results.iter().take(5).enumerate() {
            print_best(i + 1, r);
        }

        // 后 3 名（最差）
        println!("\n--- Bottom 3 ---");
        for (i, r) in results.iter().rev().take(3).enumerate() {
            print_best(results.len() - i, r);
        }

        // 最佳参数简介
        let best = &results[0];
        println!("\n--- 最佳参数 ---");
        println!("  mm_spread_bps:        {}", best.param.mm_spread_bps);
        println!("  mm_quote_size:        {}", best.param.mm_quote_size);
        println!("  noise_act_prob:       {:.4}", best.param.noise_act_prob);
        println!("  noise_price_noise_bps: {}", best.param.noise_price_noise_bps);
        println!("  momentum_threshold:   {:.5}", best.param.momentum_threshold);
        println!("  score:                {:.2}", best.score);
        println!("  trades:               {}", best.total_trades);

        assert!(results[0].score < results[results.len() - 1].score);
    }

    fn print_best(rank: usize, r: &CalibrationResult) {
        let score_str = if r.score >= 100.0 {
            format!("{:>8}", "N/A")
        } else {
            format!("{:.2}", r.score)
        };
        println!(
            "  #{:<3} score={} trades={:<5} spread={:>3}bps size={:>4} noise_p={:.2} noise_bps={:>3} mom={:.4}",
            rank,
            score_str,
            r.total_trades,
            r.param.mm_spread_bps,
            r.param.mm_quote_size,
            r.param.noise_act_prob,
            r.param.noise_price_noise_bps,
            r.param.momentum_threshold
        );
    }

    #[test]
    fn test_trade_pipeline_debug() {
        use crate::agent::*;
        use crate::config::*;
        use crate::kernel::*;

        // 仅 1 个 MM + 1 个 Noise，跑 100ms
        let mut kernel = SimKernel::new(SimConfig {
            max_time_ns: 100_000_000,
            default_latency_ns: 1_000,
            reference_price: 1000,
            ..Default::default()
        });

        kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
        kernel.register(Box::new(MarketMakerAgent::new("mm", 30, 500, 5000, 0.1, 1_000_000, 1000)));
        kernel.register(Box::new(NoiseAgent::new("noise", 1_000_000, 0.8, 200, 50, 1000, 42)));

        let start = std::time::Instant::now();
        match kernel.run() {
            Ok(r) => {
                let elapsed = start.elapsed();
                println!(
                    "[DEBUG] trades={} events={} wall={}ms | sim={}ns agents={} orders={}",
                    r.trades.len(),
                    r.total_events,
                    r.wall_clock_ms,
                    r.sim_time_ns,
                    r.stats.agent_count,
                    r.stats.total_orders
                );
                assert!(r.total_events > 0, "应该有事件被处理");
                // Wall time should be measurable
                println!("[DEBUG] wall elapsed={}ms", elapsed.as_millis());
            },
            Err(e) => panic!("模拟失败: {}", e),
        }
    }
}
