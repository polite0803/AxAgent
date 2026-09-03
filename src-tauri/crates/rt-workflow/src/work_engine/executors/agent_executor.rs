// SPDX-License-Identifier: AGPL-3.0-only

//! Agent 执行器 —— 支持 inline role 和 agent_profile 两种模式，均自动使用系统默认模型。
//!
//! 两阶段 prompt 处理：
//!   1. 加载时：compile_prompt() 提取 {{path}} 占位符
//!   2. 执行时：render_prompt() 用 ExecutionState.variables 填充模板

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest, RagContextResult};
use axagent_harness::workflow_types::WorkflowNode;
use futures::StreamExt;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::work_engine::WorkEngine;
use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, check_cancellation_or_pause, error_code,
};
use crate::work_engine::prompt_template::{
    BuiltinVarsProvider, CompiledPrompt, DomainConstraintsFn, TemplateSegment, compile_prompt,
    render_prompt,
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

/// Profile 缓存项：profile 数据 + 写入时间戳。
///
/// 修复缺陷 6：profile_cache 一致性问题。
/// TTL 由 `PROFILE_CACHE_TTL` 控制，超时后缓存项失效，强制重新查询 DB，
/// 保证用户在执行工作流期间修改 profile 后能及时生效。
#[derive(Clone)]
pub struct CachedProfile {
    pub profile: axagent_harness::types::AgentProfile,
    pub cached_at: std::time::Instant,
}

/// Profile 缓存 TTL：60 秒。
///
/// 取 60 秒是为了在"工作流内多节点复用"和"用户修改 profile 后及时生效"之间取平衡。
/// 工作流通常在分钟级完成，60 秒 TTL 既能避免短时间内的重复查询，
/// 又能保证跨较长时间工作流的 profile 修改能被感知。
pub(crate) const PROFILE_CACHE_TTL: Duration = Duration::from_secs(60);

pub(crate) type ProfileCache = HashMap<String, CachedProfile>;

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
    master_key: [u8; 32],
    /// RAG 知识源检索回调（由 WorkEngine.set_rag_callback 共享注入，None = 未启用）
    rag_callback: Arc<parking_lot::Mutex<Option<RagCallback>>>,
    /// Plan 模式：注入 WorkEngine 引用（set_engine 共享槽，None = 未启用）
    engine: Arc<parking_lot::Mutex<Option<Arc<super::super::WorkEngine>>>>,
    #[allow(clippy::type_complexity)]
    /// Plan 模式：注入 PlannerAdapter（set_planner 共享槽，None = 未启用 Plan 模式）
    planner: Arc<
        parking_lot::Mutex<Option<Arc<parking_lot::Mutex<dyn axagent_harness::PlannerAdapter>>>>,
    >,
    /// 默认 provider 缓存（同一次工作流执行内复用）
    default_provider_cache: Arc<Mutex<ProviderCache>>,
    /// Agent profile 缓存（同一工作流内多个节点共用同 profile 时复用）
    profile_cache: Arc<Mutex<ProfileCache>>,
    /// 由 Harness 注入的 ProviderRegistry（构造时一次性注入，运行期不可变）
    /// Option 仅用于 self::empty() 默认构造；WorkEngine::new 路径下必为 Some。
    provider_registry: Option<Arc<dyn axagent_harness::registry::ProviderRegistry>>,
    /// 领域约束注入回调（可选，None 时不注入任何约束，行为与现状完全一致）。
    ///
    /// 由 WorkEngine 在每次 run_workflow 开始时通过 set_rag_callback 模式
    /// 同步转发当前注册的 DomainConstraintsFn。
    domain_constraints: Arc<parking_lot::Mutex<Option<DomainConstraintsFn>>>,
    /// 内建变量提供器（可选，None 时不注入任何内建变量，行为与现状完全一致）。
    ///
    /// 返回的 HashMap 中每个 key 对应 `{{key}}` 模板占位符。主 crate 在
    /// `as_of` 模式时通过 WorkEngine.set_builtin_vars_provider 注入
    /// `data_freshness` / `as_of_date` / `is_replay` / `data_scope` 等
    /// 跨领域通用状态。
    builtin_vars_provider: Arc<parking_lot::Mutex<Option<BuiltinVarsProvider>>>,
}

impl AgentExecutor {
    pub fn new(master_key: [u8; 32]) -> Self {
        Self::empty(master_key)
    }

