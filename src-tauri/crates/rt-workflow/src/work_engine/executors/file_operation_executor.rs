// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct FileOperationExecutor;

impl FileOperationExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileOperationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for FileOperationExecutor {
    fn node_type(&self) -> &'static str {
        "fileOperation"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        _ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::FileOperation(n) = node else {
            return Err(NodeError::type_mismatch("fileOperation", self.node_type()));
        };
        let c = &n.config;
        let result = match c.operation.as_str() {
            "read" => std::fs::read_to_string(&c.file_path)
                .map_err(|e| NodeError::exec_failed("file_read_error", e)),
            "write" => {
                if let Some(content) = &c.content {
                    std::fs::write(&c.file_path, content)
                        .map_err(|e| NodeError::exec_failed("file_write_error", e))
                        .map(|_| String::new())
                } else {
                    Err(NodeError::exec_failed("empty_content", "No content to write"))
                }
            },
            "delete" => std::fs::remove_file(&c.file_path)
                .map_err(|e| NodeError::exec_failed("file_delete_error", e))
                .map(|_| String::new()),
            "exists" => Ok(if std::path::Path::new(&c.file_path).exists() {
                "true"
            } else {
                "false"
            }
            .to_string()),
            _ => Err(NodeError::exec_failed(
                "unknown_operation",
                format!("Unknown operation: {}", c.operation),
            )),
        }?;
        Ok(NodeOutput {
            output: serde_json::json!({"operation": c.operation, "path": c.file_path, "result": result, "node_id": node.base_id()}),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
