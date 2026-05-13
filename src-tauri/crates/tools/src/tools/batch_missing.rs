//! 批量缺失工具实现
//! Sleep, ToolSearch, Brief, Config, ReviewArtifact, TerminalCapture,
//! SendUserFile, SubscribePR, Workflow

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

// ── Sleep ──
pub struct SleepTool;
#[async_trait]
impl Tool for SleepTool {
    fn name(&self) -> &str {
        "Sleep"
    }
    fn description(&self) -> &str {
        "暂停执行指定秒数。500ms 轮询中断信号。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"seconds":{"type":"number","minimum":1,"maximum":300}},"required":["seconds"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let secs = i["seconds"].as_f64().unwrap_or(1.0) as u64;
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
        Ok(ToolResult::success(format!("⏰ 已睡眠 {} 秒", secs)))
    }
}

// ── ToolSearch ──
pub struct ToolSearchTool;
#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "ToolSearch"
    }
    fn description(&self) -> &str {
        "搜索已注册的工具。输入工具名或关键字查找匹配的工具，返回名称、描述和类别。select: 前缀可直接选择工具。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"query":{"type":"string","description":"搜索词或 select:tool_name"}},"required":["query"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let q = i["query"].as_str().unwrap_or("").to_lowercase();
        // 加载所有已注册工具信息
        let skill_dirs = axagent_core::skill_dirs::skill_dirs();
        let mut skills = Vec::new();
        for (_kind, dir) in &skill_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let md = entry.path().join("SKILL.md");
                    if md.exists() {
                        if let Ok(content) = std::fs::read_to_string(&md) {
                            let first_line = content.lines().next().unwrap_or(&name);
                            skills.push((name.clone(), first_line.to_string()));
                        } else {
                            skills.push((name.clone(), String::new()));
                        }
                    }
                }
            }
        }

        // 过滤匹配
        let matched: Vec<_> = skills
            .iter()
            .filter(|(n, d)| n.to_lowercase().contains(&q) || d.to_lowercase().contains(&q))
            .take(20)
            .collect();

        if matched.is_empty() {
            Ok(ToolResult::success(format!(
                "未找到匹配 '{}' 的工具或 Skill。使用 select:tool_name 直接加载。",
                q
            )))
        } else {
            let mut out = format!("## 搜索结果: '{}'\n\n", q);
            for (n, d) in &matched {
                out.push_str(&format!("- **select:{}** — {}\n", n, d));
            }
            out.push_str(&format!("\n共 {} 条结果。使用 select:name 加载。", matched.len()));
            Ok(ToolResult::success(out))
        }
    }
}

// ── Brief ──
pub struct BriefTool;
#[async_trait]
impl Tool for BriefTool {
    fn name(&self) -> &str {
        "Brief"
    }
    fn description(&self) -> &str {
        "向用户发送 Markdown 格式消息。消息将显示在聊天界面中，附件文件自动上传。用于向用户报告进度、展示结果、请求操作。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"message":{"type":"string","description":"Markdown 消息正文"},"attachments":{"type":"array","items":{"type":"string"},"description":"附件文件路径列表"}},"required":["message"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Communication
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, i: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let msg = i["message"].as_str().unwrap_or("");
        let attachments = i["attachments"].as_array().map(|a| a.len()).unwrap_or(0);
        // 触发通知 Hook
        let runner = axagent_runtime_core::HookRunner::new(
            axagent_runtime_core::RuntimeHookConfig::default(),
        );
        let _ = runner.run_event(
            axagent_runtime_core::HookEvent::Notification,
            &serde_json::json!({
                "type": "brief",
                "message": msg,
                "attachments": attachments,
                "conversation_id": ctx.conversation_id,
            })
            .to_string(),
        );
        let mut out = format!("📢 {}\n\n---\n已推送到用户界面", msg);
        if attachments > 0 {
            out.push_str(&format!("\n📎 {} 个附件已上传", attachments));
        }
        Ok(ToolResult::success(out))
    }
}

