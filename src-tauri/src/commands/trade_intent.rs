// SPDX-License-Identifier: AGPL-3.0-only

//! 交易意图命令 — 安全自动化的交易记录层 Tauri 命令
//!
//! 提供前端调用接口，用于：
//! - 查询待审核交易意图列表
//! - 审核通过/驳回交易意图
//! - 关联实际交易
//! - 标记过期
//! - 触发分析完成后的自动记录

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_analysis_engine::trade_intent::{
    ReviewTradeIntentRequest, TradeIntentItem, TradeIntentService,
};
use axagent_entities::stock_analyses;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tauri::State;

/// 查询待审核交易意图列表
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "查询待审核的交易意图列表")]
pub async fn list_pending_trade_intents(
    state: State<'_, AppState>,
    limit: Option<u64>,
) -> Result<Vec<TradeIntentItem>, String> {
    let db = state.harness.db();
    TradeIntentService::list_pending(db, limit.unwrap_or(50)).await
}

/// 查询指定股票的交易意图历史
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "查询指定股票的交易意图历史记录")]
pub async fn list_trade_intents_by_stock(
    state: State<'_, AppState>,
    stock_code: String,
    limit: Option<u64>,
) -> Result<Vec<TradeIntentItem>, String> {
    let db = state.harness.db();
    TradeIntentService::list_by_stock(db, &stock_code, limit.unwrap_or(20)).await
}

/// 审核通过交易意图
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "审核通过交易意图，状态变更为 reviewed")]
pub async fn approve_trade_intent(
    state: State<'_, AppState>,
    req: ReviewTradeIntentRequest,
) -> Result<axagent_analysis_engine::trade_intent::ReviewTradeIntentResult, String> {
    let db = state.harness.db();
    TradeIntentService::approve_intent(db, req).await
}

/// 驳回交易意图
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "驳回交易意图，状态变更为 rejected")]
pub async fn reject_trade_intent(
    state: State<'_, AppState>,
    req: ReviewTradeIntentRequest,
) -> Result<axagent_analysis_engine::trade_intent::ReviewTradeIntentResult, String> {
    let db = state.harness.db();
    TradeIntentService::reject_intent(db, req).await
}

/// 关联实际交易（标记为已执行）
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "关联交易意图与实际交易记录，状态变更为 executed")]
pub async fn link_trade_intent_to_trade(
    state: State<'_, AppState>,
    analysis_id: String,
    trade_id: String,
    reviewed_by: String,
) -> Result<axagent_analysis_engine::trade_intent::ReviewTradeIntentResult, String> {
    let db = state.harness.db();
    TradeIntentService::link_actual_trade(db, &analysis_id, &trade_id, &reviewed_by).await
}

/// 分析完成后自动记录交易意图
///
/// 在分析引擎完成决策后调用，将决策字段自动标记为"待审核"状态。
/// 如果分析结果为中性（持有/观望），则不生成交易意图。
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "分析完成后自动记录交易意图（待审核状态）")]
pub async fn record_analysis_trade_intent(
    state: State<'_, AppState>,
    analysis_id: String,
    source: Option<String>,
    source_ref_id: Option<String>,
) -> Result<(), String> {
    let db = state.harness.db();
    let source_enum = match source.as_deref() {
        Some("conditional_order") => {
            axagent_analysis_engine::trade_intent::TradeIntentSource::ConditionalOrder
        },
        Some("quant_signal") => {
            axagent_analysis_engine::trade_intent::TradeIntentSource::QuantSignal
        },
        Some("portfolio_monitor") => {
            axagent_analysis_engine::trade_intent::TradeIntentSource::PortfolioMonitor
        },
        _ => axagent_analysis_engine::trade_intent::TradeIntentSource::Analysis,
    };
    TradeIntentService::record_analysis_intent(db, &analysis_id, source_enum, source_ref_id).await
}

/// 批量过期处理（将超时的 pending 标记为 expired）
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "批量过期处理超时的待审核交易意图")]
pub async fn expire_old_trade_intents(
    state: State<'_, AppState>,
    max_age_hours: Option<i64>,
) -> Result<u64, String> {
    let db = state.harness.db();
    TradeIntentService::expire_old_intents(db, max_age_hours.unwrap_or(72)).await
}

/// 交易意图统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeIntentStats {
    pub pending_count: u64,
    pub reviewed_count: u64,
    pub executed_count: u64,
    pub rejected_count: u64,
    pub expired_count: u64,
    pub total_count: u64,
}

/// 获取交易意图统计
#[tauri::command]
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取交易意图各状态的统计数量")]
pub async fn get_trade_intent_stats(
    state: State<'_, AppState>,
) -> Result<TradeIntentStats, String> {
    let db = state.harness.db();

    let all = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::TradeIntentStatus.is_not_null())
        .all(db)
        .await
        .map_err(|e| format!("查询交易意图统计失败: {e}"))?;

    let mut stats = TradeIntentStats {
        pending_count: 0,
        reviewed_count: 0,
        executed_count: 0,
        rejected_count: 0,
        expired_count: 0,
        total_count: all.len() as u64,
    };

    for item in &all {
        match item.trade_intent_status.as_str() {
            "pending" => stats.pending_count += 1,
            "reviewed" => stats.reviewed_count += 1,
            "executed" => stats.executed_count += 1,
            "rejected" => stats.rejected_count += 1,
            "expired" => stats.expired_count += 1,
            _ => {},
        }
    }

    Ok(stats)
}
