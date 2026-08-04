// SPDX-License-Identifier: AGPL-3.0-only

//! Orchestrator Tauri 命令 —— 自然语言 → 工作流 DAG。
//!
//! 同时提供 `TauriEventStreamReporter` 实现，桥接 `AgentStreamReporter` trait
//! 与 Tauri 事件系统，使子任务执行过程的流式 chunk 可推送到前端。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::app_state::AppState;
use agent_macro::agent_command;
use axagent_harness::streaming::{AgentStreamChunk, AgentStreamReporter};
use axagent_harness::workflow_types::WorkflowNode;
use axagent_orchestrator::{DynamicSubGraph, OrchestrationStrategy, OrchestratorExecutor};
use axagent_runtime::RuntimeSubTaskDispatcher;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{broadcast, mpsc};

// ── TauriEventStreamReporter ──────────────────────────────────────

/// Tauri 事件流式报告器 —— 将 `AgentStreamReporter` trait 桥接到 Tauri 事件系统。
///
/// 工作原理：
/// - `report_chunk` 时通过 `broadcast::Sender` 广播给对应 agent 的订阅者，
///   同时通过 `AppHandle::emit` 推送 Tauri 事件给前端
/// - `subscribe` 时创建（或复用）对应 agent 的 broadcast channel，
///   并 spawn 一个转发 task 将 broadcast receiver 转为 mpsc receiver
///
/// 事件名：`orchestrator://stream-chunk`，payload 为 `AgentStreamChunk` JSON
pub struct TauriEventStreamReporter {
    /// Tauri 应用句柄，用于向前端 emit 事件
    app_handle: Option<AppHandle>,
    /// 每个 agent_id 对应一个 broadcast sender
    // 注意：此处使用 std::sync::RwLock 而非 tokio::sync::RwLock，
    // 因为 AgentStreamReporter trait 的 report_chunk/subscribe 方法是同步签名，
    // 无法在方法体内 .await。此锁仅保护 HashMap 查找，不跨 await，
    // 操作极快，不存在 std guard 跨 await 的 UB 风险。
    channels: RwLock<HashMap<String, broadcast::Sender<AgentStreamChunk>>>,
}

impl TauriEventStreamReporter {
    /// 创建一个不绑定 AppHandle 的报告器（仅内部 channel 广播，不 emit Tauri 事件）
    pub fn new() -> Self {
        Self { app_handle: None, channels: RwLock::new(HashMap::new()) }
    }

    /// 绑定 AppHandle，使 chunk 同时通过 Tauri 事件推送前端
    pub fn with_app_handle(app_handle: AppHandle) -> Self {
        Self { app_handle: Some(app_handle), channels: RwLock::new(HashMap::new()) }
    }

    /// 流式事件名常量
    pub const EVENT_NAME: &'static str = "orchestrator://stream-chunk";
}

impl Default for TauriEventStreamReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentStreamReporter for TauriEventStreamReporter {
    fn report_chunk(&self, chunk: AgentStreamChunk) {
        // 1. 广播给内部订阅者
        {
            let channels = self.channels.read().unwrap_or_else(|e| {
                tracing::warn!(
                    "TauriEventStreamReporter channels lock poisoned, recovering: {}",
                    e
                );
                e.into_inner()
            });
            if let Some(sender) = channels.get(&chunk.agent_id) {
                // broadcast::send 是同步方法，返回接收者数量
                let _ = sender.send(chunk.clone());
            }
        }

        // 2. 通过 Tauri 事件推送给前端
        if let Some(ref handle) = self.app_handle {
            if let Err(e) = handle.emit(Self::EVENT_NAME, &chunk) {
                tracing::warn!(error = %e, "emit stream-chunk 事件失败");
            }
        }
    }

    fn subscribe(&self, agent_id: &str) -> mpsc::Receiver<AgentStreamChunk> {
        let broadcast_rx = {
            let mut channels = self.channels.write().unwrap_or_else(|e| {
                tracing::warn!(
                    "TauriEventStreamReporter channels lock poisoned, recovering: {}",
                    e
                );
                e.into_inner()
            });
            let sender = channels.entry(agent_id.to_string()).or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(256);
                tx
            });
            sender.subscribe()
        };

        // spawn 转发 task：broadcast::Receiver → mpsc::Receiver
        let (mpsc_tx, mpsc_rx) = mpsc::channel(256);
        tokio::spawn(async move {
            let mut broadcast_rx = broadcast_rx;
            loop {
                match broadcast_rx.recv().await {
                    Ok(chunk) => {
                        if mpsc_tx.send(chunk).await.is_err() {
                            // mpsc 接收端已关闭，停止转发
                            break;
                        }
                    },
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "stream 订阅者落后，跳过部分 chunk");
                        continue;
                    },
                    Err(broadcast::error::RecvError::Closed) => {
                        // broadcast sender 全部关闭，停止转发
                        break;
                    },
                }
            }
        });

        mpsc_rx
    }
}

