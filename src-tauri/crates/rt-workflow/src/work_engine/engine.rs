//! 统一工作流引擎 —— DAG 管理 + 并发执行 + DB 持久化。
//!
//! 节点类型统一为 axagent_core::workflow_types::WorkflowNode（28 种），
//! 执行通过 NodeDispatcher 分发到对应执行器。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use axagent_core::workflow_types::{
    BackoffType, CompensationStrategy, EdgeType, JsonSchema, Variable, WorkflowEdge, WorkflowNode,
};

use axagent_harness::RhaiEngineAdapter;
use rhai::AST;

/// Convert Rhai dynamic map to JSON value
fn rhai_map_to_json(map: rhai::Map) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (k, v) in map {
        let val: serde_json::Value = if v.is_int() {
            v.as_int()
                .map(|n| serde_json::Value::Number(n.into()))
                .unwrap_or(serde_json::Value::Null)
        } else if v.is_string() {
            v.try_cast::<String>()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null)
        } else if v.is_float() {
            match v.as_float() {
                Ok(f) => serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),
                Err(_) => serde_json::Value::Null,
            }
        } else if v.is_bool() {
            v.as_bool()
                .map(serde_json::Value::Bool)
                .unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        };
        obj.insert(k.to_string(), val);
    }
    serde_json::Value::Object(obj)
}

/// Rhai 脚本缓存类型：workflow_id -> (tool_name -> compiled AST)
type RhaiScriptCache = HashMap<String, Arc<AST>>;

