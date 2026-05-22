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

use axagent_core::workflow_types::{WorkflowEdge, WorkflowNode};

use crate::workflow_engine::{
    NodeRuntimeState, NodeStatus, Workflow, WorkflowError, WorkflowStatus, current_epoch_ms,
    current_timestamp,
};

use super::dispatcher::NodeDispatcher;
use super::execution_state::{ExecutionState, ExecutionStatus, NodeExecutionRecord};
use super::executors::{
    AgentExecutor, LlmExecutor, SubWorkflowCallback, ToolCallback, VectorRetrieveCallback,
};
use super::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

/// 工作流运行选项
#[derive(Debug, Clone)]
pub struct RunOptions {
    pub max_concurrent: usize,
    pub step_timeout: Duration,
    /// 调用方指定的模型 ID（来自会话/用户设置），执行器优先使用
    pub model_id: Option<String>,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            step_timeout: Duration::from_secs(300),
            model_id: None,
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

pub struct WorkEngine {
    db: Arc<DatabaseConnection>,
    executions: Arc<Mutex<HashMap<String, ExecutionState>>>,
    workflows: Arc<tokio::sync::RwLock<HashMap<String, Workflow>>>,
    cancel_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    dispatcher: Arc<tokio::sync::RwLock<NodeDispatcher>>,
    tool_callback: Arc<Mutex<Option<ToolCallback>>>,
    subworkflow_callback: Arc<Mutex<Option<SubWorkflowCallback>>>,
    vector_retrieve_callback: Arc<Mutex<Option<VectorRetrieveCallback>>>,
}

impl WorkEngine {
    /// 注册/替换节点执行器（Arc<WorkEngine> 下可安全调用）
    pub async fn register_executor<E: NodeExecutorTrait + 'static>(&self, executor: E) {
        self.dispatcher.write().await.register(executor);
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

