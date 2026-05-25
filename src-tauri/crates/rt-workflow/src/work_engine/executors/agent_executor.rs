//! Agent 执行器 —— 支持 inline role 和 agent_profile 两种模式，均自动使用系统默认模型。
//!
//! 两阶段 prompt 处理：
//!   1. 加载时：compile_prompt() 提取 {{path}} 占位符
//!   2. 执行时：render_prompt() 用 ExecutionState.variables 填充模板

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_core::workflow_types::WorkflowNode;
use futures::StreamExt;
use sea_orm::DatabaseConnection;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};
use crate::work_engine::prompt_template::{
    CompiledPrompt, TemplateSegment, compile_prompt, render_prompt,
};

// 缓存类型（pub(crate) 供 WorkEngine 引用）
pub(crate) type ProviderCache =
    Option<(axagent_core::types::ProviderConfig, axagent_core::types::ProviderKey, String)>;
pub(crate) type ProfileCache = HashMap<String, axagent_core::entity::agent_profiles::Model>;

pub struct AgentExecutor {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
    /// 默认 provider 缓存（同一次工作流执行内复用）
    default_provider_cache: Arc<Mutex<ProviderCache>>,
    /// Agent profile 缓存（同一工作流内多个节点共用同 profile 时复用）
    profile_cache: Arc<Mutex<ProfileCache>>,
}

impl AgentExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self {
            db,
            master_key,
            default_provider_cache: Arc::new(Mutex::new(None)),
            profile_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 构造使用共享缓存的 executor（供 WorkEngine 注入，用于跨执行清除）。
    pub fn with_shared_caches(
        db: Arc<DatabaseConnection>,
        master_key: [u8; 32],
        default_provider_cache: Arc<Mutex<ProviderCache>>,
        profile_cache: Arc<Mutex<ProfileCache>>,
    ) -> Self {
        Self {
            db,
            master_key,
            default_provider_cache,
            profile_cache,
        }
    }
}

