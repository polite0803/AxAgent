// SPDX-License-Identifier: AGPL-3.0-only

//! MCP 服务契约 — 让 tools/gateway 不依赖 mcp crate。

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::mcp_types::{McpPrompt, McpPromptResult, McpResource, McpResourceContent};

/// MCP server 的最小连接配置 —— 让 gateway 不依赖 `axagent_entities::mcp_servers::Model`
/// 与 SeaORM。字段对齐 `mcp_client::discover_tools_unified` / `call_tool_unified`
/// 所需的 transport/command/args/env/endpoint。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    /// 传输类型：stdio / http / sse。
    pub transport: String,
    /// stdio 传输的可执行命令。
    pub command: Option<String>,
    /// stdio 传输的命令行参数。
    pub args: Option<Vec<String>>,
    /// stdio 传输注入的环境变量。
    pub env: Option<HashMap<String, String>>,
    /// http / sse 传输的服务端点 URL。
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredMcpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    // H1: resources / prompts support
    async fn list_resources(&self, server: &McpServerConfig) -> Result<Vec<McpResource>, String>;

    async fn read_resource(
        &self,
        server: &McpServerConfig,
        uri: &str,
    ) -> Result<Vec<McpResourceContent>, String>;

    async fn list_prompts(&self, server: &McpServerConfig) -> Result<Vec<McpPrompt>, String>;

    async fn get_prompt(
        &self,
        server: &McpServerConfig,
        name: &str,
        args: serde_json::Value,
    ) -> Result<McpPromptResult, String>;
}
