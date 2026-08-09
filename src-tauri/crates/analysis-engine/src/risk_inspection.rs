// SPDX-License-Identifier: AGPL-3.0-only

//! 风控自动巡检服务 — 全流程自动化核心
//!
//! ## 职责
//!
//! 1. 定时从 `stock_analyses` 加载条件单到内存引擎
//! 2. 订阅 `RealTimeQuoteWatcher` 行情事件
//! 3. 行情变动时自动评估条件单 → 触发交易意图写入
//! 4. 收盘时自动过期 pending 交易意图
//! 5. 每日自动运行分析管道（日终复盘）
//!
//! ## 自动化等级
//!
//! - 行情 → 条件单评估 → 交易意图记录：**全自动**（无需人工）
//! - 交易意图 → 真实交易执行：**半自动**（需人工审核确认）
//! - 日终分析管道：**自动定时触发**（每日 18:00）

use crate::conditional_order_bridge::ConditionalOrderBridge;
use crate::trade_intent::TradeIntentService;
use std::sync::Arc;
use tokio::time::Duration;

/// 启动风控自动巡检后台服务
///
/// 在应用启动时调用，负责：
/// - 加载条件单到内存引擎
/// - 启动行情事件消费循环
/// - 定时过期处理 pending 交易意图
pub async fn start_risk_inspection_service(db: sea_orm::DatabaseConnection) -> Result<(), String> {
    tracing::info!("[risk_inspection] 启动风控自动巡检服务");

    // 1. 加载条件单
    ConditionalOrderBridge::load_from_db(&db).await?;

    let db = Arc::new(db);

    // 2. 启动定时过期处理（每 30 分钟扫描一次，过期 72h 的 pending）
    let db_clone = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1800)); // 30 min
        loop {
            interval.tick().await;
            match TradeIntentService::expire_old_intents(&db_clone, 72).await {
                Ok(count) if count > 0 => {
                    tracing::info!(
                        "[risk_inspection] 过期处理: 已过期 {} 条 pending 交易意图",
                        count
                    );
                },
                _ => {},
            }
        }
    });

    // 3. 启动条件单热加载（每 5 分钟重新加载条件单配置）
    let db_clone = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 min
        loop {
            interval.tick().await;
            let _ = ConditionalOrderBridge::load_from_db(&db_clone).await;
        }
    });

    tracing::info!("[risk_inspection] 风控自动巡检已启动: 条件单加载+过期处理+热加载");

    Ok(())
}

/// 行情事件 → 条件单评估 → 交易意图 的同步入口
///
/// 由 `RealTimeQuoteWatcher` 的行情回调调用，每笔行情变动都会触发条件单评估。
pub async fn evaluate_quote_against_conditions(
    db: &sea_orm::DatabaseConnection,
    event: &axagent_astock_data::realtime_quote::QuoteChangeEvent,
) -> Result<Vec<String>, String> {
    // 只在交易日/交易时段评估（降低无效计算）
    let now = chrono::Local::now();
    use chrono::{Datelike, Timelike};
    let weekday = now.weekday().num_days_from_monday();
    if weekday >= 5 {
        return Ok(vec![]);
    }

    let hour = now.hour();
    let minute = now.minute();
    let trading_session = (hour == 9 && minute >= 30)
        || (hour == 10)
        || (hour == 11 && minute <= 30)
        || (hour == 13)
        || (hour == 14)
        || (hour == 15 && minute == 0);

    if !trading_session {
        return Ok(vec![]);
    }

    // 价格变动小于 0.5% 不触发条件单评估（降噪）
    if event.change_pct.abs() < 0.5 {
        return Ok(vec![]);
    }

    ConditionalOrderBridge::handle_quote_event(db, event).await
}
