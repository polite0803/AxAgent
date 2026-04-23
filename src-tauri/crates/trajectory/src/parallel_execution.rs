//! Parallel Execution Module - Multi-agent parallel task execution and result aggregation
//!
//! This module provides infrastructure for executing multiple independent tasks in parallel:
//! - Parallel task dispatch and execution
//! - Result aggregation and presentation
//! - Execution status tracking
//! - Timeout and error handling

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_prompt: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub agent_id: Option<String>,
}

impl ParallelTask {
    pub fn new(name: String, description: String, task_prompt: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            task_prompt,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            progress: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            agent_id: None,
        }
    }

    pub fn start(&mut self, agent_id: String) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
        self.agent_id = Some(agent_id);
    }

    pub fn complete(&mut self, result: String) {
        self.status = TaskStatus::Completed;
        self.result = Some(result);
        self.completed_at = Some(Utc::now());
        self.progress = 1.0;
    }

    pub fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(Utc::now());
    }

    pub fn update_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.completed_at
            .and_then(|completed| {
                self.started_at
                    .map(|started| (completed - started).num_milliseconds() as u64)
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecution {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tasks: Vec<ParallelTask>,
    pub status: ExecutionStatus,
    pub strategy: ExecutionStrategy,
    pub max_parallel: usize,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub aggregated_result: Option<String>,
}

impl ParallelExecution {
    pub fn new(name: String, description: String, strategy: ExecutionStrategy, max_parallel: usize) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            tasks: Vec::new(),
            status: ExecutionStatus::Pending,
            strategy,
            max_parallel,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            aggregated_result: None,
        }
    }

    pub fn add_task(&mut self, task: ParallelTask) {
        self.tasks.push(task);
    }

    pub fn add_tasks(&mut self, tasks: Vec<ParallelTask>) {
        self.tasks.extend(tasks);
    }

    pub fn start(&mut self) {
        self.status = ExecutionStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn is_complete(&self) -> bool {
        self.tasks.iter().all(|t| {
            t.status == TaskStatus::Completed
                || t.status == TaskStatus::Failed
                || t.status == TaskStatus::Cancelled
                || t.status == TaskStatus::Timeout
        })
    }

    pub fn completed_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count()
    }

    pub fn failed_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Failed).count()
    }

    pub fn running_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Running).count()
    }

    pub fn pending_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Pending).count()
    }

    pub fn overall_progress(&self) -> f32 {
        if self.tasks.is_empty() {
            return 0.0;
        }
        let total: f32 = self.tasks.iter().map(|t| t.progress).sum();
        total / self.tasks.len() as f32
    }

    pub fn aggregate_results(&mut self) -> String {
        let mut lines = vec![
            format!("# {} - 执行汇总\n", self.name),
            format!("总任务数: {}\n", self.tasks.len()),
            format!("成功: {}, 失败: {}\n", self.completed_count(), self.failed_count()),
            format!("执行时间: {} ms\n", self.duration_ms().unwrap_or(0)),
            "\n## 任务结果:\n".to_string(),
        ];

        for (i, task) in self.tasks.iter().enumerate() {
            lines.push(format!("\n### {}. {} [{}]", i + 1, task.name, format_status(&task.status)));

            if let Some(ref result) = task.result {
                lines.push(format!("\n结果:\n{}\n", result));
            }
            if let Some(ref error) = task.error {
                lines.push(format!("\n错误:\n{}\n", error));
            }
            if let Some(ms) = task.duration_ms() {
                lines.push(format!("耗时: {} ms\n", ms));
            }
        }

        let aggregated = lines.join("");
        self.aggregated_result = Some(aggregated.clone());
        aggregated
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.completed_at.and_then(|completed| {
            self.started_at.map(|started| (completed - started).num_milliseconds() as u64)
        })
    }
}

