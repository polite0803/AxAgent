//! Git Worktree 隔离工具
//!
//! EnterWorktree (创建隔离工作目录 + 分支), ExitWorktree (keep/remove)

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::process::Command;

fn fire_hook(event: axagent_runtime_core::HookEvent, data: &serde_json::Value) {
    let runner =
        axagent_runtime_core::HookRunner::new(axagent_runtime_core::RuntimeHookConfig::default());
    let _ = runner.run_event(event, &data.to_string());
}

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
        serde_json::json!({"type":"object","properties":{"name":{"type":"string","description":"worktree 名称，字母/数字/横线，最多 64 字符"}},"required":[]})
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

        let existing = list_worktrees().unwrap_or_default();
        if existing.iter().any(|(p, _, _)| p == &worktree_path) {
            return Err(ToolError::invalid_input(format!("worktree '{}' 已存在", sanitized)));
        }
        let branch_status = Command::new("git")
            .args(["branch", &branch_name, &base])
            .current_dir(&root)
            .output()
            .map_err(|e| ToolError::execution_failed(format!("创建分支失败: {}", e)))?;
        if !branch_status.status.success() {
            let err = String::from_utf8_lossy(&branch_status.stderr);
            return Err(ToolError::execution_failed(format!("创建分支失败: {}", err)));
        }
        let output = Command::new("git")
            .args(["worktree", "add", &worktree_path, &branch_name])
            .current_dir(&root)
            .output()
            .map_err(|e| ToolError::execution_failed(format!("git worktree add 失败: {}", e)))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let _ = Command::new("git")
                .args(["branch", "-D", &branch_name])
                .current_dir(&root)
                .output();
            return Err(ToolError::execution_failed(format!("创建 worktree 失败: {}", err)));
        }
        fire_hook(
            axagent_runtime_core::HookEvent::ConfigChange,
            &json!({
                "name": sanitized, "branch": branch_name, "path": worktree_path, "root": root,
            }),
        );
        Ok(ToolResult::success(format!(
            "## 🌳 Worktree 已创建\n\n**名称**: {}\n**分支**: {}\n**路径**: {}\n**基础分支**: {}",
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
        "退出 worktree 会话。remove: 删除目录及关联分支；keep: 仅离开（保留文件）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["keep","remove"]},"discard_changes":{"type":"boolean","default":false}},"required":["action"]})
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
        if action != "remove" {
            fire_hook(axagent_runtime_core::HookEvent::ConfigChange, &json!({"action": "keep"}));
            return Ok(ToolResult::success("📤 已离开 worktree（文件已保留）"));
        }
        let discard = i["discard_changes"].as_bool().unwrap_or(false);
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let worktrees = list_worktrees().unwrap_or_default();
        let current = worktrees.iter().find(|(p, _, _)| cwd.starts_with(p));
        let (wt_path, wt_branch) = match current {
            Some((p, _, b)) => (p.clone(), b.clone()),
            None => {
                return Err(ToolError::execution_failed(format!(
                    "当前目录 '{}' 不在 worktree 中",
                    cwd
                )));
            },
        };
        let root = git_root()?;
        if cwd != root {
            std::env::set_current_dir(&root).map_err(|e| {
                ToolError::execution_failed(format!("无法切换到根目录 {}: {}", root, e))
            })?;
        }
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
                "\n提示: 有未提交更改，使用 discard_changes=true 强制删除"
            } else {
                ""
            };
            return Err(ToolError::execution_failed(format!("删除失败: {}{}", err, hint)));
        }
        if wt_branch != "detached" {
            let _ = Command::new("git")
                .args(["branch", "-D", &wt_branch])
                .current_dir(&root)
                .output();
        }
        fire_hook(
            axagent_runtime_core::HookEvent::ConfigChange,
            &json!({
                "action": "remove", "path": wt_path, "branch": wt_branch, "discard_changes": discard,
            }),
        );
        Ok(ToolResult::success(format!(
            "## 🗑️ Worktree 已删除\n\n**路径**: {}\n**分支**: {}\n**已返回根目录**: {}",
            wt_path, wt_branch, root
        )))
    }
}
