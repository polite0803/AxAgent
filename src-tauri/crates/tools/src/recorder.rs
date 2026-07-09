// SPDX-License-Identifier: AGPL-3.0-only

//! ToolExecutionRecorder - 工具执行记录器（数据库审计）
//!
//! 记录每次工具执行的开始、成功、失败状态到 SQLite。
//!
//! Implements `axagent_harness::ToolExecutionAudit` to decouple
//! tools crate from the dao persistence layer.

use async_trait::async_trait;
use axagent_harness::ToolExecutionAudit;
use axagent_harness::repositories::tool_execution_repository;

#[derive(Clone)]
pub struct ToolExecutionRecorder;

impl ToolExecutionRecorder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolExecutionAudit for ToolExecutionRecorder {
    async fn record_start(
        &self,
        conversation_id: &str,
        message_id: Option<&str>,
        server_id: &str,
        tool_name: &str,
        input: Option<&str>,
    ) -> Result<String, String> {
        tool_execution_repository()
            .create_tool_execution(conversation_id, message_id, server_id, tool_name, input)
            .await
            .map(|e| e.id)
    }

    async fn record_success(
        &self,
        execution_id: &str,
        output: &str,
        _duration_ms: Option<i64>,
    ) -> Result<(), String> {
        tool_execution_repository()
            .update_tool_execution_status(execution_id, "success", Some(output), None)
            .await
            .map(|_| ())
    }

    async fn record_error(
        &self,
        execution_id: &str,
        error: &str,
        _duration_ms: Option<i64>,
    ) -> Result<(), String> {
        tool_execution_repository()
            .update_tool_execution_status(execution_id, "failed", None, Some(error))
            .await
            .map(|_| ())
    }
}
