//! TaskCreateTool / TaskGetTool / TaskListTool / TaskStopTool / TaskUpdateTool / TaskOutputTool

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TaskInfo {
    id: String,
    title: String,
    description: String,
    status: String,
    created_by: String,
    blocked_by: Vec<String>,
    output: Vec<String>,
}
static TASKS: LazyLock<RwLock<HashMap<String, TaskInfo>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct TaskCreateTool;
pub struct TaskGetTool;
pub struct TaskListTool;
pub struct TaskStopTool;
pub struct TaskUpdateTool;
pub struct TaskOutputTool;

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "TaskCreate"
    }
    fn description(&self) -> &str {
        "创建后台任务，返回 task_id。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"title":{"type":"string"},"description":{"type":"string"}},"required":["title","description"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = uuid::Uuid::new_v4().to_string();
        let title = input["title"].as_str().unwrap_or("untitled").to_string();
        let desc = input["description"].as_str().unwrap_or("").to_string();
        TASKS.write().unwrap().insert(
            id.clone(),
            TaskInfo {
                id: id.clone(),
                title: title.clone(),
                description: desc,
                status: "created".into(),
                created_by: ctx.conversation_id.clone().unwrap_or_default(),
                blocked_by: vec![],
                output: vec![],
            },
        );
        Ok(ToolResult::success(format!(
            "✅ 任务已创建: **{}** (ID: {})",
            title, id
        )))
    }
}

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "TaskGet"
    }
    fn description(&self) -> &str {
        "按 ID 获取任务详情"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"task_id":{"type":"string"}},"required":["task_id"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, input: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        if let Some(t) = TASKS.read().unwrap().get(id) {
            Ok(ToolResult::success(format!(
                "**{}** [{}] ID: {}\n{}",
                t.title, t.status, t.id, t.description
            )))
        } else {
            Ok(ToolResult::success(format!("任务 '{}' 未找到", id)))
        }
    }
}

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "TaskList"
    }
    fn description(&self) -> &str {
        "列出所有任务"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, _i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let tasks = TASKS.read().unwrap();
        if tasks.is_empty() {
            return Ok(ToolResult::success("(无任务)"));
        }
        let mut out = String::from("## 任务列表\n\n");
        for t in tasks.values() {
            out.push_str(&format!("- [{}] **{}**: {}\n", t.status, t.title, t.id));
        }
        Ok(ToolResult::success(out))
    }
}

#[async_trait]
impl Tool for TaskStopTool {
    fn name(&self) -> &str {
        "TaskStop"
    }
    fn description(&self) -> &str {
        "停止运行中的任务"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"task_id":{"type":"string"}},"required":["task_id"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, input: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        if let Some(t) = TASKS.write().unwrap().get_mut(id) {
            t.status = "stopped".into();
        }
        Ok(ToolResult::success(format!("⏹️ 任务 '{}' 已停止", id)))
    }
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "TaskUpdate"
    }
    fn description(&self) -> &str {
        "更新任务状态"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"task_id":{"type":"string"},"status":{"type":"string"}},"required":["task_id"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, input: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        let s = input["status"].as_str().unwrap_or("pending").to_string();
        if let Some(t) = TASKS.write().unwrap().get_mut(id) {
            t.status = s.clone();
        }
        Ok(ToolResult::success(format!("📝 任务 '{}' → {}", id, s)))
    }
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "TaskOutput"
    }
    fn description(&self) -> &str {
        "获取任务输出"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"task_id":{"type":"string"}},"required":["task_id"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, input: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        if let Some(t) = TASKS.read().unwrap().get(id) {
            Ok(ToolResult::success(if t.output.is_empty() {
                "(无输出)".into()
            } else {
                t.output.join("\n")
            }))
        } else {
            Ok(ToolResult::success(format!("任务 '{}' 未找到", id)))
        }
    }
}
