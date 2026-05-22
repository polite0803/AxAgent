//! 文档解析执行器 —— 解析文档内容（PDF、DOCX、Excel 等）。

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct DocumentParserExecutor;

impl DocumentParserExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocumentParserExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for DocumentParserExecutor {
    fn node_type(&self) -> &'static str {
        "document_parser"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "parsed",
                "node_id": node.base_id(),
                "content_preview": "文档解析器尚未完全实现，返回模拟输出",
            }),
            output_var: None,
        })
    }
}
