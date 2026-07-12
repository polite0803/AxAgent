// SPDX-License-Identifier: AGPL-3.0-only

//! 统一工作流引擎 —— DAG 管理 + 并发执行 + DB 持久化。
//!
//! 节点类型统一为 axagent_harness::workflow_types::WorkflowNode（28 种），
//! 执行通过 NodeDispatcher 分发到对应执行器。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use axagent_harness::workflow_types::{
    BackoffType, CompensationStrategy, DegradeStrategy, EdgeType, JsonSchema, OnFailureAction,
    RetryConfig, Variable, WorkflowEdge, WorkflowNode,
};

use axagent_harness::RhaiEngineAdapter;
use axagent_harness::repo_dtos::WorkflowExecutionData;
use axagent_harness::repositories::{
    loop_checkpoint_repository, workflow_execution_repository, workflow_template_repository,
};
use rhai::{EvalAltResult, Position};

pub mod dag_store;
pub mod node_state;
pub mod output_builder;
pub mod rhai_runtime;
use output_builder::{
    build_workflow_output, collect_workflow_tool_names, extract_end_output, validate_input,
};
use rhai_runtime::{LocalRhaiToolFn, RhaiScriptCache, rhai_map_to_json};

use dag_store::skip_disabled_branch_nodes;
use node_state::{NodeCircuitBreaker, NodeResult, compute_backoff};

use crate::workflow_engine::{
    NodeRuntimeState, NodeStatus, Workflow, WorkflowError, WorkflowStatus, current_epoch_ms,
    current_timestamp,
};

use super::dispatcher::NodeDispatcher;
use super::error_handling::ErrorContext;
use super::execution_state::{
    ExecutionContextCallbacks, ExecutionState, ExecutionStatus, NodeExecutionRecord,
};
use super::executors::{
    AgentExecutor, ConditionExecutor, LlmClassifierExecutor, LlmExecutor, PlanCallbacks,
    ProfileCache, ProviderCache, RagCallback, SubWorkflowCallback, ToolCallback,
};
use super::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput, node_type_name};
use super::prompt_template::{CompiledPrompt, DomainConstraintsFn, compile_prompt};

/// 工具解析器：给定工具名，返回对应的 ToolCallback（若可解析）。
/// 用于 run_workflow 启动时自动扫描工作流节点并注册工具。
pub type ToolResolver = Arc<
    dyn Fn(
            String,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<ToolCallback>> + Send>>
        + Send
        + Sync,
>;

/// 工作流运行选项
#[derive(Clone)]
pub struct RunOptions {
    pub max_concurrent: usize,
    /// 按节点类型细粒度并发限制：工具/文件操作为 10，LLM 调用为 3；None 则统一用 max_concurrent
    pub max_concurrent_by_type: Option<HashMap<String, usize>>,
    pub step_timeout: Duration,
    /// 调用方指定的模型 ID（来自会话/用户设置），执行器优先使用
    pub model_id: Option<String>,
    /// 调用方指定的 provider ID（来自会话/用户设置），执行器优先使用
    pub provider_id: Option<String>,
    /// 步骤进度回调（用于向前端推送实时进度事件）
    pub progress_callback: Option<ProgressCallback>,
    /// 工作流输入参数（替代默认的 `{}`，会经过 input_schema 校验）
    pub input: Option<serde_json::Value>,
    /// 输入 JSON Schema（非空时对 input 做校验）
    pub input_schema: Option<JsonSchema>,
    /// 输出 JSON Schema（非空时对 results 做过滤，写入 Workflow.output）
    pub output_schema: Option<JsonSchema>,
    /// 模板级变量列表（来自 WorkflowTemplateData.variables），写入执行上下文
    pub variables: Option<Vec<Variable>>,
    /// 干跑模式：不实际调用 LLM/Tool，用 mock 输出验证流程
    pub dry_run: bool,
    /// Plan 模式回调：审批 + 步骤进度事件（通过 ExecutionState 传递给 AgentExecutor）
    pub plan_callbacks: Option<PlanCallbacks>,
    pub parent_execution_id: Option<String>,
    pub execution_id: Option<String>,
    pub parent_cancel_token: Option<CancellationToken>,
}

/// 步骤进度事件
#[derive(Debug, Clone)]
pub struct StepProgressEvent {
    pub node_id: String,
    pub status: String,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub execution_id: Option<String>,
}

/// 步骤进度回调：`&self` 不可用时使用独立函数签名
pub type ProgressCallback = Arc<
    dyn Fn(StepProgressEvent) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

impl std::fmt::Debug for RunOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOptions")
            .field("max_concurrent", &self.max_concurrent)
            .field("max_concurrent_by_type", &self.max_concurrent_by_type)
            .field("step_timeout", &self.step_timeout)
            .field("model_id", &self.model_id)
            .field("provider_id", &self.provider_id)
            .field("progress_callback", &self.progress_callback.is_some())
            .field("input", &self.input)
            .field("input_schema", &self.input_schema.is_some())
            .field("output_schema", &self.output_schema.is_some())
            .field("variables", &self.variables.as_ref().map(|v| v.len()))
            .field("plan_callbacks", &self.plan_callbacks.is_some())
            .finish()
    }
}

impl Default for RunOptions {
    fn default() -> Self {
        let mut by_type = std::collections::HashMap::new();
        by_type.insert("tool".to_string(), 10);
        by_type.insert("file".to_string(), 10);
        by_type.insert("llm".to_string(), 3);
        by_type.insert("agent".to_string(), 3);

        Self {
            max_concurrent: 3,
            max_concurrent_by_type: Some(by_type),
            step_timeout: Duration::from_secs(300),
            model_id: None,
            provider_id: None,
            progress_callback: None,
            input: None,
            input_schema: None,
            output_schema: None,
            variables: None,
            dry_run: false,
            plan_callbacks: None,
            parent_execution_id: None,
            execution_id: None,
            parent_cancel_token: None,
        }
    }
}

impl RunOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }
    pub fn with_step_timeout(mut self, timeout: Duration) -> Self {
        self.step_timeout = timeout;
        self
    }
    pub fn with_model(mut self, model_id: String) -> Self {
        self.model_id = Some(model_id);
        self
    }
    pub fn with_provider(mut self, provider_id: String) -> Self {
        self.provider_id = Some(provider_id);
        self
    }
    pub fn with_progress_callback(mut self, cb: ProgressCallback) -> Self {
        self.progress_callback = Some(cb);
        self
    }
    /// 注入模板级变量列表，运行时写入 ExecutionState.variables
    pub fn with_variables(mut self, variables: Vec<Variable>) -> Self {
        self.variables = Some(variables);
        self
    }
}

// ── WorkEngine ──

#[derive(Clone)]

pub struct WorkEngine {
    executions: Arc<Mutex<HashMap<String, ExecutionState>>>,
    workflows: Arc<tokio::sync::RwLock<HashMap<String, Workflow>>>,
    /// 编译后的 prompt 模板：workflow_id -> (node_id -> CompiledPrompt)
    compiled_prompts: Arc<tokio::sync::RwLock<HashMap<String, HashMap<String, CompiledPrompt>>>>,
    /// 编译后的 Rhai 脚本：workflow_id -> (tool_name -> AST)
    compiled_rhai_scripts: Arc<tokio::sync::RwLock<HashMap<String, RhaiScriptCache>>>,
    /// Rhai 引擎适配器（可选注入，优先使用；未设置时降级为 compiled_rhai_scripts）
    rhai_engine: Option<Arc<dyn RhaiEngineAdapter>>,
    /// Plan 模式：PlannerAdapter（由外部注入，None = 未启用 Plan 模式）
    planner: Option<Arc<std::sync::Mutex<dyn axagent_harness::PlannerAdapter>>>,
    cancel_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    dispatcher: Arc<tokio::sync::RwLock<NodeDispatcher>>,
    /// 按工具名注册的 handler 映射（多路注册，优先级最高）
    tool_handlers: Arc<Mutex<HashMap<String, ToolCallback>>>,
    /// 旧版全局回调（fallback，tool_handlers 未命中时使用）
    tool_fallback: Arc<Mutex<Option<ToolCallback>>>,
    /// 工具解析器（按需延迟注册，从全局 tool registry 查找工具）
    tool_resolver: Arc<Mutex<Option<ToolResolver>>>,
    rag_callback: Arc<Mutex<Option<RagCallback>>>,
    /// 领域约束注入回调（可选，None 时不注入任何约束，行为与现状一致）。
    ///
    /// 由主 binary（如 stock-analysis）在 setup 时通过 `set_domain_constraints`
    /// 注册。回调签名：`(role_name: &str) -> ConstraintBlocks`。
    ///
    /// 字段类型与 `set_rag_callback` 行为完全对齐：
    /// - `Arc<Mutex<Option<...>>>`：支持热更新与多 owner（dispatcher / 共享 Arc）
    /// - 使用 `std::sync::Mutex`：回调在 setup 阶段同步注册，调用频率极低，
    ///   无需 async 锁；后续 `domain_constraints()` getter 在 agent 节点执行
    ///   时取 Arc clone，持有时间极短。
    domain_constraints: Arc<std::sync::Mutex<Option<DomainConstraintsFn>>>,
    /// 业务规则引擎（可选，None = 不执行任何业务规则检查）。
    /// 硬约束，在执行层直接拦截违规操作。
    /// 通过 `set_business_rule_engine` 注入。
    business_rule_engine:
        Arc<std::sync::Mutex<Option<Arc<crate::business_rules::BusinessRuleEngine>>>>,
    /// Agent executor 共享缓存（跨节点复用，每次 run_workflow 开始时清空）
    agent_provider_cache: Arc<tokio::sync::Mutex<ProviderCache>>,
    agent_profile_cache: Arc<tokio::sync::Mutex<ProfileCache>>,
    /// 断点集（节点 ID → 是否启用，外部通过 set_breakpoints / resume 控制）
    pub breakpoints: Arc<Mutex<HashSet<String>>>,
    /// 节点断路器状态（跨 workflow 运行持久化，防止重试风暴）
    node_breakers: Arc<Mutex<HashMap<String, NodeCircuitBreaker>>>,
    /// 稳定持有的 AgentExecutor 引用（dispatcher 中的注册和这里共享同一个 Arc）。
    /// 所有 agent executor 的可变状态（engine、rag_callback）都通过这个 Arc
    /// 的 setter 方法共享更新；禁止再在外部通过 dispatcher.register 重新注册
    /// agent executor，否则会丢失之前注入的 provider_registry。
    /// master_key / provider_registry 由 AgentExecutor / LlmExecutor / ConditionExecutor /
    /// LlmClassifierExecutor 各自持有，WorkEngine 不再冗余存储。
    agent_executor: Arc<AgentExecutor>,
    /// P0-2: dispatcher 的 register 改 async 后，WorkEngine::new 处于同步上下文，
    /// 无法直接 await register。这里把内置 executor（Llm/Condition/LlmClassifier）
    /// 的注册动作延迟到 `init_dispatcher` 阶段在 tokio runtime 中完成。
    /// 存储的是构造好的实例（已经注入 provider_registry），init_dispatcher 时消费。
    /// 用 Option 包裹，便于 init_dispatcher 后清空释放。
    pending_dispatcher_registrations: Arc<tokio::sync::Mutex<Vec<Box<dyn NodeExecutorTrait>>>>,
    /// 触发器管理器（Schedule / Webhook / Event）
    pub trigger_manager: Arc<crate::trigger::TriggerManager>,
    /// 审计记录器（可选，None = 不记录审计日志）
    pub audit_recorder: Arc<std::sync::Mutex<Option<Arc<dyn axagent_harness::AuditRecorder>>>>,
    /// 工具注册表（可选，设置后 tool_executor 优先通过 ToolRegistry.execute_tool() 执行工具）
    tool_registry: Arc<std::sync::Mutex<Option<Arc<dyn axagent_harness::ToolRegistry>>>>,
    /// per-execution partial_result 广播器。LoopExecutor 通过 ExecutionState.partial_result_tx
    /// 拿到 sender，每次迭代完成时 broadcast 一个 PartialResultEvent。
    /// 外部通过 `subscribe_partial_results(execution_id)` 拿到 Receiver 实时观察进度。
    loop_partial_txs: Arc<
        Mutex<
            HashMap<
                String,
                tokio::sync::broadcast::Sender<super::execution_state::PartialResultEvent>,
            >,
        >,
    >,
    /// per-execution Loop interrupt 信号。LoopExecutor 检测到 interrupt 时进入
    /// `interrupt_signal.notified().await`，调用方通过 `resume_loop_iteration`
    /// 触发 `notify_waiters()` 唤醒。
    loop_interrupt_signals: Arc<Mutex<HashMap<String, std::sync::Arc<tokio::sync::Notify>>>>,
}

/// P1-14: 提取节点的类型字符串（用于白名单校验）。
/// 与 `dispatcher.rs::node_type_name` 等价；放这里以让外部 binary 不必
/// 跨 crate 引入 dispatcher 内部函数。
pub fn node_type_of(node: &axagent_harness::workflow_types::WorkflowNode) -> &'static str {
    use axagent_harness::workflow_types::WorkflowNode;
    match node {
        WorkflowNode::Trigger(_) => "trigger",
        WorkflowNode::Agent(_) => "agent",
        WorkflowNode::Tool(_) => "tool",
        WorkflowNode::Condition(_) => "condition",
        WorkflowNode::Switch(_) => "switch",
        WorkflowNode::Loop(_) => "loop",
        WorkflowNode::Parallel(_) => "parallel",
        WorkflowNode::Merge(_) => "merge",
        WorkflowNode::Delay(_) => "delay",
        WorkflowNode::SubWorkflow(_) => "subWorkflow",
        WorkflowNode::End(_) => "end",
        WorkflowNode::Validation(_) => "validation",
        WorkflowNode::Code(_) => "code",
        WorkflowNode::DocumentParser(_) => "documentParser",
        WorkflowNode::VectorRetrieve(_) => "vectorRetrieve",
        WorkflowNode::Debate(_) => "debate",
        WorkflowNode::HttpRequest(_) => "httpRequest",
        WorkflowNode::DatabaseQuery(_) => "databaseQuery",
        WorkflowNode::Notification(_) => "notification",
        WorkflowNode::Approval(_) => "approval",
        WorkflowNode::FileOperation(_) => "fileOperation",
        WorkflowNode::DataTransformer(_) => "dataTransformer",
        WorkflowNode::WebhookSend(_) => "webhookSend",
        WorkflowNode::Logging(_) => "logging",
        WorkflowNode::Storage(_) => "storage",
        WorkflowNode::Llm(_) => "llm",
        WorkflowNode::LlmClassifier(_) => "llmClassifier",
        WorkflowNode::Aggregator(_) => "aggregator",
        WorkflowNode::Email(_) => "email",
        WorkflowNode::WorkflowRef(_) => "workflowRef",
        WorkflowNode::Swarm(_) => "swarm",
    }
}

