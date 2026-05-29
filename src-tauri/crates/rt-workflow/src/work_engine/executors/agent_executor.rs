//! Agent 执行器 —— 支持 inline role 和 agent_profile 两种模式，均自动使用系统默认模型。
//!
//! 两阶段 prompt 处理：
//!   1. 加载时：compile_prompt() 提取 {{path}} 占位符
//!   2. 执行时：render_prompt() 用 ExecutionState.variables 填充模板

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest, RagContextResult};
use axagent_core::workflow_types::WorkflowNode;
use futures::StreamExt;
use sea_orm::DatabaseConnection;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::work_engine::WorkEngine;
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

pub type RagCallback = Arc<
    dyn Fn(
            Vec<String>,
            Vec<String>,
            Vec<String>,
            String,
        ) -> Pin<Box<dyn Future<Output = Result<RagContextResult, String>> + Send>>
        + Send
        + Sync,
>;

pub struct AgentExecutor {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
    rag_callback: Option<RagCallback>,
    /// Plan 模式用：注入 WorkEngine 引用（临时工作流创建+执行）
    engine: Option<Arc<super::super::WorkEngine>>,
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
            rag_callback: None,
            engine: None,
            default_provider_cache: Arc::new(Mutex::new(None)),
            profile_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_engine(mut self, engine: Arc<WorkEngine>) -> Self {
        self.engine = Some(engine);
        self
    }

