// SPDX-License-Identifier: AGPL-3.0-only

//! 观测层：执行轨迹系统（时间旅行调试）
//!
//! 本模块定义工作流执行过程的完整轨迹记录系统，
//! 支持"视频回放式"调试——定位到任意节点，查看输入/输出/中间变量。
//!
//! # 核心理念
//! - **完整记录**：每个节点的输入、输出、工具调用、Schema 校验结果都被记录
//! - **时间旅行**：支持按时间戳回溯到任意执行点
//! - **差异对比**：高亮显示"预期 Schema"与"实际输出"的差异
//! - **根因定位**：快速锁定出错节点，缩小调试范围
//!
//! # 架构定位
//! - 定义在 harness 层（foundation），纯数据 DTO
//! - 运行时记录逻辑由 rt-workflow 实现
//! - 前端消费这些数据渲染时间旅行面板

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::business_state_machine::FsmTransitionRecord;
use crate::schema::SchemaValidationResult;
use crate::workflow_types::NodeStatus;

// ── 执行轨迹主结构 ──

/// 工作流执行轨迹
///
/// 这是时间旅行调试的核心数据结构，
/// 包含完整的执行上下文、节点轨迹和状态机转移记录。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExecutionTrace {
    /// 轨迹 ID
    pub id: String,
    /// 工作流 ID
    pub workflow_id: String,
    /// 执行实例 ID
    pub execution_id: String,
    /// 开始时间戳（毫秒）
    pub started_at_ms: u64,
    /// 结束时间戳（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<u64>,
    /// 节点执行轨迹列表
    #[serde(default)]
    pub node_traces: Vec<NodeExecutionTrace>,
    /// 业务状态机转移记录（如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fsm_transitions: Option<Vec<FsmTransitionRecord>>,
    /// 整体执行状态
    pub status: TraceStatus,
    /// 全局输入
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_input: Option<serde_json::Value>,
    /// 全局输出
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_output: Option<serde_json::Value>,
    /// 错误摘要（如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_summary: Option<TraceErrorSummary>,
    /// 决策日志列表（时间旅行核心）
    #[serde(default)]
    pub decision_logs: Vec<DecisionLog>,
}

impl ExecutionTrace {
    pub fn new(execution_id: impl Into<String>, workflow_id: impl Into<String>) -> Self {
        let now_ms = current_timestamp_ms();
        Self {
            id: format!("trace_{}", uuid::Uuid::new_v4()),
            workflow_id: workflow_id.into(),
            execution_id: execution_id.into(),
            started_at_ms: now_ms,
            completed_at_ms: None,
            node_traces: Vec::new(),
            fsm_transitions: None,
            status: TraceStatus::InProgress,
            global_input: None,
            global_output: None,
            error_summary: None,
            decision_logs: Vec::new(),
        }
    }

    /// 追加决策日志
    pub fn add_decision_log(&mut self, log: DecisionLog) {
        self.decision_logs.push(log);
    }

    /// 获取指定类型的决策日志
    pub fn get_decision_logs_by_type(&self, log_type: &DecisionLogType) -> Vec<&DecisionLog> {
        self.decision_logs.iter().filter(|l| &l.decision_type == log_type).collect()
    }

    /// 获取节点相关的决策日志
    pub fn get_decision_logs_for_node(&self, node_id: &str) -> Vec<&DecisionLog> {
        self.decision_logs.iter().filter(|l| l.node_id.as_deref() == Some(node_id)).collect()
    }

    /// 获取决策日志数量
    pub fn decision_log_count(&self) -> usize {
        self.decision_logs.len()
    }

    /// 追加节点轨迹
    pub fn add_node_trace(&mut self, trace: NodeExecutionTrace) {
        self.node_traces.push(trace);
    }

    /// 完成轨迹
    pub fn complete(&mut self, output: Option<serde_json::Value>) {
        let now_ms = current_timestamp_ms();
        self.completed_at_ms = Some(now_ms);
        self.status = TraceStatus::Completed;
        self.global_output = output;
    }

