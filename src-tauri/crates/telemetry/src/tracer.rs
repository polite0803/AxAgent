use crate::exporter::{NoopExporter, TraceExport, TraceExporter};
use crate::metrics::{CostMetrics, TraceMetrics};
use crate::span::{Span, SpanError, SpanEvent, SpanStatus, SpanType, TraceMetadata};
use crate::storage::{InMemoryTraceStorage, TraceStorage};
use chrono::Utc;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

fn generate_trace_id() -> String {
    Uuid::new_v4().to_string()
}

fn generate_span_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone)]
pub struct TracerConfig {
    pub include_inputs: bool,
    pub include_outputs: bool,
    pub include_events: bool,
    pub service_name: Option<String>,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            include_inputs: true,
            include_outputs: true,
            include_events: true,
            service_name: None,
        }
    }
}

pub struct Tracer {
    trace_id: String,
    session_id: String,
    current_span: Option<String>,
    spans: Arc<RwLock<Vec<Span>>>,
    storage: Arc<dyn TraceStorage>,
    exporter: Box<dyn TraceExporter>,
    config: TracerConfig,
    metadata: TraceMetadata,
}

impl Clone for Tracer {
    fn clone(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            session_id: self.session_id.clone(),
            current_span: self.current_span.clone(),
            spans: self.spans.clone(),
            storage: self.storage.clone(),
            exporter: Box::new(NoopExporter),
            config: self.config.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

impl Tracer {
    pub fn new(trace_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            session_id: session_id.into(),
            current_span: None,
            spans: Arc::new(RwLock::new(Vec::new())),
            storage: Arc::new(InMemoryTraceStorage::new()),
            exporter: Box::new(NoopExporter),
            config: TracerConfig::default(),
            metadata: TraceMetadata::default(),
        }
    }

    pub fn with_config(mut self, config: TracerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_storage<S: TraceStorage + 'static>(mut self, storage: S) -> Self {
        self.storage = Arc::new(storage);
        self
    }

    pub fn with_exporter<E: TraceExporter + 'static>(mut self, exporter: E) -> Self {
        self.exporter = Box::new(exporter);
        self
    }

    pub fn with_metadata(mut self, metadata: TraceMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn start_span(&mut self, name: &str, span_type: SpanType) -> SpanGuard<'_> {
        let span_id = generate_span_id();
        let parent_span_id = self.current_span.clone();

        let mut span = Span::new(
            span_id.clone(),
            self.trace_id.clone(),
            parent_span_id,
            name.to_string(),
            span_type,
        );

        if let Some(ref service_name) = self.config.service_name {
            span.service_name = Some(service_name.clone());
        }

        if self.config.include_inputs {
            span.inputs = Some(serde_json::json!({}));
        }
        if self.config.include_outputs {
            span.outputs = Some(serde_json::json!({}));
        }

        let span_clone = span.clone();
        self.spans.write().unwrap().push(span);
        self.current_span = Some(span_id.clone());

        SpanGuard {
            tracer: self,
            span_id,
            span_type,
            _span: span_clone,
        }
    }

    pub fn end_span(&mut self, span_id: &str, status: SpanStatus) {
        let mut spans = self.spans.write().unwrap();
        if let Some(span) = spans.iter_mut().find(|s| s.id == span_id) {
            span.end_time = Some(Utc::now());
            span.duration_ms = span
                .start_time
                .checked_add_signed(chrono::Duration::milliseconds(0))
                .and_then(|start| {
                    span.end_time
                        .map(|end| (end - start).num_milliseconds() as u64)
                });
            span.status = status;
        }
        drop(spans);

        if self.current_span.as_deref() == Some(span_id) {
            let spans = self.spans.read().unwrap();
            let current = spans
                .iter()
                .rfind(|s| s.parent_span_id.as_deref() == Some(span_id))
                .map(|s| s.id.clone());
            drop(spans);
            self.current_span = current;
        }
    }

    pub fn add_event(&mut self, span_id: &str, event: SpanEvent) {
        if !self.config.include_events {
            return;
        }
        let mut spans = self.spans.write().unwrap();
        if let Some(span) = spans.iter_mut().find(|s| s.id == span_id) {
            span.add_event(event);
        }
    }

