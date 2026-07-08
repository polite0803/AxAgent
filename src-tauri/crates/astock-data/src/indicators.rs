use crate::types::KLine;
use serde::{Deserialize, Serialize};

/// 技术指标计算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechnicalIndicators {
    pub stock_code: String,
    pub latest_date: String,
    /// 均线 SMA
    pub ma5: f64,
    pub ma10: f64,
    pub ma20: f64,
    pub ma60: f64,
    /// MA排列状态: "多头排列", "空头排列", "弱多头", "缠绕/交叉"
    pub ma_alignment: String,
    /// MACD (12/26/9)
    pub macd_dif: f64,
    pub macd_dea: f64,
    pub macd_bar: f64,       // (DIF - DEA) × 2
    pub macd_signal: String, // "金叉", "死叉", "多头运行", "空头运行"
    /// RSI (6/12/24)
    pub rsi6: f64,
    pub rsi12: f64,
    pub rsi24: f64,
    pub rsi_signal: String, // "超买", "超卖", "强势", "弱势", "中性"
    /// 布林带 (20,2)
    pub boll_upper: f64,
    pub boll_mid: f64, // MA20
    pub boll_lower: f64,
    pub boll_position: String, // "上轨以上", "上轨区间", "中轨附近", "下轨区间", "下轨以下"
    /// 乖离率 (%)
    pub bias_ma5: f64, // (close - MA5) / MA5 × 100
    pub bias_ma20: f64,
    /// 量能
    pub volume_ratio: f64, // 当日量 / 5日均量
    pub volume_signal: String, // "放量上涨", "缩量回调", "放量下跌", "缩量上涨", "正常"
    /// 支撑/压力位 (基于近期高低点和均线)
    pub support_levels: Vec<f64>,
    pub resistance_levels: Vec<f64>,
}

/// 指标计算参数配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorConfig {
    pub ma_periods: Vec<usize>,
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    pub rsi_periods: Vec<usize>,
    pub boll_period: usize,
    pub boll_stddev: f64,
    pub volume_lookback: usize,
    pub volume_surge_ratio: f64,
    pub volume_shrink_ratio: f64,
    pub rsi_overbought: f64,
    pub rsi_oversold: f64,
}

impl Default for IndicatorConfig {
    fn default() -> Self {
        Self {
            ma_periods: vec![5, 10, 20, 60],
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            rsi_periods: vec![6, 12, 24],
            boll_period: 20,
            boll_stddev: 2.0,
            volume_lookback: 5,
            volume_surge_ratio: 1.5,
            volume_shrink_ratio: 0.7,
            rsi_overbought: 80.0,
            rsi_oversold: 20.0,
        }
    }
}

/// Compute SMA (Simple Moving Average) — 取最近 period 个数据
fn sma(data: &[f64], period: usize) -> Option<f64> {
    if data.len() < period || period == 0 {
        return None;
    }
    let start = data.len() - period;
    Some(data[start..].iter().sum::<f64>() / period as f64)
}

/// Compute EMA for a single final value
#[allow(dead_code)]
fn ema(data: &[f64], period: usize) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut result = data[0];
    for &val in &data[1..] {
        result = (val - result) * multiplier + result;
    }
    result
}

/// Build complete EMA series (one EMA value per input point)
fn build_ema_series(data: &[f64], period: usize) -> Vec<f64> {
    if data.is_empty() || period == 0 {
        return vec![0.0];
    }
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut result = Vec::with_capacity(data.len());
    let mut ema_val = data[0];
    result.push(ema_val);
    for &val in &data[1..] {
        ema_val = (val - ema_val) * multiplier + ema_val;
        result.push(ema_val);
    }
    result
}

