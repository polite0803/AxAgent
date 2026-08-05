// SPDX-License-Identifier: AGPL-3.0-only

//! OrchestratorExecutor — the central engine that receives a high-level
//! mission, decomposes it, generates a subgraph, submits it to the work
//! engine, monitors execution, and replans on failures.
//!
//! # Minimal closed loop
//!
//! ```text
//! decompose → generate_subgraph → execute → monitor → replan ↻
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc;

use crate::decomposer::{MissionDecomposer, RuleBasedDecomposer};
use crate::dynamic_subgraph::DynamicSubGraph;
use crate::types::{
    DecompositionPlan, OrchestrationError, OrchestrationEvent, OrchestrationStrategy,
    StructuredHandover, SubTaskStatus,
};
use axagent_harness::orchestration_dispatch::{DispatchRequest, SubTaskDispatcher};
use axagent_harness::streaming::{AgentStreamChunk, AgentStreamReporter, StreamChunkKind};
use axagent_harness::workflow_types::SubGraph;

// ── OrchestratorState ──────────────────────────────────────────────────

/// Runtime state of the orchestrator across execution rounds.
#[derive(Debug, Clone)]
pub enum OrchestratorState {
    /// Awaiting a mission.
    Idle,
    /// Decomposing mission into sub-tasks.
    Decomposing,
    /// Building the DAG subgraph.
    BuildingSubGraph,
    /// Submitting and executing the subgraph.
    Executing,
    /// Monitoring execution progress.
    Monitoring,
    /// Replanning after failures.
    Replanning,
    /// All sub-tasks completed.
    Completed,
    /// Orchestration aborted (max replans exceeded or fatal error).
    Aborted(String),
}

impl std::fmt::Display for OrchestratorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Decomposing => write!(f, "Decomposing"),
            Self::BuildingSubGraph => write!(f, "BuildingSubGraph"),
            Self::Executing => write!(f, "Executing"),
            Self::Monitoring => write!(f, "Monitoring"),
            Self::Replanning => write!(f, "Replanning"),
            Self::Completed => write!(f, "Completed"),
            Self::Aborted(reason) => write!(f, "Aborted({})", reason),
        }
    }
}

// ── Event listener callback type ──────────────────────────────────────

/// Callback invoked on orchestration events.
pub type OrchestrationEventHandler = Arc<dyn Fn(OrchestrationEvent) + Send + Sync>;

/// 将 `OrchestrationEvent` 变体映射为 PascalCase 字符串,用作 `DomainEvent.kind`。
///
/// 与 agent crate 的 `AgentEventType::to_string()` 风格保持一致,
/// 便于订阅端按 `kind` 字符串过滤。
fn orchestration_event_kind(event: &OrchestrationEvent) -> &'static str {
    match event {
        OrchestrationEvent::DecompositionStarted { .. } => "DecompositionStarted",
        OrchestrationEvent::DecompositionCompleted { .. } => "DecompositionCompleted",
        OrchestrationEvent::SubTaskDispatched { .. } => "SubTaskDispatched",
        OrchestrationEvent::SubTaskCompleted { .. } => "SubTaskCompleted",
        OrchestrationEvent::SubTaskFailed { .. } => "SubTaskFailed",
        OrchestrationEvent::ReplanTriggered { .. } => "ReplanTriggered",
        OrchestrationEvent::OrchestrationCompleted { .. } => "OrchestrationCompleted",
        OrchestrationEvent::OrchestrationAborted { .. } => "OrchestrationAborted",
    }
}

// ── OrchestratorExecutor ──────────────────────────────────────────────

