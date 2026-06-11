// SPDX-License-Identifier: AGPL-3.0-only

//! 文件系统操作工具
//!
//! 将 builtin_handlers 中的 list_directory、delete_file、create_directory、
//! file_exists、get_file_info、move_file 迁移为 Tool trait 实现。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// 将字节数转为人类可读格式
fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

/// 验证并解析路径（仅允许 workspace 内的路径）
fn validate_and_resolve_path(path: &str, ctx: &ToolContext) -> Result<std::path::PathBuf, String> {
    if path.is_empty() {
        return Err("路径不能为空".into());
    }
    let p = Path::new(path);
    if p.is_absolute() {
        if !p.starts_with("/") {
            return Err(format!("不支持的操作系统路径: {}", path));
        }
        Ok(p.to_path_buf())
    } else {
        let cwd = Path::new(&ctx.working_dir);
        Ok(cwd.join(p))
    }
}

// ── ListDirectoryTool ──────────────────────────────────────────────────────

pub struct ListDirectoryTool;

#[async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &str {
        "ListDirectory"
    }

    fn description(&self) -> &str {
        "列出指定目录的内容，包括文件和子目录。返回文件名、大小和类型（文件/目录）。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要列出内容的目录路径，默认为当前工作目录"
                }
            },
            "required": []
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let resolved_path =
            validate_and_resolve_path(path, ctx).map_err(ToolError::invalid_input)?;

        let mut entries = tokio::fs::read_dir(&resolved_path)
            .await
            .map_err(|e| ToolError::execution_failed(format!("列出目录失败 '{}': {}", path, e)))?;

        let mut items = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry
                .file_type()
                .await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);
            let meta = entry.metadata().await.ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

            if is_dir {
                items.push(format!("📁 {}/", name));
            } else {
                items.push(format!("📄 {} ({})", name, human_size(size)));
            }
        }

        items.sort();
        let content = if items.is_empty() {
            format!("目录 '{}' 为空", path)
        } else {
            format!("'{}' 的内容:\n{}", path, items.join("\n"))
        };

        Ok(ToolResult {
            content,
            truncated: false,
            is_error: false,
            metadata: None,
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

// ── DeleteFileTool ─────────────────────────────────────────────────────────

pub struct DeleteFileTool;

#[async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &str {
        "DeleteFile"
    }

    fn description(&self) -> &str {
        "删除指定路径的文件。此操作不可逆，请谨慎使用。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要删除的文件路径"
                }
            },
            "required": ["path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if path.is_empty() {
            return Ok(ToolResult::error("Error: path 参数是必需的"));
        }

        let resolved_path =
            validate_and_resolve_path(path, ctx).map_err(ToolError::invalid_input)?;

        let path_str = resolved_path.to_string_lossy();
        tokio::fs::remove_file(&*path_str)
            .await
            .map_err(|e| ToolError::execution_failed(format!("删除文件失败: {}", e)))?;

        Ok(ToolResult::success(format!("文件 '{}' 已成功删除", path)))
    }
}

// ── CreateDirectoryTool ────────────────────────────────────────────────────

pub struct CreateDirectoryTool;

#[async_trait]
impl Tool for CreateDirectoryTool {
    fn name(&self) -> &str {
        "CreateDirectory"
    }

    fn description(&self) -> &str {
        "创建指定路径的目录（包括必要的父目录）。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要创建的目录路径"
                }
            },
            "required": ["path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if path.is_empty() {
            return Ok(ToolResult::error("Error: path 参数是必需的"));
        }

        let resolved_path =
            validate_and_resolve_path(path, ctx).map_err(ToolError::invalid_input)?;

        let path_str = resolved_path.to_string_lossy();
        tokio::fs::create_dir_all(&*path_str)
            .await
            .map_err(|e| ToolError::execution_failed(format!("创建目录失败: {}", e)))?;

        Ok(ToolResult::success(format!("目录 '{}' 已成功创建", path)))
    }
}

// ── FileExistsTool ─────────────────────────────────────────────────────────

pub struct FileExistsTool;

#[async_trait]
impl Tool for FileExistsTool {
    fn name(&self) -> &str {
        "FileExists"
    }

    fn description(&self) -> &str {
        "检查指定路径的文件或目录是否存在。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要检查的文件或目录路径"
                }
            },
            "required": ["path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if path.is_empty() {
            return Ok(ToolResult::error("Error: path 参数是必需的"));
        }

        let exists = match validate_and_resolve_path(path, ctx) {
            Ok(resolved) => tokio::fs::metadata(&resolved).await.is_ok(),
            Err(_) => false,
        };

        Ok(ToolResult::success(format!(
            "{}: {}",
            path,
            if exists { "存在" } else { "不存在" }
        )))
    }
}

// ── GetFileInfoTool ────────────────────────────────────────────────────────

pub struct GetFileInfoTool;

#[async_trait]
impl Tool for GetFileInfoTool {
    fn name(&self) -> &str {
        "GetFileInfo"
    }

    fn description(&self) -> &str {
        "获取指定文件的元信息，包括大小和修改时间。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要获取信息的文件路径"
                }
            },
            "required": ["path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if path.is_empty() {
            return Ok(ToolResult::error("Error: path 参数是必需的"));
        }

        let resolved_path =
            validate_and_resolve_path(path, ctx).map_err(ToolError::invalid_input)?;

        let path_str = resolved_path.to_string_lossy();
        let meta = tokio::fs::metadata(&*path_str)
            .await
            .map_err(|e| ToolError::execution_failed(format!("获取文件信息失败: {}", e)))?;

        let info = format!(
            "文件: {}\n  大小: {} ({})\n  修改时间: {:?}",
            path,
            meta.len(),
            human_size(meta.len()),
            meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        );

        Ok(ToolResult::success(info))
    }
}

// ── MoveFileTool ───────────────────────────────────────────────────────────

pub struct MoveFileTool;

#[async_trait]
impl Tool for MoveFileTool {
    fn name(&self) -> &str {
        "MoveFile"
    }

    fn description(&self) -> &str {
        "将文件从源路径移动到目标路径。可用于重命名文件。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "源文件路径"
                },
                "destination": {
                    "type": "string",
                    "description": "目标文件路径"
                }
            },
            "required": ["source", "destination"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let source = input
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let destination = input
            .get("destination")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if source.is_empty() || destination.is_empty() {
            return Ok(ToolResult::error("Error: source 和 destination 参数都是必需的"));
        }

        if source == destination {
            return Ok(ToolResult::error("Error: 源路径和目标路径相同"));
        }

        let resolved_source = validate_and_resolve_path(source, ctx)
            .map_err(|e| ToolError::invalid_input(format!("源路径: {}", e)))?;
        let resolved_dest = validate_and_resolve_path(destination, ctx)
            .map_err(|e| ToolError::invalid_input(format!("目标路径: {}", e)))?;

        let source_str = resolved_source.to_string_lossy();
        let dest_str = resolved_dest.to_string_lossy();

        if let Some(parent) = Path::new(&*dest_str).parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::execution_failed(format!("创建目标父目录失败: {}", e)))?;
        }

        tokio::fs::rename(&*source_str, &*dest_str)
            .await
            .map_err(|e| ToolError::execution_failed(format!("移动文件失败: {}", e)))?;

        Ok(ToolResult::success(format!("已将 '{}' 移动到 '{}'", source, destination)))
    }
}
