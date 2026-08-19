// SPDX-License-Identifier: AGPL-3.0-only

//! Node executor trait and related types

use async_trait::async_trait;
use axagent_harness::workflow_types::{ExecutionStatus, NodeKind, WorkflowNode};
use serde::{Deserialize, Serialize};

/// Output from a node execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    #[serde(default)]
    pub output: serde_json::Value,
    pub output_var: Option<String>,
    /// 可选控制指令（如 Suspend 挂起）。默认 None = 正常完成。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<NodeControl>,
}

/// 节点对引擎发出的控制指令（挂起/恢复等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeControl {
    /// 挂起整个工作流，等待外部审批/信号后恢复
    Suspend { resume_token: String, approval: ApprovalRequest },
}

/// 审批请求数据（由 ApprovalExecutor 产生，引擎通过 ApprovalOps 持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub execution_id: String,
    pub node_id: String,
    pub title: String,
    pub message: String,
    pub approver: Option<String>,
    pub channels: Vec<String>,
    pub payload: serde_json::Value,
    pub timeout_secs: u64,
    pub timeout_action: String,
}

// ── Error codes ──

/// 节点执行错误码（前端根据 code 查 i18n 翻译）
pub mod error_code {
    pub const PROVIDER_QUERY_FAILED: &str = "PROVIDER_QUERY_FAILED";
    pub const NO_AVAILABLE_PROVIDER: &str = "NO_AVAILABLE_PROVIDER";
    pub const API_KEY_DECRYPT_FAILED: &str = "API_KEY_DECRYPT_FAILED";
    pub const UNSUPPORTED_PROVIDER: &str = "UNSUPPORTED_PROVIDER";
    pub const LLM_CALL_FAILED: &str = "LLM_CALL_FAILED";
    pub const AGENT_PROFILE_NOT_FOUND: &str = "AGENT_PROFILE_NOT_FOUND";
    pub const TOOL_CALL_FAILED: &str = "TOOL_CALL_FAILED";
    pub const TOOL_NOT_CONFIGURED: &str = "TOOL_NOT_CONFIGURED";
    pub const SUBWORKFLOW_FAILED: &str = "SUBWORKFLOW_FAILED";
    pub const SUBWORKFLOW_NOT_CONFIGURED: &str = "SUBWORKFLOW_NOT_CONFIGURED";
    pub const VECTOR_RETRIEVE_FAILED: &str = "VECTOR_RETRIEVE_FAILED";
    pub const VECTOR_NOT_CONFIGURED: &str = "VECTOR_NOT_CONFIGURED";
    pub const VARIABLE_NOT_FOUND: &str = "VARIABLE_NOT_FOUND";
    pub const VALIDATION_FAILED: &str = "VALIDATION_FAILED";
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";
    pub const TIMEOUT: &str = "TIMEOUT";
    pub const CIRCUIT_BREAKER_OPEN: &str = "CIRCUIT_BREAKER_OPEN";
    pub const NODE_TYPE_MISMATCH: &str = "NODE_TYPE_MISMATCH";
    pub const UNSUPPORTED_NODE_TYPE: &str = "UNSUPPORTED_NODE_TYPE";
    pub const IO_ERROR: &str = "IO_ERROR";
    pub const CACHE_DESERIALIZE_FAILED: &str = "CACHE_DESERIALIZE_FAILED";
    pub const MODEL_NOT_CONFIGURED: &str = "MODEL_NOT_CONFIGURED";
    pub const NODE_NOT_FOUND: &str = "NODE_NOT_FOUND";
    pub const EXECUTION_CANCELLED: &str = "EXECUTION_CANCELLED";
}

