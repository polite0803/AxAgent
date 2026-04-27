use std::sync::Arc;
use tokio::sync::Mutex;

use sea_orm::DatabaseConnection;

use super::execution_state::{ExecutionState, ExecutionStatus, NodeExecutionRecord};

/// Work engine that drives workflow execution
pub struct WorkEngine {
    db: Arc<DatabaseConnection>,
    executions: Arc<Mutex<std::collections::HashMap<String, ExecutionState>>>,
}

impl WorkEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            executions: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Start a workflow execution
    pub async fn start_workflow(
        &self,
        workflow_id: &str,
        input: serde_json::Value,
    ) -> Result<String, WorkEngineError> {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let state =
            ExecutionState::new(execution_id.clone(), workflow_id.to_string(), input.clone());

        // Persist to database
        let input_params = serde_json::to_string(&input).ok();
        axagent_core::repo::workflow_execution::create_workflow_execution(
            &self.db,
            &execution_id,
            workflow_id,
            input_params.as_deref(),
        )
        .await
        .map_err(|e| WorkEngineError::Db(e.to_string()))?;

        // Store in memory
        self.executions
            .lock()
            .await
            .insert(execution_id.clone(), state);

        Ok(execution_id)
    }

    /// Pause a workflow execution
    pub async fn pause(&self, execution_id: &str) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.status = ExecutionStatus::Paused;
            state.updated_at = chrono::Utc::now().timestamp_millis();
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }

    /// Resume a paused workflow execution
    pub async fn resume(&self, execution_id: &str) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            if state.status == ExecutionStatus::Paused {
                state.status = ExecutionStatus::Running;
                state.updated_at = chrono::Utc::now().timestamp_millis();
            }
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }

    /// Cancel a workflow execution
    pub async fn cancel(&self, execution_id: &str) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.status = ExecutionStatus::Cancelled;
            state.updated_at = chrono::Utc::now().timestamp_millis();

            // Update database
            drop(executions);
            axagent_core::repo::workflow_execution::update_workflow_execution_status(
                &self.db,
                execution_id,
                "cancelled",
                None,
                None,
                None,
            )
            .await
            .map_err(|e| WorkEngineError::Db(e.to_string()))?;

            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }

    /// Get execution status
    pub async fn get_status(&self, execution_id: &str) -> Result<ExecutionState, WorkEngineError> {
        let executions = self.executions.lock().await;
        executions
            .get(execution_id)
            .cloned()
            .ok_or_else(|| WorkEngineError::NotFound(execution_id.to_string()))
    }

    /// List execution history for a workflow
    pub async fn list_executions(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<axagent_core::entity::workflow_executions::Model>, WorkEngineError> {
        axagent_core::repo::workflow_execution::list_workflow_executions(&self.db, workflow_id)
            .await
            .map_err(|e| WorkEngineError::Db(e.to_string()))
    }

    /// Record a node execution result
    pub async fn record_node_execution(
        &self,
        execution_id: &str,
        record: NodeExecutionRecord,
    ) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.add_node_record(record);
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }

    /// Mark execution as completed
    pub async fn complete_execution(
        &self,
        execution_id: &str,
        output: &serde_json::Value,
        total_time_ms: u64,
    ) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.status = ExecutionStatus::Completed;
            state.total_time_ms = total_time_ms;
            state.updated_at = chrono::Utc::now().timestamp_millis();

            let node_executions = serde_json::to_string(&state.node_records).ok();
            let output_result = serde_json::to_string(output).ok();

            drop(executions);
            axagent_core::repo::workflow_execution::update_workflow_execution_status(
                &self.db,
                execution_id,
                "completed",
                output_result.as_deref(),
                node_executions.as_deref(),
                Some(total_time_ms as i32),
            )
            .await
            .map_err(|e| WorkEngineError::Db(e.to_string()))?;

            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }
}

/// Work engine error
#[derive(Debug)]
pub enum WorkEngineError {
    NotFound(String),
    Db(String),
    Execution(String),
}

impl std::fmt::Display for WorkEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkEngineError::NotFound(id) => write!(f, "Execution not found: {}", id),
            WorkEngineError::Db(e) => write!(f, "Database error: {}", e),
            WorkEngineError::Execution(e) => write!(f, "Execution error: {}", e),
        }
    }
}

impl std::error::Error for WorkEngineError {}
