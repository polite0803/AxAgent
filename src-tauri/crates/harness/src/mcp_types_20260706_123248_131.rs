// SPDX-License-Identifier: AGPL-3.0-only

//! MCP-related data types shared across the harness boundary.
//!
//! These types are used by both the MCP client (mcp crate), the DAO layer
//! (dao crate), and the gateway without any of them depending on each other.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool discovered from an MCP server via tools/list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

/// Minimal server record used by the gateway to query and proxy MCP servers.
///
/// This DTO decouples the gateway from `axagent_entities::mcp_servers::Model`
/// (a SeaORM ActiveModel) while still carrying the fields needed for MCP proxy
/// handlers.
#[derive(Debug, Clone)]
pub struct McpServerRecord {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args_json: Option<String>,
    pub env_json: Option<String>,
    pub endpoint: Option<String>,
}

/// Result of an MCP tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallResult {
    pub content: Vec<McpToolContent>,
    pub is_error: Option<bool>,
}

/// A single content block in an MCP tool call result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Trait for the MCP gateway client — abstracts the MCP client operations
/// needed by the gateway (discovering tools and calling tools on MCP servers).
///
/// This decouples `axagent-gateway` from `axagent-mcp::mcp_client`.
pub trait McpGatewayClient: Send + Sync {
    /// Discover tools available on an MCP server.
    fn discover_tools(
        &self,
        transport: &str,
        command: Option<&str>,
        args: Option<&[String]>,
        env: Option<&HashMap<String, String>>,
        endpoint: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<DiscoveredTool>, String>> + Send;

    /// Call a tool on an MCP server.
    fn call_tool(
        &self,
        transport: &str,
        command: Option<&str>,
        args: Option<&[String]>,
        env: Option<&HashMap<String, String>>,
        endpoint: Option<&str>,
        tool_name: &str,
        arguments: Value,
    ) -> impl std::future::Future<Output = Result<McpToolCallResult, String>> + Send;
}

/// Trait for looking up MCP server records — used by the gateway to query
/// server metadata without depending on `axagent-entities` or SeaORM.
pub trait McpServerLookup: Send + Sync {
    /// Get all enabled MCP servers.
    fn get_enabled_servers(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<McpServerRecord>, String>> + Send;

    /// Get a single MCP server by its ID.
    fn get_server_by_id(
        &self,
        server_id: &str,
    ) -> impl std::future::Future<Output = Result<Option<McpServerRecord>, String>> + Send;
}