fn format_status(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "⏳ 等待中",
        TaskStatus::Running => "🔄 运行中",
        TaskStatus::Completed => "✅ 完成",
        TaskStatus::Failed => "❌ 失败",
        TaskStatus::Cancelled => "🚫 已取消",
        TaskStatus::Timeout => "⏱️ 超时",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStrategy {
    Sequential,
    Parallel,
    PriorityBased,
}

impl Default for ExecutionStrategy {
    fn default() -> Self {
        ExecutionStrategy::Parallel
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub total_tasks: usize,
    pub completed: usize,
    pub failed: usize,
    pub duration_ms: u64,
    pub aggregated_summary: String,
    pub task_results: Vec<TaskResultSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResultSummary {
    pub task_id: String,
    pub task_name: String,
    pub status: TaskStatus,
    pub result_preview: Option<String>,
    pub error_preview: Option<String>,
    pub duration_ms: Option<u64>,
}

impl From<&ParallelTask> for TaskResultSummary {
    fn from(task: &ParallelTask) -> Self {
        Self {
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            status: task.status,
            result_preview: task.result.as_ref().map(|r| {
                if r.len() > 200 {
                    format!("{}...", &r[..200])
                } else {
                    r.clone()
                }
            }),
            error_preview: task.error.as_ref().map(|e| {
                if e.len() > 200 {
                    format!("{}...", &e[..200])
                } else {
                    e.clone()
                }
            }),
            duration_ms: task.duration_ms(),
        }
    }
}

pub struct ParallelExecutionService {
    executions: Arc<RwLock<HashMap<String, ParallelExecution>>>,
    max_executions: usize,
}

impl Default for ParallelExecutionService {
    fn default() -> Self {
        Self::new(10)
    }
}

impl ParallelExecutionService {
    pub fn new(max_executions: usize) -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            max_executions,
        }
    }

    pub async fn create_execution(
        &self,
        name: String,
        description: String,
        tasks: Vec<(String, String, String)>,
        strategy: ExecutionStrategy,
        max_parallel: usize,
    ) -> Result<String> {
        let mut execution = ParallelExecution::new(name, description, strategy, max_parallel);

        for (task_name, task_desc, task_prompt) in tasks {
            let task = ParallelTask::new(task_name, task_desc, task_prompt);
            execution.add_task(task);
        }

        let exec_id = execution.id.clone();

        let mut executions = self.executions.write().unwrap();
        if executions.len() >= self.max_executions {
            if let Some(oldest) = executions.keys().next().cloned() {
                executions.remove(&oldest);
            }
        }
        executions.insert(exec_id.clone(), execution);

        Ok(exec_id)
    }

    pub async fn get_execution(&self, id: &str) -> Option<ParallelExecution> {
        let executions = self.executions.read().unwrap();
        executions.get(id).cloned()
    }

    pub async fn list_executions(&self) -> Vec<ParallelExecution> {
        let executions = self.executions.read().unwrap();
        executions.values().cloned().collect()
    }

    pub async fn get_next_pending_task(&self, execution_id: &str) -> Option<ParallelTask> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;

        let strategy = execution.strategy;
        let max_parallel = execution.max_parallel;
        let running = execution.running_count();

        if running >= max_parallel {
            return None;
        }

        match strategy {
            ExecutionStrategy::Sequential => {
                execution.tasks.iter_mut().find(|t| t.status == TaskStatus::Pending).map(|t| {
                    t.start(Uuid::new_v4().to_string());
                    t.clone()
                })
            }
            ExecutionStrategy::Parallel => {
                execution.tasks.iter_mut().find(|t| t.status == TaskStatus::Pending).map(|t| {
                    t.start(Uuid::new_v4().to_string());
                    t.clone()
                })
            }
            ExecutionStrategy::PriorityBased => {
                execution.tasks.iter_mut().find(|t| t.status == TaskStatus::Pending).map(|t| {
                    t.start(Uuid::new_v4().to_string());
                    t.clone()
                })
            }
        }
    }

    pub async fn update_task_result(&self, execution_id: &str, task_id: &str, result: String) -> Option<()> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;

        let task = execution.tasks.iter_mut().find(|t| t.id == task_id)?;
        task.complete(result);

        if execution.is_complete() {
            execution.status = if execution.failed_count() == 0 {
                ExecutionStatus::Completed
            } else {
                ExecutionStatus::Failed
            };
            execution.completed_at = Some(Utc::now());
            execution.aggregate_results();
        }

        Some(())
    }

    pub async fn update_task_error(&self, execution_id: &str, task_id: &str, error: String) -> Option<()> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;

        let task = execution.tasks.iter_mut().find(|t| t.id == task_id)?;
        task.fail(error);

        if execution.is_complete() {
            execution.status = ExecutionStatus::Failed;
            execution.completed_at = Some(Utc::now());
            execution.aggregate_results();
        }

        Some(())
    }

    pub async fn cancel_execution(&self, execution_id: &str) -> Option<()> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;

        for task in &mut execution.tasks {
            if task.status == TaskStatus::Pending || task.status == TaskStatus::Running {
                task.status = TaskStatus::Cancelled;
                task.completed_at = Some(Utc::now());
            }
        }

        execution.status = ExecutionStatus::Cancelled;
        execution.completed_at = Some(Utc::now());
        execution.aggregate_results();

        Some(())
    }

    pub async fn get_execution_result(&self, execution_id: &str) -> Option<ExecutionResult> {
        let executions = self.executions.read().unwrap();
        let execution = executions.get(execution_id)?;

        Some(ExecutionResult {
            execution_id: execution.id.clone(),
            status: execution.status,
            total_tasks: execution.tasks.len(),
            completed: execution.completed_count(),
            failed: execution.failed_count(),
            duration_ms: execution.duration_ms().unwrap_or(0),
            aggregated_summary: execution.aggregated_result.clone().unwrap_or_default(),
            task_results: execution.tasks.iter().map(TaskResultSummary::from).collect(),
        })
    }

    pub async fn delete_execution(&self, execution_id: &str) -> bool {
        let mut executions = self.executions.write().unwrap();
        executions.remove(execution_id).is_some()
    }

    pub async fn start_execution(&self, execution_id: &str) -> Option<()> {
        let mut executions = self.executions.write().unwrap();
        let execution = executions.get_mut(execution_id)?;
        execution.start();
        Some(())
    }
}