impl Default for AgentExecutor {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::default()),
            master_key: [0u8; 32],
            default_provider_cache: Arc::new(Mutex::new(None)),
            profile_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl NodeExecutorTrait for AgentExecutor {
    fn node_type(&self) -> &'static str {
        "agent"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Agent(an) = node else {
            return Err(NodeError::type_mismatch(
                "agent".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        // 1. 加载 agent profile（带缓存）
        use axagent_core::entity::agent_profiles;
        use sea_orm::EntityTrait;
        let profile = if let Some(ref pid) = an.config.agent_profile_id {
            // 先查缓存
            {
                let cache = self.profile_cache.lock().await;
                if let Some(cached) = cache.get(pid.as_str()) {
                    Some(cached.clone())
                } else {
                    drop(cache);
                    let result = agent_profiles::Entity::find_by_id(pid.as_str())
                        .one(self.db.as_ref())
                        .await
                        .map_err(|e| {
                            NodeError::exec_failed(
                                error_code::AGENT_PROFILE_NOT_FOUND,
                                format!("Agent profile query failed: {e}"),
                            )
                        })?;
                    if let Some(ref p) = result {
                        let mut cache = self.profile_cache.lock().await;
                        cache.insert(pid.clone(), p.clone());
                    }
                    result
                }
            }
        } else {
            None
        };

        // 2. 解析默认 provider + key + model（带缓存）
        let session_model = context
            .variables
            .get("__workflow_model__")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let (prov, key, default_model) = self.resolve_provider(profile.as_ref()).await?;
        let api_key = axagent_core::crypto::decrypt_key(&key.key_encrypted, &self.master_key)
            .map_err(|e| {
                NodeError::exec_failed(
                    error_code::AGENT_PROFILE_NOT_FOUND,
                    format!("API key decryption failed: {e}"),
                )
            })?;

        // 3. 创建 adapter
        use axagent_core::types::ProviderType;
        use axagent_providers::{ProviderAdapter, resolve_base_url_for_type};
        let adapter: Arc<dyn ProviderAdapter> = match prov.provider_type {
            ProviderType::OpenAI => Arc::new(axagent_providers::openai::OpenAIAdapter::new()),
            ProviderType::OpenAIResponses => {
                Arc::new(axagent_providers::openai_responses::OpenAIResponsesAdapter::new())
            },
            ProviderType::Anthropic => {
                Arc::new(axagent_providers::anthropic::AnthropicAdapter::new())
            },
            ProviderType::Gemini => Arc::new(axagent_providers::gemini::GeminiAdapter::new()),
            ProviderType::Ollama => Arc::new(axagent_providers::ollama::OllamaAdapter::new()),
            _ => {
                return Err(NodeError::exec_failed(
                    error_code::AGENT_PROFILE_NOT_FOUND,
                    format!("Unsupported provider: {:?}", prov.provider_type),
                ));
            },
        };

        // 4. 构建 prompt：profile 为基础 + 行内追加（合并模式）
        let role_desc = resolve_role(&an.config, profile.as_ref());
        let mut all_segments: Vec<TemplateSegment> = Vec::new();

        // 4a. 角色前缀
        all_segments.push(TemplateSegment::Static(format!("你是 {role_desc}。\n")));

        // 4b. Profile system_prompt 作为基础
        if let Some(ref p) = profile
            && !p.system_prompt.is_empty()
        {
            let profile_compiled = compile_prompt(&p.system_prompt);
            all_segments.extend(profile_compiled.segments);
        }

        // 4c. 行内 system_prompt 追加（从 pre-compiled 缓存取或现场编译）
        if !an.config.system_prompt.is_empty() {
            if let Some(ref compiled_map) = context.compiled_prompts {
                if let Some(inline_compiled) = compiled_map.get(&an.base.id) {
                    all_segments.extend(inline_compiled.segments.clone());
                } else {
                    all_segments.extend(compile_prompt(&an.config.system_prompt).segments);
                }
            } else {
                all_segments.extend(compile_prompt(&an.config.system_prompt).segments);
            }
        }

        // 4d. 上下文数据（自然语言格式化，替代 raw JSON dump）
        if !an.config.context_sources.is_empty() {
            all_segments.push(TemplateSegment::Static("\n\n--- 上游节点输出 ---\n".to_string()));
            for source in &an.config.context_sources {
                if let Some(value) = context.variables.get(source) {
                    let formatted = format_context_source(source, value);
                    all_segments.push(TemplateSegment::Static(formatted));
                }
            }
        }

        let compiled = CompiledPrompt {
            segments: all_segments,
            variable_refs: Vec::new(),
        };

        let system_prompt = render_prompt(&compiled, &context.variables).map_err(|e| {
            NodeError::exec_failed(
                error_code::VARIABLE_NOT_FOUND,
                format!("Prompt rendering failed: {e}"),
            )
        })?;

        // 5. 构建 user_prompt：仅包含 context_sources 的变量（更精准，减少噪声）
        let user_prompt = if an.config.context_sources.is_empty() {
            // 向后兼容：无 context_sources 时包含所有变量
            context
                .variables
                .iter()
                .filter(|(k, _)| !k.starts_with("__"))
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            an.config
                .context_sources
                .iter()
                .filter_map(|s| context.variables.get(s).map(|v| format!("{s}: {v}")))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let mut messages: Vec<ChatMessage> = vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(user_prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ];

        let base_url = resolve_base_url_for_type(&prov.api_host, &prov.provider_type);
        let req_ctx = axagent_providers::ProviderRequestContext {
            provider_id: prov.id.clone(),
            api_key,
            key_id: key.id.clone(),
            base_url: Some(base_url),
            api_path: None,
            proxy_config: None,
            custom_headers: None,
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        };
        let model = session_model.unwrap_or(default_model);
        let model_for_output = model.clone();

        // 构建工具定义（若配置了 tools）
        let tools: Option<Vec<axagent_core::types::ChatTool>> = if an.config.tools.is_empty() {
            None
        } else {
            Some(
                an.config
                    .tools
                    .iter()
                    .map(|td| axagent_core::types::ChatTool {
                        r#type: "function".to_string(),
                        function: axagent_core::types::ChatToolFunction {
                            name: td.name.clone(),
                            description: td.description.clone(),
                            parameters: td
                                .parameters
                                .as_ref()
                                .map(|p| {
                                    serde_json::to_value(p).unwrap_or(serde_json::json!({
                                        "type": "object",
                                        "properties": {},
                                        "additionalProperties": true,
                                    }))
                                })
                                .or_else(|| {
                                    Some(serde_json::json!({
                                        "type": "object",
                                        "properties": {},
                                        "additionalProperties": true,
                                    }))
                                }),
                        },
                    })
                    .collect(),
            )
        };

        // 最大工具调用轮数：配置值或默认 5
        let max_rounds = an.config.max_tool_rounds.unwrap_or(5).max(1);
        let mut total_usage = (0u32, 0u32);
        let mut final_content = String::new();
        let mut final_thinking: Option<String> = None;
        let mut tool_calls_made: Vec<serde_json::Value> = Vec::new();

        for round in 0..max_rounds {
            let request = ChatRequest {
                model: model.clone(),
                messages: messages.clone(),
                stream: true,
                temperature: an.config.temperature.map(|t| t as f64),
                max_tokens: an.config.max_tokens,
                top_p: None,
                // 首轮传 tools，后续轮次若 tools 为空则不传
                tools: if round == 0 { tools.clone() } else { None },
                thinking_budget: None,
                use_max_completion_tokens: None,
                thinking_param_style: None,
                api_mode: None,
                instructions: None,
                conversation: None,
                previous_response_id: None,
                store: None,
            };

            // 流式调用 LLM，聚合增量块
            let mut stream = adapter.chat_stream(&req_ctx, request, None);
            let mut stream_content = String::new();
            let mut stream_thinking: Option<String> = None;
            let mut stream_tool_calls: Option<Vec<axagent_core::types::ToolCall>> = None;
            let mut stream_usage = (0u32, 0u32);

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| {
                    NodeError::exec_failed(
                        error_code::AGENT_PROFILE_NOT_FOUND,
                        format!("Agent LLM stream error: {e}"),
                    )
                })?;

                if let Some(ref content) = chunk.content {
                    stream_content.push_str(content);
                }
                if let Some(ref thinking) = chunk.thinking {
                    stream_thinking = Some(thinking.clone());
                }
                if let Some(usage) = chunk.usage {
                    stream_usage = (usage.prompt_tokens, usage.completion_tokens);
                }
                if chunk.tool_calls.is_some() {
                    stream_tool_calls = chunk.tool_calls;
                }
            }

            total_usage.0 += stream_usage.0;
            total_usage.1 += stream_usage.1;
            final_content = stream_content.clone();
            final_thinking = stream_thinking.clone();

            // 检查是否有工具调用
            let tool_calls = stream_tool_calls;
            let has_tool_calls = tool_calls
                .as_ref()
                .map(|tc| !tc.is_empty())
                .unwrap_or(false);

            if !has_tool_calls {
                // LLM 返回纯文本，结束循环
                break;
            }

            // 处理工具调用
            let tc_list = tool_calls.as_ref().unwrap();

            // 构建 assistant 消息（含 tool_calls）
            let assistant_msg = ChatMessage {
                role: "assistant".to_string(),
                content: if stream_content.is_empty() {
                    ChatContent::Text(String::new())
                } else {
                    ChatContent::Text(stream_content.clone())
                },
                tool_calls: Some(tc_list.clone()),
                tool_call_id: None,
                thinking: stream_thinking.clone(),
            };
            messages.push(assistant_msg);

            // 执行每个工具调用
            for tc in tc_list {
                let args: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::Null);

                let tool_result = execute_tool(context, &tc.function.name, args.clone()).await;

                let result_str = match &tool_result {
                    Ok(v) => serde_json::to_string(v).unwrap_or_else(|_| format!("{v}")),
                    Err(e) => e.clone(),
                };

                tool_calls_made.push(serde_json::json!({
                    "tool": &tc.function.name,
                    "arguments": args,
                    "result": result_str,
                }));

                // 追加 tool 角色消息
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: ChatContent::Text(result_str),
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                    thinking: None,
                });
            }

            // 最后一轮即使还有 tool_calls 也结束
            if round + 1 >= max_rounds {
                break;
            }
        }

        Ok(NodeOutput {
            output: serde_json::json!({
                "role": role_desc, "model": model_for_output,
                "content": final_content, "thinking": final_thinking,
                "usage": { "input_tokens": total_usage.0, "output_tokens": total_usage.1 },
                "tool_calls_made": tool_calls_made,
                "node_id": node.base_id(),
            }),
            output_var: Some(an.config.output_var.clone()),
        })
    }
}

