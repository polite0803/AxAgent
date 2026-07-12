//! 100 分制客观评分引擎（从 stock-analysis crate 下沉到 astock-data，P1-1）
//!
//! 基于技术指标（趋势/乖离率/MACD/量能/RSI/支撑/布林带）计算客观评分。
//! 评分范围 0-100，信号分类从 "强烈买入" 到 "强烈卖出"。
//!
//! 原于 stock-analysis/src/scoring.rs，为供 tools crate（hybrid）直接复用而下沉。

use serde::{Deserialize, Serialize};

use crate::indicators::TechnicalIndicators;

/// 评分权重
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub trend: f64,
    pub deviation: f64,
    pub macd: f64,
    pub volume: f64,
    pub rsi: f64,
    pub support: f64,
    pub boll: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            trend: 30.0,
            deviation: 20.0,
            macd: 15.0,
            volume: 15.0,
            rsi: 10.0,
            support: 10.0,
            boll: 5.0,
        }
    }
}

/// 100分制客观评分
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveScore {
    pub total: u32,
    pub trend_score: u32,
    pub deviation_score: u32,
    pub macd_score: u32,
    pub volume_score: u32,
    pub rsi_score: u32,
    pub support_score: u32,
    pub boll_score: u32,
    #[serde(rename = "fundamentalAdjustment")]
    pub total_adjustment: i32,
    pub signal: String,
    pub signal_code: String,
}

/// 参数化评分分段阈值
#[derive(Debug, Clone)]
pub struct ScoreBands {
    pub deviation_band_1: f64,
    pub deviation_score_1: u32,
    pub deviation_band_2: f64,
    pub deviation_score_2: u32,
    pub deviation_band_3: f64,
    pub deviation_score_3: u32,
    pub deviation_band_4: f64,
    pub deviation_score_4: u32,
    pub deviation_band_5: f64,
    pub deviation_score_5: u32,
    pub rsi_oversold_deep: f64,
    pub rsi_oversold: f64,
    pub rsi_neutral_low: f64,
    pub rsi_neutral_high: f64,
    pub rsi_overbought: f64,
    pub rsi_overbought_high: f64,
    pub support_tolerance_pct: f64,
    pub boll_half_std_factor: f64,
}

impl Default for ScoreBands {
    fn default() -> Self {
        Self {
            deviation_band_1: 1.0,
            deviation_score_1: 20,
            deviation_band_2: 2.0,
            deviation_score_2: 18,
            deviation_band_3: 3.0,
            deviation_score_3: 15,
            deviation_band_4: 5.0,
            deviation_score_4: 10,
            deviation_band_5: 8.0,
            deviation_score_5: 5,
            rsi_oversold_deep: 20.0,
            rsi_oversold: 30.0,
            rsi_neutral_low: 40.0,
            rsi_neutral_high: 60.0,
            rsi_overbought: 70.0,
            rsi_overbought_high: 80.0,
            support_tolerance_pct: 0.03,
            boll_half_std_factor: 0.5,
        }
    }
}

/// 100分评分引擎
pub struct ScoringEngine;

impl ScoringEngine {
    /// 从技术指标计算客观评分
    pub fn score(
        indicators: &TechnicalIndicators,
        latest_price: f64,
        weights: Option<&ScoringWeights>,
    ) -> ObjectiveScore {
        Self::score_with_bands(indicators, latest_price, weights, &ScoreBands::default())
    }

