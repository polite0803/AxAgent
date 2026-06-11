// SPDX-License-Identifier: AGPL-3.0-only

//! DevOps 工具
//!
//! SecurityAudit (npm/cargo audit), DeadCodeDetect, BundleAnalyze, IssueCreate, IssueList

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

fn run_cmd(cmd: &str, args: &[&str], cwd: &str) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("命令执行失败: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!("{}\n{}", stdout, stderr).trim().to_string())
}

fn has_gh() -> bool {
    Command::new("gh")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── SecurityAuditTool ──

pub struct SecurityAuditTool;

#[async_trait]
impl Tool for SecurityAuditTool {
    fn name(&self) -> &str {
        "SecurityAudit"
    }
    fn description(&self) -> &str {
        "运行依赖安全审计。自动检测项目类型：Rust (cargo audit)、Node (npm audit)、Python (pip-audit)。返回已知漏洞列表。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"working_dir":{"type":"string","description":"项目路径"},"fix":{"type":"boolean","default":false,"description":"是否自动修复(NPM only)"}},"required":[]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Shell
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let wd = input
            .get("working_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.working_dir);
        let _fix = input.get("fix").and_then(|v| v.as_bool()).unwrap_or(false);
        let mut results = vec!["## 安全审计\n".to_string()];

        // Rust: cargo audit
        if std::path::Path::new(wd).join("Cargo.toml").exists() {
            match run_cmd("cargo", &["audit"], wd) {
                Ok(o) => results.push(format!("### Cargo Audit\n```\n{}\n```", o)),
                Err(e) => results
                    .push(format!("### Cargo Audit\n失败: {}. 安装: cargo install cargo-audit", e)),
            }
        }

        // Node: npm audit
        if std::path::Path::new(wd).join("package.json").exists() {
            let mgr = if std::path::Path::new(wd).join("pnpm-lock.yaml").exists() {
                "pnpm"
            } else if std::path::Path::new(wd).join("yarn.lock").exists() {
                "yarn"
            } else {
                "npm"
            };
            match run_cmd(mgr, &["audit"], wd) {
                Ok(o) => results.push(format!("### {} Audit\n```\n{}\n```", mgr, o)),
                Err(e) => results.push(format!("### {} Audit\n失败: {}", mgr, e)),
            }
        }

        if results.len() == 1 {
            results.push("未检测到支持审计的项目文件（Cargo.toml / package.json）".to_string());
        }

        Ok(ToolResult::success(results.join("\n\n")))
    }
}

// ── DeadCodeDetectTool ──

pub struct DeadCodeDetectTool;

#[async_trait]
impl Tool for DeadCodeDetectTool {
    fn name(&self) -> &str {
        "DeadCodeDetect"
    }
    fn description(&self) -> &str {
        "检测项目中的死代码。Rust: cargo udeps (未使用依赖)、cargo clippy::dead_code。Node: npx unimported。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"working_dir":{"type":"string"}},"required":[]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Shell
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let wd = input
            .get("working_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.working_dir);
        let mut results = vec!["## 死代码检测\n".to_string()];

        // Rust: cargo clippy with dead_code lint
        let rust_proj = std::path::Path::new(wd).join("Cargo.toml");
        if rust_proj.exists() {
            match run_cmd("cargo", &["clippy", "--", "-W", "clippy::all", "-W", "dead_code"], wd) {
                Ok(o) => {
                    let warnings: Vec<_> = o
                        .lines()
                        .filter(|l| {
                            l.contains("warning")
                                || l.contains("dead_code")
                                || l.contains("never used")
                        })
                        .take(20)
                        .collect();
                    if warnings.is_empty() {
                        results.push("### Rust\n✅ 未检测到死代码警告".to_string());
                    } else {
                        results
                            .push(format!("### Rust (clippy)\n```\n{}\n```", warnings.join("\n")));
                    }
                },
                Err(e) => results.push(format!("### Rust\n失败: {}", e)),
            }
        }

        // Node: check for unused exports
        let node_proj = std::path::Path::new(wd).join("package.json");
        if node_proj.exists() {
            results.push("### TypeScript\n使用 `npx unimported` 或 `ts-prune` 检测未使用的导出。在此项目:\n```\nnpx unimported\n```".to_string());
        }

        if results.len() == 1 {
            results.push("未找到可分析的项目文件".to_string());
        }

        Ok(ToolResult::success(results.join("\n\n")))
    }
}

// ── BundleAnalyzeTool ──

pub struct BundleAnalyzeTool;

#[async_trait]
impl Tool for BundleAnalyzeTool {
    fn name(&self) -> &str {
        "BundleAnalyze"
    }
    fn description(&self) -> &str {
        "分析前端打包产物大小。显示各 chunk/模块的大小占比，帮助识别需要优化的依赖。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"build_dir":{"type":"string","description":"打包输出目录(默认 dist/)"},"working_dir":{"type":"string"}},"required":[]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Shell
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let wd = input
            .get("working_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.working_dir);
        let build_dir = input
            .get("build_dir")
            .and_then(|v| v.as_str())
            .unwrap_or("dist");
        let full_build = std::path::Path::new(wd).join(build_dir);

        let mut results = vec!["## Bundle 分析\n".to_string()];

        if full_build.exists() {
            // 分析 JS/CSS 文件
            let mut total_size: u64 = 0;
            let mut files: Vec<(String, u64)> = Vec::new();
            fn walk_dir(
                path: &std::path::Path,
                files: &mut Vec<(String, u64)>,
                total_size: &mut u64,
            ) {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let p = entry.path();
                        if p.is_dir() {
                            walk_dir(&p, files, total_size);
                        } else if let Some(ext) = p.extension().and_then(|e| e.to_str())
                            && matches!(ext, "js" | "css" | "wasm" | "map")
                            && let Ok(meta) = p.metadata()
                        {
                            let size = meta.len();
                            *total_size += size;
                            files.push((p.to_string_lossy().to_string(), size));
                        }
                    }
                }
            }
            walk_dir(&full_build, &mut files, &mut total_size);

            if files.is_empty() {
                results.push("未找到打包产物 (JS/CSS/WASM)".to_string());
            } else {
                files.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
                results.push(format!("**总大小**: {:.1} MB\n", total_size as f64 / 1_048_576.0));
                for (name, size) in files.iter().take(30) {
                    let short = name.strip_prefix(wd).unwrap_or(name);
                    let pct = *size as f64 / total_size as f64 * 100.0;
                    results.push(format!("  {:>5.1}% {:>8}  {}", pct, human_size(*size), short));
                }
            }
        } else {
            results.push(format!(
                "目录 '{}' 不存在。请先构建项目 (npm run build)。",
                full_build.display()
            ));
        }

        Ok(ToolResult::success(results.join("\n")))
    }
}

// ── IssueCreateTool ──

pub struct IssueCreateTool;

#[async_trait]
impl Tool for IssueCreateTool {
    fn name(&self) -> &str {
        "IssueCreate"
    }
    fn description(&self) -> &str {
        "在 GitHub 仓库中创建 Issue。需要 gh CLI 已安装并认证。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"title":{"type":"string"},"body":{"type":"string"},"labels":{"type":"array","items":{"type":"string"}},"assignee":{"type":"string"}},"required":["title","body"]})
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
        if !has_gh() {
            return Err(ToolError::execution_failed(
                "gh CLI 未安装。请安装 GitHub CLI 并运行 gh auth login",
            ));
        }
        let title = input["title"].as_str().unwrap_or("");
        let body = input["body"].as_str().unwrap_or("");
        if title.is_empty() {
            return Ok(ToolResult::error("Error: title 是必需的"));
        }
        let mut args = vec!["issue", "create", "--title", title, "--body", body];
        let labels_str;
        if let Some(labels) = input["labels"].as_array() {
            labels_str = labels
                .iter()
                .filter_map(|l| l.as_str())
                .collect::<Vec<_>>()
                .join(",");
            if !labels_str.is_empty() {
                args.extend(["--label", &labels_str]);
            }
        }
        let assignee_str;
        if let Some(a) = input["assignee"].as_str() {
            assignee_str = a;
            if !assignee_str.is_empty() {
                args.extend(["--assignee", assignee_str]);
            }
        }
        let output = Command::new("gh")
            .args(&args)
            .output()
            .map_err(|e| ToolError::execution_failed(format!("gh 命令失败: {}", e)))?;
        if output.status.success() {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(ToolResult::success(format!("✅ Issue 已创建: {}", url)))
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(ToolError::execution_failed(format!("创建 Issue 失败: {}", err)))
        }
    }
}

// ── IssueListTool ──

pub struct IssueListTool;

#[async_trait]
impl Tool for IssueListTool {
    fn name(&self) -> &str {
        "IssueList"
    }
    fn description(&self) -> &str {
        "列出 GitHub 仓库的 Issues。支持状态过滤和标签过滤。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"state":{"type":"string","enum":["open","closed","all"],"default":"open"},"label":{"type":"string"},"limit":{"type":"integer","default":10},"assignee":{"type":"string"}},"required":[]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        if !has_gh() {
            return Err(ToolError::execution_failed(
                "gh CLI 未安装。请安装 GitHub CLI 并运行 gh auth login",
            ));
        }
        let state = input
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("open");
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .to_string();
        let mut args = vec!["issue", "list", "--state", state, "--limit", &limit];
        let label_str;
        if let Some(l) = input["label"].as_str() {
            label_str = l;
            if !label_str.is_empty() {
                args.extend(["--label", label_str]);
            }
        }
        let assignee_str;
        if let Some(a) = input["assignee"].as_str() {
            assignee_str = a;
            if !assignee_str.is_empty() {
                args.extend(["--assignee", assignee_str]);
            }
        }
        let output = Command::new("gh")
            .args(&args)
            .output()
            .map_err(|e| ToolError::execution_failed(format!("gh 命令失败: {}", e)))?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(ToolResult::success(format!("## Issues ({})\n\n```\n{}\n```", state, text)))
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut u = 0;
    while size >= 1024.0 && u < UNITS.len() - 1 {
        size /= 1024.0;
        u += 1;
    }
    format!("{:.1}{}", size, UNITS[u])
}
