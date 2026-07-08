//! 候选池兜底策略：仅依赖 `get_quote`，永远能出 picks
//!
//! 当 4 个主策略（trend/value/capital/reversion）因网络/数据不可用全部返回空时，
//! 这个策略保证面板至少展示 seed pool 的"系统初筛"列表，附带基于现价的合成
//! 入场/止损/目标位（按周期调整振幅）。
//!
//! 不做任何技术信号判定，所以"confidence"和"position"都给得保守（低 0.5 / 0.4 base），
//! 真实仓位由用户在账户页面自己分配。

use super::super::strategy::{read_f64, PerCodeLocks, RecoContext, RecommendStrategy};
use crate::recommender::pool::SeedItem;
use crate::recommender::scoring::{calc_confidence, calc_position};
use crate::recommender::types::{Period, RecoPick, Style};
use async_trait::async_trait;
use axagent_astock_data::AStockClient;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct WatchlistStrategy {
    pub period: Period,
}

impl WatchlistStrategy {
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
        // 只 fetch 实时 quote，不依赖 K 线 / 财务 / 资金流，确保尽量总有数据
        let quote = client.get_quote(code).await.ok()?;
        if quote.is_st {
            return None;
        }
        if quote.price <= 0.0 {
            return None;
        }
        let price = quote.price;

        // 按周期给不同的振幅
        let (entry_low, entry_high, stop_loss, target_price, base_position, holding_days, reason) =
            match self.period {
                Period::UltraShort => {
                    let el = read_f64(vars, "wl_ultra_short_entry_low", 0.998);
                    let eh = read_f64(vars, "wl_ultra_short_entry_high", 1.005);
                    let sl = read_f64(vars, "wl_ultra_short_stop", 0.98);
                    let tg = read_f64(vars, "wl_ultra_short_target", 1.03);
                    let bp = read_f64(vars, "wl_ultra_short_base_pos", 2.0);
                    let hd = read_f64(vars, "wl_ultra_short_holding_days", 2.0) as u32;
                    (
                        price * el,
                        price * eh,
                        price * sl,
                        price * tg,
                        bp,
                        hd,
                        "候选池初筛（超短线：-0.2% 进场，+3% 目标）",
                    )
                },
                Period::Short => {
                    let el = read_f64(vars, "wl_short_entry_low", 0.99);
                    let eh = read_f64(vars, "wl_short_entry_high", 1.01);
                    let sl = read_f64(vars, "wl_short_stop", 0.96);
                    let tg = read_f64(vars, "wl_short_target", 1.06);
                    let bp = read_f64(vars, "wl_short_base_pos", 4.0);
                    let hd = read_f64(vars, "wl_short_holding_days", 5.0) as u32;
                    (
                        price * el,
                        price * eh,
                        price * sl,
                        price * tg,
                        bp,
                        hd,
                        "候选池初筛（短线：日内-1% 进场，+6% 目标）",
                    )
                },
                Period::Mid => {
                    let el = read_f64(vars, "wl_mid_entry_low", 0.97);
                    let eh = read_f64(vars, "wl_mid_entry_high", 1.03);
                    let sl = read_f64(vars, "wl_mid_stop", 0.92);
                    let tg = read_f64(vars, "wl_mid_target", 1.15);
                    let bp = read_f64(vars, "wl_mid_base_pos", 6.0);
                    let hd = read_f64(vars, "wl_mid_holding_days", 28.0) as u32;
                    (
                        price * el,
                        price * eh,
                        price * sl,
                        price * tg,
                        bp,
                        hd,
                        "候选池初筛（中线：-3% 进场，+15% 目标）",
                    )
                },
                Period::Long => {
                    let el = read_f64(vars, "wl_long_entry_low", 0.95);
                    let eh = read_f64(vars, "wl_long_entry_high", 1.05);
                    let sl = read_f64(vars, "wl_long_stop", 0.88);
                    let tg = read_f64(vars, "wl_long_target", 1.25);
                    let bp = read_f64(vars, "wl_long_base_pos", 8.0);
                    let hd = read_f64(vars, "wl_long_holding_days", 90.0) as u32;
                    (
                        price * el,
                        price * eh,
                        price * sl,
                        price * tg,
                        bp,
                        hd,
                        "候选池初筛（长线：-5% 进场，+25% 目标）",
                    )
                },
            };

