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
pub trait DecomposerLlmClient: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, DecompositionError>;
}

pub struct TaskDecomposer {
    max_depth: usize,
    llm_client: Option<Arc<dyn DecomposerLlmClient>>,
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

    pub fn with_llm_client(mut self, client: Arc<dyn DecomposerLlmClient>) -> Self {
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
            Ok(format!("Task decomposition for: {}", truncate_string(prompt, 100)))
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

            let task_type = type_str.parse::<TaskType>().unwrap_or(TaskType::Query);

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
            return Err(DecompositionError::ParseError("Response too short to parse".to_string()));
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
            return Err(DecompositionError::InvalidStructure("No tasks provided".to_string()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{TaskNode, TaskType};
    use async_trait::async_trait;

    struct MockLlmClient {
        response: String,
    }

    #[async_trait]
    impl DecomposerLlmClient for MockLlmClient {
        async fn complete(&self, _prompt: &str) -> Result<String, DecompositionError> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn test_task_decomposer_new() {
        let decomposer = TaskDecomposer::new();
        assert_eq!(decomposer.max_depth, 10);
        assert!(decomposer.llm_client.is_none());
    }

    #[test]
    fn test_task_decomposer_default() {
        let decomposer = TaskDecomposer::default();
        assert_eq!(decomposer.max_depth, 10);
    }

    #[test]
    fn test_task_decomposer_with_max_depth() {
        let decomposer = TaskDecomposer::new().with_max_depth(5);
        assert_eq!(decomposer.max_depth, 5);
    }

    #[test]
    fn test_task_decomposer_with_llm_client() {
        let mock = MockLlmClient {
            response: "{}".to_string(),
        };
        let decomposer = TaskDecomposer::new().with_llm_client(Arc::new(mock));
        assert!(decomposer.llm_client.is_some());
    }

    #[test]
    fn test_decomposition_error_llm_error() {
        let err = DecompositionError::LlmError("llm failed".to_string());
        assert!(err.to_string().contains("llm failed"));
    }

    #[test]
    fn test_decomposition_error_parse_error() {
        let err = DecompositionError::ParseError("parse failed".to_string());
        assert!(err.to_string().contains("parse failed"));
    }

    #[test]
    fn test_decomposition_error_invalid_structure() {
        let err = DecompositionError::InvalidStructure("invalid".to_string());
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn test_decomposition_result_creation() {
        let result = DecompositionResult {
            tasks: vec![TaskNode::new("1", "task 1", TaskType::Query)],
            parallel_groups: vec![vec!["1".to_string()]],
            reasoning: "test reasoning".to_string(),
        };
        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.parallel_groups.len(), 1);
        assert_eq!(result.reasoning, "test reasoning");
    }

    #[test]
    fn test_decomposition_result_empty() {
        let result = DecompositionResult {
            tasks: vec![],
            parallel_groups: vec![],
            reasoning: String::new(),
        };
        assert!(result.tasks.is_empty());
        assert!(result.parallel_groups.is_empty());
    }

    #[test]
    fn test_task_decomposer_build_graph_empty_tasks() {
        let decomposer = TaskDecomposer::new();
        let result = DecompositionResult {
            tasks: vec![],
            parallel_groups: vec![],
            reasoning: String::new(),
        };
        let graph = decomposer.build_graph(result);
        assert!(graph.is_err());
        assert!(matches!(graph.unwrap_err(), DecompositionError::InvalidStructure(_)));
    }

    #[test]
    fn test_task_decomposer_build_graph_with_tasks() {
        let decomposer = TaskDecomposer::new();
        let result = DecompositionResult {
            tasks: vec![
                TaskNode::new("1", "task 1", TaskType::Query),
                TaskNode::new("2", "task 2", TaskType::Reasoning),
            ],
            parallel_groups: vec![vec!["1".to_string(), "2".to_string()]],
            reasoning: "parallel tasks".to_string(),
        };
        let graph = decomposer.build_graph(result).unwrap();
        assert_eq!(graph.tasks.len(), 2);
        assert_eq!(graph.parallel_groups.len(), 1);
    }

    #[test]
    fn test_task_decomposer_build_graph_max_depth() {
        let decomposer = TaskDecomposer::new().with_max_depth(2);
        let tasks: Vec<TaskNode> = (1..=5)
            .map(|i| TaskNode::new(i.to_string(), format!("task {}", i), TaskType::Query))
            .collect();
        let result = DecompositionResult {
            tasks,
            parallel_groups: vec![],
            reasoning: String::new(),
        };
        let graph = decomposer.build_graph(result).unwrap();
        assert_eq!(graph.tasks.len(), 2);
    }

    #[test]
    fn test_task_decomposer_build_graph_max_depth_equals_task_count() {
        let decomposer = TaskDecomposer::new().with_max_depth(3);
        let tasks: Vec<TaskNode> = (1..=3)
            .map(|i| TaskNode::new(i.to_string(), format!("task {}", i), TaskType::Query))
            .collect();
        let result = DecompositionResult {
            tasks,
            parallel_groups: vec![],
            reasoning: String::new(),
        };
        let graph = decomposer.build_graph(result).unwrap();
        assert_eq!(graph.tasks.len(), 3);
    }

    #[test]
    fn test_task_decomposer_validate_graph_valid() {
        let decomposer = TaskDecomposer::new();
        let mut graph = TaskGraph::new();
        let t1 = TaskNode::new("1", "task 1", TaskType::Query);
        let t2 =
            TaskNode::new("2", "task 2", TaskType::Query).with_dependencies(vec!["1".to_string()]);
        graph.add_task(t1);
        graph.add_task(t2);
        assert!(decomposer.validate_graph(&graph).is_ok());
    }

    #[test]
    fn test_task_decomposer_validate_graph_no_deps() {
        let decomposer = TaskDecomposer::new();
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("1", "task 1", TaskType::Query));
        graph.add_task(TaskNode::new("2", "task 2", TaskType::Query));
        assert!(decomposer.validate_graph(&graph).is_ok());
    }

    #[test]
    fn test_task_decomposer_validate_graph_missing_dependency() {
        let decomposer = TaskDecomposer::new();
        let mut graph = TaskGraph::new();
        let t1 = TaskNode::new("1", "task 1", TaskType::Query)
            .with_dependencies(vec!["nonexistent".to_string()]);
        graph.add_task(t1);
        let result = decomposer.validate_graph(&graph);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DecompositionError::InvalidStructure(_)));
    }

    #[test]
    fn test_task_decomposer_validate_graph_cycle() {
        let decomposer = TaskDecomposer::new();
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("1", "task 1", TaskType::Query);
        t1.dependencies = vec!["2".to_string()];
        let mut t2 = TaskNode::new("2", "task 2", TaskType::Query);
        t2.dependencies = vec!["1".to_string()];
        graph.add_task(t1);
        graph.add_task(t2);
        let result = decomposer.validate_graph(&graph);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_decomposer_validate_graph_self_dependency() {
        let decomposer = TaskDecomposer::new();
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("1", "task 1", TaskType::Query);
        t1.dependencies = vec!["1".to_string()];
        graph.add_task(t1);
        let result = decomposer.validate_graph(&graph);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_decomposer_validate_graph_three_node_cycle() {
        let decomposer = TaskDecomposer::new();
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("1", "task 1", TaskType::Query);
        t1.dependencies = vec!["3".to_string()];
        let mut t2 = TaskNode::new("2", "task 2", TaskType::Query);
        t2.dependencies = vec!["1".to_string()];
        let mut t3 = TaskNode::new("3", "task 3", TaskType::Query);
        t3.dependencies = vec!["2".to_string()];
        graph.add_task(t1);
        graph.add_task(t2);
        graph.add_task(t3);
        let result = decomposer.validate_graph(&graph);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_decomposer_infer_parallel_groups_empty() {
        let decomposer = TaskDecomposer::new();
        let groups = decomposer.infer_parallel_groups(&[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_task_decomposer_infer_parallel_groups_no_deps() {
        let decomposer = TaskDecomposer::new();
        let tasks = vec![
            TaskNode::new("1", "task 1", TaskType::Query),
            TaskNode::new("2", "task 2", TaskType::Query),
            TaskNode::new("3", "task 3", TaskType::Query),
        ];
        let groups = decomposer.infer_parallel_groups(&tasks);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec!["1", "2", "3"]);
    }

    #[test]
    fn test_task_decomposer_infer_parallel_groups_with_deps() {
        let decomposer = TaskDecomposer::new();
        let t1 = TaskNode::new("1", "task 1", TaskType::Query);
        let mut t2 = TaskNode::new("2", "task 2", TaskType::Query);
        t2.dependencies = vec!["1".to_string()];
        let tasks = vec![t1, t2];
        let groups = decomposer.infer_parallel_groups(&tasks);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], vec!["1"]);
        assert_eq!(groups[1], vec!["2"]);
    }

    #[test]
    fn test_task_decomposer_infer_parallel_groups_mixed() {
        let decomposer = TaskDecomposer::new();
        let t1 = TaskNode::new("1", "task 1", TaskType::Query);
        let t2 = TaskNode::new("2", "task 2", TaskType::Query);
        let mut t3 = TaskNode::new("3", "task 3", TaskType::Query);
        t3.dependencies = vec!["1".to_string()];
        let tasks = vec![t1, t2, t3];
        let groups = decomposer.infer_parallel_groups(&tasks);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], vec!["1", "2"]);
        assert_eq!(groups[1], vec!["3"]);
    }

    #[test]
    fn test_task_decomposer_parse_json_value_valid() {
        let decomposer = TaskDecomposer::new();
        let json = serde_json::json!({
            "tasks": [
                {"id": "1", "description": "first task", "type": "query", "dependencies": []},
                {"id": "2", "description": "second task", "type": "tool_call", "dependencies": ["1"]}
            ],
            "parallel_groups": [["1"], ["2"]],
            "reasoning": "test"
        });
        let result = decomposer.parse_json_value(&json).unwrap();
        assert_eq!(result.tasks.len(), 2);
        assert_eq!(result.tasks[0].id, "1");
        assert_eq!(result.tasks[1].task_type, TaskType::ToolCall);
        assert_eq!(result.tasks[1].dependencies, vec!["1"]);
        assert_eq!(result.reasoning, "test");
    }

    #[test]
    fn test_task_decomposer_parse_json_value_missing_tasks() {
        let decomposer = TaskDecomposer::new();
        let json = serde_json::json!({"reasoning": "no tasks"});
        let result = decomposer.parse_json_value(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_decomposer_parse_json_value_default_fields() {
        let decomposer = TaskDecomposer::new();
        let json = serde_json::json!({
            "tasks": [{"description": "a task"}]
        });
        let result = decomposer.parse_json_value(&json).unwrap();
        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].id, "0");
        assert_eq!(result.tasks[0].task_type, TaskType::Query);
        assert!(result.tasks[0].dependencies.is_empty());
    }

    #[test]
    fn test_task_decomposer_parse_json_value_all_task_types() {
        let decomposer = TaskDecomposer::new();
        let json = serde_json::json!({
            "tasks": [
                {"id": "1", "type": "tool_call"},
                {"id": "2", "type": "reasoning"},
                {"id": "3", "type": "query"},
                {"id": "4", "type": "validation"}
            ]
        });
        let result = decomposer.parse_json_value(&json).unwrap();
        assert_eq!(result.tasks[0].task_type, TaskType::ToolCall);
        assert_eq!(result.tasks[1].task_type, TaskType::Reasoning);
        assert_eq!(result.tasks[2].task_type, TaskType::Query);
        assert_eq!(result.tasks[3].task_type, TaskType::Validation);
    }

    #[test]
    fn test_task_decomposer_parse_json_value_invalid_type_defaults_to_query() {
        let decomposer = TaskDecomposer::new();
        let json = serde_json::json!({
            "tasks": [{"id": "1", "type": "unknown_type"}]
        });
        let result = decomposer.parse_json_value(&json).unwrap();
        assert_eq!(result.tasks[0].task_type, TaskType::Query);
    }

    #[test]
    fn test_task_decomposer_parse_json_value_inferred_parallel_groups() {
        let decomposer = TaskDecomposer::new();
        let json = serde_json::json!({
            "tasks": [
                {"id": "1", "description": "task 1", "type": "query"},
                {"id": "2", "description": "task 2", "type": "query"}
            ]
        });
        let result = decomposer.parse_json_value(&json).unwrap();
        assert_eq!(result.parallel_groups.len(), 1);
        assert_eq!(result.parallel_groups[0], vec!["1", "2"]);
    }

    #[test]
    fn test_task_decomposer_parse_fallback_response_too_short() {
        let decomposer = TaskDecomposer::new();
        let result = decomposer.parse_fallback_response("single line");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DecompositionError::ParseError(_)));
    }

