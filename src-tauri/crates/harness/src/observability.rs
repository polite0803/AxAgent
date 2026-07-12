// SPDX-License-Identifier: AGPL-3.0-only
//! 可观测性契约
use async_trait::async_trait;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservabilitySpanType {
    Agent,
    Tool,
    LlmCall,
    Task,
    Workflow,
    Custom(String),
}

#[async_trait]
pub trait ObservabilityProvider: Send + Sync {
    async fn start_span(
        &self,
        name: &str,
        span_type: ObservabilitySpanType,
        attributes: Map<String, Value>,
    );
    async fn end_span(&self, attributes: Map<String, Value>);
    async fn record_event(&self, name: &str, attributes: Map<String, Value>);
    async fn record_metric(&self, name: &str, value: f64, labels: Map<String, Value>);
    async fn record_error(&self, error: &str, attributes: Map<String, Value>);
    async fn export_traces(&self) -> Result<String, String>;
    async fn export_metrics(&self) -> Result<String, String>;
}
