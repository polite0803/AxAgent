//! MCP 协议增强模块
//!
//! OAuth 认证 + MCP → Tool 包装 + 官方注册表。
//! MCP 配置类型统一使用 `axagent_runtime::config` 中的权威定义。

pub mod mcp_tool_wrapper;
pub mod oauth;
pub mod registry;

use serde::{Deserialize, Serialize};

// 统一使用 runtime::config 的权威 MCP 类型，消除重复定义
pub use axagent_runtime::{McpServerConfig, McpTransport};

/// MCP 工具描述符（从 list_tools 返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
}

/// 官方注册表条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub endpoint: Option<String>,
    pub transport: McpTransport,
}
