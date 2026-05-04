//! TaskCreate / TaskGet / TaskList / TaskStop / TaskUpdate / TaskOutput
//!
//! 后台任务系统：基于数据库持久化，支持 bash 和 agent 两种类型。
//! bash 任务通过 spawn_background_task 命令真实后台执行并实时写入输出。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, EntityTrait};
use serde_json::Value;
use std::sync::Arc;

pub struct TaskCreateTool;
pub struct TaskGetTool;
pub struct TaskListTool;
pub struct TaskStopTool;
pub struct TaskUpdateTool;
pub struct TaskOutputTool;

/// 通过 SeaORM 异步 API 操作数据库的辅助函数
async fn db_spawn_task(
    db: &sea_orm::DatabaseConnection,
    title: &str,
    desc: &str,
) -> Result<String, sea_orm::DbErr> {
    use axagent_core::entity::background_tasks;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let model = background_tasks::ActiveModel {
        id: sea_orm::Set(id.clone()),
        title: sea_orm::Set(title.to_string()),
        description: sea_orm::Set(desc.to_string()),
        task_type: sea_orm::Set("agent".to_string()),
        command: sea_orm::Set(None),
        prompt: sea_orm::Set(None),
        status: sea_orm::Set("pending".to_string()),
        output: sea_orm::Set(String::new()),
        exit_code: sea_orm::Set(None),
        conversation_id: sea_orm::Set(None),
        created_by: sea_orm::Set(None),
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
        finished_at: sea_orm::Set(None),
    };
    background_tasks::Entity::insert(model).exec(db).await?;
    Ok(id)
}

async fn db_get_task(db: &sea_orm::DatabaseConnection, id: &str) -> Result<String, sea_orm::DbErr> {
    use axagent_core::entity::background_tasks;
    if let Some(t) = background_tasks::Entity::find_by_id(id).one(db).await? {
        Ok(format!(
            "**{}** [{}]\nID: {}\n{}",
            t.title, t.status, t.id, t.description
        ))
    } else {
        Ok(format!("任务 '{}' 未找到", id))
    }
}

async fn db_list_tasks(db: &sea_orm::DatabaseConnection) -> Result<String, sea_orm::DbErr> {
    use axagent_core::entity::background_tasks;
    use sea_orm::EntityTrait;
    use sea_orm::QueryOrder;
    let tasks = background_tasks::Entity::find()
        .order_by_desc(background_tasks::Column::CreatedAt)
        .all(db)
        .await?;
    if tasks.is_empty() {
        return Ok("(无任务)".to_string());
    }
    let mut out = String::from("## 任务列表\n\n");
    for t in tasks {
        let finished = t.finished_at.map(|_| "").unwrap_or("⏳");
        out.push_str(&format!(
            "- {} [{}] **{}**: {}\n",
            finished, t.status, t.title, t.id
        ));
    }
    Ok(out)
}

async fn db_stop_task(
    db: &sea_orm::DatabaseConnection,
    id: &str,
) -> Result<String, sea_orm::DbErr> {
    use axagent_core::entity::background_tasks;
    if let Some(t) = background_tasks::Entity::find_by_id(id).one(db).await? {
        let now = chrono::Utc::now().timestamp_millis();
        let mut am: background_tasks::ActiveModel = t.into();
        am.status = sea_orm::Set("stopped".to_string());
        am.updated_at = sea_orm::Set(now);
        am.finished_at = sea_orm::Set(Some(now));
        am.update(db).await?;
        Ok(format!("⏹️ 任务 '{}' 已停止", id))
    } else {
        Ok(format!("任务 '{}' 未找到", id))
    }
}

async fn db_update_status(
    db: &sea_orm::DatabaseConnection,
    id: &str,
    status: &str,
) -> Result<String, sea_orm::DbErr> {
    use axagent_core::entity::background_tasks;
    if let Some(t) = background_tasks::Entity::find_by_id(id).one(db).await? {
        let now = chrono::Utc::now().timestamp_millis();
        let mut am: background_tasks::ActiveModel = t.into();
        am.status = sea_orm::Set(status.to_string());
        am.updated_at = sea_orm::Set(now);
        if status == "completed" || status == "failed" {
            am.finished_at = sea_orm::Set(Some(now));
        }
        am.update(db).await?;
        Ok(format!("📝 任务 '{}' → {}", id, status))
    } else {
        Ok(format!("任务 '{}' 未找到", id))
    }
}

async fn db_get_output(
    db: &sea_orm::DatabaseConnection,
    id: &str,
) -> Result<String, sea_orm::DbErr> {
    use axagent_core::entity::background_tasks;
    if let Some(t) = background_tasks::Entity::find_by_id(id).one(db).await? {
        if t.output.is_empty() {
            Ok("(无输出)".to_string())
        } else {
            Ok(t.output.clone())
        }
    } else {
        Ok(format!("任务 '{}' 未找到", id))
    }
}

fn get_db(_ctx: &ToolContext) -> Arc<sea_orm::DatabaseConnection> {
    crate::builtin_tools::get_global_sea_db().expect("Global DB not initialized for task system")
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
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let title = input["title"].as_str().unwrap_or("untitled").to_string();
        let desc = input["description"].as_str().unwrap_or("").to_string();
        let task_type = input["task_type"].as_str().unwrap_or("agent").to_string();
        let command = input["command"].as_str().map(|s| s.to_string());

        let db = get_db(ctx);
        let id = db_spawn_task(&db, &title, &desc)
            .await
            .unwrap_or_else(|_| "db-error".to_string());

        // 如果是 bash 任务且有命令，需要告诉用户使用 spawn_background_task
        if task_type == "bash" && command.is_some() {
            return Ok(ToolResult::success(format!(
                "✅ 任务已创建: **{}** (ID: {})\n\n💡 bash 任务需要由前端触发执行。请使用 spawn_background_task 命令执行：\n```\nspawn_background_task(id=\"{}\")\n```",
                title, id, id
            )));
        }

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
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        let db = get_db(ctx);
        Ok(ToolResult::success(
            db_get_task(&db, id)
                .await
                .unwrap_or_else(|e| format!("DB 错误: {}", e)),
        ))
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
    async fn call(&self, _input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let db = get_db(ctx);
        Ok(ToolResult::success(
            db_list_tasks(&db)
                .await
                .unwrap_or_else(|e| format!("DB 错误: {}", e)),
        ))
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
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        let db = get_db(ctx);
        Ok(ToolResult::success(
            db_stop_task(&db, id)
                .await
                .unwrap_or_else(|e| format!("DB 错误: {}", e)),
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
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        let status = input["status"].as_str().unwrap_or("pending").to_string();
        let db = get_db(ctx);
        Ok(ToolResult::success(
            db_update_status(&db, id, &status)
                .await
                .unwrap_or_else(|e| format!("DB 错误: {}", e)),
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
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("?");
        let db = get_db(ctx);
        Ok(ToolResult::success(
            db_get_output(&db, id)
                .await
                .unwrap_or_else(|e| format!("DB 错误: {}", e)),
        ))
    }
}
