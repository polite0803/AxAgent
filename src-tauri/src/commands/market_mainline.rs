// SPDX-License-Identifier: AGPL-3.0-only
//! G4 市场主线自动提炼 Tauri 命令层 — 存根（stub）
//!
//! 原实现委托给已删除的 `axagent_analysis_engine::market_mainline`。
//! 所有命令返回统一错误，告知用户功能已移除。

use axagent_agent_macro::agent_command;
use serde_json::Value;

fn market_mainline_removed() -> Result<Value, String> {
    Err("市场主线功能已移除（后端 analysis_engine 依赖已删除）".to_string())
}

/// 创建单条市场主线
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "创建市场主线（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_create(
    _state: tauri::State<'_, crate::AppState>,
    _input: Value,
) -> Result<Value, String> {
    market_mainline_removed()
}

/// 按 ID 获取主线
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取主线详情（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_get(
    _state: tauri::State<'_, crate::AppState>,
    _mainline_id: String,
) -> Result<Value, String> {
    market_mainline_removed()
}

/// 列出某日所有主线（按强度降序）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "按日期列出主线（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_list_by_date(
    _state: tauri::State<'_, crate::AppState>,
    _mainline_date: String,
) -> Result<Value, String> {
    market_mainline_removed()
}

/// 列出最近 N 天的主线
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出最近主线（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_list_recent(
    _state: tauri::State<'_, crate::AppState>,
    _days: Option<usize>,
) -> Result<Value, String> {
    market_mainline_removed()
}

/// 按状态过滤主线
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "按状态列出主线（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_list_by_status(
    _state: tauri::State<'_, crate::AppState>,
    _status: String,
) -> Result<Value, String> {
    market_mainline_removed()
}

/// 按主题大类过滤主线
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "按主题列出主线（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_list_by_category(
    _state: tauri::State<'_, crate::AppState>,
    _theme_category: String,
) -> Result<Value, String> {
    market_mainline_removed()
}

/// 更新主线（部分字段）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "更新主线（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_update(
    _state: tauri::State<'_, crate::AppState>,
    _input: Value,
) -> Result<Value, String> {
    market_mainline_removed()
}

/// 归档主线（status=archived）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "归档主线（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_archive(
    _state: tauri::State<'_, crate::AppState>,
    _mainline_id: String,
) -> Result<Value, String> {
    market_mainline_removed()
}

/// 批量 upsert 主线（工作流 persist_to_db 节点用）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "批量写入主线（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_batch_upsert(
    _state: tauri::State<'_, crate::AppState>,
    _input: Value,
) -> Result<Value, String> {
    market_mainline_removed()
}

/// 清除某日所有主线（管理用，慎调）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "删除指定日期主线（功能已移除）")]
#[tauri::command]
pub async fn market_mainline_delete_by_date(
    _state: tauri::State<'_, crate::AppState>,
    _mainline_date: String,
) -> Result<u64, String> {
    // NOTE: market_mainline 功能已随 AxAgent 清理移除
    Err("market_mainline_delete_by_date 功能已随 AxAgent 清理移除".to_string())
}
