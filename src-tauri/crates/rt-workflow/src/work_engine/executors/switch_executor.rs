// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use std::sync::Arc;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

pub struct SwitchExecutor {
    // SwitchExecutor 在 use_llm 模式下需要执行 LLM 调用，需要 master_key 解密 provider key；
    // 同时保持与 AgentExecutor / LlmExecutor / ConditionExecutor 等兄弟 executor
    // 的构造接口一致(均由 WorkEngine::new 统一注入 master_key)。
    master_key: [u8; 32],
    /// 由 Harness 注入的 ProviderRegistry（运行时按 provider 类型查找 adapter）
    provider_registry: Option<Arc<dyn axagent_harness::registry::ProviderRegistry>>,
}

impl SwitchExecutor {
    pub fn new(master_key: [u8; 32]) -> Self {
        Self { master_key, provider_registry: None }
    }
}

impl Default for SwitchExecutor {
    fn default() -> Self {
        Self::new([0u8; 32])
    }
}

impl axagent_harness::HasProviderRegistry for SwitchExecutor {
    fn set_provider_registry(
        &mut self,
        registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) {
        self.provider_registry = Some(registry);
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
                    // 构造 LLM 路由 prompt：列出所有 case label + default，让 LLM 选最匹配的 label
                    let input_text = actual_str.as_deref().unwrap_or("");
                    let cases_list = c
                        .cases
                        .iter()
                        .enumerate()
                        .map(|(i, case)| format!("{}. {}", i + 1, case.label))
                        .collect::<Vec<_>>()
                        .join("\n");

                    let default_label = c.default_case.as_deref().unwrap_or("(none)");
                    let prompt = if let Some(ref custom_prompt) = c.llm_prompt {
                        format!(
                            "{custom_prompt}\n\n\
                             ## 可选分支（请输出最匹配的分支 label，仅输出 label 文本）\n{cases_list}\n\
                             ## 默认分支\n{default_label}\n\n\
                             ## 输入文本\n{input_text}\n\n\
                             请仅输出最匹配的分支 label 文本，不要包含任何其他内容。",
                        )
                    } else {
                        format!(
                            "你是一个路由判断器。请根据输入文本，选择最匹配的分支。\n\n\
                             ## 可选分支（请输出最匹配的分支 label，仅输出 label 文本）\n{cases_list}\n\
                             ## 默认分支\n{default_label}\n\n\
                             ## 输入文本\n{input_text}\n\n\
                             请仅输出最匹配的分支 label 文本，不要包含任何其他内容。",
                        )
                    };

                    let node_model = c.llm_model.as_deref().filter(|m| !m.is_empty());
                    let session_model =
                        context.variables.get(super::WORKFLOW_MODEL_VAR).and_then(|v| v.as_str());
                    let session_provider_id = context
                        .variables
                        .get(super::WORKFLOW_PROVIDER_ID_VAR)
                        .and_then(|v| v.as_str());

                    let (prov, key, model, adapter, api_key) = super::resolve_provider_and_adapter(
                        &self.master_key,
                        self.provider_registry.as_ref(),
                        node_model,
                        session_model,
                        session_provider_id,
                        None,
                        "SwitchExecutor",
                    )
                    .await?;

                    if context.dry_run {
                        tracing::info!("[SwitchExecutor] dry_run 模式：LLM 路由短路返回首个 case");
                        found = c
                            .cases
                            .first()
                            .map(|case| case.label.clone())
                            .or_else(|| c.default_case.clone());
                    } else {
                        use axagent_harness::build_provider_request_context;
                        use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};

                        let req_ctx = build_provider_request_context(&prov, &key, api_key);
                        let request = ChatRequest {
                            model: model.clone(),
                            messages: vec![ChatMessage {
                                role: "user".to_string(),
                                content: ChatContent::Text(prompt),
                                tool_calls: None,
                                tool_call_id: None,
                                thinking: None,
                            }],
                            stream: false,
                            temperature: Some(0.0),
                            max_tokens: Some(64),
                            top_p: None,
                            tools: None,
                            thinking_budget: None,
                            use_max_completion_tokens: None,
                            thinking_param_style: None,
                            api_mode: None,
                            instructions: None,
                            conversation: None,
                            previous_response_id: None,
                            store: None,
                            response_format: None,
                        };

                        let llm_config = axagent_harness::LlmCallConfig::default();
                        let response =
                            axagent_harness::execute_llm(&*adapter, &req_ctx, request, &llm_config)
                                .await
                                .map_err(|e| {
                                    NodeError::exec_failed(
                                        error_code::UNSUPPORTED_PROVIDER,
                                        format!("Switch LLM routing call failed: {e}"),
                                    )
                                })?;

                        let raw_label = response.response.content.trim();
                        // 优先精确匹配 case label；未命中则尝试包含匹配；最后 fallback 到 default
                        let matched = c
                            .cases
                            .iter()
                            .find(|case| case.label == raw_label)
                            .or_else(|| c.cases.iter().find(|case| raw_label.contains(&case.label)))
                            .map(|case| case.label.clone());

                        if let Some(label) = matched {
                            tracing::info!(
                                "[SwitchExecutor] LLM 路由命中 case '{}' (raw: {raw_label:?})",
                                label
                            );
                            found = Some(label);
                        } else {
                            tracing::warn!(
                                "[SwitchExecutor] LLM 路由未命中任何 case (raw: {raw_label:?})，回退到默认分支"
                            );
                            found = c.default_case.clone();
                        }
                    }

                    found
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
        c: &axagent_harness::workflow_types::SwitchNodeConfig,
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
            control: None,
        }
    }
}
