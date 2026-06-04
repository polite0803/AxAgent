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
            context
                .variables
                .get(&c.input_var)
                .map(|v| v.to_string())
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
            && cc_config.enabled {
                let secondary_request = if matches!(
                    cc_config.mode,
                    axagent_harness::ConsistencyMode::CrossModelCompare
                ) {
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
                    let cc_result = check_consistency(
                        &primary_val,
                        &secondary_val,
                        cc_config.deviation_threshold,
                    );
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
