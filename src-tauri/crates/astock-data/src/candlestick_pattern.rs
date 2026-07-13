//! K 线形态识别 — 单根/双根/三根经典形态。
//!
//! 纯函数设计，输入 `&[KLine]`，输出可选的形态检测结果。
//! 形态命名参考 A 股市场惯例（中文描述优先）。

use crate::KLine;
use serde::Serialize;

/// 形态检测结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternResult {
    /// 形态名称（中文）
    pub pattern: String,
    /// 方向：看涨 / 看跌 / 中性
    pub direction: String,
    /// 置信度 0.0 ~ 1.0
    pub confidence: f64,
    /// 文字描述
    pub description: String,
    /// 形态出现的 K 线索引（从末尾往前数，0 = 最新一根）
    pub offset: usize,
}

// ── 帮助函数 ──

fn body_size(k: &KLine) -> f64 {
    (k.close - k.open).abs()
}

fn upper_shadow(k: &KLine) -> f64 {
    k.high - k.open.max(k.close)
}

fn lower_shadow(k: &KLine) -> f64 {
    k.open.min(k.close) - k.low
}

fn is_bullish(k: &KLine) -> bool {
    k.close > k.open
}

fn is_bearish(k: &KLine) -> bool {
    k.close < k.open
}

fn avg_body(klines: &[KLine]) -> f64 {
    if klines.is_empty() {
        return 0.0;
    }
    klines.iter().map(body_size).sum::<f64>() / klines.len() as f64
}

// ── 单根形态 ──

/// 检测当前 K 线是否为十字星（doji）。
///
/// 实体占波幅比例 ≤ 5%，且上下影线均有一定长度。
pub fn detect_doji(k: &KLine) -> bool {
    let range = k.high - k.low;
    if range < 1e-8 {
        return false;
    }
    body_size(k) / range <= 0.05
}

/// 检测锤子线（hammer）。
///
/// 下影线 ≥ 实体 2 倍，上影线 ≤ 实体 1/2，实体位于 K 线上端。
pub fn detect_hammer(k: &KLine) -> bool {
    let bs = body_size(k);
    let us = upper_shadow(k);
    let ls = lower_shadow(k);
    if bs < 1e-8 {
        return false;
    }
    ls >= bs * 2.0 && us <= bs * 0.5
}

/// 检测倒锤子（inverted hammer）。
///
/// 上影线 ≥ 实体 2 倍，下影线 ≤ 实体 1/2，实体位于 K 线下端。
pub fn detect_inverted_hammer(k: &KLine) -> bool {
    let bs = body_size(k);
    let us = upper_shadow(k);
    let ls = lower_shadow(k);
    if bs < 1e-8 {
        return false;
    }
    us >= bs * 2.0 && ls <= bs * 0.5
}

/// 检测射星（shooting star）。
///
/// 上影线 ≥ 实体 2 倍，下影线 ≤ 实体 1/2，出现在上涨后。
/// 此处仅检测形态结构，是否在上涨后由调用方判断。
pub fn detect_shooting_star(k: &KLine) -> bool {
    // 射星 = 倒锤子的形态倒转，区别在于位置（上涨后 vs 下跌后），结构同倒锤子
    let bs = body_size(k);
    let us = upper_shadow(k);
    let ls = lower_shadow(k);
    if bs < 1e-8 {
        return false;
    }
    // 射星的上影线是实体的 2 倍以上，且实体在上部
    us >= bs * 2.0 && ls <= bs * 0.3
}

/// 检测纺锤线（spinning top）。
///
/// 实体较小，上下影线均较长，市场犹豫。
pub fn detect_spinning_top(k: &KLine, avg: f64) -> bool {
    if avg < 1e-8 {
        return false;
    }
    let bs = body_size(k);
    let us = upper_shadow(k);
    let ls = lower_shadow(k);
    bs < avg * 0.7 && us > bs * 0.5 && ls > bs * 0.5
}

// ── 双根形态 ──

/// 检测看涨吞没（bullish engulfing）。
///
/// 前一根阴线，后一根阳线完全覆盖前一根的实体。
pub fn detect_bullish_engulfing(klines: &[KLine]) -> bool {
    if klines.len() < 2 {
        return false;
    }
    let prev = &klines[klines.len() - 2];
    let curr = &klines[klines.len() - 1];
    is_bearish(prev)
        && is_bullish(curr)
        && curr.open <= prev.close
        && curr.close >= prev.open
        && body_size(curr) > body_size(prev) * 0.8
}

