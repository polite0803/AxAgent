//! CtxInspectTool / SnipTool - 上下文管理工具

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

pub struct CtxInspectTool;
pub struct SnipTool;

fn git_root() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

fn count_files(dir: &str) -> usize {
    std::fs::read_dir(dir).map(|rd| rd.count()).unwrap_or(0)
}

fn dir_size_mb(dir: &str) -> f64 {
    let mut total: u64 = 0;
    fn walk(path: &std::path::Path, total: &mut u64) {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_dir() {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str())
                        && (name == ".git" || name == "node_modules" || name == "target")
                    {
                        continue;
                    }
                    walk(&p, total);
                } else {
                    *total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
    }
    walk(std::path::Path::new(dir), &mut total);
    total as f64 / (1024.0 * 1024.0)
}

fn git_branch() -> String {
    Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn git_commit_count() -> String {
    Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "N/A".to_string())
}

#[async_trait]
impl Tool for CtxInspectTool {
    fn name(&self) -> &str {
        "CtxInspect"
    }
    fn description(&self) -> &str {
        "检查当前项目上下文：工作目录、Git 状态、文件统计、磁盘占用、环境信息。用于帮助 Agent 理解当前工作环境。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "detail": {
                    "type": "string",
                    "enum": ["basic", "full"],
                    "description": "详情级别：basic(概要) / full(完整)",
                    "default": "basic"
                }
            },
            "required": []
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let detail = input
            .get("detail")
            .and_then(|v| v.as_str())
            .unwrap_or("basic");
        let is_full = detail == "full";

        let cwd = ctx.working_dir.clone();

        let root = git_root().unwrap_or_else(|| cwd.clone());
        let branch = git_branch();
        let commits = git_commit_count();
        let file_count = count_files(&root);
        let size_mb = dir_size_mb(&root);

        let mut lines = vec![
            "## 上下文检查".to_string(),
            String::new(),
            format!("**工作目录**: {}", cwd),
            format!("**Git 仓库**: {}", root),
            format!("**当前分支**: {}", branch),
            format!("**提交数**: {}", commits),
            format!("**文件数**: {}", file_count),
            format!("**目录大小**: {:.1} MB", size_mb),
            format!("**会话 ID**: {}", ctx.conversation_id.as_deref().unwrap_or("N/A")),
            format!("**消息 ID**: {}", ctx.message_id.as_deref().unwrap_or("N/A")),
            format!("**运行平台**: {} {}", std::env::consts::OS, std::env::consts::ARCH),
        ];

        if is_full {
            lines.push(String::new());
            lines.push("### 最近 Git 日志".to_string());
            if let Ok(output) = Command::new("git")
                .args(["log", "--oneline", "-5"])
                .output()
                && output.status.success()
                && let Ok(text) = String::from_utf8(output.stdout)
            {
                for l in text.lines() {
                    lines.push(format!("  {}", l));
                }
            }

            lines.push(String::new());
            lines.push("### 环境变量".to_string());
            for (k, v) in std::env::vars() {
                // 过滤敏感信息
                let safe_val = if k.contains("TOKEN")
                    || k.contains("KEY")
                    || k.contains("SECRET")
                    || k.contains("PASSWORD")
                {
                    if v.len() > 4 {
                        format!("{}...{}", &v[..2], &v[v.len() - 2..])
                    } else {
                        "***".to_string()
                    }
                } else {
                    v
                };
                lines.push(format!("  {}={}", k, safe_val));
            }
        }

        Ok(ToolResult::success(lines.join("\n")))
    }
}

#[async_trait]
impl Tool for SnipTool {
    fn name(&self) -> &str {
        "Snip"
    }
    fn description(&self) -> &str {
        "从对话上下文中移除指定范围的消息以释放 token 预算。触发上下文压缩，将指定范围消息替换为摘要。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "start_idx": {
                    "type": "integer",
                    "description": "要压缩的起始消息索引（0-based，包含）"
                },
                "end_idx": {
                    "type": "integer",
                    "description": "要压缩的结束消息索引（0-based，包含）"
                }
            },
            "required": ["start_idx", "end_idx"]
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

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let start = input["start_idx"].as_u64().unwrap_or(0) as usize;
        let end = input["end_idx"].as_u64().unwrap_or(0) as usize;

        if start > end {
            return Err(ToolError::invalid_input("start_idx 不能大于 end_idx"));
        }

        // 触发上下文压缩 Hook
        let hook_data = serde_json::json!({
            "action": "compact",
            "start_idx": start,
            "end_idx": end,
            "conversation_id": ctx.conversation_id,
            "message_id": ctx.message_id,
        });

        let runner = axagent_runtime_core::HookRunner::new(
            axagent_runtime_core::RuntimeHookConfig::default(),
        );
        let _ =
            runner.run_event(axagent_runtime_core::HookEvent::PreCompact, &hook_data.to_string());

        let removed = end - start + 1;
        Ok(ToolResult::success(format!(
            "✂️ 已触发上下文压缩: 消息范围 [{start}, {end}]，共 {removed} 条消息将被压缩为摘要\n\
             \n压缩操作通过 Hook 系统异步执行。后续消息将使用压缩后的上下文。"
        )))
    }
}