    /// 设置工具回调（Arc<WorkEngine> 下可安全调用，注入后 ToolExecutor 即生效）
    pub async fn set_tool_callback(&self, cb: ToolCallback) {
        *self.tool_callback.lock().await = Some(cb);
    }
    /// 设置子工作流回调
    pub async fn set_subworkflow_callback(&self, cb: SubWorkflowCallback) {
        *self.subworkflow_callback.lock().await = Some(cb);
    }
    /// 设置向量检索回调
    pub async fn set_vector_retrieve_callback(&self, cb: VectorRetrieveCallback) {
        *self.vector_retrieve_callback.lock().await = Some(cb);
    }
}

impl WorkEngine {
    pub fn new(db: Arc<DatabaseConnection>, master_key: [u8; 32]) -> Self {
        let mut dispatcher = NodeDispatcher::new();
        dispatcher.register(LlmExecutor::new(db.clone(), master_key));
        dispatcher.register(AgentExecutor::new(db.clone(), master_key));
        Self {
            db,
            executions: Arc::new(Mutex::new(HashMap::new())),
            workflows: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
            dispatcher: Arc::new(tokio::sync::RwLock::new(dispatcher)),
            tool_callback: Arc::new(Mutex::new(None)),
            subworkflow_callback: Arc::new(Mutex::new(None)),
            vector_retrieve_callback: Arc::new(Mutex::new(None)),
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
            if !done_or_skipped.contains(edge.source.as_str()) {
                *remaining_deps.entry(edge.target.as_str()).or_insert(0) += 1;
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
    fn get_node_dependency_results(
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
            workflow.results.insert(node_id.to_string(), r);
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
        let cancel_token = CancellationToken::new();
        {
            let mut tokens = self.cancel_tokens.lock().await;
            tokens.insert(workflow_id.to_string(), cancel_token.clone());
        }

        // 构建执行输入：将 model_id 写入上下文，供执行器读取
        let input = if let Some(ref model_id) = options.model_id {
            serde_json::json!({"__workflow_model__": model_id})
        } else {
            serde_json::json!({})
        };

        let execution_id = self
            .start_workflow(workflow_id, input)
            .await
            .map_err(|e| WorkflowError::SerializationError(e.to_string()))?;

        // 将调用方指定的 model_id 写入变量区，供 Agent/LlmExecutor 读取
        if let Some(ref model_id) = options.model_id {
            let mut executions = self.executions.lock().await;
            if let Some(state) = executions.get_mut(&execution_id) {
                state.variables.insert(
                    "__workflow_model__".to_string(),
                    serde_json::Value::String(model_id.clone()),
                );
            }
        }

        {
            let mut workflows = self.workflows.write().await;
            if let Some(workflow) = workflows.get_mut(workflow_id) {
                workflow.status = WorkflowStatus::Running;
            }
        }

        let mut breakers: HashMap<String, NodeCircuitBreaker> = HashMap::new();

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

            // 1. 取一个就绪节点
            let ready_nodes = self.get_ready_steps(workflow_id).await?;
            let Some(node_id) = ready_nodes.into_iter().next() else {
                break; // 无就绪节点，结束
            };

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
            let started_at = Utc::now().timestamp_millis();

            self.update_node_status(workflow_id, &node_id, NodeStatus::Running, None, None)
                .await
                .ok();

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

            // 注入运行时回调到执行上下文
            {
                let tool_cb = self.tool_callback.lock().await.clone();
                let sub_cb = self.subworkflow_callback.lock().await.clone();
                let vr_cb = self.vector_retrieve_callback.lock().await.clone();
                exec_ctx.callbacks = Some(super::execution_state::ExecutionContextCallbacks {
                    tool: tool_cb,
                    subworkflow: sub_cb,
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

                    self.update_node_status(
                        workflow_id,
                        &node_id,
                        NodeStatus::Completed,
                        Some(output.output.clone()),
                        None,
                    )
                    .await
                    .ok();

                    self.record_node_execution(
                        &execution_id,
                        NodeExecutionRecord {
                            node_id: node_id.clone(),
                            node_type: "workflow_node".to_string(),
                            status: "completed".to_string(),
                            input: None,
                            output: Some(output.output),
                            execution_time_ms: Some(elapsed_ms),
                            error: None,
                            started_at,
                            completed_at: Some(Utc::now().timestamp_millis()),
                        },
                    )
                    .await
                    .ok();
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
                        )
                        .await
                        .ok();
                    }

                    self.record_node_execution(
                        &execution_id,
                        NodeExecutionRecord {
                            node_id: node_id.clone(),
                            node_type: "workflow_node".to_string(),
                            status: "failed".to_string(),
                            input: None,
                            output: None,
                            execution_time_ms: Some(elapsed_ms),
                            error: Some(err_msg),
                            started_at,
                            completed_at: Some(Utc::now().timestamp_millis()),
                        },
                    )
                    .await
                    .ok();
                },
                Err(_) => {
                    // 超时
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
                        )
                        .await
                        .ok();
                    }

                    self.record_node_execution(
                        &execution_id,
                        NodeExecutionRecord {
                            node_id: node_id.clone(),
                            node_type: "workflow_node".to_string(),
                            status: "failed".to_string(),
                            input: None,
                            output: None,
                            execution_time_ms: Some(elapsed_ms),
                            error: Some(err_msg),
                            started_at,
                            completed_at: Some(Utc::now().timestamp_millis()),
                        },
                    )
                    .await
                    .ok();
                },
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

        let result = {
            let workflows = self.workflows.read().await;
            workflows.get(workflow_id).cloned()
        };

        if let Some(ref wf) = result {
            let output = serde_json::to_value(&wf.results).unwrap_or(serde_json::json!(null));
            let total_time_ms = wf
                .completed_at
                .map(|end| end.saturating_sub(wf.created_at) * 1000)
                .unwrap_or(0);
            self.complete_execution(&execution_id, &output, total_time_ms)
                .await
                .ok();
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
    ) -> Result<String, WorkEngineError> {
        let execution_id = uuid::Uuid::new_v4().to_string();
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
