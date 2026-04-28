use crate::work_engine::engine::WorkEngine;
use crate::work_engine::execution_state::ExecutionState;
use crate::workflow_engine::{
    WorkflowEngine, WorkflowRunner, WorkflowStep, StepExecutor,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeExecutionResult {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: String,
    pub step_results: HashMap<String, String>,
    pub total_time_ms: u64,
}

pub struct WorkflowBridge {
    work_engine: Arc<WorkEngine>,
    workflow_engine: Arc<WorkflowEngine>,
}

impl WorkflowBridge {
    pub fn new(
        work_engine: Arc<WorkEngine>,
        workflow_engine: Arc<WorkflowEngine>,
    ) -> Self {
        Self {
            work_engine,
            workflow_engine,
        }
    }

    pub async fn execute_skill_as_workflow(
        &self,
        skill_id: &str,
        skill_name: &str,
        steps: Vec<WorkflowStep>,
        executor: StepExecutor,
    ) -> Result<BridgeExecutionResult, String> {
        let workflow = self
            .workflow_engine
            .create_workflow(&format!("skill_{}", skill_name), steps)
            .map_err(|e| format!("Failed to create workflow: {}", e))?;

        let execution_id = self
            .work_engine
            .start_workflow(&workflow.id, serde_json::json!({ "skill_id": skill_id }))
            .await
            .map_err(|e| format!("Failed to start workflow execution: {}", e))?;

        let runner = WorkflowRunner::new(Arc::clone(&self.workflow_engine), executor);
        let result = runner
            .run(&workflow.id)
            .await
            .map_err(|e| format!("Workflow execution failed: {}", e))?;

        let total_time_ms = result
            .completed_at
            .map(|end| end.saturating_sub(result.created_at) * 1000)
            .unwrap_or(0);

        let status_str = match result.status {
            crate::workflow_engine::WorkflowStatus::Completed => "completed",
            crate::workflow_engine::WorkflowStatus::PartiallyCompleted => "partially_completed",
            crate::workflow_engine::WorkflowStatus::Failed => "failed",
            crate::workflow_engine::WorkflowStatus::Cancelled => "cancelled",
            _ => "running",
        };

        if status_str == "completed" || status_str == "partially_completed" {
            self.work_engine
                .complete_execution(
                    &execution_id,
                    &serde_json::to_value(&result.results).unwrap_or(serde_json::json!(null)),
                    total_time_ms,
                )
                .await
                .map_err(|e| format!("Failed to complete execution: {}", e))?;
        }

        Ok(BridgeExecutionResult {
            execution_id,
            workflow_id: workflow.id,
            status: status_str.to_string(),
            step_results: result.results,
            total_time_ms,
        })
    }

    pub async fn get_execution_status(
        &self,
        execution_id: &str,
    ) -> Result<ExecutionState, String> {
        self.work_engine
            .get_status(execution_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn pause_execution(&self, execution_id: &str) -> Result<(), String> {
        self.work_engine
            .pause(execution_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn resume_execution(&self, execution_id: &str) -> Result<(), String> {
        self.work_engine
            .resume(execution_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn cancel_execution(&self, execution_id: &str) -> Result<(), String> {
        self.work_engine
            .cancel(execution_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_executions(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<axagent_core::entity::workflow_executions::Model>, String> {
        self.work_engine
            .list_executions(workflow_id)
            .await
            .map_err(|e| e.to_string())
    }
}