    /// 标记为失败
    pub fn fail(&mut self, error: TraceErrorSummary) {
        let now_ms = current_timestamp_ms();
        self.completed_at_ms = Some(now_ms);
        self.status = TraceStatus::Failed;
        self.error_summary = Some(error);
    }

    /// 获取总执行时间（毫秒）
    pub fn total_duration_ms(&self) -> Option<u64> {
        self.completed_at_ms.map(|end| end.saturating_sub(self.started_at_ms))
    }

    /// 查找出错的节点
    pub fn find_error_nodes(&self) -> Vec<&NodeExecutionTrace> {
        self.node_traces.iter().filter(|n| matches!(n.status, NodeStatus::Failed)).collect()
    }

    /// 查找 Schema 校验失败的节点
    pub fn find_schema_errors(&self) -> Vec<&NodeExecutionTrace> {
        self.node_traces
            .iter()
            .filter(|n| matches!(n.output_validation, Some(SchemaValidationResult::Invalid { .. })))
            .collect()
    }

    /// 获取指定时间范围内的节点轨迹
    pub fn nodes_in_range(&self, start_ms: u64, end_ms: u64) -> Vec<&NodeExecutionTrace> {
        self.node_traces
            .iter()
            .filter(|n| {
                let node_start = n.started_at_ms.unwrap_or(0);
                let node_end = n.completed_at_ms.unwrap_or(u64::MAX);
                node_start >= start_ms && node_end <= end_ms
            })
            .collect()
    }

    /// 计算成功率
    pub fn success_rate(&self) -> f64 {
        if self.node_traces.is_empty() {
            return 1.0;
        }
        let completed =
            self.node_traces.iter().filter(|n| matches!(n.status, NodeStatus::Completed)).count();
        completed as f64 / self.node_traces.len() as f64
    }
}

// ── 节点执行轨迹 ──

/// 单个节点的执行轨迹（时间旅行的核心单元）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NodeExecutionTrace {
    /// 节点 ID
    pub node_id: String,
    /// 节点类型（如 "agent", "tool", "llm" 等）
    pub node_type: String,
    /// 节点名称
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,
    /// 执行状态
    pub status: NodeStatus,
    /// 开始时间戳（毫秒）
    pub started_at_ms: Option<u64>,
    /// 完成时间戳（毫秒）
    pub completed_at_ms: Option<u64>,
    /// 执行耗时（毫秒）
    pub duration_ms: Option<u64>,
    /// 节点输入（原始 JSON）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    /// 节点输出（原始 JSON）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// 输入 Schema 校验结果
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_validation: Option<SchemaValidationResult>,
    /// 输出 Schema 校验结果
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_validation: Option<SchemaValidationResult>,
    /// 节点内工具调用记录
    #[serde(default)]
    pub tool_calls: Vec<ToolCallTrace>,
    /// 重试次数
    #[serde(default)]
    pub retry_count: u32,
    /// 错误信息（如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<NodeErrorDetail>,
    /// 内存使用量（字节，如有记录）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_usage_bytes: Option<u64>,
    /// Token 使用量（如有记录）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsageTrace>,
    /// 业务状态转换（如果关联 FSM）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_state_transition: Option<FsmTransitionRecord>,
}