/// The orchestrator executor that implements the full decomposition→execution→monitor→replan loop.
pub struct OrchestratorExecutor {
    /// Current orchestrator state.
    state: RwLock<OrchestratorState>,
    /// Current decomposition plan (None until decompose() called).
    /// Contains both the plan tree, sub-task statuses, and replan round counter
    /// within a single lock to prevent consistency issues.
    plan: RwLock<Option<DecompositionPlan>>,
    /// Dynamic subgraph builder.
    subgraph_builder: RwLock<DynamicSubGraph>,
    /// Mission decomposer strategy.
    decomposer: Box<dyn MissionDecomposer>,
    /// Event listeners notified on state transitions.
    event_listeners: RwLock<Vec<OrchestrationEventHandler>>,
    /// 可选的流式报告器 —— 用于多 Agent 实时协作场景推送流式 chunk
    stream_reporter: Option<Arc<dyn AgentStreamReporter>>,
    /// 可选的 SubTask 派发器 —— 注入后 execute_subgraph() 可实际派发执行
    dispatcher: Option<Arc<dyn SubTaskDispatcher>>,
    /// 统一事件总线（可选，由 wiring 层注入）。
    ///
    /// 注入后,`emit()` 在通知本地 listeners 之外,额外 publish 一份
    /// `DomainEvent` 到统一总线,供跨 crate 订阅者消费。
    /// 未注入时保持原有行为。用 `RwLock` 包裹以支持 `Arc<Self>` 上运行时注入。
    event_bus: RwLock<Option<Arc<dyn axagent_harness::EventBus>>>,
}

impl OrchestratorExecutor {
    /// Create a new executor with a custom decomposer strategy.
    pub fn with_decomposer(decomposer: Box<dyn MissionDecomposer>) -> Self {
        Self {
            state: RwLock::new(OrchestratorState::Idle),
            plan: RwLock::new(None),
            subgraph_builder: RwLock::new(DynamicSubGraph::new()),
            decomposer,
            event_listeners: RwLock::new(Vec::new()),
            stream_reporter: None,
            dispatcher: None,
            event_bus: RwLock::new(None),
        }
    }

    /// Create a new executor with the default rule-based decomposer.
    pub fn new() -> Self {
        Self::with_decomposer(Box::new(RuleBasedDecomposer::new()))
    }

    /// 注入流式报告器，启用实时 chunk 推送能力。
    ///
    /// 注入后，`report_sub_task_completed` / `report_sub_task_failed` 会自动向
    /// reporter 发送对应类型的 chunk；上层可通过 `subscribe_to_subtask` 订阅。
    pub fn with_stream_reporter(mut self, reporter: Arc<dyn AgentStreamReporter>) -> Self {
        self.stream_reporter = Some(reporter);
        self
    }

    /// 注入 SubTask 派发器，启用 execute_subgraph() 的实际派发能力。
    ///
    /// 由 runtime/wiring 层在初始化时注入实现方（agent 或 work_engine）。
    pub fn with_dispatcher(mut self, dispatcher: Arc<dyn SubTaskDispatcher>) -> Self {
        self.dispatcher = Some(dispatcher);
        self
    }

    /// 注入统一事件总线,启用跨 crate 事件桥接。
    ///
    /// 注入后,`emit()` 在通知本地 listeners 之外,额外 publish 一份
    /// `DomainEvent` 到统一总线。通常由 wiring 层调用。
    pub async fn set_event_bus(&self, bus: Arc<dyn axagent_harness::EventBus>) {
        let mut guard = self.event_bus.write().await;
        *guard = Some(bus);
    }

    // ── Event system ────────────────────────────────────────────────

    /// Register an event listener.
    pub async fn on_event(&self, handler: OrchestrationEventHandler) {
        self.event_listeners.write().await.push(handler);
    }

    async fn emit(&self, event: OrchestrationEvent) {
        tracing::info!(?event, "orchestrator event");
        for listener in self.event_listeners.read().await.iter() {
            listener(event.clone());
        }

        // 桥接到统一事件总线(若已注入):把 OrchestrationEvent 转为 DomainEvent
        let bus_clone = {
            let guard = self.event_bus.read().await;
            guard.as_ref().map(Arc::clone)
        };
        if let Some(bus) = bus_clone {
            let kind = orchestration_event_kind(&event);
            let payload = serde_json::to_value(&event).unwrap_or(serde_json::Value::Null);
            let domain_event = axagent_harness::DomainEvent::new(
                axagent_harness::EventCategory::Orchestration,
                kind,
                payload,
                "orchestrator",
            );
            bus.publish(domain_event).await;
        }
    }

    // ── State management ────────────────────────────────────────────

    pub async fn current_state(&self) -> OrchestratorState {
        self.state.read().await.clone()
    }

    async fn transition(&self, new_state: OrchestratorState) {
        let old = {
            let mut state = self.state.write().await;
            let old = state.clone();
            *state = new_state.clone();
            old
        };
        tracing::info!(from = %old, to = %new_state, "orchestrator state transition");
    }

    // ── Core loop: decompose → generate → execute → monitor → replan ─

