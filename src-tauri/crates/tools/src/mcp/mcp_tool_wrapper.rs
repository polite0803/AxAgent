// SPDX-License-Identifier: AGPL-3.0-only

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
        let harness_server = axagent_harness::McpServerConfig {
            id: self.name.clone(),
            name: self.tool_name.clone(),
            transport: match &self.transport {
                McpTransportConfig::Stdio { .. } => "stdio".into(),
                McpTransportConfig::Http { .. } => "http".into(),
                McpTransportConfig::Sse { .. } => "sse".into(),
            },
            command: match &self.transport {
                McpTransportConfig::Stdio { command, .. } => Some(command.clone()),
                _ => None,
            },
            args_json: match &self.transport {
                McpTransportConfig::Stdio { args, .. } => {
                    Some(serde_json::to_string(args).unwrap_or_default())
                },
                _ => None,
            },
            env_json: match &self.transport {
                McpTransportConfig::Stdio { env, .. } => {
                    env.as_ref().map(|e| serde_json::to_string(e).unwrap_or_default())
                },
                _ => None,
            },
            endpoint: match &self.transport {
                McpTransportConfig::Http { endpoint } | McpTransportConfig::Sse { endpoint } => {
                    Some(endpoint.clone())
                },
                _ => None,
            },
        };
        let result = crate::mcp_client_service()
            .call_tool(&harness_server, &self.tool_name, input)
            .await
            .map_err(|e| ToolError::execution_failed_for(&self.tool_name, e))?;

        // 将 harness Value content 转回字符串
        let content_str = match &result.content {
            serde_json::Value::String(s) => s.clone(),
            other => serde_json::to_string(other).unwrap_or_default(),
        };

        if result.is_error {
            Err(ToolError::execution_failed_for(&self.tool_name, content_str))
        } else {
            Ok(ToolResult::success(content_str))
        }
    }
}
