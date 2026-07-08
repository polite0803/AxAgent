//! 信号生成 — MA 金叉/死叉检测、突破/破位判断。
//!
//! 纯函数，接收 JSON 字符串输入，输出结构化信号结果。

use serde::{Deserialize, Serialize};

/// K 线最小结构（反序列化用）
#[derive(Debug, Clone, Deserialize)]
struct KLineRaw {
    #[serde(default)]
    close: f64,
    #[serde(default)]
    volume: f64,
}

// ── SMA ──

fn sma(prices: &[f64], period: usize) -> Option<f64> {
    if period == 0 || prices.len() < period {
        return None;
    }
    Some(prices[prices.len() - period..].iter().sum::<f64>() / period as f64)
}

// ── MA 交叉检测 ──

#[derive(Debug, Clone, Serialize)]
pub struct MACrossResult {
    pub signal: String, // "golden_cross" | "death_cross" | "none"
    pub fast_ma: f64,
    pub slow_ma: f64,
    pub prev_fast_ma: f64,
    pub prev_slow_ma: f64,
    pub latest_price: f64,
    pub confirmation: String,
}

/// 检测 MA 金叉/死叉。
/// klines_json: KLine JSON 数组字符串。
/// fast/slow: 快线和慢线的周期（如 5 和 20）。
pub fn detect_ma_cross(klines_json: &str, fast: usize, slow: usize) -> MACrossResult {
    let klines: Vec<KLineRaw> = serde_json::from_str(klines_json).unwrap_or_default();
    if klines.len() < slow + 1 {
        return MACrossResult {
            signal: "none".into(),
            fast_ma: 0.0,
            slow_ma: 0.0,
            prev_fast_ma: 0.0,
            prev_slow_ma: 0.0,
            latest_price: 0.0,
            confirmation: "n/a".into(),
        };
    }
    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
    let n = closes.len();
    let cur_fast = sma(&closes[..n], fast).unwrap_or(0.0);
    let cur_slow = sma(&closes[..n], slow).unwrap_or(0.0);
    let prev_fast = sma(&closes[..n - 1], fast).unwrap_or(cur_fast);
    let prev_slow = sma(&closes[..n - 1], slow).unwrap_or(cur_slow);

    let signal = if prev_fast <= prev_slow && cur_fast > cur_slow {
        "golden_cross"
    } else if prev_fast >= prev_slow && cur_fast < cur_slow {
        "death_cross"
    } else {
        "none"
    };

    let confirmation = if signal != "none" && closes.len() >= slow + 2 {
        let prev2_fast = sma(&closes[..n - 2], fast).unwrap_or(cur_fast);
        let prev2_slow = sma(&closes[..n - 2], slow).unwrap_or(cur_slow);
        if (signal == "golden_cross" && prev2_fast <= prev2_slow)
            || (signal == "death_cross" && prev2_fast >= prev2_slow)
        {
            "confirmed"
        } else {
            "unconfirmed"
        }
    } else if signal != "none" {
        "unconfirmed"
    } else {
        "n/a"
    };

    MACrossResult {
        signal: signal.into(),
        fast_ma: (cur_fast * 100.0).round() / 100.0,
        slow_ma: (cur_slow * 100.0).round() / 100.0,
        prev_fast_ma: (prev_fast * 100.0).round() / 100.0,
        prev_slow_ma: (prev_slow * 100.0).round() / 100.0,
        latest_price: klines.last().map(|k| k.close).unwrap_or(0.0),
        confirmation: confirmation.into(),
    }
}

// ── 突破/破位检测 ──

#[derive(Debug, Clone, Serialize)]
pub struct BreakoutResult {
    pub breakout_type: String, // "resistance_break" | "support_break" | "none"
    pub current_price: f64,
    pub support: f64,
    pub resistance: f64,
    pub volume_ratio: Option<f64>,
    pub confidence: String, // "high" | "medium" | "low"
    pub volume_confirmation: bool,
}

