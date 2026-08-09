// SPDX-License-Identifier: AGPL-3.0-only

//! 条件单执行桥接器 — 风控闭环核心
//!
//! ## 职责
//!
//! 将 `RealTimeQuoteWatcher` 的行情事件与 `ConditionalOrderEngine` 的条件评估、
//! `TradeIntentService` 的交易意图记录串联成完整的风控自动化闭环：
//!
//! ```text
//! QuoteChangeEvent → ConditionalOrderBridge.evaluate()
//!   → ConditionalOrderEngine 匹配条件单
//!   → 触发 ConditionalOrderExecutor
//!   → TradeIntentService.record_conditional_order_intent()
//!   → stock_analyses 写入待审核记录
//!   → 前端 trade_intent 列表展示
//! ```
//!
//! ## 安全原则
//!
//! - 条件单触发**不直接执行交易**，仅生成交易意图（pending 状态）
//! - 所有交易意图必须经人工审核后才可执行
//! - 触发动作：买入/卖出/减仓 → 生成交易意图；通知 → 仅发通知

use crate::conditional_order::{ConditionalOrder, ConditionalOrderEngine, OrderAction};
use crate::trade_intent::TradeIntentService;
use axagent_astock_data::realtime_quote::QuoteChangeEvent;
use axagent_entities::stock_analyses;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::sync::Arc;

/// 条件单执行桥接器
pub struct ConditionalOrderBridge;

impl ConditionalOrderBridge {
    pub fn new() -> Self {
        ConditionalOrderBridge
    }

    /// 从数据库加载条件单
    pub async fn load_from_db(db: &sea_orm::DatabaseConnection) -> Result<(), String> {
        use sea_orm::QueryOrder;

        let rows = stock_analyses::Entity::find()
            .filter(stock_analyses::Column::Status.eq("completed"))
            .filter(stock_analyses::Column::TradeIntentSource.eq("conditional_order"))
            .order_by_desc(stock_analyses::Column::UpdatedAt)
            .all(db)
            .await
            .map_err(|e| format!("加载条件单失败: {e}"))?;

        let orders_map: HashMap<String, ConditionalOrder> = rows
            .into_iter()
            .filter_map(|row| {
                let ref_id = row.trade_intent_source_ref_id?;
                let action_str = row.decision_action?;
                let reasoning = row.decision_reasoning.unwrap_or_default();
                let condition_type = parse_condition_from_reasoning(&reasoning);
                let order_action = parse_action(&action_str);

                Some((
                    ref_id.clone(),
                    ConditionalOrder {
                        id: ref_id,
                        stock_code: row.stock_code,
                        stock_name: row.stock_name,
                        condition: condition_type,
                        action: order_action,
                        enabled: true,
                        active_start: None,
                        active_end: None,
                        max_triggers_per_day: 3,
                        cool_down_minutes: 30,
                        today_trigger_count: 0,
                        last_triggered_at: 0,
                        created_at: row.created_at,
                        updated_at: row.updated_at,
                    },
                ))
            })
            .collect();

        let order_list: Vec<ConditionalOrder> = orders_map.values().cloned().collect();

        {
            let mut orders = Self::static_orders_write().await;
            *orders = orders_map;
        }
        {
            let mut engine = Self::static_engine_write().await;
            engine.set_orders(order_list);
        }

        tracing::info!(
            "[conditional_bridge] 已加载 {} 条条件单",
            Self::static_orders_read().await.len()
        );
        Ok(())
    }

    /// 处理行情变更事件 → 评估条件单 → 触发交易意图
    pub async fn handle_quote_event(
        db: &sea_orm::DatabaseConnection,
        event: &QuoteChangeEvent,
    ) -> Result<Vec<String>, String> {
        let orders = Self::static_orders_read().await;
        if orders.is_empty() {
            return Ok(vec![]);
        }

        let prev_close =
            event.previous.as_ref().map(|p| p.pre_close).unwrap_or(event.current.pre_close);

        let turnover_rate = event.current.turnover_rate;

        let engine = Self::static_engine_read().await;
        let triggered = engine.evaluate(
            &event.stock_code,
            event.current.price,
            prev_close,
            turnover_rate,
            chrono::Utc::now().timestamp_millis(),
        );

        if triggered.is_empty() {
            return Ok(vec![]);
        }

        let mut generated_intent_ids = Vec::new();
        let mut orders_mut = orders.clone();

        for order in &triggered {
            // 1. 更新触发计数
            if let Some(o) = orders_mut.get_mut(&order.id) {
                o.today_trigger_count += 1;
                o.last_triggered_at = chrono::Utc::now().timestamp_millis();
            }

            // 2. 仅对非 Notify 动作生成交易意图
            if !matches!(order.action, OrderAction::Notify) {
                let action_str = match &order.action {
                    OrderAction::Buy { quantity } => format!("买入({})", quantity),
                    OrderAction::Sell { quantity } => quantity
                        .map(|q| format!("卖出({})", q))
                        .unwrap_or_else(|| "卖出全部".into()),
                    OrderAction::Reduce { ratio } => format!("减仓({:.0}%)", ratio * 100.0),
                    OrderAction::Notify => "通知".into(),
                };

                let reasoning = format!(
                    "条件单触发: 条件={:?}, 现价={:.2}, 涨跌幅={:.2}%",
                    order.condition, event.current.price, event.change_pct
                );

                match TradeIntentService::record_conditional_order_intent(
                    db,
                    &order.stock_code,
                    &order.stock_name,
                    &order.id,
                    &action_str,
                    &reasoning,
                    None,
                )
                .await
                {
                    Ok(id) => {
                        tracing::info!(
                            "[conditional_bridge] 条件单触发 → 交易意图已生成: order_id={} intent_id={}",
                            order.id,
                            id
                        );
                        generated_intent_ids.push(id);
                    },
                    Err(e) => {
                        tracing::error!("[conditional_bridge] 条件单触发 → 交易意图生成失败: {e}");
                    },
                }
            } else {
                // Notify 动作仅记日志，不生成交易意图
                tracing::info!(
                    "[conditional_bridge] 条件单触发 → 通知: order_id={} stock={} change={:.2}%",
                    order.id,
                    order.stock_code,
                    event.change_pct
                );
            }
        }

        // 3. 回写更新后的触发计数
        {
            let mut w = Self::static_orders_write().await;
            *w = orders_mut;
        }

        Ok(generated_intent_ids)
    }

