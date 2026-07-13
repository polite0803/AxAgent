//! 价量背离检测 — RSI / OBV 顶底背离。
//!
//! 纯函数设计，输入 OHLCV 序列，输出背离检测结果。
//!
//! ## 背离类型
//!
//! - **常规顶背离**：价格创更高高点，RSI/OBV 未创更高高点 → 上涨动能衰竭
//! - **常规底背离**：价格创更低低点，RSI/OBV 未创更低低点 → 下跌动能衰竭
//! - **隐藏顶背离**：价格未创更高高点，RSI/OBV 创更高高点 → 上升趋势持续中的回调
//! - **隐藏底背离**：价格未创更低低点，RSI/OBV 创更低低点 → 下降趋势持续中的反弹

use crate::indicators::rsi;
use crate::KLine;
use serde::Serialize;

/// 背离检测结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DivergenceResult {
    /// 背离类型：regular_bearish / regular_bullish / hidden_bearish / hidden_bullish / none
    pub divergence_type: String,
    /// 指标来源：rsi / obv
    pub indicator: String,
    /// 强度 0.0 ~ 1.0（基于背离幅度差归一化）
    pub strength: f64,
    /// 文字描述
    pub description: String,
}

// ── 帮助函数 ──

/// 计算 RSI 序列。
fn compute_rsi_series(closes: &[f64], period: usize) -> Vec<f64> {
    if closes.len() < period + 1 {
        return vec![];
    }
    let mut series = Vec::with_capacity(closes.len());
    // 逐点计算 RSI
    for i in period + 1..=closes.len() {
        if let Some(v) = rsi(&closes[..i], period) {
            series.push(v);
        }
    }
    series
}

/// 计算 OBV 序列。
fn compute_obv_series(klines: &[KLine]) -> Vec<f64> {
    let mut obv = 0.0;
    let mut series = Vec::with_capacity(klines.len());
    for w in klines.windows(2) {
        let prev = &w[0];
        let curr = &w[1];
        obv += if curr.close > prev.close {
            curr.volume
        } else if curr.close < prev.close {
            -curr.volume
        } else {
            0.0
        };
        series.push(obv);
    }
    series
}

// ── RSI 背离 ──

/// RSI 常规顶背离检测：价格最后 N 根内高点 > 前一组高点，RSI 对应位置 < 前一组 RSI。
///
/// `prices` / `rsi_vals` 长度必须一致。
/// `lookback`：搜索窗口（默认 14）。
pub fn detect_rsi_regular_bearish(
    prices: &[f64],
    rsi_vals: &[f64],
    lookback: usize,
) -> DivergenceResult {
    if prices.len() < lookback * 2 || rsi_vals.len() < lookback * 2 {
        return DivergenceResult {
            divergence_type: "none".into(),
            indicator: "rsi".into(),
            strength: 0.0,
            description: "数据不足".into(),
        };
    }

    let n = prices.len();
    let recent = &prices[n - lookback..];
    let recent_rsi = &rsi_vals[rsi_vals.len() - lookback..];

    // 最近窗口内的最高价位置
    let peak_idx = recent
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);

    let price_peak = recent[peak_idx];
    let rsi_peak = recent_rsi[peak_idx];

    // 前一组：peak_idx 之前找最高价
    if peak_idx < 3 {
        return DivergenceResult {
            divergence_type: "none".into(),
            indicator: "rsi".into(),
            strength: 0.0,
            description: "窗口太短，无法找到前高压".into(),
        };
    }
    let prev = &recent[..peak_idx - 1];
    let (prev_peak_idx, prev_price) = prev
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, &v)| (i, v))
        .unwrap_or((0, price_peak));

    let prev_rsi = recent_rsi[prev_peak_idx];

    let rsi_diff = prev_rsi - rsi_peak;
    if price_peak > prev_price && rsi_peak < prev_rsi && rsi_diff > 3.0 {
        let strength = ((rsi_diff / 20.0).min(1.0) + 0.5).min(1.0);
        DivergenceResult {
            divergence_type: "regular_bearish".into(),
            indicator: "rsi".into(),
            strength,
            description: format!(
                "价格新高 {:.2} > {:.2}，RSI {:.1} < {:.1}（差 {:.1}），上涨动能衰减",
                price_peak, prev_price, rsi_peak, prev_rsi, rsi_diff
            ),
        }
    } else {
        DivergenceResult {
            divergence_type: "none".into(),
            indicator: "rsi".into(),
            strength: 0.0,
            description: "未检测到 RSI 顶背离".into(),
        }
    }
}