const _: fn() = || {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<WorkEngine>();
    assert_sync::<WorkEngine>();
};

/// P1-13: WorkEngine 内部锁顺序约束。
///
/// 多重锁路径上必须按以下顺序获取（顺序由低到高），反向则视为潜在死锁：
///
/// 1. `cancel_tokens` (sync `Mutex`)
/// 2. `breakpoints` / `node_breakers` / `loop_partial_txs` (sync `Mutex`)
/// 3. `executions` (sync `Mutex`)
/// 4. `workflows` (tokio `RwLock`)
/// 5. `compiled_prompts` / `compiled_rhai_scripts` (tokio `RwLock`)
/// 6. `dispatcher` (tokio `RwLock`)
///
/// 关键不变量：**先 `executions` 后 `workflows`**。`cancel_workflow` 等
/// 复合操作就是按此顺序：先看 executions 决定哪些 child 要 cancel，
/// 再写 workflows 状态。新增方法必须遵守同样的顺序。
///
/// 调试死锁时，`tokio-console` 或 `tracing` 配合 `#[instrument(skip(self))]`
/// 可以快速定位哪个路径顺序错了。
const _LOCK_ORDER_DOC: fn() = || {};

impl WorkEngine {
    /// 设置断点
    pub async fn set_breakpoints(&self, bp: HashSet<String>) {
        *self.breakpoints.lock().await = bp;
    }
    pub async fn set_breakpoints_for_execution(&self, execution_id: &str, bp: HashSet<String>) {
        *self.breakpoints.lock().await = bp.clone();
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.breakpoints = bp;
        }
    }
    /// 继续执行（通知指定执行的所有等待中的断点 + 恢复运行状态）
    pub async fn resume_breakpoints(&self, execution_id: &str) {
        let signal = {
            let executions = self.executions.lock().await;
            executions.get(execution_id).and_then(|s| s.pause_signal.clone())
        };
        if let Some(sig) = signal {
            sig.notify_waiters();
        }
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id)
            && state.status == ExecutionStatus::Paused
        {
            state.status = ExecutionStatus::Running;
            state.updated_at = Utc::now().timestamp_millis();
        }
    }
    /// 单步执行（仅通知一个等待者 + 恢复运行状态）
    pub async fn step_breakpoint(&self, execution_id: &str) {
        let signal = {
            let executions = self.executions.lock().await;
            executions.get(execution_id).and_then(|s| s.pause_signal.clone())
        };
        if let Some(sig) = signal {
            sig.notify_one();
        }
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id)
            && state.status == ExecutionStatus::Paused
        {
            state.status = ExecutionStatus::Running;
            state.updated_at = Utc::now().timestamp_millis();
        }
    }

    /// 从模板 tool_defs 预编译 Rhai 工具（覆盖 DAG 扫描结果）
    pub async fn precompile_tool_defs(
        &self,
        workflow_id: &str,
        tool_defs: &[axagent_harness::workflow_types::RhaiToolDef],
    ) {
        if tool_defs.is_empty() {
            return;
        }
        // 优先使用注入的 RhaiEngineAdapter（如果有）
        if let Some(ref engine) = self.rhai_engine {
            let json_defs: Vec<serde_json::Value> = tool_defs
                .iter()
                .map(|td| {
                    serde_json::json!({
                        "tool_name": td.tool_name,
                        "code": td.code,
                    })
                })
                .collect();
            engine.register_scripts(&json_defs);
            tracing::info!(
                "[RhaiEngine] adapter 注册了 {} 个工具 for {workflow_id}",
                tool_defs.len()
            );
            return;
        }
        // 降级：使用旧版编译缓存
        let cache = {
            use rhai::Engine;
            let mut engine = Engine::new();
            engine.set_max_operations(50_000);
            engine.set_max_call_levels(16);
            engine.set_max_string_size(1_000_000);
            engine.set_max_array_size(20_000);
            let mut c = RhaiScriptCache::new();
            for td in tool_defs {
                if td.code.is_empty() {
                    continue;
                }
                match engine.compile(&td.code) {
                    Ok(ast) => {
                        c.insert(td.tool_name.clone(), Arc::new(ast));
                    },
                    Err(e) => {
                        tracing::warn!("[RhaiEngine] 编译失败 {}: {}", td.tool_name, e);
                    },
                }
            }
            c
        };
        if !cache.is_empty() {
            tracing::info!(
                "[RhaiEngine] tool_defs 编译了 {} 个工具 for {workflow_id}",
                cache.len()
            );
            self.compiled_rhai_scripts.write().await.insert(workflow_id.to_string(), cache);
        }
    }

    /// Plan 模式专用：把 WorkEngine 自身引用注入到稳定持有的 AgentExecutor，
    /// 使其能在 plan 流程中创建/执行临时工作流。**不再**重新注册 executor
    /// （之前的实现每次都覆盖 dispatcher 里的实例，会丢 provider_registry）。
    pub async fn inject_into_agent_executor(self: &Arc<Self>, engine: Arc<WorkEngine>) {
        self.agent_executor.set_engine(engine);
        if let Some(ref planner) = self.planner {
            self.agent_executor.set_planner(planner.clone());
        }
    }

    /// 注入 Rhai 脚本引擎适配器
    #[must_use]
    pub fn with_rhai_engine(mut self, engine: Arc<dyn RhaiEngineAdapter>) -> Self {
        self.rhai_engine = Some(engine);
        self
    }

    /// 注入 Plan 模式规划器适配器
    #[must_use]
    pub fn with_planner(
        mut self,
        planner: Arc<std::sync::Mutex<dyn axagent_harness::PlannerAdapter>>,
    ) -> Self {
        // 同时注入到 WorkEngine 自身和 AgentExecutor
        self.planner = Some(planner.clone());
        self.agent_executor.set_planner(planner);
        self
    }

    pub async fn execute_node(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let node_id = node.base_id();
        let node_type = node_type_name(node);
        let start = std::time::Instant::now();
        tracing::info!(
            node_id = %node_id,
            node_type,
            "execute_node → dispatch start"
        );
        match self.dispatcher.read().await.dispatch(node, context).await {
            Ok(output) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(
                    node_id = %node_id,
                    node_type,
                    duration_ms,
                    status = "ok",
                    "execute_node → dispatch complete"
                );
                Ok(output)
            },
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::warn!(
                    node_id = %node_id,
                    node_type,
                    duration_ms,
                    error = %e,
                    "execute_node → dispatch failed"
                );
                Err(e)
            },
        }
    }

    pub async fn registered_executor_types(&self) -> Vec<&'static str> {
        self.dispatcher.read().await.registered_types().await
    }

    /// 设置工具 resolver（按需延迟注册，run_workflow 时自动扫描并注册工作流中的工具）
    /// 防呆：若已经注入过 resolver，新的 set 不会覆盖（避免静默丢弃之前注入的
    /// 解析逻辑，如 init 阶段的 builtin/mcp/workflow:: 多路解析）。
    pub async fn set_tool_resolver(&self, resolver: ToolResolver) {
        let mut slot = self.tool_resolver.lock().await;
        if slot.is_some() {
            tracing::warn!(
                "WorkEngine::set_tool_resolver: 已有 resolver 注入，新 resolver 被忽略。"
            );
            return;
        }
        *slot = Some(resolver);
    }
    /// 设置 RAG 知识源检索回调（供 Agent 节点从知识库/记忆/Wiki 检索上下文）
    pub async fn set_rag_callback(&self, cb: RagCallback) {
        *self.rag_callback.lock().await = Some(cb);
    }

    /// 注册领域约束注入回调。
    ///
    /// 回调签名：`(role_name: &str) -> ConstraintBlocks`
    /// 由主 binary（如 stock-analysis）在 setup 时调用一次。
    ///
    /// 与 `set_rag_callback` 模式完全对齐：内部用 Mutex 持有 Option，
    /// 在执行 agent 节点时由本引擎的 `domain_constraints()` getter
    /// 转发给 `AgentExecutor`。
    ///
    /// 多次调用：后者覆盖前者（标准 setter 语义）。
    pub async fn set_domain_constraints(&self, f: DomainConstraintsFn) {
        *self.domain_constraints.lock().expect("domain_constraints mutex poisoned") = Some(f);
    }

    /// 取出当前注册的领域约束（用于在执行 agent 节点时转发给 `AgentExecutor`）。
    ///
    /// 内部 clone 出 Arc，避免锁长时间持有。仅暴露给 crate 内部消费
    /// （engine.rs 的 run_workflow 中转发给 agent_executor）。
    pub(crate) fn domain_constraints(&self) -> Option<DomainConstraintsFn> {
        self.domain_constraints.lock().expect("domain_constraints mutex poisoned").clone()
    }

    /// 注册业务规则引擎。
    ///
    /// 注入后，在执行 agent / tool / httpRequest 等节点时，
    /// 会在 dispatch 之前自动进行规则评估。
    ///
    /// 多次调用：后者覆盖前者（标准 setter 语义）。
    pub async fn set_business_rule_engine(
        &self,
        engine: Arc<crate::business_rules::BusinessRuleEngine>,
    ) {
        *self.business_rule_engine.lock().expect("business_rule_engine mutex poisoned") =
            Some(engine);
    }

    /// 取出当前注册的业务规则引擎（用于在执行节点时注入到 ExecutionState）。
    fn business_rule_engine(&self) -> Option<Arc<crate::business_rules::BusinessRuleEngine>> {
        self.business_rule_engine.lock().expect("business_rule_engine mutex poisoned").clone()
    }

    /// 注册工具注册表（可选，设置后 tool_executor 优先走 ToolRegistry 中心化路径）
    pub async fn set_tool_registry(&self, registry: Arc<dyn axagent_harness::ToolRegistry>) {
        *self.tool_registry.lock().expect("tool_registry mutex poisoned") = Some(registry);
    }

    /// 取出当前注册的工具注册表（用于在执行节点时注入到 ExecutionState）
    fn tool_registry(&self) -> Option<Arc<dyn axagent_harness::ToolRegistry>> {
        self.tool_registry.lock().expect("tool_registry mutex poisoned").clone()
    }
}

