//! Event payload types for AxAgent Agent Tauri events.
//!
//! The EventEmitter struct has been removed — all event emission is done
//! directly via `app.emit()` in the command handlers. This module only
//! defines the shared payload types used by both `commands/agent.rs` and
//! `session_manager.rs`.

use serde::{Deserialize, Serialize};

/// Agent permission request payload (used by ChannelPermissionPrompter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPermissionPayload {
    pub conversation_id: String,
    pub assistant_message_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub risk_level: String,
    pub request_id: String,
    /// The tool_use_id from the LLM response (for frontend display correlation)
    #[serde(rename = "toolUseId")]
    pub tool_use_id: Option<String>,
}
