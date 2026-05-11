//! Webhook event types — shared between axagent-rt-messaging and axagent-rt-webhook.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Events that can trigger webhooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebhookEvent {
    ToolComplete,
    ToolError,
    AgentError,
    AgentStart,
    AgentEnd,
    SessionStart,
    SessionEnd,
    MessageReceived,
    MessageSent,
}

impl WebhookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ToolComplete => "tool_complete",
            Self::ToolError => "tool_error",
            Self::AgentError => "agent_error",
            Self::AgentStart => "agent_start",
            Self::AgentEnd => "agent_end",
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
            Self::MessageReceived => "message_received",
            Self::MessageSent => "message_sent",
        }
    }

    pub fn from_event_str(s: &str) -> Option<Self> {
        match s {
            "tool_complete" => Some(Self::ToolComplete),
            "tool_error" => Some(Self::ToolError),
            "agent_error" => Some(Self::AgentError),
            "agent_start" => Some(Self::AgentStart),
            "agent_end" => Some(Self::AgentEnd),
            "session_start" => Some(Self::SessionStart),
            "session_end" => Some(Self::SessionEnd),
            "message_received" => Some(Self::MessageReceived),
            "message_sent" => Some(Self::MessageSent),
            _ => None,
        }
    }
}

/// Trait for webhook dispatch — allows rt-messaging to call webhook dispatch
/// without depending on the concrete rt-webhook crate.
#[async_trait::async_trait]
pub trait WebhookDispatch: Send + Sync {
    async fn dispatch(&self, event: WebhookEvent, data: HashMap<String, serde_json::Value>) -> DispatchResult;
}

/// Result of a dispatch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResult {
    pub success_count: usize,
    pub failure_count: usize,
    pub errors: Vec<String>,
}
