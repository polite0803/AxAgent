use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostMetrics {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_cost_usd: f64,
    pub model: String,
}

impl CostMetrics {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    pub fn add_tokens(&mut self, input: u64, output: u64) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.total_tokens = self.input_tokens
            + self.output_tokens
            + self.cache_creation_tokens
            + self.cache_read_tokens;
        self.total_cost_usd = Self::calculate_cost(&self.model, self.total_tokens);
    }

    pub fn add_cache_tokens(&mut self, creation: u64, read: u64) {
        self.cache_creation_tokens += creation;
        self.cache_read_tokens += read;
        self.total_tokens = self.input_tokens
            + self.output_tokens
            + self.cache_creation_tokens
            + self.cache_read_tokens;
        self.total_cost_usd = Self::calculate_cost(&self.model, self.total_tokens);
    }

    fn calculate_cost(model: &str, tokens: u64) -> f64 {
        let per_million = match model {
            m if m.contains("claude-opus") => 15.0,
            m if m.contains("claude-sonnet") => 3.0,
            m if m.contains("claude-haiku") => 0.25,
            _ => 3.0,
        };
        (tokens as f64 / 1_000_000.0) * per_million
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceMetrics {
    pub total_duration_ms: u64,
    pub ttft_ms: Option<u64>,
    pub cost: CostMetrics,
    pub spans_count: usize,
    pub errors_count: usize,
}

impl TraceMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.total_duration_ms = duration_ms;
        self
    }

    pub fn with_cost(mut self, cost: CostMetrics) -> Self {
        self.cost = cost;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanMetrics {
    pub span_id: String,
    pub name: String,
    pub span_type: String,
    pub duration_ms: u64,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub error_count: usize,
}

impl SpanMetrics {
    pub fn from_span(span: &crate::span::Span) -> Self {
        Self {
            span_id: span.id.clone(),
            name: span.name.clone(),
            span_type: format!("{:?}", span.span_type),
            duration_ms: span.duration_ms.unwrap_or(0),
            start_time: span.start_time,
            end_time: span.end_time,
            status: format!("{:?}", span.status),
            attributes: span.attributes.clone(),
            error_count: span.errors.len(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub total_traces: usize,
    pub total_spans: usize,
    pub total_errors: usize,
    pub avg_duration_ms: f64,
    pub avg_tokens: f64,
    pub avg_cost_usd: f64,
    pub traces_by_type: HashMap<String, usize>,
    pub errors_by_type: HashMap<String, usize>,
}

impl AggregatedMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trace_metrics(&mut self, metrics: &TraceMetrics) {
        self.total_spans += metrics.spans_count;
        self.total_errors += metrics.errors_count;
    }

    pub fn calculate_averages(&mut self, trace_count: usize) {
        if trace_count > 0 {
            self.avg_duration_ms /= trace_count as f64;
            self.avg_tokens /= trace_count as f64;
            self.avg_cost_usd /= trace_count as f64;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl Usage {
    pub fn to_metrics(&self, model: &str) -> CostMetrics {
        let mut metrics = CostMetrics::new(model);
        metrics.add_tokens(self.input_tokens, self.output_tokens);
        metrics.add_cache_tokens(
            self.cache_creation_input_tokens,
            self.cache_read_input_tokens,
        );
        metrics
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetrics {
    pub app_id: String,
    pub date: DateTime<Utc>,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_cost_usd: f64,
    pub latency_ms: u64,
}

impl AppMetrics {
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            date: Utc::now(),
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_tokens: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_cost_usd: 0.0,
            latency_ms: 0,
        }
    }

    pub fn record_request(&mut self, success: bool) {
        self.total_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
    }

    pub fn record_tokens(&mut self, prompt: u64, completion: u64, cost: f64) {
        self.prompt_tokens += prompt;
        self.completion_tokens += completion;
        self.total_tokens = self.prompt_tokens + self.completion_tokens;
        self.total_cost_usd += cost;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    pub workflow_id: String,
    pub execution_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: u64,
}

impl WorkflowMetrics {
    pub fn new(workflow_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            execution_count: 0,
            success_count: 0,
            failure_count: 0,
            total_duration_ms: 0,
            avg_duration_ms: 0,
        }
    }

    pub fn record_execution(&mut self, success: bool, duration_ms: u64) {
        self.execution_count += 1;
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        self.total_duration_ms += duration_ms;
        if self.execution_count > 0 {
            self.avg_duration_ms = self.total_duration_ms / self.execution_count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let mut cost = CostMetrics::new("claude-sonnet");
        cost.add_tokens(1000, 500);
        assert_eq!(cost.input_tokens, 1000);
        assert_eq!(cost.output_tokens, 500);
        assert_eq!(cost.total_tokens, 1500);
    }

    #[test]
    fn test_usage_to_metrics() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 200,
            cache_read_input_tokens: 300,
        };
        let metrics = usage.to_metrics("claude-sonnet");
        assert_eq!(metrics.input_tokens, 100);
        assert_eq!(metrics.cache_creation_tokens, 200);
        assert_eq!(metrics.cache_read_tokens, 300);
    }

    #[test]
    fn test_app_metrics() {
        let mut metrics = AppMetrics::new("test-app");
        metrics.record_request(true);
        metrics.record_request(true);
        metrics.record_request(false);
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
    }

    #[test]
    fn test_workflow_metrics() {
        let mut metrics = WorkflowMetrics::new("workflow-1");
        metrics.record_execution(true, 100);
        metrics.record_execution(true, 200);
        metrics.record_execution(false, 150);
        assert_eq!(metrics.execution_count, 3);
        assert_eq!(metrics.success_count, 2);
        assert_eq!(metrics.failure_count, 1);
        assert_eq!(metrics.total_duration_ms, 450);
        assert_eq!(metrics.avg_duration_ms, 150);
    }
}
