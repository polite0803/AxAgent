// SPDX-License-Identifier: AGPL-3.0-only

//! TaskCreate / TaskGet / TaskList / TaskStop / TaskUpdate / TaskOutput
//!
//! 后台任务系统：基于数据库持久化，支持 bash 和 agent 两种类型。
//! bash 任务通过 spawn_background_task 命令真实后台执行并实时写入输出。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct TaskCreateTool;
pub struct TaskGetTool;
pub struct TaskListTool;
pub struct TaskStopTool;
pub struct TaskUpdateTool;
pub struct TaskOutputTool;

/// 通过 BackgroundTaskRepository trait 操作数据库的辅助函数
async fn db_spawn_task(title: &str, desc: &str) -> Result<String, String> {
    let repo = axagent_harness::repositories::background_task_repository();
    let input = axagent_harness::repo_dtos::CreateBackgroundTaskInput {
        title: title.to_string(),
        description: desc.to_string(),
        task_type: "agent".to_string(),
        command: None,
        prompt: None,
        created_by: None,
        idempotency_key: None,
    };
    let task = repo.spawn_task(input).await?;
    Ok(task.id)
}

async fn db_get_task(id: &str) -> Result<String, String> {
    let repo = axagent_harness::repositories::background_task_repository();
    match repo.get_task(id).await? {
        Some(t) => Ok(format!("**{}** [{}]\nID: {}\n{}", t.title, t.status, t.id, t.description)),
        None => Ok(format!("任务 '{}' 未找到", id)),
    }
}

async fn db_list_tasks() -> Result<String, String> {
    let repo = axagent_harness::repositories::background_task_repository();
    let tasks = repo.list_tasks().await?;
    if tasks.is_empty() {
        return Ok("(无任务)".to_string());
    }
    let mut out = String::from("## 任务列表\n\n");
    for t in tasks {
        let finished = if t.finished_at.is_some() { "" } else { "⏳" };
        out.push_str(&format!("- {} [{}] **{}**: {}\n", finished, t.status, t.title, t.id));
    }
    Ok(out)
}

async fn db_stop_task(id: &str) -> Result<String, String> {
    let repo = axagent_harness::repositories::background_task_repository();
    repo.stop_task(id).await?;
    Ok(format!("⏹️ 任务 '{}' 已停止", id))
}

async fn db_update_status(id: &str, status: &str) -> Result<String, String> {
    let repo = axagent_harness::repositories::background_task_repository();
    repo.update_status(id, status).await?;
    Ok(format!("📝 任务 '{}' → {}", id, status))
}

async fn db_get_output(id: &str) -> Result<String, String> {
    let repo = axagent_harness::repositories::background_task_repository();
    match repo.get_output(id).await? {
        Some(out) if !out.is_empty() => Ok(out),
        _ => Ok("(无输出)".to_string()),
    }
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "TaskCreate"
    }
    fn description(&self) -> &str {
        "创建后台任务，返回 task_id。支持 bash 和 agent 两种类型。bash 任务会真实后台执行并实时输出。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {"type": "string", "description": "任务标题"},
                "description": {"type": "string", "description": "任务描述"},
                "task_type": {"type": "string", "description": "bash 或 agent", "default": "agent"},
                "command": {"type": "string", "description": "bash 命令（task_type=bash 时需要）"}
            },
            "required": ["title", "description"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let title = input["title"].as_str().unwrap_or("untitled").to_string();
        let desc = input["description"].as_str().unwrap_or("").to_string();
        let task_type = input["task_type"].as_str().unwrap_or("agent").to_string();
        let command = input["command"].as_str().map(|s| s.to_string());

        let id = db_spawn_task(&title, &desc).await.unwrap_or_else(|_| "db-error".to_string());

        // 如果是 bash 任务且有命令，需要告诉用户使用 spawn_background_task
        if task_type == "bash" && command.is_some() {
            return Ok(ToolResult::success(format!(
                "✅ 任务已创建: **{}** (ID: {})\n\n💡 bash 任务需要由前端触发执行。请使用 spawn_background_task 命令执行：\n```\nspawn_background_task(id=\"{}\")\n```",
                title, id, id
            )));
        }

        Ok(ToolResult::success(format!("✅ 任务已创建: **{}** (ID: {})", title, id)))
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
        ToolCategory::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        Ok(ToolResult::success(db_get_task(id).await.unwrap_or_else(|e| format!("DB 错误: {}", e))))
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
        ToolCategory::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success(db_list_tasks().await.unwrap_or_else(|e| format!("DB 错误: {}", e))))
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
        ToolCategory::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        Ok(ToolResult::success(
            db_stop_task(id).await.unwrap_or_else(|e| format!("DB 错误: {}", e)),
        ))
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
        ToolCategory::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        let status = input["status"].as_str().unwrap_or("pending").to_string();
        Ok(ToolResult::success(
            db_update_status(id, &status).await.unwrap_or_else(|e| format!("DB 错误: {}", e)),
        ))
    }
}

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "TaskOutput"
    }
    fn description(&self) -> &str {
        "获取后台任务的实时输出内容"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"task_id":{"type":"string"}},"required":["task_id"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        Ok(ToolResult::success(
            db_get_output(id).await.unwrap_or_else(|e| format!("DB 错误: {}", e)),
        ))
    }
}
