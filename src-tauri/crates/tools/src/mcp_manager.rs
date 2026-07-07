// SPDX-License-Identifier: AGPL-3.0-only

//! MCP（Model Context Protocol）工具管理器
//!
//! 从 `UnifiedToolRegistry` 中拆分，独立管理 MCP 服务器配置、工具注册和解析。

use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub server_id: String,
    pub server_name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args_json: Option<String>,
    pub env_json: Option<String>,
    pub endpoint: Option<String>,
    pub execute_timeout_secs: Option<i32>,
    pub connection_pool_size: Option<usize>,
    pub retry_attempts: Option<u32>,
    pub retry_delay_ms: Option<u64>,
}

impl McpServerConfig {
    pub fn get_timeout(&self) -> Duration {
        Duration::from_secs(self.execute_timeout_secs.unwrap_or(30) as u64)
    }
    pub fn get_pool_size(&self) -> usize {
        self.connection_pool_size.unwrap_or(4)
    }
    pub fn get_retry_attempts(&self) -> u32 {
        self.retry_attempts.unwrap_or(3)
    }
    pub fn get_retry_delay(&self) -> Duration {
        Duration::from_millis(self.retry_delay_ms.unwrap_or(100))
    }
}

#[derive(Debug, Clone)]
pub struct McpToolConfig {
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

/// MCP 工具管理器 —— 独立管理 MCP 服务器与工具注册
#[derive(Debug, Clone, Default)]
pub struct McpManager {
    pub mcp_tools: BTreeMap<String, McpToolConfig>,
    pub mcp_servers: BTreeMap<String, McpServerConfig>,
}

impl McpManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 解析 MCP 工具名 → (server_key, tool_config)
    pub fn resolve_tool(&self, name: &str) -> Option<(String, &McpToolConfig)> {
        self.mcp_servers
            .iter()
            .find_map(|(server_key, _)| {
                let full_name = format!("{}_{}", server_key, name);
                self.mcp_tools.get(&full_name).map(|cfg| (server_key.clone(), cfg))
            })
            .or_else(|| {
                // 直接匹配（含前缀的完整名称）
                self.mcp_tools.get(name).map(|cfg| (cfg.server_id.clone(), cfg))
            })
    }

    /// 注册 MCP 工具
    pub fn register_tool(&mut self, tool: McpToolConfig) {
        let key = format!("{}_{}", tool.server_id, tool.tool_name);
        self.mcp_tools.insert(key, tool);
    }

    /// 注册 MCP 服务器
    pub fn register_server(&mut self, server: McpServerConfig) {
        self.mcp_servers.insert(server.server_id.clone(), server);
    }
}