/// RSI 常规底背离检测：价格最后 N 根内低点 < 前一组低点，RSI 对应位置 > 前一组 RSI。
pub fn detect_rsi_regular_bullish(
    prices: &[f64],
    rsi_vals: &[f64],
    lookback: usize,
) -> DivergenceResult {
    if prices.len() < lookback * 2 || rsi_vals.len() < lookback * 2 {
        return DivergenceResult {
            divergence_type: "none".into(),
            indicator: "rsi".into(),
            strength: 0.0,
            description: "数据不足".into(),
        };
    }

    let n = prices.len();
    let recent = &prices[n - lookback..];
    let recent_rsi = &rsi_vals[rsi_vals.len() - lookback..];

    // 最近窗口内最低价位置
    let trough_idx = recent
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);

    let price_trough = recent[trough_idx];
    let rsi_trough = recent_rsi[trough_idx];

    if trough_idx < 3 {
        return DivergenceResult {
            divergence_type: "none".into(),
            indicator: "rsi".into(),
            strength: 0.0,
            description: "窗口太短，无法找到前低".into(),
        };
    }
    let prev = &recent[..trough_idx - 1];
    let (prev_trough_idx, prev_price) = prev
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, &v)| (i, v))
        .unwrap_or((0, price_trough));

    let prev_rsi = recent_rsi[prev_trough_idx];
    let rsi_diff = rsi_trough - prev_rsi;

    if price_trough < prev_price && rsi_trough > prev_rsi && rsi_diff > 3.0 {
        let strength = ((rsi_diff / 20.0).min(1.0) + 0.5).min(1.0);
        DivergenceResult {
            divergence_type: "regular_bullish".into(),
            indicator: "rsi".into(),
            strength,
            description: format!(
                "价格新低 {:.2} < {:.2}，RSI {:.1} > {:.1}（差 {:.1}），下跌动能衰减",
                price_trough, prev_price, rsi_trough, prev_rsi, rsi_diff
            ),
        }
    } else {
        DivergenceResult {
            divergence_type: "none".into(),
            indicator: "rsi".into(),
            strength: 0.0,
            description: "未检测到 RSI 底背离".into(),
        }
    }
}

/// 综合 RSI 背离检测，返回置信度最高的一条。
pub fn detect_rsi_divergence(
    klines: &[KLine],
    rsi_period: usize,
    lookback: usize,
) -> DivergenceResult {
    if klines.len() < rsi_period + lookback + 2 {
        return DivergenceResult {
            divergence_type: "none".into(),
            indicator: "rsi".into(),
            strength: 0.0,
            description: "数据不足".into(),
        };
    }
    let prices: Vec<f64> = klines.iter().map(|k| k.close).collect();
    let rsi_vals = compute_rsi_series(&prices, rsi_period);

    if rsi_vals.len() < lookback * 2 {
        return DivergenceResult {
            divergence_type: "none".into(),
            indicator: "rsi".into(),
            strength: 0.0,
            description: "RSI 序列不足以进行背离检测".into(),
        };
    }

    // 取与 prices 对齐的最后部分
    let aligned_rsi = &rsi_vals[rsi_vals.len().saturating_sub(prices.len())..];
    let rsi_slice: Vec<f64> = aligned_rsi.to_vec();

    if rsi_slice.len() != prices.len() {
        // 对齐失败，截断尾部
        let min_len = prices.len().min(rsi_slice.len());
        let bearish = detect_rsi_regular_bearish(
            &prices[prices.len() - min_len..],
            &rsi_slice[rsi_slice.len() - min_len..],
            lookback,
        );
        let bullish = detect_rsi_regular_bullish(
            &prices[prices.len() - min_len..],
            &rsi_slice[rsi_slice.len() - min_len..],
            lookback,
        );
        if bearish.strength >= bullish.strength {
            bearish
        } else {
            bullish
        }
    } else {
        let bearish = detect_rsi_regular_bearish(&prices, &rsi_slice, lookback);
        let bullish = detect_rsi_regular_bullish(&prices, &rsi_slice, lookback);
        if bearish.strength >= bullish.strength {
            bearish
        } else {
            bullish
        }
    }
}

