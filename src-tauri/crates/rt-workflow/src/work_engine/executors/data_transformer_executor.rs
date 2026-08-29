// SPDX-License-Identifier: AGPL-3.0-only

//! DataTransformer executor — transforms workflow variables using Rhai
//! expressions.
//!
//! Reads the `input_var` from `ExecutionState.variables`, injects it into a
//! Rhai scope as `input`, evaluates `expression`, and writes the result to
//! `output_var`.

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct DataTransformerExecutor;

impl DataTransformerExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DataTransformerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a serde_json::Value into a Rhai Dynamic.
fn json_to_dynamic(value: &serde_json::Value) -> rhai::Dynamic {
    match value {
        serde_json::Value::Null => rhai::Dynamic::UNIT,
        serde_json::Value::Bool(b) => rhai::Dynamic::from_bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rhai::Dynamic::from_int(i)
            } else if let Some(f) = n.as_f64() {
                rhai::Dynamic::from_float(f)
            } else {
                rhai::Dynamic::UNIT
            }
        },
        serde_json::Value::String(s) => rhai::Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let mut dyn_arr = rhai::Array::with_capacity(arr.len());
            for item in arr {
                dyn_arr.push(json_to_dynamic(item));
            }
            rhai::Dynamic::from_array(dyn_arr)
        },
        serde_json::Value::Object(map) => {
            let mut dyn_map = rhai::Map::new();
            for (k, v) in map {
                dyn_map.insert(k.clone().into(), json_to_dynamic(v));
            }
            rhai::Dynamic::from_map(dyn_map)
        },
    }
}

/// Convert a Rhai Dynamic back to serde_json::Value.
fn dynamic_to_json(value: rhai::Dynamic) -> serde_json::Value {
    if value.is::<rhai::Map>() {
        let map = value.cast::<rhai::Map>();
        let mut obj = serde_json::Map::new();
        for (k, v) in map {
            obj.insert(k.to_string(), dynamic_to_json(v));
        }
        serde_json::Value::Object(obj)
    } else if value.is::<rhai::Array>() {
        let arr = value.cast::<rhai::Array>();
        let mut json_arr = Vec::with_capacity(arr.len());
        for v in arr {
            json_arr.push(dynamic_to_json(v));
        }
        serde_json::Value::Array(json_arr)
    } else if value.is::<i64>() {
        serde_json::json!(value.as_int().unwrap_or(0))
    } else if value.is::<f64>() {
        serde_json::json!(value.as_float().unwrap_or(0.0))
    } else if value.is::<bool>() {
        serde_json::json!(value.as_bool().unwrap_or(false))
    } else if value.is::<String>() {
        serde_json::json!(value.into_string().unwrap_or_default())
    } else if value.is_unit() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(value.to_string())
    }
}

/// 判断字符串是否为合法的 Rhai 标识符（字母/数字/下划线，且不以数字开头）。
fn is_valid_rhai_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        },
        _ => false,
    }
}

/// Rhai 保留关键字集合 —— 命中则不注入 scope，避免表达式求值时语义冲突。
fn is_rhai_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "else"
            | "let"
            | "const"
            | "fn"
            | "loop"
            | "while"
            | "for"
            | "in"
            | "break"
            | "continue"
            | "return"
            | "throw"
            | "try"
            | "catch"
            | "switch"
            | "case"
            | "default"
            | "import"
            | "export"
            | "true"
            | "false"
            | "null"
            | "this"
            | "type"
            | "private"
            | "public"
            | "shared"
            | "do"
            | "spawn"
            | "thread"
            | "async"
            | "await"
            | "yield"
    )
}

#[async_trait]
impl NodeExecutorTrait for DataTransformerExecutor {
    fn node_type(&self) -> &'static str {
        "dataTransformer"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::DataTransformer(n) = node else {
            return Err(NodeError::type_mismatch("dataTransformer", self.node_type()));
        };
        let c = &n.config;

        if c.expression.trim().is_empty() {
            return Err(NodeError::exec_failed(
                "TRANSFORM_EXPRESSION_EMPTY",
                "expression is empty",
            ));
        }

        let engine = rhai::Engine::new();
        let mut scope = rhai::Scope::new();

        // Inject the input variable into scope
        let input_value = if c.input_var.is_empty() {
            serde_json::Value::Null
        } else {
            ctx.variables.get(&c.input_var).cloned().unwrap_or(serde_json::Value::Null)
        };
        scope.push("input", json_to_dynamic(&input_value));

        // Inject the remaining execution variables into scope so expressions can
        // reference multiple vars (e.g. L2 rule matching reads `l1_domain` +
        // `user_input`). Only valid Rhai identifiers and non-reserved keywords are
        // injected; `input`/input_var take precedence to preserve prior behavior.
        let mut push_scoped = |name: &str, value: &serde_json::Value| {
            if !is_valid_rhai_ident(name) || is_rhai_keyword(name) {
                return;
            }
            scope.push_dynamic(name.to_string(), json_to_dynamic(value));
        };
        push_scoped(&c.input_var, &input_value);
        for (k, v) in &ctx.variables {
            if k != &c.input_var {
                push_scoped(k, v);
            }
        }

        // 使用 eval_with_scope（script mode）替代 eval_expression_with_scope（expression mode）。
        // expression mode 仅支持单行表达式，不支持 `let` 多行语句块；
        // script mode 兼容多行脚本（let + 末尾表达式）和单行表达式两种写法。
        let result: rhai::Dynamic =
            engine.eval_with_scope::<rhai::Dynamic>(&mut scope, &c.expression).map_err(|e| {
                NodeError::exec_failed(
                    "TRANSFORM_EVAL_FAILED",
                    format!("Rhai evaluation error: {e}"),
                )
            })?;

        let output_value = dynamic_to_json(result);

        tracing::info!(
            expression = %c.expression,
            input_var = %c.input_var,
            output_var = %c.output_var,
            "DataTransformer: transformed"
        );

        Ok(NodeOutput {
            output: output_value,
            control: None,
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
