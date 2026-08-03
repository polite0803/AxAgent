// SPDX-License-Identifier: AGPL-3.0-only

//! 策略权重参数进化优化器
//!
//! 使用 NumericEvolutionEngine 在 WeightDecayConfig 参数空间中搜索最优组合。
//! 适应度 = 各策略的 (weight * win_rate * confidence) 加权和。
//!
//! ## 用法
//!
//! ```ignore
//! use stock_analysis::evolution_optimizer::run_evolution;
//!
//! let result = run_evolution(db, None).await?;
//! // result.best_config 即进化搜索到的最优 WeightDecayConfig
//! // result.evolution_stats 包含进化过程数据（供前端渲染）
//! ```

use std::collections::HashMap;

use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tracing::info;

use axagent_trajectory::numeric_evolution::{
    NumericEvolutionEngine, NumericEvolutionStats, NumericGenome, ParamDef,
};
use axagent_trajectory::skill_evolution::EvolutionConfig;

use crate::evolution_drift::{load_current_weights, load_performance_window};
use crate::weight_decay::{compute_adjusted_weights, WeightDecayConfig};

/// 进化优化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionResult {
    /// 进化搜索到的最优 WeightDecayConfig
    pub best_config: WeightDecayConfig,
    /// 进化过程统计
    pub evolution_stats: NumericEvolutionStats,
    /// 默认配置作为对比基准
    pub default_config: WeightDecayConfig,
}

/// 参数定义：用于 NumericEvolutionEngine 的参数搜索空间
pub fn param_defs() -> Vec<ParamDef> {
    vec![
        ParamDef {
            name: "ewma_alpha".to_string(),
            min: 0.05,
            max: 0.80,
            step: 0.0, // 连续
        },
        ParamDef {
            name: "lookback_days".to_string(),
            min: 5.0,
            max: 120.0,
            step: 1.0, // 整数
        },
        ParamDef {
            name: "sample_saturation".to_string(),
            min: 5.0,
            max: 100.0,
            step: 1.0, // 整数
        },
    ]
}

/// 从 NumericGenome 解码为 WeightDecayConfig
pub fn decode_config(genome: &NumericGenome) -> WeightDecayConfig {
    WeightDecayConfig {
        ewma_alpha: genome.params.get("ewma_alpha").copied().unwrap_or(0.3).clamp(0.01, 0.99),
        lookback_days: genome.params.get("lookback_days").copied().unwrap_or(30.0).round() as u32,
        sample_saturation: genome.params.get("sample_saturation").copied().unwrap_or(20.0).round()
            as u32,
    }
}

/// 构建适合度函数闭包
///
/// 适应度 = Σ (weight * win_rate * confidence) 对所有 (strategy, period)
/// 分数越高表示该参数组合对历史数据拟合越好。
pub fn make_fitness_fn<'a>(
    history: &'a [crate::weight_decay::StrategyPerformanceRow],
    current_weights: &'a HashMap<(String, String), f64>,
) -> impl Fn(&NumericGenome) -> f64 + 'a {
    // 预计算默认配置的分数作为基准
    let default = WeightDecayConfig::default();
    let default_map = compute_adjusted_weights(history, &default, current_weights);
    let default_score: f64 =
        default_map.values().map(|aw| aw.new_weight * aw.win_rate * aw.confidence).sum();

    move |genome: &NumericGenome| {
        let cfg = decode_config(genome);

        // 跳过过于极端的参数
        if cfg.lookback_days < 3 || cfg.ewma_alpha < 0.01 || cfg.sample_saturation < 2 {
            return 0.0;
        }

        let result = compute_adjusted_weights(history, &cfg, current_weights);
        if result.is_empty() {
            return 0.0;
        }

        // 核心适应度：加权和
        let total_score: f64 =
            result.values().map(|aw| aw.new_weight * aw.win_rate * aw.confidence).sum();

        // 惩罚项：样本太少
        let min_sample = result.values().map(|aw| aw.sample_size).min().unwrap_or(0);
        let sample_penalty = if min_sample < 3 { 0.5 } else { 1.0 };

        // 惩罚项：参数过于激进（alpha > 0.7 且样本不足）
        let alpha_penalty = if cfg.ewma_alpha > 0.7 && min_sample < 10 {
            0.7
        } else {
            1.0
        };

        let score = total_score * sample_penalty * alpha_penalty;

        // 对抗性：略好于默认即可，不追求极端优化
        // 如果 score 低于 default_score 的 80%，直接大幅扣分
        if default_score > 0.0 && score < default_score * 0.8 {
            return score * 0.3;
        }

        score
    }
}

