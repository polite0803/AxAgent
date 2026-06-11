// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

pub struct LlmClassifierExecutor {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
    /// 由 Harness 注入的 ProviderRegistry（运行时按 provider 类型查找 adapter）
    provider_registry: Option<Arc<dyn axagent_harness::registry::ProviderRegistry>>,
}

impl LlmClassifierExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self {
            db,
            master_key,
            provider_registry: None,
        }
    }
}

impl axagent_harness::HasProviderRegistry for LlmClassifierExecutor {
    fn set_provider_registry(
        &mut self,
        registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) {
        self.provider_registry = Some(registry);
    }
}

impl Default for LlmClassifierExecutor {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::default()),
            master_key: [0u8; 32],
            provider_registry: None,
        }
    }
}

#[async_trait]
impl NodeExecutorTrait for LlmClassifierExecutor {
    fn node_type(&self) -> &'static str {
        "llmClassifier"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::LlmClassifier(n) = node else {
            return Err(NodeError::type_mismatch(
                "llmClassifier".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };
        let c = &n.config;

        let input_text = if c.input_var.is_empty() {
            context
                .variables
                .iter()
                .filter(|(k, _)| !k.starts_with("__"))
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            resolve_var_path(&c.input_var, context)
                .map(value_to_input_text)
                .unwrap_or_default()
        };

        if input_text.is_empty() {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "LlmClassifier: input_var 指向的变量为空或不存在".to_string(),
            ));
        }

        let categories_list = c
            .categories
            .iter()
            .enumerate()
            .map(|(i, cat)| format!("{}. {}", i + 1, cat))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = if c.confidence_threshold.is_some() {
            format!(
                "你是一个文本分类器。请根据以下分类规则，将输入文本归入最匹配的类别。\n\n\
                 ## 分类规则\n{prompt_rule}\n\n\
                 ## 可选类别\n{categories_list}\n\n\
                 ## 输入文本\n{input_text}\n\n\
                 请用 JSON 格式输出，包含 label（类别名称）和 confidence（0.0-1.0 的置信度）。\
                 例如：{{\"label\": \"类别名\", \"confidence\": 0.95}}。\
                 只输出 JSON，不要包含任何其他内容。",
                prompt_rule = c.prompt,
                categories_list = categories_list,
                input_text = input_text,
            )
        } else {
            format!(
                "你是一个文本分类器。请根据以下分类规则，将输入文本归入最匹配的类别。\n\n\
                 ## 分类规则\n{prompt_rule}\n\n\
                 ## 可选类别\n{categories_list}\n\n\
                 ## 输入文本\n{input_text}\n\n\
                 请只输出最匹配的类别名称，不要包含任何其他内容。",
                prompt_rule = c.prompt,
                categories_list = categories_list,
                input_text = input_text,
            )
        };

        let node_model = c.model.as_deref().filter(|m| !m.is_empty());
        let session_model = context
            .variables
            .get(super::WORKFLOW_MODEL_VAR)
            .and_then(|v| v.as_str());
        let session_provider_id = context
            .variables
            .get(super::WORKFLOW_PROVIDER_ID_VAR)
            .and_then(|v| v.as_str());

        let (prov, key, model, adapter, api_key) = super::resolve_provider_and_adapter(
            &self.db,
            &self.master_key,
            self.provider_registry.as_ref(),
            node_model,
            session_model,
            session_provider_id,
            None,
            "LlmClassifierExecutor",
        )
        .await?;