/// 检测价格是否突破支撑/阻力位（可配置成交量倍数）。
/// klines_json: KLine JSON 数组字符串（最近至少 5 根用于计算成交量均值）。
/// support/resistance: 支撑位和阻力位价格。
/// volume_mult: 成交量放大倍数阈值（用于判断高置信度）。
pub fn detect_breakout_with_volume_mult(
    klines_json: &str,
    support: f64,
    resistance: f64,
    volume_mult: f64,
) -> BreakoutResult {
    let klines: Vec<KLineRaw> = serde_json::from_str(klines_json).unwrap_or_default();
    if klines.is_empty() {
        return BreakoutResult {
            breakout_type: "none".into(),
            current_price: 0.0,
            support,
            resistance,
            volume_ratio: None,
            confidence: "low".into(),
            volume_confirmation: false,
        };
    }
    let last = klines.last().unwrap();
    let price = last.close;

    // 计算量比
    let avg_vol: f64 = if klines.len() >= 6 {
        klines[klines.len() - 6..klines.len() - 1]
            .iter()
            .map(|k| k.volume)
            .sum::<f64>()
            / 5.0
    } else if klines.len() >= 2 {
        klines[..klines.len() - 1]
            .iter()
            .map(|k| k.volume)
            .sum::<f64>()
            / (klines.len() - 1) as f64
    } else {
        klines[0].volume
    };
    let vol_ratio = if avg_vol > 0.0 {
        Some(last.volume / avg_vol)
    } else {
        None
    };

    let (breakout_type, confidence) = if price > resistance {
        let conf = if vol_ratio.unwrap_or(1.0) > volume_mult {
            "high"
        } else {
            "medium"
        };
        ("resistance_break", conf)
    } else if price < support {
        let conf = if vol_ratio.unwrap_or(1.0) > volume_mult {
            "high"
        } else {
            "medium"
        };
        ("support_break", conf)
    } else {
        ("none", "low")
    };

    BreakoutResult {
        breakout_type: breakout_type.into(),
        current_price: price,
        support,
        resistance,
        volume_ratio: vol_ratio.map(|v| (v * 100.0).round() / 100.0),
        confidence: confidence.into(),
        volume_confirmation: vol_ratio.unwrap_or(1.0) > volume_mult,
    }
}

/// 检测价格是否突破支撑/阻力位（默认成交量倍数 1.5）。
/// klines_json: KLine JSON 数组字符串（最近至少 5 根用于计算成交量均值）。
/// support/resistance: 支撑位和阻力位价格。
pub fn detect_breakout(klines_json: &str, support: f64, resistance: f64) -> BreakoutResult {
    detect_breakout_with_volume_mult(klines_json, support, resistance, 1.5)
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_klines() -> String {
        serde_json::to_string(&vec![
            serde_json::json!({"close": 10.0, "high": 10.5, "low": 9.5, "volume": 1000.0}),
            serde_json::json!({"close": 10.2, "high": 10.8, "low": 10.0, "volume": 1200.0}),
            serde_json::json!({"close": 10.1, "high": 10.3, "low": 9.8, "volume": 900.0}),
            serde_json::json!({"close": 10.5, "high": 11.0, "low": 10.2, "volume": 1500.0}),
            serde_json::json!({"close": 10.8, "high": 11.2, "low": 10.4, "volume": 2000.0}),
            serde_json::json!({"close": 10.6, "high": 10.9, "low": 10.3, "volume": 1100.0}),
            serde_json::json!({"close": 11.0, "high": 11.5, "low": 10.7, "volume": 2500.0}),
            serde_json::json!({"close": 10.9, "high": 11.2, "low": 10.5, "volume": 1300.0}),
            serde_json::json!({"close": 11.2, "high": 11.6, "low": 10.8, "volume": 1800.0}),
            serde_json::json!({"close": 11.5, "high": 12.0, "low": 11.3, "volume": 3000.0}),
        ])
        .unwrap()
    }

    #[test]
    fn test_ma_cross_golden() {
        let json = sample_klines();
        let r = detect_ma_cross(&json, 3, 7);
        assert!(r.fast_ma > 0.0);
        assert!(r.slow_ma > 0.0);
        assert!(!r.confirmation.is_empty());
    }

    #[test]
    fn test_ma_cross_empty() {
        let r = detect_ma_cross("[]", 5, 20);
        assert_eq!(r.signal, "none");
    }

    #[test]
    fn test_breakout_resistance() {
        let json = sample_klines();
        let r = detect_breakout(&json, 9.0, 11.0);
        assert_eq!(r.breakout_type, "resistance_break");
        assert!(r.volume_ratio.is_some());
    }

    #[test]
    fn test_breakout_support() {
        let json = sample_klines();
        let r = detect_breakout(&json, 12.0, 15.0);
        assert_eq!(r.breakout_type, "support_break");
    }

    #[test]
    fn test_breakout_none() {
        let json = sample_klines();
        let r = detect_breakout(&json, 5.0, 20.0);
        assert_eq!(r.breakout_type, "none");
    }
}
