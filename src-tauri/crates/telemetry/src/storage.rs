use crate::exporter::TraceExport;
use crate::span::{Span, TraceMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    pub trace_id: String,
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub span_count: usize,
    pub error_count: usize,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
}

impl TraceSummary {
    pub fn from_spans(trace_id: String, spans: &[Span], metadata: &TraceMetadata) -> Self {
        let error_count = spans.iter().map(|s| s.errors.len()).sum();
        let started_at = spans
            .iter()
            .map(|s| s.start_time)
            .min()
            .unwrap_or_else(Utc::now);
        let ended_at = spans.iter().filter_map(|s| s.end_time).max();
        let duration_ms = ended_at.map(|end| (end - started_at).num_milliseconds() as u64);

        Self {
            trace_id,
            session_id: metadata.session_id.clone(),
            started_at,
            ended_at,
            duration_ms,
            span_count: spans.len(),
            error_count,
            total_tokens: metadata.total_tokens,
            total_cost_usd: metadata.total_cost_usd,
        }
    }
}

pub trait TraceStorage: Send + Sync {
    fn store(&self, trace: TraceExport) -> Result<(), StorageError>;
    fn get(&self, trace_id: &str) -> Result<Option<TraceExport>, StorageError>;
    fn list(&self, filter: &TraceFilter) -> Result<Vec<TraceSummary>, StorageError>;
    fn delete(&self, trace_id: &str) -> Result<(), StorageError>;
    fn clear(&self) -> Result<(), StorageError>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceFilter {
    pub session_id: Option<String>,
    pub trace_id: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub min_duration_ms: Option<u64>,
    pub max_duration_ms: Option<u64>,
    pub has_errors: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl TraceFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_date_range(mut self, from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        self.from_date = Some(from);
        self.to_date = Some(to);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone)]
pub struct StorageError {
    message: String,
}

impl StorageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StorageError {}

pub struct InMemoryTraceStorage {
    traces: Arc<RwLock<HashMap<String, TraceExport>>>,
}

impl InMemoryTraceStorage {
    pub fn new() -> Self {
        Self {
            traces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            traces: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
        }
    }
}

impl Default for InMemoryTraceStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceStorage for InMemoryTraceStorage {
    fn store(&self, trace: TraceExport) -> Result<(), StorageError> {
        let mut traces = self
            .traces
            .write()
            .map_err(|_| StorageError::new("Failed to acquire write lock"))?;
        traces.insert(trace.trace_id.clone(), trace);
        Ok(())
    }

    fn get(&self, trace_id: &str) -> Result<Option<TraceExport>, StorageError> {
        let traces = self
            .traces
            .read()
            .map_err(|_| StorageError::new("Failed to acquire read lock"))?;
        Ok(traces.get(trace_id).cloned())
    }

    fn list(&self, filter: &TraceFilter) -> Result<Vec<TraceSummary>, StorageError> {
        let traces = self
            .traces
            .read()
            .map_err(|_| StorageError::new("Failed to acquire read lock"))?;

        let mut summaries: Vec<TraceSummary> = traces
            .values()
            .filter(|trace| {
                if let Some(ref session_id) = filter.session_id {
                    if &trace.metadata.session_id != session_id {
                        return false;
                    }
                }

                if let Some(ref trace_id) = filter.trace_id {
                    if &trace.trace_id != trace_id {
                        return false;
                    }
                }

                if let Some(from) = filter.from_date {
                    if trace.spans.iter().map(|s| s.start_time).min() < Some(from) {
                        return false;
                    }
                }

                if let Some(to) = filter.to_date {
                    if trace.spans.iter().map(|s| s.start_time).max() > Some(to) {
                        return false;
                    }
                }

                if let Some(has_errors) = filter.has_errors {
                    let trace_has_errors = trace.spans.iter().any(|s| !s.errors.is_empty());
                    if trace_has_errors != has_errors {
                        return false;
                    }
                }

                true
            })
            .map(|trace| {
                TraceSummary::from_spans(trace.trace_id.clone(), &trace.spans, &trace.metadata)
            })
            .collect();

        summaries.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        if let Some(limit) = filter.limit {
            summaries.truncate(limit);
        }

        if let Some(offset) = filter.offset {
            summaries = summaries.into_iter().skip(offset).collect();
        }

        Ok(summaries)
    }

    fn delete(&self, trace_id: &str) -> Result<(), StorageError> {
        let mut traces = self
            .traces
            .write()
            .map_err(|_| StorageError::new("Failed to acquire write lock"))?;
        traces.remove(trace_id);
        Ok(())
    }

    fn clear(&self) -> Result<(), StorageError> {
        let mut traces = self
            .traces
            .write()
            .map_err(|_| StorageError::new("Failed to acquire write lock"))?;
        traces.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_storage() {
        let storage = InMemoryTraceStorage::new();
        let trace = TraceExport::new("test-trace".to_string(), vec![], TraceMetadata::default());

        storage.store(trace.clone()).unwrap();
        let retrieved = storage.get("test-trace").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().trace_id, "test-trace");
    }

    #[test]
    fn test_trace_filter() {
        let filter = TraceFilter::new().with_session("session-1").with_limit(10);

        assert_eq!(filter.session_id, Some("session-1".to_string()));
        assert_eq!(filter.limit, Some(10));
    }
}