impl WorkEngine {
    pub fn new(
        master_key: [u8; 32],
        provider_registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) -> Self {
        let agent_provider_cache = Arc::new(tokio::sync::Mutex::new(None));
        let agent_profile_cache = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        let dispatcher = NodeDispatcher::new();

        // 统一走 HasProviderRegistry trait，避免 5 个 executor 各自实现 with_provider_registry。
        use axagent_harness::HasProviderRegistry;

        let mut llm_exec = LlmExecutor::new(master_key);
        // 唯一构造路径：创建 Arc<AgentExecutor>，dispatcher 与 WorkEngine.agent_executor
        // 共享同一个实例。运行期不再 register(E) 重新注册，避免丢失 provider_registry。
        let mut agent_exec = AgentExecutor::with_shared_caches(
            master_key,
            agent_provider_cache.clone(),
            agent_profile_cache.clone(),
        );
        let mut cond_exec = ConditionExecutor::new(master_key);
        let mut classifier_exec = LlmClassifierExecutor::new(master_key);

        llm_exec.set_provider_registry(provider_registry.clone());
        agent_exec.set_provider_registry(provider_registry.clone());
        cond_exec.set_provider_registry(provider_registry.clone());
        classifier_exec.set_provider_registry(provider_registry.clone());

        let agent_exec = Arc::new(agent_exec);

        // P0-2: dispatcher 的 register_arc 改 async 后，WorkEngine::new 处于同步上下文
        // 不能 await register。改用 pending_dispatcher_registrations 暂存，调用方在
        // tokio runtime 内执行 init_dispatcher 完成最终注册。
        let pending_dispatcher_registrations: Vec<Box<dyn NodeExecutorTrait>> =
            vec![Box::new(llm_exec), Box::new(cond_exec), Box::new(classifier_exec)];
        let pending_dispatcher_registrations =
            Arc::new(tokio::sync::Mutex::new(pending_dispatcher_registrations));

        // Self { ... }
        Self {
            executions: Arc::new(Mutex::new(HashMap::new())),
            workflows: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            compiled_prompts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            compiled_rhai_scripts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            rhai_engine: None,
            planner: None,
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
            dispatcher: Arc::new(tokio::sync::RwLock::new(dispatcher)),
            tool_handlers: Arc::new(Mutex::new(HashMap::new())),
            tool_fallback: Arc::new(Mutex::new(None)),
            tool_resolver: Arc::new(Mutex::new(None)),
            rag_callback: Arc::new(Mutex::new(None)),
            domain_constraints: Arc::new(std::sync::Mutex::new(None)),
            business_rule_engine: Arc::new(std::sync::Mutex::new(None)),
            agent_provider_cache,
            agent_profile_cache,
            breakpoints: Arc::new(Mutex::new(HashSet::new())),
            node_breakers: Arc::new(Mutex::new(HashMap::new())),
            agent_executor: agent_exec,
            pending_dispatcher_registrations,
            trigger_manager: Arc::new(crate::trigger::TriggerManager::new()),
            audit_recorder: Arc::new(std::sync::Mutex::new(None)),
            tool_registry: Arc::new(std::sync::Mutex::new(None)),
            loop_partial_txs: Arc::new(Mutex::new(HashMap::new())),
            loop_interrupt_signals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 初始化触发器管理器，注入引擎引用。
    ///
    /// 由于 `TriggerManager` 需要持有 `Arc<WorkEngine>` 来在触发器触发时
    /// 调用 `run_workflow`，而 `WorkEngine::new()` 返回的是 `Self` 而非
    /// `Arc<Self>`，因此必须在外部将 `WorkEngine` 包装为 `Arc` 后调用本方法。
    ///
    /// 外部调用方式：
    /// ```ignore
    /// let engine = Arc::new(WorkEngine::new(master_key, registry));
    /// rt.block_on(engine.init_trigger_manager());
    /// ```
    pub async fn init_trigger_manager(self: &Arc<Self>) {
        self.trigger_manager.set_engine(self.clone()).await;
    }

    /// P0-2: 在 tokio runtime 中完成 dispatcher 的初始化注册。
    /// 必须在 setup 阶段 rt.block_on 调用，确保 dispatch 工作前所有内置 executor 已就位。
    /// 内部顺序：
    /// 1. init_builtin：注册所有内置 executor（Trigger/Loop/End/Fallback/...）
    /// 2. 注册 self.agent_executor 到 dispatcher（与 self.agent_executor 共享同一 Arc）
    /// 3. 把 pending_dispatcher_registrations 中的 Llm/Condition/LlmClassifier 注册
    pub async fn init_dispatcher(self: &Arc<Self>) {
        let disp = self.dispatcher.read().await;
        disp.init_builtin().await;
        // 共享 Arc：dispatcher 与 self.agent_executor 指向同一 AgentExecutor 实例
        disp.register_arc(self.agent_executor.clone() as Arc<dyn NodeExecutorTrait>).await;
        drop(disp);

        // 取出 pending 中的 Llm / Condition / LlmClassifier 并注册
        let pending: Vec<Box<dyn NodeExecutorTrait>> = {
            let mut guard = self.pending_dispatcher_registrations.lock().await;
            std::mem::take(&mut *guard)
        };
        let disp = self.dispatcher.read().await;
        for exec in pending {
            let exec: Arc<dyn NodeExecutorTrait> = Arc::from(exec);
            disp.register_arc(exec).await;
        }
    }

    /// 注册自定义节点执行器（供外部 crate 扩展）。
    /// 通过 dispatcher 的 register_external 门面方法转发。
    pub async fn register_executor(&self, executor: Arc<dyn NodeExecutorTrait>) {
        self.dispatcher.read().await.register_external(executor).await;
    }

    pub async fn clear_node_breakers(&self) {
        self.node_breakers.lock().await.clear();
    }

    /// 按 workflow_id 清除关联的断路器状态
    pub async fn clear_node_breakers_for_workflow(&self, workflow_id: &str) {
        let mut breakers = self.node_breakers.lock().await;
        breakers.retain(|k, _| !k.starts_with(&format!("{}:", workflow_id)));
    }

    // ── DAG 管理 ──

    /// 创建新工作流 DAG。含重复 ID 检测、依赖校验、Kahn 算法环检测。
    pub async fn create_workflow(
        &self,
        name: &str,
        nodes: Vec<WorkflowNode>,
        edges: Vec<WorkflowEdge>,
    ) -> Result<Workflow, WorkflowError> {
        let workflow_id = format!("workflow_{}", uuid::Uuid::new_v4());

        // 校验：无重复节点 ID
        let mut node_ids: HashSet<&str> = HashSet::new();
        for node in &nodes {
            if !node_ids.insert(node.base_id()) {
                return Err(WorkflowError::DuplicateNodeId(node.base_id().to_string()));
            }
        }

        // 校验：所有 edge 引用的节点必须存在
        for edge in &edges {
            if !node_ids.contains(edge.source.as_str()) {
                return Err(WorkflowError::InvalidDependency {
                    node_id: edge.target.clone(),
                    missing_dep: edge.source.clone(),
                });
            }
            if !node_ids.contains(edge.target.as_str()) {
                return Err(WorkflowError::InvalidDependency {
                    node_id: edge.source.clone(),
                    missing_dep: edge.target.clone(),
                });
            }
        }

        // 校验：无环（Kahn 算法）
        {
            let mut in_degree: HashMap<&str, usize> = HashMap::new();
            let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
            for node in &nodes {
                in_degree.entry(node.base_id()).or_insert(0);
            }
            for edge in &edges {
                adj.entry(edge.source.as_str()).or_default().push(edge.target.as_str());
                *in_degree.entry(edge.target.as_str()).or_insert(0) += 1;
            }
            let mut queue: Vec<&str> =
                in_degree.iter().filter(|&(_, &deg)| deg == 0).map(|(&id, _)| id).collect();
            let mut visited = 0usize;
            while let Some(node) = queue.pop() {
                visited += 1;
                if let Some(neighbors) = adj.get(node) {
                    for &neighbor in neighbors {
                        if let Some(deg) = in_degree.get_mut(neighbor) {
                            *deg -= 1;
                            if *deg == 0 {
                                queue.push(neighbor);
                            }
                        }
                    }
                }
            }
            if visited != nodes.len() {
                return Err(WorkflowError::CycleDetected);
            }
        }

        let node_states: HashMap<String, NodeRuntimeState> =
            nodes.iter().map(|n| (n.base_id().to_string(), NodeRuntimeState::default())).collect();

        // 编译 Agent 节点的 prompt 模板（阶段一）
        let mut compiled_map: HashMap<String, CompiledPrompt> = HashMap::new();
        for node in &nodes {
            if let WorkflowNode::Agent(an) = node {
                compiled_map.insert(an.base.id.clone(), compile_prompt(&an.config.system_prompt));
            }
        }
        self.compiled_prompts.write().await.insert(workflow_id.clone(), compiled_map);

        // Rhai 工具由 precompile_tool_defs() 单独注册，不在 create_workflow 中编译

        let workflow = Workflow {
            id: workflow_id.clone(),
            name: name.to_string(),
            nodes,
            edges,
            status: WorkflowStatus::Created,
            created_at: current_timestamp(),
            completed_at: None,
            results: HashMap::new(),
            node_states,
            output: None,
            error_config: None,
            error_workflow_id: None,
        };

        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id.clone(), workflow.clone());

        tracing::info!(
            workflow_id = %workflow_id,
            name = %name,
            node_count = workflow.nodes.len(),
            edge_count = workflow.edges.len(),
            "DAG created"
        );

        Ok(workflow)
    }

    /// 获取依赖节点的输出结果（根据 edges 确定依赖关系）
    pub(crate) fn get_node_dependency_results(
        workflow: &Workflow,
        node_id: &str,
    ) -> HashMap<String, serde_json::Value> {
        let deps: Vec<&str> = workflow
            .edges
            .iter()
            .filter(|e| e.target == node_id)
            .map(|e| e.source.as_str())
            .collect();

        let mut results = HashMap::new();
        for dep_id in deps {
            if let Some(result) = workflow.results.get(dep_id) {
                results.insert(dep_id.to_string(), result.clone());
            }
        }
        results
    }

    pub async fn get_ready_steps(&self, workflow_id: &str) -> Result<Vec<String>, WorkflowError> {
        let workflows = self.workflows.read().await;
        let workflow = workflows.get(workflow_id).ok_or(WorkflowError::WorkflowNotFound)?;
        Ok(Self::compute_ready_nodes(workflow))
    }

    /// 更新节点运行时状态，自动推进工作流终端判定
    pub async fn update_node_status(
        &self,
        workflow_id: &str,
        node_id: &str,
        status: NodeStatus,
        result: Option<serde_json::Value>,
        error: Option<String>,
        output_var: Option<&str>,
    ) -> Result<(), WorkflowError> {
        let mut workflows = self.workflows.write().await;
        let workflow = workflows.get_mut(workflow_id).ok_or(WorkflowError::WorkflowNotFound)?;

        let state = workflow.node_states.get_mut(node_id).ok_or(WorkflowError::NodeNotFound)?;

        state.status = status;
        // ── 时间戳维护：保证 started_at/completed_at 在 status 变化时正确更新 ──
        // 修复前：这些字段从未被写入，导致所有节点 completed_at 为 None，
        // 前端无法显示节点耗时、审计也无法用 started_at/completed_at 排序。
        let now_ms: i64 = current_epoch_ms() as i64;
        if status == NodeStatus::Running && state.started_at.is_none() {
            state.started_at = Some(now_ms);
        }
        if matches!(status, NodeStatus::Completed | NodeStatus::Failed | NodeStatus::Skipped) {
            if state.started_at.is_none() {
                state.started_at = Some(now_ms);
            }
            if state.completed_at.is_none() {
                state.completed_at = Some(now_ms);
            }
        }
        if status == NodeStatus::Completed {
            state.attempts = 0;
        }
        if let Some(r) = result {
            workflow.results.insert(node_id.to_string(), r.clone());
            if let Some(var) = output_var {
                workflow.results.insert(var.to_string(), r);
            }
        }
        if let Some(e) = error {
            state.error = Some(e);
            state.attempts += 1;
        }

        // ── 回滚补偿：节点标记为 Failed 时，根据补偿策略执行操作 ──
        if status == NodeStatus::Failed
            && let Some(node) = workflow.nodes.iter().find(|n| n.base_id() == node_id)
            && let Some(ref comp) = node.base().compensation
        {
            match comp.strategy {
                CompensationStrategy::SkipWithWarning => {
                    // 删除该节点输出
                    workflow.results.remove(node_id);
                    tracing::info!("[补偿] 节点 {} (SkipWithWarning): 已移除失败输出", node_id);
                },
                CompensationStrategy::Rollback => {
                    // 删除该节点输出
                    workflow.results.remove(node_id);
                    // 收集所有下游 Pending/Ready 节点并标记为 Skipped
                    let downstream_ids: Vec<String> = workflow
                        .edges
                        .iter()
                        .filter(|e| e.source == node_id)
                        .map(|e| e.target.clone())
                        .collect();
                    for dep_id in &downstream_ids {
                        if let Some(dep_state) = workflow.node_states.get_mut(dep_id)
                            && matches!(dep_state.status, NodeStatus::Pending | NodeStatus::Ready)
                        {
                            dep_state.status = NodeStatus::Skipped;
                        }
                        // 同时清理下游结果
                        workflow.results.remove(dep_id.as_str());
                    }
                    tracing::info!(
                        "[补偿] 节点 {} (Rollback): 已移除输出并跳过 {} 个下游节点",
                        node_id,
                        downstream_ids.len()
                    );
                },
                CompensationStrategy::Escalate => {
                    tracing::warn!("[补偿] 节点 {} 需要人工处理 (Escalate)", node_id);
                },
            }
        }

        // 判定工作流终端状态
        let all_done = workflow.node_states.values().all(|s| {
            matches!(s.status, NodeStatus::Completed | NodeStatus::Skipped | NodeStatus::Failed)
        });
        let all_ok = workflow
            .node_states
            .values()
            .all(|s| matches!(s.status, NodeStatus::Completed | NodeStatus::Skipped));
        let any_skipped = workflow.node_states.values().any(|s| s.status == NodeStatus::Skipped);
        let any_failed = workflow.node_states.values().any(|s| s.status == NodeStatus::Failed);

        if all_ok && any_skipped {
            workflow.status = WorkflowStatus::PartiallyCompleted;
            workflow.completed_at = Some(current_timestamp());
        } else if all_ok {
            workflow.status = WorkflowStatus::Completed;
            workflow.completed_at = Some(current_timestamp());
        } else if all_done && any_failed {
            workflow.status = WorkflowStatus::Failed;
            workflow.completed_at = Some(current_timestamp());
        }

        Ok(())
    }

    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Option<Workflow>, WorkflowError> {
        let workflows = self.workflows.read().await;
        Ok(workflows.get(workflow_id).cloned())
    }

    pub async fn list_workflows(&self) -> Result<Vec<Workflow>, WorkflowError> {
        let workflows = self.workflows.read().await;
        Ok(workflows.values().cloned().collect())
    }

    pub async fn cancel_workflow(&self, workflow_id: &str) -> Result<Workflow, WorkflowError> {
        {
            let tokens = self.cancel_tokens.lock().await;
            if let Some(token) = tokens.get(workflow_id) {
                token.cancel();
            }
        }

        // 同步取消所有关联的 DB 执行记录
        {
            let running_exec_ids: Vec<String> = {
                let executions = self.executions.lock().await;
                executions
                    .iter()
                    .filter(|(_, s)| {
                        s.workflow_id == workflow_id && s.status == ExecutionStatus::Running
                    })
                    .map(|(id, _)| id.clone())
                    .collect()
            };
            for exec_id in &running_exec_ids {
                workflow_execution_repository()
                    .update_workflow_execution_status(exec_id, "cancelled", None, None, None)
                    .await
                    .ok();
            }
        }

        let mut workflows = self.workflows.write().await;
        let workflow = workflows.get_mut(workflow_id).ok_or(WorkflowError::WorkflowNotFound)?;

        for state in workflow.node_states.values_mut() {
            if matches!(state.status, NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Running)
            {
                state.status = NodeStatus::Skipped;
            }
        }
        workflow.status = WorkflowStatus::Cancelled;
        workflow.completed_at = Some(current_timestamp());

        Ok(workflow.clone())
    }

    pub async fn serialize_workflow(&self, workflow_id: &str) -> Result<String, WorkflowError> {
        let workflows = self.workflows.read().await;
        let wf = workflows.get(workflow_id).ok_or(WorkflowError::WorkflowNotFound)?;
        serde_json::to_string(wf).map_err(|e| WorkflowError::SerializationError(e.to_string()))
    }

    // ── 核心执行 ──

    /// 运行工作流：按 DAG 拓扑顺序逐节点执行 + 重试 + 断路器 + 超时 + DB 持久化。
    ///
    /// 每个 `WorkflowNode` 通过 `self.dispatcher` 分发到对应执行器。
    /// 执行上下文（`ExecutionState`）包含依赖节点的输出结果，下游节点可直接引用。
    pub async fn run_workflow(
        &self,
        workflow_id: &str,
        options: RunOptions,
    ) -> Result<Workflow, WorkflowError> {
        let span = tracing::info_span!(
            "run_workflow",
            workflow_id = %workflow_id,
            execution_id = %options.execution_id.as_deref().unwrap_or("auto"),
            max_concurrent = options.max_concurrent,
        );
        let _guard = span.enter();

        let cancel_token =
            options.parent_cancel_token.as_ref().map(|t| t.child_token()).unwrap_or_default();
        {
            let mut tokens = self.cancel_tokens.lock().await;
            tokens.insert(workflow_id.to_string(), cancel_token.clone());
        }

        // 构建执行输入：优先使用调用方传入的 input，否则用空对象
        let mut input = options.input.clone().unwrap_or_else(|| serde_json::json!({}));
        // 将 model_id / provider_id 写入上下文，供执行器读取
        if let Some(ref model_id) = options.model_id {
            input[super::executors::WORKFLOW_MODEL_VAR] =
                serde_json::Value::String(model_id.clone());
        }
        if let Some(ref provider_id) = options.provider_id {
            input[super::executors::WORKFLOW_PROVIDER_ID_VAR] =
                serde_json::Value::String(provider_id.clone());
        }

        // 先创建 execution 记录，再校验 input_schema。
        // 这样校验失败也能在 DB 里留下审计记录（status=Failed，error=validation message），
        // 前端可以在工作流历史里看到这次"参数不合法"的尝试。
        let execution_id = self
            .start_workflow(workflow_id, input.clone(), options.execution_id.clone())
            .await
            .map_err(|e| WorkflowError::SerializationError(e.to_string()))?;

        // 若配置了 input_schema，校验输入参数
        if let Some(ref schema) = options.input_schema
            && let Err(errors) = validate_input(&input, schema)
        {
            let detail = format!("input_schema 校验失败：{}", errors.join("; "));
            tracing::warn!("[rt-workflow] execution {} {}", execution_id, detail);
            // 持久化失败状态（status="failed"），便于审计与前端展示。
            // 注意：workflow_executions 表当前没有 error_message 字段，错误细节
            // 仅记入日志；后续若加字段再透传。
            if let Err(e) = workflow_execution_repository()
                .update_workflow_execution_status(&execution_id, "failed", None, None, None)
                .await
            {
                tracing::error!(
                    "[rt-workflow] 持久化校验失败状态失败: {e} (execution_id={execution_id})"
                );
            }
            return Err(WorkflowError::InputValidationFailed { errors });
        }

        // 一次性 lock executions 完成 5 项元信息写入（model_id / provider_id /
        // template variables / plan_callbacks / parent_execution_id），避免
        // 5 次独立 lock-then-release 的抖动。
        {
            let mut executions = self.executions.lock().await;
            if let Some(state) = executions.get_mut(&execution_id) {
                if let Some(ref model_id) = options.model_id {
                    state.variables.insert(
                        super::executors::WORKFLOW_MODEL_VAR.to_string(),
                        serde_json::Value::String(model_id.clone()),
                    );
                }
                if let Some(ref provider_id) = options.provider_id {
                    state.variables.insert(
                        super::executors::WORKFLOW_PROVIDER_ID_VAR.to_string(),
                        serde_json::Value::String(provider_id.clone()),
                    );
                }
                if let Some(ref variables) = options.variables {
                    for var in variables {
                        state.variables.insert(var.name.clone(), var.value.clone());
                    }
                }
                if options.plan_callbacks.is_some() {
                    state.plan_callbacks = options.plan_callbacks.clone();
                }
                if options.parent_execution_id.is_some() {
                    state.parent_execution_id = options.parent_execution_id.clone();
                }
            }
        }

        {
            let mut workflows = self.workflows.write().await;
            if let Some(workflow) = workflows.get_mut(workflow_id) {
                workflow.status = WorkflowStatus::Running;
            }
        }

        let current_parent_execution_id = {
            let executions = self.executions.lock().await;
            executions.get(&execution_id).and_then(|s| s.parent_execution_id.clone())
        };

        // 清空 Agent executor 缓存（每次执行使用最新数据）
        {
            *self.agent_provider_cache.lock().await = None;
            self.agent_profile_cache.lock().await.clear();
        }

        // 同步 RAG callback 到共享 AgentExecutor 槽（热更新，不再重新注册）。
        // 缓存清空已在上面 block 完成；agent_executor 实例是稳定的，所以
        // provider_registry 等一次性注入的字段始终保留。
        {
            let rag_cb = self.rag_callback.lock().await.clone();
            self.agent_executor.set_rag_callback(rag_cb);
        }

        // 同步 DomainConstraints 到共享 AgentExecutor 槽
        {
            let dc = self.domain_constraints();
            self.agent_executor.set_domain_constraints_option(dc);
        }

        // 自动扫描工作流节点中的工具定义，按需注册（模板级工具自动注册）
        {
            let resolver_opt = self.tool_resolver.lock().await.clone();
            if let Some(ref resolver) = resolver_opt {
                let workflows = self.workflows.read().await;
                if let Some(wf) = workflows.get(workflow_id) {
                    let tool_names = collect_workflow_tool_names(&wf.nodes);
                    let mut handlers = self.tool_handlers.lock().await;
                    for name in tool_names {
                        if !handlers.contains_key(&name) {
                            if let Some(cb) = resolver(name.clone()).await {
                                tracing::info!(
                                    "[WorkEngine] 自动注册工具: {} (来自工作流 {})",
                                    name,
                                    workflow_id
                                );
                                handlers.insert(name.clone(), cb);
                            } else {
                                tracing::warn!(
                                    "[WorkEngine] 工具 '{}' 在注册表中未找到 (工作流 {})",
                                    name,
                                    workflow_id
                                );
                            }
                        }
                    }
                }
            }
        }

        // 自动注册 ToolRegistry 中的工具（tool_resolver 未命中时的补充路径）
        {
            let reg_opt = self.tool_registry();
            if let Some(ref registry) = reg_opt {
                let workflows = self.workflows.read().await;
                if let Some(wf) = workflows.get(workflow_id) {
                    let tool_names = collect_workflow_tool_names(&wf.nodes);
                    let mut handlers = self.tool_handlers.lock().await;
                    for name in tool_names {
                        if !handlers.contains_key(&name) {
                            if let Some(_tool) = registry.find(&name) {
                                let reg_clone = registry.clone();
                                let tn = name.clone();
                                let cb: ToolCallback = std::sync::Arc::new(
                                    move |_tn: String, args: serde_json::Value| {
                                        let reg = reg_clone.clone();
                                        let tool_name = tn.clone();
                                        Box::pin(async move {
                                            let ctx = axagent_harness::tool::ToolContext::new(".");
                                            match reg.execute_tool(&tool_name, args, &ctx).await {
                                                Ok(result) => Ok(serde_json::json!({
                                                    "tool_name": tool_name,
                                                    "result": result.content,
                                                    "truncated": result.truncated,
                                                    "is_error": result.is_error,
                                                })),
                                                Err(e) => {
                                                    Err(format!("ToolRegistry 调用失败: {e}"))
                                                },
                                            }
                                        })
                                    },
                                );
                                tracing::info!(
                                    "[WorkEngine] 通过 ToolRegistry 自动注册工具: {}",
                                    name
                                );
                                handlers.insert(name.clone(), cb);
                            } else {
                                tracing::debug!(
                                    "[WorkEngine] 工具 '{}' 在 ToolRegistry 中也未找到",
                                    name
                                );
                            }
                        }
                    }
                }
            }
        }

        // 注册 Rhai 脚本工具（从编译缓存）
        // 优先使用注入的 RhaiEngineAdapter（如果注入），否则走旧版 AST 缓存路径
        if self.rhai_engine.is_some() {
            // adapter 已在 precompile_tool_defs 中完成编译，
            // 执行时由 adapter 内部查找并运行脚本，无需在此注册回调。
            tracing::debug!("[RhaiEngine] 使用 RhaiEngineAdapter，跳过旧版回调注册");
        } else {
            let rhai_cache = self.compiled_rhai_scripts.read().await;
            if let Some(scripts) = rhai_cache.get(workflow_id) {
                let mut handlers = self.tool_handlers.lock().await;
                for (tool_name, ast) in scripts {
                    if !handlers.contains_key(tool_name) {
                        let ast = ast.clone();
                        let tool_handlers = self.tool_handlers.clone();
                        let cb: ToolCallback = std::sync::Arc::new(
                            move |_tn: String, _args: serde_json::Value| {
                                let ast = ast.clone();
                                let tool_handlers = tool_handlers.clone();
                                Box::pin(async move {
                                    let handlers = tool_handlers.lock().await;
                                    let mut rhai_tools: std::collections::HashMap<
                                        String,
                                        LocalRhaiToolFn,
                                    > = std::collections::HashMap::new();
                                    for (k, v) in handlers.iter() {
                                        let k = k.clone();
                                        let v = v.clone();
                                        rhai_tools.insert(
                                            k,
                                            std::sync::Arc::new(
                                                move |name: String, args: serde_json::Value| {
                                                    let v = v.clone();
                                                    Box::pin(async move { v(name, args).await })
                                                },
                                            ),
                                        );
                                    }
                                    drop(handlers);
                                    {
                                        // Inline Rhai execution (no dependency on axagent-tools)
                                        use rhai::{Dynamic, Engine, Scope};
                                        let mut engine = Engine::new();
                                        // SECURITY (C4): Rhai 沙箱限制
                                        engine.set_max_operations(100_000);
                                        engine.set_max_call_levels(24);
                                        engine.set_max_modules(0);
                                        engine.set_max_string_size(2_000_000);
                                        engine.set_max_array_size(50_000);
                                        // 注册通用函数（json_parse/clamp/join），与 code_executor 一致
                                        crate::work_engine::executors::register_common_functions(&mut engine);
                                        let _scope = Scope::new();
                                        // Register tool functions
                                        for (name, handler) in &rhai_tools {
                                            let h = handler.clone();
                                            let n = name.clone();
                                            engine.register_fn(
                                                "tool",
                                                move |tool_name: &str,
                                                      tool_args: rhai::Map|
                                                      -> Result<Dynamic, Box<EvalAltResult>> {
                                                    let h = h.clone();
                                                    let json_args = rhai_map_to_json(tool_args);
                                                    // 关键修复：避免在 Rhai 调用栈内
                                                    // `Runtime::new() + block_on` 的嵌套
                                                    // runtime 反模式。spawn_blocking 线程
                                                    // 仍处于当前 tokio runtime，因此用
                                                    // `Handle::current().block_on()` +
                                                    // `block_in_place` 是 tokio 官方推荐
                                                    // 的安全模式。
                                                    let result = match tokio::runtime::Handle::try_current() {
                                                        Ok(handle) => {
                                                            tokio::task::block_in_place(|| {
                                                                handle.block_on(async move {
                                                                    h(
                                                                        tool_name.to_string(),
                                                                        json_args,
                                                                    )
                                                                    .await
                                                                })
                                                            })
                                                        },
                                                        Err(_) => {
                                                            return Err(Box::new(
                                                                EvalAltResult::ErrorRuntime(
                                                                    "no tokio runtime available"
                                                                        .into(),
                                                                    Position::NONE,
                                                                ),
                                                            ));
                                                        },
                                                    };
                                                    match result {
                                                        Ok(val) => Ok(Dynamic::from(
                                                            serde_json::to_string(&val)
                                                                .unwrap_or_default(),
                                                        )),
                                                        Err(e) => Err(Box::new(
                                                            EvalAltResult::ErrorRuntime(
                                                                format!("tool error: {e}").into(),
                                                                Position::NONE,
                                                            ),
                                                        )),
                                                    }
                                                },
                                            );
                                            _ = n;
                                        }
                                        let result = engine
                                            .eval_ast::<Dynamic>(&ast)
                                            .map_err(|e| format!("Rhai 执行失败: {e}"))?;
                                        // Convert result back to json value
                                        let text = result.to_string();
                                        serde_json::from_str::<serde_json::Value>(&text)
                                            .map_err(|e| format!("Result not json: {e}"))
                                    }
                                    .map(|v| serde_json::json!({"content": v}))
                                })
                            },
                        );
                        handlers.insert(tool_name.clone(), cb);
                    }
                }
            }
        }

        // 懒编译兜底：若工作流从 DB 加载（非 create_workflow 新建），编译模板。
        // Phase 1: deduplicate by template hash — same template across nodes compiles once.
        {
            let compiled = self.compiled_prompts.read().await;
            if !compiled.contains_key(workflow_id) {
                drop(compiled);
                let workflows = self.workflows.read().await;
                if let Some(wf) = workflows.get(workflow_id) {
                    let mut compiled_map: HashMap<String, CompiledPrompt> = HashMap::new();
                    // Dedup index: template_hash → node_id (first node with this template)
                    let mut template_dedup: HashMap<u64, String> = HashMap::new();
                    for node in &wf.nodes {
                        if let WorkflowNode::Agent(an) = node {
                            let raw = &an.config.system_prompt;
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            std::hash::Hash::hash(raw, &mut hasher);
                            let tpl_hash = std::hash::Hasher::finish(&hasher);
                            if let Some(existing_node) = template_dedup.get(&tpl_hash) {
                                // Reuse compiled prompt from the first node with same template
                                if let Some(cp) = compiled_map.get(existing_node) {
                                    compiled_map.insert(an.base.id.clone(), cp.clone());
                                    continue;
                                }
                            }
                            template_dedup.insert(tpl_hash, an.base.id.clone());
                            compiled_map.insert(an.base.id.clone(), compile_prompt(raw));
                        }
                    }
                    self.compiled_prompts
                        .write()
                        .await
                        .insert(workflow_id.to_string(), compiled_map);
                }
            }
        }

        // Rhai 工具仅从 tool_defs 编译（通过 precompile_tool_defs 调用），
        // DAG 节点不再作为工具来源

        let total_nodes = {
            let workflows = self.workflows.read().await;
            workflows.get(workflow_id).map(|w| w.nodes.len()).unwrap_or(0)
        };
        let progress_cb = options.progress_callback.clone();
        let mut breakers: HashMap<String, NodeCircuitBreaker> =
            { self.node_breakers.lock().await.clone() };

        // ── 构建分支降级策略映射 ──
        // 从 Parallel 节点的 branch 配置中提取每个子节点的 degrade_strategy，
        // 用于超时处理时判断是 skip / useDefault / strict。
        let degrade_map: HashMap<String, DegradeStrategy> = {
            let workflows = self.workflows.read().await;
            let mut map = HashMap::new();
            if let Some(wf) = workflows.get(workflow_id) {
                for node in &wf.nodes {
                    if let WorkflowNode::Parallel(pn) = node {
                        for branch in &pn.config.branches {
                            for step_id in &branch.steps {
                                map.insert(step_id.clone(), branch.degrade_strategy.clone());
                            }
                        }
                    }
                }
            }
            map
        };

        loop {
            if cancel_token.is_cancelled() {
                self.finalize_cancelled_workflow(workflow_id).await;
                self.cancel(&execution_id).await.ok();
                break;
            }

            let is_paused = {
                let executions = self.executions.lock().await;
                executions
                    .get(&execution_id)
                    .map(|s| s.status == ExecutionStatus::Paused)
                    .unwrap_or(false)
            };
            if is_paused {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            // 1. 取就绪节点（支持并行调度）
            let ready_nodes = self.get_ready_steps(workflow_id).await?;
            if ready_nodes.is_empty() {
                // P1-17: 死锁检测 —— 增加 5s grace period，避免在节点刚完成、上游
                // 状态尚未传播到下游时被误判为死锁；同时区分"上游 Failed/Skipped"
                // （真死锁）和"上游 Running 但下游已被错误判定为 Pending"（假死锁）。
                let mut workflows = self.workflows.write().await;
                if let Some(wf) = workflows.get_mut(workflow_id) {
                    // 5s grace period：最近 5s 内有节点完成 → 继续等下一轮
                    let now = current_epoch_ms() as i64;
                    let last_completion =
                        wf.node_states.values().filter_map(|s| s.completed_at).max();
                    let within_grace = last_completion.map(|t| now - t < 5_000).unwrap_or(false);
                    if within_grace {
                        // 仍在 grace period 内，释放锁让其他路径进展
                        drop(workflows);
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    // 区分依赖未完成 vs 已 Skip
                    let has_blocked = wf
                        .node_states
                        .values()
                        .any(|s| matches!(s.status, NodeStatus::Pending | NodeStatus::Ready));
                    if has_blocked {
                        // 先收集需要标记为 Skipped 的节点 key，避免 mutable + immutable
                        // 双重借用 wf.node_states。
                        let keys_to_skip: Vec<String> = wf
                            .node_states
                            .iter()
                            .filter_map(|(state_key, state)| {
                                if !matches!(state.status, NodeStatus::Pending | NodeStatus::Ready)
                                {
                                    return None;
                                }
                                let upstream_terminal = wf.edges.iter().all(|e| {
                                    if e.target != *state_key {
                                        return true;
                                    }
                                    matches!(
                                        wf.node_states.get(&e.source).map(|s| &s.status),
                                        Some(NodeStatus::Skipped | NodeStatus::Failed)
                                    )
                                });
                                upstream_terminal.then(|| state_key.clone())
                            })
                            .collect();
                        for key in keys_to_skip {
                            if let Some(state) = wf.node_states.get_mut(&key) {
                                state.status = NodeStatus::Skipped;
                            }
                        }
                        wf.status = WorkflowStatus::PartiallyCompleted;
                        wf.completed_at = Some(current_timestamp());
                    }
                }
                break;
            };

            // Track active node IDs for inter-batch early scheduling and per-type limits
            let mut active_nodes: std::collections::HashSet<String> =
                std::collections::HashSet::new();

            // Determine per-node-type concurrency limits
            let type_limits = options.max_concurrent_by_type.clone().unwrap_or_default();
            let global_limit = options.max_concurrent;

            // Pre-compute node type map to avoid async access in filter closure
            let node_type_map: HashMap<String, String> = {
                let workflows = self.workflows.read().await;
                let wf = workflows.get(workflow_id);
                wf.map(|wf| {
                    wf.nodes
                        .iter()
                        .map(|n| (n.base_id().to_string(), node_type_name(n).to_string()))
                        .collect()
                })
                .unwrap_or_default()
            };

            let batch: Vec<String> = ready_nodes
                .into_iter()
                .filter(|nid| {
                    if type_limits.is_empty() {
                        return true;
                    }
                    if let Some(nt) = node_type_map.get(nid) {
                        let limit = type_limits.get(nt.as_str()).copied().unwrap_or(global_limit);
                        let active_of_type = active_nodes
                            .iter()
                            .filter(|an| {
                                node_type_map.get(an.as_str()).map(|t| t == nt).unwrap_or(false)
                            })
                            .count();
                        if active_of_type >= limit {
                            return false;
                        }
                    }
                    true
                })
                .take(options.max_concurrent)
                .collect();

            active_nodes.extend(batch.iter().cloned());

            let mut join_set: tokio::task::JoinSet<NodeResult> = tokio::task::JoinSet::new();

            for node_id in &batch {
                let node = {
                    let workflows = self.workflows.read().await;
                    workflows
                        .get(workflow_id)
                        .and_then(|wf| wf.nodes.iter().find(|n| n.base_id() == node_id).cloned())
                };
                let Some(node) = node else {
                    continue;
                };

                let cb_open = breakers
                    .entry(format!("{}:{}", workflow_id, node_id))
                    .or_insert_with(NodeCircuitBreaker::new)
                    .is_open(current_epoch_ms());
                if cb_open {
                    tracing::warn!(
                        workflow_id = %workflow_id,
                        node_id = %node_id,
                        "Circuit breaker open — skipping node"
                    );
                    self.update_node_status(
                        workflow_id,
                        node_id,
                        NodeStatus::Failed,
                        None,
                        Some("Circuit breaker open".to_string()),
                        None,
                    )
                    .await
                    .ok();
                    continue;
                }

                let deps_results = {
                    let workflows = self.workflows.read().await;
                    workflows
                        .get(workflow_id)
                        .map(|wf| Self::get_node_dependency_results(wf, node_id))
                        .unwrap_or_default()
                };
                let input_snapshot =
                    serde_json::to_value(&deps_results).unwrap_or(serde_json::json!({}));
                let started_at = Utc::now().timestamp_millis();

                self.update_node_status(
                    workflow_id,
                    node_id,
                    NodeStatus::Running,
                    None,
                    None,
                    None,
                )
                .await
                .ok();

                if let Some(ref cb) = progress_cb {
                    let completed = {
                        let workflows = self.workflows.read().await;
                        workflows
                            .get(workflow_id)
                            .map(|w| {
                                w.node_states
                                    .values()
                                    .filter(|s| {
                                        matches!(
                                            s.status,
                                            NodeStatus::Completed
                                                | NodeStatus::Failed
                                                | NodeStatus::Skipped
                                        )
                                    })
                                    .count()
                            })
                            .unwrap_or(0)
                    };
                    cb(StepProgressEvent {
                        node_id: node_id.clone(),
                        status: "running".to_string(),
                        total_nodes,
                        completed_nodes: completed,
                        execution_id: Some(execution_id.clone()),
                    })
                    .await;
                }

                let node_timeout =
                    node.base_timeout().map(Duration::from_secs).unwrap_or(options.step_timeout);
                let mut exec_ctx = ExecutionState::new(
                    format!("node_{}", uuid::Uuid::new_v4()),
                    workflow_id.to_string(),
                    serde_json::json!({}),
                );
                // 关键修复：合并工作流全局变量与上游节点结果。历史 bug：
                // 直接 `exec_ctx.variables = deps_results` 会丢失 stock_code 等
                // 全局变量 → tool 节点 input_mapping 解析 stock_code 返回 None
                // → "stock_code不能为空"。
                // 合并策略（按优先级，覆盖优先于 fallback）：
                //   1) deps_results        — 上游节点输出
                //   2) state.variables     — options.variables 注入的 stock_code 等
                //   3) state.input_params  — start_workflow(input) 透传
                let mut merged_vars: HashMap<String, serde_json::Value> = deps_results;
                {
                    let executions = self.executions.lock().await;
                    if let Some(state) = executions.get(&execution_id) {
                        for (k, v) in &state.variables {
                            merged_vars.entry(k.clone()).or_insert_with(|| v.clone());
                        }
                        // input_params 兜底：兼容只设置 options.input 的调用方
                        if let serde_json::Value::Object(map) = &state.input_params {
                            for (k, v) in map {
                                merged_vars.entry(k.clone()).or_insert_with(|| v.clone());
                            }
                        }
                    }
                }
                exec_ctx.variables = merged_vars;
                exec_ctx.cancel_token = Some(cancel_token.clone());
                exec_ctx.dry_run = options.dry_run;
                {
                    let bp = self.breakpoints.lock().await;
                    exec_ctx.breakpoints = bp.clone();
                }
                // 注入业务规则引擎（可选），在执行节点前进行硬约束检查
                exec_ctx.business_rule_engine = self.business_rule_engine();
                // 注入工具注册表（可选），tool_executor 优先走中心化路径
                exec_ctx.tool_registry = self.tool_registry();

                let exec_pause_signal = {
                    let mut executions = self.executions.lock().await;
                    if let Some(state) = executions.get_mut(&execution_id) {
                        if state.pause_signal.is_none() {
                            state.pause_signal = Some(Arc::new(tokio::sync::Notify::new()));
                        }
                        state.pause_signal.clone().unwrap_or_else(|| {
                            let new_signal = Arc::new(tokio::sync::Notify::new());
                            state.pause_signal = Some(new_signal.clone());
                            new_signal
                        })
                    } else {
                        Arc::new(tokio::sync::Notify::new())
                    }
                };
                exec_ctx.pause_signal = Some(exec_pause_signal.clone());

                if exec_ctx.breakpoints.contains(node_id.as_str()) {
                    tracing::info!("[Breakpoint] 命中节点 {node_id}，等待 resume...");
                    {
                        let mut executions = self.executions.lock().await;
                        if let Some(state) = executions.get_mut(&execution_id) {
                            state.status = ExecutionStatus::Paused;
                            state.current_node_id = Some(node_id.clone());
                            state.updated_at = Utc::now().timestamp_millis();
                        }
                    }
                    exec_pause_signal.notified().await;
                }

                {
                    let compiled = self.compiled_prompts.read().await;
                    exec_ctx.compiled_prompts = compiled.get(workflow_id).cloned();
                }

                {
                    let tool_handlers = self.tool_handlers.lock().await.clone();
                    let tool_fallback = self.tool_fallback.lock().await.clone();
                    // 将 tool handlers 注入 per-node exec_ctx，供 tool executor 回调路径使用
                    exec_ctx.callbacks = Some(ExecutionContextCallbacks {
                        tool_handlers: tool_handlers.clone(),
                        tool_fallback: tool_fallback.clone(),
                        trigger_manager: None,
                        subworkflow: None,
                        loop_body_dispatch: None,
                        loop_checkpoint: None,
                    });

                    let engine_clone = self.clone();
                    let sub_model_id = options.model_id.clone();
                    let sub_provider_id = options.provider_id.clone();
                    let sub_step_timeout = options.step_timeout;
                    let sub_cancel_token = cancel_token.clone();
                    let sub_progress_cb = progress_cb.clone();
                    let sub_dry_run = options.dry_run;

                    let sub_cb: SubWorkflowCallback =
                        Arc::new(
                            move |sub_workflow_id: String,
                                  parent_execution_id: String,
                                  input_vars: std::collections::HashMap<
                                String,
                                serde_json::Value,
                            >| {
                                let engine = engine_clone.clone();
                                let model_id = sub_model_id.clone();
                                let provider_id = sub_provider_id.clone();
                                let cancel_token = sub_cancel_token.clone();
                                let progress_cb = sub_progress_cb.clone();
                                let dry_run = sub_dry_run;
                                let child_execution_id = uuid::Uuid::new_v4().to_string();
                                let child_eid_for_result = child_execution_id.clone();

                                // run_workflow() 返回 non-Send future（包含 Rc 等），
                                // 因此无法用 tokio::spawn。改用 spawn_blocking +
                                // current_thread runtime 在新线程中执行。
                                let (tx, rx) = tokio::sync::oneshot::channel();
                                tokio::task::spawn_blocking(move || {
                                    let rt = tokio::runtime::Builder::new_current_thread()
                                        .enable_all()
                                        .build()
                                        .expect("failed to build sub-workflow runtime");
                                    rt.block_on(async move {
                                        let result: Result<(String, serde_json::Value), String> =
                                            async {
                                                let template = workflow_template_repository()
                                                    .get_workflow_template(&sub_workflow_id)
                                                    .await
                                                    .map_err(|e| e.to_string())?
                                                    .ok_or_else(|| {
                                                        format!(
                                                            "Template {} not found",
                                                            sub_workflow_id
                                                        )
                                                    })?;

                                                let nodes: Vec<WorkflowNode> =
                                                    serde_json::from_str(&template.nodes).map_err(
                                                        |e| format!("节点解析失败: {}", e),
                                                    )?;
                                                let edges: Vec<WorkflowEdge> =
                                                    serde_json::from_str(&template.edges).map_err(
                                                        |e| format!("边解析失败: {}", e),
                                                    )?;

                                                let workflow = engine
                                                    .create_workflow(&template.name, nodes, edges)
                                                    .await
                                                    .map_err(|e| e.to_string())?;
                                                let wid = workflow.id.clone();

                                                let input_value = serde_json::to_value(&input_vars)
                                                    .unwrap_or(serde_json::json!({}));

                                                let mut opts = RunOptions {
                                                    execution_id: Some(child_execution_id),
                                                    input: Some(input_value),
                                                    dry_run,
                                                    parent_execution_id: Some(parent_execution_id),
                                                    model_id,
                                                    provider_id,
                                                    step_timeout: sub_step_timeout,
                                                    parent_cancel_token: Some(cancel_token),
                                                    ..Default::default()
                                                };
                                                if let Some(cb) = progress_cb {
                                                    opts = opts.with_progress_callback(cb);
                                                }

                                                let result = engine
                                                    .run_workflow(&wid, opts)
                                                    .await
                                                    .map_err(|e| e.to_string())?;

                                                let output = result
                                                    .output
                                                    .unwrap_or_else(|| serde_json::json!({}));

                                                Ok((child_eid_for_result, output))
                                            }
                                            .await;
                                        let _ = tx.send(result);
                                    });
                                });
                                Box::pin(async move {
                                    rx.await.map_err(|_| "Sub-workflow task dropped".to_string())?
                                })
                            },
                        );

                    exec_ctx.callbacks = Some(super::execution_state::ExecutionContextCallbacks {
                        trigger_manager: Some(self.trigger_manager.clone()),
                        tool_handlers,
                        tool_fallback,
                        subworkflow: Some(sub_cb),
                        loop_body_dispatch: Some(build_loop_body_dispatch(self.clone())),
                        loop_checkpoint: Some(build_loop_checkpoint_ops()),
                    });
                }

                // 注入 partial_result_tx / interrupt_signal（per-execution 共享）
                {
                    let ptxs = self.loop_partial_txs.lock().await;
                    if let Some(tx) = ptxs.get(&execution_id) {
                        exec_ctx.partial_result_tx = Some(tx.clone());
                    }
                }
                {
                    let sigs = self.loop_interrupt_signals.lock().await;
                    if let Some(sig) = sigs.get(&execution_id) {
                        exec_ctx.interrupt_signal = Some(sig.clone());
                    }
                }

                let dispatcher = self.dispatcher.clone();
                let node_id_owned = node_id.clone();
                let node_type = node_type_name(&node).to_string();
                tracing::info!(
                    workflow_id = %workflow_id,
                    node_id = %node_id,
                    node_type = %node_type,
                    "Dispatching node"
                );
                join_set.spawn(async move {
                    let result = tokio::time::timeout(
                        node_timeout,
                        dispatcher.read().await.dispatch(&node, &exec_ctx),
                    )
                    .await;
                    let elapsed_ms = (Utc::now().timestamp_millis() - started_at) as u64;
                    NodeResult {
                        node_id: node_id_owned,
                        node,
                        input_snapshot,
                        started_at,
                        elapsed_ms,
                        dispatch_result: result,
                    }
                });
            }

            let mut workflow_cancelled = false;
            while let Some(join_result) = join_set.join_next().await {
                let nr = match join_result {
                    Ok(nr) => nr,
                    Err(_) => continue,
                };

                match nr.dispatch_result {
                    Ok(Ok(output)) => {
                        tracing::info!(
                            workflow_id = %workflow_id,
                            node_id = %nr.node_id,
                            status = "completed",
                            duration_ms = nr.elapsed_ms,
                            "Node completed"
                        );

                        breakers
                            .entry(format!("{}:{}", workflow_id, nr.node_id))
                            .or_insert_with(NodeCircuitBreaker::new)
                            .record_success();

                        let out_var = output.output_var.clone();
                        self.update_node_status(
                            workflow_id,
                            &nr.node_id,
                            NodeStatus::Completed,
                            Some(output.output.clone()),
                            None,
                            out_var.as_deref(),
                        )
                        .await
                        .ok();

                        let node_name = Some(nr.node.base_title().to_string());
                        let node_type_str = node_type_name(&nr.node).to_string();
                        let sub_workflow_id = if let WorkflowNode::SubWorkflow(sw) = &nr.node {
                            Some(sw.config.sub_workflow_id.clone())
                        } else {
                            None
                        };
                        self.record_node_execution(
                            &execution_id,
                            NodeExecutionRecord {
                                node_id: nr.node_id.clone(),
                                node_type: node_type_str,
                                node_name,
                                status: "completed".to_string(),
                                input: Some(nr.input_snapshot.clone()),
                                output: Some(output.output.clone()),
                                execution_time_ms: Some(nr.elapsed_ms),
                                error: None,
                                started_at: nr.started_at,
                                completed_at: Some(Utc::now().timestamp_millis()),
                                parent_execution_id: current_parent_execution_id.clone(),
                                sub_workflow_id,
                            },
                        )
                        .await
                        .ok();

                        self.record_audit(
                            &nr.node_id,
                            workflow_id,
                            "completed",
                            &nr.input_snapshot,
                            Some(&output.output),
                            None,
                            nr.elapsed_ms,
                        );

                        if let Some(ref cb) = progress_cb {
                            let completed = {
                                let workflows = self.workflows.read().await;
                                workflows
                                    .get(workflow_id)
                                    .map(|w| {
                                        w.node_states
                                            .values()
                                            .filter(|s| {
                                                matches!(
                                                    s.status,
                                                    NodeStatus::Completed
                                                        | NodeStatus::Failed
                                                        | NodeStatus::Skipped
                                                )
                                            })
                                            .count()
                                    })
                                    .unwrap_or(0)
                            };
                            cb(StepProgressEvent {
                                node_id: nr.node_id.clone(),
                                status: "completed".to_string(),
                                total_nodes,
                                completed_nodes: completed,
                                execution_id: Some(execution_id.clone()),
                            })
                            .await;
                        }

                        if matches!(nr.node, WorkflowNode::Condition(_)) {
                            let mut workflows = self.workflows.write().await;
                            if let Some(wf) = workflows.get_mut(workflow_id) {
                                skip_disabled_branch_nodes(wf, &wf.edges.clone(), &nr.node_id);
                            }
                        }
                    },
                    Ok(Err(err)) => {
                        let err_msg = err.to_string();
                        tracing::warn!(
                            workflow_id = %workflow_id,
                            node_id = %nr.node_id,
                            error = %err_msg,
                            elapsed_ms = nr.elapsed_ms,
                            "Node failed"
                        );

                        breakers
                            .entry(format!("{}:{}", workflow_id, nr.node_id))
                            .or_insert_with(NodeCircuitBreaker::new)
                            .record_failure(current_epoch_ms());

                        let err_msg = err.to_string();
                        let node_retry_cfg = nr.node.base_retry().clone();
                        let (current_attempts, wf_retry_policy) = {
                            let workflows = self.workflows.read().await;
                            let wf = workflows.get(workflow_id);
                            let attempts = wf
                                .and_then(|wf| {
                                    wf.node_states.get(nr.node_id.as_str()).map(|s| s.attempts)
                                })
                                .unwrap_or(0);
                            let rp = wf
                                .and_then(|wf| wf.error_config.as_ref())
                                .and_then(|ec| ec.retry_policy.clone());
                            (attempts, rp)
                        };

                        // 确定生效的重试配置：节点级 > 工作流级 retry_policy 回退
                        let effective_retry = if node_retry_cfg.enabled {
                            Some(node_retry_cfg)
                        } else {
                            wf_retry_policy.map(|rp| RetryConfig {
                                enabled: true,
                                max_retries: rp.max_retries,
                                backoff_type: BackoffType::Exponential,
                                base_delay_ms: rp.base_delay_ms,
                                max_delay_ms: rp.max_delay_ms,
                            })
                        };

                        if let Some(ref retry_cfg) = effective_retry
                            && current_attempts < retry_cfg.max_retries
                        {
                            tracing::info!(
                                workflow_id = %workflow_id,
                                node_id = %nr.node_id,
                                attempt = current_attempts + 1,
                                max_retries = retry_cfg.max_retries,
                                "Retrying node"
                            );

                            let backoff_ms = compute_backoff(
                                retry_cfg.backoff_type.clone(),
                                retry_cfg.base_delay_ms,
                                retry_cfg.max_delay_ms,
                                current_attempts,
                            );
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

                            if cancel_token.is_cancelled() {
                                join_set.abort_all();
                                workflow_cancelled = true;
                                break;
                            }

                            self.update_node_status(
                                workflow_id,
                                &nr.node_id,
                                NodeStatus::Ready,
                                None,
                                Some(err_msg.clone()),
                                None,
                            )
                            .await
                            .ok();
                        } else {
                            self.update_node_status(
                                workflow_id,
                                &nr.node_id,
                                NodeStatus::Failed,
                                None,
                                Some(err_msg.clone()),
                                None,
                            )
                            .await
                            .ok();
                        }

                        self.record_node_execution(
                            &execution_id,
                            NodeExecutionRecord {
                                node_id: nr.node_id.clone(),
                                node_type: node_type_name(&nr.node).to_string(),
                                node_name: Some(nr.node.base_title().to_string()),
                                status: "failed".to_string(),
                                input: Some(nr.input_snapshot.clone()),
                                output: None,
                                execution_time_ms: Some(nr.elapsed_ms),
                                error: Some(err_msg.clone()),
                                started_at: nr.started_at,
                                completed_at: Some(Utc::now().timestamp_millis()),
                                parent_execution_id: current_parent_execution_id.clone(),
                                sub_workflow_id: if let WorkflowNode::SubWorkflow(sw) = &nr.node {
                                    Some(sw.config.sub_workflow_id.clone())
                                } else {
                                    None
                                },
                            },
                        )
                        .await
                        .ok();

                        self.record_audit(
                            &nr.node_id,
                            workflow_id,
                            "failed",
                            &nr.input_snapshot,
                            None,
                            Some(err_msg.clone()),
                            nr.elapsed_ms,
                        );

                        if let Some(ref cb) = progress_cb {
                            let completed = {
                                let workflows = self.workflows.read().await;
                                workflows
                                    .get(workflow_id)
                                    .map(|w| {
                                        w.node_states
                                            .values()
                                            .filter(|s| {
                                                matches!(
                                                    s.status,
                                                    NodeStatus::Completed
                                                        | NodeStatus::Failed
                                                        | NodeStatus::Skipped
                                                )
                                            })
                                            .count()
                                    })
                                    .unwrap_or(0)
                            };
                            cb(StepProgressEvent {
                                node_id: nr.node_id.clone(),
                                status: "failed".to_string(),
                                total_nodes,
                                completed_nodes: completed,
                                execution_id: Some(execution_id.clone()),
                            })
                            .await;
                        }

                        // ── 错误处理基础设施 ──
                        // 构建 ErrorContext 并注入 ExecutionState，
                        // 检查 continue_on_fail、Error 边、error_workflow。
                        {
                            let error_ctx = ErrorContext::new(
                                nr.node_id.clone(),
                                nr.node.base_title().to_string(),
                                "NODE_ERROR".to_string(),
                                err_msg.clone(),
                                workflow_id.to_string(),
                                execution_id.clone(),
                                None,
                            );

                            let mut executions = self.executions.lock().await;
                            if let Some(state) = executions.get_mut(&execution_id) {
                                state.last_error = Some(error_ctx.clone());
                                state.variables.insert(
                                    ErrorContext::variable_name().to_string(),
                                    error_ctx.to_variable(),
                                );
                            }

                            // 读取 continue_on_fail 与 error_config
                            let continue_on_fail = nr.node.base().continue_on_fail;
                            let (ec_opt, ewf_id_opt) = {
                                let workflows = self.workflows.read().await;
                                let wf = workflows.get(workflow_id);
                                (
                                    wf.and_then(|w| w.error_config.clone()),
                                    wf.and_then(|w| w.error_workflow_id.clone()),
                                )
                            };

                            let should_run_error_branch = ec_opt
                                .as_ref()
                                .map(|ec| matches!(ec.on_failure, OnFailureAction::RunErrorBranch))
                                .unwrap_or(false);

                            // 激活 Error 边目标节点
                            if should_run_error_branch {
                                let mut workflows = self.workflows.write().await;
                                if let Some(wf) = workflows.get_mut(workflow_id) {
                                    for edge in &wf.edges {
                                        if edge.source == nr.node_id
                                            && edge.edge_type == EdgeType::Error
                                            && let Some(state) =
                                                wf.node_states.get_mut(&edge.target)
                                            && matches!(state.status, NodeStatus::Pending)
                                        {
                                            state.status = NodeStatus::Ready;
                                            tracing::info!(
                                                workflow_id = %workflow_id,
                                                failed_node = %nr.node_id,
                                                error_target = %edge.target,
                                                "Activated Error edge target"
                                            );
                                        }
                                    }
                                }
                            }

                            // 如果 continue_on_fail 或 Error 边激活，确保工作流不终止
                            let needs_continuation =
                                continue_on_fail || should_run_error_branch || ewf_id_opt.is_some();

                            if needs_continuation {
                                let mut workflows = self.workflows.write().await;
                                if let Some(wf) = workflows.get_mut(workflow_id)
                                    && wf.status == WorkflowStatus::Failed
                                {
                                    let has_ready = wf.node_states.values().any(|s| {
                                        matches!(
                                            s.status,
                                            NodeStatus::Ready
                                                | NodeStatus::Pending
                                                | NodeStatus::Running
                                        )
                                    });
                                    if has_ready {
                                        wf.status = WorkflowStatus::Running;
                                        wf.completed_at = None;
                                        tracing::info!(
                                            workflow_id = %workflow_id,
                                            "continue_on_fail / ErrorBranch: reverting workflow status to Running"
                                        );
                                    } else {
                                        wf.status = WorkflowStatus::PartiallyCompleted;
                                        wf.completed_at =
                                            Some(Utc::now().timestamp_millis() as u64);
                                    }
                                }
                            }

                            // Error Workflow 触发（记录意图，后续阶段实现异步执行）
                            if let Some(ref ewf_id) = ewf_id_opt {
                                tracing::info!(
                                    workflow_id = %workflow_id,
                                    node_id = %nr.node_id,
                                    error_workflow_id = %ewf_id,
                                    "Error Workflow trigger intent recorded"
                                );
                            }
                        }
                    },
                    Err(_) => {
                        tracing::warn!(
                            workflow_id = %workflow_id,
                            node_id = %nr.node_id,
                            elapsed_ms = nr.elapsed_ms,
                            "Node timed out"
                        );

                        breakers
                            .entry(format!("{}:{}", workflow_id, nr.node_id))
                            .or_insert_with(NodeCircuitBreaker::new)
                            .record_failure(current_epoch_ms());

                        let err_msg = "Node execution timeout".to_string();
                        let retry_cfg = nr.node.base_retry();
                        let current_attempts = {
                            let workflows = self.workflows.read().await;
                            workflows
                                .get(workflow_id)
                                .and_then(|wf| {
                                    wf.node_states.get(nr.node_id.as_str()).map(|s| s.attempts)
                                })
                                .unwrap_or(0)
                        };

                        if retry_cfg.enabled && current_attempts < retry_cfg.max_retries {
                            tracing::info!(
                                workflow_id = %workflow_id,
                                node_id = %nr.node_id,
                                attempt = current_attempts + 1,
                                max_retries = retry_cfg.max_retries,
                                "Retrying node after timeout"
                            );

                            let backoff_ms = compute_backoff(
                                retry_cfg.backoff_type.clone(),
                                retry_cfg.base_delay_ms,
                                retry_cfg.max_delay_ms,
                                current_attempts,
                            );
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

                            if cancel_token.is_cancelled() {
                                join_set.abort_all();
                                workflow_cancelled = true;
                                break;
                            }

                            self.update_node_status(
                                workflow_id,
                                &nr.node_id,
                                NodeStatus::Ready,
                                None,
                                Some(err_msg.clone()),
                                None,
                            )
                            .await
                            .ok();
                        } else {
                            // 降级策略检查：超时的节点如果是并行分支的子节点，按 degrade_strategy 处理
                            let degrade = degrade_map.get(&nr.node_id);
                            match degrade {
                                Some(DegradeStrategy::Skip) => {
                                    self.update_node_status(
                                        workflow_id,
                                        &nr.node_id,
                                        NodeStatus::Skipped,
                                        None,
                                        Some(format!("{err_msg} (degraded: skip)")),
                                        None,
                                    )
                                    .await
                                    .ok();
                                },
                                Some(DegradeStrategy::UseDefault) => {
                                    // UseDefault: 注入空默认值并标记为已完成
                                    {
                                        let mut workflows = self.workflows.write().await;
                                        if let Some(wf) = workflows.get_mut(workflow_id) {
                                            wf.results.insert(
                                                nr.node_id.clone(),
                                                serde_json::json!(null),
                                            );
                                        }
                                    }
                                    self.update_node_status(
                                        workflow_id,
                                        &nr.node_id,
                                        NodeStatus::Completed,
                                        None,
                                        Some(format!("{err_msg} (degraded: useDefault)")),
                                        None,
                                    )
                                    .await
                                    .ok();
                                },
                                // Strict 或无降级配置：标记为 Failed（原行为）
                                Some(DegradeStrategy::Strict) | None => {
                                    self.update_node_status(
                                        workflow_id,
                                        &nr.node_id,
                                        NodeStatus::Failed,
                                        None,
                                        Some(err_msg.clone()),
                                        None,
                                    )
                                    .await
                                    .ok();
                                },
                            }
                        }

                        self.record_node_execution(
                            &execution_id,
                            NodeExecutionRecord {
                                node_id: nr.node_id.clone(),
                                node_type: node_type_name(&nr.node).to_string(),
                                node_name: Some(nr.node.base_title().to_string()),
                                status: "timeout".to_string(),
                                input: Some(nr.input_snapshot.clone()),
                                output: None,
                                execution_time_ms: Some(nr.elapsed_ms),
                                error: Some(err_msg.clone()),
                                started_at: nr.started_at,
                                completed_at: Some(Utc::now().timestamp_millis()),
                                parent_execution_id: current_parent_execution_id.clone(),
                                sub_workflow_id: if let WorkflowNode::SubWorkflow(sw) = &nr.node {
                                    Some(sw.config.sub_workflow_id.clone())
                                } else {
                                    None
                                },
                            },
                        )
                        .await
                        .ok();

                        self.record_audit(
                            &nr.node_id,
                            workflow_id,
                            "timeout",
                            &nr.input_snapshot,
                            None,
                            Some(err_msg),
                            nr.elapsed_ms,
                        );

                        if let Some(ref cb) = progress_cb {
                            let completed = {
                                let workflows = self.workflows.read().await;
                                workflows
                                    .get(workflow_id)
                                    .map(|w| {
                                        w.node_states
                                            .values()
                                            .filter(|s| {
                                                matches!(
                                                    s.status,
                                                    NodeStatus::Completed
                                                        | NodeStatus::Failed
                                                        | NodeStatus::Skipped
                                                )
                                            })
                                            .count()
                                    })
                                    .unwrap_or(0)
                            };
                            cb(StepProgressEvent {
                                node_id: nr.node_id.clone(),
                                status: "timeout".to_string(),
                                total_nodes,
                                completed_nodes: completed,
                                execution_id: Some(execution_id.clone()),
                            })
                            .await;
                        }
                    },
                }

                // Inter-batch early scheduling: after a node finishes, check if
                // new nodes have become ready (their deps satisfied by this completion).
                active_nodes.remove(&nr.node_id);
                {
                    let new_ready = self.get_ready_steps(workflow_id).await.unwrap_or_default();
                    if !new_ready.is_empty() {
                        tracing::info!(
                            workflow_id,
                            finished_node = %nr.node_id,
                            new_ready_count = new_ready.len(),
                            active_count = active_nodes.len(),
                            "inter-batch early scheduling: new nodes became ready"
                        );
                    }
                    for nid in new_ready {
                        if active_nodes.len() >= options.max_concurrent {
                            break;
                        }
                        if active_nodes.contains(&nid) {
                            continue;
                        }
                        active_nodes.insert(nid.clone());

                        let node = {
                            let workflows = self.workflows.read().await;
                            workflows.get(workflow_id).and_then(|wf| {
                                wf.nodes.iter().find(|n| n.base_id() == nid).cloned()
                            })
                        };
                        let Some(node) = node else {
                            continue;
                        };

                        let node_id_owned = nid.clone();
                        let deps_results = {
                            let workflows = self.workflows.read().await;
                            workflows
                                .get(workflow_id)
                                .map(|wf| Self::get_node_dependency_results(wf, &nid))
                                .unwrap_or_default()
                        };
                        let input_snapshot =
                            serde_json::to_value(&deps_results).unwrap_or(serde_json::json!({}));
                        let started_at = Utc::now().timestamp_millis();
                        self.update_node_status(
                            workflow_id,
                            &nid,
                            NodeStatus::Running,
                            None,
                            None,
                            None,
                        )
                        .await
                        .ok();

                        let node_timeout = node
                            .base_timeout()
                            .map(Duration::from_secs)
                            .unwrap_or(options.step_timeout);
                        let mut exec_ctx = ExecutionState::new(
                            format!("node_{}", uuid::Uuid::new_v4()),
                            workflow_id.to_string(),
                            serde_json::json!({}),
                        );
                        let mut merged_vars: HashMap<String, serde_json::Value> = deps_results;
                        {
                            let executions = self.executions.lock().await;
                            if let Some(state) = executions.get(&execution_id) {
                                for (k, v) in &state.variables {
                                    merged_vars.entry(k.clone()).or_insert_with(|| v.clone());
                                }
                                if let Some(obj) = state.input_params.as_object() {
                                    for (k, v) in obj {
                                        merged_vars.entry(k.clone()).or_insert_with(|| v.clone());
                                    }
                                }
                            }
                        }
                        exec_ctx.variables = merged_vars;
                        exec_ctx.tool_registry = self.tool_registry();
                        exec_ctx.cancel_token = Some(cancel_token.clone());
                        exec_ctx.dry_run = options.dry_run;
                        exec_ctx.business_rule_engine = self.business_rule_engine();
                        {
                            let bp = self.breakpoints.lock().await;
                            exec_ctx.breakpoints = bp.clone();
                        }
                        {
                            let compiled = self.compiled_prompts.read().await;
                            exec_ctx.compiled_prompts = compiled.get(workflow_id).cloned();
                        }
                        let dispatcher = Arc::clone(&self.dispatcher);
                        let _cancel_token = options
                            .parent_cancel_token
                            .clone()
                            .unwrap_or_else(|| cancel_token.clone());

                        join_set.spawn(async move {
                            let result = tokio::time::timeout(
                                node_timeout,
                                dispatcher.read().await.dispatch(&node, &exec_ctx),
                            )
                            .await;
                            let elapsed_ms = (Utc::now().timestamp_millis() - started_at) as u64;
                            NodeResult {
                                node_id: node_id_owned,
                                node,
                                input_snapshot,
                                started_at,
                                elapsed_ms,
                                dispatch_result: result,
                            }
                        });
                    }
                }

                if cancel_token.is_cancelled() {
                    join_set.abort_all();
                    workflow_cancelled = true;
                    break;
                }
            }

            if workflow_cancelled {
                self.finalize_cancelled_workflow(workflow_id).await;
                self.cancel(&execution_id).await.ok();
                break;
            }

            // 4. 检查终端状态
            if cancel_token.is_cancelled() {
                self.finalize_cancelled_workflow(workflow_id).await;
                self.cancel(&execution_id).await.ok();
                break;
            }

            let status = {
                let workflows = self.workflows.read().await;
                workflows.get(workflow_id).map(|wf| wf.status).unwrap_or(WorkflowStatus::Failed)
            };
            match status {
                WorkflowStatus::Completed
                | WorkflowStatus::PartiallyCompleted
                | WorkflowStatus::Failed
                | WorkflowStatus::Cancelled => break,
                _ => {},
            }
        }

        {
            let mut tokens = self.cancel_tokens.lock().await;
            tokens.remove(workflow_id);
        }

        let mut result = {
            let workflows = self.workflows.read().await;
            workflows.get(workflow_id).cloned()
        };

        if let Some(ref mut wf) = result {
            let end_output = extract_end_output(&wf.nodes, &wf.results);
            wf.output =
                build_workflow_output(&wf.results, end_output, options.output_schema.as_ref());

            // 若配置了 output_schema，校验输出并记录警告
            if let Some(ref schema) = options.output_schema
                && let Some(ref output) = wf.output
                && let Err(errors) = validate_input(output, schema)
            {
                tracing::warn!(
                    workflow_id = %workflow_id,
                    "Output schema validation failed: {:?}",
                    errors
                );
            }

            let persist_output = wf.output.clone().unwrap_or_else(|| {
                serde_json::to_value(&wf.results).unwrap_or(serde_json::json!(null))
            });
            let total_time_ms =
                wf.completed_at.map(|end| end.saturating_sub(wf.created_at) * 1000).unwrap_or(0);
            let final_exec_status = match wf.status {
                WorkflowStatus::Completed => ExecutionStatus::Completed,
                WorkflowStatus::PartiallyCompleted => ExecutionStatus::PartiallyCompleted,
                WorkflowStatus::Failed => ExecutionStatus::Failed,
                WorkflowStatus::Cancelled => ExecutionStatus::Cancelled,
                _ => ExecutionStatus::Completed,
            };
            self.complete_execution(
                &execution_id,
                &persist_output,
                total_time_ms,
                final_exec_status,
            )
            .await
            .ok();

            // 写回共享 HashMap，确保 workflow_get_status 可读到 output
            if wf.output.is_some() {
                let mut workflows = self.workflows.write().await;
                if let Some(shared_wf) = workflows.get_mut(workflow_id) {
                    shared_wf.output = wf.output.clone();
                }
            }
        }

        // Write back breaker state for cross-run persistence
        {
            let mut shared = self.node_breakers.lock().await;
            for (k, v) in breakers {
                shared.insert(k, v);
            }
        }

        // 仅在终端状态下移除工作流；Running/Paused 状态保留以便查询，
        // 权衡：保留非终端工作流会占用内存，但移除后会导致后续查询失败
        {
            let mut workflows = self.workflows.write().await;
            let should_remove = workflows.get(workflow_id).is_none_or(|wf| {
                matches!(
                    wf.status,
                    WorkflowStatus::Completed
                        | WorkflowStatus::Failed
                        | WorkflowStatus::Cancelled
                        | WorkflowStatus::PartiallyCompleted
                )
            });
            if should_remove {
                tracing::info!(
                    workflow_id = %workflow_id,
                    status = ?workflows.get(workflow_id).map(|w| &w.status),
                    "DAG removed — workflow in terminal state"
                );
                workflows.remove(workflow_id);
                {
                    let mut compiled = self.compiled_prompts.write().await;
                    compiled.remove(workflow_id);
                }
                {
                    let mut rhai = self.compiled_rhai_scripts.write().await;
                    rhai.remove(workflow_id);
                }
            }
        }

        Ok(result.unwrap_or_else(|| Workflow {
            id: workflow_id.to_string(),
            name: String::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            status: WorkflowStatus::Failed,
            created_at: 0,
            completed_at: None,
            results: HashMap::new(),
            node_states: HashMap::new(),
            output: None,
            error_config: None,
            error_workflow_id: None,
        }))
    }

    async fn finalize_cancelled_workflow(&self, workflow_id: &str) {
        let mut workflows = self.workflows.write().await;
        if let Some(wf) = workflows.get_mut(workflow_id) {
            for state in wf.node_states.values_mut() {
                if matches!(
                    state.status,
                    NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Running
                ) {
                    state.status = NodeStatus::Skipped;
                }
            }
            wf.status = WorkflowStatus::Cancelled;
            wf.completed_at = Some(current_timestamp());
        }
    }

    // ── 生命周期管理 ──

    pub async fn start_workflow(
        &self,
        workflow_id: &str,
        input: serde_json::Value,
        preset_execution_id: Option<String>,
    ) -> Result<String, WorkEngineError> {
        let execution_id = preset_execution_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut state =
            ExecutionState::new(execution_id.clone(), workflow_id.to_string(), input.clone());
        state.business_rule_engine = self.business_rule_engine();
        state.tool_registry = self.tool_registry();
        let input_params = serde_json::to_string(&input).ok();
        workflow_execution_repository()
            .create_workflow_execution(&execution_id, workflow_id, input_params.as_deref())
            .await
            .map_err(WorkEngineError::Db)?;
        // 为本次执行预创建 partial_result 广播器（容量 256，足够 Loop
        // 万级迭代的扇出，前端订阅者可拿到完整历史）。
        let (partial_tx, _) = tokio::sync::broadcast::channel(256);
        // interrupt 信号：每次执行唯一一份 Notify，LoopExecutor 等待此信号
        // 来挂起，外部 API 调用 notify_waiters() 唤醒。
        let interrupt_signal = std::sync::Arc::new(tokio::sync::Notify::new());
        state.partial_result_tx = Some(partial_tx.clone());
        state.interrupt_signal = Some(interrupt_signal.clone());
        self.loop_partial_txs.lock().await.insert(execution_id.clone(), partial_tx);
        self.loop_interrupt_signals.lock().await.insert(execution_id.clone(), interrupt_signal);
        self.executions.lock().await.insert(execution_id.clone(), state);
        Ok(execution_id)
    }

    pub async fn pause(&self, execution_id: &str) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.status = ExecutionStatus::Paused;
            state.updated_at = Utc::now().timestamp_millis();
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }

    pub async fn resume(&self, execution_id: &str) -> Result<(), WorkEngineError> {
        let signal = {
            let mut executions = self.executions.lock().await;
            if let Some(state) = executions.get_mut(execution_id) {
                if state.status == ExecutionStatus::Paused {
                    state.status = ExecutionStatus::Running;
                    state.updated_at = Utc::now().timestamp_millis();
                }
                state.pause_signal.clone()
            } else {
                return Err(WorkEngineError::NotFound(execution_id.to_string()));
            }
        };
        if let Some(sig) = signal {
            sig.notify_waiters();
        }
        Ok(())
    }

    pub async fn cancel(&self, execution_id: &str) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.status = ExecutionStatus::Cancelled;
            state.updated_at = Utc::now().timestamp_millis();
            let workflow_id = state.workflow_id.clone();
            drop(executions);
            {
                let tokens = self.cancel_tokens.lock().await;
                if let Some(token) = tokens.get(&workflow_id) {
                    token.cancel();
                }
            }
            // 取消时清理 Loop 检查点（避免脏数据遗留）
            if let Err(e) = loop_checkpoint_repository()
                .delete_loop_checkpoints_for_execution(execution_id)
                .await
            {
                tracing::warn!("[Loop] 取消时清理检查点失败: {e} (execution_id={execution_id})");
            }
            // 唤醒可能正在等待 interrupt 的 LoopExecutor
            if let Some(sig) = self.loop_interrupt_signals.lock().await.remove(execution_id) {
                sig.notify_waiters();
            }
            self.loop_partial_txs.lock().await.remove(execution_id);
            workflow_execution_repository()
                .update_workflow_execution_status(execution_id, "cancelled", None, None, None)
                .await
                .map_err(WorkEngineError::Db)?;
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }

    /// 订阅某次执行的 partial_result 流式事件。
    /// 每次 LoopExecutor 完成一轮迭代都会 broadcast 一个 PartialResultEvent。
    /// 注意：execution_id 对应的执行必须已 start_workflow，否则返回 None。
    pub async fn subscribe_partial_results(
        &self,
        execution_id: &str,
    ) -> Option<tokio::sync::broadcast::Receiver<super::execution_state::PartialResultEvent>> {
        self.loop_partial_txs.lock().await.get(execution_id).map(|tx| tx.subscribe())
    }

    /// 恢复因 interrupt 挂起的 Loop 节点。
    ///
    /// - `decision.approved` = true：继续下一次迭代，Loop 不会重新跑已完成的轮次
    ///   （从 checkpoint.cursor 继续）。
    /// - `decision.approved` = false：取消整个 execution（复用 cancel 路径）。
    /// - `decision.modified_iteratee`：可选地修改当前迭代的 iteratee 值
    ///   （写入下一个 body_step 的输入 context）。
    pub async fn resume_loop_iteration(
        &self,
        execution_id: &str,
        node_id: &str,
        decision: LoopResumeDecision,
    ) -> Result<(), WorkEngineError> {
        if !decision.approved {
            // 拒绝：取消整个 execution
            return self.cancel(execution_id).await;
        }
        let signal = {
            let sigs = self.loop_interrupt_signals.lock().await;
            sigs.get(execution_id).cloned()
        };
        let Some(sig) = signal else {
            return Err(WorkEngineError::NotFound(format!("execution_id={execution_id}")));
        };
        // 写入可选的 iteratee 修改（如果 Loop 关心）
        if let Some(ref new_item) = decision.modified_iteratee {
            let mut executions = self.executions.lock().await;
            if let Some(state) = executions.get_mut(execution_id)
                && let Some(ref iteratee_var) = decision.iteratee_var
            {
                state.variables.insert(iteratee_var.clone(), new_item.clone());
            }
        }
        // 唤醒 LoopExecutor
        sig.notify_waiters();
        // 恢复执行状态
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id)
            && state.status == ExecutionStatus::Paused
        {
            state.status = ExecutionStatus::Running;
            state.updated_at = Utc::now().timestamp_millis();
        }
        drop(executions);
        // 同时通知 pause_signal（处理 engine 主循环里同样的 is_paused 路径）
        let _ = node_id; // 当前实现下 node_id 仅用于日志/审计，行为靠 execution_id
        let _ = self.resume(execution_id).await;
        Ok(())
    }