impl NodeExecutionTrace {
    pub fn new(node_id: impl Into<String>, node_type: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            node_type: node_type.into(),
            node_name: None,
            status: NodeStatus::Pending,
            started_at_ms: None,
            completed_at_ms: None,
            duration_ms: None,
            input: None,
            output: None,
            input_validation: None,
            output_validation: None,
            tool_calls: Vec::new(),
            retry_count: 0,
            error: None,
            memory_usage_bytes: None,
            token_usage: None,
            business_state_transition: None,
        }
    }

    /// 标记开始执行
    pub fn mark_started(&mut self) {
        let now_ms = current_timestamp_ms();
        self.status = NodeStatus::Running;
        self.started_at_ms = Some(now_ms);
    }

    /// 标记完成
    pub fn mark_completed(&mut self, output: serde_json::Value) {
        let now_ms = current_timestamp_ms();
        self.status = NodeStatus::Completed;
        self.completed_at_ms = Some(now_ms);
        self.output = Some(output);
        self.update_duration();
    }

    /// 标记失败
    pub fn mark_failed(&mut self, error: NodeErrorDetail) {
        let now_ms = current_timestamp_ms();
        self.status = NodeStatus::Failed;
        self.completed_at_ms = Some(now_ms);
        self.error = Some(error);
        self.update_duration();
    }

    /// 追加工具调用
    pub fn add_tool_call(&mut self, call: ToolCallTrace) {
        self.tool_calls.push(call);
    }

    /// 更新耗时
    fn update_duration(&mut self) {
        if let (Some(start), Some(end)) = (self.started_at_ms, self.completed_at_ms) {
            self.duration_ms = Some(end.saturating_sub(start));
        }
    }

    /// 检查是否有 Schema 错误
    pub fn has_schema_error(&self) -> bool {
        matches!(self.output_validation, Some(SchemaValidationResult::Invalid { .. }))
    }

    /// 检查是否有工具调用错误
    pub fn has_tool_error(&self) -> bool {
        self.tool_calls.iter().any(|t| matches!(t.status, ToolCallStatus::Failed))
    }

    /// 获取耗时（毫秒）
    pub fn elapsed_ms(&self) -> u64 {
        self.duration_ms.unwrap_or_else(|| {
            let end = self.completed_at_ms.unwrap_or_else(current_timestamp_ms);
            let start = self.started_at_ms.unwrap_or(end);
            end.saturating_sub(start)
        })
    }
}

// ── 工具调用轨迹 ──