    #[test]
    fn test_task_decomposer_parse_fallback_response_empty() {
        let decomposer = TaskDecomposer::new();
        let result = decomposer.parse_fallback_response("");
        assert!(result.is_err());
    }

    #[test]
    fn test_task_decomposer_parse_fallback_response_valid() {
        let decomposer = TaskDecomposer::new();
        let response = "first task\nsecond task\nthird task";
        let result = decomposer.parse_fallback_response(response).unwrap();
        assert_eq!(result.tasks.len(), 3);
        assert_eq!(result.tasks[0].id, "1");
        assert_eq!(result.tasks[1].id, "2");
        assert_eq!(result.tasks[2].id, "3");
        assert_eq!(result.reasoning, "Simple line-by-line decomposition");
    }

    #[test]
    fn test_task_decomposer_parse_fallback_response_with_markers() {
        let decomposer = TaskDecomposer::new();
        let response = "- first task\n* second task\n→ third task\n• fourth task";
        let result = decomposer.parse_fallback_response(response).unwrap();
        assert_eq!(result.tasks.len(), 4);
        assert_eq!(result.tasks[0].description, "first task");
        assert_eq!(result.tasks[1].description, "second task");
        assert_eq!(result.tasks[2].description, "third task");
        assert_eq!(result.tasks[3].description, "fourth task");
    }

