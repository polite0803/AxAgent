//! 市场状态研判 — 基于沪深300近60日K线输出牛/熊/震荡 + 波动率
//!
//! 无需额外数据源，纯用已有 `get_klines("000300", "daily", 60)` 数据判断。
//! 不引入新依赖，纯函数模块。

use axagent_astock_data::KLine;

/// 市场状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRegime {
    /// "bull" / "bear" / "sideways"
    pub regime: String,
    /// 置信度 0-1
    pub confidence: f64,
    /// "high" / "low"
    pub volatility: String,
    /// 可读描述
    pub description: String,
}

/// 用沪深300近60日 K 线判断市场状态
///
/// 规则（纯技术面，无需额外数据）:
/// - 60日均线 > 120日均线 → 潜在牛市（进一步看60 > 250）
/// - 60日均线 < 120日均线 → 潜在熊市
/// - 60日均线在120日均线附近交叉 → 震荡
/// - 布林带宽度 > 20% → 高波动
/// - 布林带宽度 < 10% → 低波动
pub fn classify_regime(klines: &[KLine]) -> MarketRegime {
    if klines.len() < 20 {
        return MarketRegime {
            regime: "sideways".into(),
            confidence: 0.3,
            volatility: "low".into(),
            description: format!("数据不足（仅{}日），默认震荡", klines.len()),
        };
    }

    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();

    // 均线计算
    let ma20 = simple_moving_average(&closes, 20);
    let ma60 = if closes.len() >= 60 {
        simple_moving_average(&closes, 60)
    } else {
        ma20
    };

    // 当前价格相对均线的位置
    let current_price = closes.last().copied().unwrap_or(0.0);
    let price_above_ma20 = if ma20 > 0.0 {
        (current_price - ma20) / ma20
    } else {
        0.0
    };
    let price_above_ma60 = if ma60 > 0.0 {
        (current_price - ma60) / ma60
    } else {
        0.0
    };

    // 最近 N 日斜率（用线性回归简化：最后5日 vs 之前5日）
    let slope = if closes.len() >= 10 {
        let recent_5: f64 = closes[closes.len() - 5..].iter().sum::<f64>() / 5.0;
        let prev_5: f64 = closes[closes.len() - 10..closes.len() - 5]
            .iter()
            .sum::<f64>()
            / 5.0;
        (recent_5 - prev_5) / prev_5
    } else {
        0.0
    };

    // 布林带宽度（波动率）：20日收盘价标准差 / ma20
    let (bollinger_pct, vol_str) = if closes.len() >= 20 {
        let variance = closes[closes.len() - 20..]
            .iter()
            .map(|c| (c - ma20).powi(2))
            .sum::<f64>()
            / 20.0;
        let std_dev = variance.sqrt();
        let bbp = if ma20 > 0.0 { std_dev / ma20 } else { 0.0 };
        let vs = if bbp > 0.20 {
            "high"
        } else if bbp < 0.10 {
            "low"
        } else {
            "normal"
        };
        (bbp, vs.to_string())
    } else {
        (0.0, "normal".to_string())
    };

    // 决策逻辑
    let (regime, confidence, desc) = if price_above_ma60 > 0.05
        && price_above_ma20 > 0.02
        && slope > 0.01
    {
        // 价格在 MA60 上方 5% + MA20 上方 2% + 向上斜率
        let c = (price_above_ma60 * 2.0).clamp(0.5, 0.95);
        let vol_note = if bollinger_pct > 0.20 {
            "（高波动预警）"
        } else {
            ""
        };
        (
            "bull".to_string(),
            c,
            format!(
                "沪深300站上60日均线{:.1}%，短期均线多头排列{}",
                price_above_ma60 * 100.0,
                vol_note
            ),
        )
    } else if price_above_ma60 < -0.03 && price_above_ma20 < -0.01 && slope < -0.005 {
        // 价格在 MA60 下方 3% + MA20 下方 + 向下斜率
        let c = (price_above_ma60.abs() * 2.0).clamp(0.5, 0.95);
        (
            "bear".to_string(),
            c,
            format!("沪深300跌破60日均线{:.1}%，短期均线空头排列", price_above_ma60.abs() * 100.0),
        )
    } else {
        // 不满足牛/熊条件 → 震荡
        let c = 0.5 + (slope.abs() * 3.0).min(0.3); // 斜率越大信心越高
        (
            "sideways".to_string(),
            c.min(0.8),
            format!("均线交叉/粘合，方向不明确（斜率{:.2}%）", slope * 100.0),
        )
    };

    MarketRegime {
        regime,
        confidence,
        volatility: vol_str,
        description: desc,
    }
}

/// 简单移动平均
fn simple_moving_average(data: &[f64], window: usize) -> f64 {
    if data.len() < window || window == 0 {
        return 0.0;
    }
    data[data.len() - window..].iter().sum::<f64>() / window as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_klines(closes: &[f64]) -> Vec<KLine> {
        let mut klines = Vec::new();
        for (i, &c) in closes.iter().enumerate() {
            klines.push(KLine {
                date: format!("2026-01-{:02}", i + 1),
                open: c * 0.99,
                high: c * 1.02,
                low: c * 0.98,
                close: c,
                volume: 100_000.0,
                amount: c * 100_000.0 * 100.0,
                turnover_rate: None,
                adj_factor: None,
            });
        }
        klines
    }

    #[test]
    fn test_bull_market() {
        // 持续上涨 → 牛市
        let closes: Vec<f64> = (1..=60).map(|i| 3000.0 + i as f64 * 15.0).collect();
        let r = classify_regime(&make_klines(&closes));
        assert_eq!(r.regime, "bull", "持续上涨应识别为牛市: {}", r.description);
        assert!(r.confidence >= 0.5);
    }

    #[test]
    fn test_bear_market() {
        // 持续下跌 → 熊市
        let closes: Vec<f64> = (1..=60).map(|i| 4000.0 - i as f64 * 15.0).collect();
        let r = classify_regime(&make_klines(&closes));
        assert_eq!(r.regime, "bear", "持续下跌应识别为熊市: {}", r.description);
        assert!(r.confidence >= 0.5);
    }

    #[test]
    fn test_sideways_market() {
        // 窄幅震荡 → 震荡
        let mut closes = vec![3500.0; 60];
        for i in 1..60 {
            closes[i] = closes[i - 1] + (i as f64 % 5.0 - 2.0);
        }
        let r = classify_regime(&make_klines(&closes));
        assert_eq!(r.regime, "sideways", "窄幅震荡应识别为震荡: {}", r.description);
    }

    #[test]
    fn test_insufficient_data() {
        let closes = vec![3500.0; 10];
        let r = classify_regime(&make_klines(&closes));
        assert_eq!(r.regime, "sideways");
        assert!(r.confidence <= 0.3);
    }

    #[test]
    fn test_volatility_detection() {
        // 高波动：大幅震荡，振幅 > 20%
        let mut closes = Vec::new();
        for i in 0..60 {
            let cycle = (i as f64 * 0.5).sin(); // -1 ~ +1
            closes.push(3500.0 + cycle * 1200.0); // ±1200, ~34% 振幅
        }
        let r = classify_regime(&make_klines(&closes));
        assert_eq!(r.volatility, "high", "大幅震荡应识别为高波动, got={}", r.volatility);
    }
}
