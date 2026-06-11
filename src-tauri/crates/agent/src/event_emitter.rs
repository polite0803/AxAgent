// SPDX-License-Identifier: AGPL-3.0-only

//! Event payload types for AxAgent Agent Tauri events.
//!
//! This module is deprecated. All types have been moved to event_bus.rs.
//! Use AgentPermissionPayload from event_bus instead.

pub use super::event_bus::AgentPermissionPayload;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::{AgentEventBus, AgentEventType, UnifiedAgentEvent};

    #[test]
    fn test_agent_permission_payload_creation() {
        let payload = AgentPermissionPayload {
            conversation_id: "conv-1".to_string(),
            assistant_message_id: "msg-1".to_string(),
            tool_name: "FileRead".to_string(),
            input: serde_json::json!({"path": "/tmp/test"}),
            risk_level: "low".to_string(),
            request_id: "req-1".to_string(),
            tool_use_id: Some("tool-1".to_string()),
        };
        assert_eq!(payload.conversation_id, "conv-1");
        assert_eq!(payload.assistant_message_id, "msg-1");
        assert_eq!(payload.tool_name, "FileRead");
        assert_eq!(payload.risk_level, "low");
        assert_eq!(payload.request_id, "req-1");
        assert!(payload.tool_use_id.is_some());
    }

    #[test]
    fn test_agent_permission_payload_without_tool_use_id() {
        let payload = AgentPermissionPayload {
            conversation_id: "conv-2".to_string(),
            assistant_message_id: "msg-2".to_string(),
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"cmd": "ls"}),
            risk_level: "high".to_string(),
            request_id: "req-2".to_string(),
            tool_use_id: None,
        };
        assert!(payload.tool_use_id.is_none());
    }

    #[test]
    fn test_agent_permission_payload_serialization() {
        let payload = AgentPermissionPayload {
            conversation_id: "conv-3".to_string(),
            assistant_message_id: "msg-3".to_string(),
            tool_name: "Grep".to_string(),
            input: serde_json::json!({"pattern": "test"}),
            risk_level: "medium".to_string(),
            request_id: "req-3".to_string(),
            tool_use_id: Some("tool-3".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: AgentPermissionPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.conversation_id, "conv-3");
        assert_eq!(deserialized.tool_name, "Grep");
    }

    #[test]
    fn test_agent_permission_payload_tool_use_id_serialization() {
        let payload = AgentPermissionPayload {
            conversation_id: "conv-4".to_string(),
            assistant_message_id: "msg-4".to_string(),
            tool_name: "FileWrite".to_string(),
            input: serde_json::json!({}),
            risk_level: "high".to_string(),
            request_id: "req-4".to_string(),
            tool_use_id: Some("tu-123".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("toolUseId"));
    }

    #[test]
    fn test_event_type_variants() {
        let variants = vec![
            AgentEventType::TurnStarted,
            AgentEventType::TurnCompleted,
            AgentEventType::ToolUse,
            AgentEventType::ToolResult,
            AgentEventType::ToolError,
            AgentEventType::StateChanged,
            AgentEventType::IterationComplete,
            AgentEventType::ChainComplete,
            AgentEventType::ResearchPhaseChanged,
            AgentEventType::SourceFound,
            AgentEventType::CitationAdded,
            AgentEventType::ReportGenerated,
            AgentEventType::Error,
            AgentEventType::Warning,
            AgentEventType::Debug,
            AgentEventType::LlmGenerationStarted,
            AgentEventType::LlmGenerationCompleted,
            AgentEventType::PermissionRequest,
            AgentEventType::PermissionGranted,
            AgentEventType::PermissionDenied,
        ];
        assert_eq!(variants.len(), 20);
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(AgentEventType::PermissionRequest.to_string(), "PermissionRequest");
        assert_eq!(AgentEventType::PermissionGranted.to_string(), "PermissionGranted");
        assert_eq!(AgentEventType::PermissionDenied.to_string(), "PermissionDenied");
    }

    #[test]
    fn test_unified_agent_event_creation() {
        let event = UnifiedAgentEvent::new(
            "test_source",
            AgentEventType::ToolUse,
            serde_json::json!({"tool": "read"}),
        );
        assert_eq!(event.event_type, AgentEventType::ToolUse);
        assert_eq!(event.source, "test_source");
        assert!(event.correlation_id.is_none());
    }

    #[test]
    fn test_unified_agent_event_with_correlation_id() {
        let event =
            UnifiedAgentEvent::new("source", AgentEventType::ToolResult, serde_json::json!({}))
                .with_correlation_id("corr-123");
        assert_eq!(event.correlation_id.unwrap(), "corr-123");
    }

    #[tokio::test]
    async fn test_event_bus_creation() {
        let bus = AgentEventBus::new("test_bus");
        assert_eq!(bus.name(), "test_bus");
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_event_bus_subscribe_and_emit() {
        let bus = AgentEventBus::new("test");
        let mut receiver = bus.subscribe("sub1", vec![AgentEventType::ToolUse]);

        let event = UnifiedAgentEvent::new(
            "source",
            AgentEventType::ToolUse,
            serde_json::json!({"tool": "bash"}),
        );
        bus.emit(event).unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.event_type, AgentEventType::ToolUse);
        assert_eq!(received.source, "source");
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = AgentEventBus::new("test");
        let mut receiver1 = bus.subscribe("sub1", vec![AgentEventType::TurnStarted]);
        let mut receiver2 = bus.subscribe("sub2", vec![AgentEventType::TurnStarted]);

        let event =
            UnifiedAgentEvent::new("src", AgentEventType::TurnStarted, serde_json::json!({}));
        bus.emit(event).unwrap();

        let r1 = receiver1.recv().await.unwrap();
        let r2 = receiver2.recv().await.unwrap();
        assert_eq!(r1.event_type, AgentEventType::TurnStarted);
        assert_eq!(r2.event_type, AgentEventType::TurnStarted);
    }

    #[tokio::test]
    async fn test_event_bus_unsubscribe() {
        let bus = AgentEventBus::new("test");
        let _receiver = bus.subscribe("sub1", vec![AgentEventType::Error]);
        assert_eq!(bus.subscriber_count(), 1);

        bus.unsubscribe("sub1");
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_event_bus_builder() {
        let bus = AgentEventBus::builder().name("builder_test").build();
        assert_eq!(bus.name(), "builder_test");
    }

    #[tokio::test]
    async fn test_event_bus_get_subscriptions() {
        let bus = AgentEventBus::new("test");
        let _r1 = bus.subscribe("sub1", vec![AgentEventType::TurnStarted]);
        let _r2 = bus.subscribe("sub2", vec![AgentEventType::TurnStarted, AgentEventType::Error]);

        let subs = bus.get_subscriptions().await;
        assert_eq!(subs.len(), 2);
    }

    #[tokio::test]
    async fn test_event_bus_emit_permission_request() {
        let bus = AgentEventBus::new("test");
        let mut receiver = bus.subscribe("perm_sub", vec![AgentEventType::PermissionRequest]);

        let payload = AgentPermissionPayload {
            conversation_id: "conv-1".to_string(),
            assistant_message_id: "msg-1".to_string(),
            tool_name: "FileWrite".to_string(),
            input: serde_json::json!({"path": "/etc/passwd"}),
            risk_level: "high".to_string(),
            request_id: "req-1".to_string(),
            tool_use_id: None,
        };

        let event = UnifiedAgentEvent::new(
            "agent",
            AgentEventType::PermissionRequest,
            serde_json::to_value(&payload).unwrap(),
        );
        bus.emit(event).unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.event_type, AgentEventType::PermissionRequest);
        let received_payload: AgentPermissionPayload =
            serde_json::from_value(received.payload).unwrap();
        assert_eq!(received_payload.tool_name, "FileWrite");
        assert_eq!(received_payload.risk_level, "high");
    }
}
