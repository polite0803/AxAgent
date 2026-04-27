//! Node executor trait and related types
//!
//! This module defines the trait for workflow node executors and
//! the error types used during node execution.

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use serde::{Deserialize, Serialize};

/// Output from a node execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    /// The output value from the node
    pub output: serde_json::Value,
    /// Optional variable name to store the output
    pub output_var: Option<String>,
}

/// Error types for node execution
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("Unsupported node type: {0}")]
    UnsupportedNodeType(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid node type: expected {expected}, got {got}")]
    InvalidNodeType { expected: String, got: String },

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Workflow error: {0}")]
    Workflow(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<NodeError> for serde_json::Value {
    fn from(err: NodeError) -> Self {
        serde_json::json!({
            "error_type": err.to_string(),
            "message": err.to_string(),
        })
    }
}

/// Trait for workflow node executors
///
/// Implementors of this trait can execute specific types of workflow nodes.
/// The trait is async and designed to be used in a runtime-agnostic way.
#[async_trait]
pub trait NodeExecutorTrait: Send + Sync {
    /// Returns the node type this executor handles
    fn node_type(&self) -> &'static str;

    /// Executes a workflow node
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &super::ExecutionState,
    ) -> Result<NodeOutput, NodeError>;
}

/// Returns the type name for a workflow node
pub fn node_type_name(node: &WorkflowNode) -> &'static str {
    match node {
        WorkflowNode::Trigger(_) => "trigger",
        WorkflowNode::AtomicSkill(_) => "atomic_skill",
        WorkflowNode::Agent(_) => "agent",
        WorkflowNode::Llm(_) => "llm",
        WorkflowNode::Condition(_) => "condition",
        WorkflowNode::Parallel(_) => "parallel",
        WorkflowNode::Loop(_) => "loop",
        WorkflowNode::Merge(_) => "merge",
        WorkflowNode::Delay(_) => "delay",
        WorkflowNode::SubWorkflow(_) => "sub_workflow",
        WorkflowNode::DocumentParser(_) => "document_parser",
        WorkflowNode::VectorRetrieve(_) => "vector_retrieve",
        WorkflowNode::End(_) => "end",
        WorkflowNode::Tool(_) => "tool",
        WorkflowNode::Code(_) => "code",
    }
}
