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

use crate::decomposer::{MissionDecomposer, RuleBasedDecomposer};
use crate::dynamic_subgraph::DynamicSubGraph;
use crate::types::{
    DecompositionPlan, OrchestrationError, OrchestrationEvent, OrchestrationStrategy,
    StructuredHandover, SubTaskStatus,
};
use axagent_core::workflow_types::{AgentRole, SubGraph};

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
        }
    }

    /// Create a new executor with the default rule-based decomposer.
    pub fn new() -> Self {
        Self::with_decomposer(Box::new(RuleBasedDecomposer::new()))
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

    /// The caller reports that a sub-task has completed.
    ///
    /// If failures exist, auto-triggers replanning if within max rounds.
    pub async fn report_sub_task_completed(
        &self,
        sub_task_id: &str,
        handover: Option<StructuredHandover>,
    ) -> Result<Option<DecompositionPlan>, OrchestrationError> {
        self.update_sub_task_status(sub_task_id, SubTaskStatus::Completed)
            .await?;

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
        self.update_sub_task_status(sub_task_id, SubTaskStatus::Failed)
            .await?;

        // Update error field in plan
        {
            let mut plan_guard = self.plan.write().await;
            if let Some(ref mut plan) = *plan_guard
                && let Some(st) = plan.sub_tasks.iter_mut().find(|s| s.id == sub_task_id)
            {
                st.error = Some(error.to_string());
            }
        }

        self.emit(OrchestrationEvent::SubTaskFailed {
            sub_task_id: sub_task_id.to_string(),
            error: error.to_string(),
        })
        .await;

        self.monitor_and_maybe_replan().await
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
            .map(|p| {
                p.sub_tasks
                    .iter()
                    .map(|st| (st.id.clone(), st.status.to_string()))
                    .collect()
            })
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

        assert_eq!(plan.sub_tasks.len(), 4); // Refactor pattern
        assert!(matches!(plan.sub_tasks[1].role, AgentRole::Planner));
        assert!(matches!(plan.sub_tasks[2].role, AgentRole::Developer));
    }

    #[tokio::test]
    async fn test_generate_subgraph() {
        let executor = OrchestratorExecutor::new();
        executor
            .receive_mission("Fix login bug", OrchestrationStrategy::Ordered)
            .await
            .unwrap();

        let graph = executor.generate_subgraph().await.unwrap();
        assert_eq!(graph.nodes.len(), 3);
        // Should have 2 edges (analyze→implement, implement→review)
        assert!(graph.edges.len() >= 2);
    }

    #[tokio::test]
    async fn test_report_completed_and_terminal() {
        let executor = OrchestratorExecutor::new();
        executor
            .receive_mission("Quick fix", OrchestrationStrategy::Ordered)
            .await
            .unwrap();

        // Complete first two
        let result = executor
            .report_sub_task_completed("analyze", None)
            .await
            .unwrap();
        assert!(result.is_none()); // Not terminal yet

        let result = executor
            .report_sub_task_completed("implement", None)
            .await
            .unwrap();
        assert!(result.is_none()); // Still not terminal

        // Complete last
        let result = executor
            .report_sub_task_completed("review", None)
            .await
            .unwrap();
        assert!(result.is_none()); // Terminal but no failures

        let state = executor.current_state().await;
        assert!(matches!(state, OrchestratorState::Completed));
    }

    #[tokio::test]
    async fn test_replan_on_failure() {
        let executor = OrchestratorExecutor::new();
        let plan = executor
            .receive_mission("Quick fix", OrchestrationStrategy::Ordered)
            .await
            .unwrap();

        // Collect actual sub-task IDs from the plan
        let all_ids: Vec<String> = plan.sub_tasks.iter().map(|st| st.id.clone()).collect();

        // Fail ALL sub-tasks so the plan becomes terminal
        for id in &all_ids {
            executor
                .update_sub_task_status(id, SubTaskStatus::Failed)
                .await
                .unwrap();
        }

        // Trigger replan — plan is terminal with failures
        let result = executor.monitor_and_maybe_replan().await.unwrap();

        // Should trigger replan since all terminal with failures
        assert!(result.is_some());
        let new_plan = result.unwrap();
        // Failed analyze should be reset to pending
        let retried = new_plan
            .sub_tasks
            .iter()
            .find(|st| st.id == "analyze")
            .unwrap();
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
                executor
                    .update_sub_task_status(id, SubTaskStatus::Failed)
                    .await
                    .unwrap();
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
