// SPDX-License-Identifier: AGPL-3.0-only

//! 代码执行器 —— 执行 CodeNode 中的代码片段。
//!
//! 支持两种模式：
//! - `execute_directly = false`（默认）：Rhai 脚本注册为工具，由 Agent/LLM 调用
//! - `execute_directly = true`：在 DAG 中直接执行 Rhai 代码，通过 input_mapping
//!   从 context.variables 读取结构化参数，输出 JSON 结果

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use rhai::Engine;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

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

/// 共享 Rhai Engine 单例（池化 + 复用），避免每次执行重复分配与初始化。
fn shared_rhai_engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut engine = Engine::new();
        // SECURITY (C4): Rhai 沙箱限制 — 防 DoS
        engine.set_max_operations(200_000);
        engine.set_max_call_levels(32);
        engine.set_max_modules(0);
        engine.set_max_string_size(2_000_000);
        engine.set_max_array_size(50_000);
        engine.set_max_expr_depths(1024, 1024);
        engine.register_fn("clamp", |value: f64, min: f64, max: f64| -> f64 {
            if value < min {
                min
            } else if value > max {
                max
            } else {
                value
            }
        });
        engine.register_fn("join", |arr: rhai::Array, sep: &str| -> String {
            arr.iter().map(|item| item.to_string()).collect::<Vec<_>>().join(sep)
        });
        engine.register_fn("json_parse", |s: &str| -> rhai::Dynamic {
            match serde_json::from_str::<serde_json::Value>(s) {
                Ok(v) => json_value_to_dynamic(&v),
                Err(e) => {
                    tracing::warn!("[code_executor] json_parse 失败: {e}");
                    rhai::Dynamic::UNIT
                },
            }
        });
        engine
    })
}

/// 执行 Rhai 脚本的 in-process 引擎。
/// 通过 `input_mapping` 从 context.variables 读取注入值为数字/字符串，
/// 并通过 Rhai 的 `Scope` 传递给脚本，执行后收集结果构造 JSON 输出。
///
/// Phase 5: 返回 (script_result, input_params_snapshot) 二元组。
/// input_params_snapshot 是所有 input_mapping 解析值的快照，
/// 用于 What-If 回测 UI 读取原始参数值。
async fn execute_rhai_directly(
    code: &str,
    input_mapping: &std::collections::HashMap<String, String>,
    context: &ExecutionState,
) -> Result<(serde_json::Value, serde_json::Value), NodeError> {
    let mut input_params_snapshot = serde_json::Map::new();

    // V49 诊断：input_mapping 是否为空（空则所有变量丢失）
    tracing::debug!(
        "[code_executor V49] input_mapping entries={}, keys={:?}",
        input_mapping.len(),
        input_mapping.keys().collect::<Vec<_>>()
    );

    // 将 input_mapping 的值注入 Rhai scope
    let mut scope_vars: HashMap<String, rhai::Dynamic> = HashMap::new();
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
        scope_vars.insert(target_key.clone(), dyn_val);
    }
    // V29 诊断：记录所有 input_mapping resolve 结果，精确定位哪个变量解析失败
    tracing::warn!(
        "[code_executor] input_mapping snapshot: {}",
        serde_json::to_string(&input_params_snapshot).unwrap_or_default()
    );

    // 执行脚本，期望返回一个 map
    // scope_vars 已从 input_mapping 直接构建为 HashMap，避免 Rhai Scope 的 Send 限制。
    let code_owned = code.to_string();
    let join = tokio::task::spawn_blocking(move || {
        let engine = shared_rhai_engine();
        let mut scope = rhai::Scope::new();
        for (k, v) in scope_vars {
            scope.push_constant(k, v);
        }
        engine.eval_expression::<rhai::Dynamic>(&code_owned).map_err(|e| e.to_string())
    });
    let result: rhai::Dynamic = match tokio::time::timeout(
        std::time::Duration::from_secs(30), // P2-18: 30s 硬上限
        join,
    )
    .await
    {
        Ok(Ok(Ok(v))) => v,
        Ok(Ok(Err(e))) => {
            tracing::error!(error = %e, "Rhai 执行失败");
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                format!("Rhai execution failed: {e}"),
            ));
        },
        Ok(Err(join_err)) => {
            tracing::error!(error = %join_err, "Rhai 任务被取消");
            return Err(NodeError::exec_failed(
                error_code::TIMEOUT,
                "Rhai task cancelled".to_string(),
            ));
        },
        Err(_elapsed) => {
            tracing::error!("Rhai 执行超时（30s）—— 强制终止");
            return Err(NodeError::exec_failed(
                error_code::TIMEOUT,
                "Rhai execution exceeded 30s timeout".to_string(),
            ));
        },
    };

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
        return Value::Array(arr.into_iter().map(|item| dynamic_to_json_value(&item)).collect());
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
            )
            .await?;
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
                control: None,
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
                control: None,
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
            control: None,
        })
    }
}
