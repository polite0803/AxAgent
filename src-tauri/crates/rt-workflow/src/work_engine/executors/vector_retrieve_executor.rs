// SPDX-License-Identifier: AGPL-3.0-only

//! 向量检索执行器 —— 通过注入的回调查询向量存储。
//!
//! 注意：VectorRetrieveCallback 三向分裂（WorkEngine / Executor / ExecutionState）已收敛。
//! 当前无注册入口（`set_vector_retrieve_callback` / `set_callback` 已删除），节点始终返回
//! "not_configured"。若未来要重新启用，需在 init 阶段引入新的注入路径（推荐走 ExecutionContextCallbacks
//! 单源传播），禁止再分裂到 WorkEngine/Executor/State 三处。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::NodeExecutorTrait;
use axagent_harness::workflow_types::WorkflowNode;

pub struct VectorRetrieveExecutor;

impl VectorRetrieveExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for VectorRetrieveExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl NodeExecutorTrait for VectorRetrieveExecutor {
    fn node_type(&self) -> &'static str {
        "vectorRetrieve"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<
        crate::work_engine::node_executor_trait::NodeOutput,
        crate::work_engine::node_executor_trait::NodeError,
    > {
        let WorkflowNode::VectorRetrieve(vr) = node else {
            return Err(crate::work_engine::node_executor_trait::NodeError::type_mismatch(
                "vectorRetrieve".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };
        let resolved_query = resolve_query_template(&vr.config.query, &context.variables);

        // 当前未启用 callback 注入：始终返回 not_configured。
        // 未来重新启用时，只在 ExecutionContextCallbacks 单源注入 callback，
        // 不要再在 WorkEngine / Executor 上开第二/第三个入口。
        Ok(crate::work_engine::node_executor_trait::NodeOutput {
            output: serde_json::json!({
                "query": resolved_query, "knowledge_base_id": vr.config.knowledge_base_id,
                "top_k": vr.config.top_k,
                "results": [{
                    "status": "not_configured",
                    "message": "Vector retrieve callback not configured"
                }],
                "node_id": node.base_id(),
            }),
            output_var: Some(vr.config.output_var.clone()),
        })
    }
}

fn resolve_query_template(
    template: &str,
    vars: &std::collections::HashMap<String, serde_json::Value>,
) -> String {
    let mut result = template.to_string();
    for (k, v) in vars {
        let placeholder = format!("{{{{{}}}}}", k);
        let owned = v.to_string();
        let replacement = v.as_str().unwrap_or(&owned);
        result = result.replace(&placeholder, replacement);
    }
    result
}
