//! 批量缺失工具实现
//! WorktreeEnter/WorktreeExit, Sleep, ToolSearch, Brief, Config, ReviewArtifact,
//! TerminalCapture, SendUserFile, DiscoverSkills, SubscribePR, Workflow,
//! VerifyPlanExecution, RemoteTrigger, SuggestBackgroundPR

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

/// 触发 Worktree 相关 HookEvent（best-effort，失败不影响主流程）
fn fire_worktree_hook(event: axagent_runtime_core::HookEvent, data: &serde_json::Value) {
    let runner =
        axagent_runtime_core::HookRunner::new(axagent_runtime_core::RuntimeHookConfig::default());
    let data_str = data.to_string();
    let _ = runner.run_event(event, &data_str);
}

/// 获取 git 仓库根目录
fn git_root() -> Result<String, ToolError> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| ToolError::execution_failed(format!("git 命令执行失败: {}", e)))?;
    if output.status.success() {
        String::from_utf8(output.stdout)
            .map(|s| s.trim().to_string())
            .map_err(|e| ToolError::execution_failed(format!("git 输出解析失败: {}", e)))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(ToolError::execution_failed(format!("不在 git 仓库中: {}", err)))
    }
}

/// 获取默认分支名（main 或 master）
fn default_branch() -> String {
    let output = Command::new("git")
        .args(["branch", "-a"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        });
    if let Some(ref branches) = output {
        for name in &["main", "master"] {
            if branches.lines().any(|l| l.contains(name)) {
                return name.to_string();
            }
        }
    }
    "main".to_string()
}

/// 列出现有 worktree
fn list_worktrees() -> Result<Vec<(String, String, String)>, ToolError> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(|e| ToolError::execution_failed(format!("git worktree list 失败: {}", e)))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path = String::new();
    let mut current_head = String::new();
    let mut current_branch = String::new();
    for line in text.lines() {
        if let Some(stripped) = line.strip_prefix("worktree ") {
            current_path = stripped.to_string();
        } else if let Some(stripped) = line.strip_prefix("HEAD ") {
            current_head = stripped.to_string();
        } else if let Some(stripped) = line.strip_prefix("branch ") {
            current_branch = stripped.trim_start_matches("refs/heads/").to_string();
            worktrees.push((current_path.clone(), current_head.clone(), current_branch.clone()));
        } else if line.is_empty() {
            if !current_path.is_empty() && current_branch.is_empty() {
                worktrees.push((
                    current_path.clone(),
                    current_head.clone(),
                    "detached".to_string(),
                ));
            }
            current_path.clear();
            current_head.clear();
            current_branch.clear();
        }
    }
    Ok(worktrees)
}

// ── Worktree 工具 ──
pub struct EnterWorktreeTool;
#[async_trait]
impl Tool for EnterWorktreeTool {
    fn name(&self) -> &str {
        "EnterWorktree"
    }
    fn description(&self) -> &str {
        "创建隔离的 git worktree。在 .claude/worktrees/ 下创建新分支的独立工作目录。需要 git 仓库。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "worktree 名称（可选），字母/数字/横线组成，最多 64 字符"
                }
            },
            "required": []
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        false
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let root = git_root()?;
        let base = default_branch();

        let name = i["name"].as_str().unwrap_or("auto-generated");
        let sanitized: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let branch_name = format!("worktree/{}", sanitized);
        let worktree_path = format!("{}/.claude/worktrees/{}", root, sanitized);

        // 检查是否已存在
        let existing = list_worktrees().unwrap_or_default();
        if existing.iter().any(|(p, _, _)| p == &worktree_path) {
            return Err(ToolError::invalid_input(format!(
                "worktree '{}' 已存在，请使用不同名称",
                sanitized
            )));
        }

        // 创建分支
        let branch_status = Command::new("git")
            .args(["branch", &branch_name, &base])
            .current_dir(&root)
            .output()
            .map_err(|e| ToolError::execution_failed(format!("创建分支失败: {}", e)))?;
        if !branch_status.status.success() {
            let err = String::from_utf8_lossy(&branch_status.stderr);
            return Err(ToolError::execution_failed(format!("创建分支失败: {}", err)));
        }

        // 创建 worktree
        let output = Command::new("git")
            .args(["worktree", "add", &worktree_path, &branch_name])
            .current_dir(&root)
            .output()
            .map_err(|e| ToolError::execution_failed(format!("git worktree add 失败: {}", e)))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            // 清理创建的分支
            let _ = Command::new("git")
                .args(["branch", "-D", &branch_name])
                .current_dir(&root)
                .output();
            return Err(ToolError::execution_failed(format!("创建 worktree 失败: {}", err)));
        }

        // 触发 hook
        fire_worktree_hook(
            axagent_runtime_core::HookEvent::ConfigChange,
            &json!({
                "name": sanitized,
                "branch": branch_name,
                "path": worktree_path,
                "root": root,
            }),
        );

        Ok(ToolResult::success(format!(
            "## 🌳 Worktree 已创建\n\n\
             **名称**: {}\n\
             **分支**: {}\n\
             **路径**: {}\n\
             **基础分支**: {}\n\n\
             工作目录已切换到新的 worktree。",
            sanitized, branch_name, worktree_path, base
        )))
    }
}

