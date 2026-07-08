//! 技术指标辅助函数

use axagent_astock_data::KLine;

/// 简单移动平均
pub fn sma(prices: &[f64], period: usize) -> Option<f64> {
    if period == 0 || prices.len() < period {
        return None;
    }
    let n = prices.len();
    Some(prices[n - period..].iter().sum::<f64>() / period as f64)
}

/// K线收盘价序列
pub fn closes(klines: &[KLine]) -> Vec<f64> {
    klines.iter().map(|k| k.close).collect()
}

/// K线成交量序列
pub fn volumes(klines: &[KLine]) -> Vec<f64> {
    klines.iter().map(|k| k.volume).collect()
}

/// RSI(period) — 经典 Wilder 平滑
pub fn rsi(klines: &[KLine], period: usize) -> Option<f64> {
    if klines.len() < period + 1 {
        return None;
    }
    let cs = closes(klines);
    let mut gains = 0.0;
    let mut losses = 0.0;
    for i in 1..=period {
        let diff = cs[i] - cs[i - 1];
        if diff > 0.0 {
            gains += diff;
        } else {
            losses += -diff;
        }
    }
    let mut avg_gain = gains / period as f64;
    let mut avg_loss = losses / period as f64;
    for i in (period + 1)..cs.len() {
        let diff = cs[i] - cs[i - 1];
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { -diff } else { 0.0 };
        avg_gain = (avg_gain * (period - 1) as f64 + gain) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + loss) / period as f64;
    }
    if avg_loss > 1e-10 {
        let rs = avg_gain / avg_loss;
        Some(100.0 - 100.0 / (1.0 + rs))
    } else {
        Some(100.0)
    }
}

/// MACD 柱状值（最新一根）
/// 返回 (dif, dea, macd)
pub fn macd(klines: &[KLine], fast: usize, slow: usize, signal: usize) -> Option<(f64, f64, f64)> {
    let cs = closes(klines);
    if cs.len() < slow + signal {
        return None;
    }
    let emas_fast = ema_series(&cs, fast);
    let emas_slow = ema_series(&cs, slow);
    let difs: Vec<f64> = emas_fast
        .iter()
        .zip(emas_slow.iter())
        .map(|(a, b)| a - b)
        .collect();
    let deas = ema_series(&difs, signal);
    let dif = *difs.last()?;
    let dea = *deas.last()?;
    Some((dif, dea, (dif - dea) * 2.0))
}

fn ema_series(prices: &[f64], period: usize) -> Vec<f64> {
    if prices.is_empty() || period == 0 {
        return vec![];
    }
    let k = 2.0 / (period as f64 + 1.0);
    let mut emas = Vec::with_capacity(prices.len());
    // 第一个 EMA 用 SMA 初始化
    let init_n = period.min(prices.len());
    let init_sma: f64 = prices[..init_n].iter().sum::<f64>() / init_n as f64;
    emas.push(init_sma);
    for item in prices.iter().skip(1) {
        let prev = *emas.last().unwrap();
        let cur = item * k + prev * (1.0 - k);
        emas.push(cur);
    }
    emas
}

/// 最近 N 日最高 / 最低
pub fn highest(klines: &[KLine], n: usize) -> Option<f64> {
    klines
        .iter()
        .rev()
        .take(n)
        .map(|k| k.high)
        .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.max(v))))
}

pub fn lowest(klines: &[KLine], n: usize) -> Option<f64> {
    klines
        .iter()
        .rev()
        .take(n)
        .map(|k| k.low)
        .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.min(v))))
}

/// 距 N 日新高回撤（%）
pub fn drawdown_from_high(klines: &[KLine], n: usize) -> Option<f64> {
    let h = highest(klines, n)?;
    let last = klines.last()?.close;
    Some((h - last) / h * 100.0)
}

/// N 日均成交额（元）
pub fn avg_amount_n(klines: &[KLine], n: usize) -> Option<f64> {
    if klines.is_empty() {
        return None;
    }
    let take = n.min(klines.len());
    let sum: f64 = klines.iter().rev().take(take).map(|k| k.amount).sum();
    Some(sum / take as f64)
}

/// 20 日均成交额
pub fn avg_amount_20d(klines: &[KLine]) -> Option<f64> {
    avg_amount_n(klines, 20)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    fn make_kline(close: f64) -> KLine {
        KLine {
            date: Local::now().format("%Y-%m-%d").to_string(),
            open: close,
            high: close * 1.01,
            low: close * 0.99,
            close,
            volume: 1_000_000.0,
            amount: 10_000_000.0,
            turnover_rate: Some(1.0),
            adj_factor: None,
        }
    }

    #[test]
    fn sma_basic() {
        let ps = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(sma(&ps, 3), Some(4.0));
        assert_eq!(sma(&ps, 5), Some(3.0));
        assert_eq!(sma(&ps, 6), None);
    }

    #[test]
    fn rsi_all_up_is_100() {
        let klines: Vec<KLine> = (1..=30).map(|i| make_kline(i as f64)).collect();
        let r = rsi(&klines, 6).unwrap();
        assert!((r - 100.0).abs() < 0.1, "got {}", r);
    }

    #[test]
    fn rsi_all_down_is_0() {
        let klines: Vec<KLine> = (1..=30).rev().map(|i| make_kline(i as f64)).collect();
        let r = rsi(&klines, 6).unwrap();
        assert!(r < 0.1, "got {}", r);
    }

    #[test]
    fn drawdown_calc() {
        let klines: Vec<KLine> = vec![
            make_kline(100.0),
            make_kline(120.0),
            make_kline(110.0),
            make_kline(90.0),
        ];
        // make_kline 把 high 设为 close*1.01，所以 4 根内最高 = 120*1.01 = 121.2
        // 当前 90 → 回撤 (121.2 - 90) / 121.2 ≈ 25.74%
        let dd = drawdown_from_high(&klines, 4).unwrap();
        assert!((dd - 25.74).abs() < 0.1, "got {}", dd);
    }
}
