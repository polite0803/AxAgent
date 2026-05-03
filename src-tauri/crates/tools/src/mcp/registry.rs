//! MCP 官方服务器注册表
//! 内置常用 MCP 服务器的安装信息。

use crate::mcp::{McpTransport, RegistryEntry};

/// 获取官方注册表的所有条目
pub fn official_registry() -> Vec<RegistryEntry> {
    vec![
        RegistryEntry {
            name: "filesystem".into(),
            description: "安全文件系统操作".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/mcp-server-filesystem".into()],
            env: None,
            endpoint: None,
            transport: McpTransport::Stdio,
        },
        RegistryEntry {
            name: "github".into(),
            description: "GitHub 仓库和 Issue 管理".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/mcp-server-github".into()],
            env: None,
            endpoint: None,
            transport: McpTransport::Stdio,
        },
        RegistryEntry {
            name: "postgres".into(),
            description: "PostgreSQL 数据库访问".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/mcp-server-postgres".into()],
            env: None,
            endpoint: None,
            transport: McpTransport::Stdio,
        },
        RegistryEntry {
            name: "brave-search".into(),
            description: "Brave 搜索引擎".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/mcp-server-brave-search".into()],
            env: None,
            endpoint: None,
            transport: McpTransport::Stdio,
        },
    ]
}