/// 工具调用轨迹
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ToolCallTrace {
    /// 工具名称
    pub tool_name: String,
    /// 调用参数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    /// 调用结果
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// 调用状态
    pub status: ToolCallStatus,
    /// 开始时间戳（毫秒）
    pub started_at_ms: u64,
    /// 完成时间戳（毫秒）
    pub completed_at_ms: u64,
    /// 耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息（如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 是否被重试
    #[serde(default)]
    pub retried: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl ToolCallTrace {
    pub fn new(tool_name: impl Into<String>, arguments: Option<serde_json::Value>) -> Self {
        let now_ms = current_timestamp_ms();
        Self {
            tool_name: tool_name.into(),
            arguments,
            result: None,
            status: ToolCallStatus::Pending,
            started_at_ms: now_ms,
            completed_at_ms: now_ms,
            duration_ms: 0,
            error: None,
            retried: false,
        }
    }

    pub fn mark_completed(&mut self, result: serde_json::Value) {
        let now_ms = current_timestamp_ms();
        self.status = ToolCallStatus::Completed;
        self.result = Some(result);
        self.completed_at_ms = now_ms;
        self.duration_ms = now_ms.saturating_sub(self.started_at_ms);
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        let now_ms = current_timestamp_ms();
        self.status = ToolCallStatus::Failed;
        self.error = Some(error.into());
        self.completed_at_ms = now_ms;
        self.duration_ms = now_ms.saturating_sub(self.started_at_ms);
    }
}

// ── 错误详情 ──

/// 节点错误详情
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NodeErrorDetail {
    /// 错误类型
    pub error_type: NodeErrorType,
    /// 错误消息
    pub message: String,
    /// 错误堆栈（如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
    /// 原始错误码（如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// 是否可重试
    #[serde(default)]
    pub retryable: bool,
    /// 建议操作
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NodeErrorType {
    SchemaValidation,
    ToolExecution,
    LlmCall,
    Timeout,
    NetworkError,
    PermissionDenied,
    BusinessLogic,
    Unknown,
}

impl NodeErrorDetail {
    pub fn new(error_type: NodeErrorType, message: impl Into<String>) -> Self {
        Self {
            error_type,
            message: message.into(),
            stack_trace: None,
            error_code: None,
            retryable: false,
            suggestion: None,
        }
    }

    pub fn with_stack_trace(mut self, stack: impl Into<String>) -> Self {
        self.stack_trace = Some(stack.into());
        self
    }

    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

// ── Token 使用量 ──

/// Token 使用量追踪
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct TokenUsageTrace {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
}

// ── 错误摘要 ──

/// 执行轨迹错误摘要
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TraceErrorSummary {
    /// 错误节点数量
    pub error_node_count: u32,
    /// 错误节点 ID 列表
    #[serde(default)]
    pub error_node_ids: Vec<String>,
    /// Schema 错误数量
    #[serde(default)]
    pub schema_error_count: u32,
    /// 工具错误数量
    #[serde(default)]
    pub tool_error_count: u32,
    /// 最早错误时间戳
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_error_at_ms: Option<u64>,
    /// 根因分析（AI 生成，如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_cause_analysis: Option<String>,
}

// ── 辅助类型 ──

/// 轨迹状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Partial,
}

/// 时间线位置（用于前端时间旅行面板）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct TimelinePosition {
    /// 时间戳（毫秒）
    pub timestamp_ms: u64,
    /// 节点 ID
    pub node_id: String,
    /// 节点索引（0-based）
    pub node_index: u32,
    /// 相对进度（0.0 - 1.0）
    pub progress: f64,
}

impl TimelinePosition {
    /// 从 ExecutionTrace 生成时间线位置列表
    pub fn from_trace(trace: &ExecutionTrace) -> Vec<Self> {
        if trace.node_traces.is_empty() {
            return Vec::new();
        }

        let total = trace.node_traces.len() as f64;
        trace
            .node_traces
            .iter()
            .enumerate()
            .map(|(idx, node)| TimelinePosition {
                timestamp_ms: node.started_at_ms.unwrap_or(trace.started_at_ms),
                node_id: node.node_id.clone(),
                node_index: idx as u32,
                progress: (idx as f64 + 1.0) / total,
            })
            .collect()
    }
}

/// Schema 差异报告（用于高亮显示）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SchemaDiffReport {
    /// 预期 Schema 路径
    pub expected_path: String,
    /// 实际输出路径
    pub actual_path: String,
    /// 差异类型
    pub diff_type: SchemaDiffType,
    /// 预期值
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_value: Option<serde_json::Value>,
    /// 实际值
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_value: Option<serde_json::Value>,
    /// 修复建议
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SchemaDiffType {
    TypeMismatch,
    MissingField,
    ExtraField,
    ValueOutOfRange,
    PatternMismatch,
    Unknown,
}

impl SchemaDiffReport {
    /// 从 SchemaValidationError 生成差异报告
    pub fn from_validation_error(error: &crate::schema::SchemaValidationError) -> Self {
        Self {
            expected_path: error.path.clone(),
            actual_path: error.path.clone(),
            diff_type: SchemaDiffType::Unknown,
            expected_value: error.expected_type.as_ref().map(|t| serde_json::json!(t)),
            actual_value: error.actual_value.clone(),
            suggestion: Some(error.message.clone()),
        }
    }
}

// ── 聚合统计 ──

/// 执行轨迹聚合统计
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct TraceStatistics {
    /// 总节点数
    pub total_nodes: u32,
    /// 成功节点数
    pub success_nodes: u32,
    /// 失败节点数
    pub failed_nodes: u32,
    /// 跳过节点数
    pub skipped_nodes: u32,
    /// 总耗时（毫秒）
    pub total_duration_ms: u64,
    /// 平均节点耗时（毫秒）
    pub avg_node_duration_ms: f64,
    /// 最慢节点 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slowest_node_id: Option<String>,
    /// 最慢节点耗时（毫秒）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slowest_duration_ms: Option<u64>,
    /// 总工具调用次数
    pub total_tool_calls: u32,
    /// 总 Token 使用量
    pub total_tokens: u64,
    /// Schema 校验错误数
    pub schema_errors: u32,
}

impl TraceStatistics {
    /// 从 ExecutionTrace 计算统计
    pub fn from_trace(trace: &ExecutionTrace) -> Self {
        let mut stats = Self { total_nodes: trace.node_traces.len() as u32, ..Default::default() };

        let mut total_duration: u64 = 0;
        let mut slowest_duration: u64 = 0;
        let mut slowest_node_id: Option<String> = None;

        for node in &trace.node_traces {
            match node.status {
                NodeStatus::Completed => stats.success_nodes += 1,
                NodeStatus::Failed => stats.failed_nodes += 1,
                NodeStatus::Skipped => stats.skipped_nodes += 1,
                _ => {},
            }

            if let Some(duration) = node.duration_ms {
                total_duration += duration;
                if duration > slowest_duration {
                    slowest_duration = duration;
                    slowest_node_id = Some(node.node_id.clone());
                }
            }

            stats.total_tool_calls += node.tool_calls.len() as u32;

            if let Some(ref usage) = node.token_usage {
                stats.total_tokens += usage.total_tokens as u64;
            }

            if node.has_schema_error() {
                stats.schema_errors += 1;
            }
        }

        stats.total_duration_ms = trace.total_duration_ms().unwrap_or(total_duration);
        stats.avg_node_duration_ms = if stats.total_nodes > 0 {
            total_duration as f64 / stats.total_nodes as f64
        } else {
            0.0
        };
        stats.slowest_duration_ms = if slowest_duration > 0 {
            Some(slowest_duration)
        } else {
            None
        };
        stats.slowest_node_id = slowest_node_id;

        stats
    }
}

// ── 决策日志（时间旅行核心） ──

/// 决策日志类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DecisionLogType {
    /// FSM 状态转移决策
    FsmTransition,
    /// 条件分支决策（if-else）
    ConditionalBranch,
    /// 工具选择决策
    ToolSelection,
    /// Agent 路由决策
    AgentRouting,
    /// 重试决策
    Retry,
    /// 跳过决策
    Skip,
    /// 补偿决策
    Compensation,
    /// 自定义决策
    Custom,
}