    /// 查询某次执行当前的 Loop 检查点（用于前端高亮 pending approval 节点）。
    pub async fn load_loop_checkpoint(
        &self,
        execution_id: &str,
        node_id: &str,
    ) -> Result<Option<axagent_harness::workflow_types::LoopCheckpoint>, String> {
        loop_checkpoint_repository().load_loop_checkpoint(execution_id, node_id).await
    }

    pub async fn get_status(&self, execution_id: &str) -> Result<ExecutionState, WorkEngineError> {
        let executions = self.executions.lock().await;
        executions
            .get(execution_id)
            .cloned()
            .ok_or_else(|| WorkEngineError::NotFound(execution_id.to_string()))
    }

    pub async fn list_executions(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<WorkflowExecutionData>, WorkEngineError> {
        workflow_execution_repository()
            .list_workflow_executions(workflow_id)
            .await
            .map_err(WorkEngineError::Db)
    }

    pub async fn record_node_execution(
        &self,
        execution_id: &str,
        record: NodeExecutionRecord,
    ) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.add_node_record(record);
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }

    /// 记录审计日志（若已注入 AuditRecorder）
    #[allow(clippy::too_many_arguments)]
    fn record_audit(
        &self,
        node_id: &str,
        workflow_id: &str,
        status: &str,
        input_val: &serde_json::Value,
        output_val: Option<&serde_json::Value>,
        error: Option<String>,
        duration_ms: u64,
    ) {
        use std::hash::{Hash, Hasher};
        let input_str = serde_json::to_string(input_val).unwrap_or_default();
        let output_str = output_val.and_then(|o| serde_json::to_string(o).ok()).unwrap_or_default();

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        input_str.hash(&mut hasher);
        let input_hash = hasher.finish().to_string();

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        output_str.hash(&mut hasher);
        let output_hash = hasher.finish().to_string();

        let recorder_guard = match self.audit_recorder.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!("[Audit] Mutex poisoned, clearing and recovering");
                poisoned.into_inner()
            },
        };
        if let Some(ref recorder) = *recorder_guard {
            recorder.record(axagent_harness::AuditEntry {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: current_timestamp(),
                execution_type: "node".to_string(),
                session_id: None,
                tool_name: None,
                node_id: Some(node_id.to_string()),
                workflow_id: Some(workflow_id.to_string()),
                input_hash,
                output_hash,
                duration_ms,
                status: status.to_string(),
                error,
            });
        }
    }

    pub async fn complete_execution(
        &self,
        execution_id: &str,
        output: &serde_json::Value,
        total_time_ms: u64,
        final_status: ExecutionStatus,
    ) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.status = final_status.clone();
            state.total_time_ms = total_time_ms;
            state.updated_at = Utc::now().timestamp_millis();
            let node_executions = serde_json::to_string(&state.node_records).ok();
            let output_result = serde_json::to_string(output).ok();
            drop(executions);
            let db_status = match final_status {
                ExecutionStatus::Completed => "completed",
                ExecutionStatus::Failed => "failed",
                ExecutionStatus::Cancelled => "cancelled",
                ExecutionStatus::PartiallyCompleted => "partially_completed",
                _ => "completed",
            };
            workflow_execution_repository()
                .update_workflow_execution_status(
                    execution_id,
                    db_status,
                    output_result.as_deref(),
                    node_executions.as_deref(),
                    Some(total_time_ms as i32),
                )
                .await
                .map_err(WorkEngineError::Db)?;
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }
}

