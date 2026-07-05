// SPDX-License-Identifier: AGPL-3.0-only

//! MCP-related data types shared across the harness boundary.
//!
//! These types are used by both the MCP client (mcp crate) and the DAO layer
//! (dao crate) without either depending on the other.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool discovered from an MCP server via tools/list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}