        // 信心度低（0.55），因为没有技术信号支撑
        let conf = calc_confidence(
            read_f64(vars, "wl_conf_consistency", 0.55),
            read_f64(vars, "wl_conf_signal", 0.5),
            read_f64(vars, "wl_conf_direction", 0.5),
            read_f64(vars, "wl_conf_market", 0.0),
            read_f64(vars, "wl_conf_base", 1.0),
        );
        let position = calc_position(base_position, conf, self.period);

        Some(RecoPick {
            stock_code: code.into(),
            stock_name: name.into(),
            sector,
            style: Style::Watchlist,
            period: self.period,
            price,
            entry_low,
            entry_high,
            stop_loss,
            target_price,
            position_pct: position,
            holding_days,
            confidence: conf,
            reasons: vec![reason.to_string()],
            risk_notes: vec![
                "初筛列表，无技术信号，请结合其他风格确认".to_string(),
                "若数据源恢复可重新拉取获取更准确入场区间".to_string(),
            ],
            secondary_styles: vec![],
            synthetic: true,
        })
    }
}

#[async_trait]
impl RecommendStrategy for WatchlistStrategy {
    fn id(&self) -> &'static str {
        match self.period {
            Period::UltraShort => "watchlist_ultra_short",
            Period::Short => "watchlist_short",
            Period::Mid => "watchlist_mid",
            Period::Long => "watchlist_long",
        }
    }
    fn style(&self) -> Style {
        Style::Watchlist
    }
    fn period(&self) -> Period {
        self.period
    }
    fn required_vendors(&self) -> &'static [&'static str] {
        // 任意实时 quote vendor 即可
        &["tencent", "eastmoney", "mootdx"]
    }

    async fn scan(&self, ctx: &RecoContext<'_>) -> Result<Vec<RecoPick>, String> {
        let mut picks = Vec::new();
        // Watchlist 仅作为 fallback，扫描前 30 只即可满足 10 个 pick 上限
        // 限制扫描范围减少冗余 quote API 调用，降低 vendor 压力
        for (code, name, sector) in ctx.seed.iter().take(30) {
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
    fn watchlist_strategy_ids() {
        assert_eq!(WatchlistStrategy::short().id(), "watchlist_short");
        assert_eq!(WatchlistStrategy::mid().id(), "watchlist_mid");
        assert_eq!(WatchlistStrategy::long().id(), "watchlist_long");
        assert_eq!(WatchlistStrategy::short().style(), Style::Watchlist);
    }
}

/// 数据稀疏兜底：为指定 style 用 `get_quote` 拉取基础行情 emit 合成 picks
///
/// 调用场景：vendor 的 K 线 / 财务 / 资金流数据全不可用时，4 个主策略
/// (trend/value/capital/reversion) 都返回空，导致面板上对应 style 桶为空。
/// 该函数保证 5 个 style 桶都有数据展示，避免出现"只有 Watchlist 有 10 条，
/// 其他 4 个全空"的局面。
///
/// 合成 pick 的 `reasons` 明确标注"信号缺失，按现价合成"，方便用户区分
/// 真实信号 vs 兜底。`confidence` 偏低（0.45），让真实 pick 排序优先。
pub async fn emit_synthetic_picks(
    client: Arc<AStockClient>,
    style: Style,
    period: Period,
    raw_seed: &[SeedItem],
    per_code_locks: Arc<PerCodeLocks>,
    vars: &HashMap<String, Value>,
) -> Vec<RecoPick> {
    let mut picks = Vec::new();
    // 合成兜底只需前 30 只即可满足 10 个 pick 上限
    for (code, name, sector) in raw_seed.iter().take(30) {
        let _g = per_code_locks.lock_for(code).await;
        if let Some(p) =
            scan_synthetic_one(client.as_ref(), code, name, sector.clone(), style, period, vars)
                .await
        {
            picks.push(p);
        }
    }
    picks
}

async fn scan_synthetic_one(
    client: &AStockClient,
    code: &str,
    name: &str,
    sector: Option<String>,
    style: Style,
    period: Period,
    vars: &HashMap<String, Value>,
) -> Option<RecoPick> {
    let quote = client.get_quote(code).await.ok()?;
    if quote.is_st {
        return None;
    }
    if quote.price <= 0.0 {
        return None;
    }
    let price = quote.price;

    let style_label = match style {
        Style::Trend => "趋势跟踪",
        Style::Value => "价值低估",
        Style::Capital => "资金驱动",
        Style::Reversion => "超跌反弹",
        Style::Watchlist => "系统初筛",
        Style::Serenity => "瓶颈掘金（Serenity 合成）",
    };

    let (entry_low, entry_high, stop_loss, target_price, base_position, holding_days, reason) =
        match period {
            Period::UltraShort => {
                let el = read_f64(vars, "syn_ultra_short_entry_low", 0.998);
                let eh = read_f64(vars, "syn_ultra_short_entry_high", 1.005);
                let sl = read_f64(vars, "syn_ultra_short_stop", 0.98);
                let tg = read_f64(vars, "syn_ultra_short_target", 1.03);
                let bp = read_f64(vars, "syn_ultra_short_base_pos", 2.0);
                let hd = read_f64(vars, "syn_ultra_short_holding_days", 2.0) as u32;
                (
                    price * el,
                    price * eh,
                    price * sl,
                    price * tg,
                    bp,
                    hd,
                    format!("候选池初筛（超短线 — {} 信号缺失，按现价合成）", style_label),
                )
            },
            Period::Short => {
                let el = read_f64(vars, "syn_short_entry_low", 0.99);
                let eh = read_f64(vars, "syn_short_entry_high", 1.01);
                let sl = read_f64(vars, "syn_short_stop", 0.96);
                let tg = read_f64(vars, "syn_short_target", 1.06);
                let bp = read_f64(vars, "syn_short_base_pos", 4.0);
                let hd = read_f64(vars, "syn_short_holding_days", 5.0) as u32;
                (
                    price * el,
                    price * eh,
                    price * sl,
                    price * tg,
                    bp,
                    hd,
                    format!("候选池初筛（短线 — {} 信号缺失，按现价合成）", style_label),
                )
            },
            Period::Mid => {
                let el = read_f64(vars, "syn_mid_entry_low", 0.97);
                let eh = read_f64(vars, "syn_mid_entry_high", 1.03);
                let sl = read_f64(vars, "syn_mid_stop", 0.92);
                let tg = read_f64(vars, "syn_mid_target", 1.15);
                let bp = read_f64(vars, "syn_mid_base_pos", 6.0);
                let hd = read_f64(vars, "syn_mid_holding_days", 28.0) as u32;
                (
                    price * el,
                    price * eh,
                    price * sl,
                    price * tg,
                    bp,
                    hd,
                    format!("候选池初筛（中线 — {} 信号缺失，按现价合成）", style_label),
                )
            },
            Period::Long => {
                let el = read_f64(vars, "syn_long_entry_low", 0.95);
                let eh = read_f64(vars, "syn_long_entry_high", 1.05);
                let sl = read_f64(vars, "syn_long_stop", 0.88);
                let tg = read_f64(vars, "syn_long_target", 1.25);
                let bp = read_f64(vars, "syn_long_base_pos", 8.0);
                let hd = read_f64(vars, "syn_long_holding_days", 90.0) as u32;
                (
                    price * el,
                    price * eh,
                    price * sl,
                    price * tg,
                    bp,
                    hd,
                    format!("候选池初筛（长线 — {} 信号缺失，按现价合成）", style_label),
                )
            },
        };

    // 信心度低（0.45），比 Watchlist 真实初筛（0.55）更低，方便排序时真实 pick 优先
    let conf = calc_confidence(
        read_f64(vars, "syn_conf_consistency", 0.45),
        read_f64(vars, "syn_conf_signal", 0.4),
        read_f64(vars, "syn_conf_direction", 0.4),
        read_f64(vars, "syn_conf_market", 0.0),
        read_f64(vars, "syn_conf_base", 1.0),
    );
    let position = calc_position(base_position, conf, period);

    Some(RecoPick {
        stock_code: code.into(),
        stock_name: name.into(),
        sector,
        style,
        period,
        price,
        entry_low,
        entry_high,
        stop_loss,
        target_price,
        position_pct: position,
        holding_days,
        confidence: conf,
        reasons: vec![reason],
        risk_notes: vec![
            format!(
                "{} 风格专属信号缺失（K 线 / 财务 / 资金数据不可用），本条为按现价合成的兜底结果",
                style_label
            ),
            "数据源恢复后可重新拉取获取更准确入场区间".to_string(),
        ],
        secondary_styles: vec![],
        synthetic: true,
    })
}