/// 从 context.callbacks 中查找并执行工具。
async fn execute_tool(
    context: &ExecutionState,
    tool_name: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let cb = context
        .callbacks
        .as_ref()
        .and_then(|cbs| cbs.tool_handlers.get(tool_name).cloned())
        .or_else(|| {
            context
                .callbacks
                .as_ref()
                .and_then(|cbs| cbs.tool_fallback.clone())
        });

    match cb {
        Some(handler) => handler(tool_name.to_string(), args).await,
        None => Err(format!("工具 '{tool_name}' 未注册")),
    }
}

// ── 辅助方法 ──

impl AgentExecutor {
    /// 解析 provider + key + model，优先用缓存。
    /// 注意：当 profile 指定了 suggested_provider_id 时不做缓存（专用 provider），
    /// 仅对"无 profile"或"profile 无 provider 偏好"的分辨结果做缓存。
    async fn resolve_provider(
        &self,
        profile: Option<&axagent_core::entity::agent_profiles::Model>,
    ) -> Result<
        (axagent_core::types::ProviderConfig, axagent_core::types::ProviderKey, String),
        NodeError,
    > {
        // 仅当无 profile 指定 provider 时使用缓存
        let has_profile_override = profile
            .and_then(|p| p.suggested_provider_id.as_ref())
            .is_some();
        if !has_profile_override {
            let cache = self.default_provider_cache.lock().await;
            if let Some(ref cached) = *cache {
                return Ok(cached.clone());
            }
        }

        let result = if let Some(p) = profile {
            let all = axagent_core::repo::provider::list_providers(&self.db)
                .await
                .map_err(|e| {
                    NodeError::exec_failed(
                        error_code::AGENT_PROFILE_NOT_FOUND,
                        format!("Provider query failed: {e}"),
                    )
                })?;
            if let Some(ref suggested_id) = p.suggested_provider_id {
                if let Some(sp) = all
                    .into_iter()
                    .find(|pr| pr.id == *suggested_id && pr.enabled)
                {
                    let key = sp.keys.iter().find(|k| k.enabled).cloned().ok_or_else(|| {
                        NodeError::exec_failed(
                            error_code::AGENT_PROFILE_NOT_FOUND,
                            "建议的 provider 无可用 key".to_string(),
                        )
                    })?;
                    let model = sp
                        .models
                        .iter()
                        .find(|m| m.enabled)
                        .map(|m| m.model_id.clone())
                        .ok_or_else(|| {
                            NodeError::exec_failed(
                                error_code::AGENT_PROFILE_NOT_FOUND,
                                "建议的 provider 无可用模型".to_string(),
                            )
                        })?;
                    Ok::<_, NodeError>((sp, key, model))
                } else {
                    axagent_core::repo::provider::resolve_default_provider(&self.db)
                        .await
                        .map_err(|e| NodeError::exec_failed(error_code::PROVIDER_QUERY_FAILED, e))
                }
            } else {
                axagent_core::repo::provider::resolve_default_provider(&self.db)
                    .await
                    .map_err(|e| NodeError::exec_failed(error_code::PROVIDER_QUERY_FAILED, e))
            }
        } else {
            axagent_core::repo::provider::resolve_default_provider(&self.db)
                .await
                .map_err(|e| NodeError::exec_failed(error_code::PROVIDER_QUERY_FAILED, e))
        }?;

        // 仅将"默认 provider"结果写入缓存（profile 指定的 provider 不缓存）
        if !has_profile_override {
            let mut cache = self.default_provider_cache.lock().await;
            *cache = Some(result.clone());
        }

        Ok(result)
    }
}

