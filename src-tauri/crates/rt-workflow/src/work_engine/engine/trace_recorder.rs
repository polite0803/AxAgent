// SPDX-License-Identifier: AGPL-3.0-only

//! 执行轨迹记录器
//!
//! 本模块实现运行时执行轨迹的记录和查询功能，
//! 为时间旅行调试提供数据支撑。
//!
//! # 优化点
//! - 合并锁粒度：将 trace 和 node_index 合并到一个 RwLock，避免两次锁操作
//! - 批量操作：支持批量写入减少锁获取次数
//! - 内存管理：添加最大节点数限制和自动清理机制
//! - 增量更新：对于单个节点的更新采用最小锁持有时间

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::execution_trace::{
    DecisionLog, DecisionLogType, ExecutionTrace, NodeErrorDetail, NodeErrorType,
    NodeExecutionTrace, SchemaDiffReport, TraceStatistics,
};
use axagent_harness::schema::SchemaValidationResult;
use parking_lot::RwLock;

/// 最大节点数（防止内存泄漏）
const MAX_NODES: usize = 10000;

/// 内部状态（合并锁）
struct InnerState {
    /// 执行轨迹
    trace: ExecutionTrace,
    /// 节点 ID → 索引快速查找
    node_index: HashMap<String, usize>,
}

/// 执行轨迹记录器
///
/// 线程安全的执行轨迹收集器，用于在工作流执行过程中
/// 持续记录每个节点的执行信息。
pub struct TraceRecorder {
    /// 内部状态（合并锁，一次获取即可访问所有数据）
    state: Arc<RwLock<InnerState>>,
    /// 最大节点数限制
    max_nodes: usize,
}

impl TraceRecorder {
    /// 创建新的记录器
    pub fn new(execution_id: impl Into<String>, workflow_id: impl Into<String>) -> Self {
        Self {
            state: Arc::new(RwLock::new(InnerState {
                trace: ExecutionTrace::new(execution_id, workflow_id),
                node_index: HashMap::new(),
            })),
            max_nodes: MAX_NODES,
        }
    }

