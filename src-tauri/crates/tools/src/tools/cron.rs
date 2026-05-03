//! CronCreateTool / CronDeleteTool / CronListTool - 持久化定时任务管理
//! 50 个任务上限，7 天自动过期，支持 cron 表达式 + fireAt 两种模式

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

const MAX_TASKS: usize = 50;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CronTask {
    id: String,
    prompt: String,
    description: String,
    cron: Option<String>,
    fire_at: Option<String>,
    created_at: String,
    next_run: String,
    enabled: bool,
    recurring: bool,
    run_count: u32,
}

static CRON_STORE: LazyLock<RwLock<HashMap<String, CronTask>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn save_to_disk() -> Result<(), String> {
    let store = CRON_STORE.read().unwrap();
    let tasks: Vec<CronTask> = store.values().cloned().collect();
    let json = serde_json::to_string_pretty(&tasks).map_err(|e| e.to_string())?;
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".axagent").join("cron_tasks.json");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_from_disk() {
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".axagent").join("cron_tasks.json");
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(tasks) = serde_json::from_str::<Vec<CronTask>>(&content) {
                let mut store = CRON_STORE.write().unwrap();
                for t in tasks {
                    store.insert(t.id.clone(), t);
                }
            }
        }
    }
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
        "创建定时任务。5 字段 cron 表达式（如 '0 9 * * *'=每日9点）支持循环执行，fire_at 支持一次性执行。最多50个任务，循环任务7天自动过期。持久化到 ~/.axagent/cron_tasks.json。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": {"type":"string","description":"任务唯一标识(kebab-case)"},
                "prompt": {"type":"string","description":"每次执行的任务指令"},
                "description": {"type":"string","description":"简短描述"},
                "cron": {"type":"string","description":"5字段cron表达式"},
                "fire_at": {"type":"string","description":"一次性执行时间(ISO 8601)"}
            },
            "required": ["task_id","prompt","description"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["task_id"].as_str().unwrap_or("unknown").to_string();
        let prompt = input["prompt"].as_str().unwrap_or("").to_string();
        let desc = input["description"].as_str().unwrap_or("").to_string();
        let cron = input["cron"].as_str().map(|s| s.to_string());
        let fire_at = input["fire_at"].as_str().map(|s| s.to_string());
        let recurring = cron.is_some();

        let mut store = CRON_STORE.write().unwrap();
        load_from_disk(); // 确保加载最新持久化数据
        if store.len() >= MAX_TASKS {
            return Err(ToolError::invalid_input(format!(
                "已达最大任务数 {}",
                MAX_TASKS
            )));
        }

        let schedule = cron
            .clone()
            .unwrap_or_else(|| fire_at.clone().unwrap_or_default());
        store.insert(
            id.clone(),
            CronTask {
                id: id.clone(),
                prompt,
                description: desc.clone(),
                cron,
                fire_at,
                created_at: chrono::Utc::now().to_rfc3339(),
                next_run: "待调度".into(),
                enabled: true,
                recurring,
                run_count: 0,
            },
        );
        let _ = save_to_disk();

        Ok(ToolResult::success(format!(
            "✅ 定时任务已创建\n**ID**: {}\n**描述**: {}\n**调度**: {}\n**模式**: {}\n**持久化**: ~/.axagent/cron_tasks.json",
            id, desc, schedule, if recurring { "循环(7天过期)" } else { "一次性" }
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
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["id"].as_str().unwrap_or("?");
        let mut store = CRON_STORE.write().unwrap();
        if store.remove(id).is_some() {
            let _ = save_to_disk();
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
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let store = CRON_STORE.read().unwrap();
        if store.is_empty() {
            return Ok(ToolResult::success("## 定时任务\n\n(无任务)"));
        }
        let mut out = String::from("## 定时任务\n\n");
        for (_, t) in store.iter() {
            let status = if t.enabled { "✅" } else { "⏸️" };
            let sched = t
                .cron
                .as_deref()
                .unwrap_or_else(|| t.fire_at.as_deref().unwrap_or("手动"));
            out.push_str(&format!(
                "- {} **{}**: {} ({}, 已执行 {} 次)\n",
                status, t.id, t.description, sched, t.run_count
            ));
        }
        Ok(ToolResult::success(out))
    }
}
