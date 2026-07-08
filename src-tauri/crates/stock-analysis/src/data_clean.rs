//! 数据清洗 — 异常值剔除、缺失值填充、复权计算。
//!
//! 纯函数，接收 JSON 字符串输入，输出清洗后的数据。

use serde::{Deserialize, Serialize};

// ── 异常值剔除 ──

#[derive(Debug, Clone, Serialize)]
pub struct OutlierResult {
    pub cleaned: Vec<f64>,
    pub removed_count: usize,
    pub removed_indices: Vec<usize>,
    pub method: String,
}

/// 剔除异常值。method: "zscore" 或 "iqr"。
/// prices_json: f64 数组 JSON 字符串。
/// threshold: zscore 方法的 sigma 倍数（默认 2.0），或 IQR 方法的倍数（默认 1.5）。
pub fn remove_outliers(prices_json: &str, method: &str, threshold: f64) -> OutlierResult {
    let prices: Vec<f64> = serde_json::from_str(prices_json).unwrap_or_default();
    if prices.len() < 4 {
        return OutlierResult {
            cleaned: prices,
            removed_count: 0,
            removed_indices: vec![],
            method: method.into(),
        };
    }
    match method {
        "iqr" => remove_outliers_iqr(&prices, threshold),
        _ => remove_outliers_zscore(&prices, threshold),
    }
}

fn remove_outliers_zscore(prices: &[f64], threshold: f64) -> OutlierResult {
    let n = prices.len();
    let mean: f64 = prices.iter().sum::<f64>() / n as f64;
    let variance: f64 = prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let stddev = variance.sqrt();
    if stddev < 1e-10 {
        return OutlierResult {
            cleaned: prices.to_vec(),
            removed_count: 0,
            removed_indices: vec![],
            method: "zscore".into(),
        };
    }
    let mut cleaned = Vec::new();
    let mut removed = Vec::new();
    for (i, &p) in prices.iter().enumerate() {
        let z = (p - mean).abs() / stddev;
        if z > threshold {
            let clamped = if p > mean {
                mean + threshold * stddev
            } else {
                mean - threshold * stddev
            };
            cleaned.push((clamped * 100.0).round() / 100.0);
            removed.push(i);
        } else {
            cleaned.push(p);
        }
    }
    OutlierResult {
        cleaned,
        removed_count: removed.len(),
        removed_indices: removed,
        method: "zscore".into(),
    }
}

fn remove_outliers_iqr(prices: &[f64], multiplier: f64) -> OutlierResult {
    let mut sorted = prices.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q1_idx = (sorted.len() as f64 * 0.25).floor() as usize;
    let q3_idx = (sorted.len() as f64 * 0.75).floor() as usize;
    let q1 = sorted[q1_idx];
    let q3 = sorted[q3_idx];
    let iqr = q3 - q1;
    if iqr < 1e-10 {
        return OutlierResult {
            cleaned: prices.to_vec(),
            removed_count: 0,
            removed_indices: vec![],
            method: "iqr".into(),
        };
    }
    let lower = q1 - multiplier * iqr;
    let upper = q3 + multiplier * iqr;
    let mut cleaned = Vec::new();
    let mut removed = Vec::new();
    for (i, &p) in prices.iter().enumerate() {
        if p < lower {
            cleaned.push((lower * 100.0).round() / 100.0);
            removed.push(i);
        } else if p > upper {
            cleaned.push((upper * 100.0).round() / 100.0);
            removed.push(i);
        } else {
            cleaned.push(p);
        }
    }
    OutlierResult {
        cleaned,
        removed_count: removed.len(),
        removed_indices: removed,
        method: "iqr".into(),
    }
}

// ── 缺失值填充 ──

#[derive(Debug, Clone, Serialize)]
pub struct FillResult {
    pub filled: Vec<Option<f64>>,
    pub filled_count: usize,
    pub method: String,
}

/// 填充缺失值（JSON null → 填充值）。
/// method: "forward" (前向填充) 或 "linear" (线性插值)。
pub fn fill_missing(prices_json: &str, method: &str) -> FillResult {
    let prices: Vec<Option<f64>> = serde_json::from_str(prices_json).unwrap_or_default();
    if prices.is_empty() {
        return FillResult {
            filled: vec![],
            filled_count: 0,
            method: method.into(),
        };
    }
    match method {
        "linear" => fill_linear(&prices),
        _ => fill_forward(&prices),
    }
}

fn fill_forward(prices: &[Option<f64>]) -> FillResult {
    let mut result = prices.to_vec();
    let mut last_valid: Option<f64> = None;
    let mut count = 0usize;
    for v in result.iter_mut() {
        if let Some(val) = v {
            last_valid = Some(*val);
        } else if let Some(fill) = last_valid {
            *v = Some(fill);
            count += 1;
        }
    }
    FillResult {
        filled: result,
        filled_count: count,
        method: "forward".into(),
    }
}