    /// Execute the full orchestration loop for a given mission.
    ///
    /// This is the main entry point. It:
    /// 1. Decomposes the mission into sub-tasks (rule-based for now)
    /// 2. Generates a DAG subgraph
    /// 3. Returns the subgraph for the caller (work engine) to execute
    /// 4. Accepts updates on sub-task completion/failure
    /// 5. Automatically replans on failures
    pub async fn receive_mission(
        &self,
        mission: &str,
        strategy: OrchestrationStrategy,
    ) -> Result<DecompositionPlan, OrchestrationError> {
        tracing::info!(mission, ?strategy, "orchestrator received mission");

        self.transition(OrchestratorState::Decomposing).await;

        let plan = self.decompose(mission, strategy)?;

        {
            let mut p = self.plan.write().await;
            *p = Some(plan.clone());
        }

        self.emit(OrchestrationEvent::DecompositionStarted {
            mission: mission.to_string(),
            strategy: strategy.as_str().to_string(),
        })
        .await;

        self.emit(OrchestrationEvent::DecompositionCompleted {
            sub_task_count: plan.sub_tasks.len(),
            plan: plan.clone(),
        })
        .await;

        self.transition(OrchestratorState::BuildingSubGraph).await;
        Ok(plan)
    }

    /// Generate the executable subgraph from the current plan.
    pub async fn generate_subgraph(&self) -> Result<SubGraph, OrchestrationError> {
        let plan = {
            let p = self.plan.read().await;
            p.clone().ok_or_else(|| {
                OrchestrationError::InvalidConfig(
                    "No plan — call receive_mission first".to_string(),
                )
            })?
        };

        let generated = {
            let mut builder = self.subgraph_builder.write().await;
            builder.generate(&plan)?
        };

        let workflow = generated.to_workflow();
        tracing::info!(
            nodes = workflow.nodes.len(),
            edges = workflow.edges.len(),
            "subgraph generated"
        );

        self.transition(OrchestratorState::Executing).await;
        Ok(workflow)
    }

