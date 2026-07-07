// SPDX-License-Identifier: AGPL-3.0-only

//! Orchestrator Tauri 命令 —— 自然语言 → 工作流 DAG。

use crate::app_state::AppState;
use axagent_harness::workflow_types::WorkflowNode;
use axagent_orchestrator::{DynamicSubGraph, OrchestrationStrategy, OrchestratorExecutor};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrateResult {
    pub nodes: Vec<WorkflowNode>,
    pub explanation: String,
}

/// 接收自然语言使命描述，经 Orchestrator 分解为子任务 DAG 并返回。
#[tauri::command]
pub async fn orchestrate_mission(
    _state: State<'_, AppState>,
    mission: String,
    strategy: Option<String>,
) -> Result<OrchestrateResult, String> {
    let strategy = match strategy.as_deref() {
        Some("fan_out") => OrchestrationStrategy::FanOut,
        Some("pipeline") => OrchestrationStrategy::Pipeline,
        Some("race") => OrchestrationStrategy::Race,
        Some("debate") => OrchestrationStrategy::Debate,
        Some("dynamic") => OrchestrationStrategy::Dynamic,
        _ => OrchestrationStrategy::Ordered,
    };

    let mut subgraph_builder = DynamicSubGraph::new();
    let executor = OrchestratorExecutor::new();
    let plan = executor
        .receive_mission(&mission, strategy)
        .await
        .map_err(|e| format!("Orchestrator decompose failed: {e}"))?;

    let subgraph = subgraph_builder
        .generate(&plan)
        .map_err(|e| format!("Subgraph generation failed: {e}"))?;

    let explanation = format!(
        "已将使命「{}」分解为 {} 个子任务，策略：{}",
        mission,
        plan.sub_tasks.len(),
        strategy.as_str(),
    );

    Ok(OrchestrateResult {
        nodes: subgraph.nodes,
        explanation,
    })
}
