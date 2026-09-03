// SPDX-License-Identifier: AGPL-3.0-only

//! axagent-mcp — Model Context Protocol 客户端 + 服务端
//!
//! 客户端: MCP 客户端连接池、发现工具、健康检查、OAuth 认证。
//! 服务端: McpAgentServer 把 harness::Agent + AgentSessionBroker 暴露为
//! MCP stdio server，供 Claude Desktop / Cline / VSCode MCP 扩展调用。

pub mod client_service_impl;
pub mod mcp_client;
pub mod mcp_health;
pub mod mcp_oauth;
pub mod server;

pub use server::McpAgentServer;
