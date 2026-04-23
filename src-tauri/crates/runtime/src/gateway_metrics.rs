use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tokio::time::interval;

#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub enable_statsd: bool,
    pub statsd_host: Option<String>,
    pub statsd_port: Option<u16>,
    pub collection_interval: Duration,
    pub retention_period: Duration,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_statsd: false,
            statsd_host: None,
            statsd_port: None,
            collection_interval: Duration::from_secs(10),
            retention_period: Duration::from_secs(3600),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

#[derive(Debug, Clone)]
pub struct MetricValue {
    pub name: String,
    pub kind: MetricKind,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: i64,
}

impl MetricValue {
    pub fn new(name: String, kind: MetricKind, value: f64) -> Self {
        Self {
            name,
            kind,
            value,
            labels: HashMap::new(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }
}

pub struct MetricsCollector {
    counters: Arc<RwLock<HashMap<String, Counter>>>,
    gauges: Arc<RwLock<HashMap<String, Gauge>>>,
    histograms: Arc<RwLock<HashMap<String, Histogram>>>,
    summaries: Arc<RwLock<HashMap<String, Summary>>>,
    config: MetricsConfig,
}

struct Counter {
    value: u64,
    labels: HashMap<String, String>,
}

struct Gauge {
    value: f64,
    labels: HashMap<String, String>,
}

struct Histogram {
    values: Vec<f64>,
    buckets: HashMap<usize, u64>,
    labels: HashMap<String, String>,
}

struct Summary {
    values: Vec<f64>,
    labels: HashMap<String, String>,
    quantiles: Vec<f64>,
}

impl MetricsCollector {
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            summaries: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn increment_counter(&self, name: &str, value: u64, labels: Option<HashMap<String, String>>) {
        let mut counters = self.counters.write().await;
        let counter = counters.entry(name.to_string()).or_insert_with(|| Counter {
            value: 0,
            labels: labels.clone().unwrap_or_default(),
        });
        counter.value += value;
    }

    pub async fn set_gauge(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let mut gauges = self.gauges.write().await;
        let gauge = gauges.entry(name.to_string()).or_insert_with(|| Gauge {
            value: 0.0,
            labels: labels.clone().unwrap_or_default(),
        });
        gauge.value = value;
    }

    pub async fn record_histogram(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let mut histograms = self.histograms.write().await;
        let hist = histograms.entry(name.to_string()).or_insert_with(|| Histogram {
            values: Vec::new(),
            buckets: HashMap::new(),
            labels: labels.clone().unwrap_or_default(),
        });
        hist.values.push(value);
    }

    pub async fn record_summary(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let mut summaries = self.summaries.write().await;
        let summary = summaries.entry(name.to_string()).or_insert_with(|| Summary {
            values: Vec::new(),
            labels: labels.clone().unwrap_or_default(),
            quantiles: vec![0.5, 0.9, 0.99],
        });
        summary.values.push(value);
    }

    pub async fn get_metrics(&self) -> Vec<MetricValue> {
        let mut metrics = Vec::new();

        let counters = self.counters.read().await;
        for (name, counter) in counters.iter() {
            metrics.push(MetricValue {
                name: name.clone(),
                kind: MetricKind::Counter,
                value: counter.value as f64,
                labels: counter.labels.clone(),
                timestamp: chrono::Utc::now().timestamp(),
            });
        }

        let gauges = self.gauges.read().await;
        for (name, gauge) in gauges.iter() {
            metrics.push(MetricValue {
                name: name.clone(),
                kind: MetricKind::Gauge,
                value: gauge.value,
                labels: gauge.labels.clone(),
                timestamp: chrono::Utc::now().timestamp(),
            });
        }

        let histograms = self.histograms.read().await;
        for (name, hist) in histograms.iter() {
            if !hist.values.is_empty() {
                let sum: f64 = hist.values.iter().sum();
                let count = hist.values.len() as f64;
                metrics.push(MetricValue {
                    name: format!("{}_sum", name),
                    kind: MetricKind::Histogram,
                    value: sum,
                    labels: hist.labels.clone(),
                    timestamp: chrono::Utc::now().timestamp(),
                });
                metrics.push(MetricValue {
                    name: format!("{}_count", name),
                    kind: MetricKind::Histogram,
                    value: count,
                    labels: hist.labels.clone(),
                    timestamp: chrono::Utc::now().timestamp(),
                });
            }
        }

        metrics
    }

    pub async fn reset(&self) {
        let mut counters = self.counters.write().await;
        for counter in counters.values_mut() {
            counter.value = 0;
        }
        let mut gauges = self.gauges.write().await;
        gauges.clear();
        let mut histograms = self.histograms.write().await;
        histograms.clear();
        let mut summaries = self.summaries.write().await;
        summaries.clear();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new(MetricsConfig::default())
    }
}

pub struct GatewayMetrics {
    collector: Arc<MetricsCollector>,
}

impl GatewayMetrics {
    pub fn new() -> Self {
        Self {
            collector: Arc::new(MetricsCollector::new(MetricsConfig::default())),
        }
    }

    pub fn with_config(config: MetricsConfig) -> Self {
        Self {
            collector: Arc::new(MetricsCollector::new(config)),
        }
    }

    pub async fn record_connection(&self, agent_id: &str, success: bool, latency_ms: u64) {
        let labels = {
            let mut l = HashMap::new();
            l.insert("agent_id".to_string(), agent_id.to_string());
            l.insert("status".to_string(), if success { "success" } else { "failure" }.to_string());
            l
        };

        self.collector.increment_counter("gateway_connections_total", 1, Some(labels.clone())).await;
        self.collector.record_histogram("gateway_connection_latency_ms", latency_ms as f64, Some(labels)).await;
    }

    pub async fn record_message(&self, from: &str, to: &str, size_bytes: u64, latency_ms: u64) {
        let mut labels = HashMap::new();
        labels.insert("from".to_string(), from.to_string());
        labels.insert("to".to_string(), to.to_string());

        self.collector.increment_counter("gateway_messages_total", 1, Some(labels.clone())).await;
        self.collector.record_histogram("gateway_message_size_bytes", size_bytes as f64, Some(labels.clone())).await;
        self.collector.record_histogram("gateway_message_latency_ms", latency_ms as f64, Some(labels)).await;
    }

    pub async fn record_request(&self, endpoint: &str, method: &str, status: u16, latency_ms: u64) {
        let mut labels = HashMap::new();
        labels.insert("endpoint".to_string(), endpoint.to_string());
        labels.insert("method".to_string(), method.to_string());
        labels.insert("status".to_string(), status.to_string());

        self.collector.increment_counter("gateway_requests_total", 1, Some(labels.clone())).await;
        self.collector.record_histogram("gateway_request_latency_ms", latency_ms as f64, Some(labels)).await;
    }

    pub async fn record_error(&self, error_type: &str, agent_id: Option<&str>) {
        let mut labels = HashMap::new();
        labels.insert("error_type".to_string(), error_type.to_string());
        if let Some(id) = agent_id {
            labels.insert("agent_id".to_string(), id.to_string());
        }
        self.collector.increment_counter("gateway_errors_total", 1, Some(labels)).await;
    }

    pub async fn update_active_connections(&self, count: usize) {
        self.collector.set_gauge("gateway_active_connections", count as f64, None).await;
    }

    pub async fn update_queue_depth(&self, agent_id: &str, depth: usize) {
        let mut labels = HashMap::new();
        labels.insert("agent_id".to_string(), agent_id.to_string());
        self.collector.set_gauge("gateway_queue_depth", depth as f64, Some(labels)).await;
    }

    pub async fn get_all_metrics(&self) -> Vec<MetricValue> {
        self.collector.get_metrics().await
    }
}

impl Default for GatewayMetrics {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StructuredLogger {
    service_name: String,
    environment: String,
}

impl StructuredLogger {
    pub fn new(service_name: String, environment: String) -> Self {
        Self {
            service_name,
            environment,
        }
    }

    pub fn log(&self, level: &str, message: &str, context: HashMap<String, String>) {
        let log_entry = LogEntry {
            timestamp: chrono::Utc::now(),
            level: level.to_string(),
            service: self.service_name.clone(),
            environment: self.environment.clone(),
            message: message.to_string(),
            context,
        };

        let json = serde_json::to_string(&log_entry).unwrap_or_else(|_| "{}".to_string());
        match level {
            "ERROR" | "WARN" => tracing::error!("{}", json),
            "INFO" => tracing::info!("{}", json),
            "DEBUG" => tracing::debug!("{}", json),
            _ => tracing::trace!("{}", json),
        }
    }

    pub fn error(&self, message: &str, context: HashMap<String, String>) {
        self.log("ERROR", message, context);
    }

    pub fn warn(&self, message: &str, context: HashMap<String, String>) {
        self.log("WARN", message, context);
    }

    pub fn info(&self, message: &str, context: HashMap<String, String>) {
        self.log("INFO", message, context);
    }

    pub fn debug(&self, message: &str, context: HashMap<String, String>) {
        self.log("DEBUG", message, context);
    }
}

#[derive(Debug, serde::Serialize)]
struct LogEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    level: String,
    service: String,
    environment: String,
    message: String,
    context: HashMap<String, String>,
}

pub struct TracingMiddleware {
    metrics: Arc<GatewayMetrics>,
}

impl TracingMiddleware {
    pub fn new(metrics: Arc<GatewayMetrics>) -> Self {
        Self { metrics }
    }

    pub async fn trace_request<R>(&self, request_id: &str, endpoint: &str, f: impl FnOnce() -> R) -> R
    where
        R: std::future::Future<Output = ()>,
    {
        let start = Instant::now();
        let result = f().await;
        let elapsed = start.elapsed().as_millis() as u64;

        self.metrics.record_request(endpoint, "POST", 200, elapsed).await;

        tracing::info!(
            request_id = %request_id,
            endpoint = %endpoint,
            latency_ms = %elapsed,
            "Request completed"
        );

        result
    }
}