    /// 内部 helper：构造不带任何注入状态的实例（所有 slot 都为 None）。
    /// 一般不应直接调用，外部请用 `with_shared_caches` + `set_provider_registry` (via HasProviderRegistry trait)。
    fn empty(master_key: [u8; 32]) -> Self {
        Self {
            master_key,
            rag_callback: Arc::new(parking_lot::Mutex::new(None)),
            engine: Arc::new(parking_lot::Mutex::new(None)),
            planner: Arc::new(parking_lot::Mutex::new(None)),
            default_provider_cache: Arc::new(Mutex::new(None)),
            profile_cache: Arc::new(Mutex::new(HashMap::new())),
            provider_registry: None,
            domain_constraints: Arc::new(parking_lot::Mutex::new(None)),
            builtin_vars_provider: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    /// 设置内建变量提供器（线程安全，可重复设置）。
    ///
    /// 主 crate 在 as-of 模式下调用此方法，传入从 `axagent_astock_data::as_of`
    /// 拉取 `data_freshness` 等跨领域通用状态的闭包。
    pub fn set_builtin_vars_provider(&self, provider: BuiltinVarsProvider) {
        let mut slot = self.builtin_vars_provider.lock();
        *slot = Some(provider);
    }

    /// 设置 Plan 模式用的 WorkEngine 引用（共享槽，热更新）
    pub fn set_engine(&self, engine: Arc<WorkEngine>) {
        *self.engine.lock() = Some(engine);
    }

    /// Builder 形式（保留向后兼容；内部走共享槽）
    pub fn with_engine(self, engine: Arc<WorkEngine>) -> Self {
        self.set_engine(engine);
        self
    }

    /// 2.5 P1:尝试把 Agent 节点委托给已注入的 `AgentTurnRunner` 执行。
    ///
    /// 此方法在 `execute` 入口处被调用。若 `AgentTurnRunner` 已注入且
    /// `is_available()=true`,则构造 `AgentTurnRequest` 调用 `run_turn`,
    /// 把返回的 `AgentTurnResult` 包装为 `NodeOutput`。
    ///
    /// **字段映射策略**(最小化):
    /// - `system_prompt`: 直接取 `AgentNodeConfig.system_prompt`
    /// - `user_input`: 从 `context.input_params` 提取(若为 JSON object 则取所有值拼接)
    /// - `tools`: 用 `tool_defs_to_chat_tools` 转换 `AgentNodeConfig.tools`
    /// - `model` / `temperature` / `max_tokens` / `max_tool_rounds`: 透传节点配置
    /// - `history`: 空 Vec(单轮委托,多轮对话由 streaming.rs 处理)
    ///
    /// **不处理的字段**(由 inline ReAct 负责,fallback 时走原路径):
    /// - context_sources 注入 / RAG 检索 / profile 加载 / domain_constraints
    /// - 模板渲染 / input_mapping / builtin_vars
    ///
    /// 因此本方法适用于"简单 Agent 节点"(无 profile / 无 RAG / 无 context_sources)。
    /// 复杂节点仍走 inline ReAct — 由调用方根据返回 Err 自动 fallback。
    async fn try_delegate_to_turn_runner(
        &self,
        runner: &Arc<dyn axagent_harness::AgentTurnRunner>,
        node: &WorkflowNode,
        an: &axagent_harness::workflow_types::AgentNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, String> {
        // 构造 system_prompt — 节点 inline prompt(未做模板渲染,简化处理)
        let system_prompt = an.config.system_prompt.clone();

        // 构造 user_input — 从 input_params 提取
        let user_input = if let Some(obj) = context.input_params.as_object() {
            obj.values().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>().join("\n")
        } else {
            context.input_params.to_string()
        };

        // 构造 tools — 用 harness 提供的转换函数
        let tools = axagent_harness::agent_turn_runner::tool_defs_to_chat_tools(&an.config.tools);

        let request = axagent_harness::AgentTurnRequest {
            execution_id: context.execution_id.clone(),
            node_id: node.base_id().to_string(),
            role_id: an.config.agent_profile_id.clone(),
            system_prompt,
            user_input,
            history: Vec::new(),
            tools,
            tool_permissions: None,
            // 注意：model 为空时表示"使用默认模型"，空字符串在此处是语义正确的默认值
            // 而非掩盖错误的手段
            model: an.config.model.clone().unwrap_or_default(),
            provider_id: None,
            temperature: an.config.temperature,
            max_tokens: an.config.max_tokens,
            max_tool_rounds: an.config.max_tool_rounds,
            workspace_dir: None,
        };

        let result = runner
            .run_turn(request)
            .await
            .map_err(|e| format!("AgentTurnRunner::run_turn failed: {e}"))?;

        // 把 AgentTurnResult 包装为 NodeOutput
        Ok(NodeOutput {
            output: serde_json::json!({
                "content": result.content,
                "thinking": result.thinking,
                "tool_calls": result.tool_calls,
                "usage": result.usage,
                "iterations": result.iterations,
                "stopped_by_limit": result.stopped_by_limit,
            }),
            output_var: Some(an.config.output_var.clone()),
            control: None,
        })
    }

    /// 设置 Plan 模式用的 PlannerAdapter（共享槽，热更新）
    pub fn set_planner(
        &self,
        planner: Arc<parking_lot::Mutex<dyn axagent_harness::PlannerAdapter>>,
    ) {
        *self.planner.lock() = Some(planner);
    }

    /// Builder 形式
    pub fn with_planner(
        self,
        planner: Arc<parking_lot::Mutex<dyn axagent_harness::PlannerAdapter>>,
    ) -> Self {
        self.set_planner(planner);
        self
    }

    /// 设置 RAG 回调（共享槽，热更新；传 None 表示清空）
    pub fn set_rag_callback(&self, cb: Option<RagCallback>) {
        *self.rag_callback.lock() = cb;
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
    pub fn set_domain_constraints(&self, f: DomainConstraintsFn) {
        *self.domain_constraints.lock() = Some(f);
    }

    /// 由 WorkEngine 在每次 run_workflow 开始前转发 domain_constraints。
    pub fn set_domain_constraints_option(&self, f: Option<DomainConstraintsFn>) {
        *self.domain_constraints.lock() = f;
    }

    /// 构造使用共享缓存的 executor（WorkEngine 内部使用，跨执行复用缓存）。
    pub fn with_shared_caches(
        master_key: [u8; 32],
        default_provider_cache: Arc<Mutex<ProviderCache>>,
        profile_cache: Arc<Mutex<ProfileCache>>,
    ) -> Self {
        let mut s = Self::empty(master_key);
        s.default_provider_cache = default_provider_cache;
        s.profile_cache = profile_cache;
        s
    }
}

impl Default for AgentExecutor {
    fn default() -> Self {
        Self::empty([0u8; 32])
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

        // 2.5 P1:AgentTurnRunner 委托(可选)。
        //
        // 若 wiring 层注入了 AgentTurnRunner 且 is_available()=true,
        // 把 Agent 节点委托给 trait 执行(支持 trajectory / 权限询问 / 压缩)。
        // 任何错误或未注入场景都 fallback 到下方 inline ReAct,不破坏现有行为。
        //
        // 注意:此处只做"入口委托",不替换 inline ReAct — streaming.rs 等自由对话
        // 路径不受影响。未来三套 ReAct 合并时再统一入口。
        //
        // 安全性:先 clone Arc<WorkEngine> 出来,释放 parking_lot::RwLock guard
        // (guard 是 !Send,不能跨 await 持有)。
        let engine_clone = {
            let guard = self.engine.lock();
            guard.as_ref().cloned()
        };
        // P1 缺陷修复：agent-loop 接缝消费改为「注册表优先、字段回退」——
        // 先查全局能力注册表里的 agent.loop（外部组件经 register_external_agent_loop
        // 可替换内置核心，运行时生效），查不到再回退 WorkEngine 字段注入的 runner
        // （兼容未接入注册表的场景）。两者通常指向同一内置实例，无行为差异。
        let runner = axagent_harness::get_capability_registry()
            .get_agent_turn_runner()
            .or_else(|| engine_clone.as_ref().and_then(|e| e.get_agent_turn_runner()));
        if let Some(runner) = runner
            && runner.is_available()
        {
            match self.try_delegate_to_turn_runner(&runner, node, an, context).await {
                Ok(output) => return Ok(output),
                Err(e) => {
                    tracing::warn!(
                        node_id = %node.base_id(),
                        error = %e,
                        "AgentTurnRunner 委托失败,fallback 到 inline ReAct"
                    );
                },
            }
        }

        // 1. 加载 agent profile（带 TTL 缓存，经 harness repository 抽象，不直接依赖 entities）
        // 修复缺陷 6：缓存项 60 秒后失效，避免用户修改 profile 后缓存仍是旧的
        let profile = if let Some(ref pid) = an.config.agent_profile_id {
            // 先尝试从缓存获取（检查 TTL）
            let cached_profile = {
                let cache = self.profile_cache.lock().await;
                cache.get(pid.as_str()).and_then(|c| {
                    if c.cached_at.elapsed() > PROFILE_CACHE_TTL {
                        tracing::debug!(
                            profile_id = %pid,
                            "Profile cache expired (TTL={:?}), will re-fetch from DB",
                            PROFILE_CACHE_TTL
                        );
                        None
                    } else {
                        Some(c.profile.clone())
                    }
                })
            };

            // 如果缓存未命中或过期，查询数据库
            if let Some(cached) = cached_profile {
                Some(cached)
            } else {
                let result = axagent_harness::repositories::agent_profile_repository()
                    .get_agent_profile(pid.as_str())
                    .await
                    .map_err(|e| {
                        NodeError::exec_failed(
                            error_code::UNSUPPORTED_PROVIDER,
                            format!("Agent profile query failed: {e}"),
                        )
                    })?;
                if let Some(ref p) = result {
                    let mut cache = self.profile_cache.lock().await;
                    cache.insert(
                        pid.clone(),
                        CachedProfile { profile: p.clone(), cached_at: std::time::Instant::now() },
                    );
                }
                result
            }
        } else {
            None
        };

        // 2. 解析 provider + key + model（带缓存）
        // 优先级：节点 config.model > 会话 __workflow_model__/__workflow_provider_id__ > profile.suggested_provider_id > 项目默认
        let node_model = an.config.model.as_deref().filter(|m| !m.is_empty());
        let session_model =
            context.variables.get(super::WORKFLOW_MODEL_VAR).and_then(|v| v.as_str());
        let session_provider_id =
            context.variables.get(super::WORKFLOW_PROVIDER_ID_VAR).and_then(|v| v.as_str());
        let profile_suggested = profile.as_ref().and_then(|p| p.suggested_provider_id.as_deref());

        let (prov, key, model, adapter, api_key) = self
            .resolve_provider(node_model, session_model, session_provider_id, profile_suggested)
            .await?;

        // 根据 provider 类型决定是否需要扫描非标准的文本式工具调用
        // 部分模型/代理不输出 OpenAI 标准 tool_calls delta，而是将工具调用
        // 嵌入文本内容中。已知的非标准场景：
        //   - Qwen 通过 CHAT2API 代理 → <|CHAT2API|tool_calls|> 格式（ProviderType::OpenAI）
        //   - Hermes/Ollama 等本地模型 → XML 风格 <tool_call>...
        // OpenAI/Anthropic/Gemini 原生 API 标准输出无需此处理
        let needs_inline_tool_parsing = matches!(
            prov.provider_type,
            axagent_harness::types::ProviderType::OpenAI
                | axagent_harness::types::ProviderType::Hermes
                | axagent_harness::types::ProviderType::Ollama
        );

        // V53 修复(扩展1): 获取模型运行时行为提示。
        // 不同模型在处理工具调用和输出格式上存在差异。
        // agnes-2.0-flash 等模型: tool_call_empty_content=true, tool_call_xml_inline=true
        // 注：ProviderAdapter trait 暂未提供 get_behavior_hints 方法，将来通过 harness 扩展注入。
        let _ = &adapter;
        let _ = &model;

        // 4. 构建 prompt：Role + Expert + 行内追加（运行时拼接，不预缓存）
        let role_desc = resolve_role(profile.as_ref());
        let role_name =
            profile.as_ref().and_then(|p| p.agent_role.as_deref()).unwrap_or("executor");
        let mut all_segments: Vec<TemplateSegment> = Vec::new();

        // 4a. 角色前缀 + 领域头部约束（primacy 锚定）
        all_segments.push(TemplateSegment::Static(format!("你是 {role_desc}。\n")));
        if let Some(dc_fn) = self.domain_constraints.lock().as_ref() {
            let blocks = dc_fn(role_name);
            if let Some(ref head) = blocks.head {
                all_segments.push(TemplateSegment::Static(format!("\n{head}\n")));
            }
        }

        // 4b. AgentRole system_prompt（角色/岗位）+ Expert system_prompt（专家）+ 节点 inline
        //
        // 三层 prompt 拼接顺序（语义层级：高 → 低）：
        //   1. AgentRole.system_prompt    —— 角色/岗位（CEO/CTO/证券投资负责人/executor）= 在组织里担什么责、怎么干活
        //   2. Expert.system_prompt       —— 专家人才（证券分析师/代码审计专家）= 具体技能
        //   3. 节点 inline system_prompt —— 工作流编辑器中本节点自定义的覆盖提示词
        //
        // 角色 system_prompt（如证券投资负责人）统一由 agent_role 承载。
        // 节点 inline prompt 可以覆盖上层指令（primacy + recency 双锚定）。
        if let Some(ref p) = profile {
            // 1. 解析 AgentRole（角色/岗位）的提示词
            if let Some(ref role_name) = p.agent_role {
                match axagent_harness::repositories::agent_role_repository()
                    .get_agent_role(role_name)
                    .await
                {
                    Ok(Some(resolved)) if !resolved.system_prompt.is_empty() => {
                        // 修复问题 15：记录 source 字段，便于追溯提示词来源
                        tracing::debug!(
                            node_id = %node.base_id(),
                            role_name = %role_name,
                            role_source = %resolved.source,
                            role_prompt_len = resolved.system_prompt.len(),
                            "AgentRole system_prompt resolved"
                        );
                        all_segments.extend(compile_prompt(&resolved.system_prompt).segments);
                    },
                    Ok(Some(_)) => {
                        tracing::debug!(
                            node_id = %node.base_id(),
                            role_name = %role_name,
                            "AgentRole system_prompt 为空，跳过"
                        );
                    },
                    Ok(None) => {
                        tracing::warn!(
                            node_id = %node.base_id(),
                            role_name = %role_name,
                            "AgentRole 不存在（role_name 在 DB 中未找到），role 提示词不会生效"
                        );
                    },
                    Err(e) => {
                        tracing::warn!(
                            node_id = %node.base_id(),
                            role_name = %role_name,
                            error = %e,
                            "AgentRole 查询失败（DB 错误），role 提示词不会生效"
                        );
                    },
                }
            }
            // 2. 解析 Expert（业务人才）的提示词
            if let Some(ref expert_id) = p.expert_id {
                match axagent_harness::repositories::agency_expert_repository()
                    .get_agency_expert(expert_id)
                    .await
                {
                    Ok(Some(expert)) if !expert.system_prompt.is_empty() => {
                        tracing::debug!(
                            node_id = %node.base_id(),
                            expert_id = %expert_id,
                            expert_source_dir = %expert.source_dir,
                            expert_prompt_len = expert.system_prompt.len(),
                            "Expert system_prompt resolved"
                        );
                        all_segments.extend(compile_prompt(&expert.system_prompt).segments);
                    },
                    Ok(Some(_)) => {
                        tracing::debug!(
                            node_id = %node.base_id(),
                            expert_id = %expert_id,
                            "Expert system_prompt 为空，跳过"
                        );
                    },
                    Ok(None) => {
                        tracing::warn!(
                            node_id = %node.base_id(),
                            expert_id = %expert_id,
                            "Expert 不存在（expert_id 无效或已删除），expert 提示词不会生效"
                        );
                    },
                    Err(e) => {
                        tracing::warn!(
                            node_id = %node.base_id(),
                            expert_id = %expert_id,
                            error = %e,
                            "Expert 查询失败（DB 错误），expert 提示词不会生效"
                        );
                    },
                }
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
            let mut missing_sources: Vec<&String> = Vec::new();
            all_segments.push(TemplateSegment::Static("\n\n--- 上游节点输出 ---\n".to_string()));
            for source in &an.config.context_sources {
                if let Some(value) = context.variables.get(source) {
                    let formatted = format_context_source(source, value);
                    all_segments.push(TemplateSegment::Static(formatted));
                } else {
                    missing_sources.push(source);
                    tracing::error!(
                        node_id = %node.base_id(),
                        context_source = %source,
                        "context_sources 变量未在 context.variables 中找到（tool 节点可能失败或未执行）"
                    );
                }
            }
            if !missing_sources.is_empty() {
                let msg = format!(
                    "⚠️ 以下上游数据源未获取到数据: {}。\n\
——你仍然必须完成指定的分析任务，基于现有数据（行情、K线、财务数据）给出分析结论。\n\
——即使部分数据缺失，也要基于可用信息给出明确的看多/看空/中性判断，不要输出占位文本。\n\
——该股票的基本面信息（代码、名称、行业）已在上下文中给出，请充分利用。\n\
——绝不允许输出'数据缺失'、'无法获取'、'工具失败'、'抱歉'等拒绝句式。\n\n",
                    missing_sources.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                );
                all_segments.push(TemplateSegment::Static(msg));
            }
        }

        // 4e. RAG 知识源检索（从知识库/记忆/Wiki 检索相关内容注入 system prompt）
        if !an.config.rag_source_ids.is_empty() {
            let rag_cb = self.rag_callback.lock().clone();
            if let Some(rag_cb) = rag_cb {
                let rag_query = user_prompt_for_rag(&an.config, &context.variables);
                let (kb_ids, mem_ids, wiki_ids) = parse_rag_source_ids(&an.config.rag_source_ids);
                if !kb_ids.is_empty() || !mem_ids.is_empty() || !wiki_ids.is_empty() {
                    let rag_result = rag_cb(kb_ids, mem_ids, wiki_ids, rag_query).await;
                    match rag_result {
                        Ok(result) if !result.context_parts.is_empty() => {
                            tracing::info!(
                                "[RAG] agent node {} 注入 {} 条知识库片段 → system prompt（--- 知识库参考 ---）",
                                an.base.id,
                                result.context_parts.len()
                            );
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
        // 注意：OutputMode::Json 和 OutputMode::Text 的约束不同——
        //   Json 节点输出纯 JSON（按照 prompt 指定的 schema），
        //   Text 节点输出自然语言 + VERDICT 标签。
        // 混用会导致 LLM 收到矛盾的格式要求，返回空响应。
        if let Some(ref perms) = context.tool_permissions
            && perms.strict_mode
        {
            let strict_instructions = if matches!(
                an.config.output_mode,
                axagent_harness::workflow_types::OutputMode::Json
            ) {
                r#"

## 严格模式约束

你当前处于严格执行模式，必须遵守以下规则：

1. **必须直接输出纯 JSON** — 按照上述 JSON schema 格式输出，不要在 JSON 前后添加任何自然语言、markdown 代码块或其他文字
2. **不允许反问用户** — 不要询问确认意见、不要征求许可、不要请求更多信息
3. **不允许输出与当前步骤无关的内容** — 专注于完成指定任务
4. **绝不允许拒绝回答** — 即使数据不足也要如实输出低评分。必须在报告中说明数据缺口，配置相关字段为低分值。禁止输出"抱歉我无法回答"或任何拒绝句式
5. **不要做额外假设** — 只基于给定的输入数据执行操作
6. **JSON 字符串值必须正确转义** — report 等长文本字段中的半角双引号 `"` 必须写成 `\"`，换行符必须写成 `\n`（反斜杠 n），而不是真实换行。这是最常见的 JSON 错误，请输出前检查
7. **工具调用后必须输出最终 JSON** — 如果你通过工具调用获得了数据（search_stock / get_stock_financials 等），请在工具执行结果之后立即输出最终的 JSON 分析结果，不要再继续调用额外的工具。**每轮最多只能调用一次工具，调用后必须基于结果输出 JSON。**
"#
            } else {
                r#"

## 严格模式约束

你当前处于严格执行模式，必须遵守以下规则：

1. **仅输出分析报告 + VERDICT 标签** — 先输出自然语言分析报告（可包含 Markdown），然后在末尾追加 `<!-- VERDICT: {...} -->` 标签
2. **不允许反问用户** — 不要询问确认意见、不要征求许可、不要请求更多信息
3. **不允许输出与当前步骤无关的内容** — 专注于完成指定任务
4. **绝不允许拒绝回答** — 即使数据不足也要如实输出低评分。必须在报告中说明数据缺口，并在 VERDICT 标签中如实填写低分值。禁止输出"抱歉我无法回答"或任何拒绝句式
5. **不要做额外假设** — 只基于给定的输入数据执行操作
"#
            };
            all_segments.push(TemplateSegment::Static(strict_instructions.to_string()));
            tracing::warn!(
                "Agent node {} strict_mode enabled (output_mode={:?})",
                an.base.id,
                an.config.output_mode
            );
        }

        // 4g. input_mapping 变量自动注入：将声明的输入变量值注入 system_prompt 尾部
        if !an.config.input_mapping.is_empty() {
            let mut injected_lines = String::new();
            // 排序确保稳定输出顺序
            let mut pairs: Vec<(&String, &String)> = an.config.input_mapping.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            for (target_key, source_key) in &pairs {
                // 使用 resolve_var_path 支持点号路径导航（与 CodeNode 保持一致）
                if let Some(value) = super::resolve_var_path(source_key, &context.variables) {
                    let formatted = match value {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    injected_lines.push_str(&format!("【{target_key}】:{formatted}\n"));
                } else {
                    tracing::debug!(
                        "Agent node {} input_mapping: resolve_var_path('{}') returned None",
                        an.base.id,
                        source_key
                    );
                }
            }
            if !injected_lines.is_empty() {
                all_segments.push(TemplateSegment::Static(format!(
                    "\n\n--- 输入上下文 ---\n{injected_lines}"
                )));
            }
        }

        // 4h. 领域尾部约束（recency 锚定）
        if let Some(dc_fn) = self.domain_constraints.lock().as_ref() {
            let blocks = dc_fn(role_name);
            if let Some(ref tail) = blocks.tail {
                all_segments.push(TemplateSegment::Static(format!("\n\n{tail}")));
            }
        }

        // 4i. 3.7 P2:TaskScene 场景化输出约束(recency 锚定)。
        //
        // `AgentNodeConfig.task_scene` 由用户在工作流编辑器中显式指定:
        // - `None` / `Some(General)` → 无约束
        // - `Some(Code)` → 强调直接给代码、少废话
        // - `Some(Research)` → 强调结构化分析、引用、权衡
        // - `Some(Auto)` → 由 `TaskScene::infer(user_prompt)` 推断
        //
        // 由于此处 `user_prompt` 尚未构造,先取 context_sources 拼出预览文本用于推断;
        // Auto 模式下若 context 为空则按 General 处理(无指令注入)。
        let resolved_scene = match an.config.task_scene {
            Some(axagent_harness::TaskScene::Auto) | None => {
                // None 等价于 General(无约束),Auto 需推断
                if matches!(an.config.task_scene, Some(axagent_harness::TaskScene::Auto)) {
                    let preview = an
                        .config
                        .context_sources
                        .iter()
                        .filter_map(|s| context.variables.get(s).map(|v| v.to_string()))
                        .collect::<Vec<_>>()
                        .join(" ");
                    if preview.is_empty() {
                        None
                    } else {
                        let inferred = axagent_harness::TaskScene::infer(&preview);
                        if matches!(inferred, axagent_harness::TaskScene::General) {
                            None
                        } else {
                            Some(inferred)
                        }
                    }
                } else {
                    None
                }
            },
            Some(axagent_harness::TaskScene::General) => None,
            Some(
                scene @ (axagent_harness::TaskScene::Code | axagent_harness::TaskScene::Research),
            ) => Some(scene),
        };
        if let Some(scene) = resolved_scene {
            let directive = scene.concise_directive();
            if !directive.is_empty() {
                all_segments.push(TemplateSegment::Static(format!("\n\n{directive}")));
            }
        }

        let compiled = CompiledPrompt { segments: all_segments, variable_refs: Vec::new() };

        // 拉取内建变量(可选)。由主 crate 在 as-of 模式下注入 data_freshness / as_of_date 等
        // 跨领域通用状态;None 时行为与历史完全一致。
        let builtin_vars: Option<std::collections::HashMap<String, String>> =
            self.builtin_vars_provider.lock().as_ref().map(|provider| provider());
        // 内建变量注入 variables（若有），确保模板渲染时 `{{data_freshness}}` 等占位符可解析
        let mut enriched_variables = context.variables.clone();
        if let Some(ref vars) = builtin_vars {
            for (k, v) in vars {
                enriched_variables.insert(k.clone(), Value::String(v.clone()));
            }
        }
        let system_prompt = render_prompt(&compiled, &enriched_variables).map_err(|e| {
            NodeError::exec_failed(
                error_code::VARIABLE_NOT_FOUND,
                format!("Prompt rendering failed: {e}"),
            )
        })?;

        // 5. 构建 user_prompt：始终包含用户原始消息 + context_sources 变量
        let user_message =
            context.variables.get(super::USER_MESSAGE_VAR).and_then(|v| v.as_str()).unwrap_or("");

        let mut user_prompt_parts: Vec<String> = Vec::new();

        // 始终注入用户消息（若存在），作为 LLM 理解用户意图的基础
        if !user_message.is_empty() {
            user_prompt_parts.push(format!("用户消息: {user_message}"));
        }

        // 注入 context_sources 指定的变量（若有）
        if !an.config.context_sources.is_empty() {
            for source in &an.config.context_sources {
                if let Some(value) = context.variables.get(source) {
                    // 跳过 input / user_message（已在上方注入，避免重复）
                    if source == super::USER_INPUT_VAR || source == super::USER_MESSAGE_VAR {
                        continue;
                    }
                    user_prompt_parts.push(format!("{source}: {value}"));
                }
            }
        } else {
            // 向后兼容：无 context_sources 时只注入"数据变量"（节点输出 + 已知用户输入），
            // 通过 var_filter::collect_data_vars 过滤掉 100+ 模板变量（scoring_trend /
            // fscore_roe_min 等），避免把它们全部硬灌到 LLM user_prompt 里。
            // （input/user_message 已在上方注入，过滤避免重复）
            for (k, v) in super::var_filter::collect_data_vars(&context.variables) {
                if k == super::USER_INPUT_VAR || k == super::USER_MESSAGE_VAR {
                    continue;
                }
                user_prompt_parts.push(format!("{k}: {v}"));
            }
        }

        let user_prompt = user_prompt_parts.join("\n");

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
                control: None,
            });
        }

        // 构建暴露给 LLM 的工具定义
        // 固定工具（上游 ToolNode 结果已注入 context_sources）不暴露
        // 向后兼容：exposed_tools 为空时暴露全部工具
        let exposed_list: Vec<&axagent_harness::workflow_types::ToolDef> = if an
            .config
            .exposed_tools
            .is_empty()
        {
            an.config.tools.iter().collect()
        } else {
            an.config.tools.iter().filter(|td| an.config.exposed_tools.contains(&td.name)).collect()
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
        // V53 修复(M2): 连续空 content + tool 调用轮次计数器。
        // agnes-2.0-flash 等模型在工具调用后始终不生成内容，
        // 用尽 5 轮只浪费 token 和时间。2 轮连续空内容后提前终止。
        let mut consecutive_empty_tool_rounds = 0u32;

        // 进入 ReAct 循环前检查取消/暂停状态
        check_cancellation_or_pause(context).await?;

        for round in 0..max_rounds {
            // 每轮迭代前检查取消/暂停状态（修复：节点执行中途无法打断）
            check_cancellation_or_pause(context).await?;
            // V53 修复(M2): 连续 2 轮 content 为空 + 工具调用后提前终止。
            // agnes-2.0-flash 等模型在工具调用后始终不生成文本内容，
            // 用尽所有轮次浪费 token，此优化可节省 60%+ 的无效 API 调用。
            if consecutive_empty_tool_rounds >= 2 {
                tracing::warn!(
                    node_id = %node.base_id(),
                    consecutive_empty_tool_rounds,
                    round = round,
                    max_rounds,
                    "连续 {} 轮工具调用后 content 为空, 提前终止 tool 循环 (节省 {} 轮 API 调用)",
                    consecutive_empty_tool_rounds,
                    max_rounds - round,
                );
                break;
            }

            // 支持从模板变量覆盖 temperature/max_tokens（优先级：模板变量 > 节点配置）
            let runtime_temp = context
                .variables
                .get("agent_temperature")
                .and_then(|v| v.as_f64())
                .or_else(|| an.config.temperature.map(|t| t as f64));
            let runtime_max_tokens = context
                .variables
                .get("agent_max_tokens")
                .and_then(|v| v.as_u64().map(|u| u as u32))
                .or(an.config.max_tokens);
            let request = ChatRequest {
                model: model.clone(),
                messages: messages.clone(),
                stream: true,
                temperature: runtime_temp,
                max_tokens: runtime_max_tokens,
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
                response_format: None,
            };

            // 流式调用 LLM（经统一入口 execute_llm_stream，获得 PromptGuard/截断/缓存/审计）
            let llm_config = axagent_harness::LlmCallConfig::default();
            let mut stream = axagent_harness::execute_llm_stream(
                adapter.as_ref(),
                &req_ctx,
                request,
                &llm_config,
                None,
            )
            .await
            .map_err(|e| {
                NodeError::exec_failed(
                    error_code::UNSUPPORTED_PROVIDER,
                    format!("Agent LLM stream 初始化失败: {e}"),
                )
            })?;
            let mut stream_content = String::new();
            let mut stream_thinking: Option<String> = None;
            let mut stream_tool_calls: Option<Vec<axagent_harness::types::ToolCall>> = None;
            let mut stream_usage = (0u32, 0u32);

            // v8.1: per-chunk 超时，防止 LLM provider 挂起导致 engine 永久阻塞。
            // 默认 120s（v24.6: 从 60s 调到 120s）。
            // 原因：DeepSeek 等模型在大上下文（如 K-line 120 根 K 线）下的 TTFB 偶发 >60s，
            // 60s per-chunk 超时过于激进，导致首 chunk 未到就提前超时 Failed。
            // 外层还有 node_timeout 兜底，但每次 stream.next() 阻塞太久
            // 会让整个 JoinSet 卡住，其他已完成 Agent 的结果无法推进引擎。
            // #1 修复(2026-07-22): 支持节点级覆盖 — AgentNodeConfig.stream_chunk_timeout_secs。
            // 大上下文节点(如 debate-convergence: ~30k-40k input tokens)的 TTFB 偶发 >120s,
            // 配置 stream_chunk_timeout_secs=300 后单 chunk 等待可达 5 分钟。
            let chunk_timeout =
                Duration::from_secs(an.config.stream_chunk_timeout_secs.unwrap_or(120));
            while let Some(chunk) =
                tokio::time::timeout(chunk_timeout, stream.next()).await.map_err(|_| {
                    NodeError::exec_failed(
                        error_code::TIMEOUT,
                        format!(
                            "Agent LLM stream chunk timeout after {}s (round {}/{}), node={}",
                            chunk_timeout.as_secs(),
                            round + 1,
                            max_rounds,
                            node.base_id(),
                        ),
                    )
                })?
            {
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
                    // P0 修复(2026-07-23): thinking 是 delta 增量模式，必须追加而非覆盖，
                    // 否则多 chunk 流式返回时只有最后一个 chunk 的 thinking 被保留。
                    match stream_thinking {
                        Some(ref mut existing) => existing.push_str(thinking),
                        None => stream_thinking = Some(thinking.clone()),
                    }
                }
                if let Some(usage) = chunk.usage {
                    stream_usage = (usage.input_tokens, usage.output_tokens);
                }
                if chunk.tool_calls.is_some() {
                    stream_tool_calls = chunk.tool_calls;
                }
            }

            total_usage.0 += stream_usage.0;
            total_usage.1 += stream_usage.1;

            // P0 修复(2026-07-23): 多轮工具调用时，后续轮次若返回空 content（模型只输出 tool_calls
            // 或空响应），不得覆盖之前轮次已积累的有效文本，否则达到 max_rounds break 后
            // final_content 为空字符串，前端卡片显示"等待中"。
            // 修复策略：仅当本轮 stream_content 非空时更新 final_content；
            // 空轮次保留上一轮的文本内容。thinking 同理追加合并。
            if !stream_content.trim().is_empty() {
                final_content = stream_content.clone();
            }
            if let Some(ref t) = stream_thinking {
                match final_thinking {
                    Some(ref mut existing) => {
                        existing.push('\n');
                        existing.push_str(t);
                    },
                    None => final_thinking = Some(t.clone()),
                }
            }

            // 日志：LLM 返回内容为空时发出警告（帮助排查 analyst 节点无数据问题）
            let has_tc = stream_tool_calls.as_ref().is_some_and(|tc| !tc.is_empty());
            if final_content.trim().is_empty() {
                tracing::warn!(
                    node_id = %node.base_id(),
                    model = %model,
                    usage = ?stream_usage,
                    has_thinking = %final_thinking.is_some(),
                    has_tool_calls = %has_tc,
                    "Agent LLM 返回空内容 (round {}/{}, output_mode={:?})",
                    round + 1, max_rounds, an.config.output_mode
                );
                // V53 修复(M2): 空 content + 有 tool call → 记录连续空轮次
                if has_tc {
                    consecutive_empty_tool_rounds += 1;
                }
            } else {
                // 有有效内容 → 重置计数器
                consecutive_empty_tool_rounds = 0;
            }

            // 检测非标准文本式工具调用（仅对已知使用此格式的 provider 生效）。
            // 部分模型/代理（如 Qwen 通过 CHAT2API/Hermes/Ollama）不输出标准
            // tool_calls delta，而是将工具调用嵌在文本内容中。解析后注入标准
            // tool_calls 路径使工具能正常执行。
            // V53 修复(扩展1): 当 model BehaviorHints 声明 tool_call_xml_inline=true 时，
            // 即使 provider 类型不匹配也启用内联解析（适配 agnes-2.0-flash 等模型）。
            let should_parse_inline = needs_inline_tool_parsing;
            if should_parse_inline
                && stream_tool_calls.as_ref().is_none_or(|tc| tc.is_empty())
                && !stream_content.is_empty()
                && let Some(parsed) = parse_inline_tool_calls(&stream_content)
            {
                tracing::info!(
                    "从文本中解析到 {} 个内联工具调用 (format={:?})",
                    parsed.len(),
                    prov.provider_type
                );
                stream_tool_calls = Some(parsed);
                // 保留推理文本不清空：LLM 在调用工具前输出的分析思路是有价值的上下文，
                // 清空后 final_content 会始终为空（尤其当 max_tool_rounds 全部用于工具调用时）。
            }

            // 检查是否有工具调用
            let tool_calls = stream_tool_calls;
            let has_tool_calls = tool_calls.as_ref().map(|tc| !tc.is_empty()).unwrap_or(false);

            if !has_tool_calls {
                // LLM 返回纯文本，结束循环
                break;
            }

            // 处理工具调用
            let tc_list = match tool_calls.as_ref() {
                Some(list) => list,
                None => {
                    tracing::warn!(
                        target: "axagent.reliability",
                        "has_tool_calls was true but tool_calls is None; skipping tool dispatch"
                    );
                    continue;
                },
            };

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
                    serde_json::from_str(&tc.function.arguments).unwrap_or_else(|e| {
                        // P0 修复(2026-07-22): 记录反序列化失败的原始值，
                        // 便于定位 LLM 流式 tool_call arguments 累积不完整/截断问题。
                        // 根因：providers/openai.rs 流式 delta push_str 累积的 arguments
                        // 可能不完整，反序列化失败后 args=Null，下游 parse_code 返回空字符串。
                        tracing::warn!(
                            tool = %tc.function.name,
                            raw_len = tc.function.arguments.len(),
                            error = %e,
                            raw_preview = &tc.function.arguments[..tc.function.arguments.len().min(200)],
                            "LLM tool_call arguments 反序列化失败，args 将为 Null"
                        );
                        serde_json::Value::Null
                    });

                let tool_result = execute_tool(context, &tc.function.name, args.clone()).await;

                let (result_str, is_error) = match &tool_result {
                    Ok(v) => (serde_json::to_string(v).unwrap_or_else(|_| format!("{v}")), false),
                    Err(e) => (format!("Error: {e}"), true),
                };

                // P1 修复(2026-07-25): 截断大型 tool result 防止上下文膨胀。
                // 根因：资金流向(K线)、龙虎榜等数据返回数十 KB JSON，塞入 messages 后
                // 输入上下文迅速膨胀，挤占输出 token 预算，导致 final report 在 max_tokens
                // 处被截断（VERDICT 标签丢失）。
                // 限制：单条 tool result 最多保留 3000 字符（约 1500 中文字，足够 LLM 理解结论）。
                const TOOL_RESULT_MAX_CHARS: usize = 3000;
                let limited_result = if result_str.len() > TOOL_RESULT_MAX_CHARS {
                    // 安全截断：使用 floor_char_boundary 确保不切到多字节 UTF-8 字符中间
                    let end = result_str.floor_char_boundary(TOOL_RESULT_MAX_CHARS);
                    format!(
                        "{}……\n[数据过长，已截断至 {} 字符]",
                        &result_str[..end],
                        TOOL_RESULT_MAX_CHARS,
                    )
                } else {
                    result_str
                };

                tool_calls_made.push(serde_json::json!({
                    "tool": &tc.function.name,
                    "arguments": args,
                    "result": limited_result.as_str(),
                    "is_error": is_error,
                }));

                // 追加 tool 角色消息（已截断的版本）
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: ChatContent::Text(limited_result),
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                    thinking: None,
                });
            }

            // P1-3 修复(2026-07-22): 工具调用后预防性追加简短提醒，减少 GLM-5.2 空内容轮次。
            // 问题：GLM-5.2 在工具调用模式下常忽略初始 prompt 中的"工具调用后输出 JSON"要求，
            // 导致 has_tool_calls=true 但 content 为空，需要额外一轮强制总结指令补救。
            // 优化：在 tool result 后立即追加简短 system 提醒（而非等到空内容才注入长指令），
            // 让 LLM 在看到工具结果的同时就收到输出要求。
            // 仅在非最后一轮且有工具调用时注入，避免多余消息。
            if !tc_list.is_empty() && round + 1 < max_rounds {
                messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(
                        "工具数据已获取。请基于上述工具结果直接输出最终分析结果，不要再调用工具。\
                         \n**重要**：分析报告末尾必须另起一行追加 VERDICT 标签，格式如下：\
                         \n<!-- VERDICT: {\"verdict\": \"看多|偏多|中性|偏空|看空\", \"bull_score\": 0-100, \"bear_score\": 0-100, \"bull_points\": [\"2-4条看多论据\"], \"bear_points\": [\"2-4条看空论据\"], \"confidence\": 0-100} -->\
                         \n缺少 VERDICT 标签或缺少 bull_points/bear_points 的输出将被系统视为无效。"
                            .to_string(),
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                });
            }

            // 工具执行后检查取消/暂停状态
            check_cancellation_or_pause(context).await?;

            // 最后一轮即使还有 tool_calls 也结束
            if round + 1 >= max_rounds {
                break;
            }

            // V53 修复: 工具调用后 content 为空 → 注入强制总结指令
            // agnes-2.0-flash 等模型将工具调用视为"回复完成"，不生成 JSON content，
            // 导致所有轮次浪费在重复调用工具上，最终 strict_mode 降级为 fallback JSON。
            // 注入显式系统指令要求模型直接输出最终答案，不再调用工具。
            // V53(扩展1): 当 behavior_hints.tool_call_empty_content=true 时，即使
            // content 非空也注入强制总结指令（模型已知在工具调用后只返回工具调用）。
            if !tc_list.is_empty() && round + 1 < max_rounds && final_content.trim().is_empty() {
                tracing::warn!(
                    node_id = %node.base_id(),
                    round = round + 1,
                    max_rounds,
                    "tool 调用后注入强制总结指令 (第{}轮/共{}轮)",
                    round + 1, max_rounds
                );
                messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(
                        "你已经获得了足够的工具数据。现在请基于这些数据直接输出最终分析结果，不要再调用任何工具。\
                         \n如果你已获得需要的数据，直接输出最终分析报告。不需要额外确认。\
                         \n**重要**：报告末尾必须另起一行追加 VERDICT 标签，格式如下：\
                         \n<!-- VERDICT: {\"verdict\": \"看多|偏多|中性|偏空|看空\", \"bull_score\": 0-100, \"bear_score\": 0-100, \"bull_points\": [\"2-4条看多论据\"], \"bear_points\": [\"2-4条看空论据\"], \"confidence\": 0-100} -->\
                         \n缺少 VERDICT 标签或缺少 bull_points/bear_points 的输出将被系统视为无效。"
                            .to_string(),
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                });
            }
        }

        // ── VERDICT 兜底：工具调用循环结束后，若输出无 VERDICT 标签，追加一轮纯总结调用 ──
        // 根因 1：LLM 在 max_tool_rounds 内全部用于工具调用，break 时 final_content 为空。
        // 根因 2：LLM 输出被 max_tokens/模型 API 上限截断，VERDICT 标签作为最后一行被切掉。
        // 两种情况都会导致 strict_mode 降级为 fallback JSON（confidence=0），10 个分析师全部
        // "数据不足"，决策层触发保守降级。
        // 修复：追加一轮不带 tools 的 LLM 调用，明确要求输出带 VERDICT 标签的最终分析。
        // 仅对 analyst 角色（strict_mode + 有 tools 配置）生效。
        // V58 优化: 区分两种场景——
        //   (a) 截断场景（final_content 非空但无 VERDICT）：传 system prompt + 最近工具结果 +
        //       截断内容，要求 LLM 基于所有数据重写完整报告（P1 2026-07-25: 原"仅补标签"策略
        //       导致报告末尾内容永久丢失，用户看到报告突然中断）。
        //   (b) 空输出场景：用精简 messages（system + 工具结果摘要）。
        if let Some(ref perms) = context.tool_permissions
            && perms.strict_mode
            && !an.config.tools.is_empty()
        {
            let needs_verdict_retry = !final_content.trim().is_empty()
                && extract_verdict_tag(final_content.trim()).is_none()
                && serde_json::from_str::<serde_json::Value>(final_content.trim()).is_err();
            let needs_empty_retry = final_content.trim().is_empty();

            if needs_verdict_retry || needs_empty_retry {
                tracing::info!(
                    node_id = %node.base_id(),
                    content_len = final_content.len(),
                    has_verdict = %!needs_verdict_retry,
                    mode = if needs_verdict_retry { "truncated" } else { "empty" },
                    "VERDICT 兜底: 输出无 VERDICT 标签，追加纯总结轮次",
                );

                // P1 修复(2026-07-25): 截断场景改为重写完整报告而非仅补 VERDICT 标签。
                // 根因：原逻辑"只输出 VERDICT 标签本身"虽能修复标签缺失，但被截断的
                // 报告末尾内容永久丢失，用户看到报告在中间突然中断。
                // 新逻辑：传 system prompt + 最近工具结果 + 截断内容，要求基于所有数据
                // 重新生成完整报告，确保末尾包含 VERDICT 标签（类比空输出场景的 compact 策略）。
                // 场景 (a): 截断——携带原始上下文让 LLM 重写完整报告
                // 场景 (b): 空输出——用精简 messages（system + 工具结果摘要）
                let retry_messages: Vec<ChatMessage> = if needs_verdict_retry {
                    let truncated_text = final_content.trim().to_string();
                    // P1: 同空输出策略，保留 system + 最近 2 条工具结果，附加截断内容 +
                    // 重写指令。避免全量 messages 重传导致的 input 膨胀。
                    let mut compact_messages: Vec<ChatMessage> = Vec::new();
                    if let Some(first_sys) = messages.first() {
                        compact_messages.push(first_sys.clone());
                    }
                    let take = messages.len().saturating_sub(2);
                    for msg in messages.iter().skip(take.max(1)) {
                        compact_messages.push(msg.clone());
                    }
                    // 附加截断的报告内容
                    compact_messages.push(ChatMessage {
                        role: "user".to_string(),
                        content: ChatContent::Text(format!(
                            "以下是被截断的之前版本报告（末尾不完整，供参考）：\n\n{}",
                            truncated_text,
                        )),
                        tool_calls: None,
                        tool_call_id: None,
                        thinking: None,
                    });
                    // 追加重写指令
                    compact_messages.push(ChatMessage {
                        role: "system".to_string(),
                        content: ChatContent::Text(
                            "你是一位股票分析师。请基于以上工具数据和被截断的报告，\
                             重新生成一份**完整的**分析报告。\
                             \n报告正文控制在 800 字以内，重点突出关键指标解读和风险评估。\
                             \n报告末尾必须另起一行追加 VERDICT 机读标签：\
                             \n<!-- VERDICT: {{\"verdict\": \"看多|偏多|中性|偏空|看空\", \"bull_score\": 0-100整数, \"bear_score\": 0-100整数, \"bull_points\": [\"2-4条看多论据,每条不超过16字\"], \"bear_points\": [\"2-4条看空论据,每条不超过16字\"], \"confidence\": 0-100整数}} -->\
                             \n缺少 VERDICT 标签或缺少 bull_points/bear_points 的输出将被视为无效。"
                                .to_string(),
                        ),
                        tool_calls: None,
                        tool_call_id: None,
                        thinking: None,
                    });
                    compact_messages
                } else {
                    // 空输出场景：用 system prompt + 工具结果摘要（从 messages 提取最后几条）
                    // 只保留 system + 最近 2 条 tool result，避免 input 膨胀
                    let mut compact_messages: Vec<ChatMessage> = Vec::new();
                    // 保留第一条 system prompt（含上游数据 + 专家指令）
                    if let Some(first_sys) = messages.first() {
                        compact_messages.push(first_sys.clone());
                    }
                    // 保留最后 2 条消息（通常是最近的 tool result）
                    let take = messages.len().saturating_sub(2);
                    for msg in messages.iter().skip(take.max(1)) {
                        compact_messages.push(msg.clone());
                    }
                    // 追加总结指令
                    compact_messages.push(ChatMessage {
                        role: "system".to_string(),
                        content: ChatContent::Text(
                            "请基于上述数据输出最终分析报告（控制在 500 字以内），末尾追加 VERDICT 标签。\
                             \n<!-- VERDICT: {\"verdict\": \"看多|偏多|中性|偏空|看空\", \"bull_score\": 0-100, \"bear_score\": 0-100, \"bull_points\": [\"2-4条看多论据\"], \"bear_points\": [\"2-4条看空论据\"], \"confidence\": 0-100} -->\
                             \n缺少 VERDICT 标签或缺少 bull_points/bear_points 的输出将被视为无效。"
                                .to_string(),
                        ),
                        tool_calls: None,
                        tool_call_id: None,
                        thinking: None,
                    });
                    compact_messages
                };

                let retry_request = ChatRequest {
                    model: model.clone(),
                    messages: retry_messages,
                    stream: true,
                    temperature: context
                        .variables
                        .get("agent_temperature")
                        .and_then(|v| v.as_f64())
                        .or_else(|| an.config.temperature.map(|t| t as f64)),
                    max_tokens: context
                        .variables
                        .get("agent_max_tokens")
                        .and_then(|v| v.as_u64().map(|u| u as u32))
                        .or(an.config.max_tokens),
                    top_p: None,
                    tools: None, // 不传 tools，强制 LLM 只输出文本
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
                match axagent_harness::execute_llm_stream(
                    adapter.as_ref(),
                    &req_ctx,
                    retry_request,
                    &llm_config,
                    None,
                )
                .await
                {
                    Ok(mut retry_stream) => {
                        let mut retry_content = String::new();
                        let chunk_timeout =
                            Duration::from_secs(an.config.stream_chunk_timeout_secs.unwrap_or(120));
                        while let Ok(maybe_chunk) =
                            tokio::time::timeout(chunk_timeout, retry_stream.next()).await
                        {
                            match maybe_chunk {
                                Some(Ok(chunk)) => {
                                    if let Some(ref content) = chunk.content {
                                        retry_content.push_str(content);
                                    }
                                    if let Some(usage) = chunk.usage {
                                        total_usage.0 += usage.input_tokens;
                                        total_usage.1 += usage.output_tokens;
                                    }
                                },
                                Some(Err(_)) | None => break,
                            }
                        }
                        if !retry_content.trim().is_empty() {
                            // P1 修复(2026-07-25): 截断场景重试已生成完整报告，直接替换。
                            // 原逻辑：检测到 VERDICT 标签后拼接旧截断内容 + 新标签——这在新
                            // 重写模式下会导致旧截断报告 + 新报告标签的错位拼接，内容混乱。
                            // 新逻辑：截断和空输出场景统一用 retry_content 替换 final_content。
                            if needs_verdict_retry {
                                let verdict_tag = extract_verdict_tag(retry_content.trim());
                                if verdict_tag.is_some() {
                                    // 重写场景：retry 输出是完整报告 + VERDICT，直接替换
                                    final_content = retry_content;
                                    tracing::info!(
                                        node_id = %node.base_id(),
                                        retry_len = final_content.len(),
                                        "VERDICT 兜底: 截断场景重写成功(retry 含 VERDICT 标签)",
                                    );
                                } else {
                                    // 重试输出不含 VERDICT 标签（异常），直接替换后让下游 strict_mode 校验走降级
                                    final_content = retry_content;
                                    tracing::warn!(
                                        node_id = %node.base_id(),
                                        retry_len = final_content.len(),
                                        "VERDICT 兜底: 截断场景重试输出不含 VERDICT 标签，走降级路径",
                                    );
                                }
                            } else {
                                // 空输出场景：重试输出完整报告，直接替换
                                tracing::info!(
                                    node_id = %node.base_id(),
                                    retry_len = retry_content.len(),
                                    has_verdict = extract_verdict_tag(retry_content.trim()).is_some(),
                                    "VERDICT 兜底: 空输出场景重试成功，替换 final_content",
                                );
                                final_content = retry_content;
                            }
                        } else {
                            tracing::warn!(
                                node_id = %node.base_id(),
                                "VERDICT 兜底: 重试仍返回空内容，走降级路径",
                            );
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            node_id = %node.base_id(),
                            error = %e,
                            "VERDICT 兜底: 重试 LLM 调用失败，走降级路径",
                        );
                    },
                }
            }
        }

        // ── tool_json 协议拆包（最高优先级，先于 strict_mode / VERDICT 重构）──
        // Serenity 等 agent 节点输出 ```tool_json {"name":"submit_xxx","arguments":{...}} ``` 代码块，
        // 下游按 arguments 内字段下钻（a-trend-scanner.content.trends / a-chain-trendN.content.chain_nodes）。
        // 若不拆包：strict_mode 的 VERDICT 重构（下方 1557 块）会把整个块包进 report 文本，
        // 下游 resolve_var_path 无法解析（表现为 c-scorer 的 chain_analysis 取不到 chain_nodes → no_data）。
        // 必须无条件执行（不依赖输出是否合法 JSON——带 VERDICT 标签的文本输出同样要拆）。
        if !final_content.trim().is_empty() {
            if let Some(inner) = extract_tool_json_block(final_content.trim()) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&inner) {
                    if parsed.is_object() {
                        let tool_name =
                            parsed.get("name").and_then(|n| n.as_str()).unwrap_or("?").to_string();
                        let args = parsed
                            .get("arguments")
                            .or_else(|| parsed.get("input"))
                            .cloned()
                            .unwrap_or(parsed);
                        // GLM 偶发把 arguments 序列化成 JSON 字符串（双层转义），需再解析一层
                        let args = match args {
                            serde_json::Value::String(s) => {
                                serde_json::from_str::<serde_json::Value>(&s)
                                    .unwrap_or(serde_json::Value::String(s))
                            },
                            other => other,
                        };
                        if args.is_object() || args.is_array() {
                            tracing::info!(
                                node_id = %node.base_id(),
                                tool = %tool_name,
                                "通用后处理: 拆包 tool_json 代码块为结构化输出"
                            );
                            final_content = args.to_string();
                        }
                    }
                }
            }
        }

        // ── VERDICT tag 提取 + strict_mode 输出校验 ──
        // 优先尝试提取 <!-- VERDICT: {...} --> 标签：
        // 若找到 VERDICT tag，则用其内容重构 minimal JSON（analyst/debater/risk-evaluator 节点适用），
        // 全文报告保留在 `report` 字段中；若找不到 tag，走完整 strict_mode JSON 校验（portfolio-mgr 节点适用）。
        if let Some(ref perms) = context.tool_permissions
            && perms.strict_mode
        {
            let trimmed = final_content.trim().to_string();

            // 第一步：尝试提取 VERDICT tag（TradingAgents 模式：自然语言 + 末尾机读标签）
            let verdict_reconstructed = extract_verdict_tag(&trimmed).and_then(|verdict_json| {
                // 成功提取 VERDICT，用报告文本 + VERDICT 重构 minimal JSON
                let report_text = strip_verdict_tag(&trimmed);
                // V72 修复(P1): 当 LLM 仅输出 VERDICT 标签（无正文）时，
                // report_text 为空字符串，前端 VerdictView 只渲染 stance/score 标签，
                // 用户看到的只有"看多 强度:65"这类结论性标签——完全看不到分析文字。
                // 根因：LLM 有时忽略"先写分析再追加标签"的指令，只输出标签。
                // 修复：report 为空时插入标记性占位文本，防止前端渲染为空。
                let report_text = if report_text.trim().is_empty() {
                    tracing::warn!(
                        node_id = %node.base_id(),
                        verdict = %verdict_json,
                        "VERDICT tag 无正文：LLM 仅输出了 VERDICT 标签，未包含分析报告",
                    );
                    "(该辩手仅给出了结论标签，未提供详细分析文字)".to_string()
                } else {
                    report_text
                };
                let report_escaped =
                    serde_json::to_string(&report_text).unwrap_or_else(|_| "\"\"".to_string());
                let combined =
                    format!(r#"{{"report":{}, "verdict":{} }}"#, report_escaped, verdict_json);
                serde_json::from_str::<serde_json::Value>(&combined).ok()?;
                Some(combined)
            });

            if let Some(refixed) = verdict_reconstructed {
                if refixed != trimmed {
                    tracing::info!(
                        "strict_mode: 从 VERDICT tag 重构输出 (report_len={})",
                        trimmed.len()
                    );
                }
                final_content = refixed;
            } else {
                // 没有 VERDICT tag，走完整 JSON 校验（portfolio-mgr 等节点）
                let fixed = try_extract_json_fragment(&trimmed)
                    .filter(|extracted| {
                        serde_json::from_str::<serde_json::Value>(extracted).is_ok()
                    })
                    .or_else(|| {
                        let repaired = repair_json(&trimmed);
                        if repaired != trimmed
                            && serde_json::from_str::<serde_json::Value>(&repaired).is_ok()
                        {
                            Some(repaired)
                        } else {
                            None
                        }
                    })
                    .or_else(|| try_fix_truncated_json(&trimmed));

                if let Some(ref fixed_content) = fixed {
                    if fixed_content != &trimmed {
                        tracing::warn!(
                            "strict_mode: 自动修复 LLM 输出格式: {} => {}",
                            trimmed.chars().take(80).collect::<String>(),
                            fixed_content.chars().take(80).collect::<String>(),
                        );
                        final_content = fixed_content.clone();
                    }
                } else {
                    if is_refusal_plain_text(&trimmed) {
                        let error_json = serde_json::json!({
                            "error": format!("Agent refused to answer: {}", trimmed.chars().take(100).collect::<String>())
                        });
                        final_content = error_json.to_string();
                        tracing::warn!(
                            "strict_mode: 检测到模型拒绝回答，自动转为错误 JSON: {}",
                            final_content.chars().take(80).collect::<String>()
                        );
                    } else {
                        // V39 修复: strict_mode 校验失败时降级输出而非返回 NodeError。
                        // 旧的 validate_strict_mode_output(...)? 在所有 JSON 修复失败后
                        // 返回 Err(NodeError)，导致节点 Failed，下游节点拿不到输出。
                        // 降级策略：用原始文本 + 中性 VERDICT 构造可用 JSON 输出。
                        // V53 修复(M1): 当 tool 已被调用且有返回数据时，将工具执行结果
                        // 注入 tool_results_summary 字段，使下游节点有机会基于实际数据
                        // 而非纯噪音"中性/50/50"做判断。
                        if let Err(e) =
                            validate_strict_mode_output(&trimmed, &an.config.output_mode)
                        {
                            tracing::warn!(
                                "strict_mode: LLM 输出格式校验失败,降级为原始文本输出 (output_mode={:?}): {e}",
                                an.config.output_mode,
                            );
                            // H4.1 修复：主模型输出无效（空内容/格式错误）时，用 fallback_model
                            // 重试一次 LLM 调用。重试使用简化 ChatRequest（不带 tools），
                            // 避免再次进入工具调用循环；成功则替换 final_content，
                            // 失败则继续走原降级 JSON 路径。
                            let mut fallback_remedied = false;
                            if let Some(ref fb) = an.config.fallback_model
                                && fb != &model
                            {
                                tracing::info!(
                                    node_id = %an.base.id,
                                    fallback_model = %fb,
                                    primary_model = %model,
                                    "H4.1: 主模型输出无效，尝试用 fallback_model 重试",
                                );
                                match self
                                    .resolve_provider(
                                        Some(fb.as_str()),
                                        session_model,
                                        session_provider_id,
                                        profile_suggested,
                                    )
                                    .await
                                {
                                    Ok((fb_prov, fb_key, fb_model, fb_adapter, fb_api_key)) => {
                                        let fb_req_ctx =
                                            axagent_harness::build_provider_request_context(
                                                &fb_prov, &fb_key, fb_api_key,
                                            );
                                        // 支持从模板变量覆盖 temperature/max_tokens
                                        let fb_temp = context
                                            .variables
                                            .get("agent_temperature")
                                            .and_then(|v| v.as_f64())
                                            .or_else(|| an.config.temperature.map(|t| t as f64));
                                        let fb_mt = context
                                            .variables
                                            .get("agent_max_tokens")
                                            .and_then(|v| v.as_u64().map(|u| u as u32))
                                            .or(an.config.max_tokens);
                                        let fb_request = ChatRequest {
                                            model: fb_model.clone(),
                                            messages: messages.clone(),
                                            stream: true,
                                            temperature: fb_temp,
                                            max_tokens: fb_mt,
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
                                        match axagent_harness::execute_llm_stream(
                                            fb_adapter.as_ref(),
                                            &fb_req_ctx,
                                            fb_request,
                                            &llm_config,
                                            None,
                                        )
                                        .await
                                        {
                                            Ok(mut fb_stream) => {
                                                let mut fb_content = String::new();
                                                // #1: fallback 流也用节点级配置,与主流保持一致
                                                let chunk_timeout = Duration::from_secs(
                                                    an.config
                                                        .stream_chunk_timeout_secs
                                                        .unwrap_or(120),
                                                );
                                                while let Ok(maybe_chunk) = tokio::time::timeout(
                                                    chunk_timeout,
                                                    fb_stream.next(),
                                                )
                                                .await
                                                {
                                                    match maybe_chunk {
                                                        Some(Ok(chunk)) => {
                                                            if let Some(ref content) = chunk.content
                                                            {
                                                                fb_content.push_str(content);
                                                            }
                                                        },
                                                        Some(Err(_)) | None => break,
                                                    }
                                                }
                                                if !fb_content.trim().is_empty() {
                                                    tracing::info!(
                                                        node_id = %an.base.id,
                                                        fallback_model = %fb,
                                                        content_len = fb_content.len(),
                                                        "H4.1: fallback_model 重试成功，替换 final_content",
                                                    );
                                                    final_content = fb_content;
                                                    fallback_remedied = true;
                                                } else {
                                                    tracing::warn!(
                                                        node_id = %an.base.id,
                                                        fallback_model = %fb,
                                                        "H4.1: fallback_model 仍返回空输出，走降级 JSON",
                                                    );
                                                }
                                            },
                                            Err(err) => {
                                                tracing::warn!(
                                                    node_id = %an.base.id,
                                                    fallback_model = %fb,
                                                    error = %err,
                                                    "H4.1: fallback_model 流式调用失败，走降级 JSON",
                                                );
                                            },
                                        }
                                    },
                                    Err(err) => {
                                        tracing::warn!(
                                            node_id = %an.base.id,
                                            fallback_model = %fb,
                                            error = %err,
                                            "H4.1: fallback_model provider 解析失败，走降级 JSON",
                                        );
                                    },
                                }
                            }
                            // fallback 重试未成功时，走原降级 JSON 路径
                            if !fallback_remedied {
                                // 构建 tool 摘要: 若工具有实际返回数据则纳入
                                let tool_summary: Vec<serde_json::Value> = tool_calls_made
                                    .iter()
                                    .filter_map(|tc| {
                                        let result_str = tc.get("result")?.as_str()?;
                                        // 过滤掉空结果或错误结果
                                        if result_str.is_empty()
                                            || result_str.starts_with("Error:")
                                        {
                                            None
                                        } else {
                                            Some(serde_json::json!({
                                                "tool": tc.get("tool"),
                                                "arguments": tc.get("arguments"),
                                                "result_summary": result_str.chars().take(500).collect::<String>(),
                                            }))
                                        }
                                    })
                                    .collect();
                                // FIX-03: 增强 strict_mode 降级策略——标记数据异常而非伪装成正常分析
                                // V57 修复: 不再注入 bull_score=0/bear_score=0/confidence=0 零值。
                                // 零值会导致下游 portfolio-mgr 因子信号全部为 -1（(0-50)/50=-1），
                                // posterior 被拉到 0，叠加 weights_collapsed ×0.5 → confidence 恒为 0。
                                // 改为中性估值（50/50/30），让下游能区分"数据不足但非极端看空"和"看空"。
                                let fallback_json = serde_json::json!({
                                    "report": trimmed,
                                    "verdict": {
                                        "verdict": "数据不足",
                                        "bull_score": 50,
                                        "bear_score": 50,
                                        "confidence": 30,
                                        "position_pct": 50,
                                        "bull_points": ["数据不足，无法给出看多论据"],
                                        "bear_points": ["数据不足，无法给出看空论据"]
                                    },
                                    "strict_mode_fallback": true,
                                    "__data_quality_alert": true,
                                    "__untrusted": true,
                                    "strict_mode_failure_reason": "LLM 输出非标准格式，无法解析为 JSON",
                                    "tool_results_summary": tool_summary,
                                    "fallback_model_configured": an.config.fallback_model.is_some(),
                                });
                                final_content = fallback_json.to_string();
                            }
                        }
                    }
                }
            }
        }

        // ── 通用 VERDICT 标签重构（不依赖 strict_mode） ──
        // 无论 strict_mode 是否开启，只要输出包含 <!-- VERDICT: {...} --> 标签，
        // 就将其重构为 {"report": "...", "verdict": {...}} JSON 格式，
        // 使下游 resolve_var_path 能通过 content.verdict.confidence 等路径正确解析。
        //
        // V62 修复(2026-07-23): 原逻辑仅当 final_content 不是合法 JSON 时才执行重构,
        //   但 LLM 可能输出合法的裸 JSON（如 {"verdict":"中性","confidence":70,...}），
        //   此时 verdict 是字符串而非 map，下游 content.verdict.confidence 路径下钻失败。
        //   新增分支：合法 JSON 但 verdict 字段是字符串（扁平结构）时，
        //   把 verdict 相关字段提取到嵌套 map，统一为 {"report":..., "verdict":{...}}。
        if !final_content.trim().is_empty() {
            let mut parsed_opt =
                serde_json::from_str::<serde_json::Value>(final_content.trim()).ok();

            // ── tool_json 协议拆包（优先于 VERDICT 重构） ──
            // 工作流 agent 节点约定输出 ```tool_json {"name":"submit_xxx","arguments":{...}} ```
            // 代码块，下游按 content.<arguments 内字段> 下钻（如 a-trend-scanner.content.trends、
            // a-candidate-mapper.content.candidates）。不拆包则 content 保持文本，下游全部解析失败
            // （表现为 c-scorer 等节点的 input_mapping 变量为 null / Rhai Variable not found）。
            // 拆出 arguments 作为结构化 content；拆包后重新解析 parsed_opt 供后续分支使用。
            if parsed_opt.is_none() {
                if let Some(inner) = extract_tool_json_block(final_content.trim()) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&inner) {
                        if parsed.is_object() {
                            let tool_name = parsed
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("?")
                                .to_string();
                            let args = parsed
                                .get("arguments")
                                .or_else(|| parsed.get("input"))
                                .cloned()
                                .unwrap_or(parsed);
                            // GLM 偶发把 arguments 序列化成 JSON 字符串（双层转义），需再解析一层
                            let args = match args {
                                serde_json::Value::String(s) => {
                                    serde_json::from_str::<serde_json::Value>(&s)
                                        .unwrap_or(serde_json::Value::String(s))
                                },
                                other => other,
                            };
                            if args.is_object() || args.is_array() {
                                tracing::info!(
                                    node_id = %node.base_id(),
                                    tool = %tool_name,
                                    "通用后处理: 拆包 tool_json 代码块为结构化输出"
                                );
                                final_content = args.to_string();
                                parsed_opt =
                                    serde_json::from_str::<serde_json::Value>(final_content.trim())
                                        .ok();
                            }
                        }
                    }
                }
            }

            let needs_verdict_tag_reconstruct = parsed_opt.is_none();
            let needs_flat_reconstruct = parsed_opt.as_ref().is_some_and(|v| {
                v.is_object() && v.get("verdict").is_some_and(|vd| !vd.is_object())
            });

            if needs_verdict_tag_reconstruct {
                // 分支 A: 非合法 JSON → 从 VERDICT 标签提取并重构
                let trimmed = final_content.trim().to_string();
                if let Some(verdict_json) = extract_verdict_tag(&trimmed) {
                    let report_text = strip_verdict_tag(&trimmed);
                    let report_escaped =
                        serde_json::to_string(&report_text).unwrap_or_else(|_| "\"\"".to_string());
                    let combined =
                        format!(r#"{{"report":{}, "verdict":{} }}"#, report_escaped, verdict_json);
                    if serde_json::from_str::<serde_json::Value>(&combined).is_ok() {
                        tracing::info!(
                            node_id = %node.base_id(),
                            report_len = report_text.len(),
                            "通用后处理: 从 VERDICT tag 重构输出为 JSON"
                        );
                        final_content = combined;
                    }
                } else {
                    tracing::warn!(
                        node_id = %node.base_id(),
                        content_len = trimmed.len(),
                        "LLM 输出无 VERDICT 标签且非合法 JSON，analyst-brief 将标记该维度为'数据不可用'"
                    );
                }
            } else if needs_flat_reconstruct {
                // 分支 B: 合法 JSON 但 verdict 是字符串（扁平结构）→ 重构为嵌套结构
                let parsed = parsed_opt.unwrap();
                let obj = parsed.as_object().unwrap();
                let report = obj
                    .get("report")
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::String(final_content.trim().to_string()));
                // 把除了 report 之外的所有字段都放入 verdict map，
                // 避免 catalyst_level / institutional_trace / narrative_completeness 等特有字段丢失
                let verdict_map: serde_json::Map<String, serde_json::Value> = obj
                    .iter()
                    .filter(|(k, _)| k.as_str() != "report")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let combined = serde_json::json!({
                    "report": report,
                    "verdict": verdict_map
                });
                tracing::info!(
                    node_id = %node.base_id(),
                    "通用后处理: 扁平 JSON verdict(字符串) 重构为嵌套 verdict(map)"
                );
                final_content = combined.to_string();
            }
        }

        // ── 防幻觉锚定检查 ──
        // V53 修复: strict_mode 已生成 fallback JSON 时跳过后验锚定检查。
        // fallback JSON ("strict_mode_fallback": true) 是合成的中性输出，
        // 本身不包含用户内容，对其做锚定检查必然得到 score=0，除了污染日志外无意义。
        let is_fallback_content = final_content.contains("strict_mode_fallback");
        if let Some(ref hg_config) = an.config.hallucination_guard
            && hg_config.enabled
            && !final_content.is_empty()
            && !is_fallback_content
        {
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

            // P2-1 修复: source_context 过短时（<200 字符）说明上游数据严重缺失，
            // 此时锚定检查必然失败但并非 LLM 幻觉，而是数据源故障。
            // 跳过锚定检查，避免误注入 __untrusted 导致权重坍缩。
            // 仅当 source_context 足够长（≥200 字符）时才执行锚定检查。
            if source_context.len() >= 200 {
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
                    // P2-1 修复: 锚定检查未通过时注入 __untrusted=true 到 final_content
                    // 原 bug: 只打 warn 日志未注入标记，导致 portfolio-mgr.rhai 的
                    // untrusted_count 恒为 0，权重坍缩兜底从未被正确触发。
                    // 现在注入后，portfolio-mgr 能正确感知上游不可信并触发降级。
                    // 同时保留原 content 数据，只在 JSON 顶层加 __untrusted 字段。
                    if let Ok(mut parsed) =
                        serde_json::from_str::<serde_json::Value>(&final_content)
                    {
                        if parsed.is_object() {
                            parsed["__untrusted"] = serde_json::json!(true);
                            // P2-2 修复(2026-07-22): 锚定分数极低(<0.1)时注入显式数据警告。
                            // 问题：日志显示多个节点锚定分数 0.00-0.04，但报告中对数据不足
                            // 无任何提示，用户无法判断结论可信度。
                            // 优化：当 score<0.1 时在 JSON 中注入 data_sufficiency_warning 字段，
                            // 前端可据此显示显著警告横幅。
                            if anchor_result.score < 0.1 {
                                parsed["data_sufficiency_warning"] = serde_json::json!(format!(
                                    "⚠️ 数据严重不足（锚定分数 {:.2}），本节点分析结论可信度极低，仅供参考。上游数据源可能失效或返回空值。",
                                    anchor_result.score
                                ));
                            }
                            final_content = parsed.to_string();
                        }
                    }
                }
            } else if !source_context.is_empty() {
                tracing::info!(
                    node_id = %node.base_id(),
                    node_type = "agent",
                    context_len = %source_context.len(),
                    "上游数据严重不足(<200字符)，跳过锚定检查避免误触发 __untrusted"
                );
            }
        }

        // ── 通用空内容保护（不依赖 strict_mode） ──
        // P0 修复: 高并发下（如并行度=10）部分 LLM 请求可能被限流/超时/返回空流，
        // 导致 final_content 为空字符串。旧逻辑仅打 warn 日志就返回 Ok，
        // 节点被标记为 Completed 但实际无输出，前端卡片显示"等待中"。
        // 修复策略：
        //   1. 若配置了 fallback_model 且不同于主模型，用简化请求（无 tools）重试一次
        //   2. 重试失败则构造降级 JSON 输出，标记 __untrusted=true + 中性 VERDICT，
        //      确保前端能显示错误信息、下游 data-quality 能识别为低置信度。
        // 注意：必须先 clean_inline_tool_tags 再判断，否则只包含工具调用标签的输出
        //       会被误判为非空，导致前端清理后显示空内容、UI 卡片一直"等待中"
        let content_after_clean = clean_inline_tool_tags(&final_content);
        if content_after_clean.trim().is_empty() {
            tracing::error!(
                node_id = %node.base_id(),
                primary_model = %model,
                has_fallback = %an.config.fallback_model.is_some(),
                "Agent LLM 返回完全空内容，启动 fallback 补救流程"
            );

            let mut fallback_remedied = false;

            // 步骤 1: 尝试 fallback_model 重试（不带 tools，避免重复工具调用循环）
            if let Some(ref fb) = an.config.fallback_model
                && fb != &model
            {
                tracing::info!(
                    node_id = %an.base.id,
                    fallback_model = %fb,
                    primary_model = %model,
                    "通用空内容保护: 用 fallback_model 重试"
                );
                if let Ok((fb_prov, fb_key, fb_model, fb_adapter, fb_api_key)) = self
                    .resolve_provider(
                        Some(fb.as_str()),
                        session_model,
                        session_provider_id,
                        profile_suggested,
                    )
                    .await
                {
                    let fb_req_ctx = axagent_harness::build_provider_request_context(
                        &fb_prov, &fb_key, fb_api_key,
                    );
                    let fb_temp = context
                        .variables
                        .get("agent_temperature")
                        .and_then(|v| v.as_f64())
                        .or_else(|| an.config.temperature.map(|t| t as f64));
                    let fb_mt = context
                        .variables
                        .get("agent_max_tokens")
                        .and_then(|v| v.as_u64().map(|u| u as u32))
                        .or(an.config.max_tokens);
                    let fb_request = ChatRequest {
                        model: fb_model.clone(),
                        messages: messages.clone(),
                        stream: true,
                        temperature: fb_temp,
                        max_tokens: fb_mt,
                        top_p: None,
                        tools: None, // fallback 重试不带 tools，直接要求输出结果
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
                    match axagent_harness::execute_llm_stream(
                        fb_adapter.as_ref(),
                        &fb_req_ctx,
                        fb_request,
                        &llm_config,
                        None,
                    )
                    .await
                    {
                        Ok(mut fb_stream) => {
                            let mut fb_content = String::new();
                            let chunk_timeout = Duration::from_secs(
                                an.config.stream_chunk_timeout_secs.unwrap_or(120),
                            );
                            while let Ok(maybe_chunk) =
                                tokio::time::timeout(chunk_timeout, fb_stream.next()).await
                            {
                                match maybe_chunk {
                                    Some(Ok(chunk)) => {
                                        if let Some(ref content) = chunk.content {
                                            fb_content.push_str(content);
                                        }
                                    },
                                    Some(Err(_)) | None => break,
                                }
                            }
                            let fb_trimmed = fb_content.trim().to_string();
                            if !fb_trimmed.is_empty() {
                                tracing::info!(
                                    node_id = %an.base.id,
                                    fallback_model = %fb,
                                    content_len = fb_trimmed.len(),
                                    "通用空内容保护: fallback_model 重试成功"
                                );
                                fallback_remedied = true;

                                // fallback 返回的内容可能也需要 VERDICT 重构
                                let mut fb_final = fb_content;
                                if serde_json::from_str::<serde_json::Value>(&fb_trimmed).is_err() {
                                    if let Some(verdict_json) = extract_verdict_tag(&fb_trimmed) {
                                        let report_text = strip_verdict_tag(&fb_trimmed);
                                        let report_escaped = serde_json::to_string(&report_text)
                                            .unwrap_or_else(|_| "\"\"".to_string());
                                        let combined = format!(
                                            r#"{{"report":{}, "verdict":{} }}"#,
                                            report_escaped, verdict_json
                                        );
                                        if serde_json::from_str::<serde_json::Value>(&combined)
                                            .is_ok()
                                        {
                                            fb_final = combined;
                                        }
                                    }
                                }
                                final_content = fb_final;
                            } else {
                                tracing::warn!(
                                    node_id = %an.base.id,
                                    fallback_model = %fb,
                                    "通用空内容保护: fallback_model 仍返回空输出"
                                );
                            }
                        },
                        Err(err) => {
                            tracing::warn!(
                                node_id = %an.base.id,
                                fallback_model = %fb,
                                error = %err,
                                "通用空内容保护: fallback_model 流式调用失败"
                            );
                        },
                    }
                } else {
                    tracing::warn!(
                        node_id = %an.base.id,
                        fallback_model = %fb,
                        "通用空内容保护: fallback_model provider 解析失败"
                    );
                }
            }

            // 步骤 2: fallback 也失败，构造降级输出
            if !fallback_remedied {
                tracing::error!(
                    node_id = %node.base_id(),
                    model = %model,
                    "通用空内容保护: fallback 补救失败，返回降级输出"
                );
                let fallback_report = format!(
                    "## ⚠️ 分析失败\n\n\
                     该分析师节点因 LLM 服务异常（高并发限流/网络超时/服务不可用），未能生成有效分析报告。\n\
                     - 主模型: {}\n\
                     - fallback 模型: {}\n\
                     - 节点: {}\n\n\
                     本节点结论不可信，已标记为低置信度。建议降低并行度后重试。",
                    model,
                    an.config.fallback_model.as_deref().unwrap_or("(未配置)"),
                    node.base_id(),
                );
                let report_escaped =
                    serde_json::to_string(&fallback_report).unwrap_or_else(|_| "\"\"".to_string());
                let verdict = serde_json::json!({
                    "verdict": "数据不足",
                    "bull_score": 0,
                    "bear_score": 0,
                    "confidence": 0,
                    "position_pct": 0
                });
                final_content = format!(
                    r#"{{"report":{}, "verdict":{}, "__untrusted":true, "strict_mode_fallback":true, "fallback_reason":"LLM 返回空内容且 fallback 重试失败"}}"#,
                    report_escaped, verdict
                );
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
            control: None,
        })
    }
}

impl AgentExecutor {
    /// Plan 模式：LLM 生成计划 → HierarchicalPlanner 管理 → 编译 DAG → WorkEngine 执行
    /// 失败时自动重规划（最多 replan_max_retries 次）
    #[allow(clippy::too_many_arguments)]
    async fn execute_plan_mode(
        &self,
        an: &axagent_harness::workflow_types::AgentNode,
        _context: &ExecutionState,
        prov: &axagent_harness::types::ProviderConfig,
        api_key: &str,
        model: &str,
        adapter: &std::sync::Arc<dyn axagent_harness::ProviderAdapter>,
        node: &WorkflowNode,
    ) -> Result<NodeOutput, NodeError> {
        use axagent_harness::plan_types::{Plan, TaskStatus};
        use axagent_kit::plan_compiler::compile_plan_to_dag;
        let role_desc = resolve_role(None);
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
        let llm_config = axagent_harness::LlmCallConfig::default();
        let resp = axagent_harness::execute_llm(
            &**adapter,
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
                response_format: None,
            },
            &llm_config,
        )
        .await
        .map_err(|e| {
            NodeError::exec_failed(error_code::UNSUPPORTED_PROVIDER, format!("Plan LLM: {e}"))
        })?;

        let text = resp.response.content.trim();
        let json = axagent_kit::utils::extract_json_from_llm_response(text);
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
                .map(|p| PlanPhaseSummary { name: p.name.clone(), task_count: p.tasks.len() })
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
            let data = self.planner.lock();
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
            .map(|p| match serde_json::to_value(p) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Plan 阶段序列化失败: {e}, 使用 Null 占位");
                    serde_json::Value::Null
                },
            })
            .collect();
        // Bundle planner operations into a single lock scope to avoid TOCTOU
        {
            let mut planner = planner_arc.lock();
            planner.create_plan(&an.config.system_prompt, &phases_json).map_err(|e| {
                NodeError::exec_failed(error_code::VALIDATION_FAILED, format!("Plan 创建失败: {e}"))
            })?;
            planner.start_execution().map_err(|e| {
                NodeError::exec_failed(
                    error_code::UNSUPPORTED_PROVIDER,
                    format!("Plan validation: {e}"),
                )
            })?;
        }

