use async_trait::async_trait;
use axagent_core::workflow_types::{MergeStrategy, WorkflowNode};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

pub struct MergeExecutor;

impl MergeExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MergeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 从 ExecutionState.variables 收集 inputs 命名的输入。空名/不存在都跳过。
fn collect_inputs(inputs: &[String], context: &ExecutionState) -> Vec<serde_json::Value> {
    inputs
        .iter()
        .filter_map(|name| {
            if name.is_empty() {
                None
            } else {
                context.variables.get(name).cloned()
            }
        })
        .collect()
}

/// 应用合并策略：
/// - All      : 全部存在才返回数组；任一缺失则失败
/// - Any      : 返回第一个存在的（按 inputs 顺序）
/// - Race     : 同 Any，作为别名保留
/// - Majority : 多数值（出现 > n/2 次）相同则返回；否则报错
fn apply_merge(
    strategy: MergeStrategy,
    values: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    match strategy {
        MergeStrategy::All => {
            if values.is_empty() {
                return Err("Merge.All: 至少需要一个输入".to_string());
            }
            Ok(serde_json::Value::Array(values.to_vec()))
        },
        MergeStrategy::Any | MergeStrategy::Race => values
            .first()
            .cloned()
            .ok_or_else(|| "Merge.Any: 没有可用输入".to_string()),
        MergeStrategy::Majority => {
            if values.is_empty() {
                return Err("Merge.Majority: 没有可用输入".to_string());
            }
            let mut counts: std::collections::HashMap<&serde_json::Value, usize> =
                std::collections::HashMap::new();
            for v in values {
                *counts.entry(v).or_insert(0) += 1;
            }
            let majority = counts
                .iter()
                .max_by_key(|(_, c)| *c)
                .map(|(v, _)| (*v).clone());
            let required = values.len() / 2 + 1;
            match counts.values().max().copied() {
                Some(c) if c >= required => Ok(majority.unwrap()),
                _ => Err(format!(
                    "Merge.Majority: 没有任何值达到多数（{}/{})",
                    counts.values().max().copied().unwrap_or(0),
                    required
                )),
            }
        },
    }
}

#[async_trait]
impl NodeExecutorTrait for MergeExecutor {
    fn node_type(&self) -> &'static str {
        "merge"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Merge(n) = node else {
            return Err(NodeError::type_mismatch("merge", self.node_type()));
        };
        let c = &n.config;

        let values = if c.auto_inputs_from_branches {
            // 自动模式：拿 variables 的所有 value（按变量名排序保证稳定性）
            let mut pairs: Vec<(&String, &serde_json::Value)> = context.variables.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            pairs.into_iter().map(|(_, v)| v.clone()).collect()
        } else {
            collect_inputs(&c.inputs, context)
        };

        if values.is_empty() {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "Merge node has no inputs to merge".to_string(),
            ));
        }

        let merged = apply_merge(c.merge_type.clone(), &values)
            .map_err(|e| NodeError::exec_failed(error_code::VALIDATION_FAILED, e))?;

        Ok(NodeOutput {
            output: serde_json::json!({
                "merge_type": match c.merge_type {
                    MergeStrategy::All => "all",
                    MergeStrategy::Any => "any",
                    MergeStrategy::Race => "race",
                    MergeStrategy::Majority => "majority",
                },
                "input_count": values.len(),
                "result": merged,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