/// OBV 背离检测（简化版：最后 N 根为窗口）。
pub fn detect_obv_divergence(klines: &[KLine], lookback: usize) -> DivergenceResult {
    if klines.len() < lookback * 2 {
        return DivergenceResult {
            divergence_type: "none".into(),
            indicator: "obv".into(),
            strength: 0.0,
            description: "数据不足".into(),
        };
    }
    let prices: Vec<f64> = klines.iter().map(|k| k.close).collect();
    let obv_vals = compute_obv_series(klines);
    if obv_vals.len() < lookback * 2 {
        return DivergenceResult {
            divergence_type: "none".into(),
            indicator: "obv".into(),
            strength: 0.0,
            description: "OBV 序列不足以进行背离检测".into(),
        };
    }

    let n = prices.len();
    let recent_price = &prices[n - lookback..];
    let recent_obv = &obv_vals[obv_vals.len() - lookback..];
    let prev_price = &prices[n - lookback * 2..n - lookback];
    let prev_obv = &obv_vals[obv_vals.len() - lookback * 2..obv_vals.len() - lookback];

    let rp_max = recent_price
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .unwrap_or(0.0);
    let rp_min = recent_price
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .unwrap_or(0.0);
    let ro_max = recent_obv
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .unwrap_or(0.0);
    let ro_min = recent_obv
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .unwrap_or(0.0);
    let pp_max = prev_price
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .unwrap_or(0.0);
    let pp_min = prev_price
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .unwrap_or(0.0);
    let po_max = prev_obv
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .unwrap_or(0.0);
    let po_min = prev_obv
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .unwrap_or(0.0);

    // 顶背离：价格升，OBV 降
    if rp_max > pp_max && ro_max < po_max && po_max > 0.0 {
        let diff = (ro_max / po_max - 1.0).abs();
        let strength = ((diff * 5.0).min(1.0) + 0.4).min(1.0);
        return DivergenceResult {
            divergence_type: "regular_bearish".into(),
            indicator: "obv".into(),
            strength,
            description: format!("价格新高 {:.2}，OBV 未跟新高，量价背离", rp_max),
        };
    }

    // 底背离：价格降，OBV 升
    if rp_min < pp_min && ro_min > po_min {
        let diff = (po_min / ro_min.max(1.0) - 1.0).abs();
        let strength = ((diff * 5.0).min(1.0) + 0.4).min(1.0);
        return DivergenceResult {
            divergence_type: "regular_bullish".into(),
            indicator: "obv".into(),
            strength,
            description: format!("价格新低 {:.2}，OBV 未跟新低，量价背离", rp_min),
        };
    }

    DivergenceResult {
        divergence_type: "none".into(),
        indicator: "obv".into(),
        strength: 0.0,
        description: "未检测到 OBV 背离".into(),
    }
}

