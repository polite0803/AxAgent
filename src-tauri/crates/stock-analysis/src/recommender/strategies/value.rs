//! 价值低估子策略：低估值 + 基本面

use super::super::strategy::{read_f64, RecoContext, RecommendStrategy};
use crate::recommender::scoring::{calc_confidence, calc_position};
use crate::recommender::types::{Period, RecoPick, Style};
use async_trait::async_trait;
use axagent_astock_data::AStockClient;
use serde_json::Value;
use std::collections::HashMap;

pub struct ValueStrategy {
    pub period: Period,
}

impl ValueStrategy {
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
    pub const fn ultra_short() -> Self {
        Self {
            period: Period::UltraShort,
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
        let quote = client.get_quote(code).await.ok()?;
        let price = quote.price;
        let pe = quote.pe.unwrap_or(0.0);
        let pb = quote.pb.unwrap_or(0.0);
        if pe <= 0.0 && pb <= 0.0 {
            return None;
        }

        let (pre_filter_ok, mut reasons) = match self.period {
            Period::UltraShort => {
                let pe_max = read_f64(vars, "val_ultra_short_pe_max", 60.0);
                if pe > 0.0 && pe > pe_max {
                    return None;
                }
                let kline_limit = read_f64(vars, "val_ultra_short_kline_limit", 10.0) as u32;
                let klines = client.get_klines(code, "daily", kline_limit).await.ok()?;
                let min_kline_len = read_f64(vars, "val_ultra_short_min_kline_len", 5.0) as usize;
                if klines.len() < min_kline_len {
                    return None;
                }
                let cs: Vec<f64> = klines.iter().map(|k| k.close).collect();
                let ma_period = read_f64(vars, "val_ultra_short_ma_period", 10.0) as usize;
                let ma10 = crate::recommender::indicators::sma(&cs, ma_period)?;
                let ma_mult = read_f64(vars, "val_ultra_short_ma_mult", 1.005);
                if price > ma10 * ma_mult {
                    return None;
                }
                let mut r = Vec::new();
                if pe > 0.0 {
                    r.push(format!("PE {:.1} 严重压缩", pe));
                }
                if pb > 0.0 {
                    r.push(format!("PB {:.2}", pb));
                }
                r.push(format!("10日均线下方 {:.2}", ma10));
                (true, r)
            },
            Period::Short => {
                let pe_max = read_f64(vars, "val_short_pe_max", 50.0);
                if pe > 0.0 && pe > pe_max {
                    return None;
                }
                let kline_limit = read_f64(vars, "val_short_kline_limit", 30.0) as u32;
                let klines = client.get_klines(code, "daily", kline_limit).await.ok()?;
                let min_kline_len = read_f64(vars, "val_short_min_kline_len", 20.0) as usize;
                if klines.len() < min_kline_len {
                    return None;
                }
                let cs: Vec<f64> = klines.iter().map(|k| k.close).collect();
                let ma_period = read_f64(vars, "val_short_ma_period", 20.0) as usize;
                let ma20 = crate::recommender::indicators::sma(&cs, ma_period)?;
                let ma_mult = read_f64(vars, "val_short_ma_mult", 1.005);
                if price > ma20 * ma_mult {
                    return None;
                }
                let mut r = Vec::new();
                if pe > 0.0 {
                    r.push(format!("PE {:.1} 估值偏低", pe));
                }
                if pb > 0.0 {
                    r.push(format!("PB {:.2}", pb));
                }
                r.push(format!("回踩 MA{} {:.2}", ma_period, ma20));
                (true, r)
            },
            Period::Mid => {
                let pe_max = read_f64(vars, "val_mid_pe_max", 40.0);
                if pe > 0.0 && pe > pe_max {
                    return None;
                }
                let pb_max = read_f64(vars, "val_mid_pb_max", 8.0);
                if pb > 0.0 && pb > pb_max {
                    return None;
                }
                let mut r = Vec::new();
                if pe > 0.0 {
                    r.push(format!("PE {:.1} 行业中位以下", pe));
                }
                if pb > 0.0 {
                    r.push(format!("PB {:.2}", pb));
                }
                (true, r)
            },
            Period::Long => {
                let pe_max = read_f64(vars, "val_long_pe_max", 35.0);
                if pe > 0.0 && pe > pe_max {
                    return None;
                }
                let pb_max = read_f64(vars, "val_long_pb_max", 6.0);
                if pb > 0.0 && pb > pb_max {
                    return None;
                }
                let mut r = Vec::new();
                if pe > 0.0 {
                    r.push(format!("低 PE {:.1}", pe));
                }
                if pb > 0.0 {
                    r.push(format!("低 PB {:.2}", pb));
                }
                (true, r)
            },
        };

        if !pre_filter_ok {
            return None;
        }

        let (entry_low, entry_high, stop_loss, target, base_position) = match self.period {
            Period::UltraShort => {
                let el = read_f64(vars, "val_ultra_short_entry_low", 0.998);
                let eh = read_f64(vars, "val_ultra_short_entry_high", 1.005);
                let sl = read_f64(vars, "val_ultra_short_stop", 0.97);
                let tg = read_f64(vars, "val_ultra_short_target", 1.03);
                let bp = read_f64(vars, "val_ultra_short_base_pos", 3.0);
                (price * el, price * eh, price * sl, price * tg, bp)
            },
            Period::Short => {
                let el = read_f64(vars, "val_short_entry_low", 0.98);
                let eh = read_f64(vars, "val_short_entry_high", 1.02);
                let sl = read_f64(vars, "val_short_stop", 0.93);
                let tg = read_f64(vars, "val_short_target", 1.10);
                let bp = read_f64(vars, "val_short_base_pos", 5.0);
                (price * el, price * eh, price * sl, price * tg, bp)
            },
            Period::Mid => {
                let el = read_f64(vars, "val_mid_entry_low", 0.95);
                let eh = read_f64(vars, "val_mid_entry_high", 1.05);
                let sl = read_f64(vars, "val_mid_stop", 0.88);
                let tg = read_f64(vars, "val_mid_target", 1.20);
                let bp = read_f64(vars, "val_mid_base_pos", 8.0);
                (price * el, price * eh, price * sl, price * tg, bp)
            },
            Period::Long => {
                let el = read_f64(vars, "val_long_entry_low", 0.93);
                let eh = read_f64(vars, "val_long_entry_high", 1.05);
                let sl = read_f64(vars, "val_long_stop", 0.85);
                let tg = read_f64(vars, "val_long_target", 1.30);
                let bp = read_f64(vars, "val_long_base_pos", 10.0);
                (price * el, price * eh, price * sl, price * tg, bp)
            },
        };

        // 长线要求股价在 60 日均线之上（趋势过滤）
        if matches!(self.period, Period::Long) {
            let long_kline_limit = read_f64(vars, "val_long_kline_limit", 70.0) as u32;
            if let Ok(klines) = client.get_klines(code, "daily", long_kline_limit).await {
                let ma_period = read_f64(vars, "val_long_ma_period", 60.0) as usize;
                if let Some(ma60) = crate::recommender::indicators::sma(
                    &klines.iter().map(|k| k.close).collect::<Vec<_>>(),
                    ma_period,
                ) {
                    let ma60_mult = read_f64(vars, "val_long_ma60_mult", 0.90);
                    if price < ma60 * ma60_mult {
                        return None;
                    }
                    reasons.push(format!("站上 MA{} {:.2}", ma_period, ma60));
                }
            }
        }

        let conf = calc_confidence(
            read_f64(vars, "val_conf_consistency", 0.75),
            read_f64(vars, "val_conf_signal", 0.7),
            read_f64(vars, "val_conf_direction", 0.6),
            read_f64(vars, "val_conf_market", 0.0),
            1.0, // turnover_anomaly: 价值策略无量比数据，默认无惩罚
        );
        let position = calc_position(base_position, conf, self.period);

        let _risk = if matches!(self.period, Period::UltraShort) {
            "超短线估值博弈，次日即需监控"
        } else {
            "行业基本面恶化"
        };

        Some(RecoPick {
            stock_code: code.into(),
            stock_name: name.into(),
            sector,
            style: Style::Value,
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
            risk_notes: vec![_risk.to_string()],
            secondary_styles: vec![],
            synthetic: false,
        })
    }
}

#[async_trait]
impl RecommendStrategy for ValueStrategy {
    fn id(&self) -> &'static str {
        match self.period {
            Period::UltraShort => "value_ultra_short",
            Period::Short => "value_short",
            Period::Mid => "value_mid",
            Period::Long => "value_long",
        }
    }
    fn style(&self) -> Style {
        Style::Value
    }
    fn period(&self) -> Period {
        self.period
    }
    fn required_vendors(&self) -> &'static [&'static str] {
        &["eastmoney", "ths", "akshare"]
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
    fn value_strategy_ids() {
        assert_eq!(ValueStrategy::ultra_short().id(), "value_ultra_short");
        assert_eq!(ValueStrategy::short().id(), "value_short");
        assert_eq!(ValueStrategy::mid().id(), "value_mid");
        assert_eq!(ValueStrategy::long().id(), "value_long");
        assert_eq!(ValueStrategy::long().style(), Style::Value);
    }
}
