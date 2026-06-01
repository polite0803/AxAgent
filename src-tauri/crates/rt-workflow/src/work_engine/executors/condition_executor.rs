//! 条件执行器 —— 根据 ConditionNodeConfig 评估条件表达式。
//!
//! 支持两种评估模式：
//!   1. 静态条件评估：按 conditions + logical_op 逐条比较
//!   2. LLM 动态路由：调用 LLM 根据上下文判断走 true/false 分支

use async_trait::async_trait;
use axagent_core::workflow_types::{CompareOperator, LogicalOperator, WorkflowNode};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct ConditionExecutor {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
}

impl ConditionExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self { db, master_key }
    }
}

impl Default for ConditionExecutor {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::default()),
            master_key: [0u8; 32],
        }
    }
}

#[async_trait]
impl NodeExecutorTrait for ConditionExecutor {
    fn node_type(&self) -> &'static str {
        "condition"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Condition(condition_node) = node else {
            return Err(NodeError::type_mismatch(
                "condition".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        // LLM 动态路由模式：真正调用 LLM 判断分支
        if condition_node.config.judge_by_llm.unwrap_or(false) {
            return self
                .execute_llm_route(&condition_node.config, context, node.base_id())
                .await;
        }

        let mut results = Vec::new();

        for condition in &condition_node.config.conditions {
            let actual = resolve_var_path(&condition.var_path, context);
            let passed = evaluate_single(&condition.operator, &actual, &condition.value);
            results.push(passed);
        }

        let overall = match condition_node.config.logical_op {
            LogicalOperator::And => results.iter().all(|&r| r),
            LogicalOperator::Or => results.iter().any(|&r| r),
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "evaluated",
                "result": overall,
                "conditions_checked": results.len(),
                "passed_count": results.iter().filter(|&&r| r).count(),
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}

/// 评估单个条件比较。
fn evaluate_single(
    op: &CompareOperator,
    actual: &Option<serde_json::Value>,
    expected: &serde_json::Value,
) -> bool {
    let Some(actual) = actual else {
        return matches!(op, CompareOperator::Ne | CompareOperator::IsEmpty);
    };

    match op {
        CompareOperator::Eq => actual == expected,
        CompareOperator::Ne => actual != expected,
        CompareOperator::Gt => compare_values(actual, expected) == std::cmp::Ordering::Greater,
        CompareOperator::Lt => compare_values(actual, expected) == std::cmp::Ordering::Less,
        CompareOperator::Gte => {
            !matches!(compare_values(actual, expected), std::cmp::Ordering::Less)
        },
        CompareOperator::Lte => {
            !matches!(compare_values(actual, expected), std::cmp::Ordering::Greater)
        },
        CompareOperator::Contains => actual
            .as_str()
            .zip(expected.as_str())
            .is_some_and(|(a, e)| a.contains(e)),
        CompareOperator::NotContains => actual
            .as_str()
            .zip(expected.as_str())
            .is_none_or(|(a, e)| !a.contains(e)),
        CompareOperator::StartsWith => actual
            .as_str()
            .zip(expected.as_str())
            .is_some_and(|(a, e)| a.starts_with(e)),
        CompareOperator::EndsWith => actual
            .as_str()
            .zip(expected.as_str())
            .is_some_and(|(a, e)| a.ends_with(e)),
        CompareOperator::RegexMatch => {
            actual
                .as_str()
                .zip(expected.as_str())
                .is_some_and(|(a, pat)| {
                    // 简单子串匹配作为 regex 的降级实现。
                    // 完整正则支持需要引入 regex crate。
                    a.contains(pat)
                })
        },
        CompareOperator::IsEmpty => {
            actual.is_null() || actual.as_str().is_some_and(|s| s.is_empty())
        },
        CompareOperator::IsNotEmpty => {
            !actual.is_null() && actual.as_str().is_none_or(|s| !s.is_empty())
        },
    }
}

/// 比较两个 JSON 值（数值按 f64，其他按字符串）。
fn compare_values(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
    match (a.as_f64(), b.as_f64()) {
        (Some(a_num), Some(b_num)) => a_num
            .partial_cmp(&b_num)
            .unwrap_or(std::cmp::Ordering::Equal),
        _ => a.to_string().cmp(&b.to_string()),
    }
}

// ── LLM 动态路由 ──────────────────────────────────────────

impl ConditionExecutor {
    /// 调用 LLM 判断走 true 还是 false 分支。
    async fn execute_llm_route(
        &self,
        config: &axagent_core::workflow_types::ConditionNodeConfig,
        context: &ExecutionState,
        node_id: &str,
    ) -> Result<NodeOutput, NodeError> {
        // 1. 构建上下文摘要
        let vars_summary: String = context
            .variables
            .iter()
            .filter(|(k, _)| !k.starts_with("__"))
            .map(|(k, v)| format!("  {k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n");

        let routing_prompt = config
            .routing_prompt
            .as_deref()
            .unwrap_or("根据上述上下文数据，判断是否满足分支条件，只回答 true 或 false。");

        let prompt = format!(
            "你是一个条件判断器。根据上下文数据判断是否走 true 分支。\n\n\
             上下文数据：\n{vars_summary}\n\n\
             判断规则：{routing_prompt}\n\n\
             只回答 true 或 false，不要包含其他内容。"
        );

        // 2. 解析 provider
        let target_model = config.routing_model.clone().or_else(|| {
            context
                .variables
                .get("__workflow_model__")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

        let target_provider_id = context
            .variables
            .get("__workflow_provider_id__")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let result = if let Some(provider_id) = target_provider_id {
            self.route_with_specific_provider(&provider_id, &prompt, target_model.as_deref())
                .await
        } else {
            self.route_with_default_provider(&prompt, target_model.as_deref())
                .await
        };

        match result {
            Ok(branch) => Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "evaluated",
                    "result": branch,
                    "judge_mode": "llm",
                    "note": "LLM 动态路由判断",
                    "node_id": node_id,
                }),
                output_var: None,
            }),
            Err(e) => {
                // LLM 调用失败时降级为启发式
                tracing::warn!("[ConditionExecutor] LLM 路由失败，降级为启发式: {e}");
                let fallback = evaluate_llm_heuristic(config, context);
                Ok(NodeOutput {
                    output: serde_json::json!({
                        "status": "evaluated",
                        "result": fallback,
                        "judge_mode": "heuristic_fallback",
                        "note": format!("LLM 调用失败({e})，降级为启发式判断"),
                        "node_id": node_id,
                    }),
                    output_var: None,
                })
            },
        }
    }

    /// 使用指定 provider 调用 LLM 路由。
    async fn route_with_specific_provider(
        &self,
        provider_id: &str,
        prompt: &str,
        model: Option<&str>,
    ) -> Result<bool, String> {
        let all = axagent_core::repo::provider::list_providers(&self.db)
            .await
            .map_err(|e| format!("查询 provider 失败: {e}"))?;
        let prov = all
            .iter()
            .find(|p| p.id == provider_id && p.enabled)
            .ok_or_else(|| format!("Provider '{provider_id}' 不可用"))?;
        let key = prov
            .keys
            .iter()
            .find(|k| k.enabled)
            .cloned()
            .ok_or_else(|| "无可用 API key".to_string())?;
        let api_key = axagent_core::crypto::decrypt_key(&key.key_encrypted, &self.master_key)
            .map_err(|e| format!("解密 key 失败: {e}"))?;
        let default_model = prov
            .models
            .iter()
            .find(|m| m.enabled)
            .map(|m| m.model_id.clone())
            .ok_or_else(|| "无可用模型".to_string())?;
        let model = model.unwrap_or(&default_model);

        self.call_llm_and_parse(prov, &api_key, model, prompt).await
    }

    /// 使用系统默认 provider 调用 LLM 路由。
    async fn route_with_default_provider(
        &self,
        prompt: &str,
        model: Option<&str>,
    ) -> Result<bool, String> {
        let (prov, key, default_model) =
            axagent_core::repo::provider::resolve_default_provider(&self.db)
                .await
                .map_err(|e| format!("无可用默认 provider: {e}"))?;
        let api_key = axagent_core::crypto::decrypt_key(&key.key_encrypted, &self.master_key)
            .map_err(|e| format!("解密 key 失败: {e}"))?;
        let model = model.unwrap_or(&default_model);

        self.call_llm_and_parse(&prov, &api_key, model, prompt)
            .await
    }

    /// 调用 LLM 并解析 true/false 响应。
    async fn call_llm_and_parse(
        &self,
        prov: &axagent_core::types::ProviderConfig,
        api_key: &str,
        model: &str,
        prompt: &str,
    ) -> Result<bool, String> {
        use axagent_core::types::{ChatContent, ChatMessage, ChatRequest, ProviderType};
        use axagent_providers::{ProviderAdapter, resolve_base_url_for_type};

        let adapter: Arc<dyn ProviderAdapter> = match prov.provider_type {
            ProviderType::OpenAI => Arc::new(axagent_providers::openai::OpenAIAdapter::new()),
            ProviderType::Anthropic => {
                Arc::new(axagent_providers::anthropic::AnthropicAdapter::new())
            },
            ProviderType::Gemini => Arc::new(axagent_providers::gemini::GeminiAdapter::new()),
            ProviderType::Ollama => Arc::new(axagent_providers::ollama::OllamaAdapter::new()),
            _ => return Err(format!("不支持的 provider 类型: {:?}", prov.provider_type)),
        };

        let base_url = resolve_base_url_for_type(&prov.api_host, &prov.provider_type);
        let req_ctx = axagent_providers::ProviderRequestContext {
            provider_id: prov.id.clone(),
            api_key: api_key.to_string(),
            key_id: String::new(),
            base_url: Some(base_url),
            api_path: None,
            proxy_config: None,
            custom_headers: None,
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        };

        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }],
            stream: false,
            temperature: Some(0.0),
            max_tokens: Some(10),
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

        let response = adapter
            .chat(&req_ctx, request)
            .await
            .map_err(|e| format!("LLM 调用失败: {e}"))?;

        let text = response.content.trim().to_lowercase();

        // 严格解析：只接受纯 true/false/yes/no
        let trimmed = text.trim();
        let is_true = trimmed == "true" || trimmed == "yes";
        let is_false = trimmed == "false" || trimmed == "no";
        if is_true {
            Ok(true)
        } else if is_false {
            Ok(false)
        } else {
            Err(format!(
                "LLM response did not contain a clear true/false decision. Got: {}",
                text
            ))
        }
    }
}

/// 启发式降级判断：有上下文数据走 true，否则走 false。
fn evaluate_llm_heuristic(
    config: &axagent_core::workflow_types::ConditionNodeConfig,
    context: &ExecutionState,
) -> bool {
    let meaningful_vars = context
        .variables
        .iter()
        .filter(|(k, _)| !k.starts_with("__"))
        .count();
    if meaningful_vars > 0 {
        // 有变量但不足以判断，保守降级为 false（安全分支）
        return false;
    }
    if !config.conditions.is_empty() {
        let mut results = Vec::new();
        for c in &config.conditions {
            let actual = resolve_var_path(&c.var_path, context);
            results.push(evaluate_single(&c.operator, &actual, &c.value));
        }
        return match config.logical_op {
            LogicalOperator::And => results.iter().all(|&r| r),
            LogicalOperator::Or => results.iter().any(|&r| r),
        };
    }
    false
}

/// 从 ExecutionState 变量中解析点分隔路径。
fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    // 尝试按节点输出路径解析：root 为节点 ID，后续为嵌套字段
    if let Some(root) = context.variables.get(parts[0]) {
        let mut current = root.clone();
        for part in &parts[1..] {
            current = current.get(part)?.clone();
        }
        return Some(current);
    }
    // fallback：root 不是节点 ID，将整个 path 作为模板变量名直查
    context.variables.get(path).cloned()
}
