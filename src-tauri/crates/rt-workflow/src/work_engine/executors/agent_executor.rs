//! Agent 执行器 —— 支持 inline role 和 agent_profile 两种模式，均自动使用系统默认模型。
//!
//! 两阶段 prompt 处理：
//!   1. 加载时：compile_prompt() 提取 {{path}} 占位符
//!   2. 执行时：render_prompt() 用 ExecutionState.variables 填充模板

use super::provider_type_to_registry_key;
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

// ── Plan 模式回调类型 ──

use serde::Serialize;

/// Plan 模式回调集 — 应用层通过 RunOptions.plan_callbacks 注入
#[derive(Clone)]
pub struct PlanCallbacks {
    /// Plan 生成后、执行前调用。返回 Ok(true) 批准，Ok(false) 拒绝。
    pub on_plan_ready: Option<PlanApprovalCallback>,
    /// 步骤状态变化回调（推送到前端 + 写 DB）
    pub on_step_update: Option<PlanStepCallback>,
}

impl std::fmt::Debug for PlanCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlanCallbacks")
            .field("on_plan_ready", &self.on_plan_ready.is_some())
            .field("on_step_update", &self.on_step_update.is_some())
            .finish()
    }
}

pub type PlanApprovalCallback = Arc<
    dyn Fn(PlanApprovalRequest) -> Pin<Box<dyn Future<Output = Result<bool, String>> + Send>>
        + Send
        + Sync,
>;

