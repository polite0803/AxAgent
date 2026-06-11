// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

pub struct StorageExecutor;

impl StorageExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StorageExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 从 ExecutionState.variables 按 path 提取值
fn resolve_var_path(path: &str, ctx: &ExecutionState) -> Option<serde_json::Value> {
    if path.is_empty() {
        return None;
    }
    let mut current = ctx.variables.get(path)?.clone();
    for segment in path.split('.').skip(1) {
        current = current.get(segment)?.clone();
    }
    Some(current)
}

#[async_trait]
impl NodeExecutorTrait for StorageExecutor {
    fn node_type(&self) -> &'static str {
        "storage"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Storage(n) = node else {
            return Err(NodeError::type_mismatch("storage", self.node_type()));
        };
        let c = &n.config;

        // 1. 获取输入数据
        let input_data = match resolve_var_path(&c.input_var, context) {
            Some(v) => v,
            None => {
                return Err(NodeError::exec_failed(
                    error_code::VARIABLE_NOT_FOUND,
                    format!("Storage: input_var '{}' not found", c.input_var),
                ));
            },
        };

        // 2. 获取 upsert key（可选）
        let key = if let Some(ref key_var) = c.key_var {
            resolve_var_path(key_var, context)
        } else {
            None
        };

        // 3. 根据后端和操作模式执行存储
        let (rows_affected, status_msg) = match c.backend.as_str() {
            "sqlite" => {
                // SQLite 后端：写入 JSON 到 kv_store 表
                // 实际环境需注入 DatabaseConnection
                // 当前为骨架实现，返回模拟结果
                let mode = match c.operation.as_str() {
                    "upsert" => "upsert",
                    "append" => "append",
                    _ => "insert",
                };
                (1, format!("sqlite_{mode}"))
            },
            "vectorDb" => {
                // VectorDB 后端：写入/kb/{collection}
                // 当前为骨架实现
                let mode = match c.operation.as_str() {
                    "upsert" => "upsert",
                    _ => "insert",
                };
                (1, format!("vectorDb_{mode}"))
            },
            "fileSystem" => {
                // FileSystem 后端：写入文件
                match c.operation.as_str() {
                    "append" => {
                        let existing = std::fs::read_to_string(&c.collection).unwrap_or_default();
                        let content = input_data.to_string();
                        std::fs::write(&c.collection, format!("{}{}", existing, content)).map_err(
                            |e| {
                                NodeError::exec_failed(
                                    error_code::IO_ERROR,
                                    format!("Storage write failed: {e}"),
                                )
                            },
                        )?;
                        (1, "fileSystem_append".to_string())
                    },
                    _ => {
                        // insert / upsert → 覆盖写
                        let content = input_data.to_string();
                        // 确保父目录存在
                        if let Some(parent) = std::path::Path::new(&c.collection).parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        std::fs::write(&c.collection, content).map_err(|e| {
                            NodeError::exec_failed(
                                error_code::IO_ERROR,
                                format!("Storage write failed: {e}"),
                            )
                        })?;
                        (1, "fileSystem_write".to_string())
                    },
                }
            },
            other => {
                return Err(NodeError::exec_failed(
                    error_code::VALIDATION_FAILED,
                    format!("Storage: unknown backend '{other}'"),
                ));
            },
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "backend": c.backend,
                "operation": c.operation,
                "collection": c.collection,
                "rows_affected": rows_affected,
                "status": status_msg,
                "key": key,
                "node_id": node.base_id(),
            }),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