    pub fn with_rag_callback(mut self, cb: RagCallback) -> Self {
        self.rag_callback = Some(cb);
        self
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
            rag_callback: None,
            engine: None,
            default_provider_cache,
            profile_cache,
        }
    }

    pub fn with_shared_caches_and_rag_callback(
        db: Arc<DatabaseConnection>,
        master_key: [u8; 32],
        default_provider_cache: Arc<Mutex<ProviderCache>>,
        profile_cache: Arc<Mutex<ProfileCache>>,
        rag_callback: RagCallback,
    ) -> Self {
        Self {
            db,
            master_key,
            rag_callback: Some(rag_callback),
            engine: None,
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
            rag_callback: None,
            engine: None,
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

        // 2. 解析 provider + key + model（带缓存）
        // 优先级：context.__workflow_provider_id__ > profile.suggested_provider_id > 系统默认
        // 优先级：context.__workflow_model__ > profile 默认 model > 系统默认
        let session_model = context
            .variables
            .get("__workflow_model__")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let session_provider_id = context
            .variables
            .get("__workflow_provider_id__")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let (prov, key, default_model) = self
            .resolve_provider(profile.as_ref(), session_provider_id.as_deref())
            .await?;
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

        // 4. 构建 prompt：Role + Expert + 行内追加（运行时拼接，不预缓存）
        let role_desc = resolve_role(&an.config, profile.as_ref());
        let mut all_segments: Vec<TemplateSegment> = Vec::new();

        // 4a. 角色前缀
        all_segments.push(TemplateSegment::Static(format!("你是 {role_desc}。\n")));

        // 4b. AgentRole system_prompt（岗位）+ Expert system_prompt（技能）
        if let Some(ref p) = profile {
            // 解析 Role 的提示词
            if let Some(ref role_name) = p.agent_role
                && let Some(resolved) = crate::AgentRole::resolve(self.db.as_ref(), role_name).await
                && !resolved.system_prompt.is_empty()
            {
                all_segments.extend(compile_prompt(&resolved.system_prompt).segments);
            }
            // 解析 Expert 的提示词
            if let Some(ref expert_id) = p.expert_id
                && let Ok(Some(expert)) =
                    axagent_core::entity::agency_experts::Entity::find_by_id(expert_id)
                        .one(self.db.as_ref())
                        .await
                && !expert.system_prompt.is_empty()
            {
                all_segments.extend(compile_prompt(&expert.system_prompt).segments);
            }
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

        // 4e. RAG 知识源检索（从知识库/记忆/Wiki 检索相关内容注入 system prompt）
        if !an.config.rag_source_ids.is_empty() {
            if let Some(ref rag_cb) = self.rag_callback {
                let rag_query = user_prompt_for_rag(&an.config, &context.variables);
                let (kb_ids, mem_ids, wiki_ids) = parse_rag_source_ids(&an.config.rag_source_ids);
                if !kb_ids.is_empty() || !mem_ids.is_empty() || !wiki_ids.is_empty() {
                    let rag_result = rag_cb(kb_ids, mem_ids, wiki_ids, rag_query).await;
                    match rag_result {
                        Ok(result) if !result.context_parts.is_empty() => {
                            all_segments.push(TemplateSegment::Static(
                                "\n\n--- 知识库参考 ---\n".to_string(),
                            ));
                            for part in &result.context_parts {
                                all_segments.push(TemplateSegment::Static(part.clone()));
                            }
                        },
                        Ok(_) => {},
                        Err(e) => {
                            tracing::warn!(
                                "RAG context collection failed for agent node {}: {e}",
                                an.base.id
                            );
                        },
                    }
                }
            } else {
                tracing::debug!(
                    "Agent node {} has rag_source_ids but no RAG callback configured, skipping",
                    an.base.id
                );
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
        let model = session_model.unwrap_or(default_model);

        if an.config.execution_mode.as_deref() == Some("plan") {
            return self
                .execute_plan_mode(&an, context, &prov, &api_key, &model, &adapter, node)
                .await;
        }

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
        let model_for_output = model.clone();

        if context.dry_run {
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "role": role_desc, "model": model_for_output,
                    "content": "[DRY RUN] Agent 模拟输出", "thinking": null,
                    "usage": {"input_tokens":0,"output_tokens":0},
                    "tool_calls_made": [], "node_id": node.base_id(), "dry_run": true,
                }),
                output_var: Some(an.config.output_var.clone()),
            });
        }

        // 构建暴露给 LLM 的工具定义
        // 固定工具（上游 ToolNode 结果已注入 context_sources）不暴露
        // 向后兼容：exposed_tools 为空时暴露全部工具
        let exposed_list: Vec<&axagent_core::workflow_types::ToolDef> =
            if an.config.exposed_tools.is_empty() {
                an.config.tools.iter().collect()
            } else {
                an.config
                    .tools
                    .iter()
                    .filter(|td| an.config.exposed_tools.contains(&td.name))
                    .collect()
            };

        let tools: Option<Vec<axagent_core::types::ChatTool>> = if exposed_list.is_empty() {
            None
        } else {
            Some(
                exposed_list
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
            let tc_list = tool_calls.as_ref().expect("has_tool_calls ensures Some");

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

                let (result_str, is_error) = match &tool_result {
                    Ok(v) => (serde_json::to_string(v).unwrap_or_else(|_| format!("{v}")), false),
                    Err(e) => (format!("Error: {e}"), true),
                };

                tool_calls_made.push(serde_json::json!({
                    "tool": &tc.function.name,
                    "arguments": args,
                    "result": result_str,
                    "is_error": is_error,
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

impl AgentExecutor {
    /// Plan 模式：LLM 生成计划 → HierarchicalPlanner 管理 → 编译 DAG → WorkEngine 执行
    async fn execute_plan_mode(
        &self,
        an: &axagent_core::workflow_types::AgentNode,
        _context: &ExecutionState,
        prov: &axagent_core::types::ProviderConfig,
        api_key: &str,
        model: &str,
        adapter: &std::sync::Arc<dyn axagent_providers::ProviderAdapter>,
        node: &WorkflowNode,
    ) -> Result<NodeOutput, NodeError> {
        use axagent_agent::hierarchical_planner::{
            Phase, PhaseStatus, Plan, PlanStatus, PlannedTask, TaskStatus, compile_plan_to_dag,
        };
        use axagent_core::workflow_types::*;
        let role_desc = resolve_role(&an.config, None);
        let base_url =
            axagent_providers::resolve_base_url_for_type(&prov.api_host, &prov.provider_type);
        let tool_names: Vec<String> = an.config.tools.iter().map(|t| t.name.clone()).collect();

        // 1. 调用 LLM 生成 Plan（HierarchicalPlanner 格式）
        let plan_prompt = format!(
            "你是一个任务规划器。根据目标生成层次化执行计划。\n\n\
             目标: {}\n\n可用工具: {}\n\n\
             输出 JSON:\n\
             {{\"phases\":[{{\"name\":\"Phase\",\"tasks\":[\
             {{\"id\":\"t1\",\"description\":\"...\",\"action_type\":\"tool|llm|agent\",\"parameters\":{{}},\"dependencies\":[]}}\
             ]}}]}}",
            an.config.system_prompt,
            tool_names.join(", "),
        );
        let plan_ctx = axagent_providers::ProviderRequestContext {
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
        let plan_req = axagent_core::types::ChatRequest {
            model: model.to_string(),
            messages: vec![axagent_core::types::ChatMessage {
                role: "user".to_string(),
                content: axagent_core::types::ChatContent::Text(plan_prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }],
            stream: false,
            temperature: Some(0.0),
            max_tokens: Some(4096),
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
        let resp = adapter.chat(&plan_ctx, plan_req).await.map_err(|e| {
            NodeError::exec_failed(error_code::AGENT_PROFILE_NOT_FOUND, format!("Plan LLM: {e}"))
        })?;
        let text = resp.content.trim();
        let json = text
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(text);
        let plan: Plan = serde_json::from_str(json).unwrap_or(Plan {
            id: uuid::Uuid::new_v4().to_string(),
            goal: an.config.system_prompt.clone(),
            phases: vec![],
            status: PlanStatus::Draft,
            created_at: 0,
            updated_at: 0,
        });

        let phase_count = plan.phases.len();
        let task_count: u32 = plan.phases.iter().map(|p| p.tasks.len() as u32).sum();

        // 2. 编译 → DAG → WorkEngine 执行
        let plan_results = if let Some(ref engine) = self.engine {
            let (wf_nodes, wf_edges) = compile_plan_to_dag(&plan, &tool_names);
            use crate::work_engine::RunOptions;
            let wf_name = format!("plan_{}", uuid::Uuid::new_v4());
            match engine.create_workflow(&wf_name, wf_nodes, wf_edges).await {
                Ok(wf) => match engine.run_workflow(&wf.id, RunOptions::default()).await {
                    Ok(result) => serde_json::Value::Object(result.results.into_iter().collect()),
                    Err(e) => serde_json::json!({"error": format!("{e:?}")}),
                },
                Err(e) => serde_json::json!({"error": format!("{e:?}")}),
            }
        } else {
            serde_json::json!({"error": "Plan engine not available"})
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "mode":"plan","role":role_desc,"model":model.to_string(),
                "phases":phase_count,"tasks":task_count,"results":plan_results,
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
        context_provider_id: Option<&str>,
    ) -> Result<
        (axagent_core::types::ProviderConfig, axagent_core::types::ProviderKey, String),
        NodeError,
    > {
        // 仅当无 profile 指定 provider 且无上下文 provider 时使用缓存
        let has_override = profile
            .and_then(|p| p.suggested_provider_id.as_ref())
            .is_some()
            || context_provider_id.is_some();
        if !has_override {
            let cache = self.default_provider_cache.lock().await;
            if let Some(ref cached) = *cache {
                return Ok(cached.clone());
            }
        }

        // 优先级：上下文 provider_id > profile.suggested_provider_id > 系统默认
        let target_provider_id = context_provider_id
            .or_else(|| profile.and_then(|p| p.suggested_provider_id.as_deref()));

        let result = if let Some(target_id) = target_provider_id {
            let all = axagent_core::repo::provider::list_providers(&self.db)
                .await
                .map_err(|e| {
                    NodeError::exec_failed(
                        error_code::AGENT_PROFILE_NOT_FOUND,
                        format!("Provider query failed: {e}"),
                    )
                })?;
            if let Some(sp) = all.into_iter().find(|pr| pr.id == target_id && pr.enabled) {
                let key = sp.keys.iter().find(|k| k.enabled).cloned().ok_or_else(|| {
                    NodeError::exec_failed(
                        error_code::AGENT_PROFILE_NOT_FOUND,
                        "指定的 provider 无可用 key".to_string(),
                    )
                })?;
                let sm = sp
                    .models
                    .iter()
                    .find(|m| m.enabled)
                    .map(|m| m.model_id.clone());
                let model = sm.ok_or_else(|| {
                    NodeError::exec_failed(
                        error_code::AGENT_PROFILE_NOT_FOUND,
                        "指定的 provider 无可用模型".to_string(),
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
        }?;

        // 仅将"默认 provider"结果写入缓存（有指定 provider 时不缓存）
        if !has_override {
            let mut cache = self.default_provider_cache.lock().await;
            *cache = Some(result.clone());
        }

        Ok(result)
    }
}

// ── 自由函数 ──

/// 解析角色描述：从 AgentProfile 获取，无 Profile 时默认 "executor"
fn resolve_role(
    _config: &axagent_core::workflow_types::AgentNodeConfig,
    profile: Option<&axagent_core::entity::agent_profiles::Model>,
) -> String {
    if let Some(p) = profile
        && let Some(ref role) = p.agent_role
    {
        return role.clone();
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

fn parse_rag_source_ids(ids: &[String]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut kb = Vec::new();
    let mut mem = Vec::new();
    let mut wiki = Vec::new();
    for id in ids {
        if let Some(rest) = id.strip_prefix("knowledge:") {
            kb.push(rest.to_string());
        } else if let Some(rest) = id.strip_prefix("memory:") {
            mem.push(rest.to_string());
        } else if let Some(rest) = id.strip_prefix("wiki:") {
            wiki.push(rest.to_string());
        }
    }
    (kb, mem, wiki)
}

fn user_prompt_for_rag(
    config: &axagent_core::workflow_types::AgentNodeConfig,
    variables: &std::collections::HashMap<String, Value>,
) -> String {
    if !config.context_sources.is_empty() {
        config
            .context_sources
            .iter()
            .filter_map(|s| variables.get(s).map(|v| format!("{s}: {v}")))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        variables
            .iter()
            .filter(|(k, _)| !k.starts_with("__"))
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
