use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    ToolCall,
    Reasoning,
    Query,
    Validation,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::ToolCall => "tool_call",
            TaskType::Reasoning => "reasoning",
            TaskType::Query => "query",
            TaskType::Validation => "validation",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "tool_call" => Some(TaskType::ToolCall),
            "reasoning" => Some(TaskType::Reasoning),
            "query" => Some(TaskType::Query),
            "validation" => Some(TaskType::Validation),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub description: String,
    pub task_type: TaskType,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl TaskNode {
    pub fn new(id: impl Into<String>, description: impl Into<String>, task_type: TaskType) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            task_type,
            dependencies: Vec::new(),
            status: TaskStatus::Pending,
            result: None,
            error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn complete(&mut self, result: serde_json::Value) {
        self.status = TaskStatus::Completed;
        self.result = Some(result);
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = TaskStatus::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn skip(&mut self) {
        self.status = TaskStatus::Skipped;
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn is_ready(&self) -> bool {
        self.status == TaskStatus::Pending
    }

    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Skipped
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraph {
    pub tasks: Vec<TaskNode>,
    pub parallel_groups: Vec<Vec<String>>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            parallel_groups: Vec::new(),
        }
    }

    pub fn add_task(&mut self, task: TaskNode) {
        self.tasks.push(task);
    }

    pub fn get_task(&self, id: &str) -> Option<&TaskNode> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut TaskNode> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn get_ready_tasks(&self) -> Vec<&TaskNode> {
        self.tasks
            .iter()
            .filter(|t| {
                t.is_ready()
                    && t.dependencies.iter().all(|dep_id| {
                        self.get_task(dep_id)
                            .map(|t| t.is_complete())
                            .unwrap_or(false)
                    })
            })
            .collect()
    }

    pub fn all_complete(&self) -> bool {
        self.tasks.iter().all(|t| t.is_complete())
    }

    pub fn has_failures(&self) -> bool {
        self.tasks.iter().any(|t| t.status == TaskStatus::Failed)
    }

    pub fn completion_percentage(&self) -> f32 {
        if self.tasks.is_empty() {
            return 100.0;
        }
        let completed = self.tasks.iter().filter(|t| t.is_complete()).count() as f32;
        (completed / self.tasks.len() as f32) * 100.0
    }
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}
