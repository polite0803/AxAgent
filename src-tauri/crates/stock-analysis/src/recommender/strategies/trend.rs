//! 趋势跟踪子策略：MA 多头 + 突破 + 量能

use super::super::strategy::{read_f64, RecoContext, RecommendStrategy};
use crate::recommender::indicators;
use crate::recommender::scoring::{calc_confidence, calc_position};
use crate::recommender::types::{Period, RecoPick, Style};
use async_trait::async_trait;
use axagent_astock_data::AStockClient;
use serde_json::Value;
use std::collections::HashMap;

pub struct TrendStrategy {
    pub period: Period,
}

impl TrendStrategy {
    pub const fn ultra_short() -> Self {
        Self {
            period: Period::UltraShort,
        }
    }
    pub const fn short() -> Self {
        Self {
            period: Period::Short,
        }
    }
    pub const fn mid() -> Self {
        Self {
            period: Period::Mid,
        }
    }
    pub const fn long() -> Self {
        Self {
            period: Period::Long,
        }
    }

    async fn scan_one(
        &self,
        client: &AStockClient,
        code: &str,
        name: &str,
        sector: Option<String>,
        vars: &HashMap<String, Value>,
    ) -> Option<RecoPick> {
        // 按 period 差异化 K 线拉取量：超短只需少量，长线需要大量
        let kline_limit = match self.period {
            Period::UltraShort => read_f64(vars, "trend_ultra_short_kline_limit", 20.0) as u32,
            Period::Short => read_f64(vars, "trend_short_kline_limit", 60.0) as u32,
            Period::Mid => read_f64(vars, "trend_mid_kline_limit", 150.0) as u32,
            Period::Long => read_f64(vars, "trend_long_kline_limit", 300.0) as u32,
        };
        let klines = client.get_klines(code, "daily", kline_limit).await.ok()?;
        // 修复 V51：min_kline_len 按周期差异化，避免 UltraShort 拉取 20 根却要求 30 根的矛盾
        let min_kline_len = match self.period {
            Period::UltraShort => read_f64(vars, "trend_ultra_short_min_kline_len", 15.0) as usize,
            Period::Short => read_f64(vars, "trend_short_min_kline_len", 30.0) as usize,
            Period::Mid => read_f64(vars, "trend_mid_min_kline_len", 60.0) as usize,
            Period::Long => read_f64(vars, "trend_long_min_kline_len", 120.0) as usize,
        };
        if klines.len() < min_kline_len {
            return None;
        }
        let cs = indicators::closes(&klines);
        let last = *cs.last()?;

        // 拉取 20 日均成交额，量比估算
        let avg_20 = indicators::avg_amount_20d(&klines).unwrap_or(0.0);
        let today_amount = klines.last().map(|k| k.amount).unwrap_or(0.0);
        let turnover_anomaly = if avg_20 > 0.0 {
            today_amount / avg_20
        } else {
            1.0
        };
        let amount_ratio = turnover_anomaly; // 量比近似

        let (entry_low, entry_high, stop_loss, target_price, base_position, reasons) =
            match self.period {
                Period::UltraShort => {
                    let ma_period_1 = read_f64(vars, "trend_ma_ultra_short_1", 5.0) as usize;
                    let ma_period_2 = read_f64(vars, "trend_ma_ultra_short_2", 10.0) as usize;
                    let ma5 = indicators::sma(&cs, ma_period_1)?;
                    let ma10 = indicators::sma(&cs, ma_period_2)?;
                    if ma5 <= ma10 {
                        return None;
                    }
                    let high_period = read_f64(vars, "trend_high_ultra_short_period", 5.0) as usize;
                    let high_threshold = read_f64(vars, "trend_high_ultra_short_threshold", 0.995);
                    let high_5 = indicators::highest(&klines, high_period)?;
                    if last < high_5 * high_threshold {
                        return None;
                    }
                    let amount_ratio_min = read_f64(vars, "trend_amount_ratio_min", 0.8);
                    if amount_ratio < amount_ratio_min {
                        return None;
                    }
                    let reasons = vec![
                        format!("MA{} {:.2} > MA{} {:.2}", ma_period_1, ma5, ma_period_2, ma10),
                        format!("突破 {} 日高 {:.2}", high_period, high_5),
                        format!("量比 {:.2}", amount_ratio),
                    ];
                    let el = read_f64(vars, "trend_ultra_short_entry_low", 0.995);
                    let eh = read_f64(vars, "trend_ultra_short_entry_high", 1.005);
                    let sl = read_f64(vars, "trend_ultra_short_stop", 0.98);
                    let tg = read_f64(vars, "trend_ultra_short_target", 1.05);
                    let bp = read_f64(vars, "trend_ultra_short_base_pos", 3.0);
                    (ma5 * el, ma5 * eh, ma5 * sl, ma5 * tg, bp, reasons)
                },
                Period::Short => {
                    let ma_period_1 = read_f64(vars, "trend_ma_short_1", 5.0) as usize;
                    let ma_period_2 = read_f64(vars, "trend_ma_short_2", 10.0) as usize;
                    let ma_period_3 = read_f64(vars, "trend_ma_short_3", 20.0) as usize;
                    let ma5 = indicators::sma(&cs, ma_period_1)?;
                    let ma10 = indicators::sma(&cs, ma_period_2)?;
                    let ma20 = indicators::sma(&cs, ma_period_3)?;
                    // 放宽条件：短期动量（MA5>MA10）+ 股价未远离中期均线
                    // 原条件 MA5>MA10>MA20 三重多头在震荡市几乎无法满足
                    let ma20_tolerance = read_f64(vars, "trend_short_ma20_tolerance", 0.985);
                    if !(ma5 > ma10 && last >= ma20 * ma20_tolerance) {
                        return None;
                    }
                    let high_period = read_f64(vars, "trend_high_20_period", 20.0) as usize;
                    let high_threshold = read_f64(vars, "trend_high_20_threshold", 0.97);
                    let high_20 = indicators::highest(&klines, high_period)?;
                    if last < high_20 * high_threshold {
                        return None;
                    }
                    let amount_ratio_min = read_f64(vars, "trend_amount_ratio_min", 0.8);
                    if amount_ratio < amount_ratio_min {
                        return None;
                    }
                    let ma_align = if ma10 > ma20 {
                        "多头排列"
                    } else {
                        "站上均线"
                    };
                    let reasons = vec![
                        format!(
                            "MA{} {:.2} > MA{} {:.2}, {} MA{} {:.2}",
                            ma_period_1, ma5, ma_period_2, ma10, ma_align, ma_period_3, ma20
                        ),
                        format!("突破 {} 日高 {:.2}", high_period, high_20),
                        format!("量比 {:.2}", amount_ratio),
                    ];
                    let _atr = (high_20
                        - indicators::lowest(&klines, high_period).unwrap_or(ma20 * 0.95))
                        * 0.5;
                    let el = read_f64(vars, "trend_short_entry_low", 0.99);
                    let eh = read_f64(vars, "trend_short_entry_high", 1.015);
                    let sl = read_f64(vars, "trend_short_stop", 0.95);
                    let tg = read_f64(vars, "trend_short_target", 1.10);
                    let bp = read_f64(vars, "trend_short_base_pos", 5.0);
                    (ma5 * el, ma5 * eh, ma5 * sl, ma5 * tg, bp, reasons)
                },
                Period::Mid => {
                    let ma_period_s = read_f64(vars, "trend_ma_mid_s", 20.0) as usize;
                    let ma_period_l = read_f64(vars, "trend_ma_mid_l", 60.0) as usize;
                    let ma20 = indicators::sma(&cs, ma_period_s)?;
                    let ma60 = indicators::sma(&cs, ma_period_l)?;
                    let ma60_threshold = read_f64(vars, "trend_ma60_threshold", 0.985);
                    if ma60.is_nan() || last < ma60 * ma60_threshold {
                        return None;
                    }
                    let high_period = read_f64(vars, "trend_high_60_period", 60.0) as usize;
                    let high_threshold = read_f64(vars, "trend_high_60_threshold", 0.94);
                    let high_60 = indicators::highest(&klines, high_period)?;
                    if last < high_60 * high_threshold {
                        return None;
                    }
                    if let Some((dif, dea, macd_bar)) = indicators::macd(&klines, 12, 26, 9) {
                        if dif <= dea {
                            return None;
                        }
                        let reasons = vec![
                            format!("站上 MA{} {:.2}", ma_period_l, ma60),
                            format!("突破 {} 日高 {:.2}", high_period, high_60),
                            format!("MACD 红柱 {:.2}", macd_bar),
                        ];
                        let el = read_f64(vars, "trend_mid_entry_low", 0.97);
                        let eh = read_f64(vars, "trend_mid_entry_high", 1.05);
                        let sl = read_f64(vars, "trend_mid_stop", 0.92);
                        let tg = read_f64(vars, "trend_mid_target", 1.05);
                        let bp = read_f64(vars, "trend_mid_base_pos", 8.0);
                        (ma20 * el, ma20 * eh, ma20 * sl, high_60 * tg, bp, reasons)
                    } else {
                        return None;
                    }
                },
                Period::Long => {
                    let ma_period_s = read_f64(vars, "trend_ma_long_s", 60.0) as usize;
                    let ma_period_l = read_f64(vars, "trend_ma_long_l", 250.0) as usize;
                    let ma60 = indicators::sma(&cs, ma_period_s)?;
                    let ma250 = indicators::sma(&cs, ma_period_l)?;
                    let ma60_ma250_mult = read_f64(vars, "trend_ma60_ma250_mult", 0.95);
                    if ma250.is_nan() || ma60 < ma250 * ma60_ma250_mult {
                        return None;
                    }
                    let ma60_break_mult = read_f64(vars, "trend_ma60_break_mult", 0.95);
                    if last < ma60 * ma60_break_mult {
                        return None;
                    }
                    let reasons = vec![
                        format!(
                            "MA{} {:.2} > MA{} {:.2} 长期多头",
                            ma_period_s, ma60, ma_period_l, ma250
                        ),
                        format!("回踩未破 MA{}", ma_period_s),
                    ];
                    let el = read_f64(vars, "trend_long_entry_low", 0.95);
                    let eh = read_f64(vars, "trend_long_entry_high", 1.03);
                    let sl = read_f64(vars, "trend_long_stop", 0.85);
                    let tg = read_f64(vars, "trend_long_target", 1.30);
                    let bp = read_f64(vars, "trend_long_base_pos", 10.0);
                    (ma60 * el, ma60 * eh, ma60 * sl, last * tg, bp, reasons)
                },
            };

        // 置信度
        let conf_consistency = read_f64(vars, "trend_conf_consistency", 0.85);
        let conf_signal = read_f64(vars, "trend_conf_signal", 0.7);
        let conf_market = read_f64(vars, "trend_conf_market", 0.0);
        let conf = calc_confidence(
            conf_consistency,
            conf_signal,
            if amount_ratio > 1.5 { 0.8 } else { 0.5 },
            conf_market,
            turnover_anomaly,
        );
        let position = calc_position(base_position, conf, self.period);

        Some(RecoPick {
            stock_code: code.into(),
            stock_name: name.into(),
            sector,
            style: Style::Trend,
            period: self.period,
            price: last,
            entry_low,
            entry_high,
            stop_loss,
            target_price,
            position_pct: position,
            holding_days: self.period.default_holding_days(),
            confidence: conf,
            reasons,
            risk_notes: vec!["个股回调 / 跌破短期均线风险".to_string()],
            secondary_styles: vec![],
            synthetic: false,
        })
    }
}

