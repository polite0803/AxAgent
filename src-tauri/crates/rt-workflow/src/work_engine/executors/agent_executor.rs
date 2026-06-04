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
use axagent_core::workflow_types::WorkflowNode;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest, RagContextResult};
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
    CompiledPrompt, DomainConstraintsFn, TemplateSegment, compile_prompt, render_prompt,
};

// 缓存类型（pub(crate) 供 WorkEngine 引用）
// 缓存 resolve_provider_and_adapter 的完整输出（含 adapter 和 api_key），
// 同一次工作流执行内多 agent 节点复用，避免重复 decrypt_key + registry.get。
pub(crate) type ProviderCache = Option<(
    axagent_harness::types::ProviderConfig,
    axagent_harness::types::ProviderKey,
    String,
    Arc<dyn axagent_harness::ProviderAdapter>,
    String,
)>;
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
    /// RAG 知识源检索回调（由 WorkEngine.set_rag_callback 共享注入，None = 未启用）
    rag_callback: Arc<std::sync::Mutex<Option<RagCallback>>>,
    /// Plan 模式：注入 WorkEngine 引用（set_engine 共享槽，None = 未启用）
    engine: Arc<std::sync::Mutex<Option<Arc<super::super::WorkEngine>>>>,
    #[allow(clippy::type_complexity)]
    /// Plan 模式：注入 PlannerAdapter（set_planner 共享槽，None = 未启用 Plan 模式）
    planner:
        Arc<std::sync::Mutex<Option<Arc<std::sync::Mutex<dyn axagent_harness::PlannerAdapter>>>>>,
    /// 默认 provider 缓存（同一次工作流执行内复用）
    default_provider_cache: Arc<Mutex<ProviderCache>>,
    /// Agent profile 缓存（同一工作流内多个节点共用同 profile 时复用）
    profile_cache: Arc<Mutex<ProfileCache>>,
    /// 由 Harness 注入的 ProviderRegistry（构造时一次性注入，运行期不可变）
    /// Option 仅用于 self::empty() 默认构造；WorkEngine::new 路径下必为 Some。
    provider_registry: Option<Arc<dyn axagent_harness::registry::ProviderRegistry>>,
    /// 领域约束注入回调（可选，None 时不注入任何约束，行为与现状完全一致）。
    ///
    /// 由主 binary 在 `inject_into_agent_executor` 中调用 `set_domain_constraints`
    /// 注册。回调参数是 role name（如 "stock-analyst"），返回 head/tail 约束块。
    ///
    /// **当前 4a-4f 段拼装逻辑尚未消费该字段**——仅作为扩展点暴露，
    /// 行为完全向后兼容。后续领域 PR（如 stock-analysis 迁移）可在 4a/4f 处
    /// 通过 `self.domain_constraints.as_ref().map(|f| f(role_name))` 注入。
    domain_constraints: Option<DomainConstraintsFn>,
}

impl AgentExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self::empty(db, master_key)
    }

    /// 内部 helper：构造不带任何注入状态的实例（所有 slot 都为 None）。
    /// 一般不应直接调用，外部请用 `with_shared_caches` + `set_provider_registry` (via HasProviderRegistry trait)。
    fn empty(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        Self {
            db,
            master_key,
            rag_callback: Arc::new(std::sync::Mutex::new(None)),
            engine: Arc::new(std::sync::Mutex::new(None)),
            planner: Arc::new(std::sync::Mutex::new(None)),
            default_provider_cache: Arc::new(Mutex::new(None)),
            profile_cache: Arc::new(Mutex::new(HashMap::new())),
            provider_registry: None,
            domain_constraints: None,
        }
    }

    /// 设置 Plan 模式用的 WorkEngine 引用（共享槽，热更新）
    pub fn set_engine(&self, engine: Arc<WorkEngine>) {
        *self
            .engine
            .lock()
            .expect("agent_executor.engine mutex poisoned") = Some(engine);
    }

    /// Builder 形式（保留向后兼容；内部走共享槽）
    pub fn with_engine(self, engine: Arc<WorkEngine>) -> Self {
        self.set_engine(engine);
        self
    }

    /// 设置 Plan 模式用的 PlannerAdapter（共享槽，热更新）
    pub fn set_planner(&self, planner: Arc<std::sync::Mutex<dyn axagent_harness::PlannerAdapter>>) {
        *self
            .planner
            .lock()
            .expect("agent_executor.planner mutex poisoned") = Some(planner);
    }

    /// Builder 形式
    pub fn with_planner(
        self,
        planner: Arc<std::sync::Mutex<dyn axagent_harness::PlannerAdapter>>,
    ) -> Self {
        self.set_planner(planner);
        self
    }

    /// 设置 RAG 回调（共享槽，热更新；传 None 表示清空）
    pub fn set_rag_callback(&self, cb: Option<RagCallback>) {
        *self
            .rag_callback
            .lock()
            .expect("agent_executor.rag_callback mutex poisoned") = cb;
    }

    /// Builder 形式（保留向后兼容；内部走共享槽）
    pub fn with_rag_callback(self, cb: RagCallback) -> Self {
        self.set_rag_callback(Some(cb));
        self
    }

    /// 设置领域约束注入回调。
    ///
    /// 行为契约：
    /// - 默认 `None` → 不注入任何领域约束，4a-4f 段拼装结果与改造前完全一致
    /// - 注册后 → 后续 PR 可在 4a/4f 处消费 `self.domain_constraints`
    ///   自行决定是否调用、何时调用（primacy head 在 4a 之前、recency tail 在 4f 之后）
    ///
    /// 纯 API 扩展点，不修改现有 4a-4f 段拼装逻辑。stock-analysis 等领域
    /// 后续 PR 自行迁移 STOCK_HARD_CONSTRAINTS / STOCK_COLLAB_REMINDER 常量。
    pub fn set_domain_constraints(&mut self, f: DomainConstraintsFn) {
        self.domain_constraints = Some(f);
    }

    /// 构造使用共享缓存的 executor（WorkEngine 内部使用，跨执行复用缓存）。
    pub fn with_shared_caches(
        db: Arc<DatabaseConnection>,
        master_key: [u8; 32],
        default_provider_cache: Arc<Mutex<ProviderCache>>,
        profile_cache: Arc<Mutex<ProfileCache>>,
    ) -> Self {
        let mut s = Self::empty(db, master_key);
        s.default_provider_cache = default_provider_cache;
        s.profile_cache = profile_cache;
        s
    }
}