/// 运行进化搜索，寻找最优 WeightDecayConfig
pub async fn run_evolution(
    db: &DatabaseConnection,
    as_of_date: Option<&str>,
) -> Result<EvolutionResult, String> {
    let current_weights = load_current_weights(db).await?;
    let history = load_performance_window(db, 180, as_of_date).await?;

    if history.is_empty() {
        return Err("无表现数据可供进化".to_string());
    }

    info!(
        "[EvolutionOptimizer] 开始进化搜索, 历史记录 {} 条, 当前策略 {} 个",
        history.len(),
        current_weights.len()
    );

    let fitness_fn = make_fitness_fn(&history, &current_weights);

    let mut engine = NumericEvolutionEngine::new(
        EvolutionConfig {
            population_size: 30,
            elite_count: 5,
            mutation_rate: 0.2,
            crossover_rate: 0.7,
            max_generations: 50,
            convergence_threshold: 0.95,
            min_fitness_improvement: 0.005,
            use_llm_mutation: false,
            use_execution_validation: false,
            validation_rounds: 0,
            ..Default::default()
        },
        param_defs(),
    );

    let (best_genome, stats) = engine.run(fitness_fn);

    let best_config = match &best_genome {
        Some(g) => decode_config(g),
        None => WeightDecayConfig::default(),
    };

    info!(
        "[EvolutionOptimizer] 进化完成: best=({:.2}, {}, {}), best_fitness={:.4}, generations={}",
        best_config.ewma_alpha,
        best_config.lookback_days,
        best_config.sample_saturation,
        stats.best_fitness,
        stats.generation,
    );

    Ok(EvolutionResult {
        best_config,
        evolution_stats: stats,
        default_config: WeightDecayConfig::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weight_decay::StrategyPerformanceRow;

    fn sample_history(count: usize, win_rate: f64) -> Vec<StrategyPerformanceRow> {
        let mut rows = Vec::with_capacity(count);
        for i in 0..count {
            let ok = if (i as f64) < count as f64 * win_rate {
                1
            } else {
                0
            };
            rows.push(StrategyPerformanceRow {
                strategy_id: "trend".to_string(),
                period: "short".to_string(),
                was_correct: ok,
                exit_at: chrono::Utc::now().timestamp_millis() - (i as i64) * 86_400_000,
            });
        }
        // 添加 value 策略数据
        for i in 0..count / 2 {
            let ok = if (i as f64) < (count as f64 / 2.0) * (win_rate - 0.1) {
                1
            } else {
                0
            };
            rows.push(StrategyPerformanceRow {
                strategy_id: "value".to_string(),
                period: "mid".to_string(),
                was_correct: ok,
                exit_at: chrono::Utc::now().timestamp_millis() - (i as i64) * 86_400_000,
            });
        }
        rows
    }

    #[test]
    fn decode_config_roundtrip() {
        let genome = NumericGenome {
            params: [
                ("ewma_alpha".to_string(), 0.42),
                ("lookback_days".to_string(), 45.0),
                ("sample_saturation".to_string(), 18.0),
            ]
            .into(),
            fitness: 0.0,
        };
        let cfg = decode_config(&genome);
        assert!((cfg.ewma_alpha - 0.42).abs() < 0.01);
        assert_eq!(cfg.lookback_days, 45);
        assert_eq!(cfg.sample_saturation, 18);
    }

    #[test]
    fn decode_config_clamps_values() {
        let genome = NumericGenome {
            params: [
                ("ewma_alpha".to_string(), 2.0),        // 超出范围
                ("lookback_days".to_string(), -5.0),    // 负值
                ("sample_saturation".to_string(), 0.0), // 最小值
            ]
            .into(),
            fitness: 0.0,
        };
        let cfg = decode_config(&genome);
        assert!((cfg.ewma_alpha - 0.99).abs() < 0.01, "被 clamp");
        assert_eq!(cfg.lookback_days, 0); // -5 -> 0u32 自动 wrap 到 0
        assert_eq!(cfg.sample_saturation, 0);
    }

    #[test]
    fn fitness_function_returns_reasonable_score() {
        let history = sample_history(50, 0.65);
        let current = HashMap::new();
        let fn_fitness = make_fitness_fn(&history, &current);

        // 测试默认参数
        let default_genome = NumericGenome {
            params: [
                ("ewma_alpha".to_string(), 0.3),
                ("lookback_days".to_string(), 30.0),
                ("sample_saturation".to_string(), 20.0),
            ]
            .into(),
            fitness: 0.0,
        };
        let score = fn_fitness(&default_genome);
        assert!(score > 0.0, "默认参数应得到正分, 实际={}", score);

        // 测试极端参数
        let bad_genome = NumericGenome {
            params: [
                ("ewma_alpha".to_string(), 0.01),
                ("lookback_days".to_string(), 2.0),
                ("sample_saturation".to_string(), 1.0),
            ]
            .into(),
            fitness: 0.0,
        };
        let bad_score = fn_fitness(&bad_genome);
        assert!(bad_score < score, "极端参数应比默认差: bad={} default={}", bad_score, score);
    }

    #[test]
    fn evolution_runs_with_mock_data() {
        let history = sample_history(100, 0.6);
        let current = HashMap::new();
        let fitness_fn = make_fitness_fn(&history, &current);

        let mut engine = NumericEvolutionEngine::new(
            EvolutionConfig {
                population_size: 20,
                elite_count: 3,
                mutation_rate: 0.2,
                crossover_rate: 0.7,
                max_generations: 10,
                min_fitness_improvement: 0.001,
                ..Default::default()
            },
            param_defs(),
        );

        let (genome, stats) = engine.run(fitness_fn);
        assert!(stats.generation > 0, "进化应运行至少 1 代");
        assert!(stats.best_fitness > 0.0, "应产生正适应度: {}", stats.best_fitness);
        if let Some(g) = genome {
            let cfg = decode_config(&g);
            assert!(cfg.ewma_alpha > 0.0 && cfg.ewma_alpha <= 1.0, "alpha 应在范围内");
            assert!(cfg.lookback_days >= 3, "lookback 不应小于 3");
        }
    }
}