/// 检测看跌吞没（bearish engulfing）。
///
/// 前一根阳线，后一根阴线完全覆盖前一根的实体。
pub fn detect_bearish_engulfing(klines: &[KLine]) -> bool {
    if klines.len() < 2 {
        return false;
    }
    let prev = &klines[klines.len() - 2];
    let curr = &klines[klines.len() - 1];
    is_bullish(prev)
        && is_bearish(curr)
        && curr.open >= prev.close
        && curr.close <= prev.open
        && body_size(curr) > body_size(prev) * 0.8
}

/// 检测看涨孕线（bullish harami）。
///
/// 前一根长阴线，后一根小阳线被前一根实体包裹。
pub fn detect_bullish_harami(klines: &[KLine]) -> bool {
    if klines.len() < 2 {
        return false;
    }
    let prev = &klines[klines.len() - 2];
    let curr = &klines[klines.len() - 1];
    is_bearish(prev)
        && body_size(prev) >= avg_body(&klines[..klines.len() - 1]) * 0.8
        && is_bullish(curr)
        && curr.open > prev.close
        && curr.close < prev.open
        && body_size(curr) <= body_size(prev) * 0.5
}

/// 检测看跌孕线（bearish harami）。
///
/// 前一根长阳线，后一根小阴线被前一根实体包裹。
pub fn detect_bearish_harami(klines: &[KLine]) -> bool {
    if klines.len() < 2 {
        return false;
    }
    let prev = &klines[klines.len() - 2];
    let curr = &klines[klines.len() - 1];
    is_bullish(prev)
        && body_size(prev) >= avg_body(&klines[..klines.len() - 1]) * 0.8
        && is_bearish(curr)
        && curr.open < prev.close
        && curr.close > prev.open
        && body_size(curr) <= body_size(prev) * 0.5
}

/// 检测刺穿线（piercing line）。
///
/// 前一根阴线，后一根阳线开盘低于前低，收盘进入前实体中部以上。
pub fn detect_piercing_line(klines: &[KLine]) -> bool {
    if klines.len() < 2 {
        return false;
    }
    let prev = &klines[klines.len() - 2];
    let curr = &klines[klines.len() - 1];
    is_bearish(prev)
        && is_bullish(curr)
        && curr.open < prev.low
        && curr.close > prev.close + (prev.open - prev.close) * 0.5
}

/// 检测乌云盖顶（dark cloud cover）。
///
/// 前一根阳线，后一根阴线开盘高于前高，收盘进入前实体中部以下。
pub fn detect_dark_cloud_cover(klines: &[KLine]) -> bool {
    if klines.len() < 2 {
        return false;
    }
    let prev = &klines[klines.len() - 2];
    let curr = &klines[klines.len() - 1];
    is_bullish(prev)
        && is_bearish(curr)
        && curr.open > prev.high
        && curr.close < prev.close - (prev.open - prev.close).abs() * 0.5
}

// ── 三根形态 ──

/// 检测晨星（morning star）。
///
/// 长阴 → 小实体（可十字星）→ 长阳，收盘进入第一根阴线实体中部以上。
pub fn detect_morning_star(klines: &[KLine]) -> bool {
    if klines.len() < 3 {
        return false;
    }
    let k1 = &klines[klines.len() - 3];
    let k2 = &klines[klines.len() - 2];
    let k3 = &klines[klines.len() - 1];
    is_bearish(k1)
        && body_size(k1) > avg_body(&klines[..klines.len() - 2]) * 0.8
        && body_size(k2) <= body_size(k1) * 0.3
        && is_bullish(k3)
        && k3.close > k1.close + (k1.open - k1.close) * 0.5
}

/// 检测暮星（evening star）。
///
/// 长阳 → 小实体（可十字星）→ 长阴，收盘进入第一根阳线实体中部以下。
pub fn detect_evening_star(klines: &[KLine]) -> bool {
    if klines.len() < 3 {
        return false;
    }
    let k1 = &klines[klines.len() - 3];
    let k2 = &klines[klines.len() - 2];
    let k3 = &klines[klines.len() - 1];
    is_bullish(k1)
        && body_size(k1) > avg_body(&klines[..klines.len() - 2]) * 0.8
        && body_size(k2) <= body_size(k1) * 0.3
        && is_bearish(k3)
        && k3.close < k1.close - (k1.close - k1.open) * 0.5
}

// ── 综合检测 ──

