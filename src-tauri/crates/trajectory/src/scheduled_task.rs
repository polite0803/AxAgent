//! Scheduled Tasks Module - Timed and automated task execution
//!
//! This module provides infrastructure for scheduling and automating tasks:
//! - Cron-style scheduled tasks
//! - One-time delayed tasks
//! - Recurring task management
//! - Task persistence and recovery

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_type: TaskType,
    pub cron_expression: Option<String>,
    pub interval_seconds: Option<u64>,
    pub next_run_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_result: Option<TaskRunResult>,
    pub status: ScheduledTaskStatus,
    pub config: TaskConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScheduledTask {
    pub fn new(
        name: String,
        description: String,
        task_type: TaskType,
        next_run_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            task_type,
            cron_expression: None,
            interval_seconds: None,
            next_run_at,
            last_run_at: None,
            last_result: None,
            status: ScheduledTaskStatus::Active,
            config: TaskConfig::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_cron(mut self, cron: String) -> Self {
        self.cron_expression = Some(cron);
        self
    }

    pub fn with_interval(mut self, seconds: u64) -> Self {
        self.interval_seconds = Some(seconds);
        self
    }

    pub fn calculate_next_run(&mut self) {
        let now = Utc::now();
        if let Some(interval) = self.interval_seconds {
            self.next_run_at = self.last_run_at
                .unwrap_or(now)
                + chrono::Duration::seconds(interval as i64);
        } else {
            self.next_run_at = now + chrono::Duration::hours(24);
        }
    }

    pub fn update_last_run(&mut self, result: TaskRunResult) {
        self.last_run_at = Some(Utc::now());
        self.last_result = Some(result);
        self.calculate_next_run();
        self.updated_at = Utc::now();
    }

    pub fn is_due(&self) -> bool {
        Utc::now() >= self.next_run_at
    }

    pub fn pause(&mut self) {
        self.status = ScheduledTaskStatus::Paused;
        self.updated_at = Utc::now();
    }

    pub fn resume(&mut self) {
        self.status = ScheduledTaskStatus::Active;
        self.calculate_next_run();
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    DailySummary,
    Backup,
    Cleanup,
    Custom,
    HealthCheck,
    DataSync,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::DailySummary => "daily_summary",
            TaskType::Backup => "backup",
            TaskType::Cleanup => "cleanup",
            TaskType::Custom => "custom",
            TaskType::HealthCheck => "health_check",
            TaskType::DataSync => "data_sync",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduledTaskStatus {
    Active,
    Paused,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub timeout_seconds: u64,
    pub retry_on_failure: bool,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
    pub notification_enabled: bool,
    pub run_on_startup: bool,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 3600,
            retry_on_failure: true,
            max_retries: 3,
            retry_delay_seconds: 300,
            notification_enabled: true,
            run_on_startup: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub executed_at: DateTime<Utc>,
}

impl TaskRunResult {
    pub fn success(output: String, duration_ms: u64) -> Self {
        Self {
            success: true,
            output: Some(output),
            error: None,
            duration_ms,
            executed_at: Utc::now(),
        }
    }

    pub fn failure(error: String, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: None,
            error: Some(error),
            duration_ms,
            executed_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub name: String,
    pub task_type: TaskType,
    pub prompt_template: String,
    pub parameters: HashMap<String, String>,
}

impl TaskDefinition {
    pub fn new(name: String, task_type: TaskType, prompt_template: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            task_type,
            prompt_template,
            parameters: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }
}

pub struct ScheduledTaskService {
    tasks: Arc<RwLock<HashMap<String, ScheduledTask>>>,
    task_definitions: Arc<RwLock<HashMap<String, TaskDefinition>>>,
    execution_history: Arc<RwLock<Vec<TaskRunResult>>>,
    max_history_size: usize,
}

impl Default for ScheduledTaskService {
    fn default() -> Self {
        Self::new(100)
    }
}

impl ScheduledTaskService {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            task_definitions: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size,
        }
    }

    pub async fn create_task(&self, mut task: ScheduledTask) -> Result<String> {
        task.calculate_next_run();
        let task_id = task.id.clone();
        let mut tasks = self.tasks.write().unwrap();
        tasks.insert(task_id.clone(), task);
        Ok(task_id)
    }

    pub async fn create_daily_summary_task(
        &self,
        name: String,
        description: String,
        hour: u32,
        minute: u32,
    ) -> Result<String> {
        let tomorrow = (Utc::now().date_naive() + chrono::Days::new(1)).and_hms_opt(hour, minute, 0).unwrap();
        let next_run = DateTime::<Utc>::from_naive_utc_and_offset(tomorrow, Utc);

        let task = ScheduledTask::new(
            name,
            description,
            TaskType::DailySummary,
            next_run,
        );

        self.create_task(task).await
    }

    pub async fn create_backup_task(
        &self,
        name: String,
        description: String,
        interval_hours: u64,
    ) -> Result<String> {
        let next_run = Utc::now() + chrono::Duration::hours(interval_hours as i64);

        let mut task = ScheduledTask::new(
            name,
            description,
            TaskType::Backup,
            next_run,
        );
        task = task.with_interval(interval_hours * 3600);

        self.create_task(task).await
    }

    pub async fn create_cleanup_task(
        &self,
        name: String,
        description: String,
        interval_hours: u64,
    ) -> Result<String> {
        let next_run = Utc::now() + chrono::Duration::hours(interval_hours as i64);

        let task = ScheduledTask::new(
            name,
            description,
            TaskType::Cleanup,
            next_run,
        ).with_interval(interval_hours * 3600);

        self.create_task(task).await
    }

    pub async fn get_task(&self, id: &str) -> Option<ScheduledTask> {
        let tasks = self.tasks.read().unwrap();
        tasks.get(id).cloned()
    }

    pub async fn list_tasks(&self) -> Vec<ScheduledTask> {
        let tasks = self.tasks.read().unwrap();
        tasks.values().cloned().collect()
    }

    pub async fn list_due_tasks(&self) -> Vec<ScheduledTask> {
        let tasks = self.tasks.read().unwrap();
        tasks
            .values()
            .filter(|t| t.status == ScheduledTaskStatus::Active && t.is_due())
            .cloned()
            .collect()
    }

    pub async fn update_task(&self, id: &str, mut task: ScheduledTask) -> Option<()> {
        task.updated_at = Utc::now();
        let mut tasks = self.tasks.write().unwrap();
        if tasks.contains_key(id) {
            tasks.insert(id.to_string(), task);
            Some(())
        } else {
            None
        }
    }

    pub async fn delete_task(&self, id: &str) -> bool {
        let mut tasks = self.tasks.write().unwrap();
        tasks.remove(id).is_some()
    }

    pub async fn pause_task(&self, id: &str) -> Option<()> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.pause();
            Some(())
        } else {
            None
        }
    }

    pub async fn resume_task(&self, id: &str) -> Option<()> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.resume();
            Some(())
        } else {
            None
        }
    }

    pub async fn record_execution(&self, task_id: &str, result: TaskRunResult) {
        {
            let mut tasks = self.tasks.write().unwrap();
            if let Some(task) = tasks.get_mut(task_id) {
                task.update_last_run(result.clone());
            }
        }

        {
            let mut history = self.execution_history.write().unwrap();
            history.push(result);
            if history.len() > self.max_history_size {
                let drain_count = history.len() - self.max_history_size;
                history.drain(0..drain_count);
            }
        }
    }

    pub async fn get_execution_history(&self, limit: Option<usize>) -> Vec<TaskRunResult> {
        let history = self.execution_history.read().unwrap();
        let limit = limit.unwrap_or(self.max_history_size);
        history.iter().rev().take(limit).cloned().collect()
    }

    pub async fn register_task_definition(&self, definition: TaskDefinition) {
        let mut defs = self.task_definitions.write().unwrap();
        defs.insert(definition.id.clone(), definition);
    }

    pub async fn get_task_definition(&self, id: &str) -> Option<TaskDefinition> {
        let defs = self.task_definitions.read().unwrap();
        defs.get(id).cloned()
    }

    pub async fn list_task_definitions(&self) -> Vec<TaskDefinition> {
        let defs = self.task_definitions.read().unwrap();
        defs.values().cloned().collect()
    }

    pub async fn get_next_scheduled_time(&self) -> Option<DateTime<Utc>> {
        let tasks = self.tasks.read().unwrap();
        tasks
            .values()
            .filter(|t| t.status == ScheduledTaskStatus::Active)
            .map(|t| t.next_run_at)
            .min()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummaryConfig {
    pub hour: u32,
    pub minute: u32,
    pub include_trajectories: bool,
    pub include_skills: bool,
    pub include_insights: bool,
    pub include_user_profile: bool,
    pub format: SummaryFormat,
}

impl Default for DailySummaryConfig {
    fn default() -> Self {
        Self {
            hour: 9,
            minute: 0,
            include_trajectories: true,
            include_skills: true,
            include_insights: true,
            include_user_profile: true,
            format: SummaryFormat::Markdown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SummaryFormat {
    Markdown,
    Json,
    PlainText,
}