/// 决策选项（被考虑过的选项）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DecisionOption {
    /// 选项 ID
    pub option_id: String,
    /// 选项名称
    pub option_name: String,
    /// 选项描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 选项的评估分数（如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// 是否被选中
    pub selected: bool,
}

/// 决策日志（记录工作流中的关键决策点）
///
/// 用于时间旅行还原：记录每个决策点的上下文、
/// 考虑过的选项、最终选择和决策原因。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DecisionLog {
    /// 决策日志 ID
    pub id: String,
    /// 决策类型
    pub decision_type: DecisionLogType,
    /// 决策时间戳（毫秒）
    pub timestamp_ms: u64,
    /// 关联的节点 ID（如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// 关联的 FSM 实例 ID（如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fsm_instance_id: Option<String>,
    /// 决策上下文（决策时的输入条件/状态）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    /// 考虑过的选项列表
    #[serde(default)]
    pub options: Vec<DecisionOption>,
    /// 最终选择的选项 ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_option_id: Option<String>,
    /// 决策原因/解释
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 决策结果（成功/失败）
    pub result: DecisionResult,
    /// 决策耗时（毫秒，如有）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// 决策者标识（如 "system", "llm", "rule" 等）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decider: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DecisionResult {
    Success,
    Failed,
    Skipped,
    Cancelled,
}

impl DecisionLog {
    pub fn new(decision_type: DecisionLogType) -> Self {
        let now_ms = current_timestamp_ms();
        Self {
            id: format!("decision_{}", uuid::Uuid::new_v4()),
            decision_type,
            timestamp_ms: now_ms,
            node_id: None,
            fsm_instance_id: None,
            context: None,
            options: Vec::new(),
            selected_option_id: None,
            reason: None,
            result: DecisionResult::Success,
            duration_ms: None,
            decider: None,
        }
    }