    pub fn record_error(&mut self, span_id: &str, error: SpanError) {
        let mut spans = self.spans.write().unwrap();
        if let Some(span) = spans.iter_mut().find(|s| s.id == span_id) {
            span.record_error(error);
        }
    }

    pub fn set_inputs(&mut self, span_id: &str, inputs: serde_json::Value) {
        if !self.config.include_inputs {
            return;
        }
        let mut spans = self.spans.write().unwrap();
        if let Some(span) = spans.iter_mut().find(|s| s.id == span_id) {
            span.inputs = Some(inputs);
        }
    }

    pub fn set_outputs(&mut self, span_id: &str, outputs: serde_json::Value) {
        if !self.config.include_outputs {
            return;
        }
        let mut spans = self.spans.write().unwrap();
        if let Some(span) = spans.iter_mut().find(|s| s.id == span_id) {
            span.outputs = Some(outputs);
        }
    }

    pub fn set_attribute(&mut self, span_id: &str, key: &str, value: serde_json::Value) {
        let mut spans = self.spans.write().unwrap();
        if let Some(span) = spans.iter_mut().find(|s| s.id == span_id) {
            span.set_attribute(key, value);
        }
    }

    pub fn get_span(&self, span_id: &str) -> Option<Span> {
        let spans = self.spans.read().unwrap();
        spans.iter().find(|s| s.id == span_id).cloned()
    }

    pub fn get_all_spans(&self) -> Vec<Span> {
        let spans = self.spans.read().unwrap();
        spans.clone()
    }

    pub fn get_current_span_id(&self) -> Option<&str> {
        self.current_span.as_deref()
    }

    pub fn export(&self) -> Result<(), crate::exporter::TracerError> {
        let spans = self.spans.read().unwrap();
        let trace = TraceExport::new(self.trace_id.clone(), spans.clone(), self.metadata.clone());
        self.exporter.export(trace)
    }

    pub fn store(&self) -> Result<(), crate::storage::StorageError> {
        let spans = self.spans.read().unwrap();
        let trace = TraceExport::new(self.trace_id.clone(), spans.clone(), self.metadata.clone());
        self.storage.store(trace)
    }

    pub fn metrics(&self) -> TraceMetrics {
        let spans = self.spans.read().unwrap();
        let total_duration: u64 = spans.iter().filter_map(|s| s.duration_ms).sum();
        let errors_count = spans.iter().map(|s| s.errors.len()).sum();

        TraceMetrics {
            total_duration_ms: total_duration,
            ttft_ms: None,
            cost: CostMetrics::new(&self.metadata.model),
            spans_count: spans.len(),
            errors_count,
        }
    }

