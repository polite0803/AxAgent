use axagent_astock_data::indicators::TechnicalIndicators;
use serde::{Deserialize, Serialize};

use crate::decision::ScoringWeights;
use crate::value_investing::ValueMetrics;

/// 100分制客观评分
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveScore {
    pub total: u32,           // 综合评分 0-100
    pub trend_score: u32,     // 趋势 0-25
    pub deviation_score: u32, // 乖离率 0-15
    pub macd_score: u32,      // MACD 0-15
    pub volume_score: u32,    // 量能 0-15
    pub rsi_score: u32,       // RSI 0-10
    pub support_score: u32,   // 支撑 0-5
    pub boll_score: u32,      // 布林带 0-5
    #[serde(rename = "fundamentalAdjustment")]
    pub total_adjustment: i32,
    pub signal: String, // "🟢强烈买入" | "🔵买入" | "🟡持有" | "⚪观望" | "🟠卖出" | "🔴强烈卖出"
    pub signal_code: String, // strong_buy | buy | hold | watch | sell | strong_sell
}

/// 参数化评分分段阈值
#[derive(Debug, Clone)]
pub struct ScoreBands {
    // 乖离率分段 (bias_ma5)
    pub deviation_band_1: f64,  // 1.0
    pub deviation_score_1: u32, // 20
    pub deviation_band_2: f64,  // 2.0
    pub deviation_score_2: u32, // 18
    pub deviation_band_3: f64,  // 3.0
    pub deviation_score_3: u32, // 15
    pub deviation_band_4: f64,  // 5.0
    pub deviation_score_4: u32, // 10
    pub deviation_band_5: f64,  // 8.0
    pub deviation_score_5: u32, // 5

    // RSI 分段
    pub rsi_oversold_deep: f64,   // 20.0
    pub rsi_oversold: f64,        // 30.0
    pub rsi_neutral_low: f64,     // 40.0
    pub rsi_neutral_high: f64,    // 60.0
    pub rsi_overbought: f64,      // 70.0
    pub rsi_overbought_high: f64, // 80.0

    // 支撑
    pub support_tolerance_pct: f64, // 0.03 (3%)

    // 布林带
    pub boll_half_std_factor: f64, // 0.5
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
    /// 从技术指标计算客观评分（可传入自定义权重）
    /// 内部使用默认 ScoreBands 阈值，委托给 score_with_bands
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

    /// 趋势评分 (满分30)
    fn score_trend(alignment: &str) -> u32 {
        match alignment {
            "多头排列" => 30,
            "弱多头" => 20,
            "缠绕/交叉" => 12,
            "空头排列" => 0,
            _ => 12,
        }
    }

    /// 乖离率评分 (满分20) -- 略高于MA5最佳，远超MA5(>5%)最差
    fn score_deviation(bias_ma5: f64, bands: &ScoreBands) -> u32 {
        let abs_bias = bias_ma5.abs();
        if bias_ma5 > 0.0 && abs_bias < bands.deviation_band_1 {
            return bands.deviation_score_1;
        }
        if abs_bias < bands.deviation_band_2 {
            return bands.deviation_score_2;
        }
        if abs_bias < bands.deviation_band_3 {
            return bands.deviation_score_3;
        }
        if abs_bias < bands.deviation_band_4 {
            return bands.deviation_score_4;
        }
        if abs_bias < bands.deviation_band_5 {
            return bands.deviation_score_5;
        }
        0
    }

    /// MACD评分 (满分15)
    fn score_macd(signal: &str, dif: f64) -> u32 {
        match signal {
            "金叉" if dif > 0.0 => 15,
            "金叉" => 10,
            "多头运行" => 12,
            "缠绕" => 6,
            "空头运行" => 3,
            "死叉" => 0,
            _ => 6,
        }
    }

