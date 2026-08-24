// SPDX-License-Identifier: AGPL-3.0-only
//! G2 模拟观察组合（Paper Trading Portfolio）Tauri 命令层
//!
//! 对应前端 IPC 调用，全部走 `#[tauri::command]`，返回 `Result<T, String>`。
//! 业务实现委托给 `axagent_analysis_engine::paper_portfolio`。
//!
//! 命令清单：
//! - `paper_portfolio_create` —— 创建模拟组合
//! - `paper_portfolio_list` —— 列出所有组合（按状态过滤）
//! - `paper_portfolio_get` —— 获取单个组合详情（含持仓 + 实时盈亏）
//! - `paper_portfolio_close` —— 关闭组合
//! - `paper_portfolio_archive` —— 归档组合
//! - `paper_portfolio_add_position` —— 添加虚拟持仓
//! - `paper_portfolio_close_position` —— 平仓单个持仓
//! - `paper_portfolio_close_all_positions` —— 批量平仓
//! - `paper_portfolio_list_active_details` —— 列出所有 active 组合详情（Dashboard 用）

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_analysis_engine::paper_portfolio::{
    self, AddPositionInput, ClosePositionInput, CreatePortfolioInput, PortfolioDetail,
};
use tauri::State;

/// 创建模拟组合
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "创建模拟组合")]
#[tauri::command]
pub async fn paper_portfolio_create(
    state: State<'_, AppState>,
    input: CreatePortfolioInput,
) -> Result<axagent_entities::paper_portfolios::Model, String> {
    paper_portfolio::create_portfolio(state.harness.db(), input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 列出所有组合（按状态过滤，None = 全部）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出所有模拟组合")]
#[tauri::command]
pub async fn paper_portfolio_list(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<axagent_entities::paper_portfolios::Model>, String> {
    paper_portfolio::list_portfolios(state.harness.db(), status.as_deref()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取单个组合详情（含持仓 + 实时盈亏）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取模拟组合详情")]
#[tauri::command]
pub async fn paper_portfolio_get(
    state: State<'_, AppState>,
    portfolio_id: String,
) -> Result<Option<PortfolioDetail>, String> {
    paper_portfolio::get_portfolio_detail(state.harness.db(), &*state.astock_client, &portfolio_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

/// 关闭组合（status=closed）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "关闭模拟组合")]
#[tauri::command]
pub async fn paper_portfolio_close(
    state: State<'_, AppState>,
    portfolio_id: String,
) -> Result<axagent_entities::paper_portfolios::Model, String> {
    paper_portfolio::close_portfolio(state.harness.db(), &portfolio_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 归档组合（status=archived）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "归档模拟组合")]
#[tauri::command]
pub async fn paper_portfolio_archive(
    state: State<'_, AppState>,
    portfolio_id: String,
) -> Result<axagent_entities::paper_portfolios::Model, String> {
    paper_portfolio::archive_portfolio(state.harness.db(), &portfolio_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 添加虚拟持仓
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "添加虚拟持仓")]
#[tauri::command]
pub async fn paper_portfolio_add_position(
    state: State<'_, AppState>,
    input: AddPositionInput,
) -> Result<axagent_entities::paper_positions::Model, String> {
    paper_portfolio::add_position(state.harness.db(), input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 平仓单个持仓
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "平仓单个持仓")]
#[tauri::command]
pub async fn paper_portfolio_close_position(
    state: State<'_, AppState>,
    input: ClosePositionInput,
) -> Result<axagent_entities::paper_positions::Model, String> {
    paper_portfolio::close_position(state.harness.db(), input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 批量平仓（按 portfolio_id 平仓所有 open 持仓）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "批量平仓所有持仓")]
#[tauri::command]
pub async fn paper_portfolio_close_all_positions(
    state: State<'_, AppState>,
    portfolio_id: String,
    exit_price: f64,
    exit_date: String,
) -> Result<u64, String> {
    paper_portfolio::close_all_positions(state.harness.db(), &portfolio_id, exit_price, &exit_date)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

/// 列出所有 active 组合的详情（前端 Dashboard 用）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出活跃组合详情")]
#[tauri::command]
pub async fn paper_portfolio_list_active_details(
    state: State<'_, AppState>,
) -> Result<Vec<PortfolioDetail>, String> {
    paper_portfolio::list_active_portfolios_detail(state.harness.db(), &*state.astock_client)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}