    #[test]
    fn test_task_decomposer_parse_fallback_response_blank_lines() {
        let decomposer = TaskDecomposer::new();
        let response = "first task\n\n\nsecond task";
        let result = decomposer.parse_fallback_response(response).unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    #[test]
    fn test_task_decomposer_parse_response_json() {
        let decomposer = TaskDecomposer::new();
        let response = r#"{"tasks":[{"id":"1","description":"test","type":"query","dependencies":[]}],"parallel_groups":[["1"]],"reasoning":"test"}"#;
        let result = decomposer.parse_response(response).unwrap();
        assert_eq!(result.tasks.len(), 1);
    }

    #[test]
    fn test_task_decomposer_parse_response_fallback() {
        let decomposer = TaskDecomposer::new();
        let response = "task one\ntask two";
        let result = decomposer.parse_response(response).unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    #[test]
    fn test_task_decomposer_parse_response_invalid_json_fallback() {
        let decomposer = TaskDecomposer::new();
        let response = "{invalid json}\nsecond line";
        let result = decomposer.parse_response(response).unwrap();
        assert_eq!(result.tasks.len(), 2);
    }

    #[test]
    fn test_truncate_string_short() {
        let result = truncate_string("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_string_long() {
        let long_str = "a".repeat(200);
        let result = truncate_string(&long_str, 100);
        assert!(result.len() <= 100);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_string_exact_length() {
        let s = "a".repeat(100);
        let result = truncate_string(&s, 100);
        assert_eq!(result.len(), 100);
        assert!(!result.ends_with("..."));
    }

    #[test]
    fn test_truncate_string_empty() {
        let result = truncate_string("", 10);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_string_small_max() {
        let result = truncate_string("abc", 2);
        assert_eq!(result, "...");
    }
}
