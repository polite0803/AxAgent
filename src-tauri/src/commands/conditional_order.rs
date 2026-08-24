// SPDX-License-Identifier: AGPL-3.0-only

//! 条件单管理命令 — 风控闭环的人工管理接口
//!
//! 提供前端调用接口，用于：
//! - 创建/更新条件单（加载到内存引擎，持久化由 bridge 处理）
//! - 查询条件单列表
//! - 启用/停用条件单
//! - 手动触发条件单评估

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_analysis_engine::conditional_order::{ConditionalOrder, OrderAction};
use axagent_analysis_engine::conditional_order_bridge::ConditionalOrderBridge;
use axagent_analysis_engine::risk_inspection;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tauri::State;

/// 创建条件单请求
#[derive(Debug, Deserialize)]
pub struct CreateConditionalOrderRequest {
    pub stock_code: String,
    pub stock_name: String,
    pub condition_type: String,
    pub action_type: String,
    pub action_value: Option<f64>,
    pub cool_down_minutes: Option<i32>,
    pub max_triggers_per_day: Option<i32>,
}

/// 创建条件单
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "创建条件单，设置触发条件和动作")]
pub async fn create_conditional_order(
    _state: State<'_, AppState>,
    req: CreateConditionalOrderRequest,
) -> Result<String, String> {
    use axagent_analysis_engine::conditional_order::ConditionType;

    let condition = match req.condition_type.as_str() {
        "stop_loss" => ConditionType::StopLoss,
        "take_profit" => ConditionType::TakeProfit,
        "resistance_break" => ConditionType::ResistanceBreak,
        "support_break" => ConditionType::SupportBreak,
        "turnover_rate" => {
            ConditionType::TurnoverRate { threshold: req.action_value.unwrap_or(10.0) }
        },
        "change_pct" => ConditionType::ChangePct { threshold: req.action_value.unwrap_or(3.0) },
        _ => return Err(format!("未知条件类型: {}", req.condition_type)),
    };

    let action = match req.action_type.as_str() {
        "buy" => OrderAction::Buy { quantity: req.action_value.map(|v| v as i32).unwrap_or(100) },
        "sell" => OrderAction::Sell { quantity: None },
        "reduce" => OrderAction::Reduce { ratio: req.action_value.unwrap_or(50.0) / 100.0 },
        "notify" => OrderAction::Notify,
        _ => return Err(format!("未知动作类型: {}", req.action_type)),
    };

    let order_id = format!("co_{}_{}", req.stock_code, chrono::Utc::now().timestamp());

    let now = chrono::Utc::now().timestamp_millis();

    // 加载到内存引擎（持久化由 bridge 的 load_from_db 完成）
    let order = ConditionalOrder {
        id: order_id.clone(),
        stock_code: req.stock_code,
        stock_name: req.stock_name,
        condition,
        action,
        enabled: true,
        active_start: None,
        active_end: None,
        max_triggers_per_day: req.max_triggers_per_day.unwrap_or(3) as u32,
        cool_down_minutes: req.cool_down_minutes.unwrap_or(30) as u32,
        today_trigger_count: 0,
        last_triggered_at: 0,
        created_at: now,
        updated_at: now,
    };

    ConditionalOrderBridge::add_order(order).await;

    Ok(order_id)
}

/// 条件单列表项
#[derive(Debug, Serialize, Clone)]
pub struct ConditionalOrderItem {
    pub order_id: String,
    pub stock_code: String,
    pub stock_name: String,
    pub condition_type: String,
    pub action_type: String,
    pub enabled: bool,
    pub today_trigger_count: i32,
    pub last_triggered_at: i64,
    pub cool_down_minutes: i32,
    pub max_triggers_per_day: i32,
}

