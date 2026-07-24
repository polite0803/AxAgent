//! DAG 集成验证测试 — 确保 simulation-validate 节点在工作流中正确运行。
//!
//! 覆盖 5 项验证：
//!   ① StrategyAgent 模式产出有效结果
//!   ② Rhai 解析 simulation_result
//!   ③ 置信度修正（高/中/低生存率）
//!   ④ 无 simulation_result 时无影响
//!   ⑤ 性能基准

use async_trait::async_trait;
use axagent_harness::indicators::sma;
use axagent_market_sim::{
    BEST_PARAMS, ExchangeAgent, MarketMakerAgent, MomentumAgent, NoiseAgent, SimConfig, SimKernel,
    SimResult, StrategyAgent,
};

// ── 验证①: StrategyAgent 模式产出有效结果 ──

#[test]
fn v1_strategy_mode_produces_valid_results() {
    for (action, desc) in
        &[("买入", "买入模式"), ("持有", "持有模式"), ("观望", "观望模式"), ("卖出", "卖出模式")]
    {
        let result = run_validation_sim(action, 1050, 950, 500, 500_000_000);

        assert!(result.total_events > 50, "{}: events={} 应 >50", desc, result.total_events);
        assert!(result.sim_time_ns > 0, "{}: sim_time_ns 应 >0", desc);

        eprintln!(
            "[V1] {}: events={:<5} trades={:<5} sim={}ms mid={:?}",
            desc,
            result.total_events,
            result.trades.len(),
            result.sim_time_ns / 1_000_000,
            result.final_mid_price,
        );
    }
}

// ── 验证②: Rhai 解析 simulation_result（纯逻辑验证） ──

/// 模拟 portfolio-mgr.rhai 中 simulation_result 解析逻辑的 Rust 版本
fn parse_simulation_result(json_str: &str, confidence: f64) -> (f64, String) {
    let val: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return (confidence, String::new()),
    };

    let mut note = String::new();
    let mut adj_confidence = confidence;

    if let Some(survival) = val.get("survivalRate").and_then(|v| v.as_f64()) {
        let survival_pct = (survival * 100.0).round() as u64;
        note = format!(" | 模拟:生存率={}%", survival_pct);

        if survival < 0.40 {
            adj_confidence = (adj_confidence * 0.65 * 100.0).round() / 100.0;
            note.push_str("(低生存率,置信×0.65)");
        } else if survival < 0.60 {
            adj_confidence = (adj_confidence * 0.85 * 100.0).round() / 100.0;
            note.push_str("(中等生存率,置信×0.85)");
        }
    }

    if let (Some(best), Some(worst), Some(cons)) = (
        val.get("bestScenario").and_then(|v| v.as_str()),
        val.get("worstScenario").and_then(|v| v.as_str()),
        val.get("consistencyScore").and_then(|v| v.as_f64()),
    ) {
        let cons_pct = (cons * 100.0).round() / 100.0;
        if !note.is_empty() {
            note.push_str(&format!(" 最佳={} 最差={} 一致性={}", best, worst, cons_pct));
        }
    }

    (adj_confidence, note)
}

#[test]
fn v2_rhai_parsing_valid_input() {
    let input = r#"{"survivalRate": 0.85, "bestScenario": "牛市", "worstScenario": "熊市", "consistencyScore": 0.31}"#;
    let (conf, note) = parse_simulation_result(input, 70.0);

    assert!(note.contains("生存率=85%"), "should contain survival rate");
    assert!(note.contains("最佳=牛市"), "should contain best scenario");
    assert!(note.contains("最差=熊市"), "should contain worst scenario");
    assert_eq!(conf, 70.0, "高生存率不应降低置信度");
    eprintln!("[V2] note='{}' conf={}", note, conf);
}

#[test]
fn v2_rhai_parsing_empty_input() {
    let (conf, note) = parse_simulation_result("", 70.0);
    assert_eq!(conf, 70.0, "空输入不应改变置信度");
    assert!(note.is_empty(), "空输入不应产生 note");
}

// ── 验证③: 置信度修正 ──