pub type PlanStepCallback =
    Arc<dyn Fn(PlanStepEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
pub struct PlanApprovalRequest {
    pub goal: String,
    pub role_desc: String,
    pub model: String,
    pub phase_count: usize,
    pub task_count: u32,
    pub phases: Vec<PlanPhaseSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanPhaseSummary {
    pub name: String,
    pub task_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanStepEvent {
    pub node_id: String,
    pub phase_index: u32,
    pub task_index: u32,
    pub task_id: String,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

// ── AgentExecutor ──

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
    /// 由 Harness 注入的 ProviderRegistry
    provider_registry: Option<Arc<dyn axagent_harness::registry::ProviderRegistry>>,
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
            provider_registry: None,
        }
    }

    /// 注入 ProviderRegistry（由 Harness 在创建 executor 时调用）
    pub fn with_provider_registry(
        mut self,
        registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) -> Self {
        self.provider_registry = Some(registry);
        self
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
            provider_registry: None,
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
            provider_registry: None,
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
            provider_registry: None,
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
                                error_code::UNSUPPORTED_PROVIDER,
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
        // 优先级：节点 config.model > 会话 __workflow_model__/__workflow_provider_id__ > profile.suggested_provider_id > 项目默认
        let node_model = an.config.model.as_deref().filter(|m| !m.is_empty());
        let session_model = context
            .variables
            .get("__workflow_model__")
            .and_then(|v| v.as_str());
        let session_provider_id = context
            .variables
            .get("__workflow_provider_id__")
            .and_then(|v| v.as_str());
        let profile_suggested = profile
            .as_ref()
            .and_then(|p| p.suggested_provider_id.as_deref());

        let (prov, key, model) = self
            .resolve_provider(node_model, session_model, session_provider_id, profile_suggested)
            .await?;
        let api_key = axagent_core::crypto::decrypt_key(&key.key_encrypted, &self.master_key)
            .map_err(|e| {
                NodeError::exec_failed(
                    error_code::UNSUPPORTED_PROVIDER,
                    format!("API key decryption failed: {e}"),
                )
            })?;

        // 3. 创建 adapter
        use axagent_harness::{ProviderAdapter, resolve_base_url_for_type};
        let registry_key = provider_type_to_registry_key(&prov.provider_type);
        let adapter: Arc<dyn ProviderAdapter> = self
            .provider_registry
            .as_ref()
            .and_then(|reg| reg.get(registry_key))
            .ok_or_else(|| {
                NodeError::exec_failed(
                    error_code::UNSUPPORTED_PROVIDER,
                    format!("AgentExecutor 未找到 ProviderAdapter for type: {}", registry_key),
                )
            })?;

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

        if an.config.execution_mode.as_deref() == Some("plan") {
            return self
                .execute_plan_mode(an, context, &prov, &api_key, &model, &adapter, node)
                .await;
        }

        let req_ctx = axagent_harness::ProviderRequestContext {
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
                        error_code::UNSUPPORTED_PROVIDER,
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
    /// 失败时自动重规划（最多 replan_max_retries 次）
    #[allow(clippy::too_many_arguments)]
    async fn execute_plan_mode(
        &self,
        an: &axagent_core::workflow_types::AgentNode,
        _context: &ExecutionState,
        prov: &axagent_core::types::ProviderConfig,
        api_key: &str,
        model: &str,
        adapter: &std::sync::Arc<dyn axagent_harness::ProviderAdapter>,
        node: &WorkflowNode,
    ) -> Result<NodeOutput, NodeError> {
        use axagent_agent::hierarchical_planner::{
            HierarchicalPlanner, Plan, ReplanAction, ReplanReason, TaskStatus, compile_plan_to_dag,
        };
        let role_desc = resolve_role(&an.config, None);
        let base_url =
            axagent_harness::resolve_base_url_for_type(&prov.api_host, &prov.provider_type);
        let tool_names: Vec<String> = an.config.tools.iter().map(|t| t.name.clone()).collect();
        let replan_max_retries = an.config.max_tool_rounds.unwrap_or(2);

        // 1. 调用 LLM 生成 Plan
        let plan_prompt =
            build_plan_extraction_prompt(&an.config.system_prompt, &role_desc, &tool_names);
        let plan_ctx = axagent_harness::ProviderRequestContext {
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
        let resp = adapter
            .chat(
                &plan_ctx,
                axagent_core::types::ChatRequest {
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
                },
            )
            .await
            .map_err(|e| {
                NodeError::exec_failed(error_code::UNSUPPORTED_PROVIDER, format!("Plan LLM: {e}"))
            })?;

        let text = resp.content.trim();
        let json = axagent_core::extract_json_from_llm_response(text);
        let plan: Plan = serde_json::from_str(json).map_err(|e| {
            let preview = &json[..200.min(json.len())];
            NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                format!("Plan 模式 LLM 返回了无效的 JSON: {e}。原始响应前 200 字符: {preview}"),
            )
        })?;

        // 1.5 审批回调（Plan 生成后、执行前）
        if let Some(ref cbs) = _context.plan_callbacks
            && let Some(ref on_ready) = cbs.on_plan_ready
        {
            let phase_summaries: Vec<PlanPhaseSummary> = plan
                .phases
                .iter()
                .map(|p| PlanPhaseSummary {
                    name: p.name.clone(),
                    task_count: p.tasks.len(),
                })
                .collect();
            let approved = on_ready(PlanApprovalRequest {
                goal: plan.goal.clone(),
                role_desc: role_desc.clone(),
                model: model.to_string(),
                phase_count: plan.phases.len(),
                task_count: plan.phases.iter().map(|p| p.tasks.len() as u32).sum(),
                phases: phase_summaries,
            })
            .await
            .map_err(|e| {
                NodeError::exec_failed(
                    error_code::UNSUPPORTED_PROVIDER,
                    format!("Plan 审批回调失败: {e}"),
                )
            })?;
            if !approved {
                return Err(NodeError::exec_failed(
                    error_code::UNSUPPORTED_PROVIDER,
                    "用户拒绝执行此 Plan".to_string(),
                ));
            }
        }

        // 2. HierarchicalPlanner 接管：验证、执行管理、重规划
        let mut planner = HierarchicalPlanner::new().with_max_retries(replan_max_retries);
        planner.create_plan(&an.config.system_prompt, plan.phases.clone());

        if let Err(e) = planner.start_execution() {
            return Err(NodeError::exec_failed(
                error_code::UNSUPPORTED_PROVIDER,
                format!("Plan validation: {e}"),
            ));
        }

        let phase_count = plan.phases.len();
        let task_count: u32 = plan.phases.iter().map(|p| p.tasks.len() as u32).sum();
        let engine_available = self.engine.is_some();

        // 3. 编译 DAG → WorkEngine 执行，失败时重规划
        let mut current_plan = plan;
        let mut attempt = 0u32;
        let plan_results = loop {
            if !engine_available {
                return Err(NodeError::exec_failed(
                    error_code::VALIDATION_FAILED,
                    "Plan 模式需要 WorkEngine 引用，请通过 AgentExecutor::with_engine() 注入"
                        .to_string(),
                ));
            }
            let engine = self.engine.as_ref().unwrap();
            let (wf_nodes, wf_edges) = compile_plan_to_dag(&current_plan, &tool_names);
            let wf_name = format!("plan_{}_{}", uuid::Uuid::new_v4(), attempt);

            let exec_result = match engine.create_workflow(&wf_name, wf_nodes, wf_edges).await {
                Ok(wf) => engine
                    .run_workflow(&wf.id, crate::work_engine::RunOptions::default())
                    .await
                    .map(|r| (r, wf)),
                Err(e) => Err(e),
            };

            match exec_result {
                Ok((wf_result, _wf)) => {
                    let plan_mut = planner.get_plan_mut();
                    if let Some(plan) = plan_mut {
                        for (pi, phase) in plan.phases.iter_mut().enumerate() {
                            for (ti, task) in phase.tasks.iter_mut().enumerate() {
                                let key = format!("r_p{pi}_t{ti}_{}", task.id);
                                if let Some(v) = wf_result.results.get(&key) {
                                    task.status = TaskStatus::Completed;
                                    task.result = Some(v.clone());
                                }
                            }
                        }
                    }
                    // 步骤事件推送
                    if let Some(ref cbs) = _context.plan_callbacks
                        && let Some(ref on_step) = cbs.on_step_update
                    {
                        let phases_snapshot = {
                            let plan_guard = planner.get_plan();
                            plan_guard.cloned()
                        };
                        if let Some(ref p) = phases_snapshot {
                            for (pi, phase) in p.phases.iter().enumerate() {
                                for (ti, task) in phase.tasks.iter().enumerate() {
                                    on_step(PlanStepEvent {
                                        node_id: format!("p{pi}_t{ti}_{}", task.id),
                                        phase_index: pi as u32,
                                        task_index: ti as u32,
                                        task_id: task.id.clone(),
                                        status: if task.status == TaskStatus::Completed {
                                            "completed".to_string()
                                        } else {
                                            "failed".to_string()
                                        },
                                        result: task.result.clone(),
                                        error: task.error.clone(),
                                    })
                                    .await;
                                }
                            }
                        }
                    }
                    break serde_json::Value::Object(wf_result.results.into_iter().collect());
                },
                Err(e) if attempt < replan_max_retries => {
                    attempt += 1;
                    // 从 planner 获取真实的失败/待处理任务 ID
                    let failed_ids = planner.get_failed_steps();
                    let pending_ids = planner.get_pending_steps();
                    let task_ids_to_retry: Vec<String> = if failed_ids.is_empty() {
                        pending_ids
                    } else {
                        failed_ids
                    };

                    if task_ids_to_retry.is_empty() {
                        break serde_json::json!({"error": format!("Exec failed with no retryable tasks: {e:?}")});
                    }

                    let reason = ReplanReason::StepFailed {
                        task_id: task_ids_to_retry[0].clone(),
                        error: format!("{e:?}"),
                    };
                    let actions: Vec<ReplanAction> = task_ids_to_retry
                        .iter()
                        .map(|tid| ReplanAction::Retry {
                            task_id: tid.clone(),
                            modified_parameters: None,
                        })
                        .collect();

                    if planner.replan(reason, actions).is_ok() {
                        if let Some(p) = planner.get_plan().cloned() {
                            current_plan = p;
                        } else {
                            break serde_json::json!({"error": "Replan produced no plan"});
                        }
                    } else {
                        break serde_json::json!({"error": format!("Replan failed: {e:?}")});
                    }
                },
                Err(e) => {
                    break serde_json::json!({"error": format!("Exec failed: {e:?}")});
                },
            }
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "mode":"plan","role":role_desc,"model":model.to_string(),
                "phases":phase_count,"tasks":task_count,"attempts":attempt+1,"results":plan_results,
                "node_id": node.base_id(),
            }),
            output_var: Some(an.config.output_var.clone()),
        })
    }
}

