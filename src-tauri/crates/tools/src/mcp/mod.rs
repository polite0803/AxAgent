// SPDX-License-Identifier: AGPL-3.0-only

//! MCP 协议增强模块
//!
//! OAuth 认证 + MCP → Tool 包装 + 官方注册表。
//! MCP 配置类型统一使用 `axagent_runtime_core::config` 中的权威定义。

pub mod mcp_tool_wrapper;
pub mod oauth;
pub mod registry;

use serde::{Deserialize, Serialize};

// 统一使用 runtime::config 的权威 MCP 类型，消除重复定义
pub use axagent_runtime_core::{McpServerConfig, McpTransport};

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

/// 拆分复合工具名 `"server/tool"` → `("server", "tool")`。
/// 无 `/` 时返回 `("", full_name)`。
pub fn parse_tool_name(full_name: &str) -> (&str, &str) {
    if let Some(idx) = full_name.find('/') {
        (&full_name[..idx], &full_name[idx + 1..])
    } else {
        ("", full_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_slash() {
        assert_eq!(parse_tool_name("myserver/mytool"), ("myserver", "mytool"));
    }

    #[test]
    fn without_slash() {
        assert_eq!(parse_tool_name("mytool"), ("", "mytool"));
    }

    #[test]
    fn multiple_slashes() {
        assert_eq!(parse_tool_name("server/path/tool"), ("server", "path/tool"));
    }

    #[test]
    fn empty() {
        assert_eq!(parse_tool_name(""), ("", ""));
    }
}
