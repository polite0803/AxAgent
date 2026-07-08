//! 复盘 → 进化：策略权重调整算法
//!
//! 输入：`Vec<StrategyPerformanceRow>`（来自 strategy_performance 表）
//! 输出：`HashMap<(strategy_id, period), AdjustedWeight>`
//!
//! 算法组合：
//! 1. **贝叶斯平滑**：用 Beta(1,1) 先验，避免小样本极端估计
//!    smoothed_win_rate = (wins + 1) / (total + 2)
//! 2. **EWMA 平滑**：权重 = α · 新权重 + (1-α) · 旧权重，避免单日抖动
//! 3. **样本量降权**：log-饱和函数，< 5 几乎不参与调整
//!
//! 关键设计：
//! - 纯函数，无 I/O（方便单元测试）
//! - `lookback_days` 默认 30，与前端 EvolutionDriftPanel 的 30/90/180 切换对齐
//! - `current_weights` 可选传入，若传则计算 EWMA；不传则只算 raw 新权重

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 一行 strategy 表现记录（精简版，避免与 entity 直接耦合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPerformanceRow {
    pub strategy_id: String,
    pub period: String,
    pub was_correct: i32, // 0/1
    pub exit_at: i64,     // ms
}

/// 调整后的权重结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdjustedWeight {
    pub strategy_id: String,
    pub period: String,
    pub new_weight: f64,
    pub win_rate: f64,    // 平滑后胜率（0-1）
    pub sample_size: u32, // 该 (strategy, period) 的样本数
    pub confidence: f64,  // 样本量饱和度（0-1，1 表示 >= 20 个样本）
}

/// 权重调整配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightDecayConfig {
    /// 回溯窗口天数
    pub lookback_days: u32,
    /// EWMA 平滑系数：新权重占比（0-1）。默认 0.3 表示单日调整最多贡献 30%
    pub ewma_alpha: f64,
    /// 样本量饱和参考值。>= 此值时 confidence 接近 1
    pub sample_saturation: u32,
}

impl Default for WeightDecayConfig {
    fn default() -> Self {
        Self {
            lookback_days: 30,
            ewma_alpha: 0.3,
            sample_saturation: 20,
        }
    }
}

/// 计算所有 (strategy_id, period) 组合的调整后权重。
///
/// 若 `current_weights` 非空，对每个 key 应用 EWMA 平滑（new_effective = alpha * raw + (1-alpha) * current）。
/// 若无 current，按 raw 直接输出。
pub fn compute_adjusted_weights(
    history: &[StrategyPerformanceRow],
    cfg: &WeightDecayConfig,
    current_weights: &HashMap<(String, String), f64>,
) -> HashMap<(String, String), AdjustedWeight> {
    // 1. 时间窗口过滤
    let now_ms = chrono::Utc::now().timestamp_millis();
    let cutoff_ms = now_ms - (cfg.lookback_days as i64) * 86_400_000;

    // 2. 按 (strategy_id, period) 聚合 wins / total
    let mut agg: HashMap<(String, String), (u32, u32)> = HashMap::new();
    for row in history {
        if row.exit_at < cutoff_ms {
            continue;
        }
        let key = (row.strategy_id.clone(), row.period.clone());
        let entry = agg.entry(key).or_insert((0, 0));
        if row.was_correct != 0 {
            entry.0 += 1;
        }
        entry.1 += 1;
    }

    // 3. 计算贝叶斯平滑胜率 + 样本量饱和度
    let mut out = HashMap::new();
    for ((sid, period), (wins, total)) in agg {
        // Beta(1,1) 平滑
        let smoothed = (wins as f64 + 1.0) / (total as f64 + 2.0);
        // log 饱和
        let confidence =
            ((1.0 + total as f64).ln() / (1.0 + cfg.sample_saturation as f64).ln()).clamp(0.0, 1.0);
        // raw 新权重：smoothed × confidence（样本不足时降权）
        let raw_new = (smoothed * confidence).clamp(0.05, 1.0);

        // EWMA 平滑
        let new_weight = match current_weights.get(&(sid.clone(), period.clone())) {
            Some(&cur) => {
                (cfg.ewma_alpha * raw_new + (1.0 - cfg.ewma_alpha) * cur).clamp(0.05, 1.0)
            },
            None => raw_new,
        };

        out.insert(
            (sid.clone(), period.clone()),
            AdjustedWeight {
                strategy_id: sid,
                period,
                new_weight,
                win_rate: smoothed,
                sample_size: total,
                confidence,
            },
        );
    }

    out
}