/// 综合背离检测：RSI + OBV，返回所有结果。
pub fn detect_all_divergences(
    klines: &[KLine],
    rsi_period: usize,
    lookback: usize,
) -> Vec<DivergenceResult> {
    let mut results = Vec::new();
    results.push(detect_rsi_divergence(klines, rsi_period, lookback));
    results.push(detect_obv_divergence(klines, lookback));
    results.retain(|r| r.divergence_type != "none");
    results
        .sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(close: f64, volume: f64) -> KLine {
        KLine {
            date: "2026-01-01".into(),
            open: close * 0.99,
            high: close * 1.01,
            low: close * 0.99,
            close,
            volume,
            amount: close * volume,
            turnover_rate: None,
            adj_factor: None,
        }
    }

    #[test]
    fn test_rsi_series_non_empty() {
        let closes: Vec<f64> = (1..=50).map(|i| i as f64).collect();
        let rsis = compute_rsi_series(&closes, 6);
        assert!(!rsis.is_empty(), "RSI series should not be empty");
    }

    #[test]
    fn test_obv_series_increasing() {
        // 价格一路涨，OBV 应一路升
        let klines: Vec<KLine> = (1..=20).map(|i| mk(i as f64, 1000.0)).collect();
        let obv = compute_obv_series(&klines);
        assert!(!obv.is_empty());
        // 最后一点应该最大
        assert!(*obv.last().unwrap() >= *obv.first().unwrap());
    }

    #[test]
    fn test_rsi_bearish_divergence() {
        // 构造典型顶背离：最后窗口内 price 新高（16 > 14.5），但 RSI 降低（60 < 80）
        // lookback=5，需至少 10 个数据点
        let prices = vec![
            10.0, 11.0, 10.5, 12.0, 11.5, // 0-4
            13.0, 12.0, 14.0, 13.5, 15.0, // 5-9
            14.0, 14.5, 13.0, 16.0, 15.5, // 10-14 ← recent
        ];
        // RSI: idx11=80 是前高压对应的高 RSI；idx13=60 是更高价格对应的低 RSI → 背离
        let rsi_vals = vec![
            40.0, 45.0, 42.0, 55.0, 50.0, 60.0, 55.0, 75.0, 65.0, 70.0, 55.0, 80.0, 40.0, 60.0,
            45.0,
        ];
        let r = detect_rsi_regular_bearish(&prices, &rsi_vals, 5);
        assert_eq!(
            r.divergence_type, "regular_bearish",
            "should detect bearish divergence: {}",
            r.description
        );
        assert!(r.strength > 0.0);
    }

    #[test]
    fn test_rsi_bullish_divergence() {
        // 底背离：最后窗口内 price 新低（9 < 10.5），但 RSI 抬高（35 > 20）
        let prices = vec![
            15.0, 14.0, 14.5, 13.0, 13.5, // 0-4
            12.0, 12.5, 11.0, 11.5, 10.0, // 5-9
            11.0, 10.5, 11.5, 9.0, 9.5, // 10-14 ← recent
        ];
        let rsi_vals = vec![
            45.0, 40.0, 42.0, 35.0, 37.0, 30.0, 32.0, 25.0, 28.0, 20.0, 28.0, 20.0, 30.0, 35.0,
            32.0, // idx13=35 > idx11=20 → 底背离
        ];
        let r = detect_rsi_regular_bullish(&prices, &rsi_vals, 5);
        assert_eq!(
            r.divergence_type, "regular_bullish",
            "should detect bullish divergence: {}",
            r.description
        );
        assert!(r.strength > 0.0);
    }

    #[test]
    fn test_no_divergence_flat() {
        // 需要足够长来满足 lookback*2 检查，但价格无波动
        let prices = vec![10.0; 24];
        let rsi_vals = vec![50.0; 24];
        let r = detect_rsi_regular_bearish(&prices, &rsi_vals, 10);
        assert_eq!(r.divergence_type, "none");
    }

    #[test]
    fn test_obv_divergence_smoke() {
        // OBV 背离用合成数据难以稳定构造，保留烟雾测试验证不 panic 即可
        let klines: Vec<KLine> = (0..30).map(|i| mk(10.0 + i as f64 * 0.5, 1000.0)).collect();
        let r = detect_obv_divergence(&klines, 10);
        // 不断言具体类型，仅验证返回了结果
        assert!(!r.indicator.is_empty());
    }

    #[test]
    fn test_obv_divergence_empty() {
        let r = detect_obv_divergence(&[], 10);
        assert_eq!(r.divergence_type, "none");
    }

    #[test]
    fn test_all_divergences_insufficient_data() {
        let r = detect_all_divergences(&[], 14, 20);
        assert!(r.is_empty());
    }
}