#[async_trait]
impl RecommendStrategy for TrendStrategy {
    fn id(&self) -> &'static str {
        match self.period {
            Period::UltraShort => "trend_ultra_short",
            Period::Short => "trend_short",
            Period::Mid => "trend_mid",
            Period::Long => "trend_long",
        }
    }
    fn style(&self) -> Style {
        Style::Trend
    }
    fn period(&self) -> Period {
        self.period
    }
    fn required_vendors(&self) -> &'static [&'static str] {
        &["eastmoney", "tencent", "ths", "akshare"]
    }

    async fn scan(&self, ctx: &RecoContext<'_>) -> Result<Vec<RecoPick>, String> {
        let mut picks = Vec::new();
        for (code, name, sector) in ctx.seed {
            let _g = ctx.per_code_locks.lock_for(code).await;
            if let Some(p) = self
                .scan_one(ctx.client, code, name, sector.clone(), ctx.vars)
                .await
            {
                picks.push(p);
            }
        }
        Ok(picks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trend_strategy_ids() {
        assert_eq!(TrendStrategy::short().id(), "trend_short");
        assert_eq!(TrendStrategy::mid().id(), "trend_mid");
        assert_eq!(TrendStrategy::long().id(), "trend_long");
        assert_eq!(TrendStrategy::short().style(), Style::Trend);
        assert_eq!(TrendStrategy::mid().period(), Period::Mid);
    }
}
