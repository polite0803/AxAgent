//! Session 级别追踪契约。
//!
//! 提供轻量的事件追踪接口，用于记录 LLM 会话中的关键事件
//!（turn_started、tool_execution_started、turn_completed 等）。
//!
//! 实现方（`axagent-telemetry`）将事件持久化到 JSONL 文件或发送到 OTLP 端点。

use serde_json::{Map, Value};
use std::fmt;

/// Session 级别的追踪契约
///
/// - `record`：记录一个命名事件，附加 key-value 属性
pub trait SessionTracer: fmt::Debug + Send + Sync {
    /// 记录一个追踪事件
    fn record(&self, name: &str, attributes: Map<String, Value>);
}

/// 空实现 SessionTracer — 丢弃所有事件。
///
/// 在未配置 telemetry 时作为默认 fallback 使用。
#[derive(Debug)]
pub struct NoopSessionTracer;

impl SessionTracer for NoopSessionTracer {
    fn record(&self, _name: &str, _attributes: Map<String, Value>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_does_not_panic() {
        let tracer = NoopSessionTracer;
        tracer.record("test_event", Map::new());
        tracer.record("another_event", {
            let mut m = Map::new();
            m.insert("key".to_string(), Value::String("val".to_string()));
            m
        });
    }
}