/// Compute RSI (Wilder's smoothing method)
fn rsi(closes: &[f64], period: usize) -> f64 {
    if closes.len() < period + 1 || period == 0 {
        return 50.0;
    }
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=period {
        let diff = closes[i] - closes[i - 1];
        if diff > 0.0 {
            avg_gain += diff;
        } else {
            avg_loss += -diff;
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;
    for i in (period + 1)..closes.len() {
        let diff = closes[i] - closes[i - 1];
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { -diff } else { 0.0 };
        avg_gain = (avg_gain * (period - 1) as f64 + gain) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + loss) / period as f64;
    }
    if avg_loss < 1e-10 {
        return 100.0;
    }
    let rs = avg_gain / avg_loss;
    100.0 - (100.0 / (1.0 + rs))
}

/// Compute sample standard deviation for Bollinger Bands (n-1 denominator)
fn stddev(data: &[f64], mean: f64) -> f64 {
    let n = data.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let variance = data.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    variance.sqrt()
}

/// Compute all technical indicators from K-line data with configurable parameters.
/// Pass `None` for `config` to use default parameters.
pub fn compute_indicators_with_config(
    stock_code: &str,
    klines: &[KLine],
    config: Option<&IndicatorConfig>,
) -> TechnicalIndicators {
    let default_config = IndicatorConfig::default();
    let cfg = config.unwrap_or(&default_config);

    if klines.is_empty() {
        return TechnicalIndicators {
            stock_code: stock_code.to_string(),
            latest_date: String::new(),
            ma5: 0.0,
            ma10: 0.0,
            ma20: 0.0,
            ma60: 0.0,
            ma_alignment: "无数据".to_string(),
            macd_dif: 0.0,
            macd_dea: 0.0,
            macd_bar: 0.0,
            macd_signal: "无数据".to_string(),
            rsi6: 50.0,
            rsi12: 50.0,
            rsi24: 50.0,
            rsi_signal: "无数据".to_string(),
            boll_upper: 0.0,
            boll_mid: 0.0,
            boll_lower: 0.0,
            boll_position: "无数据".to_string(),
            bias_ma5: 0.0,
            bias_ma20: 0.0,
            volume_ratio: 1.0,
            volume_signal: "无数据".to_string(),
            support_levels: vec![],
            resistance_levels: vec![],
        };
    }
    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
    let volumes: Vec<f64> = klines.iter().map(|k| k.volume).collect();
    let latest = klines.last();
    let latest_date = latest.map(|k| k.date.clone()).unwrap_or_default();
    let latest_close = latest.map(|k| k.close).unwrap_or(0.0);
    let latest_volume = latest.map(|k| k.volume).unwrap_or(0.0);
    let prev_close = klines
        .get(klines.len().saturating_sub(2))
        .map(|k| k.close)
        .unwrap_or(0.0);
    let price_change = latest_close - prev_close;

    // MA — 计算配置中所有周期，按 period 值映射到命名域
    let mut ma5 = latest_close;
    let mut ma10 = latest_close;
    let mut ma20 = latest_close;
    let mut ma60 = latest_close;
    for &period in &cfg.ma_periods {
        let val = sma(&closes, period).unwrap_or(latest_close);
        match period {
            5 => ma5 = val,
            10 => ma10 = val,
            20 => ma20 = val,
            60 => ma60 = val,
            _ => {},
        }
    }

    // MA alignment
    let ma_alignment = if ma5 > ma10 && ma10 > ma20 && ma20 > ma60 {
        "多头排列".to_string()
    } else if ma5 < ma10 && ma10 < ma20 && ma20 < ma60 {
        "空头排列".to_string()
    } else if ma5 > ma10 && ma10 > ma20 {
        "弱多头".to_string()
    } else {
        "缠绕/交叉".to_string()
    };

    // MACD: 计算完整 DIF 序列后再做 EMA(signal) 得到 DEA
    let dif_series: Vec<f64> = if closes.len() >= cfg.macd_slow {
        let ema_fast_series = build_ema_series(&closes, cfg.macd_fast);
        let ema_slow_series = build_ema_series(&closes, cfg.macd_slow);
        ema_fast_series
            .iter()
            .zip(ema_slow_series.iter())
            .map(|(&e_fast, &e_slow)| e_fast - e_slow)
            .collect()
    } else {
        vec![0.0]
    };
    let dea_series = build_ema_series(&dif_series, cfg.macd_signal);
    let dif = dif_series.last().copied().unwrap_or(0.0);
    let prev_dif = if dif_series.len() >= 2 {
        dif_series[dif_series.len() - 2]
    } else {
        dif
    };
    let dea = dea_series.last().copied().unwrap_or(0.0);
    let prev_dea = if dea_series.len() >= 2 {
        dea_series[dea_series.len() - 2]
    } else {
        dea
    };
    let bar = (dif - dea) * 2.0;

    // MACD signal
    let macd_signal = if prev_dif <= prev_dea && dif > dea {
        "金叉".to_string()
    } else if prev_dif >= prev_dea && dif < dea {
        "死叉".to_string()
    } else if dif > dea {
        "多头运行".to_string()
    } else if dif < dea {
        "空头运行".to_string()
    } else {
        "缠绕".to_string()
    };

    // RSI — 计算配置中所有周期，按 period 值映射到命名域
    let mut rsi6 = 50.0;
    let mut rsi12 = 50.0;
    let mut rsi24 = 50.0;
    for &period in &cfg.rsi_periods {
        let val = rsi(&closes, period);
        match period {
            6 => rsi6 = val,
            12 => rsi12 = val,
            24 => rsi24 = val,
            _ => {},
        }
    }

    let rsi_signal = if rsi6 > cfg.rsi_overbought {
        "超买".to_string()
    } else if rsi6 < cfg.rsi_oversold {
        "超卖".to_string()
    } else if rsi6 > 60.0 {
        "强势".to_string()
    } else if rsi6 < 40.0 {
        "弱势".to_string()
    } else {
        "中性".to_string()
    };

    // Bollinger Bands — 取最近 boll_period 根K线计算
    let boll_mid = sma(&closes, cfg.boll_period).unwrap_or(latest_close);
    let boll_std = if closes.len() >= cfg.boll_period {
        stddev(&closes[closes.len() - cfg.boll_period..], boll_mid)
    } else if !closes.is_empty() {
        stddev(&closes, boll_mid)
    } else {
        0.0
    };
    let boll_upper = boll_mid + cfg.boll_stddev * boll_std;
    let boll_lower = boll_mid - cfg.boll_stddev * boll_std;

    let half_std = boll_std * 0.5;
    let boll_position = if latest_close > boll_upper {
        "上轨以上".to_string()
    } else if latest_close > boll_mid + half_std {
        "上轨区间".to_string()
    } else if latest_close >= boll_mid - half_std {
        "中轨附近".to_string()
    } else if latest_close > boll_lower {
        "下轨区间".to_string()
    } else {
        "下轨以下".to_string()
    };

    // Bias (deviation rate)
    let bias_ma5 = if ma5 > 0.0 {
        ((latest_close - ma5) / ma5) * 100.0
    } else {
        0.0
    };
    let bias_ma20 = if ma20 > 0.0 {
        ((latest_close - ma20) / ma20) * 100.0
    } else {
        0.0
    };

    // Volume ratio — 取最近 volume_lookback 日均量
    let avg_vol = if volumes.len() > cfg.volume_lookback {
        volumes[volumes.len() - cfg.volume_lookback - 1..volumes.len() - 1]
            .iter()
            .sum::<f64>()
            / cfg.volume_lookback as f64
    } else if volumes.len() >= 2 {
        volumes[..volumes.len() - 1].iter().sum::<f64>() / (volumes.len() - 1) as f64
    } else {
        latest_volume
    };
    let volume_ratio = if avg_vol > 0.0 {
        latest_volume / avg_vol
    } else {
        1.0
    };

    let volume_signal = if volume_ratio > cfg.volume_surge_ratio && price_change > 0.0 {
        "放量上涨".to_string()
    } else if volume_ratio < cfg.volume_shrink_ratio && price_change < 0.0 {
        "缩量回调".to_string()
    } else if volume_ratio > cfg.volume_surge_ratio && price_change < 0.0 {
        "放量下跌".to_string()
    } else if volume_ratio < cfg.volume_shrink_ratio && price_change > 0.0 {
        "缩量上涨".to_string()
    } else {
        "正常".to_string()
    };

    // Support/Resistance from MAs and Bollinger
    let mut support_levels = vec![ma5.min(ma10).min(ma20), ma20.min(ma60)];
    support_levels.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    support_levels.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    let mut resistance_levels = vec![ma5.max(ma10).max(ma20), boll_upper];
    resistance_levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    resistance_levels.dedup_by(|a, b| (*a - *b).abs() < 0.01);

    TechnicalIndicators {
        stock_code: stock_code.to_string(),
        latest_date,
        ma5,
        ma10,
        ma20,
        ma60,
        ma_alignment,
        macd_dif: dif,
        macd_dea: dea,
        macd_bar: bar,
        macd_signal,
        rsi6,
        rsi12,
        rsi24,
        rsi_signal,
        boll_upper,
        boll_mid,
        boll_lower,
        boll_position,
        bias_ma5,
        bias_ma20,
        volume_ratio,
        volume_signal,
        support_levels,
        resistance_levels,
    }
}

/// Compute all technical indicators from K-line data (使用默认参数)
pub fn compute_indicators(stock_code: &str, klines: &[KLine]) -> TechnicalIndicators {
    compute_indicators_with_config(stock_code, klines, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_kline(date: &str, open: f64, high: f64, low: f64, close: f64, volume: f64) -> KLine {
        KLine {
            date: date.to_string(),
            open,
            high,
            low,
            close,
            volume,
            amount: volume * close,
            turnover_rate: None,
            adj_factor: None,
        }
    }

    #[test]
    fn test_sma_basic() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert!((sma(&data, 5).unwrap() - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_sma_takes_latest() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
        let result = sma(&data, 3).unwrap();
        assert!((result - 50.0).abs() < 1e-6, "SMA(3) of last 3 should be 50.0, got {result}");
    }

    #[test]
    fn test_sma_insufficient_data() {
        let data = vec![10.0, 20.0];
        assert!(sma(&data, 5).is_none());
    }

    #[test]
    fn test_ema_non_empty() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = ema(&data, 5);
        assert!(result > 0.0);
    }

    #[test]
    fn test_rsi_uniform() {
        let closes = vec![10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0];
        let result = rsi(&closes, 6);
        // 零波动时 avg_loss=0，RSI 为 100
        assert!((result - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_rsi_all_gains() {
        let closes: Vec<f64> = (0..8).map(|i| i as f64 * 10.0).collect();
        let result = rsi(&closes, 6);
        assert!(result > 80.0);
    }

    #[test]
    fn test_stddev_calculation() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let sd = stddev(&data, mean);
        assert!(sd > 0.0);
    }

    #[test]
    fn test_compute_indicators_empty() {
        let result = compute_indicators("000001", &[]);
        assert_eq!(result.stock_code, "000001");
        assert_eq!(result.ma_alignment, "无数据");
        assert_eq!(result.macd_signal, "无数据");
    }

    #[test]
    fn test_compute_indicators_basic() {
        let klines: Vec<KLine> = (0..65)
            .map(|i| {
                make_kline(
                    &format!("2025-01-{:02}", i + 1),
                    10.0,
                    10.5,
                    9.5,
                    10.0 + i as f64 * 0.1,
                    10000.0,
                )
            })
            .collect();
        let result = compute_indicators("000001", &klines);
        assert!(result.ma5 > 0.0);
        assert!(result.ma10 > 0.0);
        assert!(result.ma20 > 0.0);
        assert!(!result.macd_signal.is_empty());
        assert!(result.rsi6 >= 0.0 && result.rsi6 <= 100.0);
    }

    #[test]
    fn test_compute_indicators_ma_alignment() {
        // SMA 取最近 N 个元素，递增价格 → 最新价格高 → MA5 > MA10 > MA20 > MA60 为多头排列
        let klines: Vec<KLine> = (0..65)
            .map(|i| {
                let price = 10.0 + i as f64 * 0.5;
                make_kline(
                    &format!("2025-01-{:02}", i + 1),
                    price,
                    price + 1.0,
                    price - 1.0,
                    price,
                    10000.0,
                )
            })
            .collect();
        let result = compute_indicators("000001", &klines);
        assert!(
            result.ma_alignment == "多头排列" || result.ma_alignment == "弱多头",
            "Expected bull alignment, got: {}",
            result.ma_alignment
        );
    }

    #[test]
    fn test_macd_dea_not_equal_dif() {
        let klines: Vec<KLine> = (0..65)
            .map(|i| {
                let price = 10.0 + (i as f64 * 0.3).sin() * 2.0;
                make_kline(
                    &format!("2025-01-{:02}", i + 1),
                    price,
                    price + 0.5,
                    price - 0.5,
                    price,
                    10000.0,
                )
            })
            .collect();
        let result = compute_indicators("000001", &klines);
        assert!(
            (result.macd_dea - result.macd_dif).abs() > 0.001 || result.macd_bar.abs() > 0.001,
            "DEA should differ from DIF with sufficient data"
        );
    }

    #[test]
    fn test_compute_indicators_with_config_custom() {
        let mut config = IndicatorConfig::default();
        config.ma_periods = vec![5, 10];
        config.rsi_periods = vec![6, 12];
        let klines: Vec<KLine> = (0..65)
            .map(|i| {
                make_kline(
                    &format!("2025-01-{:02}", i + 1),
                    10.0,
                    10.5,
                    9.5,
                    10.0 + i as f64 * 0.1,
                    10000.0,
                )
            })
            .collect();
        let result = compute_indicators_with_config("000001", &klines, Some(&config));
        assert!(result.ma5 > 0.0);
        assert!(result.ma10 > 0.0);
        assert!(!result.macd_signal.is_empty());
        assert!(result.rsi6 >= 0.0 && result.rsi6 <= 100.0);
    }
}
