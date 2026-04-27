use crate::task::{TaskGraph, TaskStatus};
use crate::task_decomposer::{DecompositionError, TaskDecomposer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

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
}

impl TaskExecutor {
    pub fn new() -> Self {
        let decomposer = Arc::new(TaskDecomposer::new());
        let (event_sender, _) = broadcast::channel(100);

        Self {
            decomposer,
            graph: Arc::new(RwLock::new(None)),
            event_sender,
        }
    }

    pub fn with_decomposer(mut self, decomposer: TaskDecomposer) -> Self {
        self.decomposer = Arc::new(decomposer);
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
        let graph = graph_guard.clone().ok_or(ExecutionError::NotPrepared)?;
        drop(graph_guard);

        self.emit(ExecutionEvent::Started);

        let mut executor = tokio::task::JoinSet::new();

        loop {
            let ready_tasks = graph.get_ready_tasks();

            if ready_tasks.is_empty() {
                if graph.all_complete() {
                    break;
                }
                if graph.has_failures() {
                    let failed_ids: Vec<_> = graph
                        .tasks
                        .iter()
                        .filter(|t| t.status == TaskStatus::Failed)
                        .map(|t| t.id.clone())
                        .collect();
                    self.emit(ExecutionEvent::Failed(format!(
                        "Tasks failed: {}",
                        failed_ids.join(", ")
                    )));
                    *self.graph.write().await = Some(graph);
                    return Err(ExecutionError::TaskFailed(failed_ids));
                }
                break;
            }

            for task in ready_tasks {
                let task_id = task.id.clone();
                let graph_arc = Arc::clone(&self.graph);

                executor.spawn(async move {
                    let mut guard = graph_arc.write().await;
                    if let Some(g) = guard.as_mut() {
                        if let Some(t) = g.get_task_mut(&task_id) {
                            t.start();
                        }
                    }
                    drop(guard);

                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    let mut guard = graph_arc.write().await;
                    if let Some(g) = guard.as_mut() {
                        if let Some(t) = g.get_task_mut(&task_id) {
                            t.complete(serde_json::json!({ "output": "Task completed" }));
                        }
                    }
                });
            }

            while let Some(result) = executor.join_next().await {
                if result.is_err() {
                    continue;
                }
            }

            let guard = self.graph.read().await;
            if let Some(g) = guard.as_ref() {
                let progress = ExecutionProgress::new(g);
                self.emit(ExecutionEvent::Progress(progress.clone()));

                for task in &g.tasks {
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
            }
        }

        *self.graph.write().await = Some(graph.clone());

        if graph.has_failures() {
            self.emit(ExecutionEvent::Failed("Some tasks failed".to_string()));
        } else {
            self.emit(ExecutionEvent::Completed);
        }

        Ok(graph)
    }

    pub async fn execute_with_groups(&self) -> Result<TaskGraph, ExecutionError> {
        let graph_guard = self.graph.read().await;
        let graph = graph_guard.clone().ok_or(ExecutionError::NotPrepared)?;
        drop(graph_guard);

        self.emit(ExecutionEvent::Started);

        let mut group_idx = 0;

        while group_idx < graph.parallel_groups.len() {
            let group = &graph.parallel_groups[group_idx];
            let mut handles = Vec::new();

            for task_id in group {
                if let Some(task) = graph.get_task(task_id) {
                    if !task.is_ready() {
                        continue;
                    }

                    let task_id_clone = task_id.clone();
                    let graph_arc = Arc::clone(&self.graph);

                    let handle = tokio::spawn(async move {
                        {
                            let mut guard = graph_arc.write().await;
                            if let Some(g) = guard.as_mut() {
                                if let Some(t) = g.get_task_mut(&task_id_clone) {
                                    t.start();
                                }
                            }
                        }

                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                        {
                            let mut guard = graph_arc.write().await;
                            if let Some(g) = guard.as_mut() {
                                if let Some(t) = g.get_task_mut(&task_id_clone) {
                                    t.complete(serde_json::json!({ "output": "completed" }));
                                }
                            }
                        }

                        task_id_clone
                    });

                    handles.push(handle);
                }
            }

            for handle in handles {
                if let Ok(task_id) = handle.await {
                    self.emit(ExecutionEvent::TaskCompleted(task_id));
                }
            }

            let guard = self.graph.read().await;
            if let Some(g) = guard.as_ref() {
                let progress = ExecutionProgress::new(g);
                self.emit(ExecutionEvent::Progress(progress));
            }

            group_idx += 1;
        }

        *self.graph.write().await = Some(graph.clone());

        if graph.has_failures() {
            self.emit(ExecutionEvent::Failed("Some tasks failed".to_string()));
        } else {
            self.emit(ExecutionEvent::Completed);
        }

        Ok(graph)
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
