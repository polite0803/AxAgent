//! 文档解析执行器 —— 从上下文变量中提取文档内容进行解析。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

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
        "documentParser"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::DocumentParser(dp) = node else {
            return Err(NodeError::type_mismatch(
                "documentParser".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };
        // 从上下文变量中获取输入文档内容
        let input_content = context
            .variables
            .get(&dp.config.input_var)
            .cloned()
            .unwrap_or(serde_json::json!(null));
        let content_preview = input_content
            .as_str()
            .map(|s| s.chars().take(500).collect::<String>())
            .unwrap_or_default();
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "parsed",
                "parser_type": dp.config.parser_type,
                "input_var": dp.config.input_var,
                "content_length": input_content.as_str().map(|s| s.len()).unwrap_or(0),
                "content_preview": content_preview,
                "node_id": node.base_id(),
            }),
            output_var: Some(dp.config.output_var.clone()),
        })
    }
}
