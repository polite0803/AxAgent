// SPDX-License-Identifier: AGPL-3.0-only
//! Structured metrics pipeline: Prometheus-backed counters/histograms
//! for LLM calls (count, latency, success rate) with JSON export.
//!
//! Initialization:
//!   `RuntimeMetrics::init()`             — register all metrics once (idempotent via `lazy_static`)
//! Recording:
//!   `RuntimeMetrics::record_llm_call(success, duration_ms, model, tokens)`
//! Export:
//!   `RuntimeMetrics::export_json()`      — returns a `serde_json::Value` JSON snapshot

#![allow(clippy::disallowed_types)]

use lazy_static::lazy_static;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};
use serde_json::{Map, Value};
use std::sync::Mutex;

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();

    /// LLM call counter — labelled by provider, model, status.
    static ref LLM_CALL_COUNTER: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "llm_calls_total",
            "Total number of LLM API calls.",
        ),
        &["provider", "model", "status"]
    )
    .expect("llm_calls_total metric");

    /// LLM call latency histogram (ms).
    static ref LLM_LATENCY_MS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "llm_latency_ms",
            "LLM API call latency in milliseconds.",
        )
        .buckets(vec![
            100.0, 250.0, 500.0, 750.0, 1000.0, 2000.0, 5000.0,
            10_000.0, 30_000.0, 60_000.0,
        ]),
        &["provider", "model"]
    )
    .expect("llm_latency_ms metric");

    /// LLM token consumption — labelled by provider, model, direction.
    static ref LLM_TOKENS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "llm_tokens_total",
            "Total tokens consumed by LLM calls.",
        ),
        &["provider", "model", "direction"]
    )
    .expect("llm_tokens_total metric");

    /// Locked export buffer for Prometheus text format → JSON convert.
    static ref EXPORT_BUF: Mutex<Vec<u8>> = Mutex::new(Vec::new());
}

/// Returns `true` once on first call (idempotent).
fn ensure_registered() -> bool {
    static REGISTERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    if REGISTERED.load(std::sync::atomic::Ordering::Relaxed) {
        return false;
    }

    // Each metric only registers successfully once.
    let _ = REGISTRY.register(Box::new(LLM_CALL_COUNTER.clone()));
    let _ = REGISTRY.register(Box::new(LLM_LATENCY_MS.clone()));
    let _ = REGISTRY.register(Box::new(LLM_TOKENS.clone()));

    REGISTERED.store(true, std::sync::atomic::Ordering::Relaxed);
    true
}

pub struct RuntimeMetrics;

impl RuntimeMetrics {
    /// Register all metrics with the Prometheus registry.  Idempotent —
    /// subsequent calls are cheap no-ops.
    pub fn init() {
        ensure_registered();
    }

    /// Record a single LLM API call.
    pub fn record_llm_call(
        provider: &str,
        model: &str,
        success: bool,
        duration_ms: f64,
        input_tokens: u64,
        output_tokens: u64,
    ) {
        // Ensure metrics are registered (idempotent).
        ensure_registered();

        let status = if success { "success" } else { "failure" };

        LLM_CALL_COUNTER.with_label_values(&[provider, model, status]).inc();
        LLM_LATENCY_MS.with_label_values(&[provider, model]).observe(duration_ms);
        LLM_TOKENS.with_label_values(&[provider, model, "input"]).inc_by(input_tokens);
        LLM_TOKENS.with_label_values(&[provider, model, "output"]).inc_by(output_tokens);
    }

    /// Export all metrics as a JSON object, suitable for a `/metrics` endpoint
    /// or writing to `metrics.json`.
    pub fn export_json() -> Value {
        let mut buf = EXPORT_BUF.lock().unwrap_or_else(|e| e.into_inner());
        buf.clear();

        let encoder = TextEncoder::new();
        let metric_families = REGISTRY.gather();
        if encoder.encode(&metric_families, &mut *buf).is_err() {
            return Value::Object(Map::new());
        }

        let text = String::from_utf8_lossy(&buf);
        parse_prometheus_text_to_json(&text)
    }
}

/// Minimal Prometheus text-format → JSON converter.
fn parse_prometheus_text_to_json(text: &str) -> Value {
    let mut map = Map::new();

    for line in text.lines() {
        let line = line.trim();
        // Skip comments and blanks
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse "metric_name{labels} value [timestamp]"
        let split: Vec<&str> = line.splitn(2, '{').collect();
        if split.len() < 2 {
            // Untyped or unlabelled metric
            if let Some((name, val)) = split[0].split_once(' ')
                && let Ok(v) = val.parse::<f64>()
            {
                map.insert(name.to_string(), Value::from(v));
            }
            continue;
        }

        let name = split[0].to_string();
        let rest = split[1]; // labels} value ..."
        let brace_end = rest.find('}').unwrap_or(rest.len());
        let labels_str = &rest[..brace_end];
        let value_str = rest[brace_end + 1..].trim();

        // Parse value
        let value: f64 =
            value_str.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0.0);

        let entry = map.entry(name.clone()).or_insert_with(|| Value::Array(Vec::new()));

        if let Value::Array(arr) = entry {
            let mut metric = Map::new();
            metric.insert("value".to_string(), Value::from(value));

            // Parse labels into sub-object
            let mut labels = Map::new();
            for pair in labels_str.split(',') {
                if let Some((k, v)) = pair.split_once('=') {
                    let v = v.trim_matches('"');
                    labels.insert(k.trim().to_string(), Value::String(v.to_string()));
                }
            }
            if !labels.is_empty() {
                metric.insert("labels".to_string(), Value::Object(labels));
            }

            arr.push(Value::Object(metric));
        }
    }

    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        RuntimeMetrics::init();
        RuntimeMetrics::init(); // Must not panic
    }

    #[test]
    fn record_and_export_llm_calls() {
        RuntimeMetrics::init();

        RuntimeMetrics::record_llm_call("openai", "gpt-4o", true, 1234.0, 500, 200);
        RuntimeMetrics::record_llm_call("openai", "gpt-4o", false, 5600.0, 300, 0);
        RuntimeMetrics::record_llm_call("anthropic", "claude-sonnet", true, 890.0, 400, 150);

        let json = RuntimeMetrics::export_json();
        let obj = json.as_object().expect("export should be an object");

        // llm_calls_total should have entries
        assert!(obj.contains_key("llm_calls_total"));
        // Histogram metrics emit suffixed keys (bucket/sum/count), not the base name
        assert!(
            obj.contains_key("llm_latency_ms_sum") || obj.contains_key("llm_latency_ms_count"),
            "expected llm_latency_ms histogram data (sum or count) in export, got keys: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert!(obj.contains_key("llm_tokens_total"));

        // Check that our recorded values appear
        let calls = &obj["llm_calls_total"];
        let calls_str = serde_json::to_string(calls).expect("测试：JSON序列化应成功");
        assert!(calls_str.contains("gpt-4o"));
        assert!(calls_str.contains("claude-sonnet"));
    }

    #[test]
    fn export_empty_registry_returns_object() {
        // Create a fresh RuntimeMetrics without any registrations
        let json = RuntimeMetrics::export_json();
        assert!(json.is_object());
    }
}