    /// 添加条件单
    pub async fn add_order(order: ConditionalOrder) {
        let mut orders = Self::static_orders_write().await;
        let id = order.id.clone();
        orders.insert(id, order);

        let list: Vec<ConditionalOrder> = orders.values().cloned().collect();
        let mut engine = Self::static_engine_write().await;
        engine.set_orders(list);
    }

    /// 移除条件单
    pub async fn remove_order(order_id: &str) {
        let mut orders = Self::static_orders_write().await;
        orders.remove(order_id);

        let list: Vec<ConditionalOrder> = orders.values().cloned().collect();
        let mut engine = Self::static_engine_write().await;
        engine.set_orders(list);
    }

    /// 获取当前条件单数量
    pub async fn order_count() -> usize {
        Self::static_orders_read().await.len()
    }

    /// 获取所有条件单（供前端列表展示）
    pub async fn get_all_orders() -> Vec<ConditionalOrder> {
        let orders = Self::static_orders_read().await;
        orders.values().cloned().collect()
    }

    // ── 静态存储（供全局共享，用模块级 static OnceLock） ──

    fn static_orders() -> &'static Arc<tokio::sync::RwLock<HashMap<String, ConditionalOrder>>> {
        static ORDERS: std::sync::OnceLock<
            Arc<tokio::sync::RwLock<HashMap<String, ConditionalOrder>>>,
        > = std::sync::OnceLock::new();
        ORDERS.get_or_init(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())))
    }

    fn static_engine() -> &'static Arc<tokio::sync::RwLock<ConditionalOrderEngine>> {
        static ENGINE: std::sync::OnceLock<Arc<tokio::sync::RwLock<ConditionalOrderEngine>>> =
            std::sync::OnceLock::new();
        ENGINE.get_or_init(|| Arc::new(tokio::sync::RwLock::new(ConditionalOrderEngine::new())))
    }

    async fn static_orders_read(
    ) -> tokio::sync::RwLockReadGuard<'static, HashMap<String, ConditionalOrder>> {
        Self::static_orders().read().await
    }

    async fn static_orders_write(
    ) -> tokio::sync::RwLockWriteGuard<'static, HashMap<String, ConditionalOrder>> {
        Self::static_orders().write().await
    }

    async fn static_engine_read() -> tokio::sync::RwLockReadGuard<'static, ConditionalOrderEngine> {
        Self::static_engine().read().await
    }

    async fn static_engine_write() -> tokio::sync::RwLockWriteGuard<'static, ConditionalOrderEngine>
    {
        Self::static_engine().write().await
    }
}

impl Default for ConditionalOrderBridge {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_condition_from_reasoning(reasoning: &str) -> crate::conditional_order::ConditionType {
    use crate::conditional_order::ConditionType;
    if reasoning.contains("止损") {
        ConditionType::StopLoss
    } else if reasoning.contains("止盈") {
        ConditionType::TakeProfit
    } else if reasoning.contains("压力") {
        ConditionType::ResistanceBreak
    } else if reasoning.contains("支撑") {
        ConditionType::SupportBreak
    } else if reasoning.contains("换手") {
        ConditionType::TurnoverRate { threshold: 10.0 }
    } else {
        ConditionType::ChangePct { threshold: 3.0 }
    }
}

fn parse_action(action_str: &str) -> OrderAction {
    if action_str.contains("买入") {
        OrderAction::Buy { quantity: 100 }
    } else if action_str.contains("卖出全部") {
        OrderAction::Sell { quantity: None }
    } else if let Some(q) = action_str
        .strip_prefix("卖出(")
        .and_then(|s| s.strip_suffix(')'))
        .and_then(|s| s.parse::<i32>().ok())
    {
        OrderAction::Sell { quantity: Some(q) }
    } else if let Some(ratio_str) =
        action_str.strip_prefix("减仓(").and_then(|s| s.strip_suffix("%)"))
    {
        let ratio = ratio_str.parse::<f64>().unwrap_or(50.0) / 100.0;
        OrderAction::Reduce { ratio }
    } else {
        OrderAction::Notify
    }
}
