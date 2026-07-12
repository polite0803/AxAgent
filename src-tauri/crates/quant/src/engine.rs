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
use crate::metrics::{annualized, max_drawdown, sharpe_ratio};
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
        // Strategy trait 已下沉到 harness，on_init 返回 Result<_, AxAgentError>，
        // 用 map_err 转回 QuantError 保持 BacktestEngine::run 的错误类型不变
        strategy.on_init(&mut ctx).await.map_err(|e| QuantError::Strategy(e.to_string()))?;

        // 4. 主事件循环
        //    P0-4 修复：信号在 bar 收盘后产生，缓存到 pending_signals，
        //    在下一根 bar 的 open 撮合（避免用同根 bar 的 open 成交造成 time-travel bias）
        let mut pending_signals: Vec<Signal> = Vec::new();
        for bar in bars {
            // 4.1 跨日：结算昨日权益点
            if !prev_date.is_empty() && bar.date != prev_date {
                push_equity_point(&mut ctx, &prev_date);
            }
            prev_date = bar.date.clone();
            ctx.current_date = bar.date.clone();

            // 4.2 push bar 到 history
            ctx.bar_history.entry(bar.code.clone()).or_default().push(bar.clone());

            // 4.3 更新 last_price / market_value / unrealized_pnl
            if let Some(pos) = ctx.positions.get_mut(&bar.code) {
                pos.last_price = bar.close;
                pos.market_value = bar.close * pos.quantity as f64;
                pos.unrealized_pnl = (bar.close - pos.cost_basis) * pos.quantity as f64;
            }

            // 4.4 先撮合上一根 bar 产生的 pending signals（用当前 bar 的 open 成交）
            //    OrderType::Market 文档约定"下一根 K 线开盘价成交"，此处兑现该约定
            if !pending_signals.is_empty() {
                // 多标的场景下各 code 独立推进，只撮合与当前 bar.code 匹配的信号
                let (to_match, remaining): (Vec<Signal>, Vec<Signal>) =
                    pending_signals.drain(..).partition(|s| s.code == bar.code);
                pending_signals = remaining;
                for sig in to_match {
                    let pos = ctx.positions.get(&bar.code);
                    let order =
                        signal_to_order(&sig, &bar, ctx.cash, pos.map(|p| p.quantity).unwrap_or(0));
                    let fill = self.matcher.match_order(order, &bar, pos, ctx.cash);
                    all_fills.push(fill.clone());
                    if fill.matched {
                        apply_fill(&mut ctx, &fill);
                    }
                }
            }

            // 4.5 调策略（基于当前已收盘 bar 产生信号）
            let signals = strategy
                .on_bar(&bar, &mut ctx)
                .await
                .map_err(|e| QuantError::Strategy(e.to_string()))?;
            all_signals.extend(signals.iter().cloned());

            // 4.6 信号缓存到 pending_signals，下一根 bar 开盘时撮合
            pending_signals.extend(signals);
        }

        // 4.7 循环结束后，剩余 pending_signals 无下一根 bar 可成交，跳过并记录 warn
        //     （用 close 撮合会重新引入 time-travel bias，故不采用）
        if !pending_signals.is_empty() {
            tracing::warn!(
                count = pending_signals.len(),
                "回测结束时有未撮合的 pending signals（最后一根 bar 产生），已跳过"
            );
        }

        // 5. 收尾权益点
        if !prev_date.is_empty() {
            push_equity_point(&mut ctx, &prev_date);
        }

        // 6. on_finish
        strategy.on_finish(&mut ctx).await.map_err(|e| QuantError::Strategy(e.to_string()))?;

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

fn signal_to_order(sig: &Signal, bar: &Bar, cash: f64, position_qty: u64) -> Order {
    let side = match sig.action {
        SignalAction::Buy => Side::Long,
        SignalAction::Sell => Side::Short,
        SignalAction::Hold => Side::Flat,
    };
    let quantity = if matches!(sig.action, SignalAction::Hold) {
        0
    } else {
        match side {
            Side::Long => {
                // 根据可用资金计算可买股数（取 95% 资金，向下取整到整手）
                let lot_size = 100u64;
                let max_shares = if bar.close > 0.0 {
                    ((cash * 0.95) / bar.close) as u64
                } else {
                    0
                };
                // 当 close == 0 或资金不足以买 1 手时直接返回 0，避免 max(lot_size) 强制最少 1 手
                if max_shares < lot_size {
                    0
                } else {
                    let rounded = (max_shares / lot_size) * lot_size;
                    rounded.min(10_000) // 最多 1 万
                }
            },
            Side::Short => {
                // 卖出全部持仓
                position_qty
            },
            _ => 0,
        }
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
    // P0-3 修复：在状态变更前计算 realized_for_trade，避免 pos.quantity 被减少后恒为 0
    let mut realized_for_trade: f64 = 0.0;
    match order.side {
        Side::Long => {
            let cost = fill.fill_amount + fill.commission + fill.stamp_tax;
            ctx.cash -= cost;
            ctx.commission_paid += fill.commission;
            ctx.stamp_tax_paid += fill.stamp_tax;
            ctx.slippage_paid += fill.slippage;
            let pos = ctx.positions.entry(order.code.clone()).or_insert_with(|| Position {
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
            // 修复 H1: 成本基准须含买入佣金/印花税（每股分摊），否则后续 realized_pnl 偏高
            let buy_fee_per_share = if order.quantity > 0 {
                (fill.commission + fill.stamp_tax) / order.quantity as f64
            } else {
                0.0
            };
            pos.cost_basis = if new_qty > 0 {
                (pos.cost_basis * pos.quantity as f64
                    + (fill.fill_price + buy_fee_per_share) * order.quantity as f64)
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
                // 修复 H1: realized 须扣卖出佣金/印花税（每股分摊），否则净亏可能算盈利
                let sell_fee_per_share = if sell_qty > 0 {
                    (fill.commission + fill.stamp_tax) / sell_qty as f64
                } else {
                    0.0
                };
                let realized =
                    (fill.fill_price - sell_fee_per_share - pos.cost_basis) * sell_qty as f64;
                // 在 pos.quantity 变更前保存，供 Trade 记录使用
                realized_for_trade = realized;
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
    let (max_dd, max_dd_pct) = max_drawdown(&ctx.equity_curve);
    // 修复 H2: 统一使用 A 股 244 交易日年化口径，对齐 metrics.rs 的 A_SHARE_TRADING_DAYS_PER_YEAR
    let sharpe =
        sharpe_ratio(&ctx.equity_curve, 0.025, crate::metrics::A_SHARE_TRADING_DAYS_PER_YEAR);
    let annualized = annualized(&ctx.equity_curve, crate::metrics::A_SHARE_TRADING_DAYS_PER_YEAR);
    (total_return, sharpe, max_dd, max_dd_pct, annualized)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(date: &str, equity: f64) -> EquityPoint {
        EquityPoint { date: date.to_string(), equity, cash: equity, position_value: 0.0 }
    }

    #[test]
    fn compute_sharpe_short_curve_returns_zero_no_panic() {
        // 单点：直接返回 0
        let single = vec![ep("2026-01-01", 100.0)];
        assert_eq!(sharpe_ratio(&single, 0.025, 252.0), 0.0);
        // 两点：rets.len()==1，样本方差需 n≥2，应返回 0 而非除零 panic
        let two = vec![ep("2026-01-01", 100.0), ep("2026-01-02", 101.0)];
        assert_eq!(sharpe_ratio(&two, 0.025, 252.0), 0.0);
    }
}