// ── Tauri 命令 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrateResult {
    pub nodes: Vec<WorkflowNode>,
    pub explanation: String,
}

/// 接收自然语言使命描述，经 Orchestrator 分解为子任务 DAG 并返回。
#[agent_command(domain = "orchestrator", safety = Safe, call_mode = StateInput, description = "将使命分解为子任务工作流")]
#[tauri::command]
pub async fn orchestrate_mission(
    state: State<'_, AppState>,
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

    // 构建 executor：优先注入 LlmBasedDecomposer（语义化分解），降级到 RuleBasedDecomposer
    let executor = {
        let master_key = state.harness.master_key_owned();
        crate::init::llm_providers::build_llm_decomposer_from_db(&master_key).await.map_or_else(
            || {
                tracing::info!("[orchestrate] 未配置可用 LLM provider，降级到 RuleBasedDecomposer");
                OrchestratorExecutor::new()
            },
            OrchestratorExecutor::with_decomposer,
        )
    }
    .with_dispatcher(Arc::new(RuntimeSubTaskDispatcher::noop()));
    // 注入统一事件总线(与 agent / rt-workflow 共享同一份 Arc<dyn EventBus>),
    // 让 orchestrator 的分解 / 状态迁移事件可被跨 crate 订阅者消费。
    executor.set_event_bus(Arc::clone(&state.event_bus)).await;
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

/// 订阅指定 agent 的流式输出。
///
/// 返回一个 Tauri channel，前端可通过 `invoke` + `onEvent` 持续接收 `AgentStreamChunk`。
/// 每次调用创建独立的订阅，互不干扰。
#[agent_command(domain = "orchestrator", safety = Safe, call_mode = StateInput, description = "订阅编排器流式输出")]
#[tauri::command]
pub async fn subscribe_orchestrator_stream(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<(), String> {
    let reporter = state
        .stream_reporter
        .read()
        .await
        .clone()
        .ok_or_else(|| "Stream reporter 未初始化".to_string())?;

    let mut rx = reporter.subscribe(&agent_id);

    // spawn 消费 task：将 mpsc 中的 chunk 通过 Tauri 事件推送前端
    // 前端通过 listen("orchestrator://stream-chunk") 接收
    tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            // chunk 已经在 report_chunk 中 emit 过了，这里不需要重复 emit
            // 此 task 仅用于保持 channel 存活，直到前端取消订阅
            tracing::trace!(agent_id = %chunk.agent_id, kind = ?chunk.kind, "stream chunk 已转发");
        }
    });

    Ok(())
}

/// 将 reporter 注入到 AppState（在初始化阶段调用）
pub fn create_stream_reporter(app_handle: AppHandle) -> Arc<TauriEventStreamReporter> {
    Arc::new(TauriEventStreamReporter::with_app_handle(app_handle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reporter_new() {
        let reporter = TauriEventStreamReporter::new();
        // 不应 panic
        let chunk = AgentStreamChunk {
            agent_id: "agent-1".to_string(),
            sub_task_id: "task-1".to_string(),
            kind: axagent_harness::streaming::StreamChunkKind::TextDelta,
            payload: serde_json::json!({"text": "hello"}),
            timestamp: 1700000000,
        };
        reporter.report_chunk(chunk);
    }

    #[tokio::test]
    async fn test_subscribe_and_receive() {
        let reporter = Arc::new(TauriEventStreamReporter::new());
        let mut rx = reporter.subscribe("agent-1");

        let chunk = AgentStreamChunk {
            agent_id: "agent-1".to_string(),
            sub_task_id: "task-1".to_string(),
            kind: axagent_harness::streaming::StreamChunkKind::Progress,
            payload: serde_json::json!({"percent": 50}),
            timestamp: 1700000000,
        };

        reporter.report_chunk(chunk.clone());

        let received = rx.recv().await.expect("应收到 chunk");
        assert_eq!(received.agent_id, "agent-1");
        assert_eq!(received.sub_task_id, "task-1");
    }

    #[tokio::test]
    async fn test_subscribe_no_agent() {
        let reporter = TauriEventStreamReporter::new();
        // 订阅不存在的 agent，然后发送 chunk 给另一个 agent，不应收到
        let mut rx = reporter.subscribe("agent-x");

        let chunk = AgentStreamChunk {
            agent_id: "agent-other".to_string(),
            sub_task_id: "task-1".to_string(),
            kind: axagent_harness::streaming::StreamChunkKind::TextDelta,
            payload: serde_json::json!({"text": "hello"}),
            timestamp: 1700000000,
        };
        reporter.report_chunk(chunk);

        // 应该超时或收到 None（因为 chunk 发给了别的 agent）
        let result = tokio::time::timeout(tokio::time::Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_err() || result.unwrap().is_none(), "不应收到其他 agent 的 chunk");
    }
}