    /// 创建带自定义最大节点数的记录器
    pub fn with_max_nodes(
        execution_id: impl Into<String>,
        workflow_id: impl Into<String>,
        max_nodes: usize,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(InnerState {
                trace: ExecutionTrace::new(execution_id, workflow_id),
                node_index: HashMap::new(),
            })),
            max_nodes,
        }
    }

    /// 开始记录节点执行
    pub fn start_node(
        &self,
        node_id: impl Into<String>,
        node_type: impl Into<String>,
        node_name: Option<String>,
        input: Option<serde_json::Value>,
    ) {
        let node_id_str: String = node_id.into();
        let node_type_str: String = node_type.into();

        let mut state = self.state.write();

        // 检查内存限制
        if state.trace.node_traces.len() >= self.max_nodes {
            tracing::warn!(
                "TraceRecorder 已达到最大节点数限制 ({}), 跳过记录节点: {}",
                self.max_nodes,
                node_id_str
            );
            return;
        }

        let mut node = NodeExecutionTrace::new(node_id_str.clone(), node_type_str);
        node.node_name = node_name;
        node.input = input;
        node.mark_started();

        let idx = state.trace.node_traces.len();
        state.trace.add_node_trace(node);
        state.node_index.insert(node_id_str, idx);
    }

    /// 记录节点完成
    pub fn complete_node(&self, node_id: &str, output: serde_json::Value) {
        let mut state = self.state.write();
        if let Some(idx) = state.node_index.get(node_id).copied() {
            if let Some(node) = state.trace.node_traces.get_mut(idx) {
                node.mark_completed(output);
            }
        }
    }

    /// 记录节点失败
    pub fn fail_node(&self, node_id: &str, error_type: NodeErrorType, message: impl Into<String>) {
        let mut state = self.state.write();
        if let Some(idx) = state.node_index.get(node_id).copied() {
            if let Some(node) = state.trace.node_traces.get_mut(idx) {
                let error = NodeErrorDetail::new(error_type, message);
                node.mark_failed(error);
            }
        }
    }

    /// 记录输入校验结果
    pub fn record_input_validation(&self, node_id: &str, result: SchemaValidationResult) {
        let mut state = self.state.write();
        if let Some(idx) = state.node_index.get(node_id).copied() {
            if let Some(node) = state.trace.node_traces.get_mut(idx) {
                node.input_validation = Some(result);
            }
        }
    }

    /// 记录输出校验结果
    pub fn record_output_validation(&self, node_id: &str, result: SchemaValidationResult) {
        let mut state = self.state.write();
        if let Some(idx) = state.node_index.get(node_id).copied() {
            if let Some(node) = state.trace.node_traces.get_mut(idx) {
                node.output_validation = Some(result);
            }
        }
    }

    /// 追加工具调用记录
    pub fn add_tool_call(
        &self,
        node_id: &str,
        tool_name: impl Into<String>,
        arguments: Option<serde_json::Value>,
    ) {
        let mut state = self.state.write();
        if let Some(idx) = state.node_index.get(node_id).copied() {
            if let Some(node) = state.trace.node_traces.get_mut(idx) {
                let call =
                    axagent_harness::execution_trace::ToolCallTrace::new(tool_name, arguments);
                node.add_tool_call(call);
            }
        }
    }

    /// 记录 Token 使用量
    pub fn record_token_usage(
        &self,
        node_id: &str,
        input_tokens: u32,
        output_tokens: u32,
        model: Option<String>,
    ) {
        let mut state = self.state.write();
        if let Some(idx) = state.node_index.get(node_id).copied() {
            if let Some(node) = state.trace.node_traces.get_mut(idx) {
                node.token_usage = Some(axagent_harness::execution_trace::TokenUsageTrace {
                    input_tokens,
                    output_tokens,
                    total_tokens: input_tokens + output_tokens,
                    model,
                    estimated_cost_usd: None,
                });
            }
        }
    }

    /// 批量追加工具调用记录（减少锁获取次数）
    pub fn batch_add_tool_calls(&self, calls: Vec<(&str, String, Option<serde_json::Value>)>) {
        let mut state = self.state.write();
        for (node_id, tool_name, arguments) in calls {
            if let Some(idx) = state.node_index.get(node_id).copied() {
                if let Some(node) = state.trace.node_traces.get_mut(idx) {
                    let call =
                        axagent_harness::execution_trace::ToolCallTrace::new(tool_name, arguments);
                    node.add_tool_call(call);
                }
            }
        }
    }

    /// 标记轨迹完成
    pub fn complete_trace(&self, output: Option<serde_json::Value>) {
        let mut state = self.state.write();
        state.trace.complete(output);
    }

    /// 标记轨迹失败
    pub fn fail_trace(&self, error_summary: axagent_harness::execution_trace::TraceErrorSummary) {
        let mut state = self.state.write();
        state.trace.fail(error_summary);
    }

    /// 获取当前轨迹快照
    pub fn get_trace(&self) -> ExecutionTrace {
        let state = self.state.read();
        state.trace.clone()
    }

    /// 获取节点轨迹
    pub fn get_node_trace(&self, node_id: &str) -> Option<NodeExecutionTrace> {
        let state = self.state.read();
        state.node_index.get(node_id).and_then(|idx| state.trace.node_traces.get(*idx).cloned())
    }

    /// 计算统计信息
    pub fn compute_statistics(&self) -> TraceStatistics {
        let state = self.state.read();
        TraceStatistics::from_trace(&state.trace)
    }

    /// 获取所有 Schema 错误的差异报告
    pub fn get_schema_diff_reports(&self) -> Vec<SchemaDiffReport> {
        let state = self.state.read();
        let mut reports = Vec::new();

        for node in &state.trace.node_traces {
            if let Some(SchemaValidationResult::Invalid { errors }) = &node.output_validation {
                for error in errors {
                    reports.push(SchemaDiffReport::from_validation_error(error));
                }
            }
        }

        reports
    }

    /// 获取出错的节点列表
    pub fn get_failed_nodes(&self) -> Vec<NodeExecutionTrace> {
        let state = self.state.read();
        state.trace.find_error_nodes().into_iter().cloned().collect()
    }

    /// 获取执行时间线（用于前端时间旅行面板）
    pub fn get_timeline(&self) -> Vec<axagent_harness::execution_trace::TimelinePosition> {
        let state = self.state.read();
        axagent_harness::execution_trace::TimelinePosition::from_trace(&state.trace)
    }

    /// 获取节点数量
    pub fn node_count(&self) -> usize {
        let state = self.state.read();
        state.trace.node_traces.len()
    }

    /// 检查是否包含指定节点
    pub fn contains_node(&self, node_id: &str) -> bool {
        let state = self.state.read();
        state.node_index.contains_key(node_id)
    }

    /// 记录决策日志
    pub fn record_decision(&self, log: DecisionLog) {
        let mut state = self.state.write();
        state.trace.add_decision_log(log);
    }

    /// 批量记录决策日志
    pub fn batch_record_decisions(&self, logs: Vec<DecisionLog>) {
        let mut state = self.state.write();
        for log in logs {
            state.trace.add_decision_log(log);
        }
    }

    /// 获取所有决策日志
    pub fn get_decision_logs(&self) -> Vec<DecisionLog> {
        let state = self.state.read();
        state.trace.decision_logs.clone()
    }

    /// 获取指定类型的决策日志
    pub fn get_decision_logs_by_type(&self, log_type: &DecisionLogType) -> Vec<DecisionLog> {
        let state = self.state.read();
        state.trace.get_decision_logs_by_type(log_type).into_iter().cloned().collect()
    }

    /// 获取节点相关的决策日志
    pub fn get_decision_logs_for_node(&self, node_id: &str) -> Vec<DecisionLog> {
        let state = self.state.read();
        state.trace.get_decision_logs_for_node(node_id).into_iter().cloned().collect()
    }

    /// 获取决策日志数量
    pub fn decision_log_count(&self) -> usize {
        let state = self.state.read();
        state.trace.decision_log_count()
    }
}