    /// 执行阶段：把 ready 的 SubTask 派发给执行方，等待结果并自动更新状态。
    ///
    /// 需要先调用 `receive_mission` 和 `generate_subgraph`。
    /// 若未注入 `SubTaskDispatcher`，返回 `OrchestrationError::InvalidConfig`。
    ///
    /// ## 执行流程
    ///
    /// 1. 遍历 plan，找出所有 `Ready` 状态的 SubTask（依赖已满足）
    /// 2. 对每个 Ready SubTask 发出 `SubTaskDispatched` 事件（修复 G9）
    /// 3. 调用 `dispatcher.dispatch()` 派发执行
    /// 4. 根据 `DispatchResult.success` 调用 `report_sub_task_completed/failed`
    /// 5. 返回最终的 plan 快照（可能已触发 replan）
    pub async fn execute_subgraph(&self) -> Result<Option<DecompositionPlan>, OrchestrationError> {
        let dispatcher = self.dispatcher.as_ref().ok_or_else(|| {
            OrchestrationError::InvalidConfig(
                "No SubTaskDispatcher injected — call with_dispatcher() first".to_string(),
            )
        })?;

        self.transition(OrchestratorState::Executing).await;

        // 循环派发：每轮收集 ready SubTask → dispatch → 处理结果
        // 直至 plan 终态或无 ready 任务（依赖链解锁后下一轮可继续）
        loop {
            // 检查 plan 是否已终态
            let is_terminal = {
                let plan_guard = self.plan.read().await;
                plan_guard.as_ref().map(|p| p.is_terminal()).unwrap_or(true)
            };
            if is_terminal {
                break;
            }

            // 收集 ready sub_tasks（快照后释放锁，避免 dispatch 中长时间持锁）
            let ready_tasks: Vec<DispatchRequest> = {
                let plan_guard = self.plan.read().await;
                let plan = plan_guard.as_ref().ok_or_else(|| {
                    OrchestrationError::InvalidConfig(
                        "No plan — call receive_mission first".to_string(),
                    )
                })?;

                plan.sub_tasks
                    .iter()
                    .filter(|st| st.status == SubTaskStatus::Pending)
                    .filter(|st| {
                        // 依赖全部 Completed
                        st.dependencies.iter().all(|dep_id| {
                            plan.sub_tasks
                                .iter()
                                .find(|o| o.id == *dep_id)
                                .map(|dep| dep.status == SubTaskStatus::Completed)
                                .unwrap_or(false)
                        })
                    })
                    .map(|st| DispatchRequest {
                        sub_task_id: st.id.clone(),
                        mission: st.description.clone(),
                        role: st.role.clone(),
                        system_prompt: st.system_prompt.clone(),
                        tools: st.tools.clone(),
                        output_var: st.output_var.clone(),
                    })
                    .collect()
            };

            if ready_tasks.is_empty() {
                // 无 ready 任务但 plan 未终态：可能存在依赖循环或全部在 Running
                tracing::info!("execute_subgraph: no ready sub-tasks but plan not terminal");
                break;
            }

            // 标记为 Running 并发出 SubTaskDispatched 事件（修复 G9）
            for req in &ready_tasks {
                self.update_sub_task_status(&req.sub_task_id, SubTaskStatus::Running).await?;
                self.emit(OrchestrationEvent::SubTaskDispatched {
                    sub_task_id: req.sub_task_id.clone(),
                    worker_node_id: req.sub_task_id.clone(),
                })
                .await;
            }

            // 批量派发（dispatcher 内部可并行）
            let results = dispatcher.dispatch_batch(ready_tasks).await.map_err(|e| {
                OrchestrationError::DispatchFailed(format!("batch dispatch failed: {e}"))
            })?;

            // 处理结果：直接更新状态，避免循环内反复触发 monitor_and_maybe_replan
            // （monitor 会在循环结束后统一调用，完成 terminal/replan/Completed 转换）
            for result in results {
                if result.success {
                    let handover = result.handover_json.as_deref().and_then(|s| {
                        serde_json::from_str::<StructuredHandover>(s).ok().or_else(|| {
                            // 非 StructuredHandover 格式时，包装为通用 handover
                            Some(StructuredHandover {
                                completed_work: s.to_string(),
                                changes: vec![],
                                next_steps: String::new(),
                                remaining_issues: String::new(),
                                dependencies: String::new(),
                                validation_evidence: String::new(),
                            })
                        })
                    });
                    self.finalize_sub_task_completed(&result.sub_task_id, handover).await?;
                } else {
                    let err = result.error.unwrap_or_else(|| "unknown error".to_string());
                    self.finalize_sub_task_failed(&result.sub_task_id, &err).await?;
                }
            }
            // 下一轮：已完成的依赖会解锁后续 Pending task
        }

        // 循环结束：统一调用 monitor 完成 terminal 检查 + replan/Completed 转换
        // 忽略 monitor 返回值（其 Ok(None) 表示无 replan；调用方仍需 plan 快照）
        let _ = self.monitor_and_maybe_replan().await?;

        // 返回最终 plan 快照（可能已被 replan 更新）
        let plan_guard = self.plan.read().await;
        Ok(plan_guard.clone())
    }