// ── 错误类型 ──

#[derive(Debug)]
pub enum WorkEngineError {
    NotFound(String),
    Db(String),
    TimeoutMs(u64),
    Cancelled,
    ToolError { name: String, message: String },
    Execution(String),
}

impl std::fmt::Display for WorkEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "Execution record not found: {id}"),
            Self::Db(e) => write!(f, "数据库错误: {e}"),
            Self::TimeoutMs(ms) => write!(f, "执行超时: {ms}ms"),
            Self::Cancelled => write!(f, "执行已取消"),
            Self::ToolError { name, message } => write!(f, "工具执行错误 [{name}]: {message}"),
            Self::Execution(e) => write!(f, "执行错误: {e}"),
        }
    }
}

impl std::error::Error for WorkEngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// Loop interrupt 恢复决策。
///
/// 由 `WorkEngine::resume_loop_iteration` 接收，传递给 LoopExecutor 决定下一步：
/// - `approved = true` 继续迭代；`approved = false` 取消 execution。
/// - `modified_iteratee` 可选地把当前迭代的 iteratee 变量改成新值（仅在 Loop
///   节点被 interrupt 之后、人为修订了 item 的场景使用）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct LoopResumeDecision {
    pub approved: bool,
    /// 可选：把 iteratee 变量重写为指定值。
    pub modified_iteratee: Option<serde_json::Value>,
    /// 可选：iteratee 在 context.variables 中的 key。若为 None 则不写入。
    pub iteratee_var: Option<String>,
}

