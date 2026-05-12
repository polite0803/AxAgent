//! CronCreateTool / CronDeleteTool / CronListTool
//! 委托到 axagent_runtime_core::CronJobStore，由 runtime/cron 调度器统一执行。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_runtime_core::CronJobStore;
use serde_json::Value;
use std::sync::{Arc, OnceLock};

const MAX_TASKS: usize = 50;

static SHARED_CRON_STORE: OnceLock<Arc<CronJobStore>> = OnceLock::new();

/// 初始化共享 CronJobStore（由 runtime init 调用）
pub fn init_cron_store(store: Arc<CronJobStore>) {
    let _ = SHARED_CRON_STORE.set(store);
}

/// 获取共享 CronJobStore
fn cron_store() -> Arc<CronJobStore> {
    SHARED_CRON_STORE
        .get()
        .cloned()
        .unwrap_or_else(|| Arc::new(CronJobStore::new()))
}

pub struct CronCreateTool;
pub struct CronDeleteTool;
pub struct CronListTool;

#[async_trait]
impl Tool for CronCreateTool {
    fn name(&self) -> &str {
        "CronCreate"
    }
    fn description(&self) -> &str {
        "创建定时任务。5 字段 cron 表达式（如 '0 9 * * *'=每日9点），最多50个任务。由系统调度器自动执行。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": {"type":"string","description":"任务唯一标识(kebab-case)"},
                "prompt": {"type":"string","description":"每次执行的任务指令"},
                "description": {"type":"string","description":"简短描述"},
                "cron": {"type":"string","description":"5字段cron表达式"}
            },
            "required": ["task_id","prompt","description"]
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
        let id = input["task_id"].as_str().unwrap_or("unknown").to_string();
        let prompt = input["prompt"].as_str().unwrap_or("").to_string();
        let desc = input["description"].as_str().unwrap_or("").to_string();
        let schedule = input["cron"].as_str().unwrap_or("0 9 * * *").to_string();

        let store = cron_store();
        if store.count().await >= MAX_TASKS {
            return Err(ToolError::invalid_input(format!("已达最大任务数 {}", MAX_TASKS)));
        }

        // 检查是否已存在
        if store.get(&id).await.is_some() {
            return Err(ToolError::invalid_input(format!(
                "任务 ID '{}' 已存在，请使用不同 ID",
                id
            )));
        }

        let job = axagent_runtime_core::CronJob::new(&id, &schedule, &prompt, &desc);
        store.add(job).await;

        Ok(ToolResult::success(format!(
            "✅ 定时任务已创建\n**ID**: {}\n**描述**: {}\n**调度**: {}\n系统调度器每30秒检查一次",
            id, desc, schedule
        )))
    }
}

#[async_trait]
impl Tool for CronDeleteTool {
    fn name(&self) -> &str {
        "CronDelete"
    }
    fn description(&self) -> &str {
        "删除指定 ID 的定时任务"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]})
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
        let id = input["id"].as_str().unwrap_or("?");
        let store = cron_store();
        if store.remove(id).await {
            Ok(ToolResult::success(format!("🗑️ 已删除定时任务: {}", id)))
        } else {
            Ok(ToolResult::success(format!("⚠️ 未找到任务: {}", id)))
        }
    }
}

#[async_trait]
impl Tool for CronListTool {
    fn name(&self) -> &str {
        "CronList"
    }
    fn description(&self) -> &str {
        "列出所有已注册的定时任务"
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
        let store = cron_store();
        let jobs = store.list().await;
        if jobs.is_empty() {
            return Ok(ToolResult::success("## 定时任务\n\n(无任务)"));
        }
        let mut out = String::from("## 定时任务\n\n");
        for job in &jobs {
            let status = if job.is_active() { "✅" } else { "⏸️" };
            out.push_str(&format!(
                "- {} **{}**: {} ({}, 已执行 {} 次)\n",
                status, job.name, job.description, job.schedule, job.run_count
            ));
        }
        Ok(ToolResult::success(out))
    }
}