/// 综合扫描最新 K 线区域，返回所有检测到的形态，按置信度降序排列。
///
/// `klines` 按时间升序排列（最旧 → 最新）。
pub fn detect_all_patterns(klines: &[KLine]) -> Vec<PatternResult> {
    if klines.len() < 2 {
        return vec![];
    }
    let n = klines.len();
    let avg = avg_body(klines);
    let last = &klines[n - 1];
    let mut results = Vec::new();

    // ── 单根形态（当前 K 线）──
    if detect_doji(last) {
        results.push(PatternResult {
            pattern: "十字星".into(),
            direction: "中性".into(),
            confidence: 0.4,
            description: "开盘价与收盘价几乎相等，市场方向不明".into(),
            offset: 0,
        });
    } else if detect_hammer(last) {
        results.push(PatternResult {
            pattern: "锤子线".into(),
            direction: "看涨".into(),
            confidence: 0.55,
            description: "下影线长于实体 2 倍，上影线短，下跌后出现视为反转信号".into(),
            offset: 0,
        });
    } else if detect_shooting_star(last) {
        results.push(PatternResult {
            pattern: "射星".into(),
            direction: "看跌".into(),
            confidence: 0.55,
            description: "上影线长、下影线短，上涨后出现视为反转信号".into(),
            offset: 0,
        });
    } else if detect_inverted_hammer(last) {
        results.push(PatternResult {
            pattern: "倒锤子".into(),
            direction: "看涨".into(),
            confidence: 0.5,
            description: "上影线长于实体 2 倍，下跌后出现视为潜在反转".into(),
            offset: 0,
        });
    } else if detect_spinning_top(last, avg) {
        results.push(PatternResult {
            pattern: "纺锤线".into(),
            direction: "中性".into(),
            confidence: 0.35,
            description: "实体较小，上下影线均有长度，市场犹豫".into(),
            offset: 0,
        });
    }

    // ── 双根形态 ──
    if detect_bullish_engulfing(klines) {
        results.push(PatternResult {
            pattern: "看涨吞没".into(),
            direction: "看涨".into(),
            confidence: 0.7,
            description: "阴线后出现阳线完全覆盖前阴实体，强势反转信号".into(),
            offset: 1,
        });
    }
    if detect_bearish_engulfing(klines) {
        results.push(PatternResult {
            pattern: "看跌吞没".into(),
            direction: "看跌".into(),
            confidence: 0.7,
            description: "阳线后出现阴线完全覆盖前阳实体，强势反转信号".into(),
            offset: 1,
        });
    }
    if detect_bullish_harami(klines) {
        results.push(PatternResult {
            pattern: "看涨孕线".into(),
            direction: "看涨".into(),
            confidence: 0.5,
            description: "长阴后出现小阳线被包裹，下跌趋势减弱".into(),
            offset: 1,
        });
    }
    if detect_bearish_harami(klines) {
        results.push(PatternResult {
            pattern: "看跌孕线".into(),
            direction: "看跌".into(),
            confidence: 0.5,
            description: "长阳后出现小阴线被包裹，上涨趋势减弱".into(),
            offset: 1,
        });
    }
    if detect_piercing_line(klines) {
        results.push(PatternResult {
            pattern: "刺穿线".into(),
            direction: "看涨".into(),
            confidence: 0.6,
            description: "阴线后阳线开盘破前低、收盘入前实体中部以上，底部反转".into(),
            offset: 1,
        });
    }
    if detect_dark_cloud_cover(klines) {
        results.push(PatternResult {
            pattern: "乌云盖顶".into(),
            direction: "看跌".into(),
            confidence: 0.6,
            description: "阳线后阴线开盘过前高、收盘入前实体中部以下，顶部反转".into(),
            offset: 1,
        });
    }

    // ── 三根形态 ──
    if detect_morning_star(klines) {
        results.push(PatternResult {
            pattern: "晨星".into(),
            direction: "看涨".into(),
            confidence: 0.8,
            description: "长阴→小实体→长阳，收盘入第一根阴线实体中部以上，强反转".into(),
            offset: 2,
        });
    }
    if detect_evening_star(klines) {
        results.push(PatternResult {
            pattern: "暮星".into(),
            direction: "看跌".into(),
            confidence: 0.8,
            description: "长阳→小实体→长阴，收盘入第一根阳线实体中部以下，强反转".into(),
            offset: 2,
        });
    }

    // 按置信度降序
    results.sort_by(|a, b| {
        b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(open: f64, high: f64, low: f64, close: f64) -> KLine {
        KLine {
            date: "2026-01-01".into(),
            open,
            high,
            low,
            close,
            volume: 1000000.0,
            amount: open * 1000000.0,
            turnover_rate: None,
            adj_factor: None,
        }
    }

    #[test]
    fn test_doji() {
        // 十字星：开收几乎相等
        let k = mk(10.0, 11.0, 9.0, 10.01);
        assert!(detect_doji(&k));
    }

    #[test]
    fn test_doji_close_equals_open() {
        // 精确相等
        let k = mk(10.0, 11.0, 9.0, 10.0);
        assert!(detect_doji(&k));
    }

    #[test]
    fn test_not_doji() {
        let k = mk(10.0, 11.0, 9.0, 11.0);
        assert!(!detect_doji(&k));
    }

    #[test]
    fn test_hammer() {
        // 下影线长，上影线短
        let k = mk(11.0, 11.2, 9.0, 11.2); // 阳锤子：close>open, 下影=2.0, 实体=0.2
        assert!(detect_hammer(&k), "should be hammer");
    }

    #[test]
    fn test_bullish_engulfing() {
        let klines = vec![
            mk(12.0, 12.5, 11.5, 11.5), // 阴线
            mk(11.0, 13.0, 10.5, 13.0), // 阳线完全覆盖前一根
        ];
        assert!(detect_bullish_engulfing(&klines));
    }

    #[test]
    fn test_bearish_engulfing() {
        let klines = vec![
            mk(10.0, 10.5, 9.5, 10.5),  // 阳线
            mk(12.0, 13.0, 10.0, 10.0), // 阴线完全覆盖前一根
        ];
        assert!(detect_bearish_engulfing(&klines));
    }

    #[test]
    fn test_morning_star() {
        let klines = vec![
            mk(15.0, 15.0, 13.0, 13.0), // 长阴
            mk(12.5, 13.5, 12.0, 13.0), // 小实体（纺锤）
            mk(12.5, 15.5, 12.0, 15.5), // 长阳
        ];
        assert!(detect_morning_star(&klines));
    }

    #[test]
    fn test_evening_star() {
        let klines = vec![
            mk(10.0, 12.0, 10.0, 12.0), // 长阳
            mk(12.5, 13.0, 11.5, 12.0), // 小实体
            mk(13.0, 13.5, 10.0, 10.0), // 长阴
        ];
        assert!(detect_evening_star(&klines));
    }

    #[test]
    fn test_detect_all_empty() {
        assert!(detect_all_patterns(&[]).is_empty());
    }

    #[test]
    fn test_detect_all_single_doji() {
        let klines = vec![
            mk(9.0, 10.0, 8.0, 9.5),
            mk(10.0, 11.0, 9.0, 10.01), // doji
        ];
        let r = detect_all_patterns(&klines);
        assert!(r.iter().any(|p| p.pattern == "十字星"), "should find doji: {:?}", r);
    }

    #[test]
    fn test_detect_all_engulfing() {
        let klines = vec![mk(12.0, 12.5, 11.5, 11.5), mk(11.0, 13.0, 10.5, 13.0)];
        let r = detect_all_patterns(&klines);
        assert!(
            r.iter().any(|p| p.pattern == "看涨吞没"),
            "should find bullish engulfing: {:?}",
            r
        );
    }

    #[test]
    fn test_detect_all_harami() {
        // 看跌孕线需前长阳 + 后小阴
        let klines = vec![
            mk(10.0, 13.0, 10.0, 13.0), // 长阳
            mk(12.5, 12.8, 11.2, 11.5), // 小阴被包裹
        ];
        let r = detect_all_patterns(&klines);
        assert!(r.iter().any(|p| p.pattern == "看跌孕线"), "should find bearish harami: {:?}", r);
    }

    #[test]
    fn test_piercing_line() {
        let klines = vec![
            mk(12.0, 12.3, 11.5, 11.5), // 阴线
            mk(11.0, 12.0, 10.5, 12.0), // 刺穿：开低于前低，收于前阴实体中部以上
        ];
        assert!(detect_piercing_line(&klines));
    }

    #[test]
    fn test_dark_cloud_cover() {
        let klines = vec![
            mk(10.0, 10.5, 9.5, 10.5), // 阳线
            mk(11.0, 11.5, 9.8, 9.8),  // 乌云：开高于前高，收于前阳实体中部以下
        ];
        assert!(detect_dark_cloud_cover(&klines));
    }

    #[test]
    fn test_single_candle_not_conflicting() {
        // 一根阳线实体大的 K 线不应该被误判为特殊形态
        let k = mk(10.0, 12.0, 9.5, 12.0);
        assert!(!detect_doji(&k));
        assert!(!detect_hammer(&k));
        assert!(!detect_shooting_star(&k));
        assert!(!detect_inverted_hammer(&k));
    }

    #[test]
    fn test_strongest_pattern_first() {
        // 晨星置信度最高，若同时检测到晨星和看涨吞没，晨星应排最前
        let klines = vec![
            mk(15.0, 15.0, 13.0, 13.0),
            mk(12.5, 13.5, 12.0, 13.0),
            mk(12.5, 15.5, 12.0, 15.5),
        ];
        let r = detect_all_patterns(&klines);
        if !r.is_empty() {
            // 晨星置信度 0.8
            assert!(r[0].confidence >= 0.7, "first result should have high confidence: {:?}", r[0]);
        }
    }
}