// ── 自由函数 ──

/// 解析角色描述：优先 agent_role_override → profile.agent_role → config.role → "executor"
fn resolve_role(
    config: &axagent_core::workflow_types::AgentNodeConfig,
    profile: Option<&axagent_core::entity::agent_profiles::Model>,
) -> String {
    if let Some(ref ov) = config.agent_role_override {
        return ov.clone();
    }
    if let Some(p) = profile
        && let Some(ref role) = p.agent_role
    {
        return role.clone();
    }
    if let Some(ref role) = config.role {
        return format!("{:?}", role);
    }
    "executor".to_string()
}

/// 将上下文源格式化为自然语言章节（替代 raw JSON dump）。
///
/// 智能检测 JSON 结构：
/// - 有 `content` 字段 → 取其文本内容
/// - 有 `summary` 字段 → 取其摘要
/// - 否则 → 紧凑 JSON 字符串
fn format_context_source(name: &str, value: &Value) -> String {
    // 提取嵌套字段（如 node_id.output.content）
    let inner = if value.is_object() {
        value.get("output").or_else(|| value.get("result"))
    } else {
        None
    };
    let target = inner.unwrap_or(value);

    let body = match target {
        Value::String(s) => s.clone(),
        Value::Object(_) => {
            // 优先提取常见语义字段
            if let Some(content) = target.get("content").and_then(|v| v.as_str()) {
                content.to_string()
            } else if let Some(summary) = target.get("summary").and_then(|v| v.as_str()) {
                summary.to_string()
            } else {
                target.to_string()
            }
        },
        other => other.to_string(),
    };

    format!("[{name}] 输出:\n{body}\n\n")
}
