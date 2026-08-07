//! 趋势跟踪子策略：MA 多头 + 突破 + 量能
//!
//! ## 参数简化（P1-7）
//! 原 ~52 个 per-period read_f64 变量→硬编码周期差异 + 共享乘数。
//! 可配置参数保留 ~10 个，见下方 TREND_VARS 文档。

use super::super::strategy::{read_f64, RecoContext, RecommendStrategy};
use crate::recommender::indicators;
use crate::recommender::scoring::{calc_confidence, calc_position_with_consistency};
use crate::recommender::types::{Period, RecoPick, Style};
use async_trait::async_trait;
use axagent_harness::market_data::MarketDataProvider;
use serde_json::Value;
use std::collections::HashMap;

// ── 可保留的用户可配参数（~10 个） ──
// 以下变量名仍通过 read_f64(vars, ...) 读取，默认值在此定义。
// 删除的 per-period 变量（如 trend_ultra_short_entry_low）默认 fallback 到硬编码。

const DEFAULT_AMOUNT_RATIO_MIN: f64 = 0.8;
const DEFAULT_MA20_TOLERANCE: f64 = 0.985; // Short：距 MA20 最小比例
const DEFAULT_MA60_THRESHOLD: f64 = 0.985; // Mid：距 MA60 ���小比例
const DEFAULT_HIGH_20_THRESHOLD: f64 = 0.97; // Short：距 20 日高最小比例
const DEFAULT_HIGH_60_THRESHOLD: f64 = 0.94; // Mid：距 60 日高最小比例
const DEFAULT_ENTRY_TIGHTNESS: f64 = 1.0; // 入场范围乘数（1.0=标准，1.5=更宽松）
const DEFAULT_STOP_MULT: f64 = 1.0; // 止损乘数（1.0=标准，0.8=更紧）
const DEFAULT_TARGET_MULT: f64 = 1.0; // 目标乘数（1.0=标准，1.2=更激进）
const DEFAULT_POS_ADJ: f64 = 1.0; // 仓位调整（1.0=标准，0.5=半仓）

/// 按周期返回硬编码的 (entry_low, entry_high, stop_loss, target, base_pos, kline_limit, min_kline)
#[inline]
fn period_defaults(p: Period) -> (f64, f64, f64, f64, f64, u32, usize) {
    match p {
        Period::UltraShort => (0.995, 1.005, 0.98, 1.05, 3.0, 20, 15),
        Period::Short => (0.99, 1.015, 0.95, 1.10, 5.0, 60, 30),
        Period::Mid => (0.97, 1.05, 0.92, 1.20, 8.0, 150, 60),
        Period::Long => (0.95, 1.03, 0.85, 1.30, 10.0, 300, 120),
    }
}

pub struct TrendStrategy {
    pub period: Period,
}

impl TrendStrategy {
    pub const fn ultra_short() -> Self {
        Self { period: Period::UltraShort }
    }
    pub const fn short() -> Self {
        Self { period: Period::Short }
    }
    pub const fn mid() -> Self {
        Self { period: Period::Mid }
    }
    pub const fn long() -> Self {
        Self { period: Period::Long }
    }

