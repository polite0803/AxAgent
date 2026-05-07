use crate::coordinator::{
    AgentConfig, AgentError, AgentImpl, AgentInput, AgentStatus, CoordinatorOutput,
};
use crate::event_bus::{AgentEventBus, AgentEventType, UnifiedAgentEvent};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AgentImplAdapter {
    status: RwLock<AgentStatus>,
    config: RwLock<Option<AgentConfig>>,
    event_bus: Arc<AgentEventBus>,
}

impl AgentImplAdapter {
    pub fn new(event_bus: Option<Arc<AgentEventBus>>) -> Self {
        Self {
            status: RwLock::new(AgentStatus::Idle),
            config: RwLock::new(None),
            event_bus: event_bus.unwrap_or_else(|| Arc::new(AgentEventBus::new("adapter"))),
        }
    }

    pub async fn set_status(&self, status: AgentStatus) {
        let mut s = self.status.write().await;
        *s = status;
    }

    pub async fn get_status(&self) -> AgentStatus {
        self.status.read().await.clone()
    }

    pub fn event_bus(&self) -> Arc<AgentEventBus> {
        Arc::clone(&self.event_bus)
    }

    async fn emit(&self, event_type: AgentEventType, payload: serde_json::Value) {
        let event = UnifiedAgentEvent::new("AgentImplAdapter", event_type, payload);
        if let Err(e) = self.event_bus.emit(event) {
            tracing::warn!("Failed to emit event: {:?}", e);
        }
    }
}

impl Default for AgentImplAdapter {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait::async_trait]
impl AgentImpl for AgentImplAdapter {
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        *status = AgentStatus::Initializing;
        drop(status);

        let mut cfg = self.config.write().await;
        *cfg = Some(config);

        let mut status = self.status.write().await;
        *status = AgentStatus::Idle;

        Ok(())
    }

    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        let mut status = self.status.write().await;
        *status = AgentStatus::Running;
        drop(status);

        self.emit(
            AgentEventType::TurnStarted,
            serde_json::json!({
                "input_preview": input.content.chars().take(100).collect::<String>()
            }),
        )
        .await;

        let result = if let Some(tool_name) = input.context.as_ref().and_then(|c| c.get("tool_name")).and_then(|v| v.as_str()) {
            let tool_input = input.context.as_ref()
                .and_then(|c| c.get("tool_input"))
                .cloned()
                .unwrap_or(serde_json::json!({}));

            let (server_name, local_name) = parse_adapter_tool_name(tool_name);
            let args = if let Some(obj) = tool_input.as_object() {
                serde_json::to_value(obj.clone()).unwrap_or(tool_input)
            } else {
                serde_json::json!({ "input": tool_input })
            };

            match axagent_tools::builtin_handlers::dispatch(server_name, local_name, args).await {
                Ok(mcp_result) => Ok(CoordinatorOutput::success(mcp_result.content, 1)),
                Err(e) => Err(AgentError::ExecutionFailed(format!("Tool '{}' failed: {}", tool_name, e))),
            }
        } else {
            Ok(CoordinatorOutput::success(input.content, 1))
        };

        let mut status = self.status.write().await;
        match &result {
            Ok(output) => {
                *status = output.status.clone();
                self.emit(
                    AgentEventType::TurnCompleted,
                    serde_json::json!({
                        "status": "Completed"
                    }),
                )
                .await;
            },
            Err(e) => {
                *status = AgentStatus::Failed(e.to_string());
                self.emit(
                    AgentEventType::Error,
                    serde_json::json!({
                        "error": e.to_string()
                    }),
                )
                .await;
            },
        }

        result
    }

    async fn pause(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        if !matches!(*status, AgentStatus::Running) {
            return Err(AgentError::InvalidState(format!("Cannot pause from status {}", status)));
        }
        *status = AgentStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        if !matches!(*status, AgentStatus::Paused) {
            return Err(AgentError::InvalidState(format!("Cannot resume from status {}", status)));
        }
        *status = AgentStatus::Running;
        Ok(())
    }

    async fn cancel(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        *status = AgentStatus::Idle;
        Ok(())
    }

    fn status(&self) -> AgentStatus {
        match self.status.try_read() {
            Ok(guard) => guard.clone(),
            Err(_) => AgentStatus::Idle,
        }
    }

    fn agent_type(&self) -> &'static str {
        "AgentImplAdapter"
    }
}

impl std::fmt::Debug for AgentImplAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentImplAdapter")
            .field("event_bus", &self.event_bus.name())
            .finish()
    }
}