/// 生成可读的 rationale 文本（用于 strategy_weight_history.rationale）
pub fn format_rationale(aw: &AdjustedWeight, sample_size: u32) -> String {
    let win_pct = (aw.win_rate * 100.0).round() as u32;
    match sample_size {
        0..=4 => format!("样本过少({} 个),胜率 {}%,权重暂时不调整", sample_size, win_pct),
        5..=19 => format!(
            "样本 {} 个,胜率 {}%,置信度 {:.0}%,按规则调整",
            sample_size,
            win_pct,
            aw.confidence * 100.0
        ),
        _ => format!(
            "样本充足({} 个),胜率 {}%,置信度 {:.0}%,稳定调整",
            sample_size,
            win_pct,
            aw.confidence * 100.0
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(s: &str, p: &str, ok: i32, days_ago: i64) -> StrategyPerformanceRow {
        let exit_at = chrono::Utc::now().timestamp_millis() - days_ago * 86_400_000;
        StrategyPerformanceRow {
            strategy_id: s.to_string(),
            period: p.to_string(),
            was_correct: ok,
            exit_at,
        }
    }

    #[test]
    fn empty_history_returns_empty() {
        let cfg = WeightDecayConfig::default();
        let w = compute_adjusted_weights(&[], &cfg, &HashMap::new());
        assert!(w.is_empty());
    }

    #[test]
    fn all_rows_outside_lookback_are_filtered() {
        let cfg = WeightDecayConfig {
            lookback_days: 30,
            ..Default::default()
        };
        let history = vec![row("trend", "short", 1, 60)];
        let w = compute_adjusted_weights(&history, &cfg, &HashMap::new());
        assert!(w.is_empty(), "应被 lookback 窗口过滤");
    }

    #[test]
    fn high_win_rate_boosts_weight() {
        let cfg = WeightDecayConfig {
            lookback_days: 30,
            ewma_alpha: 1.0, // 关掉 EWMA 便于断言
            ..Default::default()
        };
        let history: Vec<_> = (0..30).map(|_| row("trend", "short", 1, 5)).collect();
        let w = compute_adjusted_weights(&history, &cfg, &HashMap::new());
        let key = ("trend".to_string(), "short".to_string());
        let aw = w.get(&key).expect("应有 trend/short key");
        // 30 行 100% 胜率 → 平滑 = 31/32 = 0.969, confidence ≈ 1, weight ≈ 0.969
        assert!(aw.new_weight > 0.9, "高胜率应得高权重,实际={}", aw.new_weight);
        assert_eq!(aw.sample_size, 30);
    }

    #[test]
    fn low_win_rate_lowers_weight() {
        let cfg = WeightDecayConfig {
            lookback_days: 30,
            ewma_alpha: 1.0,
            ..Default::default()
        };
        let history: Vec<_> = (0..30).map(|_| row("value", "mid", 0, 5)).collect();
        let w = compute_adjusted_weights(&history, &cfg, &HashMap::new());
        let key = ("value".to_string(), "mid".to_string());
        let aw = w.get(&key).expect("应有 value/mid key");
        // 30 行 0% 胜率 → 平滑 = 1/32 = 0.031, weight clamp 到 0.05
        assert!(aw.new_weight < 0.1, "低胜率应得低权重,实际={}", aw.new_weight);
    }

    #[test]
    fn small_sample_size_dampens_weight() {
        // 同样 100% 胜率,5 行 vs 50 行,小样本应得明显更低的 confidence
        let cfg = WeightDecayConfig {
            lookback_days: 30,
            ewma_alpha: 1.0,
            sample_saturation: 20,
            ..Default::default()
        };
        let small: Vec<_> = (0..5).map(|_| row("trend", "short", 1, 5)).collect();
        let big: Vec<_> = (0..50).map(|_| row("trend", "short", 1, 5)).collect();
        let w_small = compute_adjusted_weights(&small, &cfg, &HashMap::new());
        let w_big = compute_adjusted_weights(&big, &cfg, &HashMap::new());
        let key = ("trend".to_string(), "short".to_string());
        let aw_small = w_small.get(&key).unwrap();
        let aw_big = w_big.get(&key).unwrap();
        assert!(
            aw_small.confidence < aw_big.confidence,
            "小样本 confidence 应更低:small={} big={}",
            aw_small.confidence,
            aw_big.confidence
        );
        assert!(
            aw_small.new_weight < aw_big.new_weight,
            "小样本权重应更低:small={} big={}",
            aw_small.new_weight,
            aw_big.new_weight
        );
    }

    #[test]
    fn ewma_smooths_toward_current_weight() {
        let cfg = WeightDecayConfig {
            lookback_days: 30,
            ewma_alpha: 0.3,
            ..Default::default()
        };
        let history: Vec<_> = (0..30).map(|_| row("trend", "short", 1, 5)).collect();

        // raw new ≈ 0.97, 但传 current=0.5 → EWMA 应当拉低
        let mut cur = HashMap::new();
        cur.insert(("trend".to_string(), "short".to_string()), 0.5);
        let w = compute_adjusted_weights(&history, &cfg, &cur);
        let aw = w.get(&("trend".to_string(), "short".to_string())).unwrap();
        let expected = 0.3 * aw.win_rate * aw.confidence + 0.7 * 0.5;
        // 因为 confidence=1, win_rate=0.97, raw=0.97, new = 0.3*0.97 + 0.7*0.5 = 0.641
        assert!(
            (aw.new_weight - expected).abs() < 0.01,
            "EWMA 应按 alpha=0.3 平滑,期望≈{},实际={}",
            expected,
            aw.new_weight
        );
    }

    #[test]
    fn multiple_strategies_independent() {
        let cfg = WeightDecayConfig {
            lookback_days: 30,
            ewma_alpha: 1.0,
            ..Default::default()
        };
        let history = vec![
            row("trend", "short", 1, 5),
            row("trend", "short", 1, 5),
            row("value", "mid", 0, 5),
            row("value", "mid", 0, 5),
        ];
        let w = compute_adjusted_weights(&history, &cfg, &HashMap::new());
        assert_eq!(w.len(), 2, "应有 trend/short 和 value/mid 两个 key");
        let t = w.get(&("trend".to_string(), "short".to_string())).unwrap();
        let v = w.get(&("value".to_string(), "mid".to_string())).unwrap();
        assert!(t.new_weight > v.new_weight, "trend 应远高于 value");
    }

    #[test]
    fn weight_clamped_to_min_floor() {
        let cfg = WeightDecayConfig {
            lookback_days: 30,
            ewma_alpha: 1.0,
            ..Default::default()
        };
        let history: Vec<_> = (0..50).map(|_| row("bad", "short", 0, 5)).collect();
        let w = compute_adjusted_weights(&history, &cfg, &HashMap::new());
        let aw = w.get(&("bad".to_string(), "short".to_string())).unwrap();
        // 极低胜率,但不能为 0(给策略保留最低存在感)
        assert!(aw.new_weight >= 0.05, "权重下限 0.05");
    }

    #[test]
    fn rationale_message_reflects_sample_size() {
        let cfg = WeightDecayConfig {
            lookback_days: 30,
            ewma_alpha: 1.0,
            ..Default::default()
        };
        let history: Vec<_> = (0..3).map(|_| row("trend", "short", 1, 5)).collect();
        let w = compute_adjusted_weights(&history, &cfg, &HashMap::new());
        let aw = w.get(&("trend".to_string(), "short".to_string())).unwrap();
        let r = format_rationale(aw, aw.sample_size);
        assert!(r.contains("样本"), "rationale 应说明样本量:{}", r);
    }
}
