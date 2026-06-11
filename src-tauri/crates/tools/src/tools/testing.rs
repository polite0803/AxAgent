// SPDX-License-Identifier: AGPL-3.0-only

//! 测试运行工具
//!
//! RunTests / RunTestCoverage / RunLinter

use crate::utils::spawn::safe_spawn;
use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

const DEFAULT_TIMEOUT: u64 = 300;

fn run_command(cmd: &str, args: &[&str], cwd: &str, _timeout_secs: u64) -> Result<String, String> {
    let child = safe_spawn(Command::new(cmd).args(args).current_dir(cwd))
        .map_err(|e| format!("命令启动失败: {}", e))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("等待命令结束失败: {}", e))?;

    // 忽略 timeout_secs 参数以简化（std::process::Command 不直接支持 timeout）

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&format!("## stdout\n\n```\n{}\n```\n", stdout));
    }
    if !stderr.is_empty() {
        result.push_str(&format!("\n## stderr\n\n```\n{}\n```\n", stderr));
    }
    if !output.status.success() {
        result.push_str(&format!("\n## 退出码: {} (失败)\n", output.status.code().unwrap_or(-1)));
    } else {
        result.push_str("\n## 退出码: 0 (成功)\n");
    }
    Ok(result)
}

fn detect_package_manager(cwd: &str) -> &'static str {
    let p = std::path::Path::new(cwd);
    if p.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if p.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm"
    }
}

// ── RunTestsTool ──

pub struct RunTestsTool;

#[async_trait]
impl Tool for RunTestsTool {
    fn name(&self) -> &str {
        "RunTests"
    }
    fn description(&self) -> &str {
        "运行项目的测试套件。自动检测包管理器(npm/yarn/pnpm)和测试框架(Vitest/Jest/Playwright/Cargo)。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "scope": {
                    "type": "string",
                    "enum": ["unit", "e2e", "all"],
                    "description": "测试范围: unit(单元), e2e(端到端), all(全部)",
                    "default": "unit"
                },
                "working_dir": {
                    "type": "string",
                    "description": "工作目录（默认当前目录）"
                }
            },
            "required": []
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Shell
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let scope = input
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or("unit");
        let wd = input
            .get("working_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.working_dir);

        let is_rust = std::path::Path::new(wd).join("Cargo.toml").exists();

        let result = if is_rust {
            match scope {
                "unit" => run_command("cargo", &["test"], wd, DEFAULT_TIMEOUT),
                "e2e" => run_command("cargo", &["test", "--test", "*"], wd, DEFAULT_TIMEOUT),
                _ => {
                    let unit = run_command("cargo", &["test"], wd, DEFAULT_TIMEOUT / 2)
                        .unwrap_or_default();
                    let e2e =
                        run_command("cargo", &["test", "--test", "*"], wd, DEFAULT_TIMEOUT / 2)
                            .unwrap_or_default();
                    Ok(format!("{}\n{}", unit, e2e))
                },
            }
        } else {
            let mgr = detect_package_manager(wd);
            match scope {
                "unit" => run_command(mgr, &["run", "test:run"], wd, DEFAULT_TIMEOUT),
                "e2e" => run_command(mgr, &["run", "test:e2e"], wd, DEFAULT_TIMEOUT),
                _ => run_command(mgr, &["run", "test"], wd, DEFAULT_TIMEOUT),
            }
        };

        match result {
            Ok(output) => Ok(ToolResult::success(format!("## 测试执行: {}\n\n{}", scope, output))),
            Err(e) => Ok(ToolResult::error(format!("测试执行失败: {}", e))),
        }
    }
}

// ── RunLinterTool ──

pub struct RunLinterTool;

#[async_trait]
impl Tool for RunLinterTool {
    fn name(&self) -> &str {
        "RunLinter"
    }
    fn description(&self) -> &str {
        "运行代码质量检查：cargo clippy (Rust) 或 TypeScript typecheck。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "enum": ["rust", "typescript", "all"],
                    "description": "检查语言",
                    "default": "all"
                },
                "working_dir": { "type": "string" }
            }
        })
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
        let lang = input
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        let wd = input
            .get("working_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.working_dir);

        let mut results = vec!["## 代码检查\n".to_string()];

        if lang == "rust" || lang == "all" {
            let rust_dir = std::path::Path::new(wd).join("src-tauri");
            if rust_dir.exists() {
                match run_command(
                    "cargo",
                    &["clippy", "--", "-D", "warnings"],
                    &rust_dir.to_string_lossy(),
                    300,
                ) {
                    Ok(o) => results.push(format!("### Rust (clippy)\n{}", o)),
                    Err(e) => results.push(format!("### Rust (clippy)\n失败: {}", e)),
                }
            }
        }

        if lang == "typescript" || lang == "all" {
            let mgr = detect_package_manager(wd);
            match run_command(mgr, &["run", "typecheck"], wd, 120) {
                Ok(o) => results.push(format!("### TypeScript (tsc)\n{}", o)),
                Err(e) => results.push(format!("### TypeScript (tsc)\n失败: {}", e)),
            }
        }

        Ok(ToolResult::success(results.join("\n\n")))
    }
}

// ── RunTestCoverageTool ──

pub struct RunTestCoverageTool;

#[async_trait]
impl Tool for RunTestCoverageTool {
    fn name(&self) -> &str {
        "RunTestCoverage"
    }
    fn description(&self) -> &str {
        "运行测试覆盖率报告。TypeScript: vitest --coverage, Rust: cargo llvm-cov。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "enum": ["rust", "typescript"],
                    "description": "语言"
                },
                "working_dir": { "type": "string" }
            },
            "required": ["language"]
        })
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
        let lang = input
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("typescript");
        let wd = input
            .get("working_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.working_dir);

        let result = match lang {
            "rust" => {
                let rust_dir = std::path::Path::new(wd).join("src-tauri");
                run_command("cargo", &["llvm-cov", "--html"], &rust_dir.to_string_lossy(), 300)
            },
            _ => {
                let mgr = detect_package_manager(wd);
                run_command(mgr, &["run", "test:coverage"], wd, 180)
            },
        };

        match result {
            Ok(o) => Ok(ToolResult::success(format!("## 覆盖率 ({})\n\n{}", lang, o))),
            Err(e) => Ok(ToolResult::error(format!("覆盖率运行失败: {}", e))),
        }
    }
}