fn parse_adapter_tool_name(full_name: &str) -> (&str, &str) {
    if let Some(idx) = full_name.find('/') {
        let server = &full_name[..idx];
        let tool = &full_name[idx + 1..];
        (server, tool)
    } else {
        ("", full_name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRuntimeBindingMode {
    Owned,
    Shared,
}

pub struct AgentRuntimeAdapter<M: AgentRuntimeManager> {
    manager: M,
    status: RwLock<AgentStatus>,
    config: RwLock<Option<AgentConfig>>,
    event_bus: Arc<AgentEventBus>,
}

impl<M: AgentRuntimeManager> AgentRuntimeAdapter<M> {
    pub fn new(manager: M, event_bus: Option<Arc<AgentEventBus>>) -> Self {
        Self {
            manager,
            status: RwLock::new(AgentStatus::Idle),
            config: RwLock::new(None),
            event_bus: event_bus.unwrap_or_else(|| Arc::new(AgentEventBus::new("agent_runtime"))),
        }
    }

    pub fn event_bus(&self) -> Arc<AgentEventBus> {
        Arc::clone(&self.event_bus)
    }

    async fn emit(&self, event_type: AgentEventType, payload: serde_json::Value) {
        let event = UnifiedAgentEvent::new("AgentRuntimeAdapter", event_type, payload);
        if let Err(e) = self.event_bus.emit(event) {
            tracing::warn!("Failed to emit event: {:?}", e);
        }
    }
}

#[async_trait::async_trait]
pub trait AgentRuntimeManager: Send + Sync {
    async fn execute(&self, input: &str) -> Result<crate::AgentOutput, crate::AgentRuntimeError>;
    fn agent_type(&self) -> &'static str;
}

#[async_trait::async_trait]
impl<M: AgentRuntimeManager + Send + Sync> AgentImpl for AgentRuntimeAdapter<M> {
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        *status = AgentStatus::Initializing;
        drop(status);

        let mut cfg = self.config.write().await;
        *cfg = Some(config);

        let mut status = self.status.write().await;
        *status = AgentStatus::Idle;

        self.emit(
            AgentEventType::StateChanged,
            serde_json::json!({
                "from": "Initializing",
                "to": "Idle"
            }),
        )
        .await;

        Ok(())
    }

    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        {
            let mut status = self.status.write().await;
            if matches!(*status, AgentStatus::Running) {
                return Err(AgentError::AlreadyRunning);
            }
            *status = AgentStatus::Running;
        }

        self.emit(
            AgentEventType::TurnStarted,
            serde_json::json!({
                "input_preview": input.content.chars().take(100).collect::<String>()
            }),
        )
        .await;

        let result = self
            .manager
            .execute(&input.content)
            .await
            .map_err(|e| AgentError::ExecutionFailed(e.to_string()));

        let mut status = self.status.write().await;
        match &result {
            Ok(output) => {
                *status = AgentStatus::Completed;
                self.emit(
                    AgentEventType::TurnCompleted,
                    serde_json::json!({
                        "iterations": output.iterations,
                        "tool_call_count": output.tool_call_count
                    }),
                )
                .await;
            },
            Err(e) => {
                *status = AgentStatus::Failed(e.to_string());
                self.emit(
                    AgentEventType::Error,
                    serde_json::json!({
                        "error": e.to_string()
                    }),
                )
                .await;
            },
        }

        let final_status = self.status.read().await.clone();
        result.map(|output| CoordinatorOutput {
            content: output.response,
            status: final_status,
            iterations: output.iterations,
            metadata: serde_json::json!({
                "tool_call_count": output.tool_call_count
            }),
        })
    }

    async fn pause(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        if !matches!(*status, AgentStatus::Running) {
            return Err(AgentError::InvalidState(format!("Cannot pause from status {}", status)));
        }
        *status = AgentStatus::Paused;

        self.emit(
            AgentEventType::StateChanged,
            serde_json::json!({
                "from": "Running",
                "to": "Paused"
            }),
        )
        .await;

        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        if !matches!(*status, AgentStatus::Paused) {
            return Err(AgentError::InvalidState(format!("Cannot resume from status {}", status)));
        }
        *status = AgentStatus::Running;

        self.emit(
            AgentEventType::StateChanged,
            serde_json::json!({
                "from": "Paused",
                "to": "Running"
            }),
        )
        .await;

        Ok(())
    }

    async fn cancel(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        *status = AgentStatus::Idle;

        self.emit(
            AgentEventType::StateChanged,
            serde_json::json!({
                "to": "Idle"
            }),
        )
        .await;

        Ok(())
    }

    fn status(&self) -> AgentStatus {
        match self.status.try_read() {
            Ok(guard) => guard.clone(),
            Err(_) => AgentStatus::Idle,
        }
    }

    fn agent_type(&self) -> &'static str {
        self.manager.agent_type()
    }
}