    /// 从技术指标计算客观评分（可传入自定义权重和分段参数）
    pub fn score_with_bands(
        indicators: &TechnicalIndicators,
        latest_price: f64,
        weights: Option<&ScoringWeights>,
        bands: &ScoreBands,
    ) -> ObjectiveScore {
        let default_weights = ScoringWeights::default();
        let w = weights.unwrap_or(&default_weights);

        let trend = (Self::score_trend(&indicators.ma_alignment) as f64 * w.trend / 30.0) as u32;
        let deviation =
            (Self::score_deviation(indicators.bias_ma5, bands) as f64 * w.deviation / 20.0) as u32;
        let macd = (Self::score_macd(&indicators.macd_signal, indicators.macd_dif) as f64 * w.macd
            / 15.0) as u32;
        let volume =
            (Self::score_volume(&indicators.volume_signal) as f64 * w.volume / 15.0) as u32;
        let rsi = (Self::score_rsi(indicators.rsi6, bands) as f64 * w.rsi / 10.0) as u32;
        let support = (Self::score_support(latest_price, &indicators.support_levels, bands) as f64
            * w.support
            / 5.0) as u32;
        let boll =
            (Self::score_boll(&indicators.boll_position, bands) as f64 * w.boll / 5.0) as u32;
        let total = (trend + deviation + macd + volume + rsi + support + boll).min(100);

        let (signal, signal_code) = Self::map_signal(total, &indicators.ma_alignment);

        ObjectiveScore {
            total,
            trend_score: trend,
            deviation_score: deviation,
            macd_score: macd,
            volume_score: volume,
            rsi_score: rsi,
            support_score: support,
            boll_score: boll,
            total_adjustment: 0,
            signal: signal.to_string(),
            signal_code: signal_code.to_string(),
        }
    }

    /// 基本面调整：根据 PE / PB / ROE 对客观评分做增量调整
    pub fn apply_fundamental_adjustment(
        score: &mut ObjectiveScore,
        pe: f64,
        pb: f64,
        roe: Option<f64>,
    ) {
        let mut adj: i32 = 0;
        if pe > 0.0 && pe < 15.0 {
            adj += 5;
        } else if pe > 50.0 {
            adj -= 5;
        }
        if pb > 0.0 && pb < 1.5 {
            adj += 3;
        } else if pb > 5.0 {
            adj -= 3;
        }
        if let Some(r) = roe {
            if r > 15.0 {
                adj += 5;
            } else if r < 5.0 {
                adj -= 3;
            }
        }
        score.total_adjustment += adj;
        score.total = (score.total as i32 + adj).clamp(0, 100) as u32;
    }

    /// 行业相对估值调整：个股 PE/PB 相对行业中位数的偏离
    pub fn apply_industry_adjustment(
        score: &mut ObjectiveScore,
        pe: f64,
        industry_pe: Option<f64>,
        pb: f64,
        industry_pb: Option<f64>,
    ) {
        let mut adj: i32 = 0;
        if let Some(ind_pe) = industry_pe {
            if pe > 0.0 && ind_pe > 0.0 {
                if pe < ind_pe * 0.8 {
                    adj += 4;
                } else if pe > ind_pe * 1.2 {
                    adj -= 4;
                }
            }
        }
        if let Some(ind_pb) = industry_pb {
            if pb > 0.0 && ind_pb > 0.0 {
                if pb < ind_pb * 0.8 {
                    adj += 3;
                } else if pb > ind_pb * 1.2 {
                    adj -= 3;
                }
            }
        }
        score.total_adjustment += adj;
        score.total = (score.total as i32 + adj).clamp(0, 100) as u32;
    }

    fn score_trend(alignment: &str) -> u32 {
        match alignment {
            "多头排列" => 30,
            "弱多头" => 20,
            "缠绕/交叉" => 12,
            "空头排列" => 0,
            _ => 12,
        }
    }

    fn score_deviation(bias_ma5: f64, bands: &ScoreBands) -> u32 {
        let abs_bias = bias_ma5.abs();
        if bias_ma5 > 0.0 && abs_bias < bands.deviation_band_1 {
            bands.deviation_score_1
        } else if abs_bias < bands.deviation_band_2 {
            bands.deviation_score_2
        } else if abs_bias < bands.deviation_band_3 {
            bands.deviation_score_3
        } else if abs_bias < bands.deviation_band_4 {
            bands.deviation_score_4
        } else {
            bands.deviation_score_5
        }
    }

    fn score_macd(signal: &str, macd_dif: f64) -> u32 {
        match signal {
            "金叉" if macd_dif > 0.0 => 20,
            "金叉" => 15,
            "多头运行" if macd_dif > 0.0 => 15,
            "多头运行" => 12,
            "死叉" if macd_dif < 0.0 => 3,
            "死叉" => 5,
            "空头运行" if macd_dif < 0.0 => 3,
            "空头运行" => 5,
            _ => 10,
        }
    }

