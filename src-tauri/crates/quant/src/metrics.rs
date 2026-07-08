//! 完整绩效指标
//!
//! 提供独立于 BacktestEngine 的指标计算：
//! - `MetricsReport::from_equity_curve()`: 纯函数（接受 equity + trades）
//! - `MetricsReport::from_backtest_result()`: 从 BacktestResult 重建
//!
//! ## 覆盖
//!
//! M1 实现：total_return, annualized_return, volatility, sharpe, sortino,
//!          max_drawdown (金额 + 百分比 + 持续天数), win_rate, profit_factor,
//!          avg_win, avg_loss, payoff_ratio
//!
//! M2 阶段：Calmar ratio, IC, IR, alpha, beta (相对基准)

use serde::{Deserialize, Serialize};

use crate::ctx::{EquityPoint, Trade};
use crate::engine::BacktestResult;

/// 完整绩效报告
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsReport {
    /// 总收益率（区间累计）
    pub total_return: f64,
    /// 年化收益率
    pub annualized_return: f64,
    /// 年化波动率
    pub annualized_volatility: f64,
    /// Sharpe ratio（年化）
    pub sharpe: f64,
    /// Sortino ratio（年化，仅下行风险）
    pub sortino: f64,
    /// 最大回撤（金额）
    pub max_drawdown: f64,
    /// 最大回撤（百分比）
    pub max_drawdown_pct: f64,
    /// 最大回撤持续天数
    pub max_drawdown_duration_days: i64,
    /// 胜率
    pub win_rate: f64,
    /// 盈亏比（总盈利 / |总亏损|）
    pub profit_factor: f64,
    /// 平均盈利（仅盈利平仓）
    pub avg_win: f64,
    /// 平均亏损（仅亏损平仓）
    pub avg_loss: f64,
    /// 赔率（avg_win / |avg_loss|）
    pub payoff_ratio: f64,
    /// 总成交笔数（含开仓 + 平仓）
    pub total_trades: usize,
    /// 盈利平仓数
    pub winning_trades: usize,
    /// 亏损平仓数
    pub losing_trades: usize,
    /// 平均持仓天数（M2 完善）
    pub avg_holding_days: f64,
    /// Calmar ratio = annualized_return / |max_drawdown_pct|（M2）
    pub calmar: Option<f64>,
    /// 信息系数（M2，需预测序列 vs 实际收益）
    pub ic: Option<f64>,
    /// 信息比率（M2）
    pub ir: Option<f64>,
}

impl MetricsReport {
    /// 从权益曲线 + 成交记录构建完整指标
    pub fn from_equity_curve(
        curve: &[EquityPoint],
        trades: &[Trade],
        risk_free_annual: f64,
        trading_days_per_year: f64,
    ) -> Self {
        let total_return = total_return(curve);
        let annualized_return = annualized(curve, trading_days_per_year);
        let annualized_volatility = annual_volatility(curve, trading_days_per_year);
        let sharpe = sharpe_ratio(curve, risk_free_annual, trading_days_per_year);
        let sortino = sortino_ratio(curve, risk_free_annual, trading_days_per_year);
        let (max_dd, max_dd_pct) = max_drawdown(curve);
        let max_dd_duration = max_drawdown_duration(curve);
        let (win_rate, profit_factor, avg_win, avg_loss, payoff_ratio) = trade_stats(trades);
        let winning = count_winning(trades);
        let losing = count_losing(trades);

        Self {
            total_return,
            annualized_return,
            annualized_volatility,
            sharpe,
            sortino,
            max_drawdown: max_dd,
            max_drawdown_pct: max_dd_pct,
            max_drawdown_duration_days: max_dd_duration,
            win_rate,
            profit_factor,
            avg_win,
            avg_loss,
            payoff_ratio,
            total_trades: trades.len(),
            winning_trades: winning,
            losing_trades: losing,
            avg_holding_days: 0.0, // M2 完善
            calmar: None,
            ic: None,
            ir: None,
        }
    }

    /// 从 BacktestResult 构建
    pub fn from_backtest_result(result: &BacktestResult, risk_free_annual: f64) -> Self {
        Self::from_equity_curve(&result.equity_curve, &result.trades, risk_free_annual, 252.0)
    }
}

// ===================== 指标实现 =====================

