// SPDX-License-Identifier: AGPL-3.0-only

//! MCP 服务契约 — 让 tools/gateway 不依赖 mcp crate。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub command: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredMcpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallResult {
    pub success: bool,
    pub content: serde_json::Value,
}

#[async_trait]
pub trait McpServerStore: Send + Sync {
    async fn list_enabled(&self) -> Result<Vec<McpServerConfig>, String>;
    async fn get_by_id(&self, id: &str) -> Result<Option<McpServerConfig>, String>;
}

#[async_trait]
pub trait McpClientService: Send + Sync {
    async fn discover_tools(
        &self,
        server: &McpServerConfig,
    ) -> Result<Vec<DiscoveredMcpTool>, String>;
    async fn call_tool(
        &self,
        server: &McpServerConfig,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<McpToolCallResult, String>;
}

#[derive(Debug, Default)]
pub struct NoopMcpServerStore;

#[async_trait]
impl McpServerStore for NoopMcpServerStore {
    async fn list_enabled(&self) -> Result<Vec<McpServerConfig>, String> {
        Ok(Vec::new())
    }
    async fn get_by_id(&self, _id: &str) -> Result<Option<McpServerConfig>, String> {
        Ok(None)
    }
}

#[derive(Debug, Default)]
pub struct NoopMcpClientService;

#[async_trait]
impl McpClientService for NoopMcpClientService {
    async fn discover_tools(
        &self,
        _server: &McpServerConfig,
    ) -> Result<Vec<DiscoveredMcpTool>, String> {
        Ok(Vec::new())
    }
    async fn call_tool(
        &self,
        _server: &McpServerConfig,
        _tool_name: &str,
        _args: serde_json::Value,
    ) -> Result<McpToolCallResult, String> {
        Ok(McpToolCallResult {
            success: false,
            content: serde_json::Value::Null,
        })
    }
}