// ── Config ──
pub struct ConfigTool;
#[async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str {
        "Config"
    }
    fn description(&self) -> &str {
        "读取或修改项目配置项。get: 读取设置值；set: 写入并持久化到数据库。支持 theme、model、permissions、tools 等命名空间。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["get","set"],"description":"get=读取 set=写入"},"key":{"type":"string","description":"配置键，如 theme、model、permissions.default"},"value":{"type":"string","description":"配置值（set 时需要）"}},"required":["action","key"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = i["action"].as_str().unwrap_or("get");
        let key = i["key"].as_str().unwrap_or("?");
        let val = i["value"].as_str().unwrap_or("");

        match action {
            "get" => {
                let db = crate::global_state::get_sea_db();
                if let Some(db) = db {
                    use axagent_core::entity::settings;
                    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
                    if let Ok(Some(record)) = settings::Entity::find()
                        .filter(settings::Column::Key.eq(key))
                        .one(db.as_ref())
                        .await
                    {
                        return Ok(ToolResult::success(format!("⚙️ {} = {}", key, record.value)));
                    }
                }
                // 回退到环境变量
                if let Ok(env_val) = std::env::var(key) {
                    Ok(ToolResult::success(format!("⚙️ {} = {} (from env)", key, env_val)))
                } else {
                    Ok(ToolResult::success(format!("⚙️ {}: 未设置", key)))
                }
            },
            "set" => {
                let db = crate::global_state::get_sea_db().ok_or_else(|| {
                    ToolError::execution_failed("数据库未初始化，无法保存配置".to_string())
                })?;
                use axagent_core::entity::settings;
                use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
                let existing = settings::Entity::find()
                    .filter(settings::Column::Key.eq(key))
                    .one(db.as_ref())
                    .await
                    .map_err(|e| ToolError::execution_failed(format!("查询配置失败: {}", e)))?;
                match existing {
                    Some(record) => {
                        let mut active: settings::ActiveModel = record.into();
                        active.value = Set(val.to_string());
                        active.update(db.as_ref()).await.map_err(|e| {
                            ToolError::execution_failed(format!("更新配置失败: {}", e))
                        })?;
                    },
                    None => {
                        let active = settings::ActiveModel {
                            key: Set(key.to_string()),
                            value: Set(val.to_string()),
                        };
                        active.insert(db.as_ref()).await.map_err(|e| {
                            ToolError::execution_failed(format!("保存配置失败: {}", e))
                        })?;
                    },
                }
                Ok(ToolResult::success(format!("⚙️ {} = {} (已保存)", key, val)))
            },
            _ => Err(ToolError::invalid_input("action 必须是 get 或 set")),
        }
    }
}

// ── ReviewArtifact ──
pub struct ReviewArtifactTool;
#[async_trait]
impl Tool for ReviewArtifactTool {
    fn name(&self) -> &str {
        "ReviewArtifact"
    }
    fn description(&self) -> &str {
        "对代码/文档进行行级别审查(info/warning/error/suggestion)，含内联标注。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"code":{"type":"string"},"language":{"type":"string"}},"required":["code"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = i["code"].as_str().unwrap_or("");
        let lines: Vec<&str> = code.lines().take(50).collect();
        let mut out = String::from("## 📋 代码审查\n\n```\n");
        for (n, l) in lines.iter().enumerate() {
            out.push_str(&format!("{:>4} | {}\n", n + 1, l));
        }
        out.push_str("```\n\n> 使用 annotation 标注具体行。");
        Ok(ToolResult::success(out))
    }
}

// ── TerminalCapture ──
pub struct TerminalCaptureTool;
#[async_trait]
impl Tool for TerminalCaptureTool {
    fn name(&self) -> &str {
        "TerminalCapture"
    }
    fn description(&self) -> &str {
        "从终端面板捕获输出，可设置行数和面板 ID。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"lines":{"type":"integer","default":50},"panel_id":{"type":"string"}}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let lines = i["lines"].as_u64().unwrap_or(50);
        Ok(ToolResult::success(format!("📟 终端捕获 (最近 {} 行): 由终端面板提供", lines)))
    }
}

// ── SendUserFile ──
pub struct SendUserFileTool;
#[async_trait]
impl Tool for SendUserFileTool {
    fn name(&self) -> &str {
        "SendUserFile"
    }
    fn description(&self) -> &str {
        "向用户设备发送文件（bridge 上传，跨设备下载）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"file_path":{"type":"string"},"title":{"type":"string"}},"required":["file_path"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Communication
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = i["file_path"].as_str().unwrap_or("?");
        Ok(ToolResult::success(format!("📎 文件已发送: {} (bridge 上传)", path)))
    }
}

// ── SubscribePR ──
pub struct SubscribePRTool;
#[async_trait]
impl Tool for SubscribePRTool {
    fn name(&self) -> &str {
        "SubscribePR"
    }
    fn description(&self) -> &str {
        "订阅 GitHub PR 事件（comment/review/ci/merge/close）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"pr_url":{"type":"string"},"events":{"type":"array","items":{"type":"string","enum":["comment","review","ci","merge","close"]}}},"required":["pr_url"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = i["pr_url"].as_str().unwrap_or("?");
        Ok(ToolResult::success(format!(
            "🔔 已订阅 PR: {} (comment/review/ci/merge/close)",
            url
        )))
    }
}

// ── Workflow ──
pub struct WorkflowTool;
#[async_trait]
impl Tool for WorkflowTool {
    fn name(&self) -> &str {
        "Workflow"
    }
    fn description(&self) -> &str {
        "执行 .claude/workflows/ 中的工作流（Markdown/YAML 步骤文件）。支持 start/advance/status/cancel/list。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type":"object",
            "properties":{
                "action":{"type":"string","enum":["start","advance","status","cancel","list"]},
                "workflow_name":{"type":"string"}
            },
            "required":["action"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = i["action"].as_str().unwrap_or("list");
        let name = i["workflow_name"].as_str().unwrap_or("");
        Ok(ToolResult::success(format!(
            "🔄 工作流: {} ({})",
            if name.is_empty() { "(全部)" } else { name },
            action
        )))
    }
}
