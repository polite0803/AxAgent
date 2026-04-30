use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPermissionPayload {
    pub conversation_id: String,
    pub assistant_message_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub risk_level: String,
    pub request_id: String,
    #[serde(rename = "toolUseId")]
    pub tool_use_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentEventType {
    TurnStarted,
    TurnCompleted,
    ToolUse,
    ToolResult,
    ToolError,
    StateChanged,
    IterationComplete,
    ChainComplete,
    ResearchPhaseChanged,
    SourceFound,
    CitationAdded,
    ReportGenerated,
    Error,
    Warning,
    Debug,
    LlmGenerationStarted,
    LlmGenerationCompleted,
    PermissionRequest,
    PermissionGranted,
    PermissionDenied,
}

impl std::fmt::Display for AgentEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentEventType::TurnStarted => write!(f, "TurnStarted"),
            AgentEventType::TurnCompleted => write!(f, "TurnCompleted"),
            AgentEventType::ToolUse => write!(f, "ToolUse"),
            AgentEventType::ToolResult => write!(f, "ToolResult"),
            AgentEventType::ToolError => write!(f, "ToolError"),
            AgentEventType::StateChanged => write!(f, "StateChanged"),
            AgentEventType::IterationComplete => write!(f, "IterationComplete"),
            AgentEventType::ChainComplete => write!(f, "ChainComplete"),
            AgentEventType::ResearchPhaseChanged => write!(f, "ResearchPhaseChanged"),
            AgentEventType::SourceFound => write!(f, "SourceFound"),
            AgentEventType::CitationAdded => write!(f, "CitationAdded"),
            AgentEventType::ReportGenerated => write!(f, "ReportGenerated"),
            AgentEventType::Error => write!(f, "Error"),
            AgentEventType::Warning => write!(f, "Warning"),
            AgentEventType::Debug => write!(f, "Debug"),
            AgentEventType::LlmGenerationStarted => write!(f, "LlmGenerationStarted"),
            AgentEventType::LlmGenerationCompleted => write!(f, "LlmGenerationCompleted"),
            AgentEventType::PermissionRequest => write!(f, "PermissionRequest"),
            AgentEventType::PermissionGranted => write!(f, "PermissionGranted"),
            AgentEventType::PermissionDenied => write!(f, "PermissionDenied"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAgentEvent {
    pub event_type: AgentEventType,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
}

impl UnifiedAgentEvent {
    pub fn new(
        source: impl Into<String>,
        event_type: AgentEventType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_type,
            timestamp: Utc::now(),
            source: source.into(),
            payload,
            correlation_id: None,
        }
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

#[derive(Debug)]
pub struct EventSubscription {
    pub event_types: Vec<AgentEventType>,
    pub receiver: broadcast::Receiver<UnifiedAgentEvent>,
}

pub struct AgentEventBus {
    sender: broadcast::Sender<UnifiedAgentEvent>,
    subscriptions: tokio::sync::RwLock<HashMap<String, EventSubscription>>,
    name: String,
}

impl AgentEventBus {
    pub fn new(name: impl Into<String>) -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            sender,
            subscriptions: tokio::sync::RwLock::new(HashMap::new()),
            name: name.into(),
        }
    }

    pub fn builder() -> AgentEventBusBuilder {
        AgentEventBusBuilder::new()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn subscribe(
        &self,
        subscriber_id: impl Into<String>,
        event_types: Vec<AgentEventType>,
    ) -> broadcast::Receiver<UnifiedAgentEvent> {
        let receiver = self.sender.subscribe();
        let subscription = EventSubscription {
            event_types,
            receiver: self.sender.subscribe(),
        };

        let subscriber_id = subscriber_id.into();
        let mut subs = self.subscriptions.blocking_write();
        subs.insert(subscriber_id, subscription);

        receiver
    }

    pub fn unsubscribe(&self, subscriber_id: &str) {
        let mut subs = self.subscriptions.blocking_write();
        subs.remove(subscriber_id);
    }

    pub fn emit(
        &self,
        event: UnifiedAgentEvent,
    ) -> Result<usize, broadcast::error::SendError<UnifiedAgentEvent>> {
        tracing::debug!(
            "EventBus[{}] emitting: {} from {}",
            self.name,
            event.event_type,
            event.source
        );

        self.sender.send(event)
    }

    pub fn emit_to_all(
        &self,
        event: UnifiedAgentEvent,
    ) -> Result<usize, broadcast::error::SendError<UnifiedAgentEvent>> {
        let count = self.sender.len();
        self.sender.send(event)?;
        Ok(count)
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscriptions.try_read().map(|s| s.len()).unwrap_or(0)
    }

    pub async fn get_subscriptions(&self) -> Vec<(String, Vec<AgentEventType>)> {
        let subs = self.subscriptions.read().await;
        subs.iter()
            .map(|(id, sub)| (id.clone(), sub.event_types.clone()))
            .collect()
    }
}

impl std::fmt::Debug for AgentEventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentEventBus")
            .field("name", &self.name)
            .field("subscriber_count", &self.subscriber_count())
            .finish()
    }
}

pub struct AgentEventBusBuilder {
    name: String,
}

impl AgentEventBusBuilder {
    pub fn new() -> Self {
        Self {
            name: "default".to_string(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn build(self) -> AgentEventBus {
        AgentEventBus::new(self.name)
    }
}

impl Default for AgentEventBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_basic() {
        let bus = AgentEventBus::new("test");
        let mut receiver = bus.subscribe("sub1", vec![AgentEventType::TurnStarted]);

        let event = UnifiedAgentEvent::new(
            "test_source",
            AgentEventType::TurnStarted,
            serde_json::json!({"iteration": 1}),
        );

        bus.emit(event).unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.event_type, AgentEventType::TurnStarted);
        assert_eq!(received.source, "test_source");
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = AgentEventBus::new("test");
        let _receiver1 = bus.subscribe("sub1", vec![AgentEventType::TurnStarted]);
        let _receiver2 = bus.subscribe(
            "sub2",
            vec![AgentEventType::TurnStarted, AgentEventType::Error],
        );

        let event =
            UnifiedAgentEvent::new("source", AgentEventType::TurnStarted, serde_json::json!({}));
        bus.emit(event).unwrap();

        let subscriptions = bus.get_subscriptions().await;
        assert_eq!(subscriptions.len(), 2);
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(AgentEventType::TurnStarted.to_string(), "TurnStarted");
        assert_eq!(AgentEventType::ToolError.to_string(), "ToolError");
    }
}