/// 查询条件单列表（从内存引擎获取）
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "查询条件单列表")]
pub async fn list_conditional_orders(
    state: State<'_, AppState>,
) -> Result<Vec<ConditionalOrderItem>, String> {
    let db = state.harness.db().clone();

    // 先从数据库加载最新的条件单到引擎
    let _ = ConditionalOrderBridge::load_from_db(&db).await;

    let orders = ConditionalOrderBridge::get_all_orders().await;

    let items: Vec<ConditionalOrderItem> = orders
        .into_iter()
        .map(|o| ConditionalOrderItem {
            order_id: o.id,
            stock_code: o.stock_code,
            stock_name: o.stock_name,
            condition_type: format!("{:?}", o.condition),
            action_type: format!("{:?}", o.action),
            enabled: o.enabled,
            today_trigger_count: o.today_trigger_count as i32,
            last_triggered_at: o.last_triggered_at,
            cool_down_minutes: o.cool_down_minutes as i32,
            max_triggers_per_day: o.max_triggers_per_day as i32,
        })
        .collect();

    Ok(items)
}

/// 停用条件单
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "停用指定条件单")]
pub async fn disable_conditional_order(
    state: State<'_, AppState>,
    order_id: String,
) -> Result<(), String> {
    let db = state.harness.db().clone();

    use axagent_entities::stock_analyses;
    let now = chrono::Utc::now().timestamp_millis();

    stock_analyses::Entity::update_many()
        .col_expr(
            stock_analyses::Column::Status,
            sea_orm::sea_query::Expr::value("disabled".to_string()),
        )
        .col_expr(stock_analyses::Column::UpdatedAt, sea_orm::sea_query::Expr::value(now))
        .filter(stock_analyses::Column::TradeIntentSource.eq("conditional_order"))
        .filter(stock_analyses::Column::TradeIntentSourceRefId.eq(order_id.clone()))
        .exec(&db)
        .await
        .map_err(|e| format!("停用条件单失败: {e}"))?;

    ConditionalOrderBridge::remove_order(&order_id).await;

    Ok(())
}

/// 手动触发条件单评估（调试用）
#[tauri::command]
#[agent_command(domain = "finance", safety = Caution, call_mode = Manual, description = "手动触发条件单评估，测试风控链路")]
pub async fn manual_evaluate_conditions(
    state: State<'_, AppState>,
    stock_code: Option<String>,
) -> Result<u32, String> {
    let db = state.harness.db().clone();
    let client = state.astock_client.clone();

    let codes = if let Some(ref code) = stock_code {
        vec![code.clone()]
    } else {
        use axagent_entities::portfolio_holdings;
        let holdings = portfolio_holdings::Entity::find()
            .filter(portfolio_holdings::Column::Shares.gt(0.0))
            .all(&db)
            .await
            .map_err(|e| format!("查询持仓失败: {e}"))?;

        holdings.into_iter().map(|h| h.stock_code).collect()
    };

    if codes.is_empty() {
        return Ok(0);
    }

    let mut count = 0u32;
    for code in codes {
        let quote = match client.get_quote(&code).await {
            Ok(q) => q,
            Err(e) => {
                tracing::warn!("[manual_evaluate] 股票 {code} 行情获取失败: {e}");
                continue;
            },
        };

        let event = axagent_astock_data::realtime_quote::QuoteChangeEvent {
            stock_code: quote.code.clone(),
            previous: None,
            current: quote.clone(),
            change_pct: quote.change_pct,
            trigger: "manual".to_string(),
        };

        match risk_inspection::evaluate_quote_against_conditions(&db, &event).await {
            Ok(ids) => {
                if !ids.is_empty() {
                    count += ids.len() as u32;
                    tracing::info!(
                        "[manual_evaluate] 股票 {} 触发 {} 条交易意图",
                        quote.code,
                        ids.len()
                    );
                }
            },
            Err(e) => {
                tracing::warn!("[manual_evaluate] 股票 {} 评估失败: {e}", quote.code);
            },
        }
    }

    Ok(count)
}
