// SPDX-License-Identifier: AGPL-3.0-only

//! CtxInspectTool / SnipTool / ContextResolveTool - 上下文管理工具

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

const FILE_REF_SIZE_LIMIT: usize = 100 * 1024;
const URL_REF_SIZE_LIMIT: usize = 50 * 1024;
const URL_REF_TIMEOUT_SECS: u64 = 30;

pub struct CtxInspectTool;
pub struct SnipTool;
pub struct ContextResolveTool;

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

#[async_trait]
impl Tool for ContextResolveTool {
    fn name(&self) -> &str {
        "ContextResolve"
    }
    fn description(&self) -> &str {
        "解析文本中的上下文引用：@file:path 读取文件内容、@url:https://... 获取网页内容、@skill:name 加载 Skill 描述，以及处理 <!-- if:... --> 条件区块。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "包含引用标记的文本内容"
                },
                "base_dir": {
                    "type": "string",
                    "description": "@file: 引用的基准目录，默认为当前工作目录"
                }
            },
            "required": ["text"]
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
        let text = input["text"].as_str().unwrap_or("").to_string();

        if text.is_empty() {
            return Err(ToolError::invalid_input("text 参数不能为空"));
        }

        let base_dir = input
            .get("base_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.working_dir);

        let base_path = Path::new(base_dir);
        let resolved = resolve_references_impl(&text, base_path).await;

        Ok(ToolResult::success(resolved))
    }
}

async fn resolve_references_impl(content: &str, base_dir: &Path) -> String {
    let content = resolve_file_refs(content, base_dir);
    let content = resolve_url_refs(&content).await;
    let content = resolve_skill_refs(&content);
    strip_conditional_blocks(&content)
}

fn resolve_file_refs(content: &str, base_dir: &Path) -> String {
    let re = regex::Regex::new(r"@file:([^\s]+)").unwrap();
    re.replace_all(content, |caps: &regex::Captures| {
        let ref_path = &caps[1];
        let full_path = base_dir.join(ref_path);
        match std::fs::read_to_string(&full_path) {
            Ok(file_content) => {
                if file_content.len() > FILE_REF_SIZE_LIMIT {
                    format!(
                        "[Error: file '{}' exceeds {}KB size limit]",
                        ref_path,
                        FILE_REF_SIZE_LIMIT / 1024
                    )
                } else {
                    file_content
                }
            },
            Err(e) => format!("[Error reading file '{}': {}]", ref_path, e),
        }
    })
    .to_string()
}

async fn resolve_url_refs(content: &str) -> String {
    let re = regex::Regex::new(r"@url:(https?://[^\s]+)").unwrap();
    let mut result = content.to_string();

    let caps: Vec<_> = re.captures_iter(content).collect();
    for cap in caps {
        let url = &cap[1];
        let placeholder = format!("@url:{}", url);
        let replacement = match fetch_url_for_ref(url).await {
            Ok(body) => body,
            Err(e) => format!("[Error fetching URL '{}': {}]", url, e),
        };
        result = result.replacen(&placeholder, &replacement, 1);
    }

    result
}

async fn fetch_url_for_ref(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(URL_REF_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(url).send().await.map_err(|e| e.to_string())?;

    let body = response.text().await.map_err(|e| e.to_string())?;

    if body.len() > URL_REF_SIZE_LIMIT {
        Err(format!(
            "response exceeds {}KB size limit ({} bytes)",
            URL_REF_SIZE_LIMIT / 1024,
            body.len()
        ))
    } else {
        Ok(body)
    }
}

fn resolve_skill_refs(content: &str) -> String {
    let re = regex::Regex::new(r"@skill:([a-zA-Z0-9_-]+)").unwrap();
    let dirs = axagent_core::skill_dirs::skill_dirs();

    re.replace_all(content, |caps: &regex::Captures| {
        let skill_name = &caps[1];
        for (_kind, dir) in &dirs {
            let skill_md = dir.join(skill_name).join("SKILL.md");
            if skill_md.exists()
                && let Ok(md_content) = std::fs::read_to_string(&skill_md)
            {
                let first_line = md_content.lines().next().unwrap_or(skill_name);
                return first_line.to_string();
            }
        }
        format!("[Skill '{}' not found]", skill_name)
    })
    .to_string()
}

fn strip_conditional_blocks(content: &str) -> String {
    let re = regex::Regex::new(r"<!--\s*if:(\w+):(\w+)\s*-->([\s\S]*?)<!--\s*endif\s*-->").unwrap();

    re.replace_all(content, |caps: &regex::Captures| {
        let condition_type = &caps[1];
        let condition_value = &caps[2];
        let inner_content = &caps[3];

        if eval_condition(condition_type, condition_value) {
            inner_content.to_string()
        } else {
            String::new()
        }
    })
    .to_string()
}

fn eval_condition(condition_type: &str, condition_value: &str) -> bool {
    match condition_type {
        "platform" => std::env::consts::OS == condition_value,
        "toolset" => is_toolset_available(condition_value),
        "personality" => std::env::var("AXAGENT_PERSONALITY")
            .unwrap_or_default()
            .eq(condition_value),
        _ => false,
    }
}

fn is_toolset_available(toolset: &str) -> bool {
    matches!(
        toolset,
        "web" | "file" | "shell" | "git" | "network" | "system" | "browser" | "database"
    )
}
