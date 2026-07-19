// SPDX-License-Identifier: AGPL-3.0-only

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
        Self { model: model.into(), ..Default::default() }
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
        metrics.add_cache_tokens(self.cache_creation_input_tokens, self.cache_read_input_tokens);
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
            self.avg_duration_ms =
                self.total_duration_ms.checked_div(self.execution_count).unwrap_or(0);
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

// ── 全局工作流 metrics(Prometheus 风格原子计数器) ──
//
// 提供 WorkEngine / NodeDispatcher 在关键节点调用的全局 metrics 函数。
// 使用 atomic + OnceLock 实现,无锁,线程安全。
// 后续可被 Prometheus exporter / OpenTelemetry collector 抓取。

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// 全局工作流 metrics 注册表(单例)。
pub struct WorkflowMetricsRegistry {
    pub total_executions: AtomicU64,
    pub successful_executions: AtomicU64,
    pub failed_executions: AtomicU64,
    pub active_workflows: AtomicU64,
    pub total_duration_ms: AtomicU64,
    /// 节点级 dispatch 次数(含成功/失败)
    pub node_dispatch_total: AtomicU64,
    pub node_dispatch_failed: AtomicU64,
}

static WORKFLOW_METRICS: OnceLock<WorkflowMetricsRegistry> = OnceLock::new();

fn registry() -> &'static WorkflowMetricsRegistry {
    WORKFLOW_METRICS.get_or_init(|| WorkflowMetricsRegistry {
        total_executions: AtomicU64::new(0),
        successful_executions: AtomicU64::new(0),
        failed_executions: AtomicU64::new(0),
        active_workflows: AtomicU64::new(0),
        total_duration_ms: AtomicU64::new(0),
        node_dispatch_total: AtomicU64::new(0),
        node_dispatch_failed: AtomicU64::new(0),
    })
}

/// 记录一次工作流执行结果(成功/失败)。
/// 由 WorkEngine.run_workflow 在结束时调用。
pub fn inc_workflow_execution(success: bool) {
    let r = registry();
    r.total_executions.fetch_add(1, Ordering::Relaxed);
    if success {
        r.successful_executions.fetch_add(1, Ordering::Relaxed);
    } else {
        r.failed_executions.fetch_add(1, Ordering::Relaxed);
    }
}

/// 记录工作流执行耗时(毫秒)。
pub fn observe_workflow_duration_ms(duration_ms: u64) {
    registry().total_duration_ms.fetch_add(duration_ms, Ordering::Relaxed);
}

/// 活跃工作流计数 +1(由 run_workflow 开始时调用)。
pub fn inc_active_workflows() {
    registry().active_workflows.fetch_add(1, Ordering::Relaxed);
}

/// 活跃工作流计数 -1(由 run_workflow 结束时调用,无论成功/失败)。
pub fn dec_active_workflows() {
    registry().active_workflows.fetch_sub(1, Ordering::Relaxed);
}

/// 直接设置活跃工作流计数(用于校准场景)。
pub fn set_active_workflows(count: u64) {
    registry().active_workflows.store(count, Ordering::Relaxed);
}

/// 记录一次节点 dispatch(由 NodeDispatcher.dispatch 调用)。
pub fn inc_node_dispatch() {
    registry().node_dispatch_total.fetch_add(1, Ordering::Relaxed);
}

/// 记录一次节点 dispatch 失败。
pub fn inc_node_dispatch_failed() {
    registry().node_dispatch_failed.fetch_add(1, Ordering::Relaxed);
}

/// 获取当前活跃工作流数。
pub fn get_active_workflows() -> u64 {
    registry().active_workflows.load(Ordering::Relaxed)
}

/// 全局工作流 metrics 快照(用于导出 / Prometheus scrape)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetricsSnapshot {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub active_workflows: u64,
    pub total_duration_ms: u64,
    pub node_dispatch_total: u64,
    pub node_dispatch_failed: u64,
}

pub fn get_workflow_metrics_snapshot() -> WorkflowMetricsSnapshot {
    let r = registry();
    WorkflowMetricsSnapshot {
        total_executions: r.total_executions.load(Ordering::Relaxed),
        successful_executions: r.successful_executions.load(Ordering::Relaxed),
        failed_executions: r.failed_executions.load(Ordering::Relaxed),
        active_workflows: r.active_workflows.load(Ordering::Relaxed),
        total_duration_ms: r.total_duration_ms.load(Ordering::Relaxed),
        node_dispatch_total: r.node_dispatch_total.load(Ordering::Relaxed),
        node_dispatch_failed: r.node_dispatch_failed.load(Ordering::Relaxed),
    }
}

/// RAII guard:在作用域开始时 inc_active_workflows,
/// 在 drop 时 dec_active_workflows + observe_workflow_duration_ms。
///
/// `inc_workflow_execution(success)` 需要调用方在 return 前显式调用
/// (因为 Drop 无法访问执行结果)。
///
/// 用法:
/// ```ignore
/// let _guard = WorkflowMetricsGuard::new();
/// // ... 执行工作流 ...
/// if success {
///     inc_workflow_execution(true);
/// } else {
///     inc_workflow_execution(false);
/// }
/// ```
pub struct WorkflowMetricsGuard {
    start: std::time::Instant,
    finished: bool,
}

impl WorkflowMetricsGuard {
    /// 创建 guard 并 inc_active_workflows。
    pub fn new() -> Self {
        inc_active_workflows();
        Self { start: std::time::Instant::now(), finished: false }
    }

    /// 显式完成(避免 drop 时重复 dec)。
    /// 调用方在 return 前调用此方法 + inc_workflow_execution(success)。
    pub fn finish(&mut self, success: bool) {
        if !self.finished {
            self.finished = true;
            dec_active_workflows();
            observe_workflow_duration_ms(self.start.elapsed().as_millis() as u64);
            inc_workflow_execution(success);
        }
    }
}

impl Drop for WorkflowMetricsGuard {
    fn drop(&mut self) {
        if !self.finished {
            // 调用方未显式 finish,默认按失败处理
            dec_active_workflows();
            observe_workflow_duration_ms(self.start.elapsed().as_millis() as u64);
            // 不在此处 inc_workflow_execution(避免重复计数)
        }
    }
}

impl Default for WorkflowMetricsGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod workflow_metrics_tests {
    use super::*;

    #[test]
    fn test_inc_workflow_execution() {
        // 注意:全局 metrics,测试间会累积。这里只验证增量。
        let before = get_workflow_metrics_snapshot();
        inc_workflow_execution(true);
        inc_workflow_execution(false);
        let after = get_workflow_metrics_snapshot();
        assert_eq!(after.total_executions - before.total_executions, 2);
        assert_eq!(after.successful_executions - before.successful_executions, 1);
        assert_eq!(after.failed_executions - before.failed_executions, 1);
    }

    #[test]
    fn test_active_workflows_counter() {
        let before = get_active_workflows();
        inc_active_workflows();
        inc_active_workflows();
        assert_eq!(get_active_workflows(), before + 2);
        dec_active_workflows();
        assert_eq!(get_active_workflows(), before + 1);
        // 清理
        dec_active_workflows();
    }

    #[test]
    fn test_observe_duration() {
        let before = get_workflow_metrics_snapshot();
        observe_workflow_duration_ms(500);
        let after = get_workflow_metrics_snapshot();
        assert_eq!(after.total_duration_ms - before.total_duration_ms, 500);
    }
}
