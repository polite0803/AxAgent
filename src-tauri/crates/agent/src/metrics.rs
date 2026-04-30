use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Timing,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Counter => write!(f, "counter"),
            MetricType::Gauge => write!(f, "gauge"),
            MetricType::Histogram => write!(f, "histogram"),
            MetricType::Timing => write!(f, "timing"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
    pub metric_type: MetricType,
    pub labels: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl MetricValue {
    pub fn new(name: impl Into<String>, value: f64, metric_type: MetricType) -> Self {
        Self {
            name: name.into(),
            value,
            metric_type,
            labels: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn with_labels(mut self, labels: HashMap<String, serde_json::Value>) -> Self {
        self.labels = labels;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

impl StructuredLogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            timestamp: Utc::now(),
            source: source.into(),
            fields: HashMap::new(),
            correlation_id: None,
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(&value) {
            self.fields.insert(key.into(), v);
        }
        self
    }

    pub fn with_fields(mut self, fields: HashMap<String, serde_json::Value>) -> Self {
        self.fields.extend(fields);
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

pub struct MetricsCollector {
    counters: RwLock<HashMap<String, f64>>,
    gauges: RwLock<HashMap<String, f64>>,
    timings: RwLock<HashMap<String, Vec<f64>>>,
    max_timing_samples: usize,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            timings: RwLock::new(HashMap::new()),
            max_timing_samples: 1000,
        }
    }

    pub fn with_max_timing_samples(mut self, max_samples: usize) -> Self {
        self.max_timing_samples = max_samples;
        self
    }

    pub async fn increment_counter(&self, name: &str, value: f64) {
        let mut counters = self.counters.write().await;
        *counters.entry(name.to_string()).or_insert(0.0) += value;
    }

    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), value);
    }

    pub async fn record_timing(&self, name: &str, duration_ms: f64) {
        let mut timings = self.timings.write().await;
        let samples = timings.entry(name.to_string()).or_insert_with(Vec::new);
        samples.push(duration_ms);
        if samples.len() > self.max_timing_samples {
            samples.remove(0);
        }
    }

    pub async fn get_counter(&self, name: &str) -> f64 {
        let counters = self.counters.read().await;
        counters.get(name).copied().unwrap_or(0.0)
    }

    pub async fn get_gauge(&self, name: &str) -> Option<f64> {
        let gauges = self.gauges.read().await;
        gauges.get(name).copied()
    }

    pub async fn get_timing_stats(&self, name: &str) -> Option<TimingStats> {
        let timings = self.timings.read().await;
        let samples = timings.get(name)?;

        if samples.is_empty() {
            return None;
        }

        let sum: f64 = samples.iter().sum();
        let count = samples.len() as f64;
        let mean = sum / count;

        let mut sorted = samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let min = sorted.first().copied().unwrap_or(0.0);
        let max = sorted.last().copied().unwrap_or(0.0);

        let median = if sorted.len().is_multiple_of(2) {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };

        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / count;
        let std_dev = variance.sqrt();

        Some(TimingStats {
            count,
            min,
            max,
            mean,
            median,
            std_dev,
        })
    }

    pub async fn get_all_metrics(&self) -> HashMap<String, MetricValue> {
        let mut result = HashMap::new();

        let counters = self.counters.read().await;
        for (name, value) in counters.iter() {
            result.insert(
                name.clone(),
                MetricValue::new(name, *value, MetricType::Counter),
            );
        }

        let gauges = self.gauges.read().await;
        for (name, value) in gauges.iter() {
            result.insert(
                name.clone(),
                MetricValue::new(name, *value, MetricType::Gauge),
            );
        }

        let timings = self.timings.read().await;
        for (name, samples) in timings.iter() {
            if let Some(stats) = self.calculate_timing_stats_sync(samples) {
                result.insert(
                    name.clone(),
                    MetricValue::new(name, stats.mean, MetricType::Timing).with_labels(
                        vec![
                            ("count".to_string(), serde_json::json!(stats.count)),
                            ("min".to_string(), serde_json::json!(stats.min)),
                            ("max".to_string(), serde_json::json!(stats.max)),
                            ("median".to_string(), serde_json::json!(stats.median)),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                );
            }
        }

        result
    }

    fn calculate_timing_stats_sync(&self, samples: &[f64]) -> Option<TimingStats> {
        if samples.is_empty() {
            return None;
        }

        let sum: f64 = samples.iter().sum();
        let count = samples.len() as f64;
        let mean = sum / count;

        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let min = sorted.first().copied().unwrap_or(0.0);
        let max = sorted.last().copied().unwrap_or(0.0);

        let median = if sorted.len().is_multiple_of(2) {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };

        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / count;
        let std_dev = variance.sqrt();

        Some(TimingStats {
            count,
            min,
            max,
            mean,
            median,
            std_dev,
        })
    }

    pub async fn reset(&self) {
        let mut counters = self.counters.write().await;
        let mut gauges = self.gauges.write().await;
        let mut timings = self.timings.write().await;

        counters.clear();
        gauges.clear();
        timings.clear();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingStats {
    pub count: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
}

pub struct TimedGuard {
    start: Instant,
    metric_name: String,
    duration_ms: f64,
}

impl TimedGuard {
    pub fn new(metric_name: &str) -> Self {
        Self {
            start: Instant::now(),
            metric_name: metric_name.to_string(),
            duration_ms: 0.0,
        }
    }

    pub fn finish(&mut self) {
        self.duration_ms = self.start.elapsed().as_millis() as f64;
    }

    pub fn duration_ms(&self) -> f64 {
        self.duration_ms
    }

    pub fn metric_name(&self) -> &str {
        &self.metric_name
    }
}

impl Drop for TimedGuard {
    fn drop(&mut self) {
        if self.duration_ms == 0.0 {
            self.finish();
        }
    }
}

pub async fn record_timing_async(
    collector: &MetricsCollector,
    metric_name: &str,
    duration_ms: f64,
) {
    collector.record_timing(metric_name, duration_ms).await;
}

pub fn log_with_fields(
    level: LogLevel,
    message: &str,
    source: &str,
    fields: HashMap<String, serde_json::Value>,
) {
    let entry = StructuredLogEntry::new(level, message, source).with_fields(fields);

    match level {
        LogLevel::Error => tracing::error!(
            ?entry,
            source = %entry.source,
            "{}",
            entry.message
        ),
        LogLevel::Warn => tracing::warn!(
            ?entry,
            source = %entry.source,
            "{}",
            entry.message
        ),
        LogLevel::Info => tracing::info!(
            ?entry,
            source = %entry.source,
            "{}",
            entry.message
        ),
        LogLevel::Debug => tracing::debug!(
            ?entry,
            source = %entry.source,
            "{}",
            entry.message
        ),
        LogLevel::Trace => tracing::trace!(
            ?entry,
            source = %entry.source,
            "{}",
            entry.message
        ),
    }
}

#[macro_export]
macro_rules! log_info {
    ($source:expr, $($key:expr => $value:expr),*) => {{
        use std::collections::HashMap;
        let mut fields = HashMap::new();
        $(fields.insert($key.to_string(), serde_json::json!($value));)*
        $crate::metrics::log_with_fields(
            $crate::metrics::LogLevel::Info,
            &format_args!("").to_string(),
            $source,
            fields,
        );
    }};
}

#[macro_export]
macro_rules! log_error {
    ($source:expr, $msg:expr, $($key:expr => $value:expr),*) => {{
        use std::collections::HashMap;
        let mut fields = HashMap::new();
        $(fields.insert($key.to_string(), serde_json::json!($value));)*
        $crate::metrics::log_with_fields(
            $crate::metrics::LogLevel::Error,
            $msg,
            $source,
            fields,
        );
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_counter_increment() {
        let collector = MetricsCollector::new();

        collector.increment_counter("test_counter", 1.0).await;
        collector.increment_counter("test_counter", 2.0).await;

        assert_eq!(collector.get_counter("test_counter").await, 3.0);
    }

    #[tokio::test]
    async fn test_gauge() {
        let collector = MetricsCollector::new();

        collector.set_gauge("test_gauge", 42.0).await;
        assert_eq!(collector.get_gauge("test_gauge").await, Some(42.0));

        collector.set_gauge("test_gauge", 100.0).await;
        assert_eq!(collector.get_gauge("test_gauge").await, Some(100.0));
    }

    #[tokio::test]
    async fn test_timing_stats() {
        let collector = MetricsCollector::new();

        collector.record_timing("test_timing", 100.0).await;
        collector.record_timing("test_timing", 200.0).await;
        collector.record_timing("test_timing", 300.0).await;

        let stats = collector.get_timing_stats("test_timing").await;
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.count, 3.0);
        assert_eq!(stats.min, 100.0);
        assert_eq!(stats.max, 300.0);
        assert_eq!(stats.mean, 200.0);
    }

    #[tokio::test]
    async fn test_reset() {
        let collector = MetricsCollector::new();

        collector.increment_counter("test", 1.0).await;
        collector.set_gauge("test", 42.0).await;

        collector.reset().await;

        assert_eq!(collector.get_counter("test").await, 0.0);
        assert!(collector.get_gauge("test").await.is_none());
    }
}