impl Clone for TraceRecorder {
    fn clone(&self) -> Self {
        Self { state: self.state.clone(), max_nodes: self.max_nodes }
    }
}

// ── 辅助函数 ──

/// 创建默认的错误摘要
pub fn create_error_summary(
    trace: &ExecutionTrace,
) -> axagent_harness::execution_trace::TraceErrorSummary {
    let error_nodes = trace.find_error_nodes();
    let schema_errors = trace.find_schema_errors();

    let first_error_at_ms = error_nodes.iter().filter_map(|n| n.completed_at_ms).min();

    axagent_harness::execution_trace::TraceErrorSummary {
        error_node_count: error_nodes.len() as u32,
        error_node_ids: error_nodes.iter().map(|n| n.node_id.clone()).collect(),
        schema_error_count: schema_errors.len() as u32,
        tool_error_count: 0,
        first_error_at_ms,
        root_cause_analysis: None,
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::workflow_types::NodeStatus;

    #[test]
    fn test_recorder_creation() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");
        let trace = recorder.get_trace();
        assert_eq!(trace.execution_id, "exec-1");
        assert_eq!(trace.workflow_id, "wf-1");
        assert_eq!(recorder.node_count(), 0);
    }

    #[test]
    fn test_start_and_complete_node() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", Some("Agent 1".to_string()), None);

        let trace = recorder.get_trace();
        assert_eq!(trace.node_traces.len(), 1);
        assert_eq!(trace.node_traces[0].node_id, "node-1");
        assert_eq!(trace.node_traces[0].status, NodeStatus::Running);

        recorder.complete_node("node-1", serde_json::json!({"result": "ok"}));

        let trace = recorder.get_trace();
        assert_eq!(trace.node_traces[0].status, NodeStatus::Completed);
        assert!(trace.node_traces[0].duration_ms.is_some());
    }

    #[test]
    fn test_fail_node() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", None, None);
        recorder.fail_node("node-1", NodeErrorType::SchemaValidation, "输出不符合 Schema");

        let trace = recorder.get_trace();
        assert_eq!(trace.node_traces[0].status, NodeStatus::Failed);
        assert!(trace.node_traces[0].error.is_some());
    }

    #[test]
    fn test_record_validation_results() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "tool", None, None);

        let validation = SchemaValidationResult::Valid;
        recorder.record_input_validation("node-1", validation);

        let invalid =
            SchemaValidationResult::invalid(vec![axagent_harness::schema::SchemaValidationError {
                path: "/test".to_string(),
                message: "错误".to_string(),
                expected_type: None,
                actual_value: None,
            }]);
        recorder.record_output_validation("node-1", invalid);

        let trace = recorder.get_trace();
        assert!(trace.node_traces[0].input_validation.as_ref().unwrap().is_valid());
        assert!(trace.node_traces[0].output_validation.as_ref().unwrap().is_invalid());
    }

    #[test]
    fn test_add_tool_call() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", None, None);
        recorder.add_tool_call("node-1", "search", Some(serde_json::json!({"q": "test"})));

        let trace = recorder.get_trace();
        assert_eq!(trace.node_traces[0].tool_calls.len(), 1);
        assert_eq!(trace.node_traces[0].tool_calls[0].tool_name, "search");
    }

    #[test]
    fn test_batch_add_tool_calls() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", None, None);

        let calls = vec![
            ("node-1", "search".to_string(), Some(serde_json::json!({"q": "test"}))),
            ("node-1", "calculator".to_string(), Some(serde_json::json!({"x": 42}))),
        ];
        recorder.batch_add_tool_calls(calls);

        let trace = recorder.get_trace();
        assert_eq!(trace.node_traces[0].tool_calls.len(), 2);
    }

    #[test]
    fn test_record_token_usage() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "llm", None, None);
        recorder.record_token_usage("node-1", 100, 50, Some("gpt-4".to_string()));

        let trace = recorder.get_trace();
        let usage = trace.node_traces[0].token_usage.as_ref().unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn test_complete_trace() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", None, None);
        recorder.complete_node("node-1", serde_json::json!({"ok": true}));
        recorder.complete_trace(Some(serde_json::json!({"final": "result"})));

        let trace = recorder.get_trace();
        assert_eq!(trace.status, axagent_harness::execution_trace::TraceStatus::Completed);
        assert!(trace.completed_at_ms.is_some());
    }

    #[test]
    fn test_compute_statistics() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        for i in 1..=3 {
            let node_id = format!("node-{i}");
            recorder.start_node(&node_id, "agent", None, None);
            recorder.complete_node(&node_id, serde_json::json!({}));
        }

        let stats = recorder.compute_statistics();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.success_nodes, 3);
        assert_eq!(stats.failed_nodes, 0);
    }

    #[test]
    fn test_get_schema_diff_reports() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", None, None);

        let invalid =
            SchemaValidationResult::invalid(vec![axagent_harness::schema::SchemaValidationError {
                path: "/data".to_string(),
                message: "类型错误".to_string(),
                expected_type: Some("string".to_string()),
                actual_value: Some(serde_json::json!(42)),
            }]);
        recorder.record_output_validation("node-1", invalid);
        recorder.complete_node("node-1", serde_json::json!(42));

        let reports = recorder.get_schema_diff_reports();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].expected_path, "/data");
    }

    #[test]
    fn test_get_failed_nodes() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", None, None);
        recorder.fail_node("node-1", NodeErrorType::Timeout, "超时");

        let failed = recorder.get_failed_nodes();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].node_id, "node-1");
    }

    #[test]
    fn test_get_timeline() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        for i in 1..=3 {
            let node_id = format!("node-{i}");
            recorder.start_node(&node_id, "agent", None, None);
            recorder.complete_node(&node_id, serde_json::json!({}));
        }

        let timeline = recorder.get_timeline();
        assert_eq!(timeline.len(), 3);
        assert_eq!(timeline[0].node_index, 0);
        assert_eq!(timeline[2].node_index, 2);
    }

    #[test]
    fn test_get_node_trace() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", Some("节点1".to_string()), None);

        let node_trace = recorder.get_node_trace("node-1");
        assert!(node_trace.is_some());
        assert_eq!(node_trace.unwrap().node_name, Some("节点1".to_string()));

        let non_existent = recorder.get_node_trace("nonexistent");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_contains_node() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", None, None);

        assert!(recorder.contains_node("node-1"));
        assert!(!recorder.contains_node("nonexistent"));
    }

    #[test]
    fn test_max_nodes_limit() {
        let recorder = TraceRecorder::with_max_nodes("exec-1", "wf-1", 3);

        // 添加 3 个节点
        for i in 1..=3 {
            let node_id = format!("node-{i}");
            recorder.start_node(&node_id, "agent", None, None);
        }
        assert_eq!(recorder.node_count(), 3);

        // 第 4 个节点应被跳过
        recorder.start_node("node-4", "agent", None, None);
        assert_eq!(recorder.node_count(), 3);
    }

    #[test]
    fn test_create_error_summary() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");

        recorder.start_node("node-1", "agent", None, None);
        recorder.fail_node("node-1", NodeErrorType::ToolExecution, "工具失败");

        let trace = recorder.get_trace();
        let summary = create_error_summary(&trace);

        assert_eq!(summary.error_node_count, 1);
        assert_eq!(summary.error_node_ids, vec!["node-1".to_string()]);
    }

    #[test]
    fn test_clone() {
        let recorder = TraceRecorder::new("exec-1", "wf-1");
        recorder.start_node("node-1", "agent", None, None);

        let cloned = recorder.clone();
        assert_eq!(cloned.node_count(), 1);
        assert!(cloned.contains_node("node-1"));
    }
}