fn fill_linear(prices: &[Option<f64>]) -> FillResult {
    let mut result = prices.to_vec();
    let mut count = 0usize;
    let n = result.len();
    // 找第一个有效值
    let first_valid = result.iter().position(|v| v.is_some());
    if first_valid.is_none() {
        return FillResult {
            filled: result,
            filled_count: 0,
            method: "linear".into(),
        };
    }
    let first = first_valid.unwrap();
    // 前向填充头部
    let head_val = result[first].unwrap();
    for v in result.iter_mut().take(first) {
        *v = Some(head_val);
        count += 1;
    }
    // 线性插值间隙
    let mut i = first;
    while i < n {
        if result[i].is_some() {
            i += 1;
            continue;
        }
        let gap_start = i;
        while i < n && result[i].is_none() {
            i += 1;
        }
        let gap_end = i;
        if gap_end < n {
            let left = result[gap_start - 1].unwrap();
            let right = result[gap_end].unwrap();
            let steps = (gap_end - gap_start + 1) as f64;
            for (j, v) in result.iter_mut().enumerate().take(gap_end).skip(gap_start) {
                let t = (j - gap_start + 1) as f64 / steps;
                *v = Some(left + (right - left) * t);
                count += 1;
            }
        } else {
            // 尾部：用最后一个有效值填充
            let tail_val = result[gap_start - 1].unwrap();
            for v in result.iter_mut().skip(gap_start) {
                *v = Some(tail_val);
                count += 1;
            }
        }
    }
    FillResult {
        filled: result,
        filled_count: count,
        method: "linear".into(),
    }
}

// ── 复权计算 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedKLine {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdjustResult {
    pub adjusted_klines: Vec<AdjustedKLine>,
    pub adjustment_factor: f64,
}

/// 前复权：将历史价格乘调整因子。
/// klines_json: KLine JSON 数组字符串（含 date/open/high/low/close/volume）。
/// dividends_json: 分红 JSON 数组字符串（含 date/cash_dividend/share_dividend）。
pub fn adjust_prices(klines_json: &str, dividends_json: &str) -> AdjustResult {
    #[derive(Deserialize)]
    struct RawKLine {
        date: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        #[serde(default)]
        volume: f64,
    }
    #[derive(Deserialize)]
    struct Dividend {
        date: String,
        #[serde(default)]
        cash_dividend: f64,
        #[serde(default)]
        share_dividend: f64,
    }

    let mut klines: Vec<RawKLine> = serde_json::from_str(klines_json).unwrap_or_default();
    let dividends: Vec<Dividend> = serde_json::from_str(dividends_json).unwrap_or_default();

    if klines.is_empty() {
        return AdjustResult {
            adjusted_klines: vec![],
            adjustment_factor: 1.0,
        };
    }

    // 按日期排序（最新在前）
    klines.sort_by(|a, b| b.date.cmp(&a.date));

    let mut factor = 1.0;
    for k in klines.iter_mut() {
        for d in &dividends {
            if d.date == k.date {
                let total_return = d.cash_dividend / k.close + d.share_dividend;
                if total_return > 0.0 {
                    factor /= 1.0 + total_return;
                }
            }
        }
        k.open = (k.open * factor * 100.0).round() / 100.0;
        k.high = (k.high * factor * 100.0).round() / 100.0;
        k.low = (k.low * factor * 100.0).round() / 100.0;
        k.close = (k.close * factor * 100.0).round() / 100.0;
        k.volume = (k.volume / factor * 100.0).round() / 100.0;
    }

    let adjusted: Vec<AdjustedKLine> = klines
        .into_iter()
        .map(|k| AdjustedKLine {
            date: k.date,
            open: k.open,
            high: k.high,
            low: k.low,
            close: k.close,
            volume: k.volume,
        })
        .collect();

    AdjustResult {
        adjusted_klines: adjusted,
        adjustment_factor: (factor * 10000.0).round() / 10000.0,
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outliers_zscore() {
        let json = "[10.0, 10.2, 10.1, 50.0, 10.3, 10.0]";
        let r = remove_outliers(json, "zscore", 2.0);
        assert_eq!(r.removed_count, 1);
        assert_eq!(r.removed_indices[0], 3);
    }

    #[test]
    fn test_outliers_iqr() {
        let json = "[10.0, 10.2, 10.1, 50.0, 10.3, 10.0]";
        let r = remove_outliers(json, "iqr", 1.5);
        assert!(r.removed_count >= 1);
    }

    #[test]
    fn test_fill_forward() {
        let json = "[10.0, null, null, 10.3, null, 10.5]";
        let r = fill_missing(json, "forward");
        assert_eq!(r.filled_count, 3);
        assert_eq!(r.filled[1].unwrap(), 10.0);
        assert_eq!(r.filled[2].unwrap(), 10.0);
        assert_eq!(r.filled[4].unwrap(), 10.3);
    }

    #[test]
    fn test_fill_linear() {
        let json = "[10.0, null, null, 13.0]";
        let r = fill_missing(json, "linear");
        assert_eq!(r.filled_count, 2);
        // 插值: (13-10)/3 steps, positions 1 and 2
        assert!((r.filled[1].unwrap() - 11.0).abs() < 0.1);
        assert!((r.filled[2].unwrap() - 12.0).abs() < 0.1);
    }

    #[test]
    fn test_adjust_prices() {
        let klines = r#"[{"date":"2024-01-03","open":10.0,"high":10.5,"low":9.5,"close":10.2,"volume":1000},{"date":"2024-01-02","open":9.8,"high":10.0,"low":9.6,"close":9.9,"volume":800},{"date":"2024-01-01","open":9.5,"high":9.8,"low":9.4,"close":9.7,"volume":600}]"#;
        let dividends = r#"[{"date":"2024-01-02","cash_dividend":0.5,"share_dividend":0.0}]"#;
        let r = adjust_prices(klines, dividends);
        assert_eq!(r.adjusted_klines.len(), 3);
        assert!(r.adjustment_factor < 1.0, "前复权因子应小于1");
        assert!(r.adjusted_klines[0].close > 0.0, "最新价不变");
        assert!(r.adjusted_klines[2].close < 9.7, "历史价应被调低");
    }
}
