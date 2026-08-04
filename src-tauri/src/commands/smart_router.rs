// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::smart_router::{RouteDecision, RouteHistoryEntry, RouteOutcome, RouteStats};
use agent_macro::agent_command;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tauri::State;

/// 路由分类请求
#[derive(Debug, Serialize, Deserialize)]
pub struct ClassifyRouteRequest {
    pub prompt: String,
}

/// 路由结果（含决策和 prompt_hash，用于后续 feedback 关联）
#[derive(Debug, Serialize, Deserialize)]
pub struct RouteResult {
    pub decision: RouteDecision,
    /// prompt 哈希，前端在 LLM 调用后用此值关联 feedback
    pub prompt_hash: String,
}

/// 生成 prompt 哈希（用于历史记录关联，非加密用途）
fn prompt_hash(prompt: &str) -> String {
    let mut hasher = DefaultHasher::new();
    prompt.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 对用户提示进行分类，返回模型路由建议。
/// 这是基于启发式的快速分类器——无需 LLM 调用。
/// 前端用于在发送前决定使用哪个模型层级。
#[agent_command(domain = router, safety = Safe, call_mode = Manual, description = "分类路由请求")]
#[tauri::command]
pub fn classify_route(request: ClassifyRouteRequest) -> RouteDecision {
    crate::smart_router::classify_and_route(&request.prompt)
}

/// ML 成本感知路由（走 CostAwareRouter::route，含 ML 覆盖 + 成本预算）
///
/// 同时内部 record_decision，返回 prompt_hash 供后续 smart_router_record_feedback 关联。
#[agent_command(domain = router, safety = Caution, call_mode = StateInput, description = "执行智能路由")]
#[tauri::command]
pub fn smart_router_route(state: State<'_, AppState>, prompt: String) -> RouteResult {
    let heuristic = crate::smart_router::classify_and_route(&prompt);
    let decision = state.cost_aware_router.route(&prompt);
    let hash = prompt_hash(&prompt);

    state.cost_aware_router.record_decision(RouteHistoryEntry {
        prompt_hash: hash.clone(),
        prompt_preview: prompt.chars().take(200).collect(),
        heuristic_tier: heuristic.tier,
        selected_tier: decision.tier,
        outcome: None,
        timestamp: chrono::Utc::now().timestamp(),
        features: decision.features.clone(),
    });

    RouteResult { decision, prompt_hash: hash }
}

/// 记录 LLM 调用结果反馈（成功/质量/延迟/成本），更新 ML 统计
///
/// 返回更新后的 RouteStats，若 prompt_hash 未找到则返回 null。
#[agent_command(domain = router, safety = Caution, call_mode = StateInput, description = "记录路由反馈")]
#[tauri::command]
pub fn smart_router_record_feedback(
    state: State<'_, AppState>,
    prompt_hash: String,
    outcome: RouteOutcome,
) -> Option<RouteStats> {
    state.cost_aware_router.record_feedback(&prompt_hash, outcome)
}

/// 获取路由聚合统计（tier 分布/成功率/延迟/成本/ML 覆盖率/节省成本）
#[agent_command(domain = router, safety = Safe, call_mode = StateOnly, description = "获取路由统计")]
#[tauri::command]
pub fn smart_router_stats(state: State<'_, AppState>) -> RouteStats {
    state.cost_aware_router.compute_stats()
}

/// 设置成本预算上限（USD），0 = 无限制。超限时自动降级 tier。
#[agent_command(domain = router, safety = Caution, call_mode = StateInput, description = "设置成本预算")]
#[tauri::command]
pub fn smart_router_set_cost_budget(state: State<'_, AppState>, limit_usd: f64) {
    state.cost_aware_router.set_cost_budget(limit_usd);
}
