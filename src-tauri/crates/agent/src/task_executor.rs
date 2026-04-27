use crate::task::{TaskGraph, TaskNode, TaskStatus, TopologicalSortError};
use crate::task_decomposer::{DecompositionError, TaskDecomposer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub current_tasks: Vec<String>,
    pub percentage: f32,
}

impl ExecutionProgress {
    pub fn new(graph: &TaskGraph) -> Self {
        Self {
            total_tasks: graph.tasks.len(),
            completed_tasks: 0,
            failed_tasks: 0,
            current_tasks: Vec::new(),
            percentage: 0.0,
        }
    }

    pub fn update(&mut self, graph: &TaskGraph) {
        self.total_tasks = graph.tasks.len();
        self.completed_tasks = graph
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        self.failed_tasks = graph
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();
        self.current_tasks = graph
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .map(|t| t.id.clone())
            .collect();
        self.percentage = if self.total_tasks > 0 {
            (self.completed_tasks as f32 / self.total_tasks as f32) * 100.0
        } else {
            100.0
        };
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    Started,
    TaskStarted(String),
    TaskCompleted(String),
    TaskFailed(String, String),
    Progress(ExecutionProgress),
    Completed,
    Failed(String),
}

pub struct TaskExecutor {
    decomposer: Arc<TaskDecomposer>,
    graph: Arc<RwLock<Option<TaskGraph>>>,
    event_sender: broadcast::Sender<ExecutionEvent>,
    inner_executor: Arc<DefaultTaskExecutorImpl>,
    config: TaskExecutorConfig,
}

#[derive(Debug, Clone)]
pub struct TaskExecutorConfig {
    pub continue_on_failure: bool,
    pub task_timeout_ms: u64,
    pub max_concurrent: usize,
    pub enable_retry: bool,
    pub max_retries: usize,
}

impl Default for TaskExecutorConfig {
    fn default() -> Self {
        Self {
            continue_on_failure: false,
            task_timeout_ms: 300_000,
            max_concurrent: 10,
            enable_retry: true,
            max_retries: 3,
        }
    }
}

pub trait TaskExecutorImpl: Send + Sync {
    fn execute_task(&self, context: &TaskContext) -> impl std::future::Future<Output = Result<serde_json::Value, TaskExecutorError>> + Send;
}

#[derive(Debug, Clone)]
pub struct TaskContext {
    pub task_id: String,
    pub task_type: crate::task::TaskType,
    pub description: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl TaskResult {
    pub fn success(output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            output,
            error: None,
            duration_ms,
        }
    }

