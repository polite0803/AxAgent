use crate::task::{TaskGraph, TaskNode, TaskStatus, TopologicalSortError};
use crate::task_decomposer::{DecompositionError, TaskDecomposer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, broadcast};
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
    fn execute_task(
        &self,
        context: &TaskContext,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, TaskExecutorError>> + Send;
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
        Self::default_inner().with_inner_executor(DefaultTaskExecutorImpl)
    }

    fn default_inner() -> Self {
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
                },
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

            let results = self.execute_batch(batch, &graph).await?;

            for (task_id, result) in results {
                self.update_task_state(&mut graph, &task_id, result);
            }

            let progress = ExecutionProgress::new(&graph);
            self.emit(ExecutionEvent::Progress(progress.clone()));

            for task in &graph.tasks {
                match task.status {
                    TaskStatus::Running => {
                        self.emit(ExecutionEvent::TaskStarted(task.id.clone()));
                    },
                    TaskStatus::Completed => {
                        self.emit(ExecutionEvent::TaskCompleted(task.id.clone()));
                    },
                    TaskStatus::Failed => {
                        self.emit(ExecutionEvent::TaskFailed(
                            task.id.clone(),
                            task.error.clone().unwrap_or_default(),
                        ));
                    },
                    _ => {},
                }
            }

            if graph.has_failures() && !self.config.continue_on_failure {
                self.emit(ExecutionEvent::Failed(graph.get_failed_task_ids().join(", ")));
                *self.graph.write().await = Some(graph.clone());
                return Err(ExecutionError::TaskFailed(graph.get_failed_task_ids()));
            }
        }

        *self.graph.write().await = Some(graph.clone());

        if graph.has_failures() {
            self.emit(ExecutionEvent::Failed(graph.get_failed_task_ids().join(", ")));
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
                },
            };

            let handle = self.spawn_task(task_id.clone(), context);
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok((task_id, Ok(result))) => {
                    results.push((task_id, result));
                },
                Ok((task_id, Err(e))) => {
                    results.push((task_id, TaskResult::failed(e.to_string())));
                },
                Err(e) => {
                    tracing::error!("Task panicked: {:?}", e);
                },
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

            let result =
                timeout(Duration::from_millis(timeout_ms), executor.execute_task(&context)).await;

            let duration_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(Ok(output)) => (task_id_clone, Ok(TaskResult::success(output, duration_ms))),
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
    async fn execute_task(
        &self,
        context: &TaskContext,
    ) -> Result<serde_json::Value, TaskExecutorError> {
        match context.task_type {
            crate::task::TaskType::ToolCall => {
                let tool_name = context
                    .inputs
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let tool_input = context
                    .inputs
                    .get("tool_input")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                if tool_name.is_empty() {
                    return Err(TaskExecutorError::ExecutionFailed(
                        "ToolCall task missing 'tool_name' in inputs".to_string(),
                    ));
                }

                let (_server_name, _local_name) = parse_tool_name(tool_name);
                let _args = if let Some(obj) = tool_input.as_object() {
                    serde_json::to_value(obj.clone()).unwrap_or(tool_input.clone())
                } else {
                    serde_json::json!({ "input": tool_input })
                };

                // DefaultTaskExecutorImpl 是轻量默认实现，不持有工具注册表。
                // 在完整 Harness 运行中，实际工具执行由运行时层的 ToolExecutor 处理。
                Err(TaskExecutorError::ExecutionFailed(format!(
                    "Tool '{}' execution requires runtime-injected ToolExecutor",
                    tool_name
                )))
            },
            crate::task::TaskType::Reasoning => {
                let prompt = context
                    .inputs
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&context.description);

                Ok(serde_json::json!({
                    "output": format!("Reasoning completed for: {}", context.task_id),
                    "task_id": context.task_id,
                    "prompt_used": prompt,
                }))
            },
            crate::task::TaskType::Query => {
                let query = context
                    .inputs
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&context.description);

                Ok(serde_json::json!({
                    "output": format!("Query executed: {}", context.task_id),
                    "task_id": context.task_id,
                    "query": query,
                }))
            },
            crate::task::TaskType::Validation => {
                let target = context
                    .inputs
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&context.description);
                let expected = context
                    .inputs
                    .get("expected")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let passed = if !expected.is_empty() {
                    target.contains(expected)
                } else {
                    !target.is_empty()
                };

                Ok(serde_json::json!({
                    "output": format!("Validation {}: {}", if passed { "passed" } else { "failed" }, context.task_id),
                    "task_id": context.task_id,
                    "passed": passed,
                    "target": target,
                    "expected": expected,
                }))
            },
        }
    }
}

