//! MCP 工具包装器 - 将 MCP 工具暴露为 Tool trait

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct McpToolWrapper {
    pub server_id: String,
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
}

#[async_trait]
impl Tool for McpToolWrapper {
    fn name(&self) -> &str {
        &self.tool_name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success(
            "[MCP 工具执行结果将通过上层调度器处理]",
        ))
    }
}