    pub fn failed(error: String) -> Self {
        Self {
            output: serde_json::Value::Null,
            error: Some(error),
            duration_ms: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TaskExecutorError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Cancelled: {0}")]
    Cancelled(String),
}

impl TaskExecutor {
    pub fn new() -> Self {
        let decomposer = Arc::new(TaskDecomposer::new());
        let (event_sender, _) = broadcast::channel(100);

        Self {
            decomposer,
            graph: Arc::new(RwLock::new(None)),
            event_sender,
            inner_executor: Arc::new(DefaultTaskExecutorImpl),
            config: TaskExecutorConfig::default(),
        }
    }

    pub fn with_decomposer(mut self, decomposer: TaskDecomposer) -> Self {
        self.decomposer = Arc::new(decomposer);
        self
    }

    #[allow(dead_code)]
    pub(crate) fn with_inner_executor(mut self, executor: DefaultTaskExecutorImpl) -> Self {
        self.inner_executor = Arc::new(executor);
        self
    }

    pub fn with_config(mut self, config: TaskExecutorConfig) -> Self {
        self.config = config;
        self
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ExecutionEvent> {
        self.event_sender.subscribe()
    }

    pub async fn prepare(&self, user_input: &str) -> Result<TaskGraph, DecompositionError> {
        let graph = self.decomposer.decompose(user_input)?;
        self.decomposer.validate_graph(&graph)?;
        *self.graph.write().await = Some(graph.clone());
        Ok(graph)
    }

    pub async fn execute(&self) -> Result<TaskGraph, ExecutionError> {
        let graph_guard = self.graph.read().await;
        let mut graph = graph_guard.clone().ok_or(ExecutionError::NotPrepared)?;
        drop(graph_guard);

        self.emit(ExecutionEvent::Started);

        let execution_order = graph.topological_sort().map_err(|e| {
            ExecutionError::InvalidGraph(match e {
                TopologicalSortError::CircularDependency(tasks) => {
                    format!("Circular dependency detected: {:?}", tasks)
                }
            })
        })?;

        tracing::info!(
            "Task execution order: {:?}",
            execution_order
                .iter()
                .map(|batch| batch.len())
                .collect::<Vec<_>>()
        );

        for (batch_idx, batch) in execution_order.iter().enumerate() {
            tracing::info!("Executing batch {} with {} tasks", batch_idx, batch.len());

            let results = self.execute_batch(&batch, &graph).await?;

            for (task_id, result) in results {
                self.update_task_state(&mut graph, &task_id, result);
            }

            let progress = ExecutionProgress::new(&graph);
            self.emit(ExecutionEvent::Progress(progress.clone()));

            for task in &graph.tasks {
                match task.status {
                    TaskStatus::Running => {
                        self.emit(ExecutionEvent::TaskStarted(task.id.clone()));
                    }
                    TaskStatus::Completed => {
                        self.emit(ExecutionEvent::TaskCompleted(task.id.clone()));
                    }
                    TaskStatus::Failed => {
                        self.emit(ExecutionEvent::TaskFailed(
                            task.id.clone(),
                            task.error.clone().unwrap_or_default(),
                        ));
                    }
                    _ => {}
                }
            }

            if graph.has_failures() && !self.config.continue_on_failure {
                self.emit(ExecutionEvent::Failed(
                    graph.get_failed_task_ids().join(", "),
                ));
                *self.graph.write().await = Some(graph.clone());
                return Err(ExecutionError::TaskFailed(
                    graph.get_failed_task_ids(),
                ));
            }
        }

        *self.graph.write().await = Some(graph.clone());

        if graph.has_failures() {
            self.emit(ExecutionEvent::Failed(
                graph.get_failed_task_ids().join(", "),
            ));
        } else {
            self.emit(ExecutionEvent::Completed);
        }

        Ok(graph)
    }

    async fn execute_batch(
        &self,
        task_ids: &[String],
        graph: &TaskGraph,
    ) -> Result<Vec<(String, TaskResult)>, ExecutionError> {
        let mut handles = Vec::new();

        for task_id in task_ids {
            let task = match graph.get_task(task_id) {
                Some(t) => t,
                None => continue,
            };

            if !graph.dependencies_ready(task_id) {
                tracing::warn!("Task {} dependencies not ready, skipping", task_id);
                continue;
            }

            let context = match self.build_context(task, graph) {
                Ok(ctx) => ctx,
                Err(e) => {
                    return Err(e);
                }
            };

            let handle = self.spawn_task(task_id.clone(), context);
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok((task_id, Ok(result))) => {
                    results.push((task_id, result));
                }
                Ok((task_id, Err(e))) => {
                    results.push((task_id, TaskResult::failed(e.to_string())));
                }
                Err(e) => {
                    tracing::error!("Task panicked: {:?}", e);
                }
            }
        }

        Ok(results)
    }

    fn spawn_task(
        &self,
        task_id: String,
        context: TaskContext,
    ) -> tokio::task::JoinHandle<(String, Result<TaskResult, TaskExecutorError>)> {
        let executor = self.inner_executor.clone();
        let task_id_clone = task_id.clone();
        let timeout_ms = self.config.task_timeout_ms;

        tokio::spawn(async move {
            let start = Instant::now();

            let result = timeout(
                Duration::from_millis(timeout_ms),
                executor.execute_task(&context),
            )
            .await;

            let duration_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(Ok(output)) => (
                    task_id_clone,
                    Ok(TaskResult::success(output, duration_ms)),
                ),
                Ok(Err(e)) => (task_id_clone, Err(e)),
                Err(_) => (
                    task_id_clone,
                    Err(TaskExecutorError::Timeout(format!(
                        "Task execution timed out after {}ms",
                        timeout_ms
                    ))),
                ),
            }
        })
    }

    fn build_context(
        &self,
        task: &TaskNode,
        graph: &TaskGraph,
    ) -> Result<TaskContext, ExecutionError> {
        let mut context = TaskContext {
            task_id: task.id.clone(),
            task_type: task.task_type,
            description: task.description.clone(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        };

        for dep_id in &task.dependencies {
            if let Some(dep_task) = graph.get_task(dep_id) {
                if let Some(ref output) = dep_task.result {
                    context.inputs.insert(dep_id.clone(), output.clone());
                } else if dep_task.status == TaskStatus::Failed {
                    return Err(ExecutionError::InvalidGraph(format!(
                        "Dependency {} failed",
                        dep_id
                    )));
                }
            }
        }

        Ok(context)
    }

    fn update_task_state(&self, graph: &mut TaskGraph, task_id: &str, result: TaskResult) {
        if let Some(task) = graph.get_task_mut(task_id) {
            if result.is_success() {
                task.status = TaskStatus::Completed;
                task.result = Some(result.output);
            } else {
                task.status = TaskStatus::Failed;
                task.error = Some(result.error.clone().unwrap_or_default());
            }
        }
    }

    pub async fn execute_with_groups(&self) -> Result<TaskGraph, ExecutionError> {
        self.execute().await
    }

    pub async fn get_progress(&self) -> Option<ExecutionProgress> {
        let guard = self.graph.read().await;
        guard.as_ref().map(|g| {
            let mut progress = ExecutionProgress::new(g);
            progress.update(g);
            progress
        })
    }

    pub async fn get_graph(&self) -> Option<TaskGraph> {
        let guard = self.graph.read().await;
        guard.clone()
    }

    fn emit(&self, event: ExecutionEvent) {
        let _ = self.event_sender.send(event);
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct DefaultTaskExecutorImpl;

impl TaskExecutorImpl for DefaultTaskExecutorImpl {
    async fn execute_task(&self, context: &TaskContext) -> Result<serde_json::Value, TaskExecutorError> {
        match context.task_type {
            crate::task::TaskType::ToolCall => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(serde_json::json!({
                    "output": format!("Executed tool call: {}", context.task_id),
                    "task_id": context.task_id,
                }))
            }
            crate::task::TaskType::Reasoning => {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(serde_json::json!({
                    "output": format!("Reasoning completed for: {}", context.task_id),
                    "task_id": context.task_id,
                }))
            }
            crate::task::TaskType::Query => {
                tokio::time::sleep(Duration::from_millis(150)).await;
                Ok(serde_json::json!({
                    "output": format!("Query executed: {}", context.task_id),
                    "task_id": context.task_id,
                }))
            }
            crate::task::TaskType::Validation => {
                tokio::time::sleep(Duration::from_millis(75)).await;
                Ok(serde_json::json!({
                    "output": format!("Validation passed: {}", context.task_id),
                    "task_id": context.task_id,
                }))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Task executor not prepared")]
    NotPrepared,

    #[error("Some tasks failed: {0:?}")]
    TaskFailed(Vec<String>),

    #[error("Graph validation failed: {0}")]
    InvalidGraph(String),

    #[error("Execution error: {0}")]
    Other(String),
}

impl From<DecompositionError> for ExecutionError {
    fn from(e: DecompositionError) -> Self {
        ExecutionError::InvalidGraph(e.to_string())
    }
}

impl From<TopologicalSortError> for ExecutionError {
    fn from(e: TopologicalSortError) -> Self {
        ExecutionError::InvalidGraph(e.to_string())
    }
}
