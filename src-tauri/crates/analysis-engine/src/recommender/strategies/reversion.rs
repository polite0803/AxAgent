//! 超跌反弹子策略：RSI 超卖 / RSI 底背离 / 看涨 K 线形态 / 缩量回踩
//!
//! v1 不做长线（设计文档 §2.3）

use super::super::strategy::{read_f64, RecoContext, RecommendStrategy};
use crate::candlestick_pattern;
use crate::divergence;
use crate::recommender::indicators;
use crate::recommender::scoring::{calc_confidence, calc_position};
use crate::recommender::types::{Period, RecoPick, Style};
use async_trait::async_trait;
use axagent_harness::market_data::MarketDataProvider;
use serde_json::Value;
use std::collections::HashMap;

pub struct ReversionStrategy {
    pub period: Period,
}

impl ReversionStrategy {
    pub const fn ultra_short() -> Self {
        Self { period: Period::UltraShort }
    }
    pub const fn short() -> Self {
        Self { period: Period::Short }
    }
    pub const fn mid() -> Self {
        Self { period: Period::Mid }
    }

    async fn scan_one(
        &self,
        client: &dyn MarketDataProvider,
        code: &str,
        name: &str,
        sector: Option<String>,
        vars: &HashMap<String, Value>,
    ) -> Option<RecoPick> {
        let kline_limit = read_f64(vars, "rev_kline_limit", 250.0) as u32;
        let klines = client.get_klines(code, "daily", kline_limit, None).await.ok()?;
        let min_kline_len = read_f64(vars, "rev_min_kline_len", 30.0) as usize;
        if klines.len() < min_kline_len {
            return None;
        }
        let price = klines.last()?.close;

        // ── 新增：底背离检测 + 看涨 K 线形态检测 ──
        let lookback = read_f64(vars, "rev_divergence_lookback", 14.0) as usize;
        let min_divergence_strength = read_f64(vars, "rev_min_divergence_strength", 0.3);
        let divergence_result = divergence::detect_all_divergences(&klines, 14, lookback);
        let has_bullish_divergence = divergence_result.iter().any(|d| {
            d.divergence_type == "regular_bullish" && d.strength >= min_divergence_strength
        });
        let best_divergence =
            divergence_result.into_iter().find(|d| d.divergence_type == "regular_bullish");

        let patterns = candlestick_pattern::detect_all_patterns(&klines);
        let has_bullish_pattern =
            patterns.iter().any(|p| p.direction == "看涨" && p.confidence >= 0.5);
        let best_bullish_pattern =
            patterns.into_iter().find(|p| p.direction == "看涨" && p.confidence >= 0.5);

        let divergence_bonus = if has_bullish_divergence {
            read_f64(vars, "rev_divergence_bonus", 0.10)
        } else {
            0.0
        };
        let pattern_bonus = if has_bullish_pattern {
            read_f64(vars, "rev_pattern_bonus", 0.05)
        } else {
            0.0
        };
        // ── 新增结束 ──

        let rsi_period = read_f64(vars, "rev_rsi_period", 6.0) as usize;
        let rsi_value = indicators::rsi(&klines, rsi_period)?;

        let (pass, reasons) = match self.period {
            Period::UltraShort => return None, // 超短线不适用超跌反弹
            Period::Short => {
                // 底背离 → 放宽 RSI 阈值从 35 至 40，提前捕捉反转
                let rsi_short_max = if has_bullish_divergence {
                    read_f64(vars, "rev_rsi_short_max_divergence", 40.0)
                } else {
                    read_f64(vars, "rev_rsi_short_max", 35.0)
                };
                if rsi_value >= rsi_short_max {
                    return None;
                }
                let avg_period = read_f64(vars, "rev_avg_amount_period", 5.0) as usize;
                let avg_mult = read_f64(vars, "rev_avg_amount_mult", 1.2);
                let avg_5 = indicators::avg_amount_n(&klines, avg_period).unwrap_or(0.0);
                let today = klines.last().map(|k| k.amount).unwrap_or(0.0);
                if avg_5 <= 0.0 || today > avg_5 * avg_mult {
                    return None;
                }
                let mut r = vec![
                    format!("RSI({}) {:.1} 超卖", rsi_period, rsi_value),
                    format!("量比 {} 日均 {:.0}%", avg_period, today / avg_5 * 100.0),
                ];
                // 追加背离/形态理由
                if let Some(d) = &best_divergence {
                    r.push(format!(
                        "{}（强度 {:.0}%）",
                        d.description.split('，').next().unwrap_or(&d.description),
                        d.strength * 100.0
                    ));
                }
                if let Some(p) = &best_bullish_pattern {
                    r.push(format!("出现{}形态", p.pattern));
                }
                (true, r)
            },
            Period::Mid => {
                let dd_period = read_f64(vars, "rev_dd_period", 250.0) as usize;
                let dd_min = read_f64(vars, "rev_dd_min", 20.0);
                let dd = indicators::drawdown_from_high(&klines, dd_period).unwrap_or(0.0);
                if dd < dd_min {
                    return None;
                }
                let rsi_mid_period = read_f64(vars, "rev_rsi_mid_period", 30.0) as usize;
                let rsi_mid_max = if has_bullish_divergence {
                    // 底背离放宽月线 RSI 阈值从 50 至 55
                    read_f64(vars, "rev_rsi_mid_max_divergence", 55.0)
                } else {
                    read_f64(vars, "rev_rsi_mid_max", 50.0)
                };
                let rsi_30 = indicators::rsi(&klines, rsi_mid_period).unwrap_or(rsi_mid_max);
                if rsi_30 > rsi_mid_max {
                    return None;
                }
                let mut r = vec![
                    format!("距 {} 日高回撤 {:.0}%", dd_period, dd),
                    format!("月线 RSI {:.1}", rsi_30),
                ];
                if let Some(d) = &best_divergence {
                    r.push(format!(
                        "{}（强度 {:.0}%）",
                        d.description.split('，').next().unwrap_or(&d.description),
                        d.strength * 100.0
                    ));
                }
                if let Some(p) = &best_bullish_pattern {
                    r.push(format!("出现{}形态", p.pattern));
                }
                (true, r)
            },
            Period::Long => return None, // v1 不做长线超跌
        };

        if !pass {
            return None;
        }

        let (entry_low, entry_high, stop_loss, target, base_position) = match self.period {
            Period::UltraShort => return None,
            Period::Short => {
                let el = read_f64(vars, "rev_short_entry_low", 0.97);
                let eh = read_f64(vars, "rev_short_entry_high", 1.03);
                let sl = read_f64(vars, "rev_short_stop", 0.93);
                let tg = read_f64(vars, "rev_short_target", 1.08);
                let bp = read_f64(vars, "rev_short_base_pos", 3.0);
                (price * el, price * eh, price * sl, price * tg, bp)
            },
            Period::Mid => {
                let el = read_f64(vars, "rev_mid_entry_low", 0.95);
                let eh = read_f64(vars, "rev_mid_entry_high", 1.05);
                let sl = read_f64(vars, "rev_mid_stop", 0.88);
                let tg = read_f64(vars, "rev_mid_target", 1.20);
                let bp = read_f64(vars, "rev_mid_base_pos", 5.0);
                (price * el, price * eh, price * sl, price * tg, bp)
            },
            Period::Long => return None,
        };

        // 底背离 + 看涨形态 → 提升 signal_strength 系数
        let signal_strength =
            (read_f64(vars, "rev_conf_signal", 0.8) + divergence_bonus + pattern_bonus).min(0.99);
        let conf = calc_confidence(
            read_f64(vars, "rev_conf_consistency", 0.7),
            signal_strength,
            read_f64(vars, "rev_conf_direction", 0.6),
            read_f64(vars, "rev_conf_market", 0.0),
            read_f64(vars, "rev_conf_base", 1.0),
        );
        let position = calc_position(base_position, conf, self.period);

        // 底背离 → 在 risk_notes 中减弱"抄底过早"警告
        let mut risk_notes = vec!["下跌趋势未尽 / 抄底过早".to_string()];
        if has_bullish_divergence {
            risk_notes.push("RSI 底背离出现，下跌动能衰减".to_string());
        }

        Some(RecoPick {
            stock_code: code.into(),
            stock_name: name.into(),
            sector,
            style: Style::Reversion,
            period: self.period,
            price,
            entry_low,
            entry_high,
            stop_loss,
            target_price: target,
            position_pct: position,
            holding_days: self.period.default_holding_days(),
            confidence: conf,
            reasons,
            risk_notes,
            secondary_styles: vec![],
            synthetic: false,
        })
    }
}

#[async_trait]
impl RecommendStrategy for ReversionStrategy {
    fn id(&self) -> &'static str {
        match self.period {
            Period::UltraShort => "rev_ultra_short",
            Period::Short => "rev_short",
            Period::Mid => "rev_mid",
            Period::Long => "rev_long",
        }
    }
    fn style(&self) -> Style {
        Style::Reversion
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
            if let Some(p) = self.scan_one(ctx.client, code, name, sector.clone(), ctx.vars).await {
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
    fn reversion_strategy_ids() {
        assert_eq!(ReversionStrategy::ultra_short().id(), "rev_ultra_short");
        assert_eq!(ReversionStrategy::short().id(), "rev_short");
        assert_eq!(ReversionStrategy::mid().id(), "rev_mid");
    }
}
