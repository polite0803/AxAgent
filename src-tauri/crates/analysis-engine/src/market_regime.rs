//! 市场状态研判 — 基于沪深300近60日K线输出牛/熊/震荡 + 波动率
//!
//! 无需额外数据源，纯用已有 `get_klines("000300", "daily", 60)` 数据判断。
//! 不引入新依赖，纯函数模块。

use crate::types::{MarketRegimeType, VolatilityLevel};
use axagent_astock_data::KLine;

/// 市场状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRegime {
    pub regime: MarketRegimeType,
    /// 置信度 0-1
    pub confidence: f64,
    pub volatility: VolatilityLevel,
    /// 可读描述
    pub description: String,
}

/// 用沪深300近60日 K 线判断市场状态
///
/// 规则（纯技术面，仅用近60日数据，无需额外数据）:
/// - 价格站上 MA60 且 MA20 多头排列 + 向上斜率 → 牛市
/// - 价格跌破 MA60 且 MA20 空头排列 + 向下斜率 → 熊市
/// - 其余 → 震荡
/// - 布林带宽度（20日收盘价标准差 / MA20）> 20% → 高波动，< 10% → 低波动
///
/// 限制：仅取近60日K线，不计算 MA120/MA250，牛熊判定基于价格相对均线的位置与斜率。
pub fn classify_regime(klines: &[KLine]) -> MarketRegime {
    if klines.len() < 20 {
        return MarketRegime {
            regime: MarketRegimeType::Sideways,
            confidence: 0.3,
            volatility: VolatilityLevel::Low,
            description: format!("数据不足（仅{}日），默认震荡", klines.len()),
        };
    }

    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();

    // 均线计算
    let ma20 = simple_moving_average(&closes, 20);
    let ma60_is_real = closes.len() >= 60;
    let ma60 = if ma60_is_real {
        simple_moving_average(&closes, 60)
    } else {
        ma20
    };
    // len<60 时 ma60 实为 ma20，描述文案需如实反映，避免误标"60日均线"
    let ma60_label = if ma60_is_real {
        "60日均线"
    } else {
        "短期均线"
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
        let prev_5: f64 = closes[closes.len() - 10..closes.len() - 5].iter().sum::<f64>() / 5.0;
        (recent_5 - prev_5) / prev_5
    } else {
        0.0
    };

    // 布林带宽度（波动率）：20日收盘价标准差 / ma20
    let (bollinger_pct, vol_level) = if closes.len() >= 20 {
        let variance =
            closes[closes.len() - 20..].iter().map(|c| (c - ma20).powi(2)).sum::<f64>() / 20.0;
        let std_dev = variance.sqrt();
        let bbp = if ma20 > 0.0 { std_dev / ma20 } else { 0.0 };
        let vl = VolatilityLevel::from_bollinger_pct(bbp);
        (bbp, vl)
    } else {
        (0.0, VolatilityLevel::Normal)
    };

    // 决策逻辑
    let (regime, confidence, desc) =
        if price_above_ma60 > 0.05 && price_above_ma20 > 0.02 && slope > 0.01 {
            // 价格在 MA60 上方 5% + MA20 上方 2% + 向上斜率
            let c = (price_above_ma60 * 2.0).clamp(0.5, 0.95);
            let vol_note = if bollinger_pct > 0.20 {
                "（高波动预警）"
            } else {
                ""
            };
            (
                MarketRegimeType::Bull,
                c,
                format!(
                    "沪深300站上{ma60_label}{:.1}%，短期均线多头排列{}",
                    price_above_ma60 * 100.0,
                    vol_note
                ),
            )
        } else if price_above_ma60 < -0.03 && price_above_ma20 < -0.01 && slope < -0.005 {
            // 价格在 MA60 下方 3% + MA20 下方 + 向下斜率
            let c = (price_above_ma60.abs() * 2.0).clamp(0.5, 0.95);
            (
                MarketRegimeType::Bear,
                c,
                format!(
                    "沪深300跌破{ma60_label}{:.1}%，短期均线空头排列",
                    price_above_ma60.abs() * 100.0
                ),
            )
        } else {
            // 不满足牛/熊条件 → 震荡
            let c = 0.5 + (slope.abs() * 3.0).min(0.3); // 斜率越大信心越高
            (
                MarketRegimeType::Sideways,
                c.min(0.8),
                format!("均线交叉/粘合，方向不明确（斜率{:.2}%）", slope * 100.0),
            )
        };

    MarketRegime { regime, confidence, volatility: vol_level, description: desc }
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
        assert_eq!(r.regime, MarketRegimeType::Bull, "持续上涨应识别为牛市: {}", r.description);
        assert!(r.confidence >= 0.5);
    }

    #[test]
    fn test_bear_market() {
        // 持续下跌 → 熊市
        let closes: Vec<f64> = (1..=60).map(|i| 4000.0 - i as f64 * 15.0).collect();
        let r = classify_regime(&make_klines(&closes));
        assert_eq!(r.regime, MarketRegimeType::Bear, "持续下跌应识别为熊市: {}", r.description);
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
        assert_eq!(r.regime, MarketRegimeType::Sideways, "窄幅震荡应识别为震荡: {}", r.description);
    }

    #[test]
    fn test_insufficient_data() {
        let closes = vec![3500.0; 10];
        let r = classify_regime(&make_klines(&closes));
        assert_eq!(r.regime, MarketRegimeType::Sideways);
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
        assert_eq!(
            r.volatility,
            VolatilityLevel::High,
            "大幅震荡应识别为高波动, got={:?}",
            r.volatility
        );
    }
}
