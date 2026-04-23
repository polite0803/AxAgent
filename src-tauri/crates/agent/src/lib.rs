//! AxAgent Agent - ClawCode Runtime Integration

pub mod provider_adapter;
pub mod session_manager;
pub mod tool_registry;
pub mod local_tool_registry;
pub mod event_emitter;

pub use provider_adapter::{AxAgentApiClient, StreamEventCallback};
pub use session_manager::{AgentSession, SessionManager, TauriHookProgressReporter, ChannelPermissionPrompter};
pub use tool_registry::{ToolRegistry, ToolContext, ToolResult, ToolError, ToolExecutionRecorder, McpToolConfig, McpServerConfig};
pub use local_tool_registry::{LocalToolRegistry, LocalToolDef, LocalToolGroup};
pub use event_emitter::AgentPermissionPayload;