    /// 关联节点
    pub fn with_node(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    /// 关联 FSM 实例
    pub fn with_fsm_instance(mut self, fsm_instance_id: impl Into<String>) -> Self {
        self.fsm_instance_id = Some(fsm_instance_id.into());
        self
    }

    /// 设置决策上下文
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    /// 添加决策选项
    pub fn add_option(&mut self, option: DecisionOption) {
        self.options.push(option);
    }

    /// 选择选项
    pub fn select_option(&mut self, option_id: impl Into<String>) {
        let id = option_id.into();
        for option in &mut self.options {
            option.selected = option.option_id == id;
        }
        self.selected_option_id = Some(id);
    }

    /// 设置决策原因
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// 设置决策者
    pub fn with_decider(mut self, decider: impl Into<String>) -> Self {
        self.decider = Some(decider.into());
        self
    }

    /// 标记决策成功
    pub fn mark_success(&mut self) {
        self.result = DecisionResult::Success;
        self.finalize();
    }

    /// 标记决策失败
    pub fn mark_failed(&mut self) {
        self.result = DecisionResult::Failed;
        self.finalize();
    }

    /// 标记为跳过
    pub fn mark_skipped(&mut self) {
        self.result = DecisionResult::Skipped;
        self.finalize();
    }

    /// 完成决策记录
    fn finalize(&mut self) {
        let now_ms = current_timestamp_ms();
        self.duration_ms = Some(now_ms.saturating_sub(self.timestamp_ms));
    }

    /// 获取选中的选项
    pub fn get_selected_option(&self) -> Option<&DecisionOption> {
        self.selected_option_id
            .as_ref()
            .and_then(|id| self.options.iter().find(|o| o.option_id == *id))
    }

    /// 检查是否有多个选项被考虑
    pub fn has_multiple_options(&self) -> bool {
        self.options.len() > 1
    }
}

impl DecisionOption {
    pub fn new(option_id: impl Into<String>, option_name: impl Into<String>) -> Self {
        Self {
            option_id: option_id.into(),
            option_name: option_name.into(),
            description: None,
            score: None,
            selected: false,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_score(mut self, score: f64) -> Self {
        self.score = Some(score);
        self
    }
}

// ── 工具函数 ──

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SchemaValidationResult;

    fn create_test_trace() -> ExecutionTrace {
        let mut trace = ExecutionTrace::new("exec-1", "workflow-1");

        let mut node1 = NodeExecutionTrace::new("node-1", "agent");
        node1.mark_started();
        std::thread::sleep(std::time::Duration::from_millis(1));
        node1.mark_completed(serde_json::json!({"result": "ok"}));
        node1.token_usage = Some(TokenUsageTrace {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            model: Some("gpt-4".to_string()),
            estimated_cost_usd: Some(0.003),
        });
        trace.add_node_trace(node1);

        let mut node2 = NodeExecutionTrace::new("node-2", "tool");
        node2.mark_started();
        std::thread::sleep(std::time::Duration::from_millis(1));
        node2.mark_completed(serde_json::json!({"status": "success"}));
        trace.add_node_trace(node2);

        let mut node3 = NodeExecutionTrace::new("node-3", "agent");
        node3.mark_started();
        std::thread::sleep(std::time::Duration::from_millis(1));
        node3.mark_failed(NodeErrorDetail::new(
            NodeErrorType::SchemaValidation,
            "输出不符合 Schema",
        ));
        node3.output_validation =
            Some(SchemaValidationResult::invalid(vec![crate::schema::SchemaValidationError {
                path: "/result".to_string(),
                message: "类型不匹配".to_string(),
                expected_type: Some("string".to_string()),
                actual_value: Some(serde_json::json!(42)),
            }]));
        trace.add_node_trace(node3);

        trace.complete(Some(serde_json::json!({"final": "ok"})));
        trace
    }

    #[test]
    fn test_create_trace() {
        let trace = ExecutionTrace::new("exec-1", "wf-1");
        assert_eq!(trace.execution_id, "exec-1");
        assert_eq!(trace.workflow_id, "wf-1");
        assert_eq!(trace.status, TraceStatus::InProgress);
        assert!(trace.node_traces.is_empty());
    }

    #[test]
    fn test_add_node_trace() {
        let mut trace = ExecutionTrace::new("exec-1", "wf-1");
        let node = NodeExecutionTrace::new("node-1", "agent");
        trace.add_node_trace(node);
        assert_eq!(trace.node_traces.len(), 1);
    }

    #[test]
    fn test_complete_trace() {
        let mut trace = ExecutionTrace::new("exec-1", "wf-1");
        trace.complete(Some(serde_json::Value::Null));
        assert_eq!(trace.status, TraceStatus::Completed);
        assert!(trace.completed_at_ms.is_some());
        assert!(trace.global_output.is_some());
    }

    #[test]
    fn test_node_trace_lifecycle() {
        let mut node = NodeExecutionTrace::new("test-node", "agent");
        assert_eq!(node.status, NodeStatus::Pending);

        node.mark_started();
        assert_eq!(node.status, NodeStatus::Running);
        assert!(node.started_at_ms.is_some());

        node.mark_completed(serde_json::json!({"ok": true}));
        assert_eq!(node.status, NodeStatus::Completed);
        assert!(node.completed_at_ms.is_some());
        assert!(node.duration_ms.is_some());
        assert!(node.output.is_some());
    }

    #[test]
    fn test_node_trace_failed() {
        let mut node = NodeExecutionTrace::new("test-node", "tool");
        node.mark_started();
        node.mark_failed(NodeErrorDetail::new(NodeErrorType::ToolExecution, "工具调用失败"));
        assert_eq!(node.status, NodeStatus::Failed);
        assert!(node.error.is_some());
        assert_eq!(node.error.as_ref().unwrap().error_type, NodeErrorType::ToolExecution);
    }

    #[test]
    fn test_find_error_nodes() {
        let trace = create_test_trace();
        let errors = trace.find_error_nodes();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].node_id, "node-3");
    }

