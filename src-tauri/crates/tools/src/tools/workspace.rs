// SPDX-License-Identifier: AGPL-3.0-only

//! 工作区记忆文件工具
//!
//! WorkspaceRead / WorkspaceWrite — 读写项目工作区记忆文件

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

fn truncate_text(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

pub struct WorkspaceReadTool;

#[async_trait]
impl Tool for WorkspaceReadTool {
    fn name(&self) -> &str {
        "WorkspaceRead"
    }
    fn description(&self) -> &str {
        "读取工作区记忆文件的内容（如 CLAUDE.md, GEMINI.md 等）。用于恢复项目上下文。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"filename":{"type":"string"},"workspace_path":{"type":"string"}},"required":["workspace_path"]})
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

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let filename = input.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        let workspace_path = input
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if workspace_path.is_empty() {
            return Ok(ToolResult::error("Error: workspace_path 是必需的"));
        }
        let safe_name = filename
            .replace("..", "")
            .replace("\\", "")
            .replace("/", "");
        let file_path = Path::new(workspace_path).join(&safe_name);
        if !file_path.starts_with(workspace_path) {
            return Ok(ToolResult::error("Error: 文件名包含非法路径组件"));
        }
        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                let truncated = truncate_text(&content, 20000);
                if truncated.is_empty() {
                    Ok(ToolResult::success(format!("文件 '{}' 存在但为空", safe_name)))
                } else {
                    Ok(ToolResult::success(truncated))
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ToolResult::success(format!(
                "文件 '{}' 在 {} 中不存在。使用 WorkspaceWrite 创建。",
                safe_name, workspace_path
            ))),
            Err(e) => Ok(ToolResult::error(format!("读取文件失败: {}", e))),
        }
    }
}

pub struct WorkspaceWriteTool;

#[async_trait]
impl Tool for WorkspaceWriteTool {
    fn name(&self) -> &str {
        "WorkspaceWrite"
    }
    fn description(&self) -> &str {
        "写入或追加工作区记忆文件内容。mode: append(追加)/overwrite(覆盖)。用于持久化项目上下文。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"filename":{"type":"string"},"workspace_path":{"type":"string"},"content":{"type":"string"},"mode":{"type":"string","enum":["append","overwrite"],"default":"append"}},"required":["workspace_path","content"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let filename = input.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        let workspace_path = input
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let content_str = input
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let mode = input
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("append");
        if workspace_path.is_empty() {
            return Ok(ToolResult::error("Error: workspace_path 是必需的"));
        }
        if content_str.is_empty() {
            return Ok(ToolResult::error("Error: content 是必需的"));
        }

        let safe_name = filename
            .replace("..", "")
            .replace("\\", "")
            .replace("/", "");
        let file_path = Path::new(workspace_path).join(&safe_name);
        if !file_path.starts_with(workspace_path) {
            return Ok(ToolResult::error("Error: 文件名包含非法路径组件"));
        }
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::execution_failed(format!("创建目录失败: {}", e)))?;
        }
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let data = match mode {
            "overwrite" => format!("{}\n\n[最后更新: {}]\n", content_str, timestamp),
            _ => {
                let existing = std::fs::read_to_string(&file_path).unwrap_or_default();
                if existing.is_empty() {
                    format!("{}\n\n[创建于: {}]\n", content_str, timestamp)
                } else {
                    format!("{}\n\n---\n{}\n\n[追加于: {}]\n", existing, content_str, timestamp)
                }
            },
        };
        std::fs::write(&file_path, &data)
            .map_err(|e| ToolError::execution_failed(format!("写入失败: {}", e)))?;
        Ok(ToolResult::success(format!("已写入: {} (mode: {})", file_path.display(), mode)))
    }
}