// ── Loop 内部驱动回调工厂 ──

/// 构造 LoopExecutor 用的 `loop_body_dispatch` 回调。
///
/// 闭包捕获 `WorkEngine` 的克隆（WorkEngine 自身 #[derive(Clone)]，所有内部
/// 状态都是 Arc<...>，clone 是廉价的），按 (body_step_node_id, ctx) 在
/// dispatcher 中查找对应 WorkflowNode 并 dispatch。`ctx` 已经由 LoopExecutor
/// 拷贝（包含 iteratee_var 注入、当前 iteration 的 variables 快照）。
pub fn build_loop_body_dispatch(engine: WorkEngine) -> super::execution_state::LoopBodyDispatchFn {
    Arc::new(move |step_id: String, ctx: super::execution_state::ExecutionState| {
        let engine = engine.clone();
        Box::pin(async move {
            // 从 ctx.workflow_id 找到对应 workflow，从 nodes 里找出 step_id 对应节点。
            let workflow_id = ctx.workflow_id.clone();
            let workflows = engine.workflows.read().await;
            let node = workflows
                .get(&workflow_id)
                .and_then(|wf| wf.nodes.iter().find(|n| n.base_id() == step_id).cloned());
            drop(workflows);
            let Some(node) = node else {
                return Err(super::node_executor_trait::NodeError::exec_failed(
                    super::node_executor_trait::error_code::NODE_NOT_FOUND,
                    format!("Loop body step '{step_id}' not found in workflow '{workflow_id}'"),
                ));
            };
            engine.dispatcher.read().await.dispatch(&node, &ctx).await
        })
    })
}

