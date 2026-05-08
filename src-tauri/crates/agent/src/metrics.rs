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
            result.insert(name.clone(), MetricValue::new(name, *value, MetricType::Counter));
        }

        let gauges = self.gauges.read().await;
        for (name, value) in gauges.iter() {
            result.insert(name.clone(), MetricValue::new(name, *value, MetricType::Gauge));
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

    #[test]
    fn test_metric_type_display() {
        assert_eq!(MetricType::Counter.to_string(), "counter");
        assert_eq!(MetricType::Gauge.to_string(), "gauge");
        assert_eq!(MetricType::Histogram.to_string(), "histogram");
        assert_eq!(MetricType::Timing.to_string(), "timing");
    }

    #[test]
    fn test_metric_type_equality() {
        assert_eq!(MetricType::Counter, MetricType::Counter);
        assert_ne!(MetricType::Counter, MetricType::Gauge);
    }

    #[test]
    fn test_metric_value_new() {
        let mv = MetricValue::new("test_metric", 42.0, MetricType::Counter);
        assert_eq!(mv.name, "test_metric");
        assert_eq!(mv.value, 42.0);
        assert_eq!(mv.metric_type, MetricType::Counter);
        assert!(mv.labels.is_empty());
    }

    #[test]
    fn test_metric_value_with_labels() {
        let mut labels = HashMap::new();
        labels.insert("key1".to_string(), serde_json::json!("value1"));
        labels.insert("key2".to_string(), serde_json::json!(42));
        let mv = MetricValue::new("test", 1.0, MetricType::Gauge).with_labels(labels);
        assert_eq!(mv.labels.len(), 2);
        assert_eq!(mv.labels.get("key1").unwrap(), "value1");
    }

    #[test]
    fn test_metric_value_serialization() {
        let mv = MetricValue::new("test_metric", 99.5, MetricType::Timing);
        let json = serde_json::to_string(&mv).unwrap();
        let deserialized: MetricValue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test_metric");
        assert_eq!(deserialized.value, 99.5);
        assert_eq!(deserialized.metric_type, MetricType::Timing);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Trace.to_string(), "TRACE");
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Warn.to_string(), "WARN");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn test_log_level_equality() {
        assert_eq!(LogLevel::Info, LogLevel::Info);
        assert_ne!(LogLevel::Info, LogLevel::Error);
    }

    #[test]
    fn test_structured_log_entry_new() {
        let entry = StructuredLogEntry::new(LogLevel::Info, "test message", "test_source");
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.message, "test message");
        assert_eq!(entry.source, "test_source");
        assert!(entry.fields.is_empty());
        assert!(entry.correlation_id.is_none());
    }

    #[test]
    fn test_structured_log_entry_with_field() {
        let entry = StructuredLogEntry::new(LogLevel::Warn, "warning msg", "src")
            .with_field("key1", "value1")
            .with_field("key2", 42);
        assert_eq!(entry.fields.len(), 2);
        assert_eq!(entry.fields.get("key1").unwrap(), "value1");
        assert_eq!(entry.fields.get("key2").unwrap(), 42);
    }

    #[test]
    fn test_structured_log_entry_with_fields() {
        let mut fields = HashMap::new();
        fields.insert("a".to_string(), serde_json::json!(1));
        fields.insert("b".to_string(), serde_json::json!(2));
        let entry = StructuredLogEntry::new(LogLevel::Error, "err", "src")
            .with_field("c", 3)
            .with_fields(fields);
        assert_eq!(entry.fields.len(), 3);
    }

    #[test]
    fn test_structured_log_entry_with_correlation_id() {
        let entry =
            StructuredLogEntry::new(LogLevel::Debug, "msg", "src").with_correlation_id("corr-123");
        assert_eq!(entry.correlation_id, Some("corr-123".to_string()));
    }

    #[test]
    fn test_structured_log_entry_serialization() {
        let entry = StructuredLogEntry::new(LogLevel::Info, "msg", "src")
            .with_field("k", "v")
            .with_correlation_id("id-1");
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: StructuredLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.level, LogLevel::Info);
        assert_eq!(deserialized.message, "msg");
        assert_eq!(deserialized.correlation_id, Some("id-1".to_string()));
    }

    #[tokio::test]
    async fn test_counter_increment() {
        let collector = MetricsCollector::new();
        collector.increment_counter("test_counter", 1.0).await;
        collector.increment_counter("test_counter", 2.0).await;
        assert_eq!(collector.get_counter("test_counter").await, 3.0);
    }

    #[tokio::test]
    async fn test_counter_increment_multiple_names() {
        let collector = MetricsCollector::new();
        collector.increment_counter("counter_a", 5.0).await;
        collector.increment_counter("counter_b", 10.0).await;
        assert_eq!(collector.get_counter("counter_a").await, 5.0);
        assert_eq!(collector.get_counter("counter_b").await, 10.0);
    }

    #[tokio::test]
    async fn test_counter_nonexistent() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.get_counter("nonexistent").await, 0.0);
    }

    #[tokio::test]
    async fn test_counter_increment_by_zero() {
        let collector = MetricsCollector::new();
        collector.increment_counter("zero_counter", 0.0).await;
        assert_eq!(collector.get_counter("zero_counter").await, 0.0);
    }

    #[tokio::test]
    async fn test_counter_increment_negative() {
        let collector = MetricsCollector::new();
        collector.increment_counter("neg_counter", 10.0).await;
        collector.increment_counter("neg_counter", -3.0).await;
        assert_eq!(collector.get_counter("neg_counter").await, 7.0);
    }

    #[tokio::test]
    async fn test_gauge_set() {
        let collector = MetricsCollector::new();
        collector.set_gauge("test_gauge", 42.0).await;
        assert_eq!(collector.get_gauge("test_gauge").await, Some(42.0));
    }

    #[tokio::test]
    async fn test_gauge_overwrite() {
        let collector = MetricsCollector::new();
        collector.set_gauge("test_gauge", 42.0).await;
        collector.set_gauge("test_gauge", 100.0).await;
        assert_eq!(collector.get_gauge("test_gauge").await, Some(100.0));
    }

    #[tokio::test]
    async fn test_gauge_nonexistent() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.get_gauge("nonexistent").await, None);
    }

    #[tokio::test]
    async fn test_record_timing() {
        let collector = MetricsCollector::new();
        collector.record_timing("api_call", 100.0).await;
        collector.record_timing("api_call", 200.0).await;
        collector.record_timing("api_call", 300.0).await;
        let stats = collector.get_timing_stats("api_call").await;
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.count, 3.0);
        assert_eq!(stats.min, 100.0);
        assert_eq!(stats.max, 300.0);
        assert_eq!(stats.mean, 200.0);
    }

    #[tokio::test]
    async fn test_timing_stats_nonexistent() {
        let collector = MetricsCollector::new();
        assert!(collector.get_timing_stats("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_timing_stats_median_odd() {
        let collector = MetricsCollector::new();
        collector.record_timing("test", 100.0).await;
        collector.record_timing("test", 200.0).await;
        collector.record_timing("test", 300.0).await;
        let stats = collector.get_timing_stats("test").await.unwrap();
        assert_eq!(stats.median, 200.0);
    }

    #[tokio::test]
    async fn test_timing_stats_median_even() {
        let collector = MetricsCollector::new();
        collector.record_timing("test", 100.0).await;
        collector.record_timing("test", 200.0).await;
        let stats = collector.get_timing_stats("test").await.unwrap();
        assert_eq!(stats.median, 150.0);
    }

    #[tokio::test]
    async fn test_timing_stats_std_dev() {
        let collector = MetricsCollector::new();
        collector.record_timing("test", 100.0).await;
        collector.record_timing("test", 200.0).await;
        collector.record_timing("test", 300.0).await;
        let stats = collector.get_timing_stats("test").await.unwrap();
        assert!(stats.std_dev > 0.0);
    }

    #[tokio::test]
    async fn test_timing_stats_single_sample() {
        let collector = MetricsCollector::new();
        collector.record_timing("single", 42.0).await;
        let stats = collector.get_timing_stats("single").await.unwrap();
        assert_eq!(stats.count, 1.0);
        assert_eq!(stats.min, 42.0);
        assert_eq!(stats.max, 42.0);
        assert_eq!(stats.mean, 42.0);
        assert_eq!(stats.median, 42.0);
        assert_eq!(stats.std_dev, 0.0);
    }

    #[tokio::test]
    async fn test_timing_max_samples_limit() {
        let collector = MetricsCollector::new().with_max_timing_samples(5);
        for i in 0..10 {
            collector.record_timing("limited", i as f64).await;
        }
        let stats = collector.get_timing_stats("limited").await.unwrap();
        assert_eq!(stats.count, 5.0);
        assert_eq!(stats.min, 5.0);
        assert_eq!(stats.max, 9.0);
    }

    #[tokio::test]
    async fn test_get_all_metrics_empty() {
        let collector = MetricsCollector::new();
        let metrics = collector.get_all_metrics().await;
        assert!(metrics.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_metrics_with_data() {
        let collector = MetricsCollector::new();
        collector.increment_counter("cnt", 5.0).await;
        collector.set_gauge("gauge", 42.0).await;
        collector.record_timing("timing", 100.0).await;

        let metrics = collector.get_all_metrics().await;
        assert!(metrics.contains_key("cnt"));
        assert!(metrics.contains_key("gauge"));
        assert!(metrics.contains_key("timing"));

        assert_eq!(metrics.get("cnt").unwrap().metric_type, MetricType::Counter);
        assert_eq!(metrics.get("gauge").unwrap().metric_type, MetricType::Gauge);
        assert_eq!(metrics.get("timing").unwrap().metric_type, MetricType::Timing);
    }

    #[tokio::test]
    async fn test_get_all_metrics_timing_has_labels() {
        let collector = MetricsCollector::new();
        collector.record_timing("api", 100.0).await;
        collector.record_timing("api", 200.0).await;

        let metrics = collector.get_all_metrics().await;
        let timing_metric = metrics.get("api").unwrap();
        assert!(timing_metric.labels.contains_key("count"));
        assert!(timing_metric.labels.contains_key("min"));
        assert!(timing_metric.labels.contains_key("max"));
        assert!(timing_metric.labels.contains_key("median"));
    }

    #[tokio::test]
    async fn test_reset() {
        let collector = MetricsCollector::new();
        collector.increment_counter("test", 1.0).await;
        collector.set_gauge("test", 42.0).await;
        collector.record_timing("test", 100.0).await;

        collector.reset().await;

        assert_eq!(collector.get_counter("test").await, 0.0);
        assert!(collector.get_gauge("test").await.is_none());
        assert!(collector.get_timing_stats("test").await.is_none());
    }

    #[tokio::test]
    async fn test_reset_then_add() {
        let collector = MetricsCollector::new();
        collector.increment_counter("test", 10.0).await;
        collector.reset().await;
        collector.increment_counter("test", 5.0).await;
        assert_eq!(collector.get_counter("test").await, 5.0);
    }

    #[test]
    fn test_metrics_collector_default() {
        let collector = MetricsCollector::default();
        assert_eq!(collector.max_timing_samples, 1000);
    }

    #[test]
    fn test_metrics_collector_with_max_timing_samples() {
        let collector = MetricsCollector::new().with_max_timing_samples(50);
        assert_eq!(collector.max_timing_samples, 50);
    }

    #[test]
    fn test_timed_guard_new() {
        let guard = TimedGuard::new("test_metric");
        assert_eq!(guard.metric_name(), "test_metric");
        assert_eq!(guard.duration_ms(), 0.0);
    }

    #[test]
    fn test_timed_guard_finish() {
        let mut guard = TimedGuard::new("test_metric");
        std::thread::sleep(std::time::Duration::from_millis(10));
        guard.finish();
        assert!(guard.duration_ms() > 0.0);
    }

    #[test]
    fn test_timed_guard_drop_auto_finish() {
        let duration = {
            let guard = TimedGuard::new("auto_metric");
            std::thread::sleep(std::time::Duration::from_millis(10));
            guard.duration_ms()
        };
        assert_eq!(duration, 0.0);
    }

    #[test]
    fn test_timed_guard_manual_finish_prevents_double() {
        let mut guard = TimedGuard::new("test");
        guard.finish();
        let first = guard.duration_ms();
        std::thread::sleep(std::time::Duration::from_millis(10));
        guard.finish();
        let second = guard.duration_ms();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn test_record_timing_async() {
        let collector = MetricsCollector::new();
        record_timing_async(&collector, "async_timing", 123.0).await;
        let stats = collector.get_timing_stats("async_timing").await;
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().mean, 123.0);
    }

    #[test]
    fn test_log_with_fields_info() {
        let mut fields = HashMap::new();
        fields.insert("key".to_string(), serde_json::json!("value"));
        log_with_fields(LogLevel::Info, "test info message", "test_source", fields);
    }

    #[test]
    fn test_log_with_fields_error() {
        let mut fields = HashMap::new();
        fields.insert("error_code".to_string(), serde_json::json!(500));
        log_with_fields(LogLevel::Error, "test error message", "error_source", fields);
    }

    #[test]
    fn test_log_with_fields_warn() {
        let mut fields = HashMap::new();
        fields.insert("warning".to_string(), serde_json::json!("deprecated"));
        log_with_fields(LogLevel::Warn, "test warn message", "warn_source", fields);
    }

    #[test]
    fn test_log_with_fields_debug() {
        let fields = HashMap::new();
        log_with_fields(LogLevel::Debug, "test debug message", "debug_source", fields);
    }

    #[test]
    fn test_log_with_fields_trace() {
        let fields = HashMap::new();
        log_with_fields(LogLevel::Trace, "test trace message", "trace_source", fields);
    }

    #[test]
    fn test_timing_stats_serialization() {
        let stats = TimingStats {
            count: 5.0,
            min: 10.0,
            max: 100.0,
            mean: 50.0,
            median: 45.0,
            std_dev: 15.0,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: TimingStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.count, 5.0);
        assert_eq!(deserialized.min, 10.0);
        assert_eq!(deserialized.max, 100.0);
        assert_eq!(deserialized.mean, 50.0);
        assert_eq!(deserialized.median, 45.0);
        assert_eq!(deserialized.std_dev, 15.0);
    }

    #[tokio::test]
    async fn test_concurrent_counter_increment() {
        let collector = std::sync::Arc::new(MetricsCollector::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let c = collector.clone();
            handles.push(tokio::spawn(async move {
                c.increment_counter("concurrent", 1.0).await;
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(collector.get_counter("concurrent").await, 10.0);
    }
}