/// 构建 Plan 模式 LLM 提示词，包含角色定义、JSON schema 和示例
fn build_plan_extraction_prompt(goal: &str, role_desc: &str, tool_names: &[String]) -> String {
    let tools_list = if tool_names.is_empty() {
        "无可用工具".to_string()
    } else {
        tool_names.join(", ")
    };

    format!(
        r#"你是一个任务规划专家。你的职责是将用户目标分解为可执行的结构化计划。

## 角色上下文
{role_desc}

## 输出格式
你必须**只**输出一个合法的 JSON 对象，不要包含任何其他文本。JSON schema 如下：

```json
{{
  "phases": [
    {{
      "name": "阶段名称",
      "description": "阶段描述",
      "tasks": [
        {{
          "id": "task_0",
          "description": "任务描述",
          "action_type": "agent",
          "parameters": {{"prompt": "该任务的详细执行指令"}},
          "dependencies": []
        }}
      ]
    }}
  ]
}}
```

## 字段说明
- `phases`: 阶段数组，按顺序执行。每个阶段内的任务可并行执行
- `phases[].name`: 阶段名称（简短）
- `phases[].tasks[].id`: 任务唯一标识符，使用 "task_N" 格式
- `phases[].tasks[].description`: 任务需要完成什么的简洁描述
- `phases[].tasks[].action_type`: "tool"（特定工具执行）、"agent"（通用代理）或 "llm"（纯推理）
- `phases[].tasks[].parameters`: 任务参数。对于 "agent" 类型，必须包含 "prompt" 字段
- `phases[].tasks[].dependencies`: 此任务所依赖的任务 ID 列表。对于无依赖的任务留空

## 指南
- 将复杂任务分解为 1-3 个阶段，每个阶段包含 1-5 个任务
- 阶段应顺序执行（分析 → 实现 → 验证）
- 任务应具体且可执行，避免模糊描述
- 对于需要特定工具的任务使用 action_type="tool"
- 每个任务最多 3 个依赖项

## 目标
{goal}

## 可用工具
{tools_list}"#,
    )
}
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
        node_model: Option<&str>,
        session_model: Option<&str>,
        session_provider_id: Option<&str>,
        profile_suggested_provider: Option<&str>,
    ) -> Result<
        (axagent_core::types::ProviderConfig, axagent_core::types::ProviderKey, String),
        NodeError,
    > {
        let has_override = session_provider_id.is_some()
            || profile_suggested_provider.is_some()
            || node_model.is_some();
        if !has_override {
            let cache = self.default_provider_cache.lock().await;
            if let Some(ref cached) = *cache {
                return Ok(cached.clone());
            }
        }

        let result = axagent_core::repo::provider::resolve_model_for_node(
            &self.db,
            node_model,
            session_model,
            session_provider_id,
            profile_suggested_provider,
        )
        .await
        .map_err(|e| NodeError::exec_failed(error_code::UNSUPPORTED_PROVIDER, e))?;

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
