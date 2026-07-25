// SPDX-License-Identifier: AGPL-3.0-only
//! G4 市场主线自动提炼 Tauri 命令层
//!
//! 对应前端 IPC 调用，全部走 `#[tauri::command]`，返回 `Result<T, String>`。
//! 业务实现委托给 `axagent_stock_analysis::market_mainline`。
//!
//! 命令清单：
//! - `market_mainline_create` —— 创建单条主线
//! - `market_mainline_get` —— 按 ID 获取主线
//! - `market_mainline_list_by_date` —— 列出某日所有主线（按强度降序）
//! - `market_mainline_list_recent` —— 列出最近 N 天的主线
//! - `market_mainline_list_by_status` —— 按状态过滤主线
//! - `market_mainline_list_by_category` —— 按主题大类过滤主线
//! - `market_mainline_update` —— 更新主线（部分字段）
//! - `market_mainline_archive` —— 归档主线
//! - `market_mainline_batch_upsert` —— 批量 upsert（工作流用）
//! - `market_mainline_delete_by_date` —— 清除某日所有主线

use crate::AppState;
use axagent_stock_analysis::market_mainline::{
    self, BatchUpsertInput, BatchUpsertResult, CreateMainlineInput, UpdateMainlineInput,
};
use tauri::State;

/// 创建单条市场主线
#[tauri::command]
pub async fn market_mainline_create(
    state: State<'_, AppState>,
    input: CreateMainlineInput,
) -> Result<axagent_entities::market_mainlines::Model, String> {
    market_mainline::create_mainline(state.harness.db(), input).await.map_err(|e| e.to_string())
}

/// 按 ID 获取主线
#[tauri::command]
pub async fn market_mainline_get(
    state: State<'_, AppState>,
    mainline_id: String,
) -> Result<Option<axagent_entities::market_mainlines::Model>, String> {
    market_mainline::get_mainline(state.harness.db(), &mainline_id).await.map_err(|e| e.to_string())
}

/// 列出某日所有主线（按强度降序）
#[tauri::command]
pub async fn market_mainline_list_by_date(
    state: State<'_, AppState>,
    mainline_date: String,
) -> Result<Vec<axagent_entities::market_mainlines::Model>, String> {
    market_mainline::list_mainlines_by_date(state.harness.db(), &mainline_date)
        .await
        .map_err(|e| e.to_string())
}

/// 列出最近 N 天的主线
#[tauri::command]
pub async fn market_mainline_list_recent(
    state: State<'_, AppState>,
    days: Option<usize>,
) -> Result<Vec<axagent_entities::market_mainlines::Model>, String> {
    let d = days.unwrap_or(7);
    market_mainline::list_recent_mainlines(state.harness.db(), d).await.map_err(|e| e.to_string())
}

/// 按状态过滤主线
#[tauri::command]
pub async fn market_mainline_list_by_status(
    state: State<'_, AppState>,
    status: String,
) -> Result<Vec<axagent_entities::market_mainlines::Model>, String> {
    market_mainline::list_mainlines_by_status(state.harness.db(), &status)
        .await
        .map_err(|e| e.to_string())
}

/// 按主题大类过滤主线
#[tauri::command]
pub async fn market_mainline_list_by_category(
    state: State<'_, AppState>,
    theme_category: String,
) -> Result<Vec<axagent_entities::market_mainlines::Model>, String> {
    market_mainline::list_mainlines_by_category(state.harness.db(), &theme_category)
        .await
        .map_err(|e| e.to_string())
}

/// 更新主线（部分字段）
#[tauri::command]
pub async fn market_mainline_update(
    state: State<'_, AppState>,
    input: UpdateMainlineInput,
) -> Result<axagent_entities::market_mainlines::Model, String> {
    market_mainline::update_mainline(state.harness.db(), input).await.map_err(|e| e.to_string())
}

/// 归档主线（status=archived）
#[tauri::command]
pub async fn market_mainline_archive(
    state: State<'_, AppState>,
    mainline_id: String,
) -> Result<axagent_entities::market_mainlines::Model, String> {
    market_mainline::archive_mainline(state.harness.db(), &mainline_id)
        .await
        .map_err(|e| e.to_string())
}

/// 批量 upsert 主线（工作流 persist_to_db 节点用）
#[tauri::command]
pub async fn market_mainline_batch_upsert(
    state: State<'_, AppState>,
    input: BatchUpsertInput,
) -> Result<BatchUpsertResult, String> {
    market_mainline::batch_upsert_mainlines(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

/// 清除某日所有主线（管理用，慎调）
#[tauri::command]
pub async fn market_mainline_delete_by_date(
    state: State<'_, AppState>,
    mainline_date: String,
) -> Result<u64, String> {
    market_mainline::delete_mainlines_by_date(state.harness.db(), &mainline_date)
        .await
        .map_err(|e| e.to_string())
}
