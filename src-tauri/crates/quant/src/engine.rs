//! BacktestEngine — 回测事件循环
//!
//! ## 主循环流程（每根 bar）
//!
//! 1. push bar 到 `ctx.bar_history[code]`（Engine 职责，策略只读）
//! 2. 更新持仓 `last_price` / `market_value` / `unrealized_pnl`
//! 3. 调 `strategy.on_bar(bar, &mut ctx)` → 0..N 个 Signal
//! 4. Signal → Order → 撮合（Matcher::match_order）
//! 5. 应用 Fill：更新 cash / positions / trades
//! 6. 跨日时记录权益曲线点
//!
//! ## 撮合顺序保证
//!
//! - 同一 bar 的多只股票：按 K 线数组顺序逐个处理
//! - 同一股票的多个 signal：按数组顺序逐个撮合（前一 Order 可能改变持仓，影响后一 Order 的 T+1 校验）
//!
//! ## 关于"美式偷看未来"防御
//!
//! - 策略拿到的 bar 是当前 bar（不含未来）
//! - 撮合价用 `bar.open`（不偷看 close 决定成交）
//! - 限价单校验只用当前 bar H/L（不偷看后续 bar）
//!
//! ## 占位说明
//!
//! - 默认每 signal 数量 100 股（M2 阶段接入 portfolio sizing）
//! - sharpe/max_dd 基础版内联；MetricsReport 完整版在 todo #8

use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::ctx::{EquityPoint, Position, StrategyCtx, Trade};
use crate::error::QuantError;
use crate::matcher::{Matcher, MatcherConfig};
use crate::strategy::Strategy;
use crate::types::{Bar, Fill, Order, OrderType, Side, Signal, SignalAction};

/// 回测配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestConfig {
    /// 初始资金（元）
    pub initial_cash: f64,
    /// 撮合器配置
    pub matcher: MatcherConfig,
    /// 起始日期过滤（含），None 表示不限制
    pub start_date: Option<String>,
    /// 截止日期过滤（含），None 表示不限制
    pub end_date: Option<String>,
    /// 关注的股票代码列表，None/空 表示不限制
    pub codes: Vec<String>,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_cash: 1_000_000.0,
            matcher: MatcherConfig::default(),
            start_date: None,
            end_date: None,
            codes: vec![],
        }
    }
}

/// 回测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestResult {
    pub strategy_name: String,
    pub strategy_version: String,
    pub strategy_params: serde_json::Value,
    pub config: BacktestConfig,
    pub initial_cash: f64,
    pub final_equity: f64,
    pub total_return: f64,
    pub annualized_return: f64,
    pub sharpe: f64,
    pub max_drawdown: f64,
    pub max_drawdown_pct: f64,
    pub win_rate: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub trades: Vec<Trade>,
    pub signals: Vec<Signal>,
    pub fills: Vec<Fill>,
    pub equity_curve: Vec<EquityPoint>,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
}

/// 回测引擎
pub struct BacktestEngine {
    config: BacktestConfig,
    matcher: Matcher,
}

impl BacktestEngine {
    pub fn new(config: BacktestConfig) -> Self {
        let matcher = Matcher::new(config.matcher.clone());
        Self { config, matcher }
    }

    pub fn with_defaults() -> Self {
        Self::new(BacktestConfig::default())
    }

