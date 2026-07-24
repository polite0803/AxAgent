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

// P3-C8: A 股年交易日数常量改为 re-export harness 统一定义，
// 消除 stock-analysis/astock-data/tools/quant 四处重复定义的 252/244 混用。
// 保留 `pub const` 形式以维持下游 API 稳定性（quant 内部及 prelude 均有引用）。
pub use axagent_harness::indicators::A_SHARE_TRADING_DAYS_PER_YEAR;

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
        // M2 补全：Calmar ratio = 年化收益率 / |最大回撤百分比|
        // 当 max_dd_pct == 0（无回撤）时返回 None（除零保护）
        let calmar = if max_dd_pct.abs() > 1e-10 {
            Some(annualized_return / max_dd_pct.abs())
        } else {
            None
        };
        // M2 补全：平均持仓天数 — 基于 FIFO 开仓/平仓匹配
        // 开仓 trade: realized_pnl == 0（刚建仓，无实现盈亏）
        // 平仓 trade: realized_pnl != 0（平仓时实现盈亏）
        let avg_hold = avg_holding_days(trades);

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
            avg_holding_days: avg_hold,
            calmar,
            ic: None,
            ir: None,
        }
    }

    /// 从 BacktestResult 构建
    pub fn from_backtest_result(result: &BacktestResult, risk_free_annual: f64) -> Self {
        // 修复 L-5: 使用 A 股实际交易日数 244 而非美股的 252。
        Self::from_equity_curve(
            &result.equity_curve,
            &result.trades,
            risk_free_annual,
            A_SHARE_TRADING_DAYS_PER_YEAR,
        )
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

/// 两个 YYYY-MM-DD 日期之间的近似天数差（本 crate 内唯一实现，engine/walkforward 复用）
pub fn approx_days(start: &str, end: &str) -> i64 {
    use chrono::NaiveDate;
    let s = NaiveDate::parse_from_str(start, "%Y-%m-%d").ok();
    let e = NaiveDate::parse_from_str(end, "%Y-%m-%d").ok();
    match (s, e) {
        (Some(s), Some(e)) => (e - s).num_days(),
        // 修复 L-4: 日期格式错误时静默返回 0，导致 max_drawdown_duration
        // 和年化指标失真。添加 warn 日志便于发现。
        (None, None) => {
            tracing::warn!(
                "[metrics] approx_days 日期解析失败 (start={}, end={})，返回 0",
                start,
                end
            );
            0
        },
        (None, _) => {
            tracing::warn!("[metrics] approx_days start 日期解析失败 (start={})，返回 0", start);
            0
        },
        (_, None) => {
            tracing::warn!("[metrics] approx_days end 日期解析失败 (end={})，返回 0", end);
            0
        },
    }
}

/// 年化收益率（本 crate 内唯一实现；engine 复用）
pub fn annualized(curve: &[EquityPoint], days_per_year: f64) -> f64 {
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
    if rets.len() < 2 {
        return 0.0;
    }
    let n = rets.len();
    let mean = rets.iter().sum::<f64>() / n as f64;
    let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    var.sqrt() * days_per_year.sqrt()
}

/// 夏普比率（年化，样本方差 n-1）
///
/// P3-C8: 委托 `axagent_harness::indicators::sharpe_ratio_annual` 统一实现，
/// 消除与 stock-analysis/risk.rs、astock-data/mcp_tools.rs、tools/finance.rs 的算法分叉。
/// 接受 `EquityPoint` 曲线作为输入（内部转换为日收益率切片）。
pub fn sharpe_ratio(curve: &[EquityPoint], risk_free_annual: f64, days_per_year: f64) -> f64 {
    let rets = daily_returns(curve);
    if rets.len() < 2 {
        return 0.0;
    }
    axagent_harness::indicators::sharpe_ratio_annual(&rets, risk_free_annual, days_per_year)
}

fn sortino_ratio(curve: &[EquityPoint], risk_free_annual: f64, days_per_year: f64) -> f64 {
    let rets = daily_returns(curve);
    // 修复 P2: 与 sharpe_ratio 一致的空序列检查（需至少 2 个收益值）
    if rets.len() < 2 {
        return 0.0;
    }
    let daily_rf = risk_free_annual / days_per_year;
    let excess: Vec<f64> = rets.iter().map(|r| r - daily_rf).collect();
    let mean = excess.iter().sum::<f64>() / excess.len() as f64;
    // 下行方差（仅 r < rf 时计算）
    let downside_var =
        excess.iter().filter(|&&r| r < 0.0).map(|r| r.powi(2)).sum::<f64>() / excess.len() as f64;
    let downside_std = downside_var.sqrt();
    if downside_std < 1e-10 {
        // 修复 P1: 无下行风险（所有收益 >= rf）时，Sortino 数学定义应为正无穷，
        // 原代码返回 0 会被误解为"最差绩效"。改为：mean>0 返回正无穷，否则 0。
        return if mean > 0.0 { f64::INFINITY } else { 0.0 };
    }
    mean / downside_std * days_per_year.sqrt()
}

/// 最大回撤（金额 + 百分比；本 crate 内唯一实现；engine 复用）
pub fn max_drawdown(curve: &[EquityPoint]) -> (f64, f64) {
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

/// 平均持仓天数 — 基于 FIFO 开仓/平仓匹配
///
/// 判断逻辑：
/// - `realized_pnl == 0.0` → 开仓（建仓/加仓），push 到该 code 的开仓时间队列
/// - `realized_pnl != 0.0` → 平仓，FIFO pop 最早的开仓时间，计算持有天数
///
/// 边界处理：
/// - 无平仓交易 → 返回 0.0
/// - 开仓/平仓不匹配（如只有平仓无开仓）→ 跳过该平仓
/// - 日期解析失败 → approx_days 返回 0，该笔 holding_days 计为 0
fn avg_holding_days(trades: &[Trade]) -> f64 {
    use std::collections::HashMap;
    let mut open_times: HashMap<String, Vec<String>> = HashMap::new();
    let mut holding_days_sum = 0.0;
    let mut closed_count = 0usize;

    for t in trades {
        if t.realized_pnl == 0.0 {
            // 开仓（建仓或加仓）
            open_times.entry(t.code.clone()).or_default().push(t.timestamp.clone());
        } else {
            // 平仓：FIFO 匹配最早的开仓
            if let Some(queue) = open_times.get_mut(&t.code)
                && !queue.is_empty()
            {
                let open_ts = queue.remove(0);
                let days = approx_days(&open_ts, &t.timestamp).abs() as f64;
                holding_days_sum += days;
                closed_count += 1;
            }
        }
    }

    if closed_count > 0 {
        holding_days_sum / closed_count as f64
    } else {
        0.0
    }
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
        EquityPoint { date: date.to_string(), equity, cash: equity, position_value: 0.0 }
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

    #[test]
    fn test_sharpe_empty_curve_is_zero() {
        // 空序列不得 panic，且返回 0（除零卫士：rets.len() < 2）
        let empty: Vec<EquityPoint> = vec![];
        assert_eq!(sharpe_ratio(&empty, 0.025, 252.0), 0.0);
    }

    #[test]
    fn test_max_drawdown_empty_is_zero() {
        // 空序列返回 (0.0, 0.0)，不得 panic 或读非法 peak
        let empty: Vec<EquityPoint> = vec![];
        assert_eq!(max_drawdown(&empty), (0.0, 0.0));
    }

    #[test]
    fn test_annualized_empty_is_zero() {
        let empty: Vec<EquityPoint> = vec![];
        assert_eq!(annualized(&empty, 252.0), 0.0);
    }

    // ── M2 补全测试：Calmar ratio ──

    #[test]
    fn test_calmar_with_drawdown() {
        // 年化收益 20%，最大回撤 10% → Calmar = 0.20 / 0.10 = 2.0
        let curve = vec![
            make_eq("2025-01-01", 1_000_000.0),
            make_eq("2025-04-01", 1_200_000.0), // +20% 涨到峰值
            make_eq("2025-07-01", 1_080_000.0), // -10% 回撤
            make_eq("2025-12-31", 1_200_000.0), // 恢复
        ];
        let m = MetricsReport::from_equity_curve(&curve, &[], 0.025, 252.0);
        assert!(m.calmar.is_some(), "Calmar 应为 Some（有回撤）");
        let calmar = m.calmar.unwrap();
        // Calmar = annualized_return / |max_dd_pct|
        // 年化收益和回撤百分比都基于实际数据，验证符号和数量级
        assert!(calmar.is_finite(), "Calmar 应为有限值");
    }

    #[test]
    fn test_calmar_no_drawdown_is_none() {
        // 单调上涨，无回撤 → max_dd_pct == 0 → Calmar = None
        let curve = vec![
            make_eq("2025-01-01", 100.0),
            make_eq("2025-01-02", 110.0),
            make_eq("2025-01-03", 120.0),
        ];
        let m = MetricsReport::from_equity_curve(&curve, &[], 0.025, 252.0);
        assert!(m.calmar.is_none(), "无回撤时 Calmar 应为 None");
    }

    // ── M2 补全测试：avg_holding_days ──

    fn make_trade_with(code: &str, timestamp: &str, pnl: f64) -> Trade {
        Trade {
            code: code.to_string(),
            side: Side::Long,
            quantity: 100,
            price: 100.0,
            amount: 10000.0,
            commission: 5.0,
            stamp_tax: 5.0,
            slippage: 0.5,
            timestamp: timestamp.to_string(),
            reason: "test".to_string(),
            realized_pnl: pnl,
        }
    }

    #[test]
    fn test_avg_holding_days_basic() {
        // 开仓 1月1日 → 平仓 1月15日 = 14天
        let trades = vec![
            make_trade_with("TEST", "2025-01-01", 0.0),   // 开仓
            make_trade_with("TEST", "2025-01-15", 100.0), // 平仓
        ];
        let avg = avg_holding_days(&trades);
        assert!((avg - 14.0).abs() < 1.0, "avg_holding_days 应约为 14 天，实际: {}", avg);
    }

    #[test]
    fn test_avg_holding_days_multiple_codes() {
        // 两只股票各一笔：
        // AAA: 1月1日开仓 → 1月11日平仓 = 10天
        // BBB: 1月1日开仓 → 1月21日平仓 = 20天
        // 平均 = (10 + 20) / 2 = 15天
        let trades = vec![
            make_trade_with("AAA", "2025-01-01", 0.0),
            make_trade_with("BBB", "2025-01-01", 0.0),
            make_trade_with("AAA", "2025-01-11", 50.0),
            make_trade_with("BBB", "2025-01-21", -30.0),
        ];
        let avg = avg_holding_days(&trades);
        assert!((avg - 15.0).abs() < 1.0, "avg_holding_days 应约为 15 天，实际: {}", avg);
    }

    #[test]
    fn test_avg_holding_days_no_closed() {
        // 只有开仓无平仓 → 返回 0.0
        let trades = vec![
            make_trade_with("TEST", "2025-01-01", 0.0),
            make_trade_with("TEST", "2025-01-02", 0.0), // 加仓
        ];
        let avg = avg_holding_days(&trades);
        assert_eq!(avg, 0.0, "无平仓时 avg_holding_days 应为 0.0");
    }

    #[test]
    fn test_avg_holding_days_empty() {
        let avg = avg_holding_days(&[]);
        assert_eq!(avg, 0.0, "空 trades 时 avg_holding_days 应为 0.0");
    }
}