fn total_return(curve: &[EquityPoint]) -> f64 {
    if curve.len() < 2 {
        return 0.0;
    }
    let first = curve.first().unwrap().equity;
    let last = curve.last().unwrap().equity;
    if first <= 0.0 {
        return 0.0;
    }
    (last - first) / first
}

fn daily_returns(curve: &[EquityPoint]) -> Vec<f64> {
    let mut rets = Vec::with_capacity(curve.len().saturating_sub(1));
    for w in curve.windows(2) {
        let prev = w[0].equity;
        let cur = w[1].equity;
        if prev > 0.0 {
            rets.push((cur - prev) / prev);
        }
    }
    rets
}

fn approx_days(start: &str, end: &str) -> i64 {
    use chrono::NaiveDate;
    let s = NaiveDate::parse_from_str(start, "%Y-%m-%d").ok();
    let e = NaiveDate::parse_from_str(end, "%Y-%m-%d").ok();
    match (s, e) {
        (Some(s), Some(e)) => (e - s).num_days(),
        _ => 0,
    }
}

fn annualized(curve: &[EquityPoint], days_per_year: f64) -> f64 {
    if curve.len() < 2 || days_per_year <= 0.0 {
        return 0.0;
    }
    let first = &curve.first().unwrap().date;
    let last = &curve.last().unwrap().date;
    let days = approx_days(first, last);
    if days <= 0 {
        return 0.0;
    }
    let tr = total_return(curve);
    if (1.0 + tr) <= 0.0 {
        return 0.0;
    }
    let years = days as f64 / days_per_year;
    (1.0 + tr).powf(1.0 / years) - 1.0
}

fn annual_volatility(curve: &[EquityPoint], days_per_year: f64) -> f64 {
    let rets = daily_returns(curve);
    if rets.is_empty() {
        return 0.0;
    }
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rets.len() as f64;
    var.sqrt() * days_per_year.sqrt()
}

fn sharpe_ratio(curve: &[EquityPoint], risk_free_annual: f64, days_per_year: f64) -> f64 {
    let rets = daily_returns(curve);
    if rets.is_empty() {
        return 0.0;
    }
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rets.len() as f64;
    let std = var.sqrt();
    if std < 1e-10 {
        return 0.0;
    }
    let daily_rf = risk_free_annual / days_per_year;
    (mean - daily_rf) / std * days_per_year.sqrt()
}

fn sortino_ratio(curve: &[EquityPoint], risk_free_annual: f64, days_per_year: f64) -> f64 {
    let rets = daily_returns(curve);
    if rets.is_empty() {
        return 0.0;
    }
    let daily_rf = risk_free_annual / days_per_year;
    let excess: Vec<f64> = rets.iter().map(|r| r - daily_rf).collect();
    let mean = excess.iter().sum::<f64>() / excess.len() as f64;
    // 下行方差（仅 r < rf 时计算）
    let downside_var = excess
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|r| r.powi(2))
        .sum::<f64>()
        / excess.len() as f64;
    let downside_std = downside_var.sqrt();
    if downside_std < 1e-10 {
        return 0.0;
    }
    mean / downside_std * days_per_year.sqrt()
}

