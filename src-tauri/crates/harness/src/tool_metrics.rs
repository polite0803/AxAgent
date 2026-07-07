// SPDX-License-Identifier: AGPL-3.0-only
//! 工具指标收集器契约
use async_trait::async_trait;

#[derive(Debug,Clone)] pub struct ToolCallRecord { pub tool_name: String, pub success: bool, pub duration_ms: u64, pub error: Option<String>, pub timestamp: i64 }
#[derive(Debug,Clone)] pub struct ToolMetricsSnapshot { pub total_calls: u64, pub success_count: u64, pub error_count: u64, pub avg_duration_ms: f64, pub p99_duration_ms: f64, pub calls_by_tool: Vec<(String, u64)> }

#[async_trait]
pub trait ToolMetricsCollector: Send + Sync {
    async fn record_call(&self, record: ToolCallRecord);
    async fn snapshot(&self) -> ToolMetricsSnapshot;
    async fn tool_stats(&self, tool_name: &str) -> Result<ToolMetricsSnapshot, String>;
    async fn reset(&self);
}
#[derive(Default)] pub struct NoopToolMetricsCollector;
#[async_trait] impl ToolMetricsCollector for NoopToolMetricsCollector { async fn record_call(&self, _: ToolCallRecord) {} async fn snapshot(&self) -> ToolMetricsSnapshot { ToolMetricsSnapshot{total_calls:0,success_count:0,error_count:0,avg_duration_ms:0.0,p99_duration_ms:0.0,calls_by_tool:Vec::new()} } async fn tool_stats(&self, _: &str) -> Result<ToolMetricsSnapshot, String> { Ok(ToolMetricsSnapshot{total_calls:0,success_count:0,error_count:0,avg_duration_ms:0.0,p99_duration_ms:0.0,calls_by_tool:Vec::new()}) } async fn reset(&self) {} }