/// Rhai 脚本工具回调类型
type LocalRhaiToolFn = Arc<
    dyn Fn(
            String,
            serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn futures::Future<Output = Result<serde_json::Value, String>> + Send>,
        > + Send
        + Sync,
>;

use crate::workflow_engine::{
    NodeRuntimeState, NodeStatus, Workflow, WorkflowError, WorkflowStatus, current_epoch_ms,
    current_timestamp,
};

use super::dispatcher::NodeDispatcher;
use super::execution_state::{ExecutionState, ExecutionStatus, NodeExecutionRecord};
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
        Self {
            max_concurrent: 3,
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

// ── 内部追踪类型 ──

/// 断路器状态（按节点追踪）
#[derive(Debug, Clone)]
struct NodeCircuitBreaker {
    failure_count: u32,
    failure_threshold: u32,
    reset_timeout_ms: u64,
    opened_at: Option<u64>,
}

impl NodeCircuitBreaker {
    fn new() -> Self {
        Self {
            failure_count: 0,
            failure_threshold: 3,
            reset_timeout_ms: 60_000,
            opened_at: None,
        }
    }

    fn is_open(&self, now_ms: u64) -> bool {
        if let Some(opened_at) = self.opened_at {
            now_ms < opened_at + self.reset_timeout_ms
        } else {
            false
        }
    }

    fn record_success(&mut self) {
        self.failure_count = 0;
        self.opened_at = None;
    }

    fn record_failure(&mut self, now_ms: u64) {
        self.failure_count += 1;
        if self.failure_count >= self.failure_threshold {
            self.opened_at = Some(now_ms);
        }
    }
}

fn compute_backoff(
    backoff_type: BackoffType,
    base_delay_ms: u64,
    max_delay_ms: u64,
    attempt: u32,
) -> u64 {
    let delay = match backoff_type {
        BackoffType::Fixed => base_delay_ms,
        BackoffType::Linear => base_delay_ms.saturating_mul(attempt as u64),
        BackoffType::Exponential => {
            let exp = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
            base_delay_ms.saturating_mul(exp)
        },
    };
    delay.min(max_delay_ms)
}

struct NodeResult {
    node_id: String,
    node: WorkflowNode,
    input_snapshot: serde_json::Value,
    started_at: i64,
    elapsed_ms: u64,
    dispatch_result: Result<Result<NodeOutput, NodeError>, tokio::time::error::Elapsed>,
}

// ── WorkEngine ──

#[derive(Clone)]

pub struct WorkEngine {
    db: Arc<DatabaseConnection>,
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
        Arc<std::sync::Mutex<Option<Arc<axagent_harness::business_rules::BusinessRuleEngine>>>>,
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
    /// 审计记录器（可选，None = 不记录审计日志）
    pub audit_recorder: Arc<std::sync::Mutex<Option<Arc<dyn axagent_harness::AuditRecorder>>>>,
    /// 工具注册表（可选，设置后 tool_executor 优先通过 ToolRegistry.execute_tool() 执行工具）
    tool_registry: Arc<std::sync::Mutex<Option<Arc<dyn axagent_harness::ToolRegistry>>>>,
}

const _: fn() = || {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<WorkEngine>();
    assert_sync::<WorkEngine>();
};

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
            executions
                .get(execution_id)
                .and_then(|s| s.pause_signal.clone())
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
            executions
                .get(execution_id)
                .and_then(|s| s.pause_signal.clone())
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
        tool_defs: &[axagent_core::workflow_types::RhaiToolDef],
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
            let engine = Engine::new();
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
            self.compiled_rhai_scripts
                .write()
                .await
                .insert(workflow_id.to_string(), cache);
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
        self.dispatcher.read().await.dispatch(node, context).await
    }

    pub async fn registered_executor_types(&self) -> Vec<&'static str> {
        self.dispatcher.read().await.registered_types()
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
        *self
            .domain_constraints
            .lock()
            .expect("domain_constraints mutex poisoned") = Some(f);
    }

    /// 取出当前注册的领域约束（用于在执行 agent 节点时转发给 `AgentExecutor`）。
    ///
    /// 内部 clone 出 Arc，避免锁长时间持有。仅暴露给 crate 内部消费
    /// （当前仅 `agent_executor.rs` 会调用，stock-analysis 等领域 PR
    /// 不应直接依赖此 getter）。
    ///
    /// `#[allow(dead_code)]`：本 PR 仅加桥接，不消费 getter。
    /// stock-analysis 后续 PR 在 `AgentExecutor::execute` 中调用此方法
    /// 取约束并注入 4a/4f 段时会用到，到时移除此标记。
    #[allow(dead_code)]
    pub(crate) fn domain_constraints(&self) -> Option<DomainConstraintsFn> {
        self.domain_constraints
            .lock()
            .expect("domain_constraints mutex poisoned")
            .clone()
    }

    /// 注册业务规则引擎。
    ///
    /// 注入后，在执行 agent / tool / httpRequest 等节点时，
    /// 会在 dispatch 之前自动进行规则评估。
    ///
    /// 多次调用：后者覆盖前者（标准 setter 语义）。
    pub async fn set_business_rule_engine(
        &self,
        engine: Arc<axagent_harness::business_rules::BusinessRuleEngine>,
    ) {
        *self
            .business_rule_engine
            .lock()
            .expect("business_rule_engine mutex poisoned") = Some(engine);
    }

    /// 取出当前注册的业务规则引擎（用于在执行节点时注入到 ExecutionState）。
    fn business_rule_engine(
        &self,
    ) -> Option<Arc<axagent_harness::business_rules::BusinessRuleEngine>> {
        self.business_rule_engine
            .lock()
            .expect("business_rule_engine mutex poisoned")
            .clone()
    }

    /// 注册工具注册表（可选，设置后 tool_executor 优先走 ToolRegistry 中心化路径）
    pub async fn set_tool_registry(&self, registry: Arc<dyn axagent_harness::ToolRegistry>) {
        *self
            .tool_registry
            .lock()
            .expect("tool_registry mutex poisoned") = Some(registry);
    }

    /// 取出当前注册的工具注册表（用于在执行节点时注入到 ExecutionState）
    fn tool_registry(&self) -> Option<Arc<dyn axagent_harness::ToolRegistry>> {
        self.tool_registry
            .lock()
            .expect("tool_registry mutex poisoned")
            .clone()
    }
}

