//! MCP 协议增强模块
//!
//! OAuth 认证 + 传输适配器 + MCP → Tool 包装 + 官方注册表。

pub mod adapters;
pub mod mcp_tool_wrapper;
pub mod oauth;
pub mod registry;

use serde::{Deserialize, Serialize};

/// MCP 传输类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpTransport {
    Stdio,
    Sse,
    StreamableHttp,
}

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub transport: McpTransport,
    /// Stdio: 命令
    pub command: Option<String>,
    /// Stdio: 参数
    pub args: Option<Vec<String>>,
    /// Stdio: 环境变量
    pub env: Option<std::collections::HashMap<String, String>>,
    /// SSE/HTTP: 端点 URL
    pub endpoint: Option<String>,
    /// 是否需要 OAuth
    pub oauth_required: bool,
    /// OAuth 配置
    pub oauth_config: Option<oauth::OAuthConfig>,
}

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