/// Error types for node execution
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("{code}: {detail}")]
    ExecutionFailed { code: &'static str, detail: String },

    #[error("{code}: {detail}")]
    Timeout { code: &'static str, detail: String },

    #[error("{code}: expected {expected}, got {got}")]
    InvalidNodeType { code: &'static str, expected: String, got: String },

    #[error("VARIABLE_NOT_FOUND: {0}")]
    VariableNotFound(String),

    #[error("VALIDATION_FAILED: {0}")]
    Validation(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl NodeError {
    /// 创建执行失败错误
    pub fn exec_failed(code: &'static str, detail: impl std::fmt::Display) -> Self {
        Self::ExecutionFailed { code, detail: detail.to_string() }
    }
    /// 创建超时错误
    pub fn timed_out(code: &'static str, detail: impl std::fmt::Display) -> Self {
        Self::Timeout { code, detail: detail.to_string() }
    }
    /// 创建节点类型不匹配错误
    pub fn type_mismatch(expected: impl Into<String>, got: impl Into<String>) -> Self {
        Self::InvalidNodeType {
            code: error_code::NODE_TYPE_MISMATCH,
            expected: expected.into(),
            got: got.into(),
        }
    }

    /// 获取错误码（前端 i18n 映射 key）
    pub fn code(&self) -> &str {
        match self {
            Self::ExecutionFailed { code, .. } => code,
            Self::Timeout { code, .. } => code,
            Self::InvalidNodeType { code, .. } => code,
            Self::VariableNotFound(_) => error_code::VARIABLE_NOT_FOUND,
            Self::Validation(_) => error_code::VALIDATION_FAILED,
            Self::Io(_) => error_code::IO_ERROR,
            // EXECUTION_CANCELLED 通过 ExecutionFailed 变体传递
        }
    }
}

impl From<NodeError> for serde_json::Value {
    fn from(err: NodeError) -> Self {
        serde_json::json!({
            "code": err.code(),
            "message": err.to_string(),
        })
    }
}

// ── Trait ──

/// 检查执行是否被取消或暂停。
///
/// 所有执行器在长时间运行的循环/操作前应调用此函数，以支持：
/// 1. 硬取消（cancel_token 被触发时立即中止）
/// 2. 软暂停（status == Paused 时挂起等待恢复信号）
///
/// # 返回值
/// - `Ok(())`: 可以继续执行
/// - `Err(NodeError)`: 应中止执行
pub async fn check_cancellation_or_pause(context: &super::ExecutionState) -> Result<(), NodeError> {
    // 1. 检查硬取消
    if let Some(ref token) = context.cancel_token
        && token.is_cancelled()
    {
        return Err(NodeError::exec_failed(error_code::EXECUTION_CANCELLED, "节点执行已取消"));
    }

    // 2. 检查软暂停（循环等待恢复信号，同时定期检查取消状态）
    // 注意：context.status 由外部恢复操作修改，此处通过 pause_signal 通知
    // 不直接修改 status 字段
    loop {
        // 先检查是否已暂停
        if context.status != ExecutionStatus::Paused {
            break;
        }

        // 尝试获取暂停信号并等待
        if let Some(signal) = context.pause_signal() {
            tokio::select! {
                _ = signal.notified() => {
                    // 恢复信号触发，跳出循环继续执行
                    tracing::debug!(
                        "[check_cancellation_or_pause] 暂停状态已恢复，继续执行"
                    );
                    break;
                }
                // 每 500ms 检查一次取消状态
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                    if let Some(ref token) = context.cancel_token
                        && token.is_cancelled()
                    {
                        return Err(NodeError::exec_failed(
                            error_code::EXECUTION_CANCELLED,
                            "节点执行已取消（暂停期间检测到取消）",
                        ));
                    }
                    // 继续循环等待
                    continue;
                }
            }
        } else {
            // 没有暂停信号但状态是 Paused，直接跳出
            break;
        }
    }

    Ok(())
}

#[async_trait]
pub trait NodeExecutorTrait: Send + Sync {
    fn node_type(&self) -> &'static str;
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &super::ExecutionState,
    ) -> Result<NodeOutput, NodeError>;
}

pub fn node_type_name(node: &WorkflowNode) -> &'static str {
    match node {
        WorkflowNode::Trigger(_) => "trigger",
        WorkflowNode::Agent(_) => "agent",
        WorkflowNode::Llm(_) => "llm",
        WorkflowNode::Condition(_) => "condition",
        WorkflowNode::Parallel(_) => "parallel",
        WorkflowNode::Loop(_) => "loop",
        WorkflowNode::Merge(_) => "merge",
        WorkflowNode::Delay(_) => "delay",
        WorkflowNode::SubWorkflow(_) => "subWorkflow",
        WorkflowNode::DocumentParser(_) => "documentParser",
        WorkflowNode::VectorRetrieve(_) => "vectorRetrieve",
        WorkflowNode::End(_) => "end",
        WorkflowNode::Tool(_) => "tool",
        WorkflowNode::Code(_) => "code",
        WorkflowNode::HttpRequest(_) => "httpRequest",
        WorkflowNode::Validation(_) => "validation",
        WorkflowNode::Switch(_) => "switch",
        WorkflowNode::DatabaseQuery(_) => "databaseQuery",
        WorkflowNode::Notification(_) => "notification",
        WorkflowNode::Approval(_) => "approval",
        WorkflowNode::FileOperation(_) => "fileOperation",
        WorkflowNode::DataTransformer(_) => "dataTransformer",
        WorkflowNode::WebhookSend(_) => "webhookSend",
        WorkflowNode::Logging(_) => "logging",
        WorkflowNode::LlmClassifier(_) => "llmClassifier",
        WorkflowNode::Aggregator(_) => "aggregator",
        WorkflowNode::Email(_) => "email",
        WorkflowNode::Debate(_) => "debate",
        WorkflowNode::Swarm(_) => "swarm",
        WorkflowNode::MultiAgent(_) => "multiAgent",
        WorkflowNode::Storage(_) => "storage",
        WorkflowNode::WorkflowRef(_) => "workflowRef",
    }
}

/// 获取节点的高级分类（NodeKind）
pub fn node_kind(node: &WorkflowNode) -> NodeKind {
    match node {
        WorkflowNode::Trigger(_) => NodeKind::Input,
        WorkflowNode::End(_) => NodeKind::Output,
        WorkflowNode::Tool(_)
        | WorkflowNode::Code(_)
        | WorkflowNode::HttpRequest(_)
        | WorkflowNode::FileOperation(_)
        | WorkflowNode::WebhookSend(_)
        | WorkflowNode::Logging(_)
        | WorkflowNode::Notification(_)
        | WorkflowNode::DatabaseQuery(_)
        | WorkflowNode::DocumentParser(_)
        | WorkflowNode::DataTransformer(_)
        | WorkflowNode::Email(_)
        | WorkflowNode::Validation(_)
        | WorkflowNode::Delay(_)
        | WorkflowNode::SubWorkflow(_)
        | WorkflowNode::WorkflowRef(_)
        | WorkflowNode::VectorRetrieve(_)
        | WorkflowNode::LlmClassifier(_)
        | WorkflowNode::Approval(_)
        | WorkflowNode::Aggregator(_) => NodeKind::Tool,
        WorkflowNode::Agent(_) | WorkflowNode::Llm(_) => NodeKind::Agent,
        WorkflowNode::Condition(_) | WorkflowNode::Switch(_) => NodeKind::Condition,
        WorkflowNode::Loop(_) => NodeKind::Loop,
        WorkflowNode::Parallel(_)
        | WorkflowNode::Merge(_)
        | WorkflowNode::Debate(_)
        | WorkflowNode::Swarm(_)
        | WorkflowNode::MultiAgent(_) => NodeKind::Container,
        WorkflowNode::Storage(_) => NodeKind::Storage,
    }
}