    /// 内部方法：完成 SubTask 的成功收尾（状态更新 + 流式推送 + 事件发射）。
    ///
    /// 与 `report_sub_task_completed` 的区别：不触发 `monitor_and_maybe_replan`，
    /// 供 `execute_subgraph` 循环内调用以避免重复的状态转换。
    async fn finalize_sub_task_completed(
        &self,
        sub_task_id: &str,
        handover: Option<StructuredHandover>,
    ) -> Result<(), OrchestrationError> {
        self.update_sub_task_status(sub_task_id, SubTaskStatus::Completed).await?;

        // 如有流式报告器，推送 Completed chunk
        if let Some(ref reporter) = self.stream_reporter {
            let chunk = AgentStreamChunk {
                agent_id: sub_task_id.to_string(),
                sub_task_id: sub_task_id.to_string(),
                kind: StreamChunkKind::Completed,
                payload: serde_json::json!({
                    "handover": handover.as_ref().map(|h| format!("{:?}", h)).unwrap_or_default(),
                }),
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            reporter.report_chunk(chunk);
        }

        self.emit(OrchestrationEvent::SubTaskCompleted {
            sub_task_id: sub_task_id.to_string(),
            handover,
        })
        .await;
        Ok(())
    }

    /// 内部方法：完成 SubTask 的失败收尾（状态更新 + error 字段 + 流式推送 + 事件发射）。
    ///
    /// 与 `report_sub_task_failed` 的区别：不触发 `monitor_and_maybe_replan`，
    /// 供 `execute_subgraph` 循环内调用以避免重复的状态转换。
    async fn finalize_sub_task_failed(
        &self,
        sub_task_id: &str,
        error: &str,
    ) -> Result<(), OrchestrationError> {
        self.update_sub_task_status(sub_task_id, SubTaskStatus::Failed).await?;

        // Update error field in plan
        {
            let mut plan_guard = self.plan.write().await;
            if let Some(ref mut plan) = *plan_guard
                && let Some(st) = plan.sub_tasks.iter_mut().find(|s| s.id == sub_task_id)
            {
                st.error = Some(error.to_string());
            }
        }

        // 如有流式报告器，推送 Failed chunk
        if let Some(ref reporter) = self.stream_reporter {
            let chunk = AgentStreamChunk {
                agent_id: sub_task_id.to_string(),
                sub_task_id: sub_task_id.to_string(),
                kind: StreamChunkKind::Failed,
                payload: serde_json::json!({ "error": error }),
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            reporter.report_chunk(chunk);
        }

        self.emit(OrchestrationEvent::SubTaskFailed {
            sub_task_id: sub_task_id.to_string(),
            error: error.to_string(),
        })
        .await;
        Ok(())
    }

    /// The caller reports that a sub-task has completed.
    ///
    /// If failures exist, auto-triggers replanning if within max rounds.
    pub async fn report_sub_task_completed(
        &self,
        sub_task_id: &str,
        handover: Option<StructuredHandover>,
    ) -> Result<Option<DecompositionPlan>, OrchestrationError> {
        self.update_sub_task_status(sub_task_id, SubTaskStatus::Completed).await?;

        // 如有流式报告器，推送 Completed chunk
        if let Some(ref reporter) = self.stream_reporter {
            let chunk = AgentStreamChunk {
                agent_id: sub_task_id.to_string(),
                sub_task_id: sub_task_id.to_string(),
                kind: StreamChunkKind::Completed,
                payload: serde_json::json!({
                    "handover": handover.as_ref().map(|h| format!("{:?}", h)).unwrap_or_default(),
                }),
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            reporter.report_chunk(chunk);
        }

        self.emit(OrchestrationEvent::SubTaskCompleted {
            sub_task_id: sub_task_id.to_string(),
            handover,
        })
        .await;

        self.monitor_and_maybe_replan().await
    }

    /// The caller reports that a sub-task has failed.
    pub async fn report_sub_task_failed(
        &self,
        sub_task_id: &str,
        error: &str,
    ) -> Result<Option<DecompositionPlan>, OrchestrationError> {
        self.update_sub_task_status(sub_task_id, SubTaskStatus::Failed).await?;

        // Update error field in plan
        {
            let mut plan_guard = self.plan.write().await;
            if let Some(ref mut plan) = *plan_guard
                && let Some(st) = plan.sub_tasks.iter_mut().find(|s| s.id == sub_task_id)
            {
                st.error = Some(error.to_string());
            }
        }

        // 如有流式报告器，推送 Failed chunk
        if let Some(ref reporter) = self.stream_reporter {
            let chunk = AgentStreamChunk {
                agent_id: sub_task_id.to_string(),
                sub_task_id: sub_task_id.to_string(),
                kind: StreamChunkKind::Failed,
                payload: serde_json::json!({ "error": error }),
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            reporter.report_chunk(chunk);
        }

        self.emit(OrchestrationEvent::SubTaskFailed {
            sub_task_id: sub_task_id.to_string(),
            error: error.to_string(),
        })
        .await;

        self.monitor_and_maybe_replan().await
    }

    /// 订阅指定子任务（以 sub_task_id 作为 agent_id）的流式输出。
    ///
    /// 返回 `mpsc::Receiver<AgentStreamChunk>`，调用方可在异步上下文中持续接收 chunk。
    /// 若未注入 stream_reporter，返回的 receiver 永远不会收到消息。
    pub fn subscribe_to_subtask(&self, sub_task_id: &str) -> mpsc::Receiver<AgentStreamChunk> {
        match &self.stream_reporter {
            Some(reporter) => reporter.subscribe(sub_task_id),
            None => {
                // 无 reporter 时返回空 receiver（sender 立即丢弃，recv 会阻塞至取消）
                let (_tx, rx) = mpsc::channel(1);
                rx
            },
        }
    }

    /// Check plan status and trigger replan if needed.
    pub async fn monitor_and_maybe_replan(
        &self,
    ) -> Result<Option<DecompositionPlan>, OrchestrationError> {
        self.transition(OrchestratorState::Monitoring).await;

        let plan = {
            let p = self.plan.read().await;
            p.clone().ok_or_else(|| {
                OrchestrationError::InvalidConfig("No plan to monitor".to_string())
            })?
        };

        if plan.is_terminal() {
            let failed = plan.failed_count();
            if failed > 0 {
                // Trigger replan
                let replan_count = plan.replan_count;

                if replan_count >= plan.max_replans {
                    self.transition(OrchestratorState::Aborted(format!(
                        "Max replan rounds ({}) exceeded",
                        plan.max_replans
                    )))
                    .await;

                    self.emit(OrchestrationEvent::OrchestrationAborted {
                        reason: format!(
                            "Max replan rounds ({}) exceeded with {} failed tasks",
                            plan.max_replans, failed
                        ),
                    })
                    .await;

                    return Err(OrchestrationError::MaxReplansExceeded(plan.max_replans));
                }

                // Collect failed sub-tasks
                let failed_ids: Vec<String> = plan
                    .sub_tasks
                    .iter()
                    .filter(|st| st.status == SubTaskStatus::Failed)
                    .map(|st| st.id.clone())
                    .collect();

                self.transition(OrchestratorState::Replanning).await;

                // Increment replan count inside the plan lock
                {
                    let mut p = self.plan.write().await;
                    if let Some(ref mut p) = *p {
                        p.replan_count += 1;
                    }
                }

                let current_replan_count = {
                    let p = self.plan.read().await;
                    p.as_ref().map(|p| p.replan_count).unwrap_or(0)
                };

                self.emit(OrchestrationEvent::ReplanTriggered {
                    failed_sub_tasks: failed_ids.clone(),
                    replan_round: current_replan_count,
                })
                .await;

                let new_plan = self.replan(&failed_ids).await?;
                return Ok(Some(new_plan));
            }

            // All completed
            self.transition(OrchestratorState::Completed).await;

            self.emit(OrchestrationEvent::OrchestrationCompleted {
                total_sub_tasks: plan.sub_tasks.len(),
                completed: plan.completed_count(),
                failed: 0,
            })
            .await;

            Ok(None)
        } else {
            // Still in progress
            let ready = plan.ready_sub_tasks();
            tracing::info!(
                ready = ready.len(),
                completed = plan.completed_count(),
                total = plan.sub_tasks.len(),
                "orchestrator monitoring: {} ready tasks",
                ready.len()
            );
            Ok(None)
        }
    }

    // ── Private methods ──────────────────────────────────────────────

    /// Decompose a mission into sub-tasks using the configured strategy.
    fn decompose(
        &self,
        mission: &str,
        strategy: OrchestrationStrategy,
    ) -> Result<DecompositionPlan, OrchestrationError> {
        let plan = self.decomposer.decompose(mission, strategy)?;
        tracing::info!(
            sub_tasks = plan.sub_tasks.len(),
            strategy = strategy.as_str(),
            "decomposition complete"
        );
        Ok(plan)
    }

    /// Replan: create a new plan subset that retries failed tasks.
    async fn replan(&self, failed_ids: &[String]) -> Result<DecompositionPlan, OrchestrationError> {
        let plan_guard = self.plan.read().await;
        let old_plan = plan_guard.as_ref().ok_or_else(|| {
            OrchestrationError::ReplanFailed("No existing plan to replan".to_string())
        })?;

        let mut new_plan =
            DecompositionPlan::new(format!("[REPLAN] {}", old_plan.mission), old_plan.strategy);
        new_plan.max_parallel = old_plan.max_parallel;
        new_plan.max_replans = old_plan.max_replans;
        // 保留复计划数，否则每次 replan 会新建 replan_count=0 的 plan 覆盖
        new_plan.replan_count = old_plan.replan_count;

        // Collect failed and not-yet-completed tasks for replanning
        for st in &old_plan.sub_tasks {
            if st.status == SubTaskStatus::Completed || st.status == SubTaskStatus::Skipped {
                // Preserve completed tasks as-is (they won't re-execute)
                new_plan.sub_tasks.push(st.clone());
            } else if st.status == SubTaskStatus::Failed
                && (failed_ids.is_empty() || failed_ids.contains(&st.id))
            {
                // Retry with reset status
                let mut retry = st.clone();
                retry.status = SubTaskStatus::Pending;
                retry.attempts += 1;
                retry.error = None;
                new_plan.sub_tasks.push(retry);
            } else if st.status == SubTaskStatus::Pending
                || st.status == SubTaskStatus::Ready
                || st.status == SubTaskStatus::Running
            {
                // Carry forward in-flight tasks (status will be reset by caller)
                new_plan.sub_tasks.push(st.clone());
            }
        }

        tracing::info!(
            original = old_plan.sub_tasks.len(),
            replanned = new_plan.sub_tasks.len(),
            failed = failed_ids.len(),
            "replan completed"
        );

        // Update plan
        drop(plan_guard);
        {
            let mut p = self.plan.write().await;
            *p = Some(new_plan.clone());
        }

        Ok(new_plan)
    }

    /// Update a sub-task's status in the plan.
    async fn update_sub_task_status(
        &self,
        sub_task_id: &str,
        new_status: SubTaskStatus,
    ) -> Result<(), OrchestrationError> {
        let mut plan_guard = self.plan.write().await;
        let plan = plan_guard
            .as_mut()
            .ok_or_else(|| OrchestrationError::InvalidConfig("No plan".to_string()))?;

        let sub_task = plan
            .sub_tasks
            .iter_mut()
            .find(|st| st.id == sub_task_id)
            .ok_or_else(|| OrchestrationError::SubTaskNotFound(sub_task_id.to_string()))?;

        sub_task.status = new_status;
        Ok(())
    }

    /// Get a snapshot of all sub-task statuses.
    pub async fn status_snapshot(&self) -> HashMap<String, String> {
        self.plan
            .read()
            .await
            .as_ref()
            .map(|p| p.sub_tasks.iter().map(|st| (st.id.clone(), st.status.to_string())).collect())
            .unwrap_or_default()
    }
}

impl Default for OrchestratorExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::orchestration_dispatch::SubTaskDispatchResult;

    /// 测试用 Mock 派发器 — 所有任务都成功返回空 handover
    struct MockDispatcher;

    #[async_trait::async_trait]
    impl SubTaskDispatcher for MockDispatcher {
        async fn dispatch(
            &self,
            request: DispatchRequest,
        ) -> axagent_harness::core_error::Result<SubTaskDispatchResult> {
            Ok(SubTaskDispatchResult {
                sub_task_id: request.sub_task_id,
                success: true,
                handover_json: Some(
                    serde_json::json!({
                        "completed_work": "mock completed",
                        "changes": [],
                        "next_steps": "",
                        "remaining_issues": "",
                        "dependencies": "",
                        "validation_evidence": ""
                    })
                    .to_string(),
                ),
                error: None,
            })
        }
    }

    #[tokio::test]
    async fn test_execute_subgraph_with_mock_dispatcher() {
        let executor = OrchestratorExecutor::new().with_dispatcher(Arc::new(MockDispatcher));
        executor.receive_mission("Quick fix", OrchestrationStrategy::Ordered).await.unwrap();
        executor.generate_subgraph().await.unwrap();

        // 执行后所有 SubTask 应该都成功完成
        let final_plan = executor.execute_subgraph().await.unwrap();
        assert!(final_plan.is_some(), "execute_subgraph should return final plan");

        let state = executor.current_state().await;
        assert!(matches!(state, OrchestratorState::Completed), "Expected Completed, got {state}");
    }

    #[tokio::test]
    async fn test_execute_subgraph_without_dispatcher_errors() {
        let executor = OrchestratorExecutor::new();
        executor.receive_mission("Quick fix", OrchestrationStrategy::Ordered).await.unwrap();

        let result = executor.execute_subgraph().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestrationError::InvalidConfig(_)));
    }

    #[tokio::test]
    async fn test_decompose_code_mission() {
        let executor = OrchestratorExecutor::new();
        let plan = executor
            .receive_mission("Implement user authentication", OrchestrationStrategy::Ordered)
            .await
            .unwrap();

        assert_eq!(plan.sub_tasks.len(), 3); // Default pattern
        assert!(plan.sub_tasks[0].dependencies.is_empty()); // analyze has no deps
        assert_eq!(plan.sub_tasks[1].dependencies, vec!["analyze"]); // implement depends on analyze
    }

    #[tokio::test]
    async fn test_decompose_refactor_mission() {
        let executor = OrchestratorExecutor::new();
        let plan = executor
            .receive_mission("Refactor the database layer", OrchestrationStrategy::Ordered)
            .await
            .unwrap();

        assert_eq!(plan.sub_tasks.len(), 9); // Refactor pattern (5 阶段)
        assert!(plan.sub_tasks.iter().any(|t| t.role == "planner"));
        assert!(plan.sub_tasks.iter().any(|t| t.role == "developer"));
    }

    #[tokio::test]
    async fn test_generate_subgraph() {
        let executor = OrchestratorExecutor::new();
        executor.receive_mission("Fix login bug", OrchestrationStrategy::Ordered).await.unwrap();

        let graph = executor.generate_subgraph().await.unwrap();
        assert_eq!(graph.nodes.len(), 3);
        // Should have 2 edges (analyze→implement, implement→review)
        assert!(graph.edges.len() >= 2);
    }

    #[tokio::test]
    async fn test_report_completed_and_terminal() {
        let executor = OrchestratorExecutor::new();
        executor.receive_mission("Quick fix", OrchestrationStrategy::Ordered).await.unwrap();

        // Complete first two
        let result = executor.report_sub_task_completed("analyze", None).await.unwrap();
        assert!(result.is_none()); // Not terminal yet

        let result = executor.report_sub_task_completed("implement", None).await.unwrap();
        assert!(result.is_none()); // Still not terminal

        // Complete last
        let result = executor.report_sub_task_completed("review", None).await.unwrap();
        assert!(result.is_none()); // Terminal but no failures

        let state = executor.current_state().await;
        assert!(matches!(state, OrchestratorState::Completed));
    }

    #[tokio::test]
    async fn test_replan_on_failure() {
        let executor = OrchestratorExecutor::new();
        let plan =
            executor.receive_mission("Quick fix", OrchestrationStrategy::Ordered).await.unwrap();

        // Collect actual sub-task IDs from the plan
        let all_ids: Vec<String> = plan.sub_tasks.iter().map(|st| st.id.clone()).collect();

        // Fail ALL sub-tasks so the plan becomes terminal
        for id in &all_ids {
            executor.update_sub_task_status(id, SubTaskStatus::Failed).await.unwrap();
        }

        // Trigger replan — plan is terminal with failures
        let result = executor.monitor_and_maybe_replan().await.unwrap();

        // Should trigger replan since all terminal with failures
        assert!(result.is_some());
        let new_plan = result.unwrap();
        // Failed analyze should be reset to pending
        let retried = new_plan.sub_tasks.iter().find(|st| st.id == "analyze").unwrap();
        assert_eq!(retried.status, SubTaskStatus::Pending);
        assert_eq!(retried.attempts, 1);

        let state = executor.current_state().await;
        assert!(matches!(state, OrchestratorState::Replanning));
    }

    #[tokio::test]
    async fn test_max_replans_exceeded() {
        let executor = OrchestratorExecutor::new();
        let plan = executor
            .receive_mission("Impossible task", OrchestrationStrategy::Ordered)
            .await
            .unwrap();

        let max_replans = plan.max_replans;

        // Collect all sub_task ids from the plan
        let all_ids: Vec<String> = plan.sub_tasks.iter().map(|st| st.id.clone()).collect();

        // Each round: fail ALL sub_tasks, then monitor triggers replan
        for _round in 0..=max_replans {
            // Fail all sub-tasks to make the plan terminal
            for id in &all_ids {
                executor.update_sub_task_status(id, SubTaskStatus::Failed).await.unwrap();
            }

            // Trigger replan check. The first max_replans rounds succeed
            let result = executor.monitor_and_maybe_replan().await;
            if let Err(e) = &result {
                assert!(
                    matches!(e, OrchestrationError::MaxReplansExceeded(_)),
                    "Expected MaxReplansExceeded, got {:?}",
                    e
                );
                let state = executor.current_state().await;
                assert!(matches!(state, OrchestratorState::Aborted(_)));
                return;
            }
        }

        panic!("Expected MaxReplansExceeded but never got it");
    }
}
