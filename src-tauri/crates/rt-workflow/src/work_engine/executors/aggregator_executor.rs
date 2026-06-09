use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

pub struct AggregatorExecutor;

impl AggregatorExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AggregatorExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 从 ExecutionState.variables 收集 input_sources 命名的输入。
/// 若 source 不存在则跳过；返回 Vec<(source_name, value)>。
fn collect_sources(
    sources: &[String],
    context: &ExecutionState,
) -> Vec<(String, serde_json::Value)> {
    sources
        .iter()
        .filter_map(|name| {
            if name.is_empty() {
                return None;
            }
            context
                .variables
                .get(name)
                .map(|v| (name.clone(), v.clone()))
        })
        .collect()
}

/// 应用聚合策略：
/// - "all"       : 收集成数组（默认）
/// - "concat"    : 把字符串拼接；其他类型 fallback 到 JSON 数组
/// - "sum"       : 累加数值，非数值忽略
/// - "merge"     : 浅合并对象（后者覆盖前者）
/// - "count"     : 返回数组长度
/// - "weighted"  : 加权求和（weighted sum），使用 config.weights；缺省为等权
/// - "llm_summarize": LLM 摘要（当前作为字符串拼接兜底，LLM 集成待后续）
fn apply_strategy(
    strategy: &str,
    pairs: &[(String, serde_json::Value)],
    weights: &[f64],
) -> serde_json::Value {
    let values: Vec<&serde_json::Value> = pairs.iter().map(|(_, v)| v).collect();
    match strategy {
        "concat" => {
            let parts: Vec<String> = values
                .iter()
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect();
            serde_json::Value::String(parts.join(""))
        },
        "weighted" => {
            // 加权求和：weights 与 input_sources 一一对应
            let mut total_weight = 0.0_f64;
            let mut weighted_sum = 0.0_f64;
            for (i, v) in values.iter().enumerate() {
                let w = weights.get(i).copied().unwrap_or(1.0);
                if let Some(n) = v.as_f64() {
                    weighted_sum += w * n;
                    total_weight += w;
                }
            }
            if total_weight > 0.0 {
                serde_json::json!(weighted_sum)
            } else {
                serde_json::json!(null)
            }
        },
        "sum" => {
            let sum: f64 = values.iter().filter_map(|v| v.as_f64()).sum();
            serde_json::json!(sum)
        },
        "merge" => {
            let mut merged = serde_json::Map::new();
            for v in values {
                if let serde_json::Value::Object(map) = v {
                    for (k, vv) in map {
                        merged.insert(k.clone(), vv.clone());
                    }
                }
            }
            serde_json::Value::Object(merged)
        },
        "count" => serde_json::json!(values.len()),
        // "llm_summarize": LLM 摘要暂不可用，fallback 到字符串拼接
        "llm_summarize" => {
            let parts: Vec<String> = values
                .iter()
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => serde_json::to_string_pretty(other).unwrap_or_default(),
                })
                .collect();
            serde_json::Value::String(parts.join("\n---\n"))
        },
        // "all" / 默认：原样数组
        _ => serde_json::Value::Array(values.into_iter().cloned().collect()),
    }
}

#[async_trait]
impl NodeExecutorTrait for AggregatorExecutor {
    fn node_type(&self) -> &'static str {
        "aggregator"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Aggregator(n) = node else {
            return Err(NodeError::type_mismatch("aggregator", self.node_type()));
        };
        let c = &n.config;

        if c.input_sources.is_empty() {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "Aggregator node has no input_sources".to_string(),
            ));
        }

        let pairs = collect_sources(&c.input_sources, context);
        let aggregated = apply_strategy(&c.strategy, &pairs, &c.weights);

        Ok(NodeOutput {
            output: serde_json::json!({
                "strategy": c.strategy,
                "source_count": pairs.len(),
                "wait_for_all": c.wait_for_all,
                "result": aggregated,
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
