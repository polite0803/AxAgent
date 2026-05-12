//! MCP 工具包装器 - 将 MCP 工具暴露为 Tool trait
//!
//! 持有 MCP 服务器的传输配置，通过 `core::mcp_client` 实际执行工具调用。

use std::collections::HashMap;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// MCP 传输方式
#[derive(Debug, Clone)]
pub enum McpTransportConfig {
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    Http {
        endpoint: String,
    },
    Sse {
        endpoint: String,
    },
}

/// MCP 工具包装器 - 将远程 MCP 工具暴露为本地 Tool trait 实现
pub struct McpToolWrapper {
    pub server_id: String,
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
    pub transport: McpTransportConfig,
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

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let result = match &self.transport {
            McpTransportConfig::Stdio { command, args, env } => {
                axagent_core::mcp_client::call_tool_stdio_pooled(
                    command,
                    args,
                    env,
                    &self.tool_name,
                    input,
                )
                .await
                .map_err(|e| {
                    ToolError::execution_failed_for(
                        &self.tool_name,
                        format!("MCP stdio 调用失败: {e}"),
                    )
                })?
            },
            McpTransportConfig::Http { endpoint } => {
                axagent_core::mcp_client::call_tool_http(endpoint, &self.tool_name, input)
                    .await
                    .map_err(|e| {
                        ToolError::execution_failed_for(
                            &self.tool_name,
                            format!("MCP HTTP 调用失败: {e}"),
                        )
                    })?
            },
            McpTransportConfig::Sse { endpoint } => {
                axagent_core::mcp_client::call_tool_sse(endpoint, &self.tool_name, input)
                    .await
                    .map_err(|e| {
                        ToolError::execution_failed_for(
                            &self.tool_name,
                            format!("MCP SSE 调用失败: {e}"),
                        )
                    })?
            },
        };

        if result.is_error {
            Err(ToolError::execution_failed_for(&self.tool_name, result.content))
        } else {
            Ok(ToolResult::success(result.content))
        }
    }
}