    #[test]
    fn test_find_schema_errors() {
        let trace = create_test_trace();
        let schema_errors = trace.find_schema_errors();
        assert_eq!(schema_errors.len(), 1);
        assert_eq!(schema_errors[0].node_id, "node-3");
    }

    #[test]
    fn test_success_rate() {
        let trace = create_test_trace();
        // 3 nodes: 2 completed, 1 failed
        assert!((trace.success_rate() - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_node_tool_calls() {
        let mut node = NodeExecutionTrace::new("test-node", "agent");
        let call = ToolCallTrace::new("search", Some(serde_json::json!({"q": "test"})));
        node.add_tool_call(call);
        assert_eq!(node.tool_calls.len(), 1);
        assert_eq!(node.tool_calls[0].tool_name, "search");
    }

    #[test]
    fn test_tool_call_completion() {
        let mut call = ToolCallTrace::new("search", None);
        // 确保 start 和 end 之间有时间差
        std::thread::sleep(std::time::Duration::from_millis(1));
        call.mark_completed(serde_json::json!({"results": []}));
        assert_eq!(call.status, ToolCallStatus::Completed);
        assert!(call.result.is_some());
    }

    #[test]
    fn test_tool_call_failure() {
        let mut call = ToolCallTrace::new("search", None);
        call.mark_failed("网络错误");
        assert_eq!(call.status, ToolCallStatus::Failed);
        assert_eq!(call.error, Some("网络错误".to_string()));
    }

    #[test]
    fn test_node_error_detail() {
        let error = NodeErrorDetail::new(NodeErrorType::Timeout, "超时")
            .with_retryable(true)
            .with_suggestion("增加超时时间");
        assert!(error.retryable);
        assert_eq!(error.suggestion, Some("增加超时时间".to_string()));
    }

    #[test]
    fn test_timeline_position() {
        let trace = create_test_trace();
        let positions = TimelinePosition::from_trace(&trace);
        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0].node_index, 0);
        assert!((positions[2].progress - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_trace_statistics() {
        let trace = create_test_trace();
        let stats = TraceStatistics::from_trace(&trace);

        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.success_nodes, 2);
        assert_eq!(stats.failed_nodes, 1);
        assert_eq!(stats.total_tool_calls, 0);
        assert_eq!(stats.total_tokens, 150);
        assert_eq!(stats.schema_errors, 1);
        assert!(stats.total_duration_ms > 0);
    }

    #[test]
    fn test_schema_diff_report() {
        let error = crate::schema::SchemaValidationError {
            path: "/data".to_string(),
            message: "应为字符串".to_string(),
            expected_type: Some("string".to_string()),
            actual_value: Some(serde_json::json!(42)),
        };

        let report = SchemaDiffReport::from_validation_error(&error);
        assert_eq!(report.expected_path, "/data");
        assert_eq!(report.diff_type, SchemaDiffType::Unknown);
        assert!(report.suggestion.is_some());
    }

    #[test]
    fn test_trace_error_summary() {
        let summary = TraceErrorSummary {
            error_node_count: 1,
            error_node_ids: vec!["node-3".to_string()],
            schema_error_count: 1,
            tool_error_count: 0,
            first_error_at_ms: Some(1000),
            root_cause_analysis: Some("Schema 不匹配".to_string()),
        };
        assert_eq!(summary.error_node_count, 1);
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsageTrace {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            model: Some("gpt-4".to_string()),
            estimated_cost_usd: Some(0.003),
        };
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn test_decision_log_creation() {
        let log = DecisionLog::new(DecisionLogType::FsmTransition)
            .with_node("node-1")
            .with_fsm_instance("fsm-1")
            .with_context(serde_json::json!({"from": "submitted", "to": "reviewing"}))
            .with_reason("状态机转移")
            .with_decider("system");

        assert_eq!(log.decision_type, DecisionLogType::FsmTransition);
        assert_eq!(log.node_id, Some("node-1".to_string()));
        assert_eq!(log.fsm_instance_id, Some("fsm-1".to_string()));
        assert!(log.context.is_some());
        assert_eq!(log.decider, Some("system".to_string()));
    }

    #[test]
    fn test_decision_options() {
        let mut log = DecisionLog::new(DecisionLogType::ToolSelection);

        let option1 = DecisionOption::new("search", "搜索引擎")
            .with_description("使用搜索引擎查找信息")
            .with_score(0.9);
        let option2 = DecisionOption::new("database", "数据库查询")
            .with_description("直接查询数据库")
            .with_score(0.5);

        log.add_option(option1);
        log.add_option(option2);

        assert_eq!(log.options.len(), 2);
        assert!(log.has_multiple_options());

        log.select_option("search");
        assert_eq!(log.selected_option_id, Some("search".to_string()));

        let selected = log.get_selected_option();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().option_id, "search");
        assert!(selected.unwrap().selected);
    }

    #[test]
    fn test_decision_log_result() {
        let mut log = DecisionLog::new(DecisionLogType::Retry);
        log.mark_success();
        assert_eq!(log.result, DecisionResult::Success);
        assert!(log.duration_ms.is_some());

        let mut log2 = DecisionLog::new(DecisionLogType::Skip);
        log2.mark_failed();
        assert_eq!(log2.result, DecisionResult::Failed);
    }

    #[test]
    fn test_decision_logs_in_trace() {
        let mut trace = ExecutionTrace::new("exec-1", "wf-1");

        let log1 = DecisionLog::new(DecisionLogType::FsmTransition).with_node("node-1");
        let log2 = DecisionLog::new(DecisionLogType::ToolSelection).with_node("node-2");
        let log3 = DecisionLog::new(DecisionLogType::Retry).with_node("node-1");

        trace.add_decision_log(log1);
        trace.add_decision_log(log2);
        trace.add_decision_log(log3);

        assert_eq!(trace.decision_log_count(), 3);

        let fsm_logs = trace.get_decision_logs_by_type(&DecisionLogType::FsmTransition);
        assert_eq!(fsm_logs.len(), 1);

        let node1_logs = trace.get_decision_logs_for_node("node-1");
        assert_eq!(node1_logs.len(), 2);
    }
}
