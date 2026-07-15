// SPDX-License-Identifier: AGPL-3.0-only

//! Orchestrator Tauri 命令 —— 自然语言 → 工作流 DAG。

use crate::app_state::AppState;
use axagent_harness::workflow_types::WorkflowNode;
use axagent_orchestrator::{DynamicSubGraph, OrchestrationStrategy, OrchestratorExecutor};
use serde::{Deserialize, Serialize};
use tauri::State;

// 桥接说明（Real-time 流式管道）：
//
// 当前 `orchestrate_mission` 命令只做"分解 + 生成子图"的一次性同步返回，
// 不涉及子任务执行阶段，因此未注入 `AgentStreamReporter`。
//
// 若未来需要将子任务执行过程的流式 chunk 推送到前端，可按以下方式接入：
//
// 1. 实现 `AgentStreamReporter` trait，例如 `TauriEventStreamReporter`：
//    - 内部维护 `HashMap<agent_id, Vec<mpsc::Sender<AgentStreamChunk>>>`
//    - `report_chunk` 时查找对应 agent 的所有 sender 并 try_send
//    - 通过 `AppHandle` 把 chunk 序列化为 Tauri 事件 emit 给前端
//      （事件名如 `orchestrator://stream-chunk`）
//
// 2. 在创建 `OrchestratorExecutor` 时通过 `.with_stream_reporter(Arc::new(reporter))` 注入
//
// 3. 新增一个 `subscribe_orchestrator_stream` 命令，返回一个流式 channel
//    给前端（或直接通过 Tauri 事件推送，前端用 `listen` API 订阅）
//
// 此处暂不实现具体桥接，保留扩展点。

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

    let subgraph =
        subgraph_builder.generate(&plan).map_err(|e| format!("Subgraph generation failed: {e}"))?;

    let explanation = format!(
        "已将使命「{}」分解为 {} 个子任务，策略：{}",
        mission,
        plan.sub_tasks.len(),
        strategy.as_str(),
    );

    Ok(OrchestrateResult { nodes: subgraph.nodes, explanation })
}