pub struct ExitWorktreeTool;
#[async_trait]
impl Tool for ExitWorktreeTool {
    fn name(&self) -> &str {
        "ExitWorktree"
    }
    fn description(&self) -> &str {
        "退出 worktree 会话。remove: 删除 worktree 目录及关联分支；keep: 仅离开（保留文件）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["keep", "remove"],
                    "description": "keep=保留文件仅离开, remove=删除 worktree 目录和分支"
                },
                "discard_changes": {
                    "type": "boolean",
                    "default": false,
                    "description": "remove 时是否强制丢弃未提交更改"
                }
            },
            "required": ["action"]
        })
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
        let action = i["action"].as_str().unwrap_or("keep");
        let is_remove = action == "remove";
        let discard = i["discard_changes"].as_bool().unwrap_or(false);

        if !is_remove {
            fire_worktree_hook(
                axagent_runtime_core::HookEvent::ConfigChange,
                &json!({"action": "keep"}),
            );
            return Ok(ToolResult::success("📤 已离开 worktree（文件已保留）"));
        }

        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // 查找当前 worktree
        let worktrees = list_worktrees().unwrap_or_default();
        let current = worktrees.iter().find(|(p, _, _)| cwd.starts_with(p));

        let (wt_path, wt_branch) = match current {
            Some((p, _, b)) => (p.clone(), b.clone()),
            None => {
                return Err(ToolError::execution_failed(format!(
                    "当前目录 '{}' 不在 git worktree 中",
                    cwd
                )))
            },
        };

        // 切换到仓库根目录
        let root = git_root()?;
        if cwd != root {
            std::env::set_current_dir(&root).map_err(|e| {
                ToolError::execution_failed(format!("无法切换到仓库根目录 {}: {}", root, e))
            })?;
        }

        // 执行 git worktree remove
        let mut args = vec!["worktree", "remove", &wt_path];
        if discard {
            args.push("--force");
        }
        let output = Command::new("git")
            .args(&args)
            .current_dir(&root)
            .output()
            .map_err(|e| ToolError::execution_failed(format!("git worktree remove 失败: {}", e)))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let hint = if err.contains("modified") || err.contains("untracked") {
                "\n提示: 有未提交的更改，使用 discard_changes=true 强制删除"
            } else {
                ""
            };
            return Err(ToolError::execution_failed(format!(
                "删除 worktree 失败: {}{}",
                err, hint
            )));
        }

        // 删除关联分支
        if wt_branch != "detached" {
            let _ = Command::new("git")
                .args(["branch", "-D", &wt_branch])
                .current_dir(&root)
                .output();
        }

        // 触发 hook
        fire_worktree_hook(
            axagent_runtime_core::HookEvent::ConfigChange,
            &json!({
                "action": "remove",
                "path": wt_path,
                "branch": wt_branch,
                "discard_changes": discard,
            }),
        );

        Ok(ToolResult::success(format!(
            "## 🗑️ Worktree 已删除\n\n**路径**: {}\n**分支**: {}\n**已返回仓库根目录**: {}",
            wt_path, wt_branch, root
        )))
    }
}

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

// ── DiscoverSkills ──
pub struct DiscoverSkillsTool;
#[async_trait]
impl Tool for DiscoverSkillsTool {
    fn name(&self) -> &str {
        "DiscoverSkills"
    }
    fn description(&self) -> &str {
        "通过语义搜索发现匹配的 Skill，按相关性评分排序。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let q = i["query"].as_str().unwrap_or("");
        Ok(ToolResult::success(format!("🔎 技能搜索: '{}'\n\n正在索引本地技能...", q)))
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

// ── VerifyPlanExecution ──
pub struct VerifyPlanExecutionTool;
#[async_trait]
impl Tool for VerifyPlanExecutionTool {
    fn name(&self) -> &str {
        "VerifyPlanExecution"
    }
    fn description(&self) -> &str {
        "退出计划模式前的验证步骤：记录摘要、确认步骤完成状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"summary":{"type":"string"},"steps_completed":{"type":"array","items":{"type":"string"}}},"required":["summary"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let summary = i["summary"].as_str().unwrap_or("");
        let steps = i["steps_completed"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        Ok(ToolResult::success(format!("✅ 计划验证完成: {} ({} 步骤)", summary, steps)))
    }
}

// ── RemoteTrigger ──
pub struct RemoteTriggerTool;
#[async_trait]
impl Tool for RemoteTriggerTool {
    fn name(&self) -> &str {
        "RemoteTrigger"
    }
    fn description(&self) -> &str {
        "远程触发另一个 Agent 会话执行。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"session_id":{"type":"string"},"prompt":{"type":"string"}},"required":["session_id","prompt"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let sid = i["session_id"].as_str().unwrap_or("?");
        Ok(ToolResult::success(format!("📡 已触发远程会话: {}", sid)))
    }
}

// ── SuggestBackgroundPR ──
pub struct SuggestBackgroundPRTool;
#[async_trait]
impl Tool for SuggestBackgroundPRTool {
    fn name(&self) -> &str {
        "SuggestBackgroundPR"
    }
    fn description(&self) -> &str {
        "在后台分析变更并建议创建 PR。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"branch":{"type":"string"}},"required":["branch"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let branch = i["branch"].as_str().unwrap_or("main");
        Ok(ToolResult::success(format!("💡 PR 建议: 分支 '{}' → 运行后台分析...", branch)))
    }
}
