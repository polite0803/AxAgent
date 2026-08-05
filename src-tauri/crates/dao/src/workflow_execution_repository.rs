// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::*;

use axagent_harness::repo_dtos::WorkflowExecutionData;
use axagent_harness::repositories::WorkflowExecutionRepository;

pub struct DaoWorkflowExecutionRepository {
    pub db: Arc<DatabaseConnection>,
}

#[async_trait]
impl WorkflowExecutionRepository for DaoWorkflowExecutionRepository {
    async fn create_workflow_execution(
        &self,
        id: &str,
        workflow_id: &str,
        input_params: Option<&str>,
    ) -> Result<(), String> {
        crate::repo::workflow_execution::create_workflow_execution(
            &self.db,
            id,
            workflow_id,
            input_params,
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn update_workflow_execution_status(
        &self,
        id: &str,
        status: &str,
        output_result: Option<&str>,
        node_executions: Option<&str>,
        total_time_ms: Option<i32>,
    ) -> Result<bool, String> {
        crate::repo::workflow_execution::update_workflow_execution_status(
            &self.db,
            id,
            status,
            output_result,
            node_executions,
            total_time_ms,
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn list_workflow_executions(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<WorkflowExecutionData>, String> {
        let models =
            crate::repo::workflow_execution::list_workflow_executions(&self.db, workflow_id)
                .await
                .map_err(|e| e.to_string())?;
        Ok(models
            .into_iter()
            .map(|m| WorkflowExecutionData {
                id: m.id,
                workflow_id: m.workflow_id,
                status: m.status,
                input_params: m.input_params,
                output_result: m.output_result,
                node_executions: m.node_executions,
                execution_state_json: m.execution_state_json,
                paused_at: m.paused_at,
                total_time_ms: m.total_time_ms,
                created_at: m.created_at,
                updated_at: m.updated_at,
            })
            .collect())
    }

    async fn save_execution_state(
        &self,
        id: &str,
        status: &str,
        execution_state_json: &str,
    ) -> Result<bool, String> {
        crate::repo::workflow_execution::save_execution_state(
            &self.db,
            id,
            status,
            execution_state_json,
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn clear_execution_state(&self, id: &str, status: &str) -> Result<bool, String> {
        crate::repo::workflow_execution::clear_execution_state(&self.db, id, status)
            .await
            .map_err(|e| e.to_string())
    }

    async fn list_paused_executions(&self) -> Result<Vec<WorkflowExecutionData>, String> {
        let models = crate::repo::workflow_execution::list_paused_executions(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(models
            .into_iter()
            .map(|m| WorkflowExecutionData {
                id: m.id,
                workflow_id: m.workflow_id,
                status: m.status,
                input_params: m.input_params,
                output_result: m.output_result,
                node_executions: m.node_executions,
                execution_state_json: m.execution_state_json,
                paused_at: m.paused_at,
                total_time_ms: m.total_time_ms,
                created_at: m.created_at,
                updated_at: m.updated_at,
            })
            .collect())
    }
}