        let phase_count = plan.phases.len();
        let task_count: u32 = plan.phases.iter().map(|p| p.tasks.len() as u32).sum();
        let engine_available = self.engine.lock().is_some();

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
            let engine = self.engine.lock().as_ref().cloned().ok_or_else(|| {
                NodeError::exec_failed(
                    error_code::VALIDATION_FAILED,
                    "Plan 模式需要 WorkEngine 引用，请通过 AgentExecutor::with_engine() 注入"
                        .to_string(),
                )
            })?;
            let (wf_nodes, wf_edges) =
                compile_plan_to_dag(&current_plan, &tool_names, an.config.agent_profile_id.clone());
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
                    // Bundle all mark_task_completed calls into a single lock scope
                    {
                        let mut planner = planner_arc.lock();
                        for (pi, phase) in current_plan.phases.iter().enumerate() {
                            for (ti, task) in phase.tasks.iter().enumerate() {
                                let key = format!("r_p{pi}_t{ti}_{}", task.id);
                                if let Some(v) = wf_result.results.get(&key) {
                                    planner.mark_task_completed(pi, ti, v.clone());
                                }
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
                    let failed_ids: Vec<String> = planner_arc.lock().get_failed_steps();
                    let pending_ids: Vec<String> = planner_arc.lock().get_pending_steps();
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

                    match planner_arc.lock().request_replan("StepFailed", &[reason_json]) {
                        Ok(()) => {
                            // Re-read from the same planner lock scope
                            current_plan = planner_arc
                                .lock()
                                .current_plan()
                                .and_then(|v| serde_json::from_value::<Plan>(v).ok())
                                .unwrap_or_else(|| current_plan.clone());
                            if current_plan.phases.is_empty() {
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
            control: None,
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
        if let Some(ref allowed) = perms.allowed_tools
            && !allowed.iter().any(|t| t == tool_name)
        {
            let reason = format!("权限拒绝: 工具 '{tool_name}' 不在允许调用列表中");
            tracing::warn!("{reason}");
            return Err(reason);
        }
    }

    let cb = context
        .callbacks
        .as_ref()
        .and_then(|cbs| cbs.tool_handlers.get(tool_name).cloned())
        .or_else(|| context.callbacks.as_ref().and_then(|cbs| cbs.tool_fallback.clone()));

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
///
/// 修复问题 11：删除未使用的 `_config` 参数（原为预留扩展点但从未实现）。
fn resolve_role(profile: Option<&axagent_harness::types::AgentProfile>) -> String {
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

    // 检测空结果并注入警告（空数组/空对象/空字符串）
    let is_empty = body.trim().is_empty()
        || body.trim() == "[]"
        || body.trim() == "{}"
        || body.trim() == "null"
        || (target.is_array() && target.as_array().is_some_and(|a| a.is_empty()))
        || (target.is_string() && target.as_str().is_some_and(|s| s.trim().is_empty()));
    if is_empty {
        return format!("⚠️ [{name}] 输出为空：该数据源无可用记录，请基于已有数据生成保守分析\n\n");
    }

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
    config: &axagent_harness::workflow_types::AgentNodeConfig,
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

/// 从 LLM 输出的文本内容中解析非标准的工具调用格式。
///
/// 部分模型/代理（如 Qwen 通过 CHAT2API）不输出 OpenAI 标准格式的
/// tool_calls，而是将工具调用嵌在文本中。此函数检测并解析这些格式：
///   <|CHAT2API|tool_calls><|CHAT2API|invoke name="fn"><|CHAT2API|parameter name="p"><![CDATA[v]]></|CHAT2API|parameter></|CHAT2API|invoke><|CHAT2API|tool_calls>
///   <tool_call><function=name><parameter=key>value</parameter></function></tool_call> (XML 风格)
/// 解析成功后返回 `Some(Vec<ToolCall>)`，调用方应将 `stream_content` 清空
/// 并将解析结果作为标准 `tool_calls` 处理。
fn parse_inline_tool_calls(text: &str) -> Option<Vec<axagent_harness::types::ToolCall>> {
    // 先尝试 CHAT2API 格式
    if let Some(results) = parse_chat2api_format(text) {
        return Some(results);
    }
    // 再尝试 XML <tool_call> 格式（agi X-2.0-flash 等模型使用）
    parse_xml_tool_call_format(text)
}

/// 清理 LLM 输出中内联的工具调用标签，判断剩余内容是否为有效非空文本。
///
/// 问题场景：部分模型只输出工具调用标签（如 <tool_call>...</tool_call>、
/// <|CHAT2API|tool_calls>...</|CHAT2API|tool_calls>）而不输出任何实际分析文本，
/// 导致 final_content 看似非空但前端 cleanToolCallTags 清理后变为空字符串，
/// UI 卡片一直显示"等待中"。
///
/// 此函数移除所有已知的内联工具调用标签格式，返回剩余文本的 trim 结果，
/// 用于判断是否有实际有效内容。
fn clean_inline_tool_tags(text: &str) -> String {
    let mut cleaned = text.to_string();

    // CHAT2API 格式: <|CHAT2API|tool_calls>...</|CHAT2API|tool_calls>
    // 使用简单的字符串替换（非贪婪，处理多段）
    while let Some(start) = cleaned.find("<|CHAT2API|tool_calls>") {
        if let Some(end) = cleaned[start..].find("</|CHAT2API|tool_calls>") {
            let end_pos = start + end + "</|CHAT2API|tool_calls>".len();
            cleaned.replace_range(start..end_pos, "");
        } else {
            // 只有开标签没有闭标签，移除开标签之后的所有内容
            cleaned.truncate(start);
            break;
        }
    }

    // XML 格式: <tool_call>...</tool_call>
    while let Some(start) = cleaned.find("<tool_call>") {
        if let Some(end) = cleaned[start..].find("</tool_call>") {
            let end_pos = start + end + "</tool_call>".len();
            cleaned.replace_range(start..end_pos, "");
        } else {
            // 只有开标签，移除开标签之后的所有内容
            if let Some(close) = cleaned[start..].find("/>") {
                cleaned.replace_range(start..start + close + 2, "");
            } else {
                cleaned.truncate(start);
                break;
            }
        }
    }

    // 清理所有 HTML/XML 风格标签的兜底（简单匹配 <...> 模式，避免过度删除正常文本）
    // 只移除已知的工具相关标签，不移除任意尖括号内容
    let tag_patterns = &[
        "<|CHAT2API|invoke",
        "<|CHAT2API|parameter",
        "</|CHAT2API|invoke>",
        "</|CHAT2API|parameter>",
        "<![CDATA[",
        "]]>",
    ];
    for pat in tag_patterns {
        cleaned = cleaned.replace(pat, "");
    }

    cleaned.trim().to_string()
}

/// 解析 CHAT2API 格式的 inline tool calls
fn parse_chat2api_format(text: &str) -> Option<Vec<axagent_harness::types::ToolCall>> {
    let tool_calls_start = text.find("<|CHAT2API|tool_calls>")?;
    let tool_calls_end = text.rfind("<|CHAT2API|tool_calls>")?;
    if tool_calls_start == tool_calls_end {
        return None; // 只有开没有闭，格式不对
    }
    let section = &text[tool_calls_start..tool_calls_end];

    let mut results = Vec::new();
    let mut search = section;
    while let Some(invoke_start) = search.find("<|CHAT2API|invoke name=\"") {
        let name_start = invoke_start + 26; // len of "<|CHAT2API|invoke name=\""
        let name_end = match search[name_start..].find('"') {
            Some(p) => name_start + p,
            None => break,
        };
        let tool_name = &search[name_start..name_end];

        // 从 name_end 之后开始找第一个 parameter 或 invoke 结束
        let after_name = &search[name_end..];
        let close_invoke = match after_name.find("</|CHAT2API|invoke>") {
            Some(p) => name_end + p,
            None => break,
        };
        let params_section = &search[name_end..close_invoke];

        // 解析 parameter
        let mut args_map = serde_json::Map::new();
        let mut param_search = params_section;
        while let Some(param_start) = param_search.find("<|CHAT2API|parameter name=\"") {
            let pname_start_pos = param_start + 29;
            let pname_end = match param_search[pname_start_pos..].find('"') {
                Some(p) => pname_start_pos + p,
                None => break,
            };
            let param_name = &param_search[pname_start_pos..pname_end];

            let after_name = &param_search[pname_end..];
            let cdata_start = match after_name.find("<![CDATA[") {
                Some(p) => p + 9,
                None => break,
            };
            let cdata_end = match after_name[cdata_start..].find("]]>") {
                Some(p) => cdata_start + p,
                None => break,
            };
            let param_value = &after_name[cdata_start..cdata_end];

            args_map
                .insert(param_name.to_string(), serde_json::Value::String(param_value.to_string()));

            param_search = &after_name[cdata_end + 3..];
        }

        let args_json = serde_json::Value::Object(args_map);
        let arguments_str = match serde_json::to_string(&args_json) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("inline tool call 参数序列化失败: {e}, 使用空字符串");
                String::new()
            },
        };

        results.push(axagent_harness::types::ToolCall {
            id: format!("inline-{}", results.len()),
            call_type: "function".to_string(),
            function: axagent_harness::types::ToolCallFunction {
                name: tool_name.to_string(),
                arguments: arguments_str,
            },
        });

        search = &search[search.len().min(close_invoke + 19)..]; // skip </|CHAT2API|invoke> (19字节)
    }

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

/// 解析 XML 格式的 inline tool calls：
///   <tool_call>
///   <function=name>
///   <parameter=key>value</parameter>
///   </function>
///   </tool_call>
///
/// 这种格式由 agnes-2.0-flash 等模型在无法使用标准 tool_calls delta 时的输出。
fn parse_xml_tool_call_format(text: &str) -> Option<Vec<axagent_harness::types::ToolCall>> {
    if !text.contains("<tool_call>") {
        return None;
    }

    let mut results = Vec::new();
    let mut remaining = text;

    while let Some(tc_start) = remaining.find("<tool_call>") {
        let content_start = tc_start + "<tool_call>".len();
        let tc_end = match remaining[content_start..].find("</tool_call>") {
            Some(p) => content_start + p,
            None => break,
        };
        let section = &remaining[content_start..tc_end];

        // 提取 <function=name>...</function>
        let fn_name = if let Some(fn_start) = section.find("<function=") {
            let after_fn_open = fn_start + "<function=".len();
            let name_end = match section[after_fn_open..].find('>') {
                Some(p) => after_fn_open + p,
                None => break,
            };
            let name = &section[after_fn_open..name_end];
            // 找 </function>
            let fn_close = match section[name_end..].find("</function>") {
                Some(p) => name_end + p,
                None => break,
            };
            let params_section = &section[name_end..fn_close];

            // 解析所有 <parameter=key>value</parameter>
            let mut args_map = serde_json::Map::new();
            let mut param_search = params_section;
            while let Some(param_start) = param_search.find("<parameter=") {
                let key_start = param_start + "<parameter=".len();
                let key_end = match param_search[key_start..].find('>') {
                    Some(p) => key_start + p,
                    None => break,
                };
                let param_key = &param_search[key_start..key_end];

                let value_start = key_end + 1;
                let close_tag = "</parameter>".to_string();
                let value_end = match param_search[value_start..].find(&close_tag) {
                    Some(p) => value_start + p,
                    None => break,
                };
                let param_value = &param_search[value_start..value_end];

                args_map.insert(
                    param_key.to_string(),
                    serde_json::Value::String(param_value.trim().to_string()),
                );

                param_search = &param_search[value_end + close_tag.len()..];
            }

            let args_json = serde_json::Value::Object(args_map);
            let arguments_str = serde_json::to_string(&args_json).unwrap_or_default();

            Some(axagent_harness::types::ToolCall {
                id: format!("xml-inline-{}", results.len()),
                call_type: "function".to_string(),
                function: axagent_harness::types::ToolCallFunction {
                    name: name.to_string(),
                    arguments: arguments_str,
                },
            })
        } else {
            None
        };

        if let Some(tc) = fn_name {
            results.push(tc);
        }
        remaining = &remaining[tc_end + "</tool_call>".len()..];
    }

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

/// 从 LLM 输出中尝试提取 JSON 片段（剥离 markdown 代码围栏）。
///
/// 处理 LLM 在 JSON 外包裹 markdown fence 的常见情况：
///   ```json
///   {...}
///   ```
/// 返回剥离后的文本，若找不到 fence 则返回 None。
fn try_extract_json_fragment(text: &str) -> Option<String> {
    let trimmed = text.trim();
    // 优先找 ```json 围栏
    if let Some(start) = trimmed.find("```json") {
        let inner = &trimmed[start + 7..];
        // 跳过内容前换行
        let inner = inner.trim_start();
        // 找关闭 fence
        if let Some(end) = inner.find("```") {
            let extracted = inner[..end].trim().to_string();
            if !extracted.is_empty() {
                return Some(strip_control_chars(&extracted));
            }
        }
        // 没有关闭 fence 也返回（可能截断了）
        let extracted = inner.trim().to_string();
        if !extracted.is_empty() {
            return Some(strip_control_chars(&extracted));
        }
    }
    // 回退：找 ``` 围栏（任意语言标签）
    if let Some(start) = trimmed.find("```") {
        let inner = &trimmed[start + 3..];
        // 跳过语言标签行和换行
        let inner = if let Some(newline) = inner.find('\n') {
            &inner[newline + 1..]
        } else {
            inner
        };
        let inner = inner.trim_start();
        if let Some(end) = inner.find("```") {
            let extracted = inner[..end].trim().to_string();
            if !extracted.is_empty() {
                return Some(strip_control_chars(&extracted));
            }
        }
        let extracted = inner.trim().to_string();
        if !extracted.is_empty() {
            return Some(strip_control_chars(&extracted));
        }
    }
    None
}

/// 轻量级 JSON 修复：处理 LLM 高频语法错误（尾逗号、nulll 等）。
/// 注：stock_workflow.rs 中有完整版 repair_json，此处是同步的简化版。
fn repair_json(s: &str) -> String {
    let mut result = s.to_string();
    // 常见 LLM 输出错误修复
    result = result.replace("nulll", "null");
    // LLM 可能在 JSON 值中输出 undefined (JS 关键字，非法 JSON)
    // 需要加双引号修复: `: undefined` → `: "undefined"`, `,undefined` → `,"undefined"`
    result = result.replace(": undefined", ": \"undefined\"");
    result = result.replace(",undefined", ",\"undefined\"");
    result = result.replace("[undefined", "[\"undefined\"");
    result = result.replace("(undefined", "(\"undefined\"");
    result = result.replace("=undefined", "=\"undefined\"");
    // 尾逗号: `,]` → `]`, `,}` → `}`
    result = result.replace(",]", "]");
    result = result.replace(",}", "}");
    // 双逗号: `,,` → `,`
    while result.contains(",,") {
        result = result.replace(",,", ",");
    }
    // 缺失逗号：数组/对象边界间缺失逗号（LLM 高频错误）
    // 处理 `]{`, `}{`, `}[` 等模式（可能含空白符）
    result = insert_comma_between_brackets(&result, ']', '{');
    result = insert_comma_between_brackets(&result, '}', '{');
    result = insert_comma_between_brackets(&result, '}', '[');
    // 缺失冒号：`"key" "value"` / `"key"  "value"` / `"key"(value`
    // 发生在 LLM 输出 JSON 时漏掉了 key-value 间的冒号
    result = insert_missing_colon(&result);
    // 双引号键修复: 连续两个引号 `""k` → `"k`
    result = result.replace("\"\"", "\"");
    // 字符串值中未转义的引号修复：LLM 在 JSON 字符串值中输出中文/英文引号时
    // 高频忘记转义（如 `"report": "text..."..."`），导致 json 解析提前截断。
    // 扫描已修复的 JSON，识别并转义字符串值中不应结尾的引号。
    result = repair_unescaped_quotes(&result);
    // 字符串值缺开引号: `"key"(` 或 `"key" "text` 但缺冒号的情况已由 insert_missing_colon 处理
    result
}

/// 修复 JSON 字符串值中未转义的引号。
/// LLM 高频在 report/description 等长文本字段中输出未转义的 `"` 字符，
/// 导致 JSON 解析器错误地提前结束字符串值。
///
/// 策略：逐字符扫描，追踪是否在字符串值内。在字符串值内遇到未转义的 `"` 时，
/// 检查其后是否可能是真正的字符串结尾——如果不是，则转义该引号。
///
/// V42 增强：逗号后的 `"key":` 模式识别。
/// 旧策略仅检查 `"` 后的下一个非空白字符是否为 `,` `}` `]` `:`，
/// 但遇到 `文本"X",文本`（即引号后跟逗号但非 JSON 键分隔）会误判为字符串结尾。
/// 新策略：当 `"` 后跟 `,` 时，额外检查逗号后是否为 `"key":` 的 JSON 键模式。
/// 只有当逗号后紧跟 `"字段名":` 模式（字段名非空且后跟冒号）时，才判定为真正的字符串结尾。
fn repair_unescaped_quotes(s: &str) -> String {
    fn is_json_ws(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | b'\r')
    }

    /// 从位置 pos 开始扫描，检查是否符合 `"key":` 的 JSON 键模式。
    /// 返回 true 当扫描到 `"` 后跟（跳过空白）`:` 时。
    fn is_json_key_pattern(bytes: &[u8], mut pos: usize) -> bool {
        // 跳过起始的空白
        while pos < bytes.len() && is_json_ws(bytes[pos]) {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b'"' {
            return false;
        }
        pos += 1; // 跳过开头的 "
        // 扫描到下一个 " 或结尾
        while pos < bytes.len() && bytes[pos] != b'"' {
            pos += 1;
        }
        if pos >= bytes.len() {
            return false;
        }
        pos += 1; // 跳过结尾的 "
        // 跳过空白，检查 :
        while pos < bytes.len() && is_json_ws(bytes[pos]) {
            pos += 1;
        }
        pos < bytes.len() && bytes[pos] == b':'
    }

    /// 检查位置 i 的 `"` 是否为真正的字符串结尾。
    /// 在字符串值内，当 `"` 后跟（跳过空白）`}` `]` 时 → 绝对结尾。
    /// 当 `"` 后跟（跳过空白）`,` 时 → 潜在结尾，需进一步检查：
    ///   如果逗号后是 `"key":` 的 JSON 键模式 → 真实结尾
    ///   否则 → 只是字符串内容中的引号
    /// 当 `"` 后跟（跳过空白）`:` 时 → JSON 键的结尾（退出字符串值状态）
    fn is_string_end(bytes: &[u8], quote_pos: usize) -> (bool, bool) {
        // (is_end, is_key_end)
        let mut j = quote_pos + 1;
        while j < bytes.len() && is_json_ws(bytes[j]) {
            j += 1;
        }
        if j >= bytes.len() {
            return (true, false);
        }
        match bytes[j] {
            b'}' | b']' => (true, false), // 绝对结尾
            b':' => (true, true),         // JSON 键结尾
            b',' if is_json_key_pattern(bytes, j + 1) => (true, false),
            b',' => (false, false),
            _ => (false, false),
        }
    }

    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len() + 64);
    let mut i = 0;

    // 状态：是否在字符串值内（即 after `"key": "` 内，不包括 key 本身）
    let mut in_string_value = false;
    // 前一个非空白字符（用于判断 `:` → 开始字符串值）
    let mut last_non_ws: Option<u8> = None;

    while i < bytes.len() {
        let b = bytes[i];

        if b == b'"' {
            if !in_string_value {
                // 检查这个引号是否开始一个字符串值
                // 条件：前一个非空白字符是 `:` 或 `[` 或 `,` 或 `{`
                if let Some(prev) = last_non_ws
                    && (prev == b':' || prev == b'[' || prev == b',' || prev == b'{')
                {
                    in_string_value = true;
                    result.push(b'"');
                    i += 1;
                    continue;
                }
                result.push(b'"');
                i += 1;
            } else {
                // 在字符串值内：检查这个引号是否是真正的字符串结尾
                let is_escaped = i > 0 && bytes[i - 1] == b'\\';

                if is_escaped {
                    // 已转义，保持原样
                    result.push(b'"');
                    i += 1;
                    continue;
                }

                let (is_end, _is_key_end) = is_string_end(bytes, i);

                if is_end {
                    // 真正的字符串结尾
                    result.push(b'"');
                    in_string_value = false;
                    // 如果结尾类型是 key 结尾（`:` 后只可能是 value 的开头），
                    // 但当前状态已在字符串值外，不需要额外处理
                    i += 1;
                } else {
                    // 字符串值中间的未转义引号 → 转义
                    result.push(b'\\');
                    result.push(b'"');
                    i += 1;
                }
            }
        } else {
            // 非引号字符
            if b == b'\\' && in_string_value && i + 1 < bytes.len() {
                // 转义序列：跳过下一个字符（如 \n, \", \\）
                result.push(b'\\');
                result.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if !is_json_ws(b) {
                last_non_ws = Some(b);
            }
            result.push(b);
            i += 1;
        }
    }
    String::from_utf8(result).unwrap_or_else(|_| s.to_string())
}

/// 在两个字符之间插入逗号（处理中间有空白符的情况）
/// 例如 `]    {` → `],    {`
fn insert_comma_between_brackets(s: &str, left: char, right: char) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len() + 16);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == left as u8 {
            // 检查后面是否有空白 + right
            let mut j = i + 1;
            while j < bytes.len()
                && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r')
            {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == right as u8 {
                // 在 left 和 right 之间插入逗号（如果还没有逗号）
                // 检查 left 和 right 之间是否已有逗号
                let has_comma = (i + 1..j).any(|k| bytes[k] == b',');
                if !has_comma {
                    result.push(left as u8);
                    // 保留中间空白
                    result.extend_from_slice(&bytes[i + 1..j]);
                    result.push(b',');
                    i = j;
                    continue;
                }
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).unwrap_or_else(|_| s.to_string())
}

/// 修复缺失冒号：`"key"` 后直接跟 `"` 或 `(`（中间可能有空白）但缺 `:`
/// 匹配 `"key" "value"` → `"key": "value"`、`"key"(value` → `"key":(value`  
fn insert_missing_colon(s: &str) -> String {
    let mut result = s.to_string();
    // `"` + 可选空白 + `"` → 中间插 `:`（仅在 `"` 闭合 key 的情况下）
    // 使用正则风格替换：找到 `"` 后跟空白再跟 `"` 的模式
    // 用简单循环 + find 来实现
    loop {
        let before = result.clone();
        // 查找 `"key" "` 模式：`"` 后跟非 `"` 非空白字符，然后 `"`，然后空白，然后 `"` 且中间没有 `:`
        let bytes = result.as_bytes();
        let mut found = false;
        let mut insert_pos = 0;
        let mut i = 0;
        while i + 2 < bytes.len() {
            if bytes[i] == b'"' {
                // 找这个 `"` 的配对（下一个未被转义的 `"`）
                let mut j = i + 1;
                let mut escaped = false;
                while j < bytes.len() {
                    if escaped {
                        escaped = false;
                        j += 1;
                        continue;
                    }
                    if bytes[j] == b'\\' {
                        escaped = true;
                        j += 1;
                        continue;
                    }
                    if bytes[j] == b'"' {
                        break;
                    }
                    j += 1;
                }
                if j >= bytes.len() {
                    break;
                } // 未闭合的字符串，跳出
                // 检查 `"` 闭合后有没有 `:`（跳过空白）
                let mut k = j + 1;
                while k < bytes.len()
                    && (bytes[k] == b' ' || bytes[k] == b'\t' || bytes[k] == b'\n')
                {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b':' {
                    i = k + 1;
                    continue;
                } // 已有冒号
                if k < bytes.len()
                    && (bytes[k] == b'"'
                        || bytes[k] == b'('
                        || bytes[k] == b'{'
                        || bytes[k] == b'[')
                {
                    // 缺冒号！在闭合 `"` 后、空白前插入 `:`
                    insert_pos = j + 1; // 在闭合 `"` 的后面
                    found = true;
                    break;
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }
        if found {
            result.insert(insert_pos, ':');
        }
        if result == before {
            break;
        }
    }
    result
}

/// 扁平 JSON 模式下不再需要复杂的未闭合字符串修复。
/// flat JSON schema（最多 5 字段、无嵌套数组）的 LLM 输出极少出现此类错误。
/// 保留此函数仅为兼容旧模板过渡期；后续可删除。
fn repair_unclosed_json_strings() -> Option<String> {
    None
}

/// 修复扁平 JSON 被截断（flat schema 下只需补全缺失的 `}` / `]` / `"`）
fn try_fix_truncated_json(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || (!s.starts_with('{') && !s.starts_with('[')) {
        return None;
    }

    // flat JSON: 只有 1-2 层嵌套，简单计数即可修复
    let mut result = s.to_string();
    let mut added = false;

    // 补全未闭合引号
    let open_quotes = result.matches('"').count();
    if open_quotes & 1 == 1 {
        result.push('"');
        added = true;
    }

    // 补全缺失的闭括号
    let open_curly = result.chars().filter(|&c| c == '{').count();
    let close_curly = result.chars().filter(|&c| c == '}').count();
    for _ in 0..open_curly.saturating_sub(close_curly) {
        result.push('}');
        added = true;
    }

    let open_sq = result.chars().filter(|&c| c == '[').count();
    let close_sq = result.chars().filter(|&c| c == ']').count();
    for _ in 0..open_sq.saturating_sub(close_sq) {
        result.push(']');
        added = true;
    }

    if !added {
        return None;
    }

    let repaired = repair_json(&result);
    if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
        return Some(repaired);
    }
    if repaired != result && serde_json::from_str::<serde_json::Value>(&result).is_ok() {
        return Some(result);
    }
    None
}

/// 检测 LLM 输出是否为纯文本拒绝（模型安全机制触发）。
/// 匹配"抱歉/无法回答/不能回答/拒绝回答"等常见拒绝模式。
/// 若发现此类模式，后续逻辑会将输出转换为结构化错误而不是让 strict_mode 校验失败。
fn is_refusal_plain_text(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }

    // 先检查是否为合法 JSON——合法 JSON 肯定不是纯文本拒绝
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return false;
    }

    // 包含 VERDICT tag 的一定不是拒绝（已有分析内容）
    if trimmed.contains("<!-- VERDICT:") {
        return false;
    }

    // 较长输出（>200字符）说明有实质性分析内容，不是拒绝
    if trimmed.chars().count() > 200 {
        return false;
    }

    let lower = trimmed.to_lowercase();
    let refusal_prefixes = [
        "抱歉我无法回答这个问题",
        "抱歉我不能回答",
        "抱歉无法回答",
        "sorry, i cannot",
        "sorry, i can't",
        "sorry cannot",
        "i cannot answer",
        "i can't answer",
        "i am unable to answer",
        "i'm unable to answer",
        "not able to answer",
        "unable to answer",
    ];

    // 检查是否是纯拒绝（以拒绝前缀开头，且后面无实质内容）
    for prefix in &refusal_prefixes {
        if lower.starts_with(prefix) {
            let after = &trimmed[prefix.len()..].trim();
            // 如果后面无实质内容（仅标点符号/空格），是真拒绝
            if after.is_empty()
                || after.chars().all(|c| {
                    c.is_ascii_punctuation() || c.is_whitespace() || c == '。' || c == '，'
                })
            {
                return true;
            }
            // 后面有实质内容（如"行业分析师数据不足"）→ 是数据不足说明，不是拒绝
            return false;
        }
    }

    // 额外检查：极端短句拒绝（纯"抱歉。" "无法回答。" 无分析内容）
    let refusal_short = ["抱歉。", "无法回答。", "不能回答。", "拒绝回答。", "sorry."];
    for pattern in &refusal_short {
        if trimmed == *pattern {
            return true;
        }
    }

    false
}

/// 从 LLM 输出中提取 `<!-- VERDICT: {...} -->` 标签中的 JSON 内容。
/// 返回 verdict JSON 的字符串表示，不含外层 HTML 注释标记。
///
/// 这是 TradingAgents 模式的 Rust 实现：
/// 分析师输出自然语言报告，末尾追加 <!-- VERDICT: {...} --> 供机读。
/// 从 LLM 输出中提取 ```tool_json（或 ```json tool_json）代码块内的 JSON 对象。
///
/// 工作流 agent 节点（trend-scanner / chain-decomposer / candidate-mapper 等）约定输出
/// tool_json 代码块（Serenity 协议），但 rt-workflow 的 agent_executor 不经过 IR Normalizer，
/// 块内容会原样保留为文本，导致下游 resolve_var_path 无法下钻。此函数提取块内 JSON。
///
/// 兼容 GLM 两种 fence 写法：```tool_json 与 ```json tool_json。
fn extract_tool_json_block(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let mut search_from = 0;
    while let Some(rel) = lower[search_from..].find("```") {
        let abs = search_from + rel;
        let line_end = text[abs..].find('\n').map(|o| abs + o).unwrap_or(text.len());
        let fence_line = text[abs..line_end].trim();
        // 围栏匹配：```tool_json / ```json tool_json（Serenity 协议），以及裸 ```json
        // （GLM 偶发用普通 json 围栏包裹 submit_* 工具调用）。裸 ```json 提取出的 JSON
        // 若无 arguments 字段，上游 unwrap_or(parsed) 保持 JSON 本身，等价于规范化输出，无害。
        let is_tool_fence = fence_line.contains("tool_json");
        let is_plain_json_fence = fence_line.trim_end().eq_ignore_ascii_case("```json")
            || fence_line.trim_end().eq_ignore_ascii_case("```JSON");
        if is_tool_fence || is_plain_json_fence {
            let rest = &text[line_end..];
            // 限定在闭合围栏（下一个 ```）之前提取，防止 rfind('}') 越过围栏
            // 取到围栏后 VERDICT 标签的 }（场景：tool_json 块在前、<!-- VERDICT --> 在后，
            // 会导致 candidate 混入标签文本解析失败）。
            let body_end = rest.find("```").unwrap_or(rest.len());
            let body = &rest[..body_end];
            if let Some(start) = body.find('{') {
                let candidate = &body[start..];
                // 从第一个 { 截到闭合围栏前最后一个 }
                if let Some(end) = candidate.rfind('}') {
                    let candidate = &candidate[..=end];
                    if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                        return Some(candidate.to_string());
                    }
                }
            }
        }
        search_from = abs + 3;
    }
    // 裸形式（无围栏）：某行（trim 后）以 `tool_json` 开头且后续内容为 JSON。
    // GLM 偶发输出不带 ``` 围栏的 `tool_json\n{...}` 形式（如 a-trend-scanner）。
    let mut line_search = 0;
    while let Some(rel) = lower[line_search..].find("tool_json") {
        let abs = line_search + rel;
        let line_end = text[abs..].find('\n').map(|o| abs + o).unwrap_or(text.len());
        let line = text[abs..line_end].trim();
        // 行首为 tool_json（或 ``` 围栏内残留的 tool_json 语言标签）
        let line_start_is_clean = line == "tool_json"
            || line == "json tool_json"
            || line.starts_with("tool_json")
            || line.starts_with("json tool_json");
        // 前面必须是行首/空白/围栏闭合，避免误匹配正文中的"tool_json"字样
        let before = &text[..abs];
        let at_line_start = before.is_empty()
            || before.ends_with('\n')
            || before.trim_end().ends_with("```")
            || before.trim_end().is_empty();
        if line_start_is_clean && at_line_start {
            let rest = &text[line_end..];
            if let Some(start) = rest.find('{') {
                let body = &rest[start..];
                if let Some(end) = body.rfind('}') {
                    let candidate = &body[..=end];
                    if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                        return Some(candidate.to_string());
                    }
                }
            }
        }
        line_search = abs + "tool_json".len();
    }
    None
}

fn extract_verdict_tag(text: &str) -> Option<String> {
    // 查找最后一个 <!-- VERDICT: 出现位置（取最后一个，因为正文中可能也有 HTML 注释）
    // 安全做法：直接在全文本上 rfind，不手动做字节切片
    let start_marker = "<!-- VERDICT: ";
    let end_marker = "-->";
    if let Some(start) = text.rfind(start_marker) {
        let json_start = start + start_marker.len();
        if let Some(end_offset) = text[json_start..].find(end_marker) {
            let verdict_str = &text[json_start..json_start + end_offset];
            let trimmed = verdict_str.trim();
            if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// 从 LLM 输出中剥离 `<!-- VERDICT: ... -->` 标签，返回纯文本报告内容
fn strip_verdict_tag(text: &str) -> String {
    let start_marker = "<!-- VERDICT: ";
    let end_marker = "-->";
    let mut result = text.to_string();
    loop {
        if let Some(start) = result.find(start_marker)
            && let Some(end) = result[start..].find(end_marker)
        {
            result.replace_range(start..start + end + end_marker.len(), "");
            continue;
        }
        break;
    }
    result.trim().to_string()
}

/// 从尾部找到最后一个完整闭合的 JSON 对象/数组，截掉后面的垃圾文本。
/// LLM 经常在 JSON 后面追加自然语言评论，导致解析失败。
fn trim_after_json(s: &str) -> &str {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len == 0 {
        return s;
    }

    // 正向扫描: 记录每个 depth=0 的位置（即每个顶层闭合点）
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut last_complete_end = 0;

    for (i, &b) in bytes.iter().enumerate().take(len) {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' && in_string {
            escaped = true;
            continue;
        }
        if b == b'"' {
            in_string = !in_string;
            continue;
        }
        if !in_string {
            match b {
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth -= 1;
                    if depth == 0 {
                        last_complete_end = i;
                    }
                },
                _ => {},
            }
        }
    }

    if last_complete_end > 0 {
        &s[..=last_complete_end]
    } else {
        s
    }
}

/// 剥离 JSON 字符串中的原始控制字符（\u{0000}-\u{001F}）
///
/// LLM 常在字符串值中直接输出原始控制字符（如未转义的换行、制表符等），
/// 导致 serde_json 解析失败（control character found while parsing a string）。
///
/// 处理策略：将原始控制字符替换为空格（保留词间分隔），
/// 已正确转义的控制字符（如 `\n` 的两个字符 '\\' + 'n'）不受影响。
fn strip_control_chars(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('\u{0000}'..='\u{001F}').contains(&c) {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// 从字符串中提取第一个 JSON 对象/数组，跳过所有前缀非 JSON 内容。
/// 覆盖 LLM 在 JSON 前加 ```json / 文本说明等所有前缀场景。
fn try_extract_first_json(s: &str) -> Option<String> {
    let s = s.trim();
    let start = s.find(['{', '['])?;
    let candidate: String = s[start..].to_string();
    // 先剥离控制字符再尝试解析
    let cleaned = strip_control_chars(&candidate);
    if serde_json::from_str::<serde_json::Value>(&cleaned).is_ok() {
        return Some(cleaned);
    }
    try_fix_truncated_json(&cleaned)
}

/// 从字符串中提取最长的合法 JSON 前缀。
/// 先查第一个 `{` 或 `[`，然后逐字符扫描用栈追踪括号平衡，
/// 在栈为空时截断。
/// 作为所有其他修复失败后的最终兜底。
fn try_extract_balanced_json(s: &str) -> Option<String> {
    let s = trim_after_json(s);
    let start = s.find(['{', '['])?;
    let candidate = &s[start..];

    let bytes = candidate.as_bytes();
    let mut stack: Vec<u8> = Vec::new(); // 用栈追踪括号嵌套顺序
    let mut in_string = false;
    let mut escaped = false;
    let mut end_pos = 0;

    for (i, &b) in bytes.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' && in_string {
            escaped = true;
            continue;
        }
        if b == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match b {
            b'{' | b'[' => stack.push(b),
            b'}' => {
                if stack.last() == Some(&b'{') {
                    stack.pop();
                } else {
                    // } 不匹配栈顶 → 优先尝试修复
                    break;
                }
            },
            b']' => {
                if stack.last() == Some(&b'[') {
                    stack.pop();
                } else {
                    break;
                }
            },
            _ => {},
        }
        if stack.is_empty() && i > 0 {
            end_pos = i + 1;
            break;
        }
    }

    if end_pos == 0 && !stack.is_empty() {
        end_pos = bytes.len();
    }

    if end_pos == 0 {
        return None;
    }

    let extracted = candidate[..end_pos].trim().to_string();
    if extracted.is_empty() {
        return None;
    }

    // 先 repair 再解析
    let repaired = repair_json(&extracted);
    if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
        return Some(repaired);
    }
    try_fix_truncated_json(&repaired)
}

/// strict_mode 下的输出格式校验：
/// - 当 output_mode 为 Json 时，验证 final_content 是否为合法 JSON
/// - 当 output_mode 为 Text 时，验证 final_content 是否为非空（防止 LLM 空输出静默通过）
/// - 若格式不合法，返回错误阻止结果传递给下游
/// - 自动处理 LLM 常见坏输出模式：markdown fence 包裹、尾逗号等
fn validate_strict_mode_output(
    final_content: &str,
    output_mode: &axagent_harness::workflow_types::OutputMode,
) -> Result<(), NodeError> {
    use axagent_harness::workflow_types::OutputMode;
    let trimmed = final_content.trim();
    // V53 修复: 在进入任何修复链之前统一剥离原始控制字符（\u{0000}-\u{001F}）。
    // LLM 常在 JSON 字符串值中直接输出原始换行/制表符等，导致 serde_json 解析失败。
    // strip_control_chars 将其替换为空格，不影响 JSON 结构。
    let cleaned = strip_control_chars(trimmed);
    let trimmed: &str = &cleaned;

    // 所有模式通用：空输出检测
    if trimmed.is_empty() {
        tracing::warn!("strict_mode: LLM 输出为空 (output_mode={:?})", output_mode);
        return Err(NodeError::exec_failed(
            error_code::VALIDATION_FAILED,
            format!("严格模式: LLM 输出为空 (output_mode={:?})", output_mode),
        ));
    }

    if matches!(output_mode, OutputMode::Json) {
        // 常见坏输出模式修复链
        let mut candidates: Vec<String> = Vec::new();
        candidates.push(trimmed.to_string());

        // 模式0: 跳过所有非 JSON 前缀，从第一个 { 或 [ 开始尝试
        if let Some(extracted) = try_extract_first_json(trimmed)
            && extracted != trimmed
        {
            candidates.push(extracted);
        }

        // 模式1: 剥离原始控制字符（LLM 常在 JSON 字符串值中输出 \u0000-\u001F）
        let stripped_control = strip_control_chars(trimmed);
        if stripped_control != trimmed {
            candidates.push(stripped_control.clone());
            // V42 修复: 控制字符剥离后再做一次 repair_json，覆盖"先有换行后有未转义引号"的场景。
            // 原始文本可能同时包含未转义换行(被 strip_control_chars 修复)和未转义引号，
            // 两者各自独立产生的候选都无法覆盖交集情况。
            let repaired_after_strip = repair_json(&stripped_control);
            if repaired_after_strip != stripped_control {
                candidates.push(repaired_after_strip);
            }
        }

        // 模式1: markdown fence 包裹 (```json \n {...} \n ```)
        if let Some(stripped) = try_extract_json_fragment(trimmed) {
            candidates.push(stripped);
        }

        // 模式2: repair_json 修复尾逗号、nulll 等
        let repaired = repair_json(trimmed);
        if repaired != trimmed {
            candidates.push(repaired.clone());
            // fence 内也尝试 repair
            if let Some(stripped) = try_extract_json_fragment(&repaired) {
                candidates.push(stripped);
            }
        }

        // 模式3: 修复未闭合的引用字符串（"[a-news 202] 缺闭合 "）
        if let Some(unclosed_fixed) = repair_unclosed_json_strings()
            && unclosed_fixed != trimmed
        {
            candidates.push(unclosed_fixed);
        }

        // 模式3b: 括号缺失/截断修复（补充缺失的 ]/}，处理"数组未关继续写父级字段"模式）
        if let Some(trunc_fixed) = try_fix_truncated_json(trimmed)
            && !candidates.iter().any(|x| x.as_str() == trunc_fixed.as_str())
        {
            candidates.push(trunc_fixed);
        }

        // 模式4: 深度追踪截断——去掉 JSON 后追加的垃圾文本
        // 例：LLM 在 JSON 后写自然语言注释导致解析失败
        {
            let truncated = trim_after_json(trimmed);
            if truncated.len() < trimmed.len() {
                candidates.push(truncated.to_string());
                // 截断后再次尝试 fence 剥离（可能原 fence 因垃圾文本被跳过）
                if let Some(stripped) = try_extract_json_fragment(truncated) {
                    candidates.push(stripped);
                }
            }
        }

        // 模式5: 终极兜底——括号平衡提取
        if let Some(balanced) = try_extract_balanced_json(trimmed)
            && !candidates.iter().any(|x| x.as_str() == balanced.as_str())
        {
            candidates.push(balanced);
        }

        // ── 遍历修复链：对所有已有候选进行二次修复 ──
        // 之前的修复（fence 剥离/repair_json/截断/闭合）只针对原始 trimmed，
        // 但 fence 剥离后的候选可能也需要同样的修复（如截断修复、未闭合字符串等）。
        let secondary_fixes: Vec<String> = candidates
            .iter()
            .flat_map(|c| {
                let mut fixes: Vec<String> = Vec::new();
                // 如果还没被剥离 fence，再尝试一次
                if let Some(stripped) = try_extract_json_fragment(c)
                    && !candidates.iter().any(|x| x.as_str() == stripped.as_str())
                {
                    fixes.push(stripped);
                }
                // 未闭合字符串修复
                if let Some(fixed) = repair_unclosed_json_strings()
                    && fixed != *c
                    && !candidates.iter().any(|x| x.as_str() == fixed.as_str())
                {
                    fixes.push(fixed);
                }
                // repair_json（尾逗号等）
                let repaired = repair_json(c);
                if repaired != *c && !candidates.iter().any(|x| x.as_str() == repaired.as_str()) {
                    fixes.push(repaired);
                }
                // 截断 JSON 修复
                if let Some(trunc_fixed) = try_fix_truncated_json(c)
                    && !candidates.iter().any(|x| x.as_str() == trunc_fixed.as_str())
                {
                    fixes.push(trunc_fixed);
                }
                // 控制字符剥离（修复 fence 提取后仍有控制字符的问题）
                let cleaned = strip_control_chars(c);
                if cleaned != *c && !candidates.iter().any(|x| x.as_str() == cleaned.as_str()) {
                    fixes.push(cleaned);
                }
                fixes
            })
            .collect();
        candidates.extend(secondary_fixes);

        // 逐个候选尝试解析
        for candidate in &candidates {
            if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                if candidate != trimmed {
                    tracing::warn!(
                        "strict_mode: 自动修复 LLM 输出格式 (原始={} => 修复后={})",
                        trimmed.chars().take(80).collect::<String>(),
                        candidate.chars().take(80).collect::<String>(),
                    );
                }
                return Ok(());
            }
        }

        // 所有修复尝试均失败 → 报告具体失败原因（列出前 3 个候选的错误）
        for (i, c) in candidates.iter().take(3).enumerate() {
            // 注意：此处用于日志记录错误信息，空字符串作为默认值是安全的
            let err = serde_json::from_str::<serde_json::Value>(c)
                .err()
                .map(|e| e.to_string())
                .unwrap_or_default();
            tracing::warn!(
                "strict_mode: 候选[{}] 解析失败: {} [前100字符: {}]",
                i,
                err,
                c.chars().take(100).collect::<String>()
            );
        }
        // 注意：此处用于日志记录错误信息，空字符串作为默认值是安全的
        let serde_err = serde_json::from_str::<serde_json::Value>(trimmed)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        let preview: String = trimmed.chars().take(200).collect();
        let full: String = trimmed.chars().take(3000).collect();
        tracing::error!("strict_mode: LLM 输出不是合法 JSON: {serde_err} [前200字符: {preview}]");
        tracing::error!("strict_mode: 完整输出(前3000字符): {full}");
        return Err(NodeError::exec_failed(
            error_code::VALIDATION_FAILED,
            format!("严格模式: LLM 输出不是合法 JSON（错误: {serde_err}, 前200字符: {preview}）"),
        ));
    }
    Ok(())
}