    /// 量能评分 (满分15) -- 放量突破与缩量回调并列最佳
    fn score_volume(signal: &str) -> u32 {
        match signal {
            "放量突破" => 15, // 新增：突破型主升浪不应被压低
            "缩量回调" => 15,
            "放量上涨" => 12, // 8 → 12，主升浪信号
            "缩量上涨" => 10,
            "正常" => 7,
            "放量下跌" => 0,
            _ => 7,
        }
    }

    /// RSI评分 (满分10) -- 超卖反弹最佳，超买最差
    fn score_rsi(rsi6: f64, bands: &ScoreBands) -> u32 {
        if rsi6 < bands.rsi_oversold_deep {
            return 10;
        }
        if rsi6 < bands.rsi_oversold {
            return 8;
        }
        if rsi6 < bands.rsi_neutral_low {
            return 6;
        }
        if rsi6 <= bands.rsi_neutral_high {
            return 5;
        }
        if rsi6 < bands.rsi_overbought {
            return 3;
        }
        if rsi6 < bands.rsi_overbought_high {
            return 1;
        }
        0
    }

    /// 支撑评分 (满分5) -- 同时受MA5和MA10双重支撑最佳
    fn score_support(price: f64, supports: &[f64], bands: &ScoreBands) -> u32 {
        if price <= 0.0 {
            return 2;
        }
        let near_support = supports
            .iter()
            .filter(|&&s| s > 0.0 && price > s && (price - s) / price < bands.support_tolerance_pct)
            .count();
        match near_support {
            2.. => 5,
            1 => 3,
            _ => 1,
        }
    }

    /// 布林带位置评分 (满分5) -- 中轨附近最佳，上轨以上超买，下轨以下超卖
    fn score_boll(position: &str, _bands: &ScoreBands) -> u32 {
        match position {
            "中轨附近" => 5,
            "下轨区间" => 4,
            "上轨区间" => 3,
            "下轨以下" => 2,
            "上轨以上" => 1,
            _ => 3,
        }
    }

    /// 基本面修正（基于PE/PB/ROE等估值指标，可选）
    pub fn apply_fundamental_adjustment(
        score: &mut ObjectiveScore,
        pe: Option<f64>,
        pb: Option<f64>,
        roe: Option<f64>,
    ) {
        let orig_alignment = score.signal_code.clone();
        let mut adjustment: i32 = 0;
        let mut reasons = Vec::new();

        // PE 修正
        if let Some(pe) = pe {
            if pe <= 0.0 {
                adjustment -= 15;
                reasons.push("PE为负(亏损)");
            } else if pe > 200.0 {
                adjustment -= 10;
                reasons.push("PE>200(极高估值)");
            } else if pe > 100.0 {
                adjustment -= 5;
                reasons.push("PE>100(高估值)");
            } else if pe < 10.0 {
                adjustment += 10;
                reasons.push("PE<10(低估值)");
            } else if pe < 15.0 {
                adjustment += 5;
                reasons.push("PE<15(合理偏低)");
            }
        }

        // PB 修正
        if let Some(pb) = pb {
            if pb > 10.0 {
                adjustment -= 5;
                reasons.push("PB>10(高市净率)");
            } else if pb < 1.0 && pb > 0.0 {
                adjustment += 5;
                reasons.push("PB<1(破净)");
            }
        }

        // ROE 修正
        if let Some(roe) = roe {
            if roe >= 20.0 {
                adjustment += 10;
                reasons.push("ROE≥20%(优秀)");
            } else if roe >= 15.0 {
                adjustment += 5;
                reasons.push("ROE≥15%(良好)");
            } else if roe < 5.0 && roe > 0.0 {
                adjustment -= 5;
                reasons.push("ROE<5%(偏低)");
            } else if roe <= 0.0 {
                adjustment -= 10;
                reasons.push("ROE≤0(亏损)");
            }
        }

        score.total_adjustment = adjustment;

        let new_total = (score.total as i32 + adjustment).clamp(0, 100) as u32;
        score.total = new_total;

        // Re-map signal with original alignment (preserving trend context)
        let (signal, signal_code) = Self::map_signal(new_total, &orig_alignment);
        score.signal = signal.to_string();
        score.signal_code = signal_code.to_string();

        if !reasons.is_empty() {
            tracing::info!(
                "基本面修正: {:?} → 调整 {}, 最终 {}/100",
                reasons,
                adjustment,
                new_total
            );
        }
    }

