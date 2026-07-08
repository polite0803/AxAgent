//! 交易统计分析 — 持仓天数分布、赢家/输家对比、月度业绩、税费汇总。

use axagent_entities::trades;
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};
use std::collections::HashMap;

// ── 公开类型 ──

/// 交易统计汇总
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeStatsSummary {
    /// 基础统计
    pub total_buys: usize,
    pub total_sells: usize,
    pub total_fees_est: f64,
    pub total_stamp_tax: f64,
    /// 盈亏统计
    pub total_realized_pnl: f64,
    pub win_count: usize,
    pub loss_count: usize,
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    /// 持有期分布
    pub holding_days_dist: Vec<HoldingDaysBucket>,
    pub avg_holding_days: f64,
    /// 月度业绩
    pub monthly_pnl: Vec<MonthlyPnl>,
    /// 按策略分组
    pub strategy_breakdown: Vec<StrategyBreakdown>,
}

/// 持有期区间
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingDaysBucket {
    pub label: String,
    pub count: usize,
    pub win_count: usize,
    pub total_pnl: f64,
}

/// 月度盈亏
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyPnl {
    pub year_month: String,
    pub realized_pnl: f64,
    pub trade_count: usize,
    pub win_count: usize,
}

/// 策略分组统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyBreakdown {
    pub strategy: String,
    pub trade_count: usize,
    pub total_pnl: f64,
    pub win_count: usize,
    pub win_rate: f64,
}

// ── 主函数 ──

