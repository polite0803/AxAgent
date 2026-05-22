//! 代码执行器 —— 执行 CodeNode 中的代码片段。
//!
//! 当前返回代码摘要（语言 + 行数 + 输出变量），后续可接入沙箱执行。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct CodeExecutor;

impl CodeExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for CodeExecutor {
    fn node_type(&self) -> &'static str {
        "code"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Code(code_node) = node else {
            return Err(NodeError::InvalidNodeType {
                expected: "code".to_string(),
                got: super::node_type_name(node).to_string(),
            });
        };

        // 当前不实际执行代码，返回代码摘要供 LLM 或下游节点使用
        let code_lines = code_node.config.code.lines().count();
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "code_ready",
                "language": code_node.config.language,
                "code_lines": code_lines,
                "code_preview": &code_node.config.code[..code_node.config.code.len().min(500)],
                "node_id": node.base_id(),
            }),
            output_var: Some(code_node.config.output_var.clone()),
        })
    }
}
