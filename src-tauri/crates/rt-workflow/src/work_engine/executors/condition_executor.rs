// SPDX-License-Identifier: AGPL-3.0-only

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
    /// 由 Harness 注入的 ProviderRegistry（运行时按 provider 类型查找 adapter）
    provider_registry: Option<Arc<dyn axagent_harness::registry::ProviderRegistry>>,
}

impl ConditionExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self {
            db,
            master_key,
            provider_registry: None,
        }
    }
}

impl axagent_harness::HasProviderRegistry for ConditionExecutor {
    fn set_provider_registry(
        &mut self,
        registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) {
        self.provider_registry = Some(registry);
    }
}

impl Default for ConditionExecutor {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::default()),
            master_key: [0u8; 32],
            provider_registry: None,
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
            // 修复：原本降级为 contains。regex crate 已在 Cargo.toml 中可用，
            // 这里使用真正的正则匹配。编译失败时返回 false（不 panic 阻断流程）。
            actual
                .as_str()
                .zip(expected.as_str())
                .is_some_and(|(a, pat)| match regex::Regex::new(pat) {
                    Ok(re) => re.is_match(a),
                    Err(e) => {
                        tracing::warn!("[condition] RegexMatch 模式 '{pat}' 编译失败: {e}");
                        false
                    },
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

        // 2. 解析 provider + model
        // 优先级：节点 routing_model > 会话 __workflow_model__/__workflow_provider_id__ > 项目默认
        let node_model = config.routing_model.as_deref().filter(|m| !m.is_empty());
        let session_model = context
            .variables
            .get(super::WORKFLOW_MODEL_VAR)
            .and_then(|v| v.as_str());
        let session_provider_id = context
            .variables
            .get(super::WORKFLOW_PROVIDER_ID_VAR)
            .and_then(|v| v.as_str());

        let result = self
            .route_with_resolved_model(
                node_model,
                session_model,
                session_provider_id,
                &prompt,
                config.confidence_threshold,
            )
            .await;

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
    async fn route_with_resolved_model(
        &self,
        node_model: Option<&str>,
        session_model: Option<&str>,
        session_provider_id: Option<&str>,
        prompt: &str,
        confidence_threshold: Option<f64>,
    ) -> Result<bool, String> {
        let (prov, _key, model, adapter, api_key) = super::resolve_provider_and_adapter(
            &self.db,
            &self.master_key,
            self.provider_registry.as_ref(),
            node_model,
            session_model,
            session_provider_id,
            None,
            "ConditionExecutor",
        )
        .await
        .map_err(|e| format!("模型解析失败: {e}"))?;

        self.call_llm_and_parse(&prov, &api_key, &model, &adapter, prompt, confidence_threshold)
            .await
    }

    /// 调用 LLM 并解析 true/false 响应。
    async fn call_llm_and_parse(
        &self,
        prov: &axagent_harness::types::ProviderConfig,
        api_key: &str,
        model: &str,
        adapter: &Arc<dyn axagent_harness::ProviderAdapter>,
        prompt: &str,
        confidence_threshold: Option<f64>,
    ) -> Result<bool, String> {
        use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
        use axagent_harness::url_utils::resolve_base_url_for_type;

        let base_url = resolve_base_url_for_type(&prov.api_host, &prov.provider_type);
        let req_ctx = axagent_harness::ProviderRequestContext {
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

        let llm_prompt = if confidence_threshold.is_some() {
            format!(
                "{}\n\n请用 JSON 格式输出，包含 decision（true 或 false）和 confidence（0.0-1.0 的置信度）。\
                 例如：{{\"decision\": true, \"confidence\": 0.95}}。\
                 只输出 JSON，不要包含其他内容。",
                prompt
            )
        } else {
            prompt.to_string()
        };

        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(llm_prompt),
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

        // ── 置信度检查 ──
        if let Some(threshold) = confidence_threshold {
            let parsed: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| format!("无法解析 LLM JSON 响应: {e}, raw: {text}"))?;
            let decision = parsed
                .get("decision")
                .and_then(|d| d.as_bool())
                .ok_or_else(|| format!("LLM JSON 响应缺少 decision 字段: {text}"))?;
            let confidence = parsed
                .get("confidence")
                .and_then(|c| c.as_f64())
                .ok_or_else(|| format!("LLM JSON 响应缺少 confidence 字段: {text}"))?;

            if confidence < threshold {
                tracing::warn!(
                    "[ConditionExecutor] LLM 路由置信度 {:.2} 低于阈值 {:.2}，降级为启发式判断",
                    confidence,
                    threshold
                );
                return Err(format!("置信度 {:.2} 低于阈值 {:.2}", confidence, threshold));
            }

            return Ok(decision);
        }

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
    // 修复：空路径直接返回 None，不再走 parts[0] 触发 panic
    if path.is_empty() {
        return None;
    }
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