impl WorkEngine {
    pub fn new(
        db: Arc<DatabaseConnection>,
        master_key: [u8; 32],
        provider_registry: Arc<dyn axagent_harness::registry::ProviderRegistry>,
    ) -> Self {
        let agent_provider_cache = Arc::new(tokio::sync::Mutex::new(None));
        let agent_profile_cache = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        let mut dispatcher = NodeDispatcher::new();

        // 统一走 HasProviderRegistry trait，避免 5 个 executor 各自实现 with_provider_registry。
        use axagent_harness::HasProviderRegistry;

        let mut llm_exec = LlmExecutor::new(db.clone(), master_key);
        // 唯一构造路径：创建 Arc<AgentExecutor>，dispatcher 与 WorkEngine.agent_executor
        // 共享同一个实例。运行期不再 register(E) 重新注册，避免丢失 provider_registry。
        let mut agent_exec = AgentExecutor::with_shared_caches(
            db.clone(),
            master_key,
            agent_provider_cache.clone(),
            agent_profile_cache.clone(),
        );
        let mut cond_exec = ConditionExecutor::new(db.clone(), master_key);
        let mut classifier_exec = LlmClassifierExecutor::new(db.clone(), master_key);

        llm_exec.set_provider_registry(provider_registry.clone());
        agent_exec.set_provider_registry(provider_registry.clone());
        cond_exec.set_provider_registry(provider_registry.clone());
        classifier_exec.set_provider_registry(provider_registry.clone());

        let agent_exec = Arc::new(agent_exec);
        dispatcher.register(llm_exec);
        dispatcher.register_arc(agent_exec.clone() as Arc<dyn NodeExecutorTrait>);
        dispatcher.register(cond_exec);
        dispatcher.register(classifier_exec);
        Self {
            db,
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
            audit_recorder: Arc::new(std::sync::Mutex::new(None)),
            tool_registry: Arc::new(std::sync::Mutex::new(None)),
        }
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
                adj.entry(edge.source.as_str())
                    .or_default()
                    .push(edge.target.as_str());
                *in_degree.entry(edge.target.as_str()).or_insert(0) += 1;
            }
            let mut queue: Vec<&str> = in_degree
                .iter()
                .filter(|&(_, &deg)| deg == 0)
                .map(|(&id, _)| id)
                .collect();
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

        let node_states: HashMap<String, NodeRuntimeState> = nodes
            .iter()
            .map(|n| (n.base_id().to_string(), NodeRuntimeState::default()))
            .collect();