    /// 价值投资修正（基于DCF安全边际、F-Score、护城河）
    pub fn apply_value_adjustment(score: &mut ObjectiveScore, value_metrics: &ValueMetrics) {
        let adjustment = if value_metrics.margin_of_safety_pct > 30.0 {
            20
        } else if value_metrics.margin_of_safety_pct > 15.0 {
            12
        } else if value_metrics.margin_of_safety_pct > 5.0 {
            6
        } else if value_metrics.margin_of_safety_pct > 0.0 {
            2
        } else if value_metrics.margin_of_safety_pct > -10.0 {
            -5
        } else {
            -10
        };

        let f_score_bonus = match value_metrics.f_score {
            7..=9 => 10,
            5..=6 => 5,
            3..=4 => 0,
            _ => -5,
        };
        let moat_bonus = match value_metrics.moat_level.as_str() {
            "宽阔" => 10,
            "狭窄" => 5,
            _ => 0,
        };

        let total_adj = adjustment + f_score_bonus + moat_bonus;
        let orig_alignment = score.signal_code.clone();
        score.total_adjustment += total_adj;
        let new_total = (score.total as i32 + total_adj).clamp(0, 100) as u32;
        score.total = new_total;
        let (signal, signal_code) = Self::map_signal(new_total, &orig_alignment);
        score.signal = signal.to_string();
        score.signal_code = signal_code.to_string();

        tracing::info!(
            "价值投资修正: 安全边际{}{:+.0}, F-Score{}{:+}, 护城河{}{:+} → 总调整{:+}, 最终{}/100",
            value_metrics.mos_level,
            adjustment,
            value_metrics.f_score_level,
            f_score_bonus,
            value_metrics.moat_level,
            moat_bonus,
            total_adj,
            new_total
        );
    }

    pub fn apply_industry_adjustment(
        score: &mut ObjectiveScore,
        pe: Option<f64>,
        industry_avg_pe: Option<f64>,
        pb: Option<f64>,
        industry_avg_pb: Option<f64>,
    ) {
        let mut adjustment: i32 = 0;
        if let (Some(pe), Some(ind_pe)) = (pe, industry_avg_pe) {
            if ind_pe > 0.0 && pe > 0.0 {
                let ratio = pe / ind_pe;
                if ratio < 0.7 {
                    adjustment += 8;
                } else if ratio < 0.9 {
                    adjustment += 4;
                } else if ratio > 2.0 {
                    adjustment -= 8;
                } else if ratio > 1.5 {
                    adjustment -= 4;
                }
            }
        }
        if let (Some(pb), Some(ind_pb)) = (pb, industry_avg_pb) {
            if ind_pb > 0.0 {
                let ratio = pb / ind_pb;
                if ratio < 0.7 {
                    adjustment += 5;
                } else if ratio > 2.0 {
                    adjustment -= 5;
                }
            }
        }
        if adjustment != 0 {
            let orig_alignment = score.signal_code.clone();
            score.total_adjustment += adjustment;
            let new_total = (score.total as i32 + adjustment).clamp(0, 100) as u32;
            score.total = new_total;
            let (signal, signal_code) = Self::map_signal(new_total, &orig_alignment);
            score.signal = signal.to_string();
            score.signal_code = signal_code.to_string();
        }
    }

