// SPDX-License-Identifier: AGPL-3.0-only

//! 信号→实盘执行桥接器
//!
//! NOTE: 整个模块已随 AxAgent 清理移除。所有 Tauri 命令函数体返回 Err。

use tauri::State;

use crate::AppState;
use axagent_agent_macro::agent_command;

// ── 以下所有命令已禁用（AxAgent 清理） ──

/// 提交信号到执行管道
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "提交信号到执行管道")]
#[tauri::command]
pub async fn execution_submit_signal(
    _state: State<'_, AppState>,
    _app: tauri::AppHandle,
    _signal_code: String,
    _signal_action: String,
    _signal_reason: String,
    _stock_name: String,
    _current_price: f64,
) -> Result<String, String> {
    Err("execution_bridge 功能已随 AxAgent 清理移除".to_string())
}

/// 确认待执行
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "确认待执行交易")]
#[tauri::command]
pub async fn execution_confirm(
    _state: State<'_, AppState>,
    _app: tauri::AppHandle,
    _pending_id: String,
    _quantity: i32,
) -> Result<String, String> {
    Err("execution_bridge 功能已随 AxAgent 清理移除".to_string())
}

/// 驳回待执行
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "驳回待执行交易")]
#[tauri::command]
pub async fn execution_reject(
    _state: State<'_, AppState>,
    _app: tauri::AppHandle,
    _pending_id: String,
    _reason: String,
) -> Result<(), String> {
    Err("execution_bridge 功能已随 AxAgent 清理移除".to_string())
}

/// 列出待执行
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出待执行记录")]
#[tauri::command]
pub async fn execution_list_pending(
    _state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    Err("execution_bridge 功能已随 AxAgent 清理移除".to_string())
}

/// 设置执行模式
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "设置执行模式")]
#[tauri::command]
pub async fn execution_set_mode(_state: State<'_, AppState>, _mode: String) -> Result<(), String> {
    Err("execution_bridge 功能已随 AxAgent 清理移除".to_string())
}

/// 获取当前执行模式
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取当前执行模式")]
#[tauri::command]
pub async fn execution_get_mode(_state: State<'_, AppState>) -> Result<String, String> {
    Err("execution_bridge 功能已随 AxAgent 清理移除".to_string())
}
