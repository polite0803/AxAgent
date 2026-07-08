//! 智能荐股 — 置信度、仓位、去重、缓存

use crate::recommender::types::{Period, RecoPick, Style};
use std::collections::{BTreeMap, HashMap};
use std::sync::RwLock;

/// 自适应评分权重（初始值为默认值，可通过回测/反思反馈调优）
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub consistency: f64,
    pub signal_strength: f64,
    pub liquidity: f64,
    pub price_momentum: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            consistency: 0.45,
            signal_strength: 0.35,
            liquidity: 0.15,
            price_momentum: 0.05,
        }
    }
}

static ADAPTIVE_WEIGHTS: RwLock<ScoringWeights> = RwLock::new(ScoringWeights {
    consistency: 0.45,
    signal_strength: 0.35,
    liquidity: 0.15,
    price_momentum: 0.05,
});

/// 读取当前自适应权重
pub fn get_scoring_weights() -> ScoringWeights {
    ADAPTIVE_WEIGHTS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// 更新自适应权重（EWMA: new = old × 0.7 + suggested × 0.3）
pub fn nudge_scoring_weights(suggested: &ScoringWeights) {
    if let Ok(mut w) = ADAPTIVE_WEIGHTS.write() {
        w.consistency = w.consistency * 0.7 + suggested.consistency * 0.3;
        w.signal_strength = w.signal_strength * 0.7 + suggested.signal_strength * 0.3;
        w.liquidity = w.liquidity * 0.7 + suggested.liquidity * 0.3;
        w.price_momentum = w.price_momentum * 0.7 + suggested.price_momentum * 0.3;
        // 归一化确保总和=1.0
        let total = w.consistency + w.signal_strength + w.liquidity + w.price_momentum;
        if total > 0.0 {
            w.consistency /= total;
            w.signal_strength /= total;
            w.liquidity /= total;
            w.price_momentum /= total;
        }
    }
}

/// 置信度计算
///
/// - `score_consistency`: 子策略内多因子方向一致率 (0-1)
/// - `signal_strength`: 关键因子偏离度分位 (0-1)
/// - `liquidity_score`: 成交额 / 换手率分位 (0-1)
/// - `price_momentum`: 距均线偏离 / 近期涨跌幅分位 (-0.5 ~ +0.5)，正=趋势有利
/// - `turnover_anomaly`: 今日成交额 / 20日均 (> 1.0 为正常，> 3.0 视为异常)
///
/// 权重分配依据：
///   - consistency(0.45) 为第一权重：多因子方向一致是置信度的核心
///   - signal_strength(0.35) 为第二权重：关键指标偏离度代表信号稀缺性
///   - liquidity(0.15) 为辅助权重：流动性能保证策略可执行
///   - price_momentum(0.05) 为微调权重：避免严重逆势入场
pub fn calc_confidence(
    score_consistency: f64,
    signal_strength: f64,
    liquidity_score: f64,
    price_momentum: f64,
    turnover_anomaly: f64,
) -> u8 {
    // sanitize inputs
    let clean = |v: f64| {
        if v.is_nan() || v.is_infinite() {
            0.0
        } else {
            v
        }
    };
    let score_consistency = clean(score_consistency);
    let signal_strength = clean(signal_strength);
    let liquidity_score = clean(liquidity_score);
    let price_momentum = clean(price_momentum);
    let turnover_anomaly = clean(turnover_anomaly);

    let w = get_scoring_weights();
    let mut c = w.consistency * score_consistency
        + w.signal_strength * signal_strength
        + w.liquidity * liquidity_score
        + w.price_momentum * price_momentum;

    // 成交额异常阶梯惩罚（替代原断崖式 40%）：
    //   1.0-2.0x → 无惩罚（正常交易）
    //   2.0-3.0x → 10% 置信度衰减（温和放量）
    //   3.0-5.0x → 25% 置信度衰减（异常放量，可能是出货）
    //   >5.0x    → 40% 置信度衰减（极端放量，高概率操纵）
    if turnover_anomaly > 5.0 {
        c *= 0.60; // −40%
    } else if turnover_anomaly > 3.0 {
        c *= 0.75; // −25%
    } else if turnover_anomaly > 2.0 {
        c *= 0.90; // −10%
    }

    if c.is_nan() || c.is_infinite() {
        return 0;
    }
    (c * 100.0).clamp(0.0, 100.0).round() as u8
}

/// 仓位动态化：Kelly 公式近似
///
/// 标准 Kelly: f* = (p·b − q) / b
///   其中 p=胜率, q=1−p, b=盈亏比(target/stop)
///
/// 本实现：base × confidence/100 × period_factor
///   - confidence/100 近似 p（置信度 ∝ 预期胜率）
///   - base 由各策略按 target/stop 比预先设定（近似 Kelly f*）
///   - period_factor 按持有期缩放（超短线 0.4 → 长线 1.0）
///
/// 简化原因：p 和 b 无法在子策略内精确估计（依赖 K 线外数据），
/// 故用信度代理概率、用参数化的 base 代理赔率调整。
pub fn calc_position(base: f64, confidence: u8, period: Period) -> f64 {
    if base.is_nan() {
        return 0.0;
    }
    let c = confidence as f64 / 100.0;
    (base * c * period.factor() * 100.0).round() / 100.0
}

/// 同票去重：保留 confidence 最高，标注次选风格
pub fn dedup_and_merge(picks: &mut Vec<RecoPick>) {
    let mut by_code: HashMap<String, RecoPick> = HashMap::new();
    for p in picks.drain(..) {
        if let Some(existing) = by_code.get_mut(&p.stock_code) {
            // 同票被多风格命中
            if p.confidence > existing.confidence {
                // p 取代 existing，existing 的风格 + 两者各自的 secondary 合入
                let mut merged = p;
                let mut secondaries: Vec<Style> = Vec::new();
                secondaries.push(existing.style);
                secondaries.extend(existing.secondary_styles.iter().copied());
                secondaries.extend(merged.secondary_styles.iter().copied());
                secondaries.sort_by_key(|st| st.as_str());
                secondaries.dedup();
                merged.secondary_styles = secondaries;
                *existing = merged;
            } else {
                // p 的 confidence 不更高：把 p 的风格追加到 existing 的 secondary
                let mut secondaries: Vec<Style> = Vec::new();
                secondaries.push(p.style);
                secondaries.extend(existing.secondary_styles.iter().copied());
                secondaries.sort_by_key(|st| st.as_str());
                secondaries.dedup();
                existing.secondary_styles = secondaries;
            }
        } else {
            by_code.insert(p.stock_code.clone(), p);
        }
    }
    picks.extend(by_code.into_values());
}

/// 按风格分组 + 每组 top N
pub fn group_by_style_and_trim(
    picks: &mut Vec<RecoPick>,
    per_style_limit: usize,
) -> BTreeMap<Style, Vec<RecoPick>> {
    let mut by_style: BTreeMap<Style, Vec<RecoPick>> = BTreeMap::new();
    for p in picks.drain(..) {
        by_style.entry(p.style).or_default().push(p);
    }
    for v in by_style.values_mut() {
        v.sort_by_key(|b| std::cmp::Reverse(b.confidence));
        v.truncate(per_style_limit);
    }
    by_style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_basic_80() {
        // 0.45*0.8 + 0.35*0.8 + 0.15*0.8 + 0.05*0.0 = 0.36+0.28+0.12 = 0.76 → 76
        let c = calc_confidence(0.8, 0.8, 0.8, 0.0, 1.0);
        assert_eq!(c, 76, "expected 76 got {}", c);
    }

    #[test]
    fn confidence_full_bull() {
        // 0.45+0.35+0.15+0.05*0.5 = 0.45+0.35+0.15+0.025 = 0.975 → 98
        let c = calc_confidence(1.0, 1.0, 1.0, 0.5, 1.0);
        assert_eq!(c, 98, "expected 98 got {}", c);
    }

    #[test]
    fn confidence_turnover_anomaly_graded() {
        // 正常 score 0.76 → 3-5x 扣 25% → 0.76*0.75 = 0.57 → 57
        let c = calc_confidence(0.8, 0.8, 0.8, 0.0, 4.0);
        assert_eq!(c, 57, "expected 57 got {}", c);
    }

    #[test]
    fn confidence_clamped_to_100() {
        let c = calc_confidence(1.0, 1.0, 1.0, 1.0, 1.0);
        assert_eq!(c, 100);
    }

    #[test]
    fn position_short_low_conf() {
        // base=5, conf=60, short factor 0.6 → 5*0.6*0.6 = 1.8
        let p = calc_position(5.0, 60, Period::Short);
        assert!((p - 1.8).abs() < 0.01, "got {}", p);
    }

    #[test]
    fn position_long_high_conf() {
        // base=10, conf=80, long factor 1.0 → 10*0.8*1.0 = 8.0
        let p = calc_position(10.0, 80, Period::Long);
        assert!((p - 8.0).abs() < 0.01, "got {}", p);
    }

    #[test]
    fn dedup_keeps_higher_confidence_and_records_secondary() {
        let mut picks = vec![
            RecoPick {
                stock_code: "600519".into(),
                stock_name: "贵州茅台".into(),
                sector: None,
                style: Style::Trend,
                period: Period::Mid,
                price: 100.0,
                entry_low: 99.0,
                entry_high: 101.0,
                stop_loss: 95.0,
                target_price: 110.0,
                position_pct: 5.0,
                holding_days: 28,
                confidence: 70,
                reasons: vec![],
                risk_notes: vec![],
                secondary_styles: vec![],
                synthetic: false,
            },
            RecoPick {
                stock_code: "600519".into(),
                stock_name: "贵州茅台".into(),
                sector: None,
                style: Style::Value,
                period: Period::Mid,
                price: 100.0,
                entry_low: 99.0,
                entry_high: 101.0,
                stop_loss: 95.0,
                target_price: 110.0,
                position_pct: 5.0,
                holding_days: 28,
                confidence: 80,
                reasons: vec![],
                risk_notes: vec![],
                secondary_styles: vec![],
                synthetic: false,
            },
        ];
        dedup_and_merge(&mut picks);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].style, Style::Value);
        assert_eq!(picks[0].confidence, 80);
        assert!(picks[0].secondary_styles.contains(&Style::Trend));
    }

    #[test]
    fn dedup_preserves_existing_secondary_styles() {
        // A 已被 Trend 命中，secondary=[Value]
        // C 用更高 confidence 命中，合并后 secondary 应是 [Trend, Value]
        let mut picks = vec![
            RecoPick {
                stock_code: "600519".into(),
                stock_name: "贵州茅台".into(),
                sector: None,
                style: Style::Trend,
                period: Period::Mid,
                price: 100.0,
                entry_low: 99.0,
                entry_high: 101.0,
                stop_loss: 95.0,
                target_price: 110.0,
                position_pct: 5.0,
                holding_days: 28,
                confidence: 60,
                reasons: vec![],
                risk_notes: vec![],
                secondary_styles: vec![Style::Value],
                synthetic: false,
            },
            RecoPick {
                stock_code: "600519".into(),
                stock_name: "贵州茅台".into(),
                sector: None,
                style: Style::Capital,
                period: Period::Mid,
                price: 100.0,
                entry_low: 99.0,
                entry_high: 101.0,
                stop_loss: 95.0,
                target_price: 110.0,
                position_pct: 5.0,
                holding_days: 28,
                confidence: 80,
                reasons: vec![],
                risk_notes: vec![],
                secondary_styles: vec![],
                synthetic: false,
            },
        ];
        dedup_and_merge(&mut picks);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].style, Style::Capital);
        let secs = &picks[0].secondary_styles;
        assert!(secs.contains(&Style::Trend), "应保留 Trend: {:?}", secs);
        assert!(secs.contains(&Style::Value), "应保留 Value: {:?}", secs);
        assert_eq!(secs.len(), 2, "去重后应只有 2 个: {:?}", secs);
    }

    #[test]
    fn group_by_style_trims_to_limit() {
        let mut picks: Vec<RecoPick> = (0..15)
            .map(|i| RecoPick {
                stock_code: format!("{}", i),
                stock_name: "X".into(),
                sector: None,
                style: Style::Trend,
                period: Period::Short,
                price: 10.0,
                entry_low: 9.5,
                entry_high: 10.5,
                stop_loss: 9.0,
                target_price: 11.0,
                position_pct: 3.0,
                holding_days: 5,
                confidence: i as u8,
                reasons: vec![],
                risk_notes: vec![],
                secondary_styles: vec![],
                synthetic: false,
            })
            .collect();
        let grouped = group_by_style_and_trim(&mut picks, 10);
        assert_eq!(grouped.get(&Style::Trend).unwrap().len(), 10);
    }
}