    async fn scan_one(
        &self,
        client: &dyn MarketDataProvider,
        code: &str,
        name: &str,
        sector: Option<String>,
        vars: &HashMap<String, Value>,
    ) -> Option<RecoPick> {
        let (el, eh, sl, tg, bp, kline_limit, min_kline_len) = period_defaults(self.period);

        let klines = client.get_klines(code, "daily", kline_limit, None).await.ok()?;
        if klines.len() < min_kline_len {
            return None;
        }

        let cs = indicators::closes(&klines);
        let last = *cs.last()?;

        // 量比
        let avg_20 = indicators::avg_amount_20d(&klines).unwrap_or(0.0);
        let today_amount = klines.last().map(|k| k.amount).unwrap_or(0.0);
        let turnover_anomaly = if avg_20 > 0.0 {
            today_amount / avg_20
        } else {
            1.0
        };
        let amount_ratio = turnover_anomaly;

        // ── 共享乘数 ──
        let entry_tightness = read_f64(vars, "trend_entry_tightness", DEFAULT_ENTRY_TIGHTNESS);
        let stop_mult = read_f64(vars, "trend_stop_mult", DEFAULT_STOP_MULT);
        let target_mult = read_f64(vars, "trend_target_mult", DEFAULT_TARGET_MULT);
        let pos_adj = read_f64(vars, "trend_position_adj", DEFAULT_POS_ADJ);

        let (reasons, price_ref) = match self.period {
            Period::UltraShort => {
                // MA5 > MA10
                let ma5 = indicators::sma(&cs, 5)?;
                let ma10 = indicators::sma(&cs, 10)?;
                if ma5 <= ma10 {
                    return None;
                }
                let high_5 = indicators::highest(&klines, 5)?;
                let high_th = read_f64(vars, "trend_high_20_threshold", DEFAULT_HIGH_20_THRESHOLD);
                if last < high_5 * high_th {
                    return None;
                }
                let amt_min = read_f64(vars, "trend_amount_ratio_min", DEFAULT_AMOUNT_RATIO_MIN);
                if amount_ratio < amt_min {
                    return None;
                }
                let r = vec![
                    format!("MA5 {:.2} > MA10 {:.2}", ma5, ma10),
                    format!("突破 5 日高 {:.2}", high_5),
                    format!("量比 {:.2}", amount_ratio),
                ];
                (r, ma5)
            },
            Period::Short => {
                let ma5 = indicators::sma(&cs, 5)?;
                let ma10 = indicators::sma(&cs, 10)?;
                let ma20 = indicators::sma(&cs, 20)?;
                let tol = read_f64(vars, "trend_short_ma20_tolerance", DEFAULT_MA20_TOLERANCE);
                if !(ma5 > ma10 && last >= ma20 * tol) {
                    return None;
                }
                let high_20 = indicators::highest(&klines, 20)?;
                let high_th = read_f64(vars, "trend_high_20_threshold", DEFAULT_HIGH_20_THRESHOLD);
                if last < high_20 * high_th {
                    return None;
                }
                let amt_min = read_f64(vars, "trend_amount_ratio_min", DEFAULT_AMOUNT_RATIO_MIN);
                if amount_ratio < amt_min {
                    return None;
                }
                let ma_align = if ma10 > ma20 {
                    "多头排列"
                } else {
                    "站上均线"
                };
                let r = vec![
                    format!("MA5 {:.2} > MA10 {:.2}, {} MA20 {:.2}", ma5, ma10, ma_align, ma20),
                    format!("突破 20 日高 {:.2}", high_20),
                    format!("量比 {:.2}", amount_ratio),
                ];
                (r, ma5)
            },
            Period::Mid => {
                let ma20 = indicators::sma(&cs, 20)?;
                let ma60 = indicators::sma(&cs, 60)?;
                let ma60_th = read_f64(vars, "trend_ma60_threshold", DEFAULT_MA60_THRESHOLD);
                if ma60.is_nan() || last < ma60 * ma60_th {
                    return None;
                }
                let high_60 = indicators::highest(&klines, 60)?;
                let high_th = read_f64(vars, "trend_high_60_threshold", DEFAULT_HIGH_60_THRESHOLD);
                if last < high_60 * high_th {
                    return None;
                }
                let (dif, dea, macd_bar) = indicators::macd(&klines, 12, 26, 9)?;
                if dif <= dea {
                    return None;
                }
                let r = vec![
                    format!("站上 MA60 {:.2}", ma60),
                    format!("突破 60 日高 {:.2}", high_60),
                    format!("MACD 红柱 {:.2}", macd_bar),
                ];
                (r, ma20)
            },
            Period::Long => {
                let ma60 = indicators::sma(&cs, 60)?;
                let ma250 = indicators::sma(&cs, 250)?;
                if ma250.is_nan() || ma60 < ma250 * 0.95 {
                    return None;
                }
                if last < ma60 * 0.95 {
                    return None;
                }
                let r = vec![
                    format!("MA60 {:.2} > MA250 {:.2} 长期多头", ma60, ma250),
                    "回踩未破 MA60".to_string(),
                ];
                (r, ma60)
            },
        };

        // 应用共享乘数到硬编码默认值
        let entry_low = price_ref * (1.0 - (1.0 - el) * entry_tightness);
        let entry_high = price_ref * (1.0 + (eh - 1.0) * entry_tightness);
        let stop_loss = price_ref * (1.0 - (1.0 - sl) * stop_mult);
        let target_price = price_ref * (1.0 + (tg - 1.0) * target_mult);
        let base_position = bp * pos_adj;

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
        let position =
            calc_position_with_consistency(base_position, conf, conf_consistency, self.period);

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
        // 获取行业排名数据，用于行业动量过滤
        let sector_momentum: HashMap<String, f64> = ctx
            .client
            .get_industry_ranking()
            .await
            .map(|industries| {
                industries
                    .iter()
                    .take(20)
                    .enumerate()
                    .map(|(i, ind)| {
                        let score = (20.0 - i as f64) / 20.0 * 100.0; // 第1名100分，第20名5分
                        (ind.industry_name.clone(), score)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut picks = Vec::new();
        for (code, name, sector) in ctx.seed {
            let _g = ctx.per_code_locks.lock_for(code).await;
            // 根据股票所属行业查找该行业的动量分
            let sector_mom = sector
                .as_ref()
                .and_then(|s| {
                    // 尝试完全匹配，再尝试前缀匹配
                    sector_momentum.get(s).copied().or_else(|| {
                        sector_momentum
                            .iter()
                            .find(|(k, _)| s.contains(k.as_str()) || k.contains(s))
                            .map(|(_, v)| *v)
                    })
                })
                .unwrap_or(50.0);
            // 注入行业动量到 vars，scan_one 可通过 "sector_momentum" 读取
            let mut enriched_vars = ctx.vars.clone();
            enriched_vars.insert("sector_momentum".to_string(), serde_json::json!(sector_mom));

            if let Some(mut p) =
                self.scan_one(ctx.client, code, name, sector.clone(), &enriched_vars).await
            {
                // 行业动量修正：低于40分的行业扣10%置信度，高于80分的加10%
                if sector_mom < 40.0 {
                    p.confidence = ((p.confidence as f64 * 0.9) as u8).max(1);
                    p.reasons.push(format!("行业动量偏低({:.0}分)，置信度下调10%", sector_mom));
                } else if sector_mom > 80.0 {
                    p.confidence = ((p.confidence as f64 * 1.1).min(100.0)) as u8;
                    p.reasons.push(format!("行业动量强劲({:.0}分)，置信度上调10%", sector_mom));
                }
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