        // 编译 Agent 节点的 prompt 模板（阶段一）
        let mut compiled_map: HashMap<String, CompiledPrompt> = HashMap::new();
        for node in &nodes {
            if let WorkflowNode::Agent(an) = node {
                compiled_map.insert(an.base.id.clone(), compile_prompt(&an.config.system_prompt));
            }
        }
        self.compiled_prompts
            .write()
            .await
            .insert(workflow_id.clone(), compiled_map);

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
        };

        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id.clone(), workflow.clone());

        Ok(workflow)
    }

    /// 根据 edges 构建邻接表，返回就绪节点（入度为 0 的节点）。
    fn compute_ready_nodes(workflow: &Workflow) -> Vec<String> {
        let done_or_skipped: HashSet<&str> = workflow
            .node_states
            .iter()
            .filter(|(_, s)| matches!(s.status, NodeStatus::Completed | NodeStatus::Skipped))
            .map(|(id, _)| id.as_str())
            .collect();

        // 计算每个未完成节点的"未完成依赖数"
        let mut remaining_deps: HashMap<&str, usize> = HashMap::new();
        for node in &workflow.nodes {
            remaining_deps.entry(node.base_id()).or_insert(0);
        }
        for edge in &workflow.edges {
            // source 未完成 → target 有未满足的依赖
            if !done_or_skipped.contains(edge.source.as_str()) {
                *remaining_deps.entry(edge.target.as_str()).or_insert(0) += 1;
                continue;
            }

            // ConditionTrue/ConditionFalse 边：根据 condition 节点的输出决定是否激活
            if edge.edge_type == EdgeType::ConditionTrue
                || edge.edge_type == EdgeType::ConditionFalse
            {
                let cond_output = workflow.results.get(edge.source.as_str());
                let result = cond_output
                    .and_then(|o| o.get("result"))
                    .and_then(|r| r.as_bool())
                    .unwrap_or(false);
                // source_handle 回退到 edge_type：ConditionTrue → "true", ConditionFalse → "false"
                let branch = edge
                    .source_handle
                    .as_deref()
                    .unwrap_or(match edge.edge_type {
                        EdgeType::ConditionTrue => "true",
                        EdgeType::ConditionFalse => "false",
                        _ => "true",
                    });
                let should_follow = (branch == "true" && result) || (branch == "false" && !result);
                if !should_follow {
                    continue;
                }
            }

            // Switch 边：根据 switch 节点的 matched_label 决定是否激活
            let is_switch_source = workflow
                .nodes
                .iter()
                .any(|n| n.base_id() == edge.source && matches!(n, WorkflowNode::Switch(_)));
            if is_switch_source {
                let switch_output = workflow.results.get(edge.source.as_str());
                let selected_case = switch_output
                    .and_then(|o| o.get("matched_label"))
                    .and_then(|v| v.as_str());
                if let Some(ref handle) = edge.source_handle
                    && selected_case.is_none_or(|case| case != handle.as_str())
                {
                    continue;
                }
            }
        }

        workflow
            .nodes
            .iter()
            .filter(|n| {
                let state = workflow.node_states.get(n.base_id());
                let is_pending = state
                    .is_none_or(|s| matches!(s.status, NodeStatus::Pending | NodeStatus::Ready));
                let deps_met = remaining_deps.get(n.base_id()).copied().unwrap_or(0) == 0;
                is_pending && deps_met && n.base_enabled()
            })
            .map(|n| n.base_id().to_string())
            .collect()
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
        let workflow = workflows
            .get(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;
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
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;

        let state = workflow
            .node_states
            .get_mut(node_id)
            .ok_or(WorkflowError::NodeNotFound)?;

        state.status = status;
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
        if status == NodeStatus::Failed {
            if let Some(node) = workflow.nodes.iter().find(|n| n.base_id() == node_id) {
                if let Some(ref comp) = node.base().compensation {
                    match comp.strategy {
                        CompensationStrategy::SkipWithWarning => {
                            // 删除该节点输出
                            workflow.results.remove(node_id);
                            tracing::info!(
                                "[补偿] 节点 {} (SkipWithWarning): 已移除失败输出",
                                node_id
                            );
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
                                if let Some(dep_state) = workflow.node_states.get_mut(dep_id) {
                                    if matches!(
                                        dep_state.status,
                                        NodeStatus::Pending | NodeStatus::Ready
                                    ) {
                                        dep_state.status = NodeStatus::Skipped;
                                    }
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
        let any_skipped = workflow
            .node_states
            .values()
            .any(|s| s.status == NodeStatus::Skipped);
        let any_failed = workflow
            .node_states
            .values()
            .any(|s| s.status == NodeStatus::Failed);

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
                axagent_core::repo::workflow_execution::update_workflow_execution_status(
                    &self.db,
                    exec_id,
                    "cancelled",
                    None,
                    None,
                    None,
                )
                .await
                .ok();
            }
        }

        let mut workflows = self.workflows.write().await;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;

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
        let wf = workflows
            .get(workflow_id)
            .ok_or(WorkflowError::WorkflowNotFound)?;
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
        let cancel_token = options
            .parent_cancel_token
            .as_ref()
            .map(|t| t.child_token())
            .unwrap_or_default();
        {
            let mut tokens = self.cancel_tokens.lock().await;
            tokens.insert(workflow_id.to_string(), cancel_token.clone());
        }

        // 构建执行输入：优先使用调用方传入的 input，否则用空对象
        let mut input = options
            .input
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));
        // 将 model_id / provider_id 写入上下文，供执行器读取
        if let Some(ref model_id) = options.model_id {
            input[super::executors::WORKFLOW_MODEL_VAR] =
                serde_json::Value::String(model_id.clone());
        }
        if let Some(ref provider_id) = options.provider_id {
            input[super::executors::WORKFLOW_PROVIDER_ID_VAR] =
                serde_json::Value::String(provider_id.clone());
        }

        // 若配置了 input_schema，校验输入参数
        if let Some(ref schema) = options.input_schema
            && let Err(errors) = validate_input(&input, schema)
        {
            return Err(WorkflowError::InputValidationFailed { errors });
        }

        let execution_id = self
            .start_workflow(workflow_id, input, options.execution_id.clone())
            .await
            .map_err(|e| WorkflowError::SerializationError(e.to_string()))?;

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
            executions
                .get(&execution_id)
                .and_then(|s| s.parent_execution_id.clone())
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
                        let cb: ToolCallback =
                            std::sync::Arc::new(move |_tn: String, _args: serde_json::Value| {
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
                                        let _scope = Scope::new();
                                        // Register tool functions
                                        for (name, handler) in &rhai_tools {
                                            let h = handler.clone();
                                            let n = name.clone();
                                            engine.register_fn(
                                                "tool",
                                                move |tool_name: &str, tool_args: rhai::Map| {
                                                    let h = h.clone();
                                                    let json_args = rhai_map_to_json(tool_args);
                                                    // Inline execution via tokio runtime
                                                    let rt = tokio::runtime::Runtime::new()
                                                        .expect("rhai runtime");
                                                    let result = rt.block_on(async {
                                                        h(tool_name.to_string(), json_args).await
                                                    });
                                                    match result {
                                                        Ok(val) => Dynamic::from(
                                                            serde_json::to_string(&val)
                                                                .unwrap_or_default(),
                                                        ),
                                                        Err(e) => Dynamic::from(format!(
                                                            "tool error: {e}"
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
                            });
                        handlers.insert(tool_name.clone(), cb);
                    }
                }
            }
        }

        // 懒编译兜底：若工作流从 DB 加载（非 create_workflow 新建），编译模板
        {
            let compiled = self.compiled_prompts.read().await;
            if !compiled.contains_key(workflow_id) {
                drop(compiled);
                let workflows = self.workflows.read().await;
                if let Some(wf) = workflows.get(workflow_id) {
                    let mut compiled_map: HashMap<String, CompiledPrompt> = HashMap::new();
                    for node in &wf.nodes {
                        if let WorkflowNode::Agent(an) = node {
                            compiled_map.insert(
                                an.base.id.clone(),
                                compile_prompt(&an.config.system_prompt),
                            );
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
            workflows
                .get(workflow_id)
                .map(|w| w.nodes.len())
                .unwrap_or(0)
        };
        let progress_cb = options.progress_callback.clone();
        let mut breakers: HashMap<String, NodeCircuitBreaker> =
            { self.node_breakers.lock().await.clone() };

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
                // 死锁检测：上游 Failed 可能永久阻塞下游 Pending/Ready 节点
                let has_blocked = {
                    let workflows = self.workflows.read().await;
                    workflows
                        .get(workflow_id)
                        .map(|wf| {
                            wf.node_states.values().any(|s| {
                                matches!(s.status, NodeStatus::Pending | NodeStatus::Ready)
                            })
                        })
                        .unwrap_or(false)
                };
                if has_blocked {
                    let mut workflows = self.workflows.write().await;
                    if let Some(wf) = workflows.get_mut(workflow_id) {
                        for state in wf.node_states.values_mut() {
                            if matches!(state.status, NodeStatus::Pending | NodeStatus::Ready) {
                                state.status = NodeStatus::Skipped;
                            }
                        }
                        wf.status = WorkflowStatus::PartiallyCompleted;
                        wf.completed_at = Some(current_timestamp());
                    }
                }
                break;
            };

            let batch: Vec<String> = ready_nodes
                .into_iter()
                .take(options.max_concurrent)
                .collect();

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

                let node_timeout = node
                    .base_timeout()
                    .map(Duration::from_secs)
                    .unwrap_or(options.step_timeout);
                let mut exec_ctx = ExecutionState::new(
                    format!("node_{}", uuid::Uuid::new_v4()),
                    workflow_id.to_string(),
                    serde_json::json!({}),
                );
                exec_ctx.variables = deps_results;
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
                        state.pause_signal.clone().unwrap()
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

                                let (tx, rx) = tokio::sync::oneshot::channel();
                                let rt = tokio::runtime::Handle::current();
                                std::thread::spawn(move || {
                                    let result = rt.block_on(async {
                                        use axagent_core::repo::workflow_template;

                                        let db = &engine.db;
                                        let template = workflow_template::get_workflow_template(
                                            db,
                                            &sub_workflow_id,
                                        )
                                        .await
                                        .map_err(|e| e.to_string())?
                                        .ok_or_else(|| {
                                            format!("Template {} not found", sub_workflow_id)
                                        })?;

                                        let nodes: Vec<WorkflowNode> =
                                            serde_json::from_str(&template.nodes)
                                                .map_err(|e| format!("节点解析失败: {}", e))?;
                                        let edges: Vec<WorkflowEdge> =
                                            serde_json::from_str(&template.edges)
                                                .map_err(|e| format!("边解析失败: {}", e))?;

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

                                        let output =
                                            result.output.unwrap_or_else(|| serde_json::json!({}));

                                        Ok::<(String, serde_json::Value), String>((
                                            child_eid_for_result,
                                            output,
                                        ))
                                    });
                                    let _ = tx.send(result);
                                });
                                Box::pin(async move {
                                    rx.await
                                        .map_err(|_| "Sub-workflow task dropped".to_string())?
                                })
                            },
                        );

                    exec_ctx.callbacks = Some(super::execution_state::ExecutionContextCallbacks {
                        tool_handlers,
                        tool_fallback,
                        subworkflow: Some(sub_cb),
                    });
                }

                let dispatcher = self.dispatcher.clone();
                let node_id_owned = node_id.clone();
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

                        if matches!(nr.node, WorkflowNode::Condition(_)) {
                            let mut workflows = self.workflows.write().await;
                            if let Some(wf) = workflows.get_mut(workflow_id) {
                                skip_disabled_branch_nodes(wf, &wf.edges.clone(), &nr.node_id);
                            }
                        }
                    },
                    Ok(Err(err)) => {
                        breakers
                            .entry(format!("{}:{}", workflow_id, nr.node_id))
                            .or_insert_with(NodeCircuitBreaker::new)
                            .record_failure(current_epoch_ms());

                        let err_msg = err.to_string();
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
                    },
                    Err(_) => {
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
                    },
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
                workflows
                    .get(workflow_id)
                    .map(|wf| wf.status)
                    .unwrap_or(WorkflowStatus::Failed)
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
            if let Some(ref schema) = options.output_schema {
                if let Some(ref output) = wf.output {
                    if let Err(errors) = validate_input(output, schema) {
                        tracing::warn!(
                            workflow_id = %workflow_id,
                            "Output schema validation failed: {:?}",
                            errors
                        );
                    }
                }
            }

            let persist_output = wf.output.clone().unwrap_or_else(|| {
                serde_json::to_value(&wf.results).unwrap_or(serde_json::json!(null))
            });
            let total_time_ms = wf
                .completed_at
                .map(|end| end.saturating_sub(wf.created_at) * 1000)
                .unwrap_or(0);
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
        axagent_core::repo::workflow_execution::create_workflow_execution(
            &self.db,
            &execution_id,
            workflow_id,
            input_params.as_deref(),
        )
        .await
        .map_err(|e| WorkEngineError::Db(e.to_string()))?;
        self.executions
            .lock()
            .await
            .insert(execution_id.clone(), state);
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
            axagent_core::repo::workflow_execution::update_workflow_execution_status(
                &self.db,
                execution_id,
                "cancelled",
                None,
                None,
                None,
            )
            .await
            .map_err(|e| WorkEngineError::Db(e.to_string()))?;
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
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
    ) -> Result<Vec<axagent_core::entity::workflow_executions::Model>, WorkEngineError> {
        axagent_core::repo::workflow_execution::list_workflow_executions(&self.db, workflow_id)
            .await
            .map_err(|e| WorkEngineError::Db(e.to_string()))
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
        let output_str = output_val
            .and_then(|o| serde_json::to_string(o).ok())
            .unwrap_or_default();

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        input_str.hash(&mut hasher);
        let input_hash = hasher.finish().to_string();

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        output_str.hash(&mut hasher);
        let output_hash = hasher.finish().to_string();

        if let Some(ref recorder) = *self.audit_recorder.lock().unwrap() {
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
            axagent_core::repo::workflow_execution::update_workflow_execution_status(
                &self.db,
                execution_id,
                db_status,
                output_result.as_deref(),
                node_executions.as_deref(),
                Some(total_time_ms as i32),
            )
            .await
            .map_err(|e| WorkEngineError::Db(e.to_string()))?;
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
    }
}

// ── 辅助函数（run_workflow 尾部使用）──

/// 扫描所有 EndNode，提取其 output_var 指向的节点输出作为聚合结果。
fn extract_end_output(
    nodes: &[WorkflowNode],
    results: &HashMap<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    let end_nodes: Vec<_> = nodes
        .iter()
        .filter_map(|n| match n {
            WorkflowNode::End(en) => Some(&en.config),
            _ => None,
        })
        .collect();

    if end_nodes.is_empty() {
        return None;
    }

    // 收集所有 EndNode 的输出
    let mut outputs = serde_json::Map::new();
    for cfg in &end_nodes {
        if let Some(ref var) = cfg.output_var
            && let Some(val) = results.get(var)
        {
            outputs.insert(var.clone(), val.clone());
        }
    }

    if outputs.is_empty() {
        None
    } else if outputs.len() == 1 {
        outputs.into_values().next()
    } else {
        Some(serde_json::Value::Object(outputs))
    }
}

/// 按 output_schema 过滤/重组输出。
/// schema 中通过 `"$source": "node_id"` 字段标记值来源节点。
fn build_workflow_output(
    results: &HashMap<String, serde_json::Value>,
    end_output: Option<serde_json::Value>,
    output_schema: Option<&JsonSchema>,
) -> Option<serde_json::Value> {
    match output_schema {
        None => {
            // 无 schema → 优先使用 EndNode 聚合输出，否则返回全部 results
            end_output.or_else(|| Some(serde_json::json!(results)))
        },
        Some(schema) => {
            let filtered = filter_by_schema(results, schema);
            Some(filtered)
        },
    }
}

/// 按 JsonSchema 从 results 中提取/重组字段。
fn filter_by_schema(
    results: &HashMap<String, serde_json::Value>,
    schema: &JsonSchema,
) -> serde_json::Value {
    let props = match &schema.properties {
        Some(p) => p,
        None => return serde_json::json!(results),
    };

    let mut out = serde_json::Map::new();
    for (key, prop) in props {
        // 检查是否有 $source 自定义字段（标记值来源节点）
        let source = prop
            .default
            .as_ref()
            .and_then(|d| d.get("$source"))
            .and_then(|s| s.as_str());

        if let Some(node_id) = source {
            // 从指定节点输出中提取
            if let Some(node_output) = results.get(node_id) {
                out.insert(key.clone(), extract_nested(node_output, key));
            }
        } else if let Some(val) = results.get(key) {
            // 按 key 名直接匹配 node_id
            out.insert(key.clone(), val.clone());
        }
    }

    if out.is_empty() {
        serde_json::json!(results)
    } else {
        serde_json::Value::Object(out)
    }
}

/// 从嵌套 JSON 中提取最内层有意义的值。
fn extract_nested(value: &serde_json::Value, _key: &str) -> serde_json::Value {
    match value {
        serde_json::Value::Object(obj) => {
            // 尝试提取常见的包装字段
            if let Some(inner) = obj
                .get("result")
                .or_else(|| obj.get("output"))
                .or_else(|| obj.get("content"))
            {
                inner.clone()
            } else {
                value.clone()
            }
        },
        _ => value.clone(),
    }
}

/// 用 jsonschema crate 校验 input 是否匹配 schema。
fn validate_input(input: &serde_json::Value, schema: &JsonSchema) -> Result<(), Vec<String>> {
    let schema_json = serde_json::to_value(schema).unwrap_or(serde_json::Value::Null);
    let validator = jsonschema::Validator::new(&schema_json)
        .map_err(|e| vec![format!("Schema compile error: {e}")])?;
    let mut errors: Vec<String> = Vec::new();
    for err in validator.iter_errors(input) {
        errors.push(format!("{}: {}", err.instance_path(), err));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// 扫描工作流节点，收集所有 AgentNode 中引用的工具名。
fn collect_workflow_tool_names(nodes: &[WorkflowNode]) -> Vec<String> {
    let mut names = std::collections::HashSet::new();
    for node in nodes {
        if let WorkflowNode::Agent(an) = node {
            for tool in &an.config.tools {
                names.insert(tool.name.clone());
            }
        }
    }
    names.into_iter().collect()
}

// ── 错误类型 ──

#[derive(Debug)]
pub enum WorkEngineError {
    NotFound(String),
    Db(String),
    Execution(String),
}

impl std::fmt::Display for WorkEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "Execution record not found: {id}"),
            Self::Db(e) => write!(f, "数据库错误: {e}"),
            Self::Execution(e) => write!(f, "执行错误: {e}"),
        }
    }
}

impl std::error::Error for WorkEngineError {}

// ── Condition 节点分支跳过辅助 ──

/// Condition 节点完成后，将不匹配分支上的所有下游节点标记为 Skipped。
fn skip_disabled_branch_nodes(workflow: &mut Workflow, edges: &[WorkflowEdge], cond_node_id: &str) {
    let cond_output = workflow.results.get(cond_node_id);
    let result = cond_output
        .and_then(|o| o.get("result"))
        .and_then(|r| r.as_bool())
        .unwrap_or(false);

    // 确定要跳过的分支：result==true → 跳过 "false" 分支；result==false → 跳过 "true" 分支
    let skip_branch = if result { "false" } else { "true" };

    for edge in edges {
        if edge.source != cond_node_id {
            continue;
        }
        if edge.edge_type != EdgeType::ConditionTrue && edge.edge_type != EdgeType::ConditionFalse {
            continue;
        }
        let actual_branch = edge
            .source_handle
            .as_deref()
            .unwrap_or(match edge.edge_type {
                EdgeType::ConditionTrue => "true",
                EdgeType::ConditionFalse => "false",
                _ => "true",
            });
        if actual_branch == skip_branch {
            mark_subtree_skipped(workflow, edges, &edge.target);
        }
    }
}

/// 递归标记节点及其所有下游节点为 Skipped
fn mark_subtree_skipped(workflow: &mut Workflow, edges: &[WorkflowEdge], node_id: &str) {
    // 如果已经标记过（Completed/Failed/Skipped），不再递归
    if let Some(state) = workflow.node_states.get(node_id)
        && matches!(state.status, NodeStatus::Completed | NodeStatus::Failed | NodeStatus::Skipped)
    {
        return;
    }

    workflow
        .node_states
        .entry(node_id.to_string())
        .or_insert_with(|| NodeRuntimeState {
            status: NodeStatus::Skipped,
            attempts: 0,
            error: None,
            started_at: None,
            completed_at: Some(current_timestamp() as i64),
        })
        .status = NodeStatus::Skipped;

    // 递归跳过所有下游节点
    for edge in edges {
        if edge.source == node_id {
            mark_subtree_skipped(workflow, edges, &edge.target);
        }
    }
}

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
    /// - `DatabaseConnection::default()`：sea-orm 的零值连接，不发起实际查询
    /// - master_key `[0u8; 32]`：占位密钥，桥接测试不涉及解密
    /// - `EmptyProviderRegistry`：空实现，桥接测试不查 provider
    fn make_test_engine() -> WorkEngine {
        WorkEngine::new(
            Arc::new(sea_orm::DatabaseConnection::default()),
            [0u8; 32],
            Arc::new(EmptyProviderRegistry),
        )
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
