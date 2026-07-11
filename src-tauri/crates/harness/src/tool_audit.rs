// SPDX-License-Identifier: AGPL-3.0-only

//! Tool execution audit trail — contract between tools and persistence layer.
//!
//! Tools crate uses this trait instead of directly invoking dao repo functions,
//! enabling dependency inversion and testability.

use async_trait::async_trait;

/// Records start/success/error events for each tool execution.
#[async_trait]
pub trait ToolExecutionAudit: Send + Sync + Clone + 'static {
    async fn record_start(
        &self,
        conversation_id: &str,
        message_id: Option<&str>,
        server_id: &str,
        tool_name: &str,
        input: Option<&str>,
    ) -> Result<String, String>;

    async fn record_success(
        &self,
        execution_id: &str,
        output: &str,
        duration_ms: Option<i64>,
    ) -> Result<(), String>;

    async fn record_error(
        &self,
        execution_id: &str,
        error: &str,
        duration_ms: Option<i64>,
    ) -> Result<(), String>;
}
