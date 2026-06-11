// SPDX-License-Identifier: AGPL-3.0-only

//! Git 操作工具
//!
//! 将 builtin_handlers 中的 git_status、git_diff、git_commit、
//! git_log、git_branch、git_review 迁移为 Tool trait 实现。
//! 实际 Git 操作委托给 axagent_core::git_tools::GitTools。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

// ── GitStatusTool ──────────────────────────────────────────────────────────

pub struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "GitStatus"
    }

    fn description(&self) -> &str {
        "获取 Git 仓库的当前状态，包括已修改、已暂存和未跟踪的文件。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "repo_path": { "type": "string", "description": "Git 仓库路径" }
            },
            "required": ["repo_path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Vcs
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let repo_path = input
            .get("repo_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if repo_path.is_empty() {
            return Ok(ToolResult::error("Error: repo_path 参数是必需的"));
        }
        match axagent_core::git_tools::GitTools::get_status(repo_path) {
            Ok(entries) => {
                let output: Vec<Value> = entries
                    .iter()
                    .map(|e| serde_json::json!({ "path": e.path, "status": e.status, "staged": e.staged }))
                    .collect();
                Ok(ToolResult::success(serde_json::to_string(&output).unwrap_or_default()))
            },
            Err(e) => Ok(ToolResult::error(format!("Error: {}", e))),
        }
    }
}

// ── GitDiffTool ────────────────────────────────────────────────────────────

pub struct GitDiffTool;

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "GitDiff"
    }

    fn description(&self) -> &str {
        "获取 Git 仓库的差异，包括暂存变更或分支差异。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "repo_path": { "type": "string", "description": "Git 仓库路径" },
                "base_branch": { "type": "string", "description": "对比的基础分支（可选，默认使用暂存区）" }
            },
            "required": ["repo_path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Vcs
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let repo_path = input
            .get("repo_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if repo_path.is_empty() {
            return Ok(ToolResult::error("Error: repo_path 参数是必需的"));
        }
        let base_branch = input.get("base_branch").and_then(|v| v.as_str());
        let result = match base_branch {
            Some(b) => axagent_core::git_tools::GitTools::get_branch_diff(repo_path, b),
            None => axagent_core::git_tools::GitTools::get_staged_diff(repo_path),
        };
        match result {
            Ok(diff) => Ok(ToolResult::success(serde_json::to_string(&diff).unwrap_or_default())),
            Err(e) => Ok(ToolResult::error(format!("Error: {}", e))),
        }
    }
}

// ── GitCommitTool ──────────────────────────────────────────────────────────

pub struct GitCommitTool;

#[async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &str {
        "GitCommit"
    }

    fn description(&self) -> &str {
        "在 Git 仓库中创建提交，可选择先暂存所有文件。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "repo_path": { "type": "string", "description": "Git 仓库路径" },
                "message": { "type": "string", "description": "提交信息" },
                "stage_all": { "type": "boolean", "description": "是否先暂存所有文件", "default": false }
            },
            "required": ["repo_path", "message"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Vcs
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let repo_path = input
            .get("repo_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let message = input
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let stage_all = input
            .get("stage_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if repo_path.is_empty() {
            return Ok(ToolResult::error("Error: repo_path 参数是必需的"));
        }
        if message.is_empty() {
            return Ok(ToolResult::error("Error: message 参数是必需的"));
        }
        if stage_all && let Err(e) = axagent_core::git_tools::GitTools::stage_all(repo_path) {
            return Ok(ToolResult::error(format!("暂存文件失败: {}", e)));
        }
        match axagent_core::git_tools::GitTools::commit(repo_path, message) {
            Ok(output) => Ok(ToolResult::success(output)),
            Err(e) => Ok(ToolResult::error(format!("Error: {}", e))),
        }
    }
}

// ── GitLogTool ─────────────────────────────────────────────────────────────

pub struct GitLogTool;

#[async_trait]
impl Tool for GitLogTool {
    fn name(&self) -> &str {
        "GitLog"
    }