#[test]
fn v3_confidence_adjustment_high() {
    let (conf, note) = parse_simulation_result(r#"{"survivalRate": 0.85}"#, 70.0);
    assert_eq!(conf, 70.0, "高生存率 → 不变");
    assert!(note.contains("85%"));
}

#[test]
fn v3_confidence_adjustment_medium() {
    let (conf, note) = parse_simulation_result(r#"{"survivalRate": 0.55}"#, 70.0);
    assert!((conf - 59.5).abs() < 0.01, "中生存率 → 70×0.85=59.5, got {}", conf);
    assert!(note.contains("中等生存率"));
}

#[test]
fn v3_confidence_adjustment_low() {
    let (conf, note) = parse_simulation_result(r#"{"survivalRate": 0.35}"#, 70.0);
    assert!((conf - 45.5).abs() < 0.01, "低生存率 → 70×0.65=45.5, got {}", conf);
    assert!(note.contains("低生存率"));
}

#[test]
fn v3_confidence_edge_cases() {
    let (conf, _) = parse_simulation_result(r#"{"survivalRate": 0.0}"#, 50.0);
    assert!((conf - 32.5).abs() < 0.01, "0% → 50×0.65=32.5");

    let (conf, _) = parse_simulation_result(r#"{"survivalRate": 1.0}"#, 50.0);
    assert_eq!(conf, 50.0, "100% → 不变");

    let (conf, _) = parse_simulation_result(r#"{"survivalRate": 0.6}"#, 50.0);
    assert_eq!(conf, 50.0, "60% → 不变（门槛是 <0.60）");

    let (conf, _) = parse_simulation_result(r#"{"survivalRate": 0.4}"#, 50.0);
    assert!((conf - 42.5).abs() < 0.01, "40% → 50×0.85=42.5");
}

// ── 验证④: 无 simulation_result 时无影响 ──

#[test]
fn v4_no_simulation_variable_no_effect() {
    let (conf, note) = parse_simulation_result(r#"{"something_else": true}"#, 70.0);
    assert_eq!(conf, 70.0, "无 survivalRate → 不变");
    assert!(note.is_empty(), "无 survivalRate → 无 note");
}

#[test]
fn v4_null_simulation_variable() {
    let (conf, note) = parse_simulation_result("null", 70.0);
    assert_eq!(conf, 70.0);
    assert!(note.is_empty());
}

#[test]
fn v4_invalid_json_simulation_variable() {
    let (conf, note) = parse_simulation_result("not valid json", 70.0);
    assert_eq!(conf, 70.0);
    assert!(note.is_empty());
}

// ── 验证⑤: 性能基准 ──

#[test]
fn v5_performance_baseline() {
    use std::time::Instant;

    let runs = 5;
    let mut wall_times = Vec::with_capacity(runs);

    for i in 0..runs {
        let start = Instant::now();
        let result = run_validation_sim("买入", 1050, 950, 500, 500_000_000);
        let elapsed = start.elapsed().as_millis();
        wall_times.push(elapsed);
        eprintln!(
            "[V5] run {}/{}: {}ms wall ({} events, {} trades)",
            i + 1,
            runs,
            elapsed,
            result.total_events,
            result.trades.len(),
        );
    }

    let avg = wall_times.iter().copied().sum::<u128>() as f64 / runs as f64;
    let max = wall_times.iter().copied().max().unwrap_or(0);
    eprintln!("[V5] avg={:.1}ms max={}ms (target: <30s/avg)", avg, max);
    assert!(avg < 30_000.0, "平均墙钟应 < 30s, 实际 {:.1}ms", avg);
}

// ── 辅助函数 ──

fn run_validation_sim(
    action: &str,
    target: i64,
    stop: i64,
    pos: u64,
    duration_ns: u64,
) -> SimResult {
    let price = 1000;
    let mut kernel = SimKernel::new(SimConfig {
        max_time_ns: duration_ns,
        default_latency_ns: 1_000,
        reference_price: price,
        ..Default::default()
    });

    kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
    kernel.register(Box::new(MarketMakerAgent::new("mm", 35, 500, 5000, 0.1, 500_000, price)));
    kernel.register(Box::new(StrategyAgent::new(
        "strategy", action, target, stop, price, pos, 1_000_000,
    )));
    kernel.register(Box::new(NoiseAgent::new("noise", 500_000, 0.27, 50, 32, price, 42)));
    kernel.register(Box::new(MomentumAgent::new(
        "momentum",
        5,
        0.0035,
        100,
        2000,
        800_000,
        price as f64,
    )));

    kernel.run().unwrap()
}

// ── 端到端管道验证 ──

/// 验证⑥：全链路 StrategyAgent → SimKernel → trades → total_orders 不为 0
#[test]
fn v6_full_pipeline_total_orders_reported() {
    let result = run_validation_sim("买入", 1050, 950, 500, 500_000_000);
    assert!(result.total_events > 0, "应该有事件");
    // total_orders 现在通过 ExchangeAgent.stats() 返回真实值
    assert!(
        result.stats.total_orders > 0,
        "total_orders 应 > 0 (实际={})",
        result.stats.total_orders
    );
    assert!(
        result.stats.total_trades > 0,
        "total_trades 应 > 0 (实际={})",
        result.stats.total_trades
    );
    assert!(result.stats.agent_count >= 4, "至少 4 个 Agent");
    eprintln!(
        "[V6] events={} orders={} trades={} agents={}",
        result.total_events,
        result.stats.total_orders,
        result.stats.total_trades,
        result.stats.agent_count,
    );
}

/// 验证⑦：BEST_PARAMS 可读取且有效
#[test]
fn v7_best_params_are_valid() {
    // BEST_PARAMS 为 pub const，断言属于编译期常量比较；
    // 用 const { assert!(..) } 消除 clippy::assertions_on_constants 警告
    const { assert!(BEST_PARAMS.mm_spread_bps > 0, "mm_spread_bps 应 > 0") };
    const { assert!(BEST_PARAMS.mm_quote_size > 0, "mm_quote_size 应 > 0") };
    const { assert!(BEST_PARAMS.noise_act_prob > 0.0, "noise_act_prob 应 > 0") };
    const { assert!(BEST_PARAMS.noise_price_noise_bps > 0, "noise_price_noise_bps 应 > 0") };
    const { assert!(BEST_PARAMS.momentum_threshold > 0.0, "momentum_threshold 应 > 0") };
    eprintln!("[V7] BEST_PARAMS validated: {:?}", BEST_PARAMS);
}

/// 验证⑧：量化策略 Agent 在 DES 中产生可测量事件
#[test]
fn v8_quant_strategy_produces_events() {
    use axagent_market_sim::QuantStrategyAgent;

    let price = 1000;
    let mut kernel = SimKernel::new(SimConfig {
        max_time_ns: 500_000_000,
        default_latency_ns: 1_000,
        reference_price: price,
        ..Default::default()
    });

    kernel.register(Box::new(ExchangeAgent::with_tick_size("exchange", 1)));
    kernel.register(Box::new(MarketMakerAgent::new("mm", 35, 500, 5000, 0.1, 500_000, price)));
    kernel.register(Box::new(QuantStrategyAgent::new(
        "quant",
        // 原 test 依赖 axagent_quant::MaCrossStrategy，按铁律 5（consumer 测试不得依赖其他 consumer），
        // 改为本地 MockMaCrossStrategy。它实现了 Strategy trait，行为足够验证事件循环。
        Box::new(MockMaCrossStrategy::new(5, 20)),
        "000001",
        price,
        1_000_000.0,
        1_000_000,
    )));
    kernel.register(Box::new(NoiseAgent::new("noise", 500_000, 0.27, 50, 32, price, 42)));

    let result = kernel.run().unwrap();
    assert!(result.total_events > 50);
    assert!(result.stats.agent_count >= 3);
    eprintln!("[V8] quant_strategy: events={} trades={}", result.total_events, result.trades.len());
}

/// 本地 MockMaCrossStrategy — 不依赖 axagent-quant（同为 consumer，违反铁律 5）
struct MockMaCrossStrategy {
    short_period: usize,
    long_period: usize,
}

impl MockMaCrossStrategy {
    fn new(short_period: usize, long_period: usize) -> Self {
        Self { short_period, long_period }
    }
}

#[async_trait]
impl axagent_harness::strategy_contract::Strategy for MockMaCrossStrategy {
    fn name(&self) -> &str {
        "mock_ma_cross"
    }
    fn params(&self) -> serde_json::Value {
        serde_json::json!({
            "short_period": self.short_period,
            "long_period": self.long_period,
        })
    }
    fn set_param(
        &mut self,
        key: &str,
        value: serde_json::Value,
    ) -> axagent_harness::core_error::Result<()> {
        use axagent_harness::core_error::AxAgentError;
        match key {
            "short_period" => {
                self.short_period =
                    value.as_u64().ok_or_else(|| AxAgentError::Validation(key.to_string()))?
                        as usize;
            },
            "long_period" => {
                self.long_period =
                    value.as_u64().ok_or_else(|| AxAgentError::Validation(key.to_string()))?
                        as usize;
            },
            _ => return Err(AxAgentError::Validation(format!("未知参数: {}", key))),
        }
        Ok(())
    }
    async fn on_bar(
        &mut self,
        bar: &axagent_harness::strategy_contract::Bar,
        ctx: &mut axagent_harness::strategy_contract::StrategyCtx,
    ) -> axagent_harness::core_error::Result<Vec<axagent_harness::strategy_contract::Signal>> {
        use axagent_harness::strategy_contract::{CloseReason, Signal, SignalAction};

        let history = match ctx.bar_history.get(&bar.code) {
            Some(h) if h.len() > self.long_period => h,
            _ => return Ok(vec![]),
        };
        if history.len() < self.long_period + 1 {
            return Ok(vec![]);
        }
        let cs: Vec<f64> = history.iter().map(|b| b.close).collect();
        let cur_short = sma(&cs, self.short_period);
        let cur_long = sma(&cs, self.long_period);
        let prev_short = sma(&cs[..cs.len() - 1], self.short_period);
        let prev_long = sma(&cs[..cs.len() - 1], self.long_period);

        if let (Some(cs_), Some(cl_), Some(ps_), Some(pl_)) =
            (cur_short, cur_long, prev_short, prev_long)
        {
            if ps_ <= pl_ && cs_ > cl_ {
                return Ok(vec![Signal {
                    code: bar.code.clone(),
                    action: SignalAction::Buy,
                    strength: 0.7,
                    reason: format!("Mock MA{} 上穿 MA{}", self.short_period, self.long_period),
                    target_weight: None,
                    close_reason: None,
                }]);
            }
            if ps_ >= pl_ && cs_ < cl_ {
                return Ok(vec![Signal {
                    code: bar.code.clone(),
                    action: SignalAction::Sell,
                    strength: 0.7,
                    reason: format!("Mock MA{} 下穿 MA{}", self.short_period, self.long_period),
                    target_weight: None,
                    close_reason: Some(CloseReason::SignalReverse),
                }]);
            }
        }
        Ok(vec![])
    }
}
