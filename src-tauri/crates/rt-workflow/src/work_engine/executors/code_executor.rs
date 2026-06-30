// SPDX-License-Identifier: AGPL-3.0-only

//! 代码执行器 —— 执行 CodeNode 中的代码片段。
//!
//! 支持两种模式：
//! - `execute_directly = false`（默认）：Rhai 脚本注册为工具，由 Agent/LLM 调用
//! - `execute_directly = true`：在 DAG 中直接执行 Rhai 代码，通过 input_mapping
//!   从 context.variables 读取结构化参数，输出 JSON 结果

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use serde_json::Value;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

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

/// 执行 Rhai 脚本的 in-process 引擎。
/// 通过 `input_mapping` 从 context.variables 读取注入值为数字/字符串，
/// 并通过 Rhai 的 `Scope` 传递给脚本，执行后收集结果构造 JSON 输出。
///
/// Phase 5: 返回 (script_result, input_params_snapshot) 二元组。
/// input_params_snapshot 是所有 input_mapping 解析值的快照，
/// 用于 What-If 回测 UI 读取原始参数值。
fn execute_rhai_directly(
    code: &str,
    input_mapping: &std::collections::HashMap<String, String>,
    context: &ExecutionState,
) -> Result<(serde_json::Value, serde_json::Value), NodeError> {
    use rhai::{Engine, Scope};

    let mut engine = Engine::new();
    // SECURITY (C4): Rhai 沙箱限制 — 防 DoS
    engine.set_max_operations(200_000);
    engine.set_max_call_levels(32);
    engine.set_max_modules(0);
    engine.set_max_string_size(2_000_000);
    engine.set_max_array_size(50_000);
    // 提升表达式复杂度上限：瓶颈计算脚本有复杂嵌套 map + 大量条件判断链
    // 默认 max_expr_depths(128,128) 在某些 Rhai 版本中对长 if 链 + map 字面量不够
    engine.set_max_expr_depths(1024, 1024);
    // Rhai 无内建 clamp，portfolio-mgr.rhai 等脚本依赖
    engine.register_fn("clamp", |value: f64, min: f64, max: f64| -> f64 {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    });
    // Rhai 原生 Array 无 join 方法，portfolio-mgr.rhai 中 data_gaps.join(", ") 依赖此函数
    engine.register_fn("join", |arr: rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });
    // V48: JSON 字符串解析 — bottleneck-calc.rhai 需要解析 Agent tool_call 输出中的
    // 嵌套 JSON（如 arguments.content 是 JSON 字符串），Rhai 原生不支持 JSON 解析。
    engine.register_fn("json_parse", |s: &str| -> rhai::Dynamic {
        match serde_json::from_str::<serde_json::Value>(s) {
            Ok(v) => json_value_to_dynamic(&v),
            Err(e) => {
                tracing::warn!(
                    "[code_executor] json_parse 失败: {e}, input={}",
                    &s[..s.len().min(200)]
                );
                rhai::Dynamic::UNIT
            },
        }
    });
    let mut scope = Scope::new();
    let mut input_params_snapshot = serde_json::Map::new();

    // V49 诊断：input_mapping 是否为空（空则所有变量丢失）
    tracing::error!(
        "[code_executor V49] input_mapping entries={}, keys={:?}",
        input_mapping.len(),
        input_mapping.keys().collect::<Vec<_>>()
    );

    // 将 input_mapping 的值注入 Rhai scope
    for (target_key, source_key) in input_mapping {
        let value = super::resolve_var_path(source_key, &context.variables);
        // 记录解析值的快照（Phase 5: What-If 回测参数持久化）
        let snapshot_value = value.clone().unwrap_or(Value::Null);
        input_params_snapshot.insert(target_key.clone(), snapshot_value);
        // V49: 统一转为 Dynamic 再 push_constant，避免 push_dynamic 在 v1.25 中静默失败
        let dyn_val = match &value {
            Some(Value::Null) | None => rhai::Dynamic::UNIT,
            Some(Value::Bool(b)) => rhai::Dynamic::from(*b),
            Some(Value::Number(n)) => {
                if let Some(f) = n.as_f64() {
                    rhai::Dynamic::from(f)
                } else if let Some(i) = n.as_i64() {
                    rhai::Dynamic::from(i as f64)
                } else if let Some(u) = n.as_u64() {
                    rhai::Dynamic::from(u as f64)
                } else {
                    rhai::Dynamic::from(0.0_f64)
                }
            },
            Some(Value::String(s)) => rhai::Dynamic::from(s.clone()),
            Some(v) => json_value_to_dynamic(v),
        };
        scope.push_constant(target_key.as_str(), dyn_val);
    }
    // V29 诊断：记录所有 input_mapping resolve 结果，精确定位哪个变量解析失败
    tracing::warn!(
        "[code_executor] input_mapping snapshot: {}",
        serde_json::to_string(&input_params_snapshot).unwrap_or_default()
    );

    // 执行脚本，期望返回一个 map
    let result: rhai::Dynamic = engine.eval_with_scope(&mut scope, code).map_err(|e| {
        tracing::error!(error = %e, "Rhai 执行失败");
        NodeError::exec_failed(error_code::VALIDATION_FAILED, format!("Rhai execution failed: {e}"))
    })?;

    // 将 Rhai 结果转换回 JSON
    Ok((dynamic_to_json_value(&result), Value::Object(input_params_snapshot)))
}