impl Default for AgentExecutor {
    fn default() -> Self {
        Self::empty(Arc::new(DatabaseConnection::default()), [0u8; 32])
    }
}

impl axagent_harness::HasProviderRegistry for AgentExecutor {
    fn set_provider_registry(
        &mut self,
        registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) {
        self.provider_registry = Some(registry);
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
            .get(super::WORKFLOW_MODEL_VAR)
            .and_then(|v| v.as_str());
        let session_provider_id = context
            .variables
            .get(super::WORKFLOW_PROVIDER_ID_VAR)
            .and_then(|v| v.as_str());
        let profile_suggested = profile
            .as_ref()
            .and_then(|p| p.suggested_provider_id.as_deref());

        let (prov, key, model, adapter, api_key) = self
            .resolve_provider(node_model, session_model, session_provider_id, profile_suggested)
            .await?;

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
            let rag_cb = self
                .rag_callback
                .lock()
                .expect("rag_callback mutex poisoned")
                .clone();
            if let Some(rag_cb) = rag_cb {
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

        // 4f. 严格模式约束（当 tool_permissions.strict_mode = true 时注入尾部）
        if let Some(ref perms) = context.tool_permissions
            && perms.strict_mode
        {
            all_segments.push(TemplateSegment::Static(STRICT_MODE_INSTRUCTIONS.to_string()));
            tracing::warn!("Agent node {} strict_mode enabled", an.base.id);
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

        if an.config.execution_mode.as_deref() == Some("plan") {
            return self
                .execute_plan_mode(an, context, &prov, &api_key, &model, &adapter, node)
                .await;
        }

        let req_ctx = axagent_harness::build_provider_request_context(&prov, &key, api_key);
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

        let tools: Option<Vec<axagent_harness::types::ChatTool>> = if exposed_list.is_empty() {
            None
        } else {
            Some(
                exposed_list
                    .iter()
                    .map(|td| axagent_harness::types::ChatTool {
                        r#type: "function".to_string(),
                        function: axagent_harness::types::ChatToolFunction {
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
            let mut stream_tool_calls: Option<Vec<axagent_harness::types::ToolCall>> = None;
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

        // ── strict_mode 输出格式校验 ──
        if let Some(ref perms) = context.tool_permissions
            && perms.strict_mode
        {
            validate_strict_mode_output(&final_content, &an.config.output_mode)?;
        }

        // ── 防幻觉锚定检查 ──
        if let Some(ref hg_config) = an.config.hallucination_guard {
            if hg_config.enabled && !final_content.is_empty() {
                // 构建源上下文：从 context_sources 变量提取
                let source_context: String = if an.config.context_sources.is_empty() {
                    context
                        .variables
                        .iter()
                        .filter(|(k, _)| !k.starts_with("__"))
                        .map(|(_, v)| v.to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    an.config
                        .context_sources
                        .iter()
                        .filter_map(|s| context.variables.get(s).map(|v| v.to_string()))
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                if !source_context.is_empty() {
                    use axagent_harness::hallucination_guard::check_anchor;
                    let anchor_result =
                        check_anchor(&final_content, &source_context, hg_config.match_threshold);
                    if !anchor_result.passed {
                        tracing::warn!(
                            node_id = %node.base_id(),
                            node_type = "agent",
                            score = %anchor_result.score,
                            threshold = %hg_config.match_threshold,
                            unverified_count = %anchor_result.unverified_claims.len(),
                            "防幻觉锚定检查未通过: {}", anchor_result.details
                        );
                    }
                }
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
        prov: &axagent_harness::types::ProviderConfig,
        api_key: &str,
        model: &str,
        adapter: &std::sync::Arc<dyn axagent_harness::ProviderAdapter>,
        node: &WorkflowNode,
    ) -> Result<NodeOutput, NodeError> {
        use axagent_core::plan_compiler::compile_plan_to_dag;
        use axagent_harness::plan_types::{Plan, TaskStatus};
        let role_desc = resolve_role(&an.config, None);
        let base_url = axagent_providers::url_utils::resolve_base_url_for_type(
            &prov.api_host,
            &prov.provider_type,
        );
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
                axagent_harness::types::ChatRequest {
                    model: model.to_string(),
                    messages: vec![axagent_harness::types::ChatMessage {
                        role: "user".to_string(),
                        content: axagent_harness::types::ChatContent::Text(plan_prompt),
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

        // 2. PlannerAdapter 接管：验证、执行管理、重规划
        let planner_arc = {
            let data = self.planner.lock().expect("planner mutex poisoned");
            data.as_ref()
                .ok_or_else(|| {
                    NodeError::exec_failed(
                        error_code::VALIDATION_FAILED,
                        "Plan 模式需要 PlannerAdapter 注入，请通过 WorkEngine.with_planner() 注入"
                            .to_string(),
                    )
                })?
                .clone()
            // data dropped here
        };
        let phases_json: Vec<serde_json::Value> = plan
            .phases
            .iter()
            .map(|p| serde_json::to_value(p).unwrap_or_default())
            .collect();
        planner_arc
            .lock()
            .expect("inner planner poisoned")
            .create_plan(&an.config.system_prompt, &phases_json)
            .map_err(|e| {
                NodeError::exec_failed(error_code::VALIDATION_FAILED, format!("Plan 创建失败: {e}"))
            })?;

        planner_arc
            .lock()
            .expect("inner planner poisoned")
            .start_execution()
            .map_err(|e| {
                NodeError::exec_failed(
                    error_code::UNSUPPORTED_PROVIDER,
                    format!("Plan validation: {e}"),
                )
            })?;

        let phase_count = plan.phases.len();
        let task_count: u32 = plan.phases.iter().map(|p| p.tasks.len() as u32).sum();
        let engine_available = self.engine.lock().expect("engine mutex poisoned").is_some();

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
            let engine = self
                .engine
                .lock()
                .expect("engine mutex poisoned")
                .as_ref()
                .cloned()
                .ok_or_else(|| {
                    NodeError::exec_failed(
                        error_code::VALIDATION_FAILED,
                        "Plan 模式需要 WorkEngine 引用，请通过 AgentExecutor::with_engine() 注入"
                            .to_string(),
                    )
                })?;
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
                    for (pi, phase) in current_plan.phases.iter().enumerate() {
                        for (ti, task) in phase.tasks.iter().enumerate() {
                            let key = format!("r_p{pi}_t{ti}_{}", task.id);
                            if let Some(v) = wf_result.results.get(&key) {
                                planner_arc
                                    .lock()
                                    .expect("inner planner poisoned")
                                    .mark_task_completed(pi, ti, v.clone());
                            }
                        }
                    }
                    // 步骤事件推送
                    if let Some(ref cbs) = _context.plan_callbacks
                        && let Some(ref on_step) = cbs.on_step_update
                    {
                        let phases_snapshot = {
                            planner_arc
                                .lock()
                                .expect("inner planner poisoned")
                                .current_plan()
                                .and_then(|v| serde_json::from_value::<Plan>(v).ok())
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
                    let failed_ids: Vec<String> = planner_arc
                        .lock()
                        .expect("inner planner poisoned")
                        .get_failed_steps();
                    let pending_ids: Vec<String> = planner_arc
                        .lock()
                        .expect("inner planner poisoned")
                        .get_pending_steps();
                    let task_ids_to_retry: Vec<String> = if failed_ids.is_empty() {
                        pending_ids
                    } else {
                        failed_ids
                    };

                    if task_ids_to_retry.is_empty() {
                        break serde_json::json!({"error": format!("Exec failed with no retryable tasks: {e:?}")});
                    }

                    let reason_json = serde_json::json!({
                        "task_id": task_ids_to_retry[0].clone(),
                        "error": format!("{e:?}"),
                    });
                    let _actions_json: Vec<serde_json::Value> = task_ids_to_retry
                        .iter()
                        .map(|tid| {
                            serde_json::json!({
                                "Retry": {
                                    "task_id": tid.clone(),
                                    "modified_parameters": None::<serde_json::Value>,
                                }
                            })
                        })
                        .collect();

                    match planner_arc
                        .lock()
                        .expect("inner planner poisoned")
                        .request_replan("StepFailed", &[reason_json])
                    {
                        Ok(()) => {
                            if let Some(p) = planner_arc
                                .lock()
                                .expect("inner planner poisoned")
                                .current_plan()
                                .and_then(|v| serde_json::from_value::<Plan>(v).ok())
                            {
                                current_plan = p;
                            } else {
                                break serde_json::json!({"error": "Replan produced no plan"});
                            }
                        },
                        Err(_) => {
                            break serde_json::json!({"error": format!("Replan failed: {e:?}")});
                        },
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
    // ── 权限校验（基于 ToolPermissions） ──
    if let Some(ref perms) = context.tool_permissions {
        // 工具名黑/白名单校验（无需 category，先做名称级检查）
        if perms.forbidden_tools.iter().any(|t| t == tool_name) {
            let reason = format!("权限拒绝: 工具 '{tool_name}' 在禁止调用列表中");
            tracing::warn!("{reason}");
            return Err(reason);
        }
        if let Some(ref allowed) = perms.allowed_tools {
            if !allowed.iter().any(|t| t == tool_name) {
                let reason = format!("权限拒绝: 工具 '{tool_name}' 不在允许调用列表中");
                tracing::warn!("{reason}");
                return Err(reason);
            }
        }
    }

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
    /// 解析 provider + key + model + adapter + api_key，优先用缓存。
    /// 注意：当 profile 指定了 suggested_provider_id 时不做缓存（专用 provider），
    /// 仅对"无 profile"或"profile 无 provider 偏好"的分辨结果做缓存。
    ///
    /// 内部走公共 helper `super::resolve_provider_and_adapter()` 完成三步链
    /// （resolve_model_for_node → decrypt_key → registry.get），避免在此重复实现。
    async fn resolve_provider(
        &self,
        node_model: Option<&str>,
        session_model: Option<&str>,
        session_provider_id: Option<&str>,
        profile_suggested_provider: Option<&str>,
    ) -> Result<
        (
            axagent_harness::types::ProviderConfig,
            axagent_harness::types::ProviderKey,
            String,
            Arc<dyn axagent_harness::ProviderAdapter>,
            String,
        ),
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

        let result = super::resolve_provider_and_adapter(
            &self.db,
            &self.master_key,
            self.provider_registry.as_ref(),
            node_model,
            session_model,
            session_provider_id,
            profile_suggested_provider,
            "AgentExecutor",
        )
        .await?;

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

/// 严格模式 system prompt 约束指令 —— 当 ToolPermissions.strict_mode = true 时追加到 prompt 尾部。
///
/// 约束 LLM 仅输出结构化 JSON、不反问、不发散、不自由发挥。
const STRICT_MODE_INSTRUCTIONS: &str = r#"

## 严格模式约束

你当前处于严格执行模式，必须遵守以下规则：

1. **仅输出符合目标 schema 的 JSON**，不添加任何解释、说明或额外文本
2. **不允许反问用户** — 不要询问确认意见、不要征求许可、不要请求更多信息
3. **不允许输出与当前步骤无关的内容** — 专注于完成指定任务
4. **如果无法完成任务**，输出 `{"error": "详细原因"}`，不要自由发挥、猜测或填充缺失信息
5. **不要做额外假设** — 只基于给定的输入数据执行操作
"#;

/// strict_mode 下的输出格式校验：
/// - 当 output_mode 为 Json 时，验证 final_content 是否为合法 JSON
/// - 若格式不合法，返回错误阻止结果传递给下游
fn validate_strict_mode_output(
    final_content: &str,
    output_mode: &axagent_harness::workflow_types::OutputMode,
) -> Result<(), NodeError> {
    use axagent_harness::workflow_types::OutputMode;
    if matches!(output_mode, OutputMode::Json) {
        let trimmed = final_content.trim();
        if trimmed.is_empty() {
            tracing::warn!("strict_mode: LLM 输出为空，期望 JSON");
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "严格模式: LLM 输出为空，期望 JSON 格式",
            ));
        }
        if serde_json::from_str::<serde_json::Value>(trimmed).is_err() {
            let preview = &trimmed[..200.min(trimmed.len())];
            tracing::warn!("strict_mode: LLM 输出不是合法 JSON: {preview}");
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                format!("严格模式: LLM 输出不是合法 JSON（前 200 字符: {preview}）"),
            ));
        }
    }
    Ok(())
}