    pub fn update_metadata(&mut self, updates: TraceMetadataUpdate) {
        if let Some(tokens) = updates.total_tokens {
            self.metadata.total_tokens = tokens;
        }
        if let Some(cost) = updates.total_cost_usd {
            self.metadata.total_cost_usd = cost;
        }
        if let Some(model) = updates.model {
            self.metadata.model = model;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TraceMetadataUpdate {
    pub total_tokens: Option<u64>,
    pub total_cost_usd: Option<f64>,
    pub model: Option<String>,
}

impl TraceMetadataUpdate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tokens(mut self, tokens: u64) -> Self {
        self.total_tokens = Some(tokens);
        self
    }

    pub fn with_cost(mut self, cost: f64) -> Self {
        self.total_cost_usd = Some(cost);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

#[must_use]
pub struct SpanGuard<'a> {
    tracer: &'a mut Tracer,
    span_id: String,
    #[allow(dead_code)]
    span_type: SpanType,
    _span: Span,
}

impl<'a> SpanGuard<'a> {
    pub fn span_id(&self) -> &str {
        &self.span_id
    }

    pub fn set_inputs(&mut self, inputs: serde_json::Value) {
        self.tracer.set_inputs(&self.span_id, inputs);
    }

    pub fn set_outputs(&mut self, outputs: serde_json::Value) {
        self.tracer.set_outputs(&self.span_id, outputs);
    }

    pub fn set_attribute(&mut self, key: &str, value: serde_json::Value) {
        self.tracer.set_attribute(&self.span_id, key, value);
    }

    pub fn add_event(&mut self, event: SpanEvent) {
        self.tracer.add_event(&self.span_id, event);
    }

    pub fn record_error(&mut self, error: SpanError) {
        self.tracer.record_error(&self.span_id, error);
    }

    pub fn finish_with_status(self, status: SpanStatus) {
        self.tracer.end_span(&self.span_id, status);
    }
}

impl<'a> Drop for SpanGuard<'a> {
    fn drop(&mut self) {
        self.tracer.end_span(&self.span_id, SpanStatus::Ok);
    }
}

pub struct TracerBuilder {
    trace_id: Option<String>,
    session_id: Option<String>,
    config: TracerConfig,
    storage: Option<Arc<dyn TraceStorage>>,
    exporter: Option<Box<dyn TraceExporter>>,
    metadata: Option<TraceMetadata>,
}

impl TracerBuilder {
    pub fn new() -> Self {
        Self {
            trace_id: None,
            session_id: None,
            config: TracerConfig::default(),
            storage: None,
            exporter: None,
            metadata: None,
        }
    }

    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn config(mut self, config: TracerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn storage<S: TraceStorage + 'static>(mut self, storage: S) -> Self {
        self.storage = Some(Arc::new(storage));
        self
    }

    pub fn exporter<E: TraceExporter + 'static>(mut self, exporter: E) -> Self {
        self.exporter = Some(Box::new(exporter));
        self
    }

    pub fn metadata(mut self, metadata: TraceMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn build(self) -> Tracer {
        let trace_id = self.trace_id.unwrap_or_else(generate_trace_id);
        let session_id = self.session_id.unwrap_or_else(|| "default".to_string());

        let mut tracer = Tracer::new(trace_id, session_id).with_config(self.config);

        if let Some(storage) = self.storage {
            tracer.storage = storage;
        }

        if let Some(exporter) = self.exporter {
            tracer.exporter = exporter;
        }

        if let Some(metadata) = self.metadata {
            tracer.metadata = metadata;
        }

        tracer
    }
}

impl Default for TracerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GlobalTracer {
    tracer: RwLock<Option<Tracer>>,
}

impl GlobalTracer {
    pub fn new() -> Self {
        Self {
            tracer: RwLock::new(None),
        }
    }

    pub fn set(&self, tracer: Tracer) {
        let mut guard = self.tracer.write().unwrap();
        *guard = Some(tracer);
    }

    pub fn get(&self) -> Option<Tracer> {
        let guard = self.tracer.read().unwrap();
        guard.clone()
    }

    pub fn clear(&self) {
        let mut guard = self.tracer.write().unwrap();
        *guard = None;
    }
}

impl Default for GlobalTracer {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_TRACER: GlobalTracer = GlobalTracer::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracer_creation() {
        let tracer = Tracer::new("trace-1", "session-1");
        assert_eq!(tracer.trace_id(), "trace-1");
        assert_eq!(tracer.session_id(), "session-1");
    }

    #[test]
    fn test_span_lifecycle() {
        let mut tracer = Tracer::new("trace-1", "session-1");

        let guard = tracer.start_span("test-span", SpanType::Agent);
        let _span_id = guard.span_id().to_string();
        drop(guard);

        let spans = tracer.get_all_spans();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].name, "test-span");
    }

    #[test]
    fn test_tracer_builder() {
        let tracer = TracerBuilder::new()
            .trace_id("trace-1")
            .session_id("session-1")
            .build();

        assert_eq!(tracer.trace_id(), "trace-1");
    }

    #[test]
    fn test_metadata_update() {
        let mut tracer = Tracer::new("trace-1", "session-1");
        tracer.update_metadata(
            TraceMetadataUpdate::new()
                .with_tokens(1000)
                .with_cost(0.05)
                .with_model("claude-sonnet"),
        );

        let metrics = tracer.metrics();
        assert_eq!(metrics.spans_count, 0);
    }
}
