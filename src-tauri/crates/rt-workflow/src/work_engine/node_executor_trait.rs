//! Node executor trait and related types

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use serde::{Deserialize, Serialize};

/// Output from a node execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    pub output: serde_json::Value,
    pub output_var: Option<String>,
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
    pub const TIMEOUT: &str = "TIMEOUT";
    pub const CIRCUIT_BREAKER_OPEN: &str = "CIRCUIT_BREAKER_OPEN";
    pub const NODE_TYPE_MISMATCH: &str = "NODE_TYPE_MISMATCH";
    pub const UNSUPPORTED_NODE_TYPE: &str = "UNSUPPORTED_NODE_TYPE";
    pub const CACHE_DESERIALIZE_FAILED: &str = "CACHE_DESERIALIZE_FAILED";
    pub const MODEL_NOT_CONFIGURED: &str = "MODEL_NOT_CONFIGURED";
    pub const NODE_NOT_FOUND: &str = "NODE_NOT_FOUND";
}

/// Error types for node execution
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("{code}: {detail}")]
    ExecutionFailed { code: &'static str, detail: String },

    #[error("{code}: {detail}")]
    Timeout { code: &'static str, detail: String },

    #[error("{code}: expected {expected}, got {got}")]
    InvalidNodeType {
        code: &'static str,
        expected: String,
        got: String,
    },

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
        Self::ExecutionFailed {
            code,
            detail: detail.to_string(),
        }
    }
    /// 创建超时错误
    pub fn timed_out(code: &'static str, detail: impl std::fmt::Display) -> Self {
        Self::Timeout {
            code,
            detail: detail.to_string(),
        }
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
            Self::Io(_) => error_code::UNSUPPORTED_NODE_TYPE,
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
        WorkflowNode::Validation(_) => "validation",
    }
}
