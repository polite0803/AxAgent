// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct SwitchExecutor;

impl SwitchExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SwitchExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析点号分隔路径，从 ExecutionState.variables 提取目标值。
/// 空路径直接返回 None；segments 中间值非对象也返回 None。
fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    super::resolve_var_path(path, &context.variables)
}

/// 将 serde_json::Value 转为 Rhai 兼容的字面量表达式。
/// - String → `"value"` (JSON 序列化自带引号和转义)
/// - Number → 原样数字
/// - Bool → true/false
/// - Null → "()" (Rhai 的 unit)
/// - Array → `[v1, v2, ...]` (递归转换)
/// - Object → `#{k1: v1, k2: v2}` (Rhai 对象映射语法)
fn json_to_rhai_literal(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => {
            // 使用 to_string 获得 JSON 字符串格式（带双引号和转义）
            serde_json::Value::String(s.clone()).to_string()
        },
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "()".to_string(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(json_to_rhai_literal).collect();
            format!("[{}]", items.join(", "))
        },
        serde_json::Value::Object(obj) => {
            let items: Vec<String> = obj
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}: {}",
                        json_to_rhai_literal(&serde_json::Value::String(k.clone())),
                        json_to_rhai_literal(v)
                    )
                })
                .collect();
            format!("#{{{}}}", items.join(", "))
        },
    }
}

#[async_trait]
impl NodeExecutorTrait for SwitchExecutor {
    fn node_type(&self) -> &'static str {
        "switch"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Switch(n) = node else {
            return Err(NodeError::type_mismatch("switch", self.node_type()));
        };
        let c = &n.config;

        // 1. 取 input_var 的实际值
        let actual = resolve_var_path(&c.input_var, context);
        let actual_str = actual.as_ref().map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => String::new(),
            other => other.to_string(),
        });

        // 2. 确定匹配的 case（matched_label）
        let matched_label: Option<String> = match actual.as_ref() {
            None => c.default_case.clone(),
            Some(actual_val) => {
                let mut found: Option<String> = None;

                // ── expression 模式：用 Rhai 表达式匹配 ──
                if c.match_mode == "expression" {
                    // 将 serde_json::Value 转为 Rhai 兼容字面量
                    let rhai_value = json_to_rhai_literal(actual_val);
                    for case in &c.cases {
                        let expr = &case.value;
                        if expr.is_empty() {
                            continue;
                        }
                        // 构造 Rhai 脚本：将实际值赋给 _value 变量，执行表达式
                        let script = format!("let _value = {}; {}", rhai_value, expr);
                        let mut e = rhai::Engine::new();
                        e.set_max_operations(10_000);
                        e.set_max_call_levels(8);
                        e.set_max_string_size(64_000);
                        e.set_max_array_size(1_000);
                        match e.eval::<bool>(&script) {
                            Ok(true) => {
                                found = Some(case.label.clone());
                                break;
                            },
                            Ok(false) => continue,
                            Err(e) => {
                                tracing::warn!(
                                    "[SwitchExecutor] case '{}' expression 求值失败 (script: {script:?}): {e}",
                                    case.label,
                                );
                                continue;
                            },
                        }
                    }
                    found.or_else(|| c.default_case.clone())
                }
                // ── use_llm 模式：用 LLM 判断 ──
                else if c.use_llm.unwrap_or(false) {
                    // LLM 模式需要 context 中有 chat_adapter 或类似机制。
                    // 当前暂不支持同步 LLM 调用；返回 first match 并标记为 LLM pending。
                    // TODO: 集成 LLM 调用（复用 LlmClassifier 的 LLM 调用基础设施）
                    tracing::warn!("[SwitchExecutor] LLM 路由暂未支持同步调用，回退到默认分支");
                    c.default_case.clone()
                }
                // ── 传统模式：exact / regex / contains ──
                else {
                    let needle = match actual_str.as_deref() {
                        Some(s) => s,
                        None => return Ok(Self::build_output(c, actual, c.default_case.clone())),
                    };
                    for case in &c.cases {
                        let hit = match c.match_mode.as_str() {
                            "regex" => match regex::Regex::new(&case.value) {
                                Ok(re) => re.is_match(needle),
                                Err(e) => {
                                    tracing::warn!(
                                        "[SwitchExecutor] case '{}' regex 编译失败: {e}",
                                        case.label
                                    );
                                    false
                                },
                            },
                            "contains" => needle.contains(&case.value),
                            _ => needle == case.value,
                        };
                        if hit {
                            found = Some(case.label.clone());
                            break;
                        }
                    }
                    found.or_else(|| c.default_case.clone())
                }
            },
        };

        Ok(Self::build_output(c, actual, matched_label))
    }
}

impl SwitchExecutor {
    fn build_output(
        c: &axagent_core::workflow_types::SwitchNodeConfig,
        actual: Option<serde_json::Value>,
        matched_label: Option<String>,
    ) -> NodeOutput {
        NodeOutput {
            output: serde_json::json!({
                "input_var": c.input_var,
                "actual_value": actual,
                "matched_label": matched_label,
                "case_count": c.cases.len(),
                "match_mode": c.match_mode,
                "node_id": "",
                "use_llm": c.use_llm.unwrap_or(false),
            }),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        }
    }
}
