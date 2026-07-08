use std::collections::HashMap;

use axagent_astock_data::calendar;
use axagent_astock_data::AStockClient;
use axagent_core::entity::stock_analyses;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

/// 收盘复盘报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyReview {
    pub date: String,
    /// "交易中" | "已收盘" | "非交易日"
    pub market_status: String,
    pub watchlist_summary: Vec<StockDaySummary>,
    pub generated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockDaySummary {
    pub stock_code: String,
    pub stock_name: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub change_pct: f64,
    /// 当日量/5日均量
    pub volume_ratio: Option<f64>,
    pub key_events: Vec<String>,
    /// 当日触发的告警描述（来自 price_alerts 表）
    pub alert_triggers: Vec<String>,
    /// 该股上次分析决策对比（新增）
    pub last_decision: Option<DecisionComparison>,
}

/// 上次分析与今日行情的对比
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionComparison {
    pub analysis_date: String,
    pub action: String, // BUY / HOLD / SELL
    pub target_price: Option<f64>,
    pub stop_loss: Option<f64>,
    pub days_since_analysis: u32,
    /// 原始决策时间维度
    pub time_horizon: Option<String>,
    /// 原始期望持有天数
    pub expected_holding_days: Option<u32>,
    /// 收盘价在目标区间内？(仅 BUY)
    pub in_target_zone: bool,
    /// 已触发止损？
    pub stop_loss_hit: bool,
    /// 已达目标价？
    pub target_hit: bool,
}

/// 收盘复盘工作流
pub struct PostCloseReview;

impl PostCloseReview {
    /// 生成每日复盘报告
    ///
    /// `triggered_alerts` 为 stock_code -> alert descriptions 的映射，
    /// `db` 用于查询 `stock_analyses` 以做决策对比。
    pub async fn generate(
        client: &AStockClient,
        watchlist: &[(String, String)],
        triggered_alerts: &HashMap<String, Vec<String>>,
        db: &DatabaseConnection,
    ) -> Result<DailyReview, String> {
        let now = chrono::Utc::now();
        let today = now.format("%Y-%m-%d").to_string();
        let today_date = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap_or_default();

        let market_status = if calendar::is_trading_day(&today_date) {
            if calendar::is_trading_time() {
                "交易中".to_string()
            } else {
                "已收盘".to_string()
            }
        } else {
            "非交易日".to_string()
        };

        let mut summaries = Vec::new();
        for (code, name) in watchlist {
            let quote = match client.get_quote(code).await {
                Ok(q) => q,
                Err(_) => continue,
            };

            let klines = client
                .get_klines(code, "daily", 6)
                .await
                .ok()
                .unwrap_or_default();
            let vol_ratio = if klines.len() >= 6 {
                let avg_vol_5 = klines.iter().rev().take(5).map(|k| k.volume).sum::<f64>() / 5.0;
                Some(klines.last().map(|k| k.volume).unwrap_or(0.0) / avg_vol_5)
            } else {
                None
            };

            let mut key_events = Vec::new();
            if quote.change_pct.abs() > 5.0 {
                key_events.push(format!("异常波动 {:.2}%", quote.change_pct));
            }
            if let Some(vr) = vol_ratio {
                if vr > 2.0 {
                    key_events.push(format!("放量 {:.1}x", vr));
                }
                if vr < 0.5 {
                    key_events.push("极度缩量".to_string());
                }
            }
            if quote.is_st {
                key_events.push("ST股票".to_string());
            }

            // 合并当日触发的告警
            let stock_alerts = triggered_alerts.get(code).cloned().unwrap_or_default();

            // 查询该股最近一次 completed 分析，做决策对比
            let last_decision = fetch_latest_analysis_decision(code, &quote.price, db).await;

            summaries.push(StockDaySummary {
                stock_code: code.clone(),
                stock_name: name.clone(),
                open: quote.open,
                high: quote.high,
                low: quote.low,
                close: quote.price,
                change_pct: quote.change_pct,
                volume_ratio: vol_ratio,
                key_events,
                alert_triggers: stock_alerts,
                last_decision,
            });
        }

        Ok(DailyReview {
            date: today,
            market_status,
            watchlist_summary: summaries,
            generated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }
}

/// 查询该股最近一次 completed 分析，对比今日收盘生成 DecisionComparison
async fn fetch_latest_analysis_decision(
    stock_code: &str,
    current_price: &f64,
    db: &DatabaseConnection,
) -> Option<DecisionComparison> {
    let row = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::StockCode.eq(stock_code))
        .filter(stock_analyses::Column::Status.eq("completed"))
        .order_by_desc(stock_analyses::Column::CreatedAt)
        .limit(1)
        .one(db)
        .await
        .ok()
        .flatten()?;

    let action = row
        .decision_action
        .as_deref()
        .unwrap_or("uncertain")
        .to_string();
    let target = row.decision_json.as_ref().and_then(|raw| {
        serde_json::from_str::<serde_json::Value>(raw)
            .ok()
            .and_then(|v| v.get("targetPrice").and_then(|p| p.as_f64()))
    });
    let stop_loss = row.decision_json.as_ref().and_then(|raw| {
        serde_json::from_str::<serde_json::Value>(raw)
            .ok()
            .and_then(|v| v.get("stopLoss").and_then(|p| p.as_f64()))
    });

    // 解析分析日期 → 计算已过天数
    let days_since = chrono::NaiveDate::parse_from_str(&row.analysis_date, "%Y-%m-%d")
        .ok()
        .and_then(|d| {
            chrono::Utc::now()
                .date_naive()
                .signed_duration_since(d)
                .num_days()
                .try_into()
                .ok()
        })
        .unwrap_or(0);

    let price = *current_price;
    let in_target_zone = target.is_some_and(|t| (price - t).abs() / t <= 0.05);
    let stop_loss_hit = stop_loss.is_some_and(|s| price <= s);
    let target_hit = target.is_some_and(|t| price >= t);

    // 从 DB 列或 decision_json 提取时间维度
    let time_horizon = row.decision_time_horizon.or_else(|| {
        row.decision_json.as_ref().and_then(|raw| {
            serde_json::from_str::<serde_json::Value>(raw)
                .ok()
                .and_then(|v| {
                    v.get("timeHorizon")
                        .or_else(|| v.get("time_horizon"))
                        .and_then(|s| s.as_str().map(|s| s.to_string()))
                })
        })
    });
    let expected_holding_days = row.decision_expected_holding_days.or_else(|| {
        row.decision_json.as_ref().and_then(|raw| {
            serde_json::from_str::<serde_json::Value>(raw)
                .ok()
                .and_then(|v| {
                    v.get("expectedHoldingDays")
                        .or_else(|| v.get("expected_holding_days"))
                        .and_then(|n| n.as_u64().map(|n| n as u32))
                })
        })
    });

    Some(DecisionComparison {
        analysis_date: row.analysis_date,
        action,
        target_price: target,
        stop_loss,
        days_since_analysis: days_since,
        time_horizon,
        expected_holding_days,
        in_target_zone,
        stop_loss_hit,
        target_hit,
    })
}