use axagent_harness::parse_tool_name;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{TaskNode, TaskStatus, TaskType};
    use crate::task_decomposer::DecompositionError;
    use std::collections::HashMap;

    #[test]
    fn test_task_context_creation() {
        let context = TaskContext {
            task_id: "task-1".to_string(),
            task_type: TaskType::Query,
            description: "test task".to_string(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        };
        assert_eq!(context.task_id, "task-1");
        assert_eq!(context.task_type, TaskType::Query);
        assert_eq!(context.description, "test task");
        assert!(context.inputs.is_empty());
        assert!(context.outputs.is_empty());
    }

    #[test]
    fn test_task_context_with_inputs() {
        let mut inputs = HashMap::new();
        inputs.insert("key".to_string(), serde_json::json!("value"));
        let context = TaskContext {
            task_id: "task-2".to_string(),
            task_type: TaskType::ToolCall,
            description: "tool task".to_string(),
            inputs,
            outputs: HashMap::new(),
        };
        assert_eq!(context.inputs.len(), 1);
        assert_eq!(context.inputs["key"], serde_json::json!("value"));
    }

    #[test]
    fn test_task_executor_error_execution_failed() {
        let err = TaskExecutorError::ExecutionFailed("something broke".to_string());
        assert!(err.to_string().contains("something broke"));
    }

    #[test]
    fn test_task_executor_error_timeout() {
        let err = TaskExecutorError::Timeout("timed out".to_string());
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn test_task_executor_error_cancelled() {
        let err = TaskExecutorError::Cancelled("was cancelled".to_string());
        assert!(err.to_string().contains("was cancelled"));
    }

    #[test]
    fn test_task_result_success() {
        let result = TaskResult::success(serde_json::json!("output"), 150);
        assert!(result.is_success());
        assert_eq!(result.output, serde_json::json!("output"));
        assert!(result.error.is_none());
        assert_eq!(result.duration_ms, 150);
    }

    #[test]
    fn test_task_result_failed() {
        let result = TaskResult::failed("error occurred".to_string());
        assert!(!result.is_success());
        assert_eq!(result.output, serde_json::Value::Null);
        assert_eq!(result.error, Some("error occurred".to_string()));
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn test_parse_tool_name_with_slash() {
        let (server, tool) = parse_tool_name("myserver/mytool");
        assert_eq!(server, "myserver");
        assert_eq!(tool, "mytool");
    }

    #[test]
    fn test_parse_tool_name_without_slash() {
        let (server, tool) = parse_tool_name("mytool");
        assert_eq!(server, "");
        assert_eq!(tool, "mytool");
    }

    #[test]
    fn test_parse_tool_name_multiple_slashes() {
        let (server, tool) = parse_tool_name("server/path/tool");
        assert_eq!(server, "server");
        assert_eq!(tool, "path/tool");
    }

    #[test]
    fn test_parse_tool_name_empty() {
        let (server, tool) = parse_tool_name("");
        assert_eq!(server, "");
        assert_eq!(tool, "");
    }

    #[test]
    fn test_execution_progress_new() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("1", "task 1", TaskType::Query));
        graph.add_task(TaskNode::new("2", "task 2", TaskType::Reasoning));
        let progress = ExecutionProgress::new(&graph);
        assert_eq!(progress.total_tasks, 2);
        assert_eq!(progress.completed_tasks, 0);
        assert_eq!(progress.failed_tasks, 0);
        assert!(progress.current_tasks.is_empty());
        assert_eq!(progress.percentage, 0.0);
    }

    #[test]
    fn test_execution_progress_update_completed() {
        let mut graph = TaskGraph::new();
        let mut task1 = TaskNode::new("1", "task 1", TaskType::Query);
        task1.status = TaskStatus::Completed;
        graph.add_task(task1);
        let mut progress = ExecutionProgress::new(&graph);
        progress.update(&graph);
        assert_eq!(progress.completed_tasks, 1);
        assert!((progress.percentage - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_execution_progress_update_running() {
        let mut graph = TaskGraph::new();
        let mut task1 = TaskNode::new("1", "task 1", TaskType::Query);
        task1.status = TaskStatus::Running;
        graph.add_task(task1);
        let mut progress = ExecutionProgress::new(&graph);
        progress.update(&graph);
        assert_eq!(progress.current_tasks, vec!["1"]);
        assert_eq!(progress.percentage, 0.0);
    }

    #[test]
    fn test_execution_progress_update_failed() {
        let mut graph = TaskGraph::new();
        let mut task1 = TaskNode::new("1", "task 1", TaskType::Query);
        task1.status = TaskStatus::Failed;
        task1.error = Some("error".to_string());
        graph.add_task(task1);
        let mut progress = ExecutionProgress::new(&graph);
        progress.update(&graph);
        assert_eq!(progress.failed_tasks, 1);
    }

    #[test]
    fn test_execution_progress_empty_graph() {
        let graph = TaskGraph::new();
        let progress = ExecutionProgress::new(&graph);
        assert_eq!(progress.total_tasks, 0);
        assert_eq!(progress.percentage, 0.0);
    }

    #[test]
    fn test_execution_progress_mixed_statuses() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("1", "t1", TaskType::Query);
        t1.status = TaskStatus::Completed;
        let mut t2 = TaskNode::new("2", "t2", TaskType::Reasoning);
        t2.status = TaskStatus::Running;
        let mut t3 = TaskNode::new("3", "t3", TaskType::Validation);
        t3.status = TaskStatus::Failed;
        t3.error = Some("err".to_string());
        graph.add_task(t1);
        graph.add_task(t2);
        graph.add_task(t3);
        let mut progress = ExecutionProgress::new(&graph);
        progress.update(&graph);
        assert_eq!(progress.total_tasks, 3);
        assert_eq!(progress.completed_tasks, 1);
        assert_eq!(progress.failed_tasks, 1);
        assert_eq!(progress.current_tasks, vec!["2"]);
        assert!((progress.percentage - 33.333334).abs() < 0.01);
    }

    #[test]
    fn test_task_executor_config_default() {
        let config = TaskExecutorConfig::default();
        assert!(!config.continue_on_failure);
        assert_eq!(config.task_timeout_ms, 300_000);
        assert_eq!(config.max_concurrent, 10);
        assert!(config.enable_retry);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_task_executor_config_custom() {
        let config = TaskExecutorConfig {
            continue_on_failure: true,
            task_timeout_ms: 60_000,
            max_concurrent: 5,
            enable_retry: false,
            max_retries: 1,
        };
        assert!(config.continue_on_failure);
        assert_eq!(config.task_timeout_ms, 60_000);
        assert_eq!(config.max_concurrent, 5);
        assert!(!config.enable_retry);
        assert_eq!(config.max_retries, 1);
    }

    #[tokio::test]
    async fn test_task_executor_new() {
        let executor = TaskExecutor::new();
        assert!(executor.get_graph().await.is_none());
    }

    #[tokio::test]
    async fn test_task_executor_default() {
        let executor = TaskExecutor::default();
        assert!(executor.get_graph().await.is_none());
    }

    #[tokio::test]
    async fn test_task_executor_with_config() {
        let config = TaskExecutorConfig {
            continue_on_failure: true,
            task_timeout_ms: 60_000,
            max_concurrent: 5,
            enable_retry: false,
            max_retries: 1,
        };
        let executor = TaskExecutor::new().with_config(config);
        assert!(executor.get_graph().await.is_none());
    }

    #[test]
    fn test_task_executor_subscribe() {
        let executor = TaskExecutor::new();
        let _receiver = executor.subscribe();
    }

    #[tokio::test]
    async fn test_task_executor_get_progress_none() {
        let executor = TaskExecutor::new();
        assert!(executor.get_progress().await.is_none());
    }

    #[tokio::test]
    async fn test_task_executor_get_graph_none() {
        let executor = TaskExecutor::new();
        assert!(executor.get_graph().await.is_none());
    }

    #[tokio::test]
    async fn test_task_executor_execute_not_prepared() {
        let executor = TaskExecutor::new();
        let result = executor.execute().await;
        assert!(matches!(result, Err(ExecutionError::NotPrepared)));
    }

    #[test]
    fn test_execution_error_not_prepared() {
        let err = ExecutionError::NotPrepared;
        assert!(err.to_string().contains("not prepared"));
    }

    #[test]
    fn test_execution_error_task_failed() {
        let err = ExecutionError::TaskFailed(vec!["t1".to_string(), "t2".to_string()]);
        assert!(err.to_string().contains("t1"));
        assert!(err.to_string().contains("t2"));
    }

    #[test]
    fn test_execution_error_invalid_graph() {
        let err = ExecutionError::InvalidGraph("bad graph".to_string());
        assert!(err.to_string().contains("bad graph"));
    }

    #[test]
    fn test_execution_error_other() {
        let err = ExecutionError::Other("misc error".to_string());
        assert!(err.to_string().contains("misc error"));
    }

    #[test]
    fn test_execution_error_from_decomposition_error() {
        let decomp_err = DecompositionError::ParseError("parse failed".to_string());
        let exec_err: ExecutionError = decomp_err.into();
        assert!(matches!(exec_err, ExecutionError::InvalidGraph(_)));
    }

    #[test]
    fn test_execution_error_from_topological_sort_error() {
        let topo_err = TopologicalSortError::CircularDependency(vec!["a".to_string()]);
        let exec_err: ExecutionError = topo_err.into();
        assert!(matches!(exec_err, ExecutionError::InvalidGraph(_)));
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_reasoning() {
        let executor = DefaultTaskExecutorImpl;
        let context = TaskContext {
            task_id: "r1".to_string(),
            task_type: TaskType::Reasoning,
            description: "reason about X".to_string(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await.unwrap();
        assert_eq!(result["task_id"], "r1");
        assert!(
            result["output"]
                .as_str()
                .unwrap()
                .contains("Reasoning completed")
        );
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_reasoning_with_prompt() {
        let executor = DefaultTaskExecutorImpl;
        let mut inputs = HashMap::new();
        inputs.insert("prompt".to_string(), serde_json::json!("custom prompt"));
        let context = TaskContext {
            task_id: "r2".to_string(),
            task_type: TaskType::Reasoning,
            description: "default".to_string(),
            inputs,
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await.unwrap();
        assert_eq!(result["prompt_used"], "custom prompt");
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_query() {
        let executor = DefaultTaskExecutorImpl;
        let context = TaskContext {
            task_id: "q1".to_string(),
            task_type: TaskType::Query,
            description: "query something".to_string(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await.unwrap();
        assert_eq!(result["task_id"], "q1");
        assert!(
            result["output"]
                .as_str()
                .unwrap()
                .contains("Query executed")
        );
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_query_with_query_input() {
        let executor = DefaultTaskExecutorImpl;
        let mut inputs = HashMap::new();
        inputs.insert("query".to_string(), serde_json::json!("custom query"));
        let context = TaskContext {
            task_id: "q2".to_string(),
            task_type: TaskType::Query,
            description: "default".to_string(),
            inputs,
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await.unwrap();
        assert_eq!(result["query"], "custom query");
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_validation_passed() {
        let executor = DefaultTaskExecutorImpl;
        let mut inputs = HashMap::new();
        inputs.insert("target".to_string(), serde_json::json!("hello world"));
        inputs.insert("expected".to_string(), serde_json::json!("world"));
        let context = TaskContext {
            task_id: "v1".to_string(),
            task_type: TaskType::Validation,
            description: "validate".to_string(),
            inputs,
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await.unwrap();
        assert_eq!(result["passed"], true);
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_validation_failed() {
        let executor = DefaultTaskExecutorImpl;
        let mut inputs = HashMap::new();
        inputs.insert("target".to_string(), serde_json::json!("hello world"));
        inputs.insert("expected".to_string(), serde_json::json!("missing"));
        let context = TaskContext {
            task_id: "v2".to_string(),
            task_type: TaskType::Validation,
            description: "validate".to_string(),
            inputs,
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await.unwrap();
        assert_eq!(result["passed"], false);
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_validation_no_expected() {
        let executor = DefaultTaskExecutorImpl;
        let mut inputs = HashMap::new();
        inputs.insert("target".to_string(), serde_json::json!("non-empty"));
        let context = TaskContext {
            task_id: "v3".to_string(),
            task_type: TaskType::Validation,
            description: "validate".to_string(),
            inputs,
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await.unwrap();
        assert_eq!(result["passed"], true);
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_validation_empty_target_no_expected() {
        let executor = DefaultTaskExecutorImpl;
        let mut inputs = HashMap::new();
        inputs.insert("target".to_string(), serde_json::json!(""));
        let context = TaskContext {
            task_id: "v4".to_string(),
            task_type: TaskType::Validation,
            description: "validate".to_string(),
            inputs,
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await.unwrap();
        assert_eq!(result["passed"], false);
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_tool_call_missing_name() {
        let executor = DefaultTaskExecutorImpl;
        let context = TaskContext {
            task_id: "tc1".to_string(),
            task_type: TaskType::ToolCall,
            description: "call tool".to_string(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskExecutorError::ExecutionFailed(_)));
    }

    #[tokio::test]
    async fn test_default_task_executor_impl_tool_call_empty_name() {
        let executor = DefaultTaskExecutorImpl;
        let mut inputs = HashMap::new();
        inputs.insert("tool_name".to_string(), serde_json::json!(""));
        let context = TaskContext {
            task_id: "tc2".to_string(),
            task_type: TaskType::ToolCall,
            description: "call tool".to_string(),
            inputs,
            outputs: HashMap::new(),
        };
        let result = executor.execute_task(&context).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskExecutorError::ExecutionFailed(_)));
    }

    #[test]
    fn test_execution_event_started() {
        let event = ExecutionEvent::Started;
        assert!(matches!(event, ExecutionEvent::Started));
    }

    #[test]
    fn test_execution_event_task_started() {
        let event = ExecutionEvent::TaskStarted("t1".to_string());
        assert!(matches!(event, ExecutionEvent::TaskStarted(_)));
    }

    #[test]
    fn test_execution_event_task_completed() {
        let event = ExecutionEvent::TaskCompleted("t1".to_string());
        assert!(matches!(event, ExecutionEvent::TaskCompleted(_)));
    }

    #[test]
    fn test_execution_event_task_failed() {
        let event = ExecutionEvent::TaskFailed("t1".to_string(), "error".to_string());
        assert!(matches!(event, ExecutionEvent::TaskFailed(_, _)));
    }

    #[test]
    fn test_execution_event_progress() {
        let progress = ExecutionProgress {
            total_tasks: 3,
            completed_tasks: 1,
            failed_tasks: 0,
            current_tasks: vec!["t2".to_string()],
            percentage: 33.33,
        };
        let event = ExecutionEvent::Progress(progress);
        assert!(matches!(event, ExecutionEvent::Progress(_)));
    }

    #[test]
    fn test_execution_event_completed() {
        let event = ExecutionEvent::Completed;
        assert!(matches!(event, ExecutionEvent::Completed));
    }

    #[test]
    fn test_execution_event_failed() {
        let event = ExecutionEvent::Failed("error msg".to_string());
        assert!(matches!(event, ExecutionEvent::Failed(_)));
    }

    #[tokio::test]
    async fn test_task_executor_with_decomposer() {
        let decomposer = crate::task_decomposer::TaskDecomposer::new();
        let executor = TaskExecutor::new().with_decomposer(decomposer);
        assert!(executor.get_graph().await.is_none());
    }

    #[tokio::test]
    async fn test_task_executor_execute_with_groups_not_prepared() {
        let executor = TaskExecutor::new();
        let result = executor.execute_with_groups().await;
        assert!(matches!(result, Err(ExecutionError::NotPrepared)));
    }

    #[tokio::test]
    async fn test_task_executor_with_inner_executor() {
        let executor = TaskExecutor::new().with_inner_executor(DefaultTaskExecutorImpl);
        assert!(executor.get_graph().await.is_none());
    }
}