impl<M: AgentRuntimeManager + Send + Sync> std::fmt::Debug for AgentRuntimeAdapter<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRuntimeAdapter")
            .field("event_bus", &self.event_bus.name())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinator::{AgentConfig, AgentError, AgentImpl, AgentInput, AgentStatus, CoordinatorOutput};
    use crate::event_bus::AgentEventType;

    #[test]
    fn test_parse_adapter_tool_name_with_slash() {
        let (server, tool) = parse_adapter_tool_name("myserver/mytool");
        assert_eq!(server, "myserver");
        assert_eq!(tool, "mytool");
    }

    #[test]
    fn test_parse_adapter_tool_name_without_slash() {
        let (server, tool) = parse_adapter_tool_name("mytool");
        assert_eq!(server, "");
        assert_eq!(tool, "mytool");
    }

    #[test]
    fn test_parse_adapter_tool_name_multiple_slashes() {
        let (server, tool) = parse_adapter_tool_name("server/path/tool");
        assert_eq!(server, "server");
        assert_eq!(tool, "path/tool");
    }

    #[test]
    fn test_parse_adapter_tool_name_empty() {
        let (server, tool) = parse_adapter_tool_name("");
        assert_eq!(server, "");
        assert_eq!(tool, "");
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_new() {
        let adapter = AgentImplAdapter::new(None);
        assert_eq!(adapter.get_status().await, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_default() {
        let adapter = AgentImplAdapter::default();
        assert_eq!(adapter.get_status().await, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_status_tracking() {
        let adapter = AgentImplAdapter::new(None);
        assert_eq!(adapter.get_status().await, AgentStatus::Idle);

        adapter.set_status(AgentStatus::Running).await;
        assert_eq!(adapter.get_status().await, AgentStatus::Running);

        adapter.set_status(AgentStatus::Completed).await;
        assert_eq!(adapter.get_status().await, AgentStatus::Completed);

        adapter.set_status(AgentStatus::Failed("error".to_string())).await;
        let status = adapter.get_status().await;
        assert!(matches!(status, AgentStatus::Failed(_)));
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_initialize() {
        let mut adapter = AgentImplAdapter::new(None);
        let config = AgentConfig::default();
        let result = adapter.initialize(config).await;
        assert!(result.is_ok());
        assert_eq!(adapter.get_status().await, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_execute_no_tool() {
        let mut adapter = AgentImplAdapter::new(None);
        adapter.initialize(AgentConfig::default()).await.unwrap();

        let input = AgentInput {
            content: "hello world".to_string(),
            context: None,
        };
        let result = adapter.execute(input).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.content, "hello world");
        assert_eq!(output.status, AgentStatus::Completed);
        assert_eq!(output.iterations, 1);
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_execute_with_context_no_tool_name() {
        let mut adapter = AgentImplAdapter::new(None);
        adapter.initialize(AgentConfig::default()).await.unwrap();

        let input = AgentInput {
            content: "test content".to_string(),
            context: Some(serde_json::json!({"other_key": "value"})),
        };
        let result = adapter.execute(input).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.content, "test content");
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_pause_not_running() {
        let mut adapter = AgentImplAdapter::new(None);
        adapter.initialize(AgentConfig::default()).await.unwrap();

        let result = adapter.pause().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgentError::InvalidState(_)));
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_resume_not_paused() {
        let mut adapter = AgentImplAdapter::new(None);
        adapter.initialize(AgentConfig::default()).await.unwrap();

        let result = adapter.resume().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgentError::InvalidState(_)));
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_cancel() {
        let mut adapter = AgentImplAdapter::new(None);
        adapter.initialize(AgentConfig::default()).await.unwrap();

        let result = adapter.cancel().await;
        assert!(result.is_ok());
        assert_eq!(adapter.get_status().await, AgentStatus::Idle);
    }

    #[test]
    fn test_agent_impl_adapter_agent_type() {
        let adapter = AgentImplAdapter::new(None);
        assert_eq!(adapter.agent_type(), "AgentImplAdapter");
    }

    #[test]
    fn test_agent_impl_adapter_status_sync() {
        let adapter = AgentImplAdapter::new(None);
        assert_eq!(adapter.status(), AgentStatus::Idle);
    }

    #[test]
    fn test_agent_impl_adapter_debug() {
        let adapter = AgentImplAdapter::new(None);
        let debug_str = format!("{:?}", adapter);
        assert!(debug_str.contains("AgentImplAdapter"));
    }

    #[test]
    fn test_agent_impl_adapter_event_bus() {
        let adapter = AgentImplAdapter::new(None);
        let bus = adapter.event_bus();
        assert_eq!(bus.name(), "adapter");
    }

    #[test]
    fn test_agent_impl_adapter_event_bus_custom() {
        let custom_bus = Arc::new(AgentEventBus::new("custom_adapter"));
        let adapter = AgentImplAdapter::new(Some(custom_bus));
        let bus = adapter.event_bus();
        assert_eq!(bus.name(), "custom_adapter");
    }

    #[test]
    fn test_agent_runtime_binding_mode_owned() {
        let mode = AgentRuntimeBindingMode::Owned;
        assert_eq!(mode, AgentRuntimeBindingMode::Owned);
    }

    #[test]
    fn test_agent_runtime_binding_mode_shared() {
        let mode = AgentRuntimeBindingMode::Shared;
        assert_eq!(mode, AgentRuntimeBindingMode::Shared);
    }

    #[test]
    fn test_agent_runtime_binding_mode_not_equal() {
        assert_ne!(AgentRuntimeBindingMode::Owned, AgentRuntimeBindingMode::Shared);
    }

    #[test]
    fn test_coordinator_output_success() {
        let output = CoordinatorOutput::success("done".to_string(), 3);
        assert_eq!(output.content, "done");
        assert_eq!(output.status, AgentStatus::Completed);
        assert_eq!(output.iterations, 3);
    }

    #[test]
    fn test_coordinator_output_failure() {
        let output = CoordinatorOutput::failure("failed".to_string(), 2);
        assert_eq!(output.content, "failed");
        assert!(matches!(output.status, AgentStatus::Failed(_)));
        assert_eq!(output.iterations, 2);
    }

    #[test]
    fn test_agent_status_display() {
        assert_eq!(AgentStatus::Idle.to_string(), "Idle");
        assert_eq!(AgentStatus::Initializing.to_string(), "Initializing");
        assert_eq!(AgentStatus::Running.to_string(), "Running");
        assert_eq!(AgentStatus::Paused.to_string(), "Paused");
        assert_eq!(AgentStatus::Completed.to_string(), "Completed");
        assert_eq!(AgentStatus::WaitingForConfirmation.to_string(), "WaitingForConfirmation");
        let failed = AgentStatus::Failed("err".to_string());
        assert!(failed.to_string().contains("err"));
    }

    #[test]
    fn test_agent_input_creation() {
        let input = AgentInput {
            content: "hello".to_string(),
            context: None,
        };
        assert_eq!(input.content, "hello");
        assert!(input.context.is_none());
    }

    #[test]
    fn test_agent_input_with_context() {
        let input = AgentInput {
            content: "hello".to_string(),
            context: Some(serde_json::json!({"key": "value"})),
        };
        assert_eq!(input.content, "hello");
        assert!(input.context.is_some());
        let ctx = input.context.unwrap();
        assert_eq!(ctx["key"], "value");
    }

    #[test]
    fn test_agent_event_type_variants() {
        let _started = AgentEventType::TurnStarted;
        let _completed = AgentEventType::TurnCompleted;
        let _tool_use = AgentEventType::ToolUse;
        let _tool_result = AgentEventType::ToolResult;
        let _tool_error = AgentEventType::ToolError;
        let _state_changed = AgentEventType::StateChanged;
        let _error = AgentEventType::Error;
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_pause_from_running() {
        let mut adapter = AgentImplAdapter::new(None);
        adapter.initialize(AgentConfig::default()).await.unwrap();
        adapter.set_status(AgentStatus::Running).await;

        let result = adapter.pause().await;
        assert!(result.is_ok());
        assert_eq!(adapter.get_status().await, AgentStatus::Paused);
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_resume_from_paused() {
        let mut adapter = AgentImplAdapter::new(None);
        adapter.initialize(AgentConfig::default()).await.unwrap();
        adapter.set_status(AgentStatus::Paused).await;

        let result = adapter.resume().await;
        assert!(result.is_ok());
        assert_eq!(adapter.get_status().await, AgentStatus::Running);
    }

    #[tokio::test]
    async fn test_agent_impl_adapter_cancel_from_running() {
        let mut adapter = AgentImplAdapter::new(None);
        adapter.initialize(AgentConfig::default()).await.unwrap();
        adapter.set_status(AgentStatus::Running).await;

        let result = adapter.cancel().await;
        assert!(result.is_ok());
        assert_eq!(adapter.get_status().await, AgentStatus::Idle);
    }
}
