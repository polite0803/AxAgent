//! ToolExecutionRecorder - 工具执行记录器（数据库审计）
//!
//! 记录每次工具执行的开始、成功、失败状态到 SQLite。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use axagent_core::repo::tool_execution;

#[derive(Clone)]
pub struct ToolExecutionRecorder {
    db: Arc<DatabaseConnection>,
}

impl ToolExecutionRecorder {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn record_start(
        &self,
        conversation_id: &str,
        message_id: Option<&str>,
        server_id: &str,
        tool_name: &str,
        input: Option<&str>,
    ) -> Result<String, String> {
        tool_execution::create_tool_execution(
            &self.db,
            conversation_id,
            message_id,
            server_id,
            tool_name,
            input,
            None,
        )
        .await
        .map(|e| e.id)
        .map_err(|e| e.to_string())
    }

    pub async fn record_success(
        &self,
        execution_id: &str,
        output: &str,
        _duration_ms: Option<i64>,
    ) -> Result<(), String> {
        tool_execution::update_tool_execution_status(
            &self.db,
            execution_id,
            "success",
            Some(output),
            None,
        )
        .await
        .map_err(|e| e.to_string())
    }

    pub async fn record_error(
        &self,
        execution_id: &str,
        error: &str,
        _duration_ms: Option<i64>,
    ) -> Result<(), String> {
        tool_execution::update_tool_execution_status(
            &self.db,
            execution_id,
            "failed",
            None,
            Some(error),
        )
        .await
        .map_err(|e| e.to_string())
    }
}