    fn score_volume(signal: &str) -> u32 {
        match signal {
            "放量突破" => 20,
            "放量上涨" => 18,
            "缩量回调" => 12,
            "正常" => 10,
            "缩量上涨" => 8,
            "放量下跌" => 3,
            _ => 10,
        }
    }

    fn score_rsi(rsi: f64, bands: &ScoreBands) -> u32 {
        if rsi < bands.rsi_oversold_deep {
            15
        } else if rsi < bands.rsi_oversold {
            12
        } else if rsi < bands.rsi_neutral_low {
            8
        } else if rsi <= bands.rsi_neutral_high {
            5
        } else if rsi <= bands.rsi_overbought {
            3
        } else if rsi <= bands.rsi_overbought_high {
            2
        } else {
            0
        }
    }

    fn score_support(price: f64, supports: &[f64], _bands: &ScoreBands) -> u32 {
        if supports.is_empty() {
            return 3;
        }
        if price <= 0.0 {
            return 0;
        }
        let nearest = supports.iter().map(|s| (price - s).abs()).fold(f64::MAX, f64::min);
        if nearest < price * 0.02 {
            8
        } else if nearest < price * 0.05 {
            5
        } else {
            3
        }
    }

    fn score_boll(position: &str, _bands: &ScoreBands) -> u32 {
        match position {
            "下轨下方" => 8,
            "下轨附近" => 6,
            "中轨附近" => 4,
            "上轨附近" => 2,
            "上轨上方" => 0,
            _ => 4,
        }
    }

    fn map_signal(total: u32, alignment: &str) -> (&'static str, &'static str) {
        match total {
            85..=100 => match alignment {
                "多头排列" => ("🟢强烈买入", "strong_buy"),
                _ => ("🔵买入", "buy"),
            },
            70..=84 => ("🔵买入", "buy"),
            55..=69 => ("🟡持有", "hold"),
            40..=54 => match alignment {
                "空头排列" => ("🟠卖出", "sell"),
                _ => ("⚪观望", "watch"),
            },
            25..=39 => ("🟠卖出", "sell"),
            _ => ("🔴强烈卖出", "strong_sell"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_indicators(
        alignment: &str,
        bias: f64,
        macd_sig: &str,
        macd_dif: f64,
        vol_sig: &str,
        rsi: f64,
        boll_pos: &str,
    ) -> TechnicalIndicators {
        TechnicalIndicators {
            ma_alignment: alignment.into(),
            bias_ma5: bias,
            macd_signal: macd_sig.into(),
            macd_dif,
            volume_signal: vol_sig.into(),
            rsi6: rsi,
            boll_position: boll_pos.into(),
            support_levels: vec![100.0],
            resistance_levels: vec![200.0],
            ..Default::default()
        }
    }

    #[test]
    fn test_bull_market_scores_high() {
        let ind = make_indicators("多头排列", 0.5, "金叉", 0.5, "放量上涨", 55.0, "中轨附近");
        let score = ScoringEngine::score(&ind, 150.0, None);
        assert!(score.total >= 70, "牛市指标应获得高分, 实际={}", score.total);
        assert!(score.trend_score >= 25);
    }

    #[test]
    fn test_bear_market_scores_low() {
        let ind = make_indicators("空头排列", -8.0, "死叉", -0.3, "放量下跌", 15.0, "上轨上方");
        let score = ScoringEngine::score(&ind, 150.0, None);
        assert!(score.total < 40, "熊市指标应获得低分, 实际={}", score.total);
    }

    #[test]
    fn test_score_with_custom_weights() {
        let ind = make_indicators("多头排列", 0.5, "金叉", 0.5, "放量上涨", 55.0, "中轨附近");
        let weights = ScoringWeights { trend: 40.0, ..Default::default() };
        let score = ScoringEngine::score(&ind, 150.0, Some(&weights));
        assert!(score.total > 0 && score.total <= 100);
    }
}