fn max_drawdown(curve: &[EquityPoint]) -> (f64, f64) {
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

fn max_drawdown_duration(curve: &[EquityPoint]) -> i64 {
    if curve.is_empty() {
        return 0;
    }
    let mut peak_idx = 0;
    let mut max_dur = 0;
    for (i, p) in curve.iter().enumerate() {
        if p.equity > curve[peak_idx].equity {
            peak_idx = i;
        }
        let days = approx_days(&curve[peak_idx].date, &p.date);
        if days > max_dur {
            max_dur = days;
        }
    }
    max_dur
}

fn count_winning(trades: &[Trade]) -> usize {
    trades.iter().filter(|t| t.realized_pnl > 0.0).count()
}

fn count_losing(trades: &[Trade]) -> usize {
    trades.iter().filter(|t| t.realized_pnl < 0.0).count()
}

fn trade_stats(trades: &[Trade]) -> (f64, f64, f64, f64, f64) {
    let mut win_sum = 0.0;
    let mut loss_sum = 0.0;
    let mut win_count = 0;
    let mut loss_count = 0;
    for t in trades {
        if t.realized_pnl > 0.0 {
            win_sum += t.realized_pnl;
            win_count += 1;
        } else if t.realized_pnl < 0.0 {
            loss_sum += t.realized_pnl;
            loss_count += 1;
        }
    }
    let total_closed = win_count + loss_count;
    let win_rate = if total_closed > 0 {
        win_count as f64 / total_closed as f64
    } else {
        0.0
    };
    let profit_factor = if loss_sum < -1e-10 {
        win_sum / (-loss_sum)
    } else {
        0.0
    };
    let avg_win = if win_count > 0 {
        win_sum / win_count as f64
    } else {
        0.0
    };
    let avg_loss = if loss_count > 0 {
        loss_sum / loss_count as f64
    } else {
        0.0
    };
    let payoff_ratio = if avg_loss.abs() > 1e-10 {
        avg_win / avg_loss.abs()
    } else {
        0.0
    };
    (win_rate, profit_factor, avg_win, avg_loss, payoff_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctx::Trade;
    use crate::types::Side;

    fn make_eq(date: &str, equity: f64) -> EquityPoint {
        EquityPoint {
            date: date.to_string(),
            equity,
            cash: equity,
            position_value: 0.0,
        }
    }

    fn make_trade(pnl: f64) -> Trade {
        Trade {
            code: "TEST".to_string(),
            side: Side::Short,
            quantity: 100,
            price: 100.0,
            amount: 10000.0,
            commission: 5.0,
            stamp_tax: 5.0,
            slippage: 0.5,
            timestamp: "2025-01-15".to_string(),
            reason: "test".to_string(),
            realized_pnl: pnl,
        }
    }

    #[test]
    fn test_total_return_uptrend() {
        let curve = vec![
            make_eq("2025-01-01", 1_000_000.0),
            make_eq("2025-01-02", 1_010_000.0),
            make_eq("2025-01-03", 1_050_000.0),
        ];
        let m = MetricsReport::from_equity_curve(&curve, &[], 0.025, 252.0);
        assert!((m.total_return - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_max_drawdown() {
        let curve = vec![
            make_eq("2025-01-01", 100.0),
            make_eq("2025-01-02", 120.0),
            make_eq("2025-01-03", 90.0), // 跌 30 (峰 120)
            make_eq("2025-01-04", 100.0),
        ];
        let (dd, dd_pct) = max_drawdown(&curve);
        assert!((dd - 30.0).abs() < 1e-6);
        assert!(((dd_pct - 0.25).abs()) < 1e-6); // 30/120
    }

    #[test]
    fn test_sharpe_zero_vol() {
        let curve = vec![
            make_eq("2025-01-01", 100.0),
            make_eq("2025-01-02", 100.0),
            make_eq("2025-01-03", 100.0),
        ];
        let m = MetricsReport::from_equity_curve(&curve, &[], 0.025, 252.0);
        assert_eq!(m.sharpe, 0.0);
    }

    #[test]
    fn test_sharpe_positive() {
        let curve = vec![
            make_eq("2025-01-01", 100.0),
            make_eq("2025-01-02", 102.0),
            make_eq("2025-01-03", 104.0),
            make_eq("2025-01-04", 103.0),
            make_eq("2025-01-05", 105.0),
        ];
        let m = MetricsReport::from_equity_curve(&curve, &[], 0.025, 252.0);
        // 收益正波动应有正 sharpe
        assert!(m.sharpe > 0.0);
    }

    #[test]
    fn test_win_rate_profit_factor() {
        let trades = vec![make_trade(100.0), make_trade(-50.0), make_trade(200.0)];
        let (wr, pf, aw, al, pay) = trade_stats(&trades);
        assert!((wr - 2.0 / 3.0).abs() < 1e-6);
        assert!((pf - 300.0 / 50.0).abs() < 1e-6);
        assert!((aw - 150.0).abs() < 1e-6);
        assert!((al - (-50.0)).abs() < 1e-6);
        assert!((pay - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_max_dd_duration() {
        let curve = vec![
            make_eq("2025-01-01", 100.0),
            make_eq("2025-01-15", 120.0), // 新峰
            make_eq("2025-02-01", 80.0),  // 谷底（距峰 17 天）
            make_eq("2025-02-10", 90.0),
        ];
        let dur = max_drawdown_duration(&curve);
        // 1月15到2月10约26天
        assert!(dur > 20);
    }
}