        if context.dry_run {
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "category": c.categories.first().cloned().unwrap_or_default(),
                    "model": model,
                    "dry_run": true,
                    "node_id": node.base_id(),
                }),
                output_var: if c.output_var.is_empty() {
                    None
                } else {
                    Some(c.output_var.clone())
                },
            });
        }

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
        };

        let response = adapter.chat(&req_ctx, request.clone()).await.map_err(|e| {
            NodeError::exec_failed(
                error_code::UNSUPPORTED_PROVIDER,
                format!("LLM classifier call failed: {e}"),
            )
        })?;

        // ── 结果一致性检查 ──
        if let Some(ref cc_config) = c.consistency_check
            && cc_config.enabled
        {
            let secondary_request =
                if matches!(cc_config.mode, axagent_harness::ConsistencyMode::CrossModelCompare) {
                    let sec_model = cc_config.secondary_model.as_deref().unwrap_or(&model);
                    ChatRequest {
                        model: sec_model.to_string(),
                        messages: request.messages.clone(),
                        ..request.clone()
                    }
                } else {
                    request.clone()
                };
            let secondary_response = adapter.chat(&req_ctx, secondary_request).await;
            if let Ok(sec_resp) = secondary_response {
                use axagent_harness::consistency_check::check_consistency;
                let primary_val = serde_json::json!(response.content);
                let secondary_val = serde_json::json!(sec_resp.content);
                let cc_result =
                    check_consistency(&primary_val, &secondary_val, cc_config.deviation_threshold);
                if !cc_result.passed {
                    tracing::warn!(
                        node_id = %node.base_id(),
                        node_type = "llmClassifier",
                        deviation = %cc_result.deviation,
                        threshold = %cc_config.deviation_threshold,
                        "一致性检查未通过: {}", cc_result.details
                    );
                }
            }
        }

        // ── 置信度检查 ──
        let raw_category = if let Some(threshold) = c.confidence_threshold {
            let parsed: serde_json::Value =
                serde_json::from_str(response.content.trim()).map_err(|e| {
                    NodeError::exec_failed(
                        error_code::VALIDATION_FAILED,
                        format!(
                            "LlmClassifier: 无法解析 LLM JSON 响应: {e}, raw: {}",
                            response.content.trim()
                        ),
                    )
                })?;
            let label = parsed
                .get("label")
                .and_then(|l| l.as_str())
                .unwrap_or("")
                .to_string();
            let confidence = parsed
                .get("confidence")
                .and_then(|c| c.as_f64())
                .unwrap_or(0.0);

            if confidence < threshold {
                let fallback = c.fallback_label.as_deref().unwrap_or("unknown");
                tracing::warn!(
                    "[LlmClassifier] 置信度 {:.2} 低于阈值 {:.2}，降级为 '{}'",
                    confidence,
                    threshold,
                    fallback
                );
                fallback.to_string()
            } else {
                label
            }
        } else {
            response.content.trim().to_string()
        };

        let matched = c
            .categories
            .iter()
            .find(|cat| cat.to_lowercase() == raw_category.to_lowercase())
            .cloned()
            .unwrap_or_else(|| {
                c.categories
                    .iter()
                    .find(|cat| raw_category.to_lowercase().contains(&cat.to_lowercase()))
                    .cloned()
                    .unwrap_or(raw_category)
            });

        Ok(NodeOutput {
            output: serde_json::json!({
                "category": matched,
                "model": model,
                "provider": prov.id,
                "input_var": c.input_var,
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

/// 从 ExecutionState 变量中解析点分隔路径（与 tool_executor / condition_executor
/// / switch_executor / validation_executor 的实现保持一致）。
///
/// 解析规则：
/// 1. 空路径直接返回 `None`
/// 2. 尝试按节点输出路径解析：`root = context.variables.get(parts[0])`，
///    然后沿 `parts[1..]` 逐层下钻嵌套字段
/// 3. fallback：root 不是节点 ID 时，将整个 `path` 作为模板变量名直查
fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    if path.is_empty() {
        return None;
    }
    let parts: Vec<&str> = path.split('.').collect();
    if let Some(root) = context.variables.get(parts[0]) {
        let mut current = root.clone();
        for part in &parts[1..] {
            current = current.get(part)?.clone();
        }
        return Some(current);
    }
    context.variables.get(path).cloned()
}

/// 将 `serde_json::Value` 序列化为 LLM 友好的可读文本。
///
/// - `String` 直接 unwrap（避免给 LLM 一串带转义的 JSON 字符串）
/// - `Null` 返回空串（与 `unwrap_or_default()` 行为一致，让上游
///   `input_text.is_empty()` 检查兜底报 VALIDATION_FAILED）
/// - 其他类型（Number / Bool / Array / Object）走 pretty JSON，
///   避免 Object 走默认 `to_string()` 得到紧凑单行 JSON
fn value_to_input_text(v: serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Null => String::new(),
        other => serde_json::to_string_pretty(&other).unwrap_or_else(|_| other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_core::workflow_types::{
        LlmClassifierNode, LlmClassifierNodeConfig, WorkflowNodeBase,
    };
    use std::collections::HashMap;

    // ── resolve_var_path 单元测试 ──────────────────────────────────────

    fn make_context_with_vars(vars: &[(&str, serde_json::Value)]) -> ExecutionState {
        let mut variables = HashMap::new();
        for (k, v) in vars {
            variables.insert((*k).to_string(), v.clone());
        }
        ExecutionState {
            variables,
            ..ExecutionState::new("test".into(), "wf".into(), serde_json::json!({}))
        }
    }

    #[test]
    fn resolve_var_path_empty_returns_none() {
        let ctx = make_context_with_vars(&[("t", serde_json::json!("v"))]);
        assert_eq!(resolve_var_path("", &ctx), None);
    }

    #[test]
    fn resolve_var_path_top_level_key() {
        let ctx = make_context_with_vars(&[("t-risk", serde_json::json!("hello"))]);
        assert_eq!(resolve_var_path("t-risk", &ctx), Some(serde_json::json!("hello")));
    }

    #[test]
    fn resolve_var_path_dotted_nested_field() {
        let ctx = make_context_with_vars(&[(
            "t-risk",
            serde_json::json!({"output": "the risk text", "score": 0.8}),
        )]);
        assert_eq!(
            resolve_var_path("t-risk.output", &ctx),
            Some(serde_json::json!("the risk text"))
        );
    }

    #[test]
    fn resolve_var_path_dotted_deeply_nested() {
        let ctx = make_context_with_vars(&[(
            "t-risk",
            serde_json::json!({"output": {"score": 0.8, "label": "high"}}),
        )]);
        assert_eq!(resolve_var_path("t-risk.output.label", &ctx), Some(serde_json::json!("high")));
    }

    #[test]
    fn resolve_var_path_missing_nested_field_returns_none() {
        let ctx =
            make_context_with_vars(&[("t-risk", serde_json::json!({"output": "the risk text"}))]);
        assert_eq!(resolve_var_path("t-risk.missing", &ctx), None);
    }

    #[test]
    fn resolve_var_path_missing_node_id_returns_none() {
        let ctx = make_context_with_vars(&[("other", serde_json::json!("v"))]);
        assert_eq!(resolve_var_path("nonexistent_node", &ctx), None);
    }

    #[test]
    fn resolve_var_path_dotted_full_key_fallback() {
        // root 不存在时（"a.b" 整体作为 key 找不到）→ fallback 失败
        let ctx = make_context_with_vars(&[("a.b", serde_json::json!("leaf"))]);
        // 注意：split('.') 后 parts[0] = "a" 不存在，fallback 走 get("a.b") → Some(leaf)
        assert_eq!(resolve_var_path("a.b", &ctx), Some(serde_json::json!("leaf")));
    }

    // ── value_to_input_text 单元测试 ──────────────────────────────────

    #[test]
    fn value_to_input_text_string_passes_through() {
        let s = value_to_input_text(serde_json::json!("hello"));
        assert_eq!(s, "hello");
    }

    #[test]
    fn value_to_input_text_null_returns_empty() {
        let s = value_to_input_text(serde_json::json!(null));
        assert_eq!(s, "");
    }

    #[test]
    fn value_to_input_text_object_uses_pretty() {
        let s = value_to_input_text(serde_json::json!({"a": 1, "b": "x"}));
        // 包含换行和缩进（pretty JSON）
        assert!(s.contains('\n'), "should use pretty JSON, got: {s}");
        assert!(s.contains("\"a\""));
    }

    #[test]
    fn value_to_input_text_array_uses_pretty() {
        let s = value_to_input_text(serde_json::json!([1, 2, 3]));
        assert!(s.contains('\n'));
    }

    #[test]
    fn value_to_input_text_number_passes_through() {
        let s = value_to_input_text(serde_json::json!(42));
        assert_eq!(s, "42");
    }

    #[test]
    fn value_to_input_text_bool_passes_through() {
        let s = value_to_input_text(serde_json::json!(true));
        assert_eq!(s, "true");
    }

    // ── execute() 早期返回 VALIDATION_FAILED 路径 ──────────────────────
    //
    // 走 execute() 完整流程能验证 fix 端到端有效（负向用例不需要 mock LLM）。
    // 正向用例需要真实 Provider/Adapter，超出单测范围——但我们用
    // `axagent_core::db::create_test_pool()` 注入真实 DB 句柄，
    // 让 executor 至少能跑过 provider 解析（最后会因
    // ProviderRegistry 为空返回 UNSUPPORTED_PROVIDER，而不是 panic
    // 在 "Disconnected" DB 上）。

    /// 负向用例不需要真实 DB（VALIDATION_FAILED 早于 provider 解析）。
    fn make_executor() -> LlmClassifierExecutor {
        LlmClassifierExecutor::default()
    }

    /// 正向用例需要真实 DB 才能跑过 `resolve_provider_and_adapter`。
    async fn make_executor_with_db() -> LlmClassifierExecutor {
        let handle = axagent_core::db::create_test_pool()
            .await
            .expect("create_test_pool");
        LlmClassifierExecutor::new(Arc::new(handle.conn), [0u8; 32])
    }

    fn make_classifier_node(input_var: &str) -> WorkflowNode {
        WorkflowNode::LlmClassifier(LlmClassifierNode {
            base: WorkflowNodeBase {
                id: "cls".to_string(),
                title: "cls".to_string(),
                description: None,
                position: Default::default(),
                retry: Default::default(),
                timeout: Some(30),
                enabled: true,
                parent_id: None,
                compensation: None,
            },
            config: LlmClassifierNodeConfig {
                categories: vec!["a".to_string(), "b".to_string()],
                prompt: "classify".to_string(),
                model: None,
                input_var: input_var.to_string(),
                output_var: String::new(),
                confidence_threshold: None,
                fallback_label: None,
                consistency_check: None,
            },
        })
    }

    /// 负向用例不需要真实 DB；用 ExecutionState::new() 即可。
    fn make_context(vars: &[(&str, serde_json::Value)]) -> ExecutionState {
        make_context_with_vars(vars)
    }

    #[tokio::test]
    async fn execute_missing_node_id_returns_validation_failed() {
        // input_var: "nonexistent_node" → 整 key 查不到 → 空串 → VALIDATION_FAILED
        let exec = make_executor();
        let node = make_classifier_node("nonexistent_node");
        let ctx = make_context(&[("other_key", serde_json::json!("v"))]);
        let err = exec.execute(&node, &ctx).await.unwrap_err();
        assert_eq!(
            err.code(),
            crate::work_engine::node_executor_trait::error_code::VALIDATION_FAILED
        );
        assert!(err.to_string().contains("input_var 指向的变量为空或不存在"));
    }

    #[tokio::test]
    async fn execute_missing_nested_field_returns_validation_failed() {
        // input_var: "t-risk.missing_field" → 点路径下钻失败 → 空串 → VALIDATION_FAILED
        let exec = make_executor();
        let node = make_classifier_node("t-risk.missing_field");
        let ctx = make_context(&[("t-risk", serde_json::json!({"output": "the risk text"}))]);
        let err = exec.execute(&node, &ctx).await.unwrap_err();
        assert_eq!(
            err.code(),
            crate::work_engine::node_executor_trait::error_code::VALIDATION_FAILED
        );
    }

    #[tokio::test]
    async fn execute_dotted_path_with_valid_input_passes_validation() {
        // input_var: "t-risk.output" → 修复前会走整 key 查表 → 永远 miss → VALIDATION_FAILED
        //                  → 修复后正确下钻到 "output" 字段 → input_text 非空
        // 后续会被 provider 解析拦住（ProviderRegistry 为空走 UNSUPPORTED_PROVIDER），
        // 但**关键证据**是 error code 不是 VALIDATION_FAILED。
        let exec = make_executor_with_db().await;
        let node = make_classifier_node("t-risk.output");
        let ctx = make_context(&[(
            "t-risk",
            serde_json::json!({"output": "the risk text", "score": 0.8}),
        )]);
        let err = exec.execute(&node, &ctx).await.unwrap_err();
        assert_ne!(
            err.code(),
            crate::work_engine::node_executor_trait::error_code::VALIDATION_FAILED,
            "修复前会被错判为 VALIDATION_FAILED；修复后应通过 input 校验，错误码应是 UNSUPPORTED_PROVIDER"
        );
    }

    #[tokio::test]
    async fn execute_top_level_key_path_passes_validation() {
        // input_var: "t-risk"（不带点）→ 修复前/后行为一致：整 key 查到 → 非空
        // 同样应在 provider 解析处失败，但 error code 不是 VALIDATION_FAILED。
        let exec = make_executor_with_db().await;
        let node = make_classifier_node("t-risk");
        let ctx = make_context(&[("t-risk", serde_json::json!("hello"))]);
        let err = exec.execute(&node, &ctx).await.unwrap_err();
        assert_ne!(
            err.code(),
            crate::work_engine::node_executor_trait::error_code::VALIDATION_FAILED
        );
    }
}