    /// 跑回测
    ///
    /// - `klines`: 已按时间排序的 K 线（单只或多只股票均可）
    /// - `strategy`: 策略实例（Engine 会调用 on_init / on_bar / on_finish）
    pub async fn run(
        &self,
        strategy: &mut dyn Strategy,
        klines: Vec<Bar>,
    ) -> Result<BacktestResult, QuantError> {
        let start = Instant::now();
        let started_at = Utc::now().to_rfc3339();

        // 1. 过滤 + 排序
        let mut bars: Vec<Bar> = klines
            .into_iter()
            .filter(|b| {
                if let Some(s) = &self.config.start_date
                    && b.date.as_str() < s.as_str()
                {
                    return false;
                }
                if let Some(e) = &self.config.end_date
                    && b.date.as_str() > e.as_str()
                {
                    return false;
                }
                if !self.config.codes.is_empty() && !self.config.codes.contains(&b.code) {
                    return false;
                }
                true
            })
            .collect();
        bars.sort_by(|a, b| a.date.cmp(&b.date).then(a.code.cmp(&b.code)));

        // 2. 初始化 ctx
        let mut ctx = StrategyCtx::new(self.config.initial_cash);
        let mut all_signals: Vec<Signal> = Vec::new();
        let mut all_fills: Vec<Fill> = Vec::new();
        let mut prev_date = String::new();

        // 3. on_init
        strategy.on_init(&mut ctx).await?;

        // 4. 主事件循环
        for bar in bars {
            // 4.1 跨日：结算昨日权益点
            if !prev_date.is_empty() && bar.date != prev_date {
                push_equity_point(&mut ctx, &prev_date);
            }
            prev_date = bar.date.clone();
            ctx.current_date = bar.date.clone();

            // 4.2 push bar 到 history
            ctx.bar_history
                .entry(bar.code.clone())
                .or_default()
                .push(bar.clone());

            // 4.3 更新 last_price / market_value / unrealized_pnl
            if let Some(pos) = ctx.positions.get_mut(&bar.code) {
                pos.last_price = bar.close;
                pos.market_value = bar.close * pos.quantity as f64;
                pos.unrealized_pnl = (bar.close - pos.cost_basis) * pos.quantity as f64;
            }

            // 4.4 调策略
            let signals = strategy.on_bar(&bar, &mut ctx).await?;
            all_signals.extend(signals.iter().cloned());

            // 4.5 Signal → Order → 撮合 → 应用 Fill
            for sig in &signals {
                let order = signal_to_order(sig, &bar);
                let pos = ctx.positions.get(&bar.code);
                let fill = self.matcher.match_order(order, &bar, pos, ctx.cash);
                all_fills.push(fill.clone());
                if fill.matched {
                    apply_fill(&mut ctx, &fill);
                }
            }
        }

        // 5. 收尾权益点
        if !prev_date.is_empty() {
            push_equity_point(&mut ctx, &prev_date);
        }

        // 6. on_finish
        strategy.on_finish(&mut ctx).await?;

        // 7. 计算基础指标
        let (total_return, sharpe, max_dd, max_dd_pct, annualized) =
            compute_basic(&ctx, self.config.initial_cash);
        let (winning, losing, win_rate) = compute_win_rate(&ctx.trades);

        let finished_at = Utc::now().to_rfc3339();
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(BacktestResult {
            strategy_name: strategy.name().to_string(),
            strategy_version: strategy.version().to_string(),
            strategy_params: strategy.params(),
            config: self.config.clone(),
            initial_cash: self.config.initial_cash,
            final_equity: ctx.total_equity(),
            total_return,
            annualized_return: annualized,
            sharpe,
            max_drawdown: max_dd,
            max_drawdown_pct: max_dd_pct,
            win_rate,
            total_trades: ctx.trades.len(),
            winning_trades: winning,
            losing_trades: losing,
            trades: ctx.trades,
            signals: all_signals,
            fills: all_fills,
            equity_curve: ctx.equity_curve,
            started_at,
            finished_at,
            duration_ms,
        })
    }
}

fn push_equity_point(ctx: &mut StrategyCtx, date: &str) {
    let position_value: f64 = ctx.positions.values().map(|p| p.market_value).sum();
    ctx.equity_curve.push(EquityPoint {
        date: date.to_string(),
        equity: ctx.cash + position_value,
        cash: ctx.cash,
        position_value,
    });
}

fn signal_to_order(sig: &Signal, bar: &Bar) -> Order {
    let side = match sig.action {
        SignalAction::Buy => Side::Long,
        SignalAction::Sell => Side::Flat,
        SignalAction::Hold => Side::Flat,
    };
    // 占位：M1 阶段每 signal 默认 100 股（最小一手）
    // M2 阶段接入 portfolio sizing（按目标权重 / 凯利公式 / 风险预算）
    let quantity = if matches!(sig.action, SignalAction::Hold) {
        0
    } else {
        100
    };
    Order {
        code: bar.code.clone(),
        side,
        quantity,
        order_type: OrderType::Market,
        timestamp: bar.date.clone(),
        reason: sig.reason.clone(),
    }
}

fn apply_fill(ctx: &mut StrategyCtx, fill: &Fill) {
    let order = &fill.order;
    if order.quantity == 0 {
        return;
    }
    match order.side {
        Side::Long => {
            let cost = fill.fill_amount + fill.commission + fill.stamp_tax;
            ctx.cash -= cost;
            ctx.commission_paid += fill.commission;
            ctx.stamp_tax_paid += fill.stamp_tax;
            ctx.slippage_paid += fill.slippage;
            let pos = ctx
                .positions
                .entry(order.code.clone())
                .or_insert_with(|| Position {
                    code: order.code.clone(),
                    name: None,
                    side: Side::Long,
                    quantity: 0,
                    cost_basis: 0.0,
                    last_price: fill.fill_price,
                    market_value: 0.0,
                    unrealized_pnl: 0.0,
                    realized_pnl: 0.0,
                    entry_date: order.timestamp.clone(),
                    entry_timestamp: order.timestamp.clone(),
                });
            let new_qty = pos.quantity + order.quantity;
            pos.cost_basis = if new_qty > 0 {
                (pos.cost_basis * pos.quantity as f64 + fill.fill_price * order.quantity as f64)
                    / new_qty as f64
            } else {
                0.0
            };
            pos.quantity = new_qty;
            pos.last_price = fill.fill_price;
            pos.market_value = fill.fill_price * new_qty as f64;
            pos.unrealized_pnl = 0.0;
        },
        Side::Short => {
            let proceeds = fill.fill_amount - fill.commission - fill.stamp_tax;
            ctx.cash += proceeds;
            ctx.commission_paid += fill.commission;
            ctx.stamp_tax_paid += fill.stamp_tax;
            ctx.slippage_paid += fill.slippage;
            if let Some(pos) = ctx.positions.get_mut(&order.code) {
                let sell_qty = order.quantity.min(pos.quantity);
                let realized = (fill.fill_price - pos.cost_basis) * sell_qty as f64;
                pos.realized_pnl += realized;
                pos.quantity -= sell_qty;
                if pos.quantity == 0 {
                    pos.cost_basis = 0.0;
                }
                pos.last_price = fill.fill_price;
                pos.market_value = fill.fill_price * pos.quantity as f64;
                pos.unrealized_pnl = 0.0;
                ctx.realized_pnl += realized;
            }
        },
        Side::Flat => {},
    }
    let realized_for_trade = if matches!(order.side, Side::Short) {
        if let Some(pos) = ctx.positions.get(&order.code) {
            (fill.fill_price - pos.cost_basis) * order.quantity.min(pos.quantity) as f64
        } else {
            0.0
        }
    } else {
        0.0
    };
    ctx.trades.push(Trade {
        code: order.code.clone(),
        side: order.side,
        quantity: order.quantity,
        price: fill.fill_price,
        amount: fill.fill_amount,
        commission: fill.commission,
        stamp_tax: fill.stamp_tax,
        slippage: fill.slippage,
        timestamp: fill.timestamp.clone(),
        reason: order.reason.clone(),
        realized_pnl: realized_for_trade,
    });
}