pub async fn get_trade_stats(db: &DatabaseConnection) -> Result<TradeStatsSummary, String> {
    let all_trades = trades::Entity::find()
        .order_by_asc(trades::Column::TradeDate)
        .all(db)
        .await
        .map_err(|e| format!("读取交易记录失败: {e}"))?;

    if all_trades.is_empty() {
        return Ok(TradeStatsSummary::default());
    }

    // 按股票代码分组，用于配对买卖计算持有期
    let mut by_stock: HashMap<String, Vec<&trades::Model>> = HashMap::new();
    for t in &all_trades {
        by_stock.entry(t.stock_code.clone()).or_default().push(t);
    }

    // 统计基础
    let buys: Vec<_> = all_trades.iter().filter(|t| t.direction == "buy").collect();
    let sells: Vec<_> = all_trades
        .iter()
        .filter(|t| t.direction == "sell")
        .collect();
    let total_realized: f64 = sells.iter().filter_map(|t| t.realized_pnl).sum();
    let wins: Vec<_> = sells
        .iter()
        .filter(|t| t.realized_pnl.is_some_and(|p| p > 0.0))
        .collect();
    let losses: Vec<_> = sells
        .iter()
        .filter(|t| t.realized_pnl.is_some_and(|p| p <= 0.0))
        .collect();

    let win_count = wins.len();
    let loss_count = losses.len();
    let win_rate = if !sells.is_empty() {
        win_count as f64 / sells.len() as f64 * 100.0
    } else {
        0.0
    };
    let avg_win = if !wins.is_empty() {
        wins.iter().filter_map(|t| t.realized_pnl).sum::<f64>() / win_count as f64
    } else {
        0.0
    };
    let avg_loss = if !losses.is_empty() {
        losses.iter().filter_map(|t| t.realized_pnl).sum::<f64>() / loss_count.max(1) as f64
    } else {
        0.0
    };
    let total_win: f64 = wins.iter().filter_map(|t| t.realized_pnl).sum();
    let total_loss: f64 = losses.iter().filter_map(|t| t.realized_pnl).sum();
    let profit_factor = if total_loss.abs() > 0.0 {
        total_win / total_loss.abs()
    } else if total_win > 0.0 {
        999.0
    } else {
        0.0
    };

    // 税费估算（印花税 = 卖出金额 × 0.001，佣金 = 成交额 × 0.00025）
    let total_stamp: f64 = sells
        .iter()
        .map(|t| t.price * t.quantity as f64 * 0.001)
        .sum();
    let total_fees: f64 = all_trades
        .iter()
        .map(|t| t.price * t.quantity as f64 * 0.00025)
        .sum();

    // 持有期分布（仅已配对卖出的交易）
    let mut holding_days: Vec<(i64, f64)> = Vec::new();
    for trades in by_stock.values() {
        let mut sorted_trades = trades.clone();
        sorted_trades.sort_by_key(|t| t.created_at);
        let mut buy_date: Option<&str> = None;
        for t in sorted_trades {
            if t.direction == "buy" {
                buy_date = Some(&t.trade_date);
            } else if let Some(bd) = buy_date {
                if let Ok(d1) = chrono::NaiveDate::parse_from_str(bd, "%Y-%m-%d") {
                    if let Ok(d2) = chrono::NaiveDate::parse_from_str(&t.trade_date, "%Y-%m-%d") {
                        let days = (d2 - d1).num_days();
                        holding_days.push((days, t.realized_pnl.unwrap_or(0.0)));
                    }
                }
                buy_date = None;
            }
        }
    }

    let avg_days = if !holding_days.is_empty() {
        holding_days.iter().map(|(d, _)| *d).sum::<i64>() as f64 / holding_days.len() as f64
    } else {
        0.0
    };

    let buckets = vec![
        ("≤3天", 0..=3),
        ("4-7天", 4..=7),
        ("8-14天", 8..=14),
        ("15-30天", 15..=30),
        ("31-60天", 31..=60),
        ("61-90天", 61..=90),
        ("91-180天", 91..=180),
        (">180天", 181..=i64::MAX),
    ];

    let holding_days_dist: Vec<HoldingDaysBucket> = buckets
        .into_iter()
        .map(|(label, range)| {
            let filtered: Vec<_> = holding_days
                .iter()
                .filter(|(d, _)| range.contains(d))
                .collect();
            HoldingDaysBucket {
                label: label.to_string(),
                count: filtered.len(),
                win_count: filtered.iter().filter(|(_, pnl)| *pnl > 0.0).count(),
                total_pnl: filtered.iter().map(|(_, pnl)| *pnl).sum(),
            }
        })
        .collect();

    // 月度盈亏
    let mut month_map: HashMap<String, (f64, usize, usize)> = HashMap::new();
    for t in &all_trades {
        if t.trade_date.len() >= 7 {
            let ym = t.trade_date[..7].to_string();
            let entry = month_map.entry(ym).or_insert((0.0, 0, 0));
            entry.1 += 1;
            if let Some(pnl) = t.realized_pnl {
                entry.0 += pnl;
                if pnl > 0.0 {
                    entry.2 += 1;
                }
            }
        }
    }
    let mut monthly_pnl: Vec<MonthlyPnl> = month_map
        .into_iter()
        .map(|(ym, (pnl, cnt, w))| MonthlyPnl {
            year_month: ym,
            realized_pnl: pnl,
            trade_count: cnt,
            win_count: w,
        })
        .collect();
    monthly_pnl.sort_by(|a, b| a.year_month.cmp(&b.year_month));

    // 策略分组
    let mut strat_map: HashMap<String, (usize, f64, usize)> = HashMap::new();
    for t in &all_trades {
        let key = t.strategy.clone().unwrap_or_else(|| "未分类".to_string());
        let entry = strat_map.entry(key).or_insert((0, 0.0, 0));
        entry.0 += 1;
        if let Some(pnl) = t.realized_pnl {
            entry.1 += pnl;
            if pnl > 0.0 {
                entry.2 += 1;
            }
        }
    }
    let strategy_breakdown: Vec<StrategyBreakdown> = strat_map
        .into_iter()
        .map(|(s, (cnt, pnl, w))| StrategyBreakdown {
            win_rate: if cnt > 0 {
                w as f64 / cnt as f64 * 100.0
            } else {
                0.0
            },
            strategy: s,
            trade_count: cnt,
            total_pnl: pnl,
            win_count: w,
        })
        .collect();

    Ok(TradeStatsSummary {
        total_buys: buys.len(),
        total_sells: sells.len(),
        total_fees_est: total_fees,
        total_stamp_tax: total_stamp,
        total_realized_pnl: total_realized,
        win_count,
        loss_count,
        win_rate,
        avg_win,
        avg_loss,
        profit_factor,
        holding_days_dist,
        avg_holding_days: avg_days,
        monthly_pnl,
        strategy_breakdown,
    })
}

impl Default for TradeStatsSummary {
    fn default() -> Self {
        Self {
            total_buys: 0,
            total_sells: 0,
            total_fees_est: 0.0,
            total_stamp_tax: 0.0,
            total_realized_pnl: 0.0,
            win_count: 0,
            loss_count: 0,
            win_rate: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            profit_factor: 0.0,
            holding_days_dist: vec![],
            avg_holding_days: 0.0,
            monthly_pnl: vec![],
            strategy_breakdown: vec![],
        }
    }
}
