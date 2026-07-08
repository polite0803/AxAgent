//! 交易复盘 — 自动分析已平仓交易 vs 分析预测的偏差，生成改进建议。

use axagent_entities::trades;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

/// 单笔交易复盘
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeReviewItem {
    pub stock_code: String,
    pub stock_name: String,
    pub entry_date: String,
    pub exit_date: String,
    pub holding_days: i64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl_pct: f64,
    pub pnl_amount: f64,
    pub analysis_target: Option<f64>,
    pub analysis_stop: Option<f64>,
    pub target_deviation_pct: Option<f64>,
    pub grade: String, // "优秀" | "良好" | "及格" | "需改进"
    pub comment: String,
}

/// 复盘汇总
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeReviewSummary {
    pub total_closed: usize,
    pub items: Vec<TradeReviewItem>,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_grade: String,
    pub suggestions: Vec<String>,
}

pub async fn get_trade_review(db: &DatabaseConnection) -> Result<TradeReviewSummary, String> {
    let all_sells = trades::Entity::find()
        .filter(trades::Column::Direction.eq("sell"))
        .order_by_desc(trades::Column::TradeDate)
        .all(db)
        .await
        .map_err(|e| format!("读取交易记录失败: {e}"))?;

    let mut items = Vec::new();
    let mut total_pnl = 0.0;
    let mut win_count = 0;

    for sell in &all_sells {
        // 找该股票最近一次买入
        let last_buy = trades::Entity::find()
            .filter(trades::Column::StockCode.eq(&sell.stock_code))
            .filter(trades::Column::Direction.eq("buy"))
            .filter(trades::Column::CreatedAt.lt(sell.created_at))
            .order_by_desc(trades::Column::CreatedAt)
            .one(db)
            .await
            .ok()
            .flatten();

        let (entry_price, entry_date, holding_days) = if let Some(ref buy) = last_buy {
            let days = if let (Ok(d1), Ok(d2)) = (
                chrono::NaiveDate::parse_from_str(&buy.trade_date, "%Y-%m-%d"),
                chrono::NaiveDate::parse_from_str(&sell.trade_date, "%Y-%m-%d"),
            ) {
                (d2 - d1).num_days().max(0)
            } else {
                0
            };
            (buy.price, buy.trade_date.clone(), days)
        } else {
            (0.0, "—".into(), 0)
        };

        let pnl = sell.realized_pnl.unwrap_or(0.0);
        total_pnl += pnl;
        if pnl > 0.0 {
            win_count += 1;
        }

        // 获取分析预测对比
        let (target, stop, deviation) = if let Some(analysis) =
            axagent_core::entity::stock_analyses::Entity::find()
                .filter(
                    axagent_core::entity::stock_analyses::Column::StockCode.eq(&sell.stock_code),
                )
                .filter(axagent_core::entity::stock_analyses::Column::Status.eq("completed"))
                .order_by_desc(axagent_core::entity::stock_analyses::Column::CreatedAt)
                .one(db)
                .await
                .ok()
                .flatten()
        {
            if let Some(ref djson) = analysis.decision_json {
                if let Ok(d) = serde_json::from_str::<serde_json::Value>(djson) {
                    let tp = d["targetPrice"].as_f64();
                    let sl = d["stopLoss"].as_f64();
                    let dev = tp.map(|t| {
                        if t != 0.0 {
                            (sell.price - t) / t * 100.0
                        } else {
                            0.0
                        }
                    });
                    (tp, sl, dev)
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        };

        let pnl_pct = if entry_price > 0.0 {
            (sell.price - entry_price) / entry_price * 100.0
        } else {
            0.0
        };

        // 评级
        let (grade, comment): (String, String) =
            if pnl_pct > 10.0 && deviation.is_some_and(|d| d.abs() < 15.0) {
                ("优秀".into(), format!("盈利 {:.1}%，接近分析目标，执行良好", pnl_pct))
            } else if pnl_pct > 5.0 {
                ("良好".into(), format!("盈利 {:.1}%，但可考虑更接近目标价出场", pnl_pct))
            } else if pnl_pct > 0.0 {
                (
                    "及格".into(),
                    format!("微盈 {:.1}%，注意目标价 {:.2}", pnl_pct, target.unwrap_or(0.0)),
                )
            } else if pnl_pct > -10.0 {
                ("需改进".into(), format!("亏损 {:.1}%，建议严格止损纪律", pnl_pct))
            } else {
                ("需改进".into(), format!("大幅亏损 {:.1}%，需复盘入场逻辑", pnl_pct))
            };

        items.push(TradeReviewItem {
            stock_code: sell.stock_code.clone(),
            stock_name: sell.stock_name.clone(),
            entry_date,
            exit_date: sell.trade_date.clone(),
            holding_days,
            entry_price,
            exit_price: sell.price,
            pnl_pct,
            pnl_amount: pnl,
            analysis_target: target,
            analysis_stop: stop,
            target_deviation_pct: deviation,
            grade,
            comment,
        });
    }

    let total = items.len();
    let win_rate = if total > 0 {
        win_count as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let avg_grade = if win_rate >= 60.0 {
        "良好"
    } else if win_rate >= 40.0 {
        "及格"
    } else {
        "需改进"
    };

    let mut suggestions = Vec::new();
    if win_rate < 50.0 {
        suggestions.push("胜率低于 50%，建议提高选股标准，减少交易频率".into());
    }
    if total_pnl < 0.0 {
        suggestions.push("整体亏损，建议暂停交易，重新评估策略".into());
    }
    let big_losses = items.iter().filter(|i| i.pnl_pct < -15.0).count();
    if big_losses > 0 {
        suggestions.push(format!("{big_losses} 笔交易亏损超过 15%，建议设置更严格的止损线"));
    }
    if suggestions.is_empty() {
        suggestions.push("整体交易表现良好，继续保持纪律".into());
    }

    Ok(TradeReviewSummary {
        total_closed: total,
        items,
        win_rate,
        total_pnl,
        avg_grade: avg_grade.to_string(),
        suggestions,
    })
}
