// SPDX-License-Identifier: AGPL-3.0-only

//! LLM 执行器 —— 解析系统默认 provider 和模型后调用 `adapter.chat()`。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};
use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use axagent_harness::build_provider_request_context;
use axagent_runtime_core::{LlmCallConfig, execute_llm};

pub struct LlmExecutor {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
    /// 由 Harness 注入的 ProviderRegistry（运行时按 provider 类型查找 adapter）
    provider_registry: Option<Arc<dyn axagent_harness::registry::ProviderRegistry>>,
}

impl LlmExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self {
            db,
            master_key,
            provider_registry: None,
        }
    }
}

impl axagent_harness::HasProviderRegistry for LlmExecutor {
    fn set_provider_registry(
        &mut self,
        registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) {
        self.provider_registry = Some(registry);
    }
}
impl Default for LlmExecutor {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::default()),
            master_key: [0u8; 32],
            provider_registry: None,
        }
    }
}

#[async_trait]
impl NodeExecutorTrait for LlmExecutor {
    fn node_type(&self) -> &'static str {
        "llm"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Llm(llm_node) = node else {
            return Err(NodeError::type_mismatch(
                "llm".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        // 解析 provider + key + model。
        // 优先级：节点 config.model > 会话 __workflow_model__/__workflow_provider_id__ > 项目默认
        let node_model = if !llm_node.config.model.is_empty() {
            Some(llm_node.config.model.as_str())
        } else {
            None
        };
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
            "LlmExecutor",
        )
        .await?;

        // 构建 messages
        let mut messages: Vec<ChatMessage> = llm_node
            .config
            .messages
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| serde_json::from_value(m).ok())
            .collect();
        if messages.is_empty() {
            let ctx_text = context
                .variables
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join("\n");
            messages = vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(llm_node.config.prompt.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(ctx_text),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
            ];
        }

        // ── 严格模式约束注入（当 tool_permissions.strict_mode = true 时注入 system prompt） ──
        // 这是节点级别的 prompt 修改，execute_llm() 不处理此逻辑
        {
            let strict_mode = context
                .tool_permissions
                .as_ref()
                .map(|p| p.strict_mode)
                .unwrap_or(false);
            if strict_mode
                && let Some(system_msg) = messages.iter_mut().find(|m| m.role == "system")
                && let ChatContent::Text(ref mut text) = system_msg.content
            {
                text.push_str(
                            "\n\n## 严格模式约束\n\n\
                             你当前处于严格执行模式，必须遵守以下规则：\n\n\
                             1. **仅输出符合目标 schema 的 JSON**，不添加任何解释、说明或额外文本\n\
                             2. **不允许反问用户** — 不要询问确认意见、不要征求许可、不要请求更多信息\n\
                             3. **不允许输出与当前步骤无关的内容** — 专注于完成指定任务\n\
                             4. **如果无法完成任务**，输出 `{\"error\": \"详细原因\"}`，不要自由发挥、猜测或填充缺失信息\n\
                             5. **不要做额外假设** — 只基于给定的输入数据执行操作",
                        );
                tracing::warn!("[LlmExecutor] node {} strict_mode enabled", node.base_id());
            }
        }

        // ── 上下文窗口管理 ──
        // 已迁移至 execute_llm() 中心化处理，此处在构建 LlmCallConfig 时传递参数

        let req_ctx = build_provider_request_context(&prov, &key, api_key);
        let model_for_output = model.clone();

        if context.dry_run {
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "content": "[DRY RUN] LLM 模拟输出", "model": model_for_output,
                    "usage": {"input_tokens":0,"output_tokens":0},
                    "dry_run": true, "node_id": node.base_id(),
                }),
                output_var: None,
            });
        }

        let request = ChatRequest {
            model: model.clone(),
            messages,
            stream: false,
            temperature: llm_node.config.temperature.map(|t| t as f64),
            max_tokens: llm_node.config.max_tokens,
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

        let llm_config = LlmCallConfig {
            max_context_tokens: llm_node.config.max_context_tokens,
            reserved_output_tokens: llm_node.config.reserved_output_tokens,
            strict_mode: context
                .tool_permissions
                .as_ref()
                .map(|p| p.strict_mode)
                .unwrap_or(false),
            ..Default::default()
        };

        let result = execute_llm(&*adapter, &req_ctx, request.clone(), &llm_config)
            .await
            .map_err(|e| {
                NodeError::exec_failed(
                    error_code::UNSUPPORTED_PROVIDER,
                    format!("LLM call failed: {e}"),
                )
            })?;

        let response = result.response;

        // ── 结果一致性检查（仍走中心化路径） ──
        if let Some(ref cc_config) = llm_node.config.consistency_check
            && cc_config.enabled
        {
            let secondary_request =
                if matches!(cc_config.mode, axagent_harness::ConsistencyMode::CrossModelCompare) {
                    let sec_model = cc_config
                        .secondary_model
                        .as_deref()
                        .unwrap_or(&model_for_output);
                    ChatRequest {
                        model: sec_model.to_string(),
                        ..request.clone()
                    }
                } else {
                    request.clone()
                };
            let secondary_result =
                execute_llm(&*adapter, &req_ctx, secondary_request, &llm_config).await;
            if let Ok(sec_result) = secondary_result {
                use axagent_harness::consistency_check::check_consistency;
                let primary_val = serde_json::json!(response.content);
                let secondary_val = serde_json::json!(sec_result.response.content);
                let cc_result =
                    check_consistency(&primary_val, &secondary_val, cc_config.deviation_threshold);
                if !cc_result.passed {
                    tracing::warn!(
                        node_id = %node.base_id(),
                        node_type = "llm",
                        deviation = %cc_result.deviation,
                        threshold = %cc_config.deviation_threshold,
                        "一致性检查未通过: {}", cc_result.details
                    );
                }
            }
        }

        Ok(NodeOutput {
            output: serde_json::json!({
                "model": model_for_output, "provider": prov.id,
                "content": response.content, "thinking": response.thinking,
                "usage": { "input_tokens": response.usage.prompt_tokens, "output_tokens": response.usage.completion_tokens },
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
