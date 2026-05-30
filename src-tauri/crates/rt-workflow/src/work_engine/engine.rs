//! 统一工作流引擎 —— DAG 管理 + 并发执行 + DB 持久化。
//!
//! 节点类型统一为 axagent_core::workflow_types::WorkflowNode（15 种），
//! 执行通过 NodeDispatcher 分发到对应执行器。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use axagent_core::workflow_types::{EdgeType, JsonSchema, Variable, WorkflowEdge, WorkflowNode};

use crate::workflow_engine::{
    NodeRuntimeState, NodeStatus, Workflow, WorkflowError, WorkflowStatus, current_epoch_ms,
    current_timestamp,
};

use super::dispatcher::NodeDispatcher;
use super::execution_state::{ExecutionState, ExecutionStatus, NodeExecutionRecord};
use super::executors::{
    AgentExecutor, ConditionExecutor, LlmExecutor, PlanCallbacks, ProfileCache, ProviderCache,
    RagCallback, SubWorkflowCallback, ToolCallback, VectorRetrieveCallback,
};
use super::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput, node_type_name};
use super::prompt_template::{CompiledPrompt, compile_prompt};

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

// ── WorkEngine ──

#[derive(Clone)]
pub struct WorkEngine {
    db: Arc<DatabaseConnection>,
    master_key: [u8; 32],
    executions: Arc<Mutex<HashMap<String, ExecutionState>>>,
    workflows: Arc<tokio::sync::RwLock<HashMap<String, Workflow>>>,
    /// 编译后的 prompt 模板：workflow_id -> (node_id -> CompiledPrompt)
    compiled_prompts: Arc<tokio::sync::RwLock<HashMap<String, HashMap<String, CompiledPrompt>>>>,
    /// 编译后的 Rhai 脚本：workflow_id -> (tool_name -> AST)
    compiled_rhai_scripts:
        Arc<tokio::sync::RwLock<HashMap<String, axagent_tools::rhai_engine::RhaiScriptCache>>>,
    cancel_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    dispatcher: Arc<tokio::sync::RwLock<NodeDispatcher>>,
    /// 按工具名注册的 handler 映射（多路注册，优先级最高）
    tool_handlers: Arc<Mutex<HashMap<String, ToolCallback>>>,
    /// 旧版全局回调（fallback，tool_handlers 未命中时使用）
    tool_fallback: Arc<Mutex<Option<ToolCallback>>>,
    /// 工具解析器（按需延迟注册，从全局 tool registry 查找工具）
    tool_resolver: Arc<Mutex<Option<ToolResolver>>>,
    vector_retrieve_callback: Arc<Mutex<Option<VectorRetrieveCallback>>>,
    rag_callback: Arc<Mutex<Option<RagCallback>>>,
    /// Agent executor 共享缓存（跨节点复用，每次 run_workflow 开始时清空）
    agent_provider_cache: Arc<tokio::sync::Mutex<ProviderCache>>,
    agent_profile_cache: Arc<tokio::sync::Mutex<ProfileCache>>,
    /// 断点集（节点 ID → 是否启用，外部通过 set_breakpoints / resume 控制）
    pub breakpoints: Arc<Mutex<HashSet<String>>>,
    /// 暂停信号（resume 时通知等待中的执行）
    pause_signal: Arc<tokio::sync::Notify>,
    /// 节点断路器状态（跨 workflow 运行持久化，防止重试风暴）
    node_breakers: Arc<Mutex<HashMap<String, NodeCircuitBreaker>>>,
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
    /// 继续执行（通知所有等待中的断点）
    pub fn resume_breakpoints(&self) {
        self.pause_signal.notify_waiters();
    }
    /// 单步执行（仅通知一个等待者）
    pub fn step_breakpoint(&self) {
        self.pause_signal.notify_one();
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
        let cache = axagent_tools::rhai_engine::compile_from_tool_defs(tool_defs);
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

    /// 注册/替换节点执行器（Arc<WorkEngine> 下可安全调用）
    pub async fn register_executor<E: NodeExecutorTrait + 'static>(&self, executor: E) {
        self.dispatcher.write().await.register(executor);
    }

