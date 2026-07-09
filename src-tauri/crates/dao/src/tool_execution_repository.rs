// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_harness::repositories::ToolExecutionRepository;
use axagent_harness::types::ToolExecution;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub struct DaoToolExecutionRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoToolExecutionRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ToolExecutionRepository for DaoToolExecutionRepository {
    async fn create_tool_execution(
        &self,
        conversation_id: &str,
        message_id: Option<&str>,
        server_id: &str,
        tool_name: &str,
        input: Option<&str>,
    ) -> Result<ToolExecution, String> {
        crate::repo::tool_execution::create_tool_execution(
            &self.db,
            conversation_id,
            message_id,
            server_id,
            tool_name,
            input,
            None,
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn update_tool_execution_status(
        &self,
        execution_id: &str,
        status: &str,
        output: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), String> {
        crate::repo::tool_execution::update_tool_execution_status(
            &self.db,
            execution_id,
            status,
            output,
            error,
        )
        .await
        .map_err(|e| e.to_string())
        .map(|_| ())
    }
}