    fn description(&self) -> &str {
        "获取 Git 仓库最近的提交历史。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "repo_path": { "type": "string", "description": "Git 仓库路径" },
                "max_count": { "type": "integer", "description": "最大返回提交数", "default": 10 }
            },
            "required": ["repo_path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Vcs
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let repo_path = input
            .get("repo_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let max_count = input
            .get("max_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        if repo_path.is_empty() {
            return Ok(ToolResult::error("Error: repo_path 参数是必需的"));
        }
        match axagent_core::git_tools::GitTools::get_log(repo_path, max_count) {
            Ok(entries) => {
                Ok(ToolResult::success(serde_json::to_string(&entries).unwrap_or_default()))
            },
            Err(e) => Ok(ToolResult::error(format!("Error: {}", e))),
        }
    }
}

// ── GitBranchTool ──────────────────────────────────────────────────────────

pub struct GitBranchTool;

#[async_trait]
impl Tool for GitBranchTool {
    fn name(&self) -> &str {
        "GitBranch"
    }

    fn description(&self) -> &str {
        "列出、创建或切换 Git 分支。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "repo_path": { "type": "string", "description": "Git 仓库路径" },
                "action": { "type": "string", "enum": ["list", "create", "switch"], "description": "操作类型" },
                "name": { "type": "string", "description": "分支名称（create/switch 时需要）" }
            },
            "required": ["repo_path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Vcs
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let repo_path = input
            .get("repo_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("list");
        let name = input.get("name").and_then(|v| v.as_str());

        if repo_path.is_empty() {
            return Ok(ToolResult::error("Error: repo_path 参数是必需的"));
        }

        match action {
            "list" => match axagent_core::git_tools::GitTools::list_branches(repo_path) {
                Ok(branches) => {
                    Ok(ToolResult::success(serde_json::to_string(&branches).unwrap_or_default()))
                },
                Err(e) => Ok(ToolResult::error(format!("Error: {}", e))),
            },
            "create" => match name {
                Some(n) => match axagent_core::git_tools::GitTools::create_branch(repo_path, n) {
                    Ok(o) => Ok(ToolResult::success(format!("已创建并切换到分支 '{}': {}", n, o))),
                    Err(e) => Ok(ToolResult::error(format!("Error: {}", e))),
                },
                None => Ok(ToolResult::error("Error: create 操作需要 name 参数")),
            },
            "switch" => match name {
                Some(n) => match axagent_core::git_tools::GitTools::switch_branch(repo_path, n) {
                    Ok(o) => Ok(ToolResult::success(format!("已切换到分支 '{}': {}", n, o))),
                    Err(e) => Ok(ToolResult::error(format!("Error: {}", e))),
                },
                None => Ok(ToolResult::error("Error: switch 操作需要 name 参数")),
            },
            _ => Ok(ToolResult::error(format!(
                "Error: 未知操作 '{}'。请使用 'list'、'create' 或 'switch'",
                action
            ))),
        }
    }
}

// ── GitReviewTool ──────────────────────────────────────────────────────────

pub struct GitReviewTool;

#[async_trait]
impl Tool for GitReviewTool {
    fn name(&self) -> &str {
        "GitReview"
    }

    fn description(&self) -> &str {
        "生成 Git 代码审查上下文摘要，包括变更文件列表、差异等信息。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "repo_path": { "type": "string", "description": "Git 仓库路径" },
                "base_branch": { "type": "string", "description": "对比的基础分支（可选）" }
            },
            "required": ["repo_path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Vcs
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let repo_path = input
            .get("repo_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let base_branch = input.get("base_branch").and_then(|v| v.as_str());

        if repo_path.is_empty() {
            return Ok(ToolResult::error("Error: repo_path 参数是必需的"));
        }
        let context = match base_branch {
            Some(b) => axagent_core::git_tools::GitTools::generate_pr_context(repo_path, b),
            None => axagent_core::git_tools::GitTools::generate_commit_context(repo_path),
        };
        match context {
            Ok(ctx) => Ok(ToolResult::success(ctx)),
            Err(e) => Ok(ToolResult::error(format!("Error: {}", e))),
        }
    }
}