/// 构造 LoopExecutor 用的 `loop_checkpoint` 回调（save/load/delete）。
pub fn build_loop_checkpoint_ops() -> super::execution_state::LoopCheckpointOps {
    use super::execution_state::LoopCheckpointOps;
    LoopCheckpointOps {
        save: Arc::new(|cp: axagent_harness::workflow_types::LoopCheckpoint| {
            Box::pin(async move {
                loop_checkpoint_repository()
                    .save_loop_checkpoint(&cp)
                    .await
                    .map_err(|e| format!("save_loop_checkpoint failed: {e}"))
            })
        }),
        load: Arc::new(|eid: String, nid: String| {
            Box::pin(async move {
                loop_checkpoint_repository()
                    .load_loop_checkpoint(&eid, &nid)
                    .await
                    .map_err(|e| format!("load_loop_checkpoint failed: {e}"))
            })
        }),
        delete: Arc::new(|eid: String, nid: String| {
            Box::pin(async move {
                loop_checkpoint_repository()
                    .delete_loop_checkpoint(&eid, &nid)
                    .await
                    .map_err(|e| format!("delete_loop_checkpoint failed: {e}"))
            })
        }),
    }
}

// ── Condition 节点分支跳过辅助 ──

// Condition 节点完成后，将不匹配分支上的所有下游节点标记为 Skipped。

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::super::prompt_template::{ConstraintBlocks, DomainConstraintsFn};
    use super::WorkEngine;
    use axagent_harness::registry::ProviderRegistry;
    use std::sync::Arc;

    /// 最小的 ProviderRegistry 实现 —— `WorkEngine::new` 构造时需要传入，
    /// 但本测试不消费任何 provider 能力（只测桥接 setter），所以 `get` 返回 `None` 即可。
    struct EmptyProviderRegistry;

    impl ProviderRegistry for EmptyProviderRegistry {
        fn get(&self, _provider_type: &str) -> Option<Arc<dyn axagent_harness::ProviderAdapter>> {
            None
        }
    }

    /// 构造一个仅用于桥接测试的 WorkEngine。
    /// - master_key `[0u8; 32]`：占位密钥，桥接测试不涉及解密
    /// - `EmptyProviderRegistry`：空实现，桥接测试不查 provider
    fn make_test_engine() -> WorkEngine {
        WorkEngine::new([0u8; 32], Arc::new(EmptyProviderRegistry))
    }

    #[tokio::test]
    async fn set_domain_constraints_stores_callback() {
        let engine = make_test_engine();

        // 初始：未注册 → getter 返回 None
        assert!(engine.domain_constraints().is_none());

        // 注册
        let cb: DomainConstraintsFn = Arc::new(|_role| ConstraintBlocks::default());
        engine.set_domain_constraints(cb.clone()).await;

        // 已注册 → getter 返回 Some
        assert!(engine.domain_constraints().is_some());
    }

    #[tokio::test]
    async fn set_domain_constraints_can_be_overwritten() {
        let engine = make_test_engine();

        let cb1: DomainConstraintsFn = Arc::new(|_| ConstraintBlocks::default());
        let cb2: DomainConstraintsFn = Arc::new(|_| ConstraintBlocks::default());

        engine.set_domain_constraints(cb1).await;
        // 第二次注册覆盖第一次（标准 setter 语义）
        engine.set_domain_constraints(cb2.clone()).await;

        let stored = engine.domain_constraints().expect("应已注册");
        // 通过指针等价性确认最新注册的是 cb2（cb1 已被覆盖）
        assert!(Arc::ptr_eq(&stored, &cb2));
    }
}
