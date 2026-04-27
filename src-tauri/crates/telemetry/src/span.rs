use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    Agent,
    Tool,
    LlmCall,
    Task,
    SubTask,
    Reflection,
    Reasoning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    Ok,
    Error,
    Cancelled,
}

impl Default for SpanStatus {
    fn default() -> Self {
        Self::Ok
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub span_type: SpanType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    pub start_time: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub status: SpanStatus,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<SpanEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<SpanError>,
}

impl Span {
    pub fn new(
        id: String,
        trace_id: String,
        parent_span_id: Option<String>,
        name: String,
        span_type: SpanType,
    ) -> Self {
        Self {
            id,
            trace_id,
            parent_span_id,
            name,
            span_type,
            service_name: None,
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
            inputs: None,
            outputs: None,
            errors: Vec::new(),
        }
    }

    pub fn with_service_name(mut self, service_name: impl Into<String>) -> Self {
        self.service_name = Some(service_name.into());
        self
    }

    pub fn finish(&mut self) {
        self.end_time = Some(Utc::now());
        if let (Some(start), Some(end)) = (
            self.start_time
                .checked_sub_signed(chrono::Duration::milliseconds(0)),
            self.end_time,
        ) {
            self.duration_ms = Some((end - start).num_milliseconds() as u64);
        }
    }

    pub fn set_status(&mut self, status: SpanStatus) {
        self.status = status;
    }

    pub fn set_attribute(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.attributes.insert(key.into(), value);
    }

    pub fn add_event(&mut self, event: SpanEvent) {
        self.events.push(event);
    }

    pub fn record_error(&mut self, error: SpanError) {
        self.errors.push(error);
        self.status = SpanStatus::Error;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, serde_json::Value>,
}

impl SpanEvent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanError {
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl SpanError {
    pub fn new(error_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error_type: error_type.into(),
            message: message.into(),
            stack_trace: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_stack_trace(mut self, stack_trace: impl Into<String>) -> Self {
        self.stack_trace = Some(stack_trace.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMetadata {
    pub user_id: String,
    pub session_id: String,
    pub agent_version: String,
    pub model: String,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub total_duration_ms: u64,
}

impl Default for TraceMetadata {
    fn default() -> Self {
        Self {
            user_id: "default".to_string(),
            session_id: "default".to_string(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            model: "unknown".to_string(),
            total_tokens: 0,
            total_cost_usd: 0.0,
            total_duration_ms: 0,
        }
    }
}