fn compute_basic(ctx: &StrategyCtx, initial_cash: f64) -> (f64, f64, f64, f64, f64) {
    let final_equity = ctx.total_equity();
    let total_return = if initial_cash > 0.0 {
        (final_equity - initial_cash) / initial_cash
    } else {
        0.0
    };
    let (max_dd, max_dd_pct) = compute_max_drawdown(&ctx.equity_curve);
    let sharpe = compute_sharpe(&ctx.equity_curve, 0.025);
    let annualized = compute_annualized(&ctx.equity_curve, initial_cash);
    (total_return, sharpe, max_dd, max_dd_pct, annualized)
}

fn compute_max_drawdown(curve: &[EquityPoint]) -> (f64, f64) {
    let mut peak = f64::MIN;
    let mut max_dd = 0.0;
    let mut max_dd_pct = 0.0;
    for p in curve {
        if p.equity > peak {
            peak = p.equity;
        }
        if peak > 0.0 {
            let dd = peak - p.equity;
            let dd_pct = dd / peak;
            if dd > max_dd {
                max_dd = dd;
            }
            if dd_pct > max_dd_pct {
                max_dd_pct = dd_pct;
            }
        }
    }
    (max_dd, max_dd_pct)
}

fn compute_sharpe(curve: &[EquityPoint], risk_free_annual: f64) -> f64 {
    if curve.len() < 2 {
        return 0.0;
    }
    let mut rets: Vec<f64> = Vec::with_capacity(curve.len() - 1);
    for w in curve.windows(2) {
        let prev = w[0].equity;
        let cur = w[1].equity;
        if prev > 0.0 {
            rets.push((cur - prev) / prev);
        }
    }
    if rets.is_empty() {
        return 0.0;
    }
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rets.len() as f64;
    let std = var.sqrt();
    if std < 1e-10 {
        return 0.0;
    }
    let daily_rf = risk_free_annual / 252.0;
    (mean - daily_rf) / std * (252.0_f64).sqrt()
}

fn compute_annualized(curve: &[EquityPoint], initial_cash: f64) -> f64 {
    if curve.len() < 2 || initial_cash <= 0.0 {
        return 0.0;
    }
    let final_eq = curve.last().unwrap().equity;
    let total_return = (final_eq - initial_cash) / initial_cash;
    let first = &curve.first().unwrap().date;
    let last = &curve.last().unwrap().date;
    let days = approx_days_between(first, last);
    if days <= 0 {
        return 0.0;
    }
    let years = days as f64 / 365.0;
    if (1.0 + total_return) <= 0.0 {
        return 0.0;
    }
    (1.0 + total_return).powf(1.0 / years) - 1.0
}

fn approx_days_between(start: &str, end: &str) -> i64 {
    use chrono::NaiveDate;
    let s = NaiveDate::parse_from_str(start, "%Y-%m-%d").ok();
    let e = NaiveDate::parse_from_str(end, "%Y-%m-%d").ok();
    match (s, e) {
        (Some(s), Some(e)) => (e - s).num_days(),
        _ => 0,
    }
}

fn compute_win_rate(trades: &[Trade]) -> (usize, usize, f64) {
    let mut winning = 0;
    let mut losing = 0;
    for t in trades {
        if t.realized_pnl > 0.0 {
            winning += 1;
        } else if t.realized_pnl < 0.0 {
            losing += 1;
        }
    }
    let total = winning + losing;
    let win_rate = if total > 0 {
        winning as f64 / total as f64
    } else {
        0.0
    };
    (winning, losing, win_rate)
}