/// 将 serde_json::Value 转换为 Rhai Dynamic
fn json_value_to_dynamic(v: &Value) -> rhai::Dynamic {
    match v {
        Value::Null => rhai::Dynamic::UNIT,
        Value::Bool(b) => rhai::Dynamic::from(*b),
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                rhai::Dynamic::from(f)
            } else if let Some(i) = n.as_i64() {
                rhai::Dynamic::from(i as f64)
            } else {
                rhai::Dynamic::from(0.0_f64)
            }
        },
        Value::String(s) => rhai::Dynamic::from(s.clone()),
        Value::Array(arr) => {
            let items: rhai::Array = arr.iter().map(json_value_to_dynamic).collect();
            rhai::Dynamic::from(items)
        },
        Value::Object(obj) => {
            let mut map = rhai::Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_value_to_dynamic(v));
            }
            rhai::Dynamic::from(map)
        },
    }
}

/// 将 Rhai Dynamic 转换回 serde_json::Value
fn dynamic_to_json_value(v: &rhai::Dynamic) -> Value {
    if v.is_unit() {
        return Value::Null;
    }
    if v.is_bool() {
        return Value::Bool(v.as_bool().unwrap_or(false));
    }
    if let Ok(s) = v.clone().into_string() {
        return Value::String(s);
    }
    if let Ok(i) = v.as_int() {
        return Value::Number(serde_json::Number::from(i));
    }
    if let Ok(f) = v.as_float() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
        return Value::Number(serde_json::Number::from(0));
    }
    // Array
    if let Some(arr) = v.clone().try_cast::<rhai::Array>() {
        return Value::Array(
            arr.into_iter()
                .map(|item| dynamic_to_json_value(&item))
                .collect(),
        );
    }
    // Map
    if let Some(map) = v.clone().try_cast::<rhai::Map>() {
        let mut obj = serde_json::Map::new();
        for (k, val) in &map {
            obj.insert(format!("{k}"), dynamic_to_json_value(val));
        }
        return Value::Object(obj);
    }
    Value::String(format!("{v}"))
}

#[async_trait]
impl NodeExecutorTrait for CodeExecutor {
    fn node_type(&self) -> &'static str {
        "code"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Code(code_node) = node else {
            return Err(NodeError::type_mismatch(
                "code".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        // ── 直接执行模式（execute_directly=true）──
        // Rhai 脚本在 DAG 中直接执行，通过 input_mapping 消费上游结构化参数。
        if code_node.config.execute_directly && code_node.config.language == "rhai" {
            tracing::warn!(
                "[code_executor] Rhai execution: node_id={}, input_mapping keys={:?}, variables keys count={}, has_t_scoring={}, has_debate_convergence={}, has_a_catalyst={}, has_raw_data={}, sample_keys={:?}, totalScore resolve={:?}, consensusScore resolve={:?}, catalyst_level resolve={:?}",
                code_node.base.id,
                code_node.config.input_mapping.keys().collect::<Vec<_>>(),
                context.variables.keys().count(),
                context.variables.contains_key("t-scoring"),
                context.variables.contains_key("debate-convergence"),
                context.variables.contains_key("a-catalyst"),
                context.variables.contains_key("raw-data"),
                context.variables.keys().take(10).collect::<Vec<_>>(),
                super::resolve_var_path("t-scoring.result.totalScore", &context.variables),
                super::resolve_var_path(
                    "debate-convergence.content.consensus_score",
                    &context.variables
                ),
                super::resolve_var_path("a-catalyst.content.catalyst_level", &context.variables),
            );
            let (result, input_params) = execute_rhai_directly(
                &code_node.config.code,
                &code_node.config.input_mapping,
                context,
            )?;
            // Phase 5: 将 input_mapping 解析值快照嵌入 output.input_params，
            // 确保 What-If 回测 UI 可直接读取原始参数值，无需从上游节点重建。
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "executed",
                    "language": "rhai",
                    "result": result,
                    "input_params": input_params,
                    "node_id": node.base_id(),
                    // 将 result 中的关键决策字段提升到 params 层，供下游 resolve_var_path 消费
                    "params": result,
                }),
                output_var: Some(code_node.config.output_var.clone()),
            });
        }

        // ── 工具注册模式（向后兼容）──
        // Rhai 脚本已在预处理阶段编译并注册为工具，DAG 中无需执行
        if code_node.config.language == "rhai" {
            let tool_name = code_node
                .config
                .tool_name
                .clone()
                .unwrap_or_else(|| format!("code_{}", code_node.base.id));
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "tool_registered",
                    "tool_name": tool_name,
                    "note": "Rhai 脚本已注册为工具，由 Agent/LLM 调用，无需 DAG 执行",
                    "node_id": node.base_id(),
                }),
                output_var: Some(code_node.config.output_var.clone()),
            });
        }

        // 非 Rhai 语言：返回代码摘要供 LLM 或下游节点使用
        let code_lines = code_node.config.code.lines().count();
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "code_ready",
                "language": code_node.config.language,
                "code_lines": code_lines,
                // V37 修复: 按 char 边界取前缀，避免 .len().min(500) 落在多字节 UTF-8
                // 字符中间导致 panic
                "code_preview": code_node.config.code.chars().take(500).collect::<String>(),
                "node_id": node.base_id(),
            }),
            output_var: Some(code_node.config.output_var.clone()),
        })
    }
}