    /// Plan 模式专用：AgentExecutor 注入自身引用，使其能创建/执行临时工作流
    pub async fn inject_into_agent_executor(self: &Arc<Self>, engine: Arc<WorkEngine>) {
        let agent = AgentExecutor::with_shared_caches(
            self.db.clone(),
            self.master_key,
            self.agent_provider_cache.clone(),
            self.agent_profile_cache.clone(),
        )
        .with_engine(engine);
        self.register_executor(agent).await;
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

    /// 按工具名注册 handler（多路注册，Arc<WorkEngine> 下可安全调用）
    pub async fn register_tool_handler(&self, tool_name: &str, cb: ToolCallback) {
        self.tool_handlers
            .lock()
            .await
            .insert(tool_name.to_string(), cb);
    }
    /// 设置工具 fallback 回调（旧版兼容，tool_handlers 未命中时使用）
    pub async fn set_tool_callback(&self, cb: ToolCallback) {
        *self.tool_fallback.lock().await = Some(cb);
    }
    /// 设置工具解析器（按需延迟注册，run_workflow 时自动扫描并注册工作流中的工具）
    pub async fn set_tool_resolver(&self, resolver: ToolResolver) {
        *self.tool_resolver.lock().await = Some(resolver);
    }
    /// 设置向量检索回调
    pub async fn set_vector_retrieve_callback(&self, cb: VectorRetrieveCallback) {
        *self.vector_retrieve_callback.lock().await = Some(cb);
    }
    /// 设置 RAG 知识源检索回调（供 Agent 节点从知识库/记忆/Wiki 检索上下文）
    pub async fn set_rag_callback(&self, cb: RagCallback) {
        *self.rag_callback.lock().await = Some(cb);
    }
}

impl WorkEngine {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        let agent_provider_cache = Arc::new(tokio::sync::Mutex::new(None));
        let agent_profile_cache = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        let mut dispatcher = NodeDispatcher::new();
        dispatcher.register(LlmExecutor::new(db.clone(), master_key));
        dispatcher.register(AgentExecutor::with_shared_caches(
            db.clone(),
            master_key,
            agent_provider_cache.clone(),
            agent_profile_cache.clone(),
        ));
        dispatcher.register(ConditionExecutor::new(db.clone(), master_key));
        Self {
            db,
            master_key,
            executions: Arc::new(Mutex::new(HashMap::new())),
            workflows: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            compiled_prompts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            compiled_rhai_scripts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
            dispatcher: Arc::new(tokio::sync::RwLock::new(dispatcher)),
            tool_handlers: Arc::new(Mutex::new(HashMap::new())),
            tool_fallback: Arc::new(Mutex::new(None)),
            tool_resolver: Arc::new(Mutex::new(None)),
            vector_retrieve_callback: Arc::new(Mutex::new(None)),
            rag_callback: Arc::new(Mutex::new(None)),
            agent_provider_cache,
            agent_profile_cache,
            breakpoints: Arc::new(Mutex::new(HashSet::new())),
            pause_signal: Arc::new(tokio::sync::Notify::new()),
            node_breakers: Arc::new(Mutex::new(HashMap::new())),
        }
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
            input["__workflow_model__"] = serde_json::Value::String(model_id.clone());
        }
        if let Some(ref provider_id) = options.provider_id {
            input["__workflow_provider_id__"] = serde_json::Value::String(provider_id.clone());
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

        // 将调用方指定的 model_id / provider_id 写入变量区，供 Agent/LlmExecutor 读取
        if let Some(ref model_id) = options.model_id {
            let mut executions = self.executions.lock().await;
            if let Some(state) = executions.get_mut(&execution_id) {
                state.variables.insert(
                    "__workflow_model__".to_string(),
                    serde_json::Value::String(model_id.clone()),
                );
            }
        }
        if let Some(ref provider_id) = options.provider_id {
            let mut executions = self.executions.lock().await;
            if let Some(state) = executions.get_mut(&execution_id) {
                state.variables.insert(
                    "__workflow_provider_id__".to_string(),
                    serde_json::Value::String(provider_id.clone()),
                );
            }
        }

        // 将模板级变量写入执行上下文，供工具节点和 Agent 节点引用
        if let Some(ref variables) = options.variables {
            let mut executions = self.executions.lock().await;
            if let Some(state) = executions.get_mut(&execution_id) {
                for var in variables {
                    state.variables.insert(var.name.clone(), var.value.clone());
                }
            }
        }
        if options.plan_callbacks.is_some() {
            let mut executions = self.executions.lock().await;
            if let Some(state) = executions.get_mut(&execution_id) {
                state.plan_callbacks = options.plan_callbacks.clone();
            }
        }
        if options.parent_execution_id.is_some() {
            let mut executions = self.executions.lock().await;
            if let Some(state) = executions.get_mut(&execution_id) {
                state.parent_execution_id = options.parent_execution_id.clone();
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

        // 重新注册 AgentExecutor（注入 RAG callback）
        {
            let rag_cb = self.rag_callback.lock().await.clone();
            let agent_executor = if let Some(cb) = rag_cb {
                AgentExecutor::with_shared_caches_and_rag_callback(
                    self.db.clone(),
                    self.master_key,
                    self.agent_provider_cache.clone(),
                    self.agent_profile_cache.clone(),
                    cb,
                )
            } else {
                AgentExecutor::with_shared_caches(
                    self.db.clone(),
                    self.master_key,
                    self.agent_provider_cache.clone(),
                    self.agent_profile_cache.clone(),
                )
            };
            self.dispatcher.write().await.register(agent_executor);
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
        {
            let rhai_cache = self.compiled_rhai_scripts.read().await;
            if let Some(scripts) = rhai_cache.get(workflow_id) {
                let mut handlers = self.tool_handlers.lock().await;
                for (tool_name, ast) in scripts {
                    if !handlers.contains_key(tool_name) {
                        let ast = ast.clone();
                        let tool_handlers = self.tool_handlers.clone();
                        let cb: ToolCallback =
                            std::sync::Arc::new(move |_tn: String, args: serde_json::Value| {
                                let ast = ast.clone();
                                let tool_handlers = tool_handlers.clone();
                                Box::pin(async move {
                                    let handlers = tool_handlers.lock().await;
                                    let mut rhai_tools: std::collections::HashMap<
                                        String,
                                        axagent_tools::rhai_engine::ToolFn,
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
                                    axagent_tools::rhai_engine::execute_rhai_ast(
                                        &ast,
                                        args,
                                        Some(&rhai_tools),
                                    )
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

            for node_id in ready_nodes {
                let node = {
                    let workflows = self.workflows.read().await;
                    workflows
                        .get(workflow_id)
                        .and_then(|wf| wf.nodes.iter().find(|n| n.base_id() == node_id).cloned())
                };
                let Some(node) = node else {
                    continue;
                };

                // 检查断路器
                let cb_open = breakers
                    .entry(node_id.clone())
                    .or_insert_with(NodeCircuitBreaker::new)
                    .is_open(current_epoch_ms());
                if cb_open {
                    self.update_node_status(
                        workflow_id,
                        &node_id,
                        NodeStatus::Failed,
                        None,
                        Some("Circuit breaker open".to_string()),
                        None,
                    )
                    .await
                    .ok();
                    continue;
                }

                // 2. 构建执行上下文（含依赖结果）
                let deps_results = {
                    let workflows = self.workflows.read().await;
                    workflows
                        .get(workflow_id)
                        .map(|wf| Self::get_node_dependency_results(wf, &node_id))
                        .unwrap_or_default()
                };
                let input_snapshot =
                    serde_json::to_value(&deps_results).unwrap_or(serde_json::json!({}));
                let started_at = Utc::now().timestamp_millis();

                self.update_node_status(
                    workflow_id,
                    &node_id,
                    NodeStatus::Running,
                    None,
                    None,
                    None,
                )
                .await
                .ok();

                // 向前端推送"步骤开始运行"进度事件
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
                    })
                    .await;
                }

                // 3. 分发执行
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
                exec_ctx.pause_signal = Some(self.pause_signal.clone());

                // 断点检查：命中时等待 resume
                if exec_ctx.breakpoints.contains(&node_id) {
                    tracing::info!("[Breakpoint] 命中节点 {node_id}，等待 resume...");
                    if let Some(ref sig) = exec_ctx.pause_signal {
                        sig.notified().await;
                    }
                }

                // 注入编译后的 prompt 模板
                {
                    let compiled = self.compiled_prompts.read().await;
                    exec_ctx.compiled_prompts = compiled.get(workflow_id).cloned();
                }

                // 注入运行时回调到执行上下文
                {
                    let tool_handlers = self.tool_handlers.lock().await.clone();
                    let tool_fallback = self.tool_fallback.lock().await.clone();
                    let vr_cb = self.vector_retrieve_callback.lock().await.clone();

                    let engine_clone = self.clone();
                    let sub_model_id = options.model_id.clone();
                    let sub_provider_id = options.provider_id.clone();
                    let sub_step_timeout = options.step_timeout;
                    let sub_cancel_token = cancel_token.clone();

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

                                        let opts = RunOptions {
                                            execution_id: Some(child_execution_id),
                                            input: Some(input_value),
                                            dry_run: false,
                                            parent_execution_id: Some(parent_execution_id),
                                            model_id,
                                            provider_id,
                                            step_timeout: sub_step_timeout,
                                            parent_cancel_token: Some(cancel_token),
                                            ..Default::default()
                                        };

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
                        vector_retrieve: vr_cb,
                    });
                }

                let dispatch_result = tokio::time::timeout(
                    node_timeout,
                    self.dispatcher.read().await.dispatch(&node, &exec_ctx),
                )
                .await;

                let elapsed_ms = (Utc::now().timestamp_millis() - started_at) as u64;

                match dispatch_result {
                    Ok(Ok(output)) => {
                        breakers
                            .entry(node_id.clone())
                            .or_insert_with(NodeCircuitBreaker::new)
                            .record_success();

                        let out_var = output.output_var.clone();
                        self.update_node_status(
                            workflow_id,
                            &node_id,
                            NodeStatus::Completed,
                            Some(output.output.clone()),
                            None,
                            out_var.as_deref(),
                        )
                        .await
                        .ok();

                        let node_name = Some(node.base_title().to_string());
                        let node_type_str = node_type_name(&node).to_string();
                        let sub_workflow_id = if let WorkflowNode::SubWorkflow(sw) = &node {
                            Some(sw.config.sub_workflow_id.clone())
                        } else {
                            None
                        };
                        self.record_node_execution(
                            &execution_id,
                            NodeExecutionRecord {
                                node_id: node_id.clone(),
                                node_type: node_type_str,
                                node_name,
                                status: "completed".to_string(),
                                input: Some(input_snapshot.clone()),
                                output: Some(output.output),
                                execution_time_ms: Some(elapsed_ms),
                                error: None,
                                started_at,
                                completed_at: Some(Utc::now().timestamp_millis()),
                                parent_execution_id: current_parent_execution_id.clone(),
                                sub_workflow_id,
                            },
                        )
                        .await
                        .ok();

                        // ConditionNode 完成后，将不匹配分支的节点标记为 Skipped
                        if matches!(node, WorkflowNode::Condition(_)) {
                            let mut workflows = self.workflows.write().await;
                            if let Some(wf) = workflows.get_mut(workflow_id) {
                                skip_disabled_branch_nodes(wf, &wf.edges.clone(), &node_id);
                            }
                        }
                    },
                    Ok(Err(err)) => {
                        breakers
                            .entry(node_id.clone())
                            .or_insert_with(NodeCircuitBreaker::new)
                            .record_failure(current_epoch_ms());

                        let err_msg = err.to_string();
                        let current_attempts = {
                            let workflows = self.workflows.read().await;
                            workflows
                                .get(workflow_id)
                                .and_then(|wf| wf.node_states.get(&node_id).map(|s| s.attempts))
                                .unwrap_or(0)
                        };
                        let max_retries = node.base_retry().max_retries;

                        if current_attempts < max_retries {
                            let backoff_ms = node.base_retry().base_delay_ms;
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

                            if cancel_token.is_cancelled() {
                                self.finalize_cancelled_workflow(workflow_id).await;
                                self.cancel(&execution_id).await.ok();
                                break;
                            }

                            self.update_node_status(
                                workflow_id,
                                &node_id,
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
                                &node_id,
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
                                node_id: node_id.clone(),
                                node_type: node_type_name(&node).to_string(),
                                node_name: Some(node.base_title().to_string()),
                                status: "failed".to_string(),
                                input: Some(input_snapshot.clone()),
                                output: None,
                                execution_time_ms: Some(elapsed_ms),
                                error: Some(err_msg),
                                started_at,
                                completed_at: Some(Utc::now().timestamp_millis()),
                                parent_execution_id: current_parent_execution_id.clone(),
                                sub_workflow_id: if let WorkflowNode::SubWorkflow(sw) = &node {
                                    Some(sw.config.sub_workflow_id.clone())
                                } else {
                                    None
                                },
                            },
                        )
                        .await
                        .ok();
                    },
                    Err(_) => {
                        breakers
                            .entry(node_id.clone())
                            .or_insert_with(NodeCircuitBreaker::new)
                            .record_failure(current_epoch_ms());

                        let err_msg = "Node execution timeout".to_string();
                        let current_attempts = {
                            let workflows = self.workflows.read().await;
                            workflows
                                .get(workflow_id)
                                .and_then(|wf| wf.node_states.get(&node_id).map(|s| s.attempts))
                                .unwrap_or(0)
                        };
                        let max_retries = node.base_retry().max_retries;

                        if current_attempts < max_retries {
                            let backoff_ms = node.base_retry().base_delay_ms;
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

                            if cancel_token.is_cancelled() {
                                self.finalize_cancelled_workflow(workflow_id).await;
                                self.cancel(&execution_id).await.ok();
                                break;
                            }

                            self.update_node_status(
                                workflow_id,
                                &node_id,
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
                                &node_id,
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
                                node_id: node_id.clone(),
                                node_type: node_type_name(&node).to_string(),
                                node_name: Some(node.base_title().to_string()),
                                status: "timeout".to_string(),
                                input: Some(input_snapshot.clone()),
                                output: None,
                                execution_time_ms: Some(elapsed_ms),
                                error: Some(err_msg),
                                started_at,
                                completed_at: Some(Utc::now().timestamp_millis()),
                                parent_execution_id: current_parent_execution_id.clone(),
                                sub_workflow_id: if let WorkflowNode::SubWorkflow(sw) = &node {
                                    Some(sw.config.sub_workflow_id.clone())
                                } else {
                                    None
                                },
                            },
                        )
                        .await
                        .ok();
                    },
                }
            } // end for node_id in ready_nodes

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
            // 提取 EndNode 的聚合输出（改进 3）
            let end_output = extract_end_output(&wf.nodes, &wf.results);
            // 应用 output_schema 过滤（改进 2b）
            wf.output =
                build_workflow_output(&wf.results, end_output, options.output_schema.as_ref());

            let persist_output = wf.output.clone().unwrap_or_else(|| {
                serde_json::to_value(&wf.results).unwrap_or(serde_json::json!(null))
            });
            let total_time_ms = wf
                .completed_at
                .map(|end| end.saturating_sub(wf.created_at) * 1000)
                .unwrap_or(0);
            self.complete_execution(&execution_id, &persist_output, total_time_ms)
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
        let state =
            ExecutionState::new(execution_id.clone(), workflow_id.to_string(), input.clone());
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
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            if state.status == ExecutionStatus::Paused {
                state.status = ExecutionStatus::Running;
                state.updated_at = Utc::now().timestamp_millis();
            }
            Ok(())
        } else {
            Err(WorkEngineError::NotFound(execution_id.to_string()))
        }
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

    pub async fn complete_execution(
        &self,
        execution_id: &str,
        output: &serde_json::Value,
        total_time_ms: u64,
    ) -> Result<(), WorkEngineError> {
        let mut executions = self.executions.lock().await;
        if let Some(state) = executions.get_mut(execution_id) {
            state.status = ExecutionStatus::Completed;
            state.total_time_ms = total_time_ms;
            state.updated_at = Utc::now().timestamp_millis();
            let node_executions = serde_json::to_string(&state.node_records).ok();
            let output_result = serde_json::to_string(output).ok();
            drop(executions);
            axagent_core::repo::workflow_execution::update_workflow_execution_status(
                &self.db,
                execution_id,
                "completed",
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
