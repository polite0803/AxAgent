// SPDX-License-Identifier: AGPL-3.0-only

//! 校验执行器 —— 根据 ValidationNodeConfig 执行断言校验。
//!
//! 支持的断言类型：
//! - `json_schema`：用 JSON Schema 校验 actual
//! - `not_null`：actual 非 null
//! - `non_empty`：actual 非空字符串/数组
//! - `contains`：actual 字符串包含 expected 子串
//! - `expression`：任意 Rhai 布尔表达式（用 ExecutionState.variables 的裸名 + $vars/$node/$input/$now/$env 求值）

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use chrono::{Datelike, Timelike, Utc};
use rhai::{Dynamic, Engine, Map, Scope};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct ValidationExecutor;

impl ValidationExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ValidationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for ValidationExecutor {
    fn node_type(&self) -> &'static str {
        "validation"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Validation(validation_node) = node else {
            return Err(NodeError::type_mismatch(
                "validation".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        let mut results = Vec::new();
        let mut all_passed = true;

        // expression 类型需要预构造 Rhai scope（避免每条断言都重新构造）
        let rhai_engine = Engine::new();
        let mut rhai_scope = Self::build_rhai_scope(context);

        for assertion in &validation_node.config.assertions {
            let passed = match assertion.assertion_type.as_str() {
                "expression" => {
                    let expr = assertion.expression.as_deref().unwrap_or("true");
                    match Self::eval_rhai_bool(&rhai_engine, &mut rhai_scope, expr) {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::warn!(
                                assertion_type = "expression",
                                expression = %expr,
                                error = %e,
                                "[ValidationExecutor] Rhai 表达式求值失败，记为 false"
                            );
                            false
                        },
                    }
                },
                "json_schema" => {
                    let actual_value = match &assertion.actual {
                        Some(path) => resolve_var_path(path, context),
                        None => None,
                    };
                    let expected_value = match &assertion.expected {
                        Some(schema_json) => {
                            serde_json::from_str::<serde_json::Value>(schema_json).ok()
                        },
                        None => None,
                    };
                    if let (Some(expected), Some(actual)) = (&expected_value, &actual_value) {
                        let (valid, _) = axagent_kit::schema_validator::validate_against_schema(
                            actual, expected,
                        );
                        valid
                    } else {
                        false
                    }
                },
                "not_null" => {
                    let actual_value = match &assertion.actual {
                        Some(path) => resolve_var_path(path, context),
                        None => None,
                    };
                    actual_value.as_ref().is_some_and(|v| !v.is_null())
                },
                "non_empty" => {
                    let actual_value = match &assertion.actual {
                        Some(path) => resolve_var_path(path, context),
                        None => None,
                    };
                    actual_value.as_ref().is_some_and(|v| {
                        v.as_str().is_some_and(|s| !s.is_empty())
                            || v.as_array().is_some_and(|a| !a.is_empty())
                    })
                },
                "contains" => {
                    let actual_value = match &assertion.actual {
                        Some(path) => resolve_var_path(path, context),
                        None => None,
                    };
                    let expected_value = match &assertion.expected {
                        Some(schema_json) => {
                            serde_json::from_str::<serde_json::Value>(schema_json).ok()
                        },
                        None => None,
                    };
                    if let (Some(actual), Some(expected)) = (&actual_value, &expected_value) {
                        actual.as_str().zip(expected.as_str()).is_some_and(|(a, e)| a.contains(e))
                    } else {
                        false
                    }
                },
                _ => {
                    // 未知断言类型：记为 false（更严格），避免静默跳过掩盖问题
                    tracing::warn!(
                        assertion_type = %assertion.assertion_type,
                        "[ValidationExecutor] 未知断言类型，记为 false"
                    );
                    false
                },
            };

            results.push(serde_json::json!({
                "assertion_type": assertion.assertion_type,
                "passed": passed,
            }));
            if !passed {
                all_passed = false;
            }
        }

        let on_fail = &validation_node.config.on_fail;
        if !all_passed && on_fail == "abort" {
            return Err(NodeError::Validation(format!(
                "Validation failed: {}",
                serde_json::to_string(&results).unwrap_or_default()
            )));
        }

        Ok(NodeOutput {
            output: serde_json::json!({
                "status": if all_passed { "validated" } else { "validation_failed" },
                "valid": all_passed,
                "results": results,
                "node_id": node.base_id(),
            }),
            output_var: None,
            control: None,
        })
    }
}

impl ValidationExecutor {
    /// 用 ExecutionState 构造 Rhai scope。
    ///
    /// 注入规则（全部同时存在，供灵活选择）：
    /// 1. **裸名顶层变量**：`variables` 的每个 key 直接推入 scope（如 `l1_result`、`user_input`），
    ///    表达式可直接写 `l1_result.category != ()`，最简洁。
    /// 2. **`$vars`**：整包 variables 作为 Map 推入（如 `$vars.l1_result.category`），与
    ///    `resolve_expression` 风格一致。
    /// 3. **`$node`**：已完成节点输出（`ExecutionState.node_outputs`），如 `$node["call_l1"].output`。
    /// 4. **`$input`**：当前节点输入（`ExecutionState.input_params`）。
    /// 5. **`$now`**：当前时间 Map（timestamp/iso/year/month/day/hour/minute）。
    /// 6. **`$env`**：环境变量 Map。
    fn build_rhai_scope(context: &ExecutionState) -> Scope<'static> {
        let mut scope = Scope::new();

