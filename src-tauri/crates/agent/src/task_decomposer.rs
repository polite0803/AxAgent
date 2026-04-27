use crate::task::{TaskGraph, TaskNode, TaskType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum DecompositionError {
    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid task structure: {0}")]
    InvalidStructure(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResult {
    pub tasks: Vec<TaskNode>,
    pub parallel_groups: Vec<Vec<String>>,
    pub reasoning: String,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, DecompositionError>;
}

pub struct TaskDecomposer {
    max_depth: usize,
    llm_client: Option<Arc<dyn LlmClient>>,
}

impl TaskDecomposer {
    pub fn new() -> Self {
        Self {
            max_depth: 10,
            llm_client: None,
        }
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_llm_client(mut self, client: Arc<dyn LlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    pub fn decompose(&self, user_input: &str) -> Result<TaskGraph, DecompositionError> {
        let parsed = self.call_llm_decompose(user_input)?;
        self.build_graph(parsed)
    }

    fn call_llm_decompose(
        &self,
        user_input: &str,
    ) -> Result<DecompositionResult, DecompositionError> {
        let prompt = format!(
            r#"你是一个任务分解专家。将以下复杂任务分解为可执行的子任务。

规则：
1. 每个子任务应该是原子的、明确的
2. 标注任务间的依赖关系
3. 识别可以并行执行的任务
4. 包含验证步骤确保任务正确完成

输入: {}

输出格式（JSON）:
{{
  "tasks": [
 {{
      "id": "1",
      "description": "...",
      "type": "tool_call|reasoning|query|validation",
      "dependencies": []
    }}
  ],
  "parallel_groups": [[1, 2], [3], [4, 5]],
  "reasoning": "分解理由..."
}}"#,
            user_input
        );

        let response = self.execute_llm(&prompt)?;
        self.parse_response(&response)
    }

    fn execute_llm(&self, prompt: &str) -> Result<String, DecompositionError> {
        if let Some(ref client) = self.llm_client {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async { client.complete(prompt).await })
        } else {
            Ok(format!(
                "Task decomposition for: {}",
                truncate_string(prompt, 100)
            ))
        }
    }

    fn parse_response(&self, response: &str) -> Result<DecompositionResult, DecompositionError> {
        let response = response.trim();

        if response.starts_with('{') {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response) {
                return self.parse_json_value(&parsed);
            }
        }

        self.parse_fallback_response(response)
    }

    fn parse_json_value(
        &self,
        value: &serde_json::Value,
    ) -> Result<DecompositionResult, DecompositionError> {
        let tasks_array = value
            .get("tasks")
            .and_then(|t| t.as_array())
            .ok_or_else(|| DecompositionError::ParseError("Missing 'tasks' array".to_string()))?;

        let mut tasks = Vec::new();
        for (idx, task_val) in tasks_array.iter().enumerate() {
            let id = task_val
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&idx.to_string())
                .to_string();

            let description = task_val
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let type_str = task_val
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("query");

            let task_type = TaskType::from_str(type_str).unwrap_or(TaskType::Query);

            let dependencies = task_val
                .get("dependencies")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let mut task = TaskNode::new(id, description, task_type);
            task.dependencies = dependencies;
            tasks.push(task);
        }

        let parallel_groups = value
            .get("parallel_groups")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|group| {
                        group.as_array().map(|g| {
                            g.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                    })
                    .collect()
            })
            .unwrap_or_else(|| self.infer_parallel_groups(&tasks));

        let reasoning = value
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(DecompositionResult {
            tasks,
            parallel_groups,
            reasoning,
        })
    }

    fn parse_fallback_response(
        &self,
        response: &str,
    ) -> Result<DecompositionResult, DecompositionError> {
        let lines: Vec<&str> = response.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.len() <= 1 {
            return Err(DecompositionError::ParseError(
                "Response too short to parse".to_string(),
            ));
        }

        let tasks: Vec<TaskNode> = lines
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                let description = line
                    .trim()
                    .trim_matches(|c| c == '-' || c == '*' || c == '•' || c == '→');
                TaskNode::new((idx + 1).to_string(), description.trim(), TaskType::Query)
            })
            .collect();

        let parallel_groups = self.infer_parallel_groups(&tasks);

        Ok(DecompositionResult {
            tasks,
            parallel_groups,
            reasoning: "Simple line-by-line decomposition".to_string(),
        })
    }

    fn infer_parallel_groups(&self, tasks: &[TaskNode]) -> Vec<Vec<String>> {
        if tasks.is_empty() {
            return Vec::new();
        }

        let mut groups = Vec::new();
        let mut current_group = Vec::new();

        for task in tasks {
            if task.dependencies.is_empty() {
                current_group.push(task.id.clone());
            } else {
                if !current_group.is_empty() {
                    groups.push(current_group);
                    current_group = Vec::new();
                }
                groups.push(vec![task.id.clone()]);
            }
        }

        if !current_group.is_empty() {
            groups.push(current_group);
        }

        groups
    }

    pub fn build_graph(
        &self,
        result: DecompositionResult,
    ) -> Result<TaskGraph, DecompositionError> {
        if result.tasks.is_empty() {
            return Err(DecompositionError::InvalidStructure(
                "No tasks provided".to_string(),
            ));
        }

        let mut graph = TaskGraph::new();

        for task in result.tasks {
            if graph.tasks.len() >= self.max_depth {
                break;
            }
            graph.add_task(task);
        }

        graph.parallel_groups = result.parallel_groups;

        Ok(graph)
    }

    pub fn validate_graph(&self, graph: &TaskGraph) -> Result<(), DecompositionError> {
        let task_ids: std::collections::HashSet<_> =
            graph.tasks.iter().map(|t| t.id.clone()).collect();

        for task in &graph.tasks {
            for dep in &task.dependencies {
                if !task_ids.contains(dep) {
                    return Err(DecompositionError::InvalidStructure(format!(
                        "Task '{}' depends on non-existent task '{}'",
                        task.id, dep
                    )));
                }
            }
        }

        if self.has_cycle(graph) {
            return Err(DecompositionError::InvalidStructure(
                "Task graph contains cycle".to_string(),
            ));
        }

        Ok(())
    }

    fn has_cycle(&self, graph: &TaskGraph) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut recursion_stack = std::collections::HashSet::new();

        for task in &graph.tasks {
            if self.visit(graph, &task.id, &mut visited, &mut recursion_stack) {
                return true;
            }
        }

        false
    }

    fn visit(
        &self,
        graph: &TaskGraph,
        task_id: &str,
        visited: &mut std::collections::HashSet<String>,
        recursion_stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        if recursion_stack.contains(task_id) {
            return true;
        }

        if visited.contains(task_id) {
            return false;
        }

        visited.insert(task_id.to_string());
        recursion_stack.insert(task_id.to_string());

        if let Some(task) = graph.get_task(task_id) {
            for dep in &task.dependencies {
                if self.visit(graph, dep, visited, recursion_stack) {
                    return true;
                }
            }
        }

        recursion_stack.remove(task_id);
        false
    }
}

impl Default for TaskDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
