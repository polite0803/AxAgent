// SPDX-License-Identifier: AGPL-3.0-only

//! 通用节点进化命令
//!
//! 支持所有节点类型（分析师/辩论/决策/工具/估值/风险）的自我进化。

use crate::AppState;
use agent_macro::agent_command;
use axagent_analysis_engine::{
    AnalystEvolutionStatus, NodeEvolutionStatus, NodeType, evolve_node, get_node_evolution_status,
};
use serde::{Deserialize, Serialize};
use tauri::State;

// ── 通用节点进化命令 ──

/// 请求进化指定节点
#[derive(Debug, Deserialize)]
pub struct EvolveNodeRequest {
    /// 节点类型 (analyst/debate/decision/tool/valuation/risk/other)
    pub node_type: String,
    /// 节点 ID
    pub node_id: String,
}

/// 获取节点进化状态请求
#[derive(Debug, Deserialize)]
pub struct GetNodeEvolutionStatusRequest {
    /// 节点类型
    pub node_type: String,
    /// 节点 ID
    pub node_id: String,
}

/// 触发节点自我进化（通用版）
///
/// 基于历史数据质量反馈，分析节点的"常见病"，
/// 生成优化建议，并返回当前的进化状态。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "触发节点自我进化：基于历史反馈优化 Prompt/配置")]
#[tauri::command]
pub async fn evolve_node_command(
    state: State<'_, AppState>,
    request: EvolveNodeRequest,
) -> Result<NodeEvolutionStatus, String> {
    if request.node_id.is_empty() {
        return Err("node_id 不能为空".to_string());
    }
    let node_type = parse_node_type(&request.node_type);
    let db = state.harness.db();
    evolve_node(db, &node_type, &request.node_id).await
}

/// 获取节点的进化状态（不触发进化，通用版）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取节点的进化状态和建议")]
#[tauri::command]
pub async fn get_node_evolution_status_command(
    state: State<'_, AppState>,
    request: GetNodeEvolutionStatusRequest,
) -> Result<NodeEvolutionStatus, String> {
    if request.node_id.is_empty() {
        return Err("node_id 不能为空".to_string());
    }
    let node_type = parse_node_type(&request.node_type);
    let db = state.harness.db();
    get_node_evolution_status(db, &node_type, &request.node_id).await
}

// ── 向后兼容的分析师专用命令 ──

/// 请求进化指定分析师
#[derive(Debug, Deserialize)]
pub struct EvolveAnalystRequest {
    /// 分析师 ID (对应 node_id)
    pub analyst_id: String,
}

/// 获取分析师进化状态请求
#[derive(Debug, Deserialize)]
pub struct GetAnalystEvolutionStatusRequest {
    /// 分析师 ID
    pub analyst_id: String,
}

/// 触发分析师自我进化（向后兼容）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "触发分析师自我进化（向后兼容）")]
#[tauri::command]
pub async fn evolve_analyst_command(
    state: State<'_, AppState>,
    request: EvolveAnalystRequest,
) -> Result<AnalystEvolutionStatus, String> {
    if request.analyst_id.is_empty() {
        return Err("analyst_id 不能为空".to_string());
    }
    let db = state.harness.db();
    // 调用通用函数，传入 Analyst 类型
    let status = evolve_node(db, &NodeType::Analyst, &request.analyst_id).await?;
    // 转换为向后兼容的状态类型
    Ok(NodeEvolutionStatusWrapper::from(status).into())
}

/// 获取分析师的进化状态（向后兼容）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description = "获取分析师的进化状态（向后兼容）")]
#[tauri::command]
pub async fn get_analyst_evolution_status_command(
    state: State<'_, AppState>,
    request: GetAnalystEvolutionStatusRequest,
) -> Result<AnalystEvolutionStatus, String> {
    if request.analyst_id.is_empty() {
        return Err("analyst_id 不能为空".to_string());
    }
    let db = state.harness.db();
    let status = get_node_evolution_status(db, &NodeType::Analyst, &request.analyst_id).await?;
    Ok(NodeEvolutionStatusWrapper::from(status).into())
}

// ── 辅助类型和函数 ──

/// 节点进化状态包装器（用于向后兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeEvolutionStatusWrapper {
    pub analyst_id: String,
    pub total_feedbacks: u64,
    pub issue_rate: f64,
    pub score_consistency_rate: f64,
    pub direction_consistency_rate: f64,
    pub last_evolution_time: Option<String>,
    pub evolution_count: u32,
    pub status: String,
    pub suggestions: Vec<String>,
}

impl From<NodeEvolutionStatus> for NodeEvolutionStatusWrapper {
    fn from(status: NodeEvolutionStatus) -> Self {
        NodeEvolutionStatusWrapper {
            analyst_id: status.node_id,
            total_feedbacks: status.total_feedbacks,
            issue_rate: status.issue_rate,
            score_consistency_rate: status
                .consistency_metrics
                .get("score_consistency_rate")
                .copied()
                .unwrap_or(1.0),
            direction_consistency_rate: status
                .consistency_metrics
                .get("direction_consistency_rate")
                .copied()
                .unwrap_or(1.0),
            last_evolution_time: status.last_evolution_time,
            evolution_count: status.evolution_count,
            status: status.status,
            suggestions: status.suggestions,
        }
    }
}

impl From<NodeEvolutionStatusWrapper> for AnalystEvolutionStatus {
    fn from(wrapper: NodeEvolutionStatusWrapper) -> Self {
        AnalystEvolutionStatus {
            analyst_id: wrapper.analyst_id,
            total_feedbacks: wrapper.total_feedbacks,
            issue_rate: wrapper.issue_rate,
            score_consistency_rate: wrapper.score_consistency_rate,
            direction_consistency_rate: wrapper.direction_consistency_rate,
            last_evolution_time: wrapper.last_evolution_time,
            evolution_count: wrapper.evolution_count,
            status: wrapper.status,
            suggestions: wrapper.suggestions,
        }
    }
}

/// 解析节点类型字符串
fn parse_node_type(type_str: &str) -> NodeType {
    match type_str {
        "analyst" => NodeType::Analyst,
        "debate" => NodeType::Debate,
        "decision" => NodeType::Decision,
        "tool" => NodeType::Tool,
        "valuation" => NodeType::Valuation,
        "risk" => NodeType::Risk,
        _ => NodeType::Other,
    }
}
