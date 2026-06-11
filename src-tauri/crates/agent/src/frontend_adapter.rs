// SPDX-License-Identifier: AGPL-3.0-only

use crate::event_bus::{AgentEventBus, AgentEventType, UnifiedAgentEvent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendEventPayload {
    pub event_type: String,
    pub timestamp: String,
    pub source: String,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
}

impl From<UnifiedAgentEvent> for FrontendEventPayload {
    fn from(event: UnifiedAgentEvent) -> Self {
        Self {
            event_type: event.event_type.to_string(),
            timestamp: event.timestamp.to_rfc3339(),
            source: event.source,
            payload: event.payload,
            correlation_id: event.correlation_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FrontendEventFilter {
    #[default]
    All,
    Specific(Vec<AgentEventType>),
}

pub struct FrontendEventAdapter {
    event_bus: Arc<AgentEventBus>,
    frontend_sender: broadcast::Sender<FrontendEventPayload>,
}

impl FrontendEventAdapter {
    pub fn new(event_bus: Arc<AgentEventBus>) -> Self {
        let (frontend_sender, _) = broadcast::channel(1000);
        Self {
            event_bus,
            frontend_sender,
        }
    }

    pub fn subscribe(
        &self,
        filter: FrontendEventFilter,
    ) -> broadcast::Receiver<FrontendEventPayload> {
        let receiver = self.frontend_sender.subscribe();

        let backend_receiver = match filter {
            FrontendEventFilter::All => self.event_bus.subscribe("frontend", vec![]),
            FrontendEventFilter::Specific(event_types) => {
                self.event_bus.subscribe("frontend", event_types)
            },
        };

        tokio::spawn({
            let sender = self.frontend_sender.clone();
            async move {
                let mut rx = backend_receiver;
                while let Ok(event) = rx.recv().await {
                    let payload: FrontendEventPayload = event.into();
                    if sender.send(payload).is_err() {
                        break;
                    }
                }
            }
        });

        receiver
    }

    pub fn broadcast(
        &self,
        event: UnifiedAgentEvent,
    ) -> Result<usize, broadcast::error::SendError<UnifiedAgentEvent>> {
        self.event_bus.emit(event)
    }

    pub fn event_bus(&self) -> Arc<AgentEventBus> {
        Arc::clone(&self.event_bus)
    }

    pub fn subscriber_count(&self) -> usize {
        self.frontend_sender.len()
    }
}

impl std::fmt::Debug for FrontendEventAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrontendEventAdapter")
            .field("event_bus", &self.event_bus.name())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrontendEventType {
    AgentTurnStarted,
    AgentTurnCompleted,
    AgentToolUse,
    AgentToolResult,
    AgentToolError,
    AgentStateChanged,
    AgentIterationComplete,
    AgentChainComplete,
    AgentResearchPhaseChanged,
    AgentSourceFound,
    AgentCitationAdded,
    AgentReportGenerated,
    AgentError,
    AgentWarning,
    AgentDebug,
    AgentLlmGenerationStarted,
    AgentLlmGenerationCompleted,
    AgentPermissionRequest,
    AgentPermissionGranted,
    AgentPermissionDenied,
}

impl From<AgentEventType> for FrontendEventType {
    fn from(e: AgentEventType) -> Self {
        match e {
            AgentEventType::TurnStarted => FrontendEventType::AgentTurnStarted,
            AgentEventType::TurnCompleted => FrontendEventType::AgentTurnCompleted,
            AgentEventType::ToolUse => FrontendEventType::AgentToolUse,
            AgentEventType::ToolResult => FrontendEventType::AgentToolResult,
            AgentEventType::ToolError => FrontendEventType::AgentToolError,
            AgentEventType::StateChanged => FrontendEventType::AgentStateChanged,
            AgentEventType::IterationComplete => FrontendEventType::AgentIterationComplete,
            AgentEventType::ChainComplete => FrontendEventType::AgentChainComplete,
            AgentEventType::ResearchPhaseChanged => FrontendEventType::AgentResearchPhaseChanged,
            AgentEventType::SourceFound => FrontendEventType::AgentSourceFound,
            AgentEventType::CitationAdded => FrontendEventType::AgentCitationAdded,
            AgentEventType::ReportGenerated => FrontendEventType::AgentReportGenerated,
            AgentEventType::Error => FrontendEventType::AgentError,
            AgentEventType::Warning => FrontendEventType::AgentWarning,
            AgentEventType::Debug => FrontendEventType::AgentDebug,
            AgentEventType::LlmGenerationStarted => FrontendEventType::AgentLlmGenerationStarted,
            AgentEventType::LlmGenerationCompleted => {
                FrontendEventType::AgentLlmGenerationCompleted
            },
            AgentEventType::PermissionRequest => FrontendEventType::AgentPermissionRequest,
            AgentEventType::PermissionGranted => FrontendEventType::AgentPermissionGranted,
            AgentEventType::PermissionDenied => FrontendEventType::AgentPermissionDenied,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TauriEventEnvelope {
    pub event_name: String,
    pub payload: FrontendEventPayload,
}

impl TauriEventEnvelope {
    pub fn new(event_type: AgentEventType, payload: FrontendEventPayload) -> Self {
        Self {
            event_name: format!("agent::{}", event_type),
            payload,
        }
    }
}

pub struct TauriEventAdapter {
    frontend_adapter: FrontendEventAdapter,
}

impl TauriEventAdapter {
    pub fn new(event_bus: Arc<AgentEventBus>) -> Self {
        Self {
            frontend_adapter: FrontendEventAdapter::new(event_bus),
        }
    }

    pub fn subscribe_all(&self) -> broadcast::Receiver<TauriEventEnvelope> {
        let (tx, rx) = broadcast::channel(1000);

        let receiver = self.frontend_adapter.subscribe(FrontendEventFilter::All);

        tokio::spawn(async move {
            let mut rx = receiver;
            while let Ok(payload) = rx.recv().await {
                let event_type = match payload.event_type.as_str() {
                    "TurnStarted" => AgentEventType::TurnStarted,
                    "TurnCompleted" => AgentEventType::TurnCompleted,
                    "ToolUse" => AgentEventType::ToolUse,
                    "ToolResult" => AgentEventType::ToolResult,
                    "ToolError" => AgentEventType::ToolError,
                    "StateChanged" => AgentEventType::StateChanged,
                    "IterationComplete" => AgentEventType::IterationComplete,
                    "ChainComplete" => AgentEventType::ChainComplete,
                    "ResearchPhaseChanged" => AgentEventType::ResearchPhaseChanged,
                    "SourceFound" => AgentEventType::SourceFound,
                    "CitationAdded" => AgentEventType::CitationAdded,
                    "ReportGenerated" => AgentEventType::ReportGenerated,
                    "Error" => AgentEventType::Error,
                    "Warning" => AgentEventType::Warning,
                    "Debug" => AgentEventType::Debug,
                    "LlmGenerationStarted" => AgentEventType::LlmGenerationStarted,
                    "LlmGenerationCompleted" => AgentEventType::LlmGenerationCompleted,
                    _ => AgentEventType::Debug,
                };

                let envelope = TauriEventEnvelope::new(event_type, payload);
                if tx.send(envelope).is_err() {
                    break;
                }
            }
        });

        rx
    }

    pub fn subscribe_specific(
        &self,
        event_types: Vec<AgentEventType>,
    ) -> broadcast::Receiver<TauriEventEnvelope> {
        let (tx, rx) = broadcast::channel(1000);

        let receiver = self
            .frontend_adapter
            .subscribe(FrontendEventFilter::Specific(event_types.clone()));

        tokio::spawn(async move {
            let mut rx = receiver;
            while let Ok(payload) = rx.recv().await {
                let event_type = match payload.event_type.as_str() {
                    "TurnStarted" => AgentEventType::TurnStarted,
                    "TurnCompleted" => AgentEventType::TurnCompleted,
                    "ToolUse" => AgentEventType::ToolUse,
                    "ToolResult" => AgentEventType::ToolResult,
                    "ToolError" => AgentEventType::ToolError,
                    "StateChanged" => AgentEventType::StateChanged,
                    "IterationComplete" => AgentEventType::IterationComplete,
                    "ChainComplete" => AgentEventType::ChainComplete,
                    "ResearchPhaseChanged" => AgentEventType::ResearchPhaseChanged,
                    "SourceFound" => AgentEventType::SourceFound,
                    "CitationAdded" => AgentEventType::CitationAdded,
                    "ReportGenerated" => AgentEventType::ReportGenerated,
                    "Error" => AgentEventType::Error,
                    "Warning" => AgentEventType::Warning,
                    "Debug" => AgentEventType::Debug,
                    "LlmGenerationStarted" => AgentEventType::LlmGenerationStarted,
                    "LlmGenerationCompleted" => AgentEventType::LlmGenerationCompleted,
                    _ => return,
                };

                if !event_types.contains(&event_type) {
                    continue;
                }

                let envelope = TauriEventEnvelope::new(event_type, payload);
                if tx.send(envelope).is_err() {
                    break;
                }
            }
        });

        rx
    }

    pub fn emit(
        &self,
        event: UnifiedAgentEvent,
    ) -> Result<usize, broadcast::error::SendError<UnifiedAgentEvent>> {
        self.frontend_adapter.broadcast(event)
    }

    pub fn event_bus(&self) -> Arc<AgentEventBus> {
        self.frontend_adapter.event_bus()
    }
}

impl std::fmt::Debug for TauriEventAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TauriEventAdapter")
            .field("event_bus", &self.frontend_adapter.event_bus().name())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontend_event_filter_default() {
        let filter = FrontendEventFilter::default();
        assert_eq!(filter, FrontendEventFilter::All);
    }

    #[test]
    fn test_frontend_event_filter_equality() {
        assert_eq!(FrontendEventFilter::All, FrontendEventFilter::All);
        assert_eq!(
            FrontendEventFilter::Specific(vec![AgentEventType::TurnStarted]),
            FrontendEventFilter::Specific(vec![AgentEventType::TurnStarted])
        );
        assert_ne!(FrontendEventFilter::All, FrontendEventFilter::Specific(vec![]));
    }

    #[test]
    fn test_frontend_event_type_from_agent_event_type() {
        assert_eq!(
            FrontendEventType::from(AgentEventType::TurnStarted),
            FrontendEventType::AgentTurnStarted
        );
        assert_eq!(
            FrontendEventType::from(AgentEventType::TurnCompleted),
            FrontendEventType::AgentTurnCompleted
        );
        assert_eq!(
            FrontendEventType::from(AgentEventType::ToolUse),
            FrontendEventType::AgentToolUse
        );
        assert_eq!(
            FrontendEventType::from(AgentEventType::ToolResult),
            FrontendEventType::AgentToolResult
        );
        assert_eq!(
            FrontendEventType::from(AgentEventType::ToolError),
            FrontendEventType::AgentToolError
        );
        assert_eq!(
            FrontendEventType::from(AgentEventType::StateChanged),
            FrontendEventType::AgentStateChanged
        );
        assert_eq!(FrontendEventType::from(AgentEventType::Error), FrontendEventType::AgentError);
        assert_eq!(
            FrontendEventType::from(AgentEventType::Warning),
            FrontendEventType::AgentWarning
        );
        assert_eq!(FrontendEventType::from(AgentEventType::Debug), FrontendEventType::AgentDebug);
    }

    #[test]
    fn test_frontend_event_payload_serialization() {
        let payload = FrontendEventPayload {
            event_type: "TurnStarted".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            source: "agent".to_string(),
            payload: serde_json::json!({"key": "value"}),
            correlation_id: Some("corr-1".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: FrontendEventPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, "TurnStarted");
        assert_eq!(deserialized.source, "agent");
        assert_eq!(deserialized.correlation_id, Some("corr-1".to_string()));
    }

    #[test]
    fn test_tauri_event_envelope_new() {
        let payload = FrontendEventPayload {
            event_type: "TurnStarted".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            source: "agent".to_string(),
            payload: serde_json::json!(null),
            correlation_id: None,
        };
        let envelope = TauriEventEnvelope::new(AgentEventType::TurnStarted, payload);
        assert_eq!(envelope.event_name, "agent::TurnStarted");
    }

    #[test]
    fn test_tauri_event_envelope_serialization() {
        let payload = FrontendEventPayload {
            event_type: "Error".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            source: "test".to_string(),
            payload: serde_json::json!({"msg": "fail"}),
            correlation_id: None,
        };
        let envelope = TauriEventEnvelope::new(AgentEventType::Error, payload);
        let json = serde_json::to_string(&envelope).unwrap();
        let deserialized: TauriEventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_name, "agent::Error");
    }

    #[test]
    fn test_frontend_event_type_all_variants() {
        let variants = vec![
            FrontendEventType::AgentTurnStarted,
            FrontendEventType::AgentTurnCompleted,
            FrontendEventType::AgentToolUse,
            FrontendEventType::AgentToolResult,
            FrontendEventType::AgentToolError,
            FrontendEventType::AgentStateChanged,
            FrontendEventType::AgentIterationComplete,
            FrontendEventType::AgentChainComplete,
            FrontendEventType::AgentResearchPhaseChanged,
            FrontendEventType::AgentSourceFound,
            FrontendEventType::AgentCitationAdded,
            FrontendEventType::AgentReportGenerated,
            FrontendEventType::AgentError,
            FrontendEventType::AgentWarning,
            FrontendEventType::AgentDebug,
            FrontendEventType::AgentLlmGenerationStarted,
            FrontendEventType::AgentLlmGenerationCompleted,
            FrontendEventType::AgentPermissionRequest,
            FrontendEventType::AgentPermissionGranted,
            FrontendEventType::AgentPermissionDenied,
        ];
        assert_eq!(variants.len(), 20);
    }

    #[test]
    fn test_frontend_event_type_equality() {
        assert_eq!(FrontendEventType::AgentTurnStarted, FrontendEventType::AgentTurnStarted);
        assert_ne!(FrontendEventType::AgentTurnStarted, FrontendEventType::AgentTurnCompleted);
    }

    #[test]
    fn test_frontend_event_payload_no_correlation_id() {
        let payload = FrontendEventPayload {
            event_type: "Debug".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            source: "agent".to_string(),
            payload: serde_json::json!(null),
            correlation_id: None,
        };
        assert!(payload.correlation_id.is_none());
    }
}
