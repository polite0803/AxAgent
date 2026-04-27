use crate::coordinator::{AgentConfig, AgentError, AgentImpl, AgentInput, AgentStatus, CoordinatorOutput};
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

        self.emit(AgentEventType::TurnStarted, serde_json::json!({
            "input_preview": input.content.chars().take(100).collect::<String>()
        })).await;

        let result = Ok(CoordinatorOutput::success(
            format!("Processed: {}", input.content),
            1,
        ));

        let mut status = self.status.write().await;
        *status = AgentStatus::Completed;

        self.emit(AgentEventType::TurnCompleted, serde_json::json!({
            "status": "Completed"
        })).await;

        result
    }

    async fn pause(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        if !matches!(*status, AgentStatus::Running) {
            return Err(AgentError::InvalidState(format!(
                "Cannot pause from status {}",
                status
            )));
        }
        *status = AgentStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        if !matches!(*status, AgentStatus::Paused) {
            return Err(AgentError::InvalidState(format!(
                "Cannot resume from status {}",
                status
            )));
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
        AgentStatus::Idle
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
    pub fn new(
        manager: M,
        event_bus: Option<Arc<AgentEventBus>>,
    ) -> Self {
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

        self.emit(AgentEventType::StateChanged, serde_json::json!({
            "from": "Initializing",
            "to": "Idle"
        })).await;

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

        self.emit(AgentEventType::TurnStarted, serde_json::json!({
            "input_preview": input.content.chars().take(100).collect::<String>()
        })).await;

        let result = self.manager.execute(&input.content).await.map_err(|e| {
            AgentError::ExecutionFailed(e.to_string())
        });

        let mut status = self.status.write().await;
        match &result {
            Ok(output) => {
                *status = AgentStatus::Completed;
                self.emit(AgentEventType::TurnCompleted, serde_json::json!({
                    "iterations": output.iterations,
                    "tool_call_count": output.tool_call_count
                })).await;
            }
            Err(e) => {
                *status = AgentStatus::Failed(e.to_string());
                self.emit(AgentEventType::Error, serde_json::json!({
                    "error": e.to_string()
                })).await;
            }
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
            return Err(AgentError::InvalidState(format!(
                "Cannot pause from status {}",
                status
            )));
        }
        *status = AgentStatus::Paused;

        self.emit(AgentEventType::StateChanged, serde_json::json!({
            "from": "Running",
            "to": "Paused"
        })).await;

        Ok(())
    }

    async fn resume(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        if !matches!(*status, AgentStatus::Paused) {
            return Err(AgentError::InvalidState(format!(
                "Cannot resume from status {}",
                status
            )));
        }
        *status = AgentStatus::Running;

        self.emit(AgentEventType::StateChanged, serde_json::json!({
            "from": "Paused",
            "to": "Running"
        })).await;

        Ok(())
    }

    async fn cancel(&mut self) -> Result<(), AgentError> {
        let mut status = self.status.write().await;
        *status = AgentStatus::Idle;

        self.emit(AgentEventType::StateChanged, serde_json::json!({
            "to": "Idle"
        })).await;

        Ok(())
    }

    fn status(&self) -> AgentStatus {
        AgentStatus::Idle
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
