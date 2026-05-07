use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
        matches!(self.status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Skipped)
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

    pub fn topological_sort(&self) -> Result<Vec<Vec<String>>, TopologicalSortError> {
        let mut result = Vec::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut visited: HashSet<String> = HashSet::new();

        for task in &self.tasks {
            in_degree.insert(task.id.clone(), task.dependencies.len());
        }

        while visited.len() < self.tasks.len() {
            let batch: Vec<String> = in_degree
                .iter()
                .filter(|(id, &degree)| degree == 0 && !visited.contains(*id))
                .map(|(id, _)| id.clone())
                .collect();

            if batch.is_empty() && visited.len() < self.tasks.len() {
                let remaining: Vec<String> = self
                    .tasks
                    .iter()
                    .filter(|t| !visited.contains(&t.id))
                    .map(|t| t.id.clone())
                    .collect();
                return Err(TopologicalSortError::CircularDependency(remaining));
            }

            if !batch.is_empty() {
                result.push(batch.clone());
            }

            for task_id in &batch {
                visited.insert(task_id.clone());
                for task in &self.tasks {
                    if task.dependencies.contains(task_id) {
                        if let Some(degree) = in_degree.get_mut(&task.id) {
                            *degree -= 1;
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn dependencies_ready(&self, task_id: &str) -> bool {
        if let Some(task) = self.get_task(task_id) {
            task.dependencies.iter().all(|dep| self.is_completed(dep))
        } else {
            false
        }
    }

    pub fn is_completed(&self, task_id: &str) -> bool {
        self.get_task(task_id)
            .map(|t| t.is_complete())
            .unwrap_or(false)
    }

    pub fn get_failed_task_ids(&self) -> Vec<String> {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .map(|t| t.id.clone())
            .collect()
    }

    pub fn get_status_summary(&self) -> TaskStatusSummary {
        let total = self.tasks.len();
        let pending = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count();
        let running = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .count();
        let completed = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();
        let skipped = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Skipped)
            .count();

        TaskStatusSummary {
            total,
            pending,
            running,
            completed,
            failed,
            skipped,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskStatusSummary {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TopologicalSortError {
    #[error("Circular dependency detected involving tasks: {0:?}")]
    CircularDependency(Vec<String>),
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_as_str() {
        assert_eq!(TaskType::ToolCall.as_str(), "tool_call");
        assert_eq!(TaskType::Reasoning.as_str(), "reasoning");
        assert_eq!(TaskType::Query.as_str(), "query");
        assert_eq!(TaskType::Validation.as_str(), "validation");
    }

    #[test]
    fn test_task_type_from_str() {
        assert_eq!(TaskType::from_str("tool_call"), Some(TaskType::ToolCall));
        assert_eq!(TaskType::from_str("reasoning"), Some(TaskType::Reasoning));
        assert_eq!(TaskType::from_str("query"), Some(TaskType::Query));
        assert_eq!(TaskType::from_str("validation"), Some(TaskType::Validation));
        assert_eq!(TaskType::from_str("unknown"), None);
    }

    #[test]
    fn test_task_status_as_str() {
        assert_eq!(TaskStatus::Pending.as_str(), "pending");
        assert_eq!(TaskStatus::Running.as_str(), "running");
        assert_eq!(TaskStatus::Completed.as_str(), "completed");
        assert_eq!(TaskStatus::Failed.as_str(), "failed");
        assert_eq!(TaskStatus::Skipped.as_str(), "skipped");
    }

    #[test]
    fn test_task_node_new() {
        let node = TaskNode::new("t1", "Test task", TaskType::ToolCall);
        assert_eq!(node.id, "t1");
        assert_eq!(node.description, "Test task");
        assert_eq!(node.task_type, TaskType::ToolCall);
        assert!(node.dependencies.is_empty());
        assert_eq!(node.status, TaskStatus::Pending);
        assert!(node.result.is_none());
        assert!(node.error.is_none());
    }

    #[test]
    fn test_task_node_with_dependencies() {
        let node = TaskNode::new("t2", "Task 2", TaskType::Reasoning)
            .with_dependencies(vec!["t1".to_string()]);
        assert_eq!(node.dependencies.len(), 1);
        assert_eq!(node.dependencies[0], "t1");
    }

    #[test]
    fn test_task_node_start() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        node.start();
        assert_eq!(node.status, TaskStatus::Running);
        assert!(node.started_at.is_some());
    }

    #[test]
    fn test_task_node_complete() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        node.start();
        node.complete(serde_json::json!("result"));
        assert_eq!(node.status, TaskStatus::Completed);
        assert!(node.result.is_some());
        assert!(node.completed_at.is_some());
    }

    #[test]
    fn test_task_node_fail() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        node.start();
        node.fail("something went wrong");
        assert_eq!(node.status, TaskStatus::Failed);
        assert_eq!(node.error, Some("something went wrong".to_string()));
        assert!(node.completed_at.is_some());
    }

    #[test]
    fn test_task_node_skip() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        node.skip();
        assert_eq!(node.status, TaskStatus::Skipped);
        assert!(node.completed_at.is_some());
    }

    #[test]
    fn test_task_node_is_ready() {
        let node = TaskNode::new("t1", "Task", TaskType::Query);
        assert!(node.is_ready());
        let mut node2 = TaskNode::new("t2", "Task", TaskType::Query);
        node2.start();
        assert!(!node2.is_ready());
    }

    #[test]
    fn test_task_node_is_complete() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        assert!(!node.is_complete());
        node.complete(serde_json::json!(42));
        assert!(node.is_complete());
    }

    #[test]
    fn test_task_graph_new() {
        let graph = TaskGraph::new();
        assert!(graph.tasks.is_empty());
        assert!(graph.parallel_groups.is_empty());
    }

    #[test]
    fn test_task_graph_add_and_get_task() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning));
        assert_eq!(graph.tasks.len(), 2);
        assert!(graph.get_task("t1").is_some());
        assert!(graph.get_task("t2").is_some());
        assert!(graph.get_task("t3").is_none());
    }

    #[test]
    fn test_task_graph_get_task_mut() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        if let Some(task) = graph.get_task_mut("t1") {
            task.start();
        }
        assert_eq!(graph.get_task("t1").unwrap().status, TaskStatus::Running);
    }

    #[test]
    fn test_task_graph_get_ready_tasks() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning)
            .with_dependencies(vec!["t1".to_string()]));
        let ready = graph.get_ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "t1");
    }

    #[test]
    fn test_task_graph_get_ready_tasks_with_completed_deps() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning)
            .with_dependencies(vec!["t1".to_string()]));
        let ready = graph.get_ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "t2");
    }

    #[test]
    fn test_task_graph_all_complete() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        assert!(graph.all_complete());
    }

    #[test]
    fn test_task_graph_not_all_complete() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        assert!(!graph.all_complete());
    }

    #[test]
    fn test_task_graph_has_failures() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.fail("error");
        graph.add_task(t1);
        assert!(graph.has_failures());
    }

    #[test]
    fn test_task_graph_no_failures() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        assert!(!graph.has_failures());
    }

    #[test]
    fn test_task_graph_completion_percentage_empty() {
        let graph = TaskGraph::new();
        assert_eq!(graph.completion_percentage(), 100.0);
    }

    #[test]
    fn test_task_graph_completion_percentage() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning));
        assert_eq!(graph.completion_percentage(), 50.0);
    }

    #[test]
    fn test_task_graph_topological_sort_simple() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning)
            .with_dependencies(vec!["t1".to_string()]));
        let result = graph.topological_sort().unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].contains(&"t1".to_string()));
        assert!(result[1].contains(&"t2".to_string()));
    }

    #[test]
    fn test_task_graph_topological_sort_circular() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall)
            .with_dependencies(vec!["t2".to_string()]));
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning)
            .with_dependencies(vec!["t1".to_string()]));
        let result = graph.topological_sort();
        assert!(matches!(result, Err(TopologicalSortError::CircularDependency(_))));
    }

    #[test]
    fn test_task_graph_dependencies_ready() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning)
            .with_dependencies(vec!["t1".to_string()]));
        assert!(graph.dependencies_ready("t2"));
        assert!(!graph.dependencies_ready("nonexistent"));
    }

    #[test]
    fn test_task_graph_is_completed() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        assert!(graph.is_completed("t1"));
        assert!(!graph.is_completed("nonexistent"));
    }

    #[test]
    fn test_task_graph_get_failed_task_ids() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.fail("error");
        graph.add_task(t1);
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning));
        let failed = graph.get_failed_task_ids();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0], "t1");
    }

    #[test]
    fn test_task_graph_get_status_summary() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        let mut t2 = TaskNode::new("t2", "Task 2", TaskType::Reasoning);
        t2.fail("error");
        graph.add_task(t1);
        graph.add_task(t2);
        graph.add_task(TaskNode::new("t3", "Task 3", TaskType::Query));
        let summary = graph.get_status_summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.pending, 1);
    }

    #[test]
    fn test_topological_sort_error_display() {
        let err = TopologicalSortError::CircularDependency(vec!["t1".to_string(), "t2".to_string()]);
        let msg = format!("{}", err);
        assert!(msg.contains("Circular dependency"));
    }
}