        // 裸名顶层变量 + $vars 整包
        let mut vars_map = Map::new();
        for (k, v) in &context.variables {
            let dyn_val = value_to_dynamic(v);
            // 同时注入裸名和 vars_map
            scope.push_dynamic(k.clone(), dyn_val.clone());
            vars_map.insert(k.clone().into(), dyn_val);
        }
        scope.push_dynamic("$vars".to_string(), Dynamic::from_map(vars_map));

        // $node：已完成节点输出
        let mut node_map = Map::new();
        for (k, v) in &context.node_outputs {
            node_map.insert(k.clone().into(), value_to_dynamic(v));
        }
        scope.push_dynamic("$node".to_string(), Dynamic::from_map(node_map));

        // $input
        scope.push_dynamic("$input".to_string(), value_to_dynamic(&context.input_params));

        // $now
        let now = Utc::now();
        let mut now_map = Map::new();
        now_map.insert("timestamp".into(), Dynamic::from_int(now.timestamp()));
        now_map.insert("iso".into(), Dynamic::from(now.to_rfc3339()));
        now_map.insert("year".into(), Dynamic::from_int(now.year() as i64));
        now_map.insert("month".into(), Dynamic::from_int(now.month() as i64));
        now_map.insert("day".into(), Dynamic::from_int(now.day() as i64));
        now_map.insert("hour".into(), Dynamic::from_int(now.hour() as i64));
        now_map.insert("minute".into(), Dynamic::from_int(now.minute() as i64));
        scope.push_dynamic("$now".to_string(), Dynamic::from_map(now_map));

        // $env
        let mut env_map = Map::new();
        for (k, v) in std::env::vars() {
            env_map.insert(k.into(), Dynamic::from(v));
        }
        scope.push_dynamic("$env".to_string(), Dynamic::from_map(env_map));

        scope
    }

    /// 把 Rhai Dynamic 转换为 bool（断言表达式的返回值）。
    /// 规则同 `resolve_loop_condition`：bool 原值；数字非零 true；字符串非空 true；其他 false。
    fn dynamic_to_bool(d: Dynamic) -> bool {
        if d.is::<bool>() {
            d.try_cast().unwrap_or(false)
        } else if d.is::<i64>() {
            d.try_cast::<i64>().unwrap_or(0) != 0
        } else if d.is::<f64>() {
            d.try_cast::<f64>().unwrap_or(0.0) != 0.0
        } else if d.is::<String>() {
            !d.try_cast::<String>().unwrap_or_default().is_empty()
        } else {
            false
        }
    }

    /// 编译并执行 Rhai 表达式，返回 bool 结果。
    fn eval_rhai_bool(
        engine: &Engine,
        scope: &mut Scope<'static>,
        expr: &str,
    ) -> Result<bool, String> {
        let ast = engine.compile_expression(expr).map_err(|e| format!("编译失败: {e}"))?;
        let result: Dynamic =
            engine.eval_ast_with_scope(scope, &ast).map_err(|e| format!("执行失败: {e}"))?;
        Ok(Self::dynamic_to_bool(result))
    }
}

/// serde_json::Value → Rhai Dynamic（本地副本，避免对 expression_engine 增加跨模块依赖）
fn value_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from_bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from_int(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from_float(f)
            } else {
                Dynamic::from(n.to_string())
            }
        },
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: Vec<Dynamic> = arr.iter().map(value_to_dynamic).collect();
            Dynamic::from_array(items)
        },
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), value_to_dynamic(v));
            }
            Dynamic::from_map(map)
        },
    }
}

/// 从 ExecutionState 变量中解析点分隔路径（如 "result.text" → variables["result"]["text"]）。
fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let root = context.variables.get(parts[0])?.clone();
    let mut current = root;
    for part in &parts[1..] {
        current = current.get(part)?.clone();
    }
    Some(current)
}
