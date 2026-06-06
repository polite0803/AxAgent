use async_trait::async_trait;
use axagent_core::workflow_types::{LoopType, WorkflowNode};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

pub struct LoopExecutor;

impl LoopExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoopExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析点号路径。空路径返回 None。执行 Loop 时由调度器读此输出
/// 来决定把哪一段 body_steps 激活以及在哪个 iteratee_var 上迭代。
#[async_trait]
impl NodeExecutorTrait for LoopExecutor {
    fn node_type(&self) -> &'static str {
        "loop"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Loop(n) = node else {
            return Err(NodeError::type_mismatch("loop", self.node_type()));
        };
        let c = &n.config;

        if c.body_steps.is_empty() {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "Loop node has empty body_steps".to_string(),
            ));
        }

        let loop_type_label = match c.loop_type {
            LoopType::ForEach => "forEach",
            LoopType::While => "while",
            LoopType::DoWhile => "doWhile",
            LoopType::Until => "until",
        };

        // 解析 items_var：把变量值规范成 Vec<Value>
        let items: Vec<serde_json::Value> = match &c.items_var {
            Some(var_name) if !var_name.is_empty() => {
                let v = context.variables.get(var_name);
                match v {
                    Some(serde_json::Value::Array(arr)) => arr.clone(),
                    Some(other) => vec![other.clone()],
                    None => Vec::new(),
                }
            },
            _ => Vec::new(),
        };

        // 估算迭代次数（实际由 engine 调度，此处只提供上限提示）
        let effective_max = c.max_iterations.unwrap_or(items.len() as u32).min(10_000);

        // 校验 While/Until 的 continue_condition
        if matches!(c.loop_type, LoopType::While | LoopType::Until)
            && c.continue_condition.is_none()
        {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                format!("{loop_type_label} loop requires continue_condition"),
            ));
        }

        Ok(NodeOutput {
            output: serde_json::json!({
                "loop_type": loop_type_label,
                "item_count": items.len(),
                "max_iterations": effective_max,
                "continue_on_error": c.continue_on_error,
                "has_continue_condition": c.continue_condition.is_some(),
                "items_var": c.items_var,
                "iteratee_var": c.iteratee_var,
                "body_steps": c.body_steps,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