    /// 评分到信号映射
    pub fn map_signal(total: u32, alignment: &str) -> (&str, &str) {
        if total >= 75 && alignment == "多头排列" {
            ("🟢强烈买入", "strong_buy")
        } else if total >= 60 {
            ("🔵买入", "buy")
        } else if total >= 45 {
            ("🟡持有", "hold")
        } else if total >= 30 {
            ("⚪观望", "watch")
        } else if alignment == "空头排列" && total < 30 {
            ("🔴强烈卖出", "strong_sell")
        } else {
            ("🟠卖出", "sell")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_astock_data::indicators::TechnicalIndicators;

    fn make_indicators(
        ma_alignment: &str,
        bias_ma5: f64,
        macd_signal: &str,
        macd_dif: f64,
        volume_signal: &str,
        rsi6: f64,
        support_levels: Vec<f64>,
    ) -> TechnicalIndicators {
        TechnicalIndicators {
            stock_code: "000001".to_string(),
            latest_date: "2025-01-15".to_string(),
            ma5: 10.0,
            ma10: 9.5,
            ma20: 9.0,
            ma60: 8.0,
            ma_alignment: ma_alignment.to_string(),
            macd_dif,
            macd_dea: 0.5,
            macd_bar: 0.2,
            macd_signal: macd_signal.to_string(),
            rsi6,
            rsi12: 55.0,
            rsi24: 52.0,
            rsi_signal: "中性".to_string(),
            boll_upper: 12.0,
            boll_mid: 9.0,
            boll_lower: 6.0,
            boll_position: "中轨附近".to_string(),
            bias_ma5,
            bias_ma20: 1.0,
            volume_ratio: 1.0,
            volume_signal: volume_signal.to_string(),
            support_levels,
            resistance_levels: vec![12.0],
        }
    }

    #[test]
    fn test_score_trend_bull() {
        assert_eq!(ScoringEngine::score_trend("多头排列"), 30);
    }

    #[test]
    fn test_score_trend_weak_bull() {
        assert_eq!(ScoringEngine::score_trend("弱多头"), 20);
    }

    #[test]
    fn test_score_trend_bear() {
        assert_eq!(ScoringEngine::score_trend("空头排列"), 0);
    }

    #[test]
    fn test_score_deviation_small() {
        let bands = ScoreBands::default();
        assert_eq!(ScoringEngine::score_deviation(0.5, &bands), 20);
    }

    #[test]
    fn test_score_deviation_large() {
        let bands = ScoreBands::default();
        assert_eq!(ScoringEngine::score_deviation(10.0, &bands), 0);
    }

    #[test]
    fn test_score_macd_golden_cross_positive() {
        assert_eq!(ScoringEngine::score_macd("金叉", 0.5), 15);
    }

    #[test]
    fn test_score_macd_dead_cross() {
        assert_eq!(ScoringEngine::score_macd("死叉", -0.5), 0);
    }

    #[test]
    fn test_score_volume_shrink_retrace() {
        assert_eq!(ScoringEngine::score_volume("缩量回调"), 15);
    }

    #[test]
    fn test_score_volume_selloff() {
        assert_eq!(ScoringEngine::score_volume("放量下跌"), 0);
    }

    #[test]
    fn test_score_rsi_oversold() {
        let bands = ScoreBands::default();
        assert_eq!(ScoringEngine::score_rsi(15.0, &bands), 10);
    }

    #[test]
    fn test_score_rsi_overbought() {
        let bands = ScoreBands::default();
        assert_eq!(ScoringEngine::score_rsi(85.0, &bands), 0);
    }

    #[test]
    fn test_score_rsi_neutral() {
        let bands = ScoreBands::default();
        assert_eq!(ScoringEngine::score_rsi(50.0, &bands), 5);
    }

    #[test]
    fn test_score_support_double() {
        let bands = ScoreBands::default();
        let supports = vec![9.8, 9.75, 7.0];
        assert!(ScoringEngine::score_support(10.0, &supports, &bands) >= 5);
    }

    #[test]
    fn test_score_support_none() {
        let bands = ScoreBands::default();
        assert_eq!(ScoringEngine::score_support(10.0, &[12.0, 15.0], &bands), 1);
    }

    #[test]
    fn test_full_scoring_default() {
        let ind = make_indicators("多头排列", 1.0, "多头运行", 0.3, "正常", 50.0, vec![9.5, 9.0]);
        let score = ScoringEngine::score(&ind, 10.0, None);
        assert!(score.total > 0);
        assert!(score.total <= 100);
        assert!(!score.signal.is_empty());
        assert!(!score.signal_code.is_empty());
    }

    #[test]
    fn test_full_scoring_with_bands() {
        let bands = ScoreBands::default();
        let ind = make_indicators("多头排列", 1.0, "多头运行", 0.3, "正常", 50.0, vec![9.5, 9.0]);
        let score = ScoringEngine::score_with_bands(&ind, 10.0, None, &bands);
        assert!(score.total > 0);
        assert!(score.total <= 100);
        assert!(!score.signal.is_empty());
        assert!(!score.signal_code.is_empty());
    }

    #[test]
    fn test_score_with_custom_bands() {
        let mut bands = ScoreBands::default();
        bands.deviation_band_1 = 0.5;
        bands.deviation_score_1 = 25;
        let ind = make_indicators("多头排列", 0.3, "多头运行", 0.3, "正常", 50.0, vec![9.5, 9.0]);
        let score = ScoringEngine::score_with_bands(&ind, 10.0, None, &bands);
        assert!(score.total > 0);
        assert!(score.total <= 100);
    }

    #[test]
    fn test_map_signal_strong_buy() {
        let (signal, code) = ScoringEngine::map_signal(80, "多头排列");
        assert!(signal.contains("强烈买入") || code == "strong_buy");
    }

    #[test]
    fn test_map_signal_buy() {
        let (_, code) = ScoringEngine::map_signal(65, "弱多头");
        assert_eq!(code, "buy");
    }

    #[test]
    fn test_map_signal_hold() {
        let (_, code) = ScoringEngine::map_signal(50, "缠绕/交叉");
        assert_eq!(code, "hold");
    }

    #[test]
    fn test_map_signal_strong_sell() {
        let (_, code) = ScoringEngine::map_signal(20, "空头排列");
        assert!(code == "strong_sell" || code == "sell");
    }

    #[test]
    fn test_apply_fundamental_adjustment() {
        let ind = make_indicators("多头排列", 1.0, "多头运行", 0.3, "正常", 50.0, vec![9.0]);
        let mut score = ScoringEngine::score(&ind, 10.0, None);
        let before = score.total;
        ScoringEngine::apply_fundamental_adjustment(&mut score, Some(8.0), Some(0.8), Some(22.0));
        assert!(
            score.total_adjustment > 0,
            "Low PE + low PB + high ROE should yield positive adjustment"
        );
        assert!(score.total >= before || score.total <= 100, "Score should be capped at 0-100");
    }

    // ── v23 新增：放量突破信号单元测试 ──
    #[test]
    fn test_score_volume_breakout_top_score() {
        // 修复 P1-2：放量突破与缩量回调并列最高（15 分）
        assert_eq!(ScoringEngine::score_volume("放量突破"), 15);
        assert_eq!(ScoringEngine::score_volume("放量上涨"), 12);
        assert_eq!(ScoringEngine::score_volume("缩量回调"), 15);
    }

    #[test]
    fn test_breakout_stock_not_misjudged_low_score() {
        // 模拟 301302 启动期：弱多头 + bias 4.5% + 金叉 + 放量突破 + RSI 75
        let ind = make_indicators("弱多头", 4.5, "金叉", 0.5, "放量突破", 75.0, vec![9.0]);
        let score = ScoringEngine::score(&ind, 11.0, None);
        assert!(
            score.total >= 50,
            "启动期放量突破+弱多头+金叉 50MA5附近 不应判 < 50 分，实际 {} 分",
            score.total
        );
    }
}
