// SPDX-License-Identifier: AGPL-3.0-only

//! 通用节点进化命令 — 存根（stub）
//!
//! 原实现委托给已删除的 `axagent_analysis_engine` crate。
//! 所有命令返回统一错误，告知用户功能已移除。

use axagent_agent_macro::agent_command;

// ── 通用节点进化命令 ──

/// 触发节点自我进化（通用版）
///
/// 已移除：后端 analysis_engine 依赖已删除。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "触发节点自我进化：基于历史反馈优化 Prompt/配置（功能已移除）")]
#[tauri::command]
pub async fn evolve_node_command(
    _state: tauri::State<'_, crate::AppState>,
    _node_type: String,
    _node_id: String,
) -> Result<serde_json::Value, String> {
    Err("节点进化功能已移除（后端 analysis_engine 依赖已删除）".to_string())
}

/// 获取节点进化状态（通用版）
///
/// 已移除：后端 analysis_engine 依赖已删除。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取节点当前进化状态（功能已移除）")]
#[tauri::command]
pub async fn get_node_evolution_status_command(
    _state: tauri::State<'_, crate::AppState>,
    _node_type: String,
    _node_id: String,
) -> Result<serde_json::Value, String> {
    Err("节点进化功能已移除（后端 analysis_engine 依赖已删除）".to_string())
}

// ── 分析师节点专用进化命令 ──

/// 触发分析师节点自我进化
///
/// 已移除：后端 analysis_engine 依赖已删除。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "触发分析师节点自我进化（功能已移除）")]
#[tauri::command]
pub async fn evolve_analyst_command(
    _state: tauri::State<'_, crate::AppState>,
    _analyst_id: String,
) -> Result<serde_json::Value, String> {
    Err("分析师进化功能已移除（后端 analysis_engine 依赖已删除）".to_string())
}

/// 获取分析师节点进化状态
///
/// 已移除：后端 analysis_engine 依赖已删除。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取分析师节点进化状态（功能已移除）")]
#[tauri::command]
pub async fn get_analyst_evolution_status_command(
    _state: tauri::State<'_, crate::AppState>,
    _analyst_id: String,
) -> Result<serde_json::Value, String> {
    Err("分析师进化功能已移除（后端 analysis_engine 依赖已删除）".to_string())
}
