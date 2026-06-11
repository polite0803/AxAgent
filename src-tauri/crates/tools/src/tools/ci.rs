// SPDX-License-Identifier: AGPL-3.0-only

//! CI/CD 工具
//!
//! CiStatus / CiTrigger / CiListWorkflows

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

fn has_gh_cli() -> bool {
    Command::new("gh")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── CiStatusTool ──

pub struct CiStatusTool;

#[async_trait]
impl Tool for CiStatusTool {
    fn name(&self) -> &str {
        "CiStatus"
    }
    fn description(&self) -> &str {
        "查看最近的 CI 运行状态。使用 gh CLI 查询 GitHub Actions 状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "branch": {
                    "type": "string",
                    "description": "分支名（默认当前分支）"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多返回的运行次数",
                    "default": 5
                }
            }
        })
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
        if !has_gh_cli() {
            return Err(ToolError::execution_failed(
                "gh CLI 未安装或未配置。请运行 'gh auth login' 配置 GitHub CLI。",
            ));
        }

        let branch = input.get("branch").and_then(|v| v.as_str());
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .to_string();

        let mut args = vec!["run", "list", "--limit", &limit];
        if let Some(b) = branch {
            args.extend(["--branch", b]);
        }

        let output = Command::new("gh")
            .args(&args)
            .output()
            .map_err(|e| ToolError::execution_failed(format!("gh 命令执行失败: {}", e)))?;

        let text = String::from_utf8_lossy(&output.stdout).to_string();
        if text.trim().is_empty() {
            return Ok(ToolResult::success("暂无 CI 运行记录"));
        }

        Ok(ToolResult::success(format!("## CI 运行状态\n\n```\n{}\n```", text)))
    }
}

// ── CiTriggerTool ──

pub struct CiTriggerTool;

#[async_trait]
impl Tool for CiTriggerTool {
    fn name(&self) -> &str {
        "CiTrigger"
    }
    fn description(&self) -> &str {
        "手动触发 CI 工作流运行。需要 gh CLI 和工作流 dispatch 权限。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "workflow": {
                    "type": "string",
                    "description": "工作流文件名 (如 pr-ci.yml, release.yml)"
                },
                "ref": {
                    "type": "string",
                    "description": "分支或标签（默认当前分支）",
                    "default": "HEAD"
                }
            },
            "required": ["workflow"]
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
        if !has_gh_cli() {
            return Err(ToolError::execution_failed(
                "gh CLI 未安装或未配置。请运行 'gh auth login' 配置 GitHub CLI。",
            ));
        }

        let workflow = input["workflow"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input("缺少 workflow 参数"))?;
        let git_ref = input["ref"].as_str().unwrap_or("HEAD");

        let output = Command::new("gh")
            .args(["workflow", "run", workflow, "--ref", git_ref])
            .output()
            .map_err(|e| ToolError::execution_failed(format!("触发 CI 失败: {}", e)))?;

        if output.status.success() {
            Ok(ToolResult::success(format!(
                "✅ 已触发 CI 工作流: {} (ref: {})",
                workflow, git_ref
            )))
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(ToolError::execution_failed(format!("触发 CI 失败: {}", err)))
        }
    }
}

// ── CiListWorkflowsTool ──

pub struct CiListWorkflowsTool;

#[async_trait]
impl Tool for CiListWorkflowsTool {
    fn name(&self) -> &str {
        "CiListWorkflows"
    }
    fn description(&self) -> &str {
        "列出仓库中可用的 CI 工作流。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": {} })
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

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        if !has_gh_cli() {
            return Err(ToolError::execution_failed(
                "gh CLI 未安装或未配置。请运行 'gh auth login' 配置 GitHub CLI。",
            ));
        }

        let output = Command::new("gh")
            .args(["workflow", "list"])
            .output()
            .map_err(|e| ToolError::execution_failed(format!("gh 命令执行失败: {}", e)))?;

        let text = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(ToolResult::success(format!("## 可用 CI 工作流\n\n```\n{}\n```", text)))
    }
}
