//! 存储管理工具
//!
//! 将 builtin_handlers 中的 get_storage_info、list_storage_files、
//! upload_storage_file、download_storage_file、delete_storage_file 迁移为 Tool trait。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use base64::Engine;
use serde_json::Value;
use std::path::Path;

const MAX_BASE64_DECODE_SIZE: usize = 100 * 1024 * 1024; // 100MB

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

fn base64_decode(input: &str) -> Result<Vec<u8>, ToolError> {
    if input.len() > MAX_BASE64_DECODE_SIZE * 4 / 3 + 100 {
        return Err(ToolError::invalid_input("Base64 输入过大"));
    }
    Engine::decode(&base64::engine::general_purpose::STANDARD, input)
        .map_err(|e| ToolError::invalid_input(format!("Base64 解码错误: {}", e)))
}

// ── GetStorageInfoTool ─────────────────────────────────────────────────────

pub struct GetStorageInfoTool;

#[async_trait]
impl Tool for GetStorageInfoTool {
    fn name(&self) -> &str {
        "GetStorageInfo"
    }

    fn description(&self) -> &str {
        "获取存储系统信息，包括根目录和文件总数。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Storage
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let docs = Path::new("documents");
        let total: u64 = if docs.exists() {
            std::fs::read_dir(docs)
                .map(|rd| rd.count() as u64)
                .unwrap_or(0)
        } else {
            0
        };
        Ok(ToolResult::success(format!(
            "存储信息:\n  根目录: documents/\n  文件总数: {}",
            total
        )))
    }
}

// ── ListStorageFilesTool ───────────────────────────────────────────────────

pub struct ListStorageFilesTool;

#[async_trait]
impl Tool for ListStorageFilesTool {
    fn name(&self) -> &str {
        "ListStorageFiles"
    }

    fn description(&self) -> &str {
        "列出存储系统中的文件，可按路径过滤。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "要列出的子目录路径", "default": "" },
                "limit": { "type": "integer", "description": "最多返回条目数", "default": 50 }
            }
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Storage
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(50);

        let docs = Path::new("documents");
        let full_path = docs.join(path);
        let mut items = Vec::new();

        if let Ok(entries) = std::fs::read_dir(full_path) {
            for entry in entries.filter_map(|e| e.ok()).take(limit) {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.path().is_dir();
                if is_dir {
                    items.push(format!("📁 {}/", name));
                } else {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    items.push(format!("📄 {} ({})", name, human_size(size)));
                }
            }
        }

        if items.is_empty() {
            Ok(ToolResult::success(format!("'{}' 中没有文件", path)))
        } else {
            Ok(ToolResult::success(format!("'{}' 中的文件:\n{}", path, items.join("\n"))))
        }
    }
}

// ── UploadStorageFileTool ──────────────────────────────────────────────────

pub struct UploadStorageFileTool;

#[async_trait]
impl Tool for UploadStorageFileTool {
    fn name(&self) -> &str {
        "UploadStorageFile"
    }

    fn description(&self) -> &str {
        "上传文件到存储系统。文件内容使用 Base64 编码。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "filename": { "type": "string", "description": "文件名" },
                "content_base64": { "type": "string", "description": "Base64 编码的文件内容" },
                "bucket": { "type": "string", "description": "存储桶/子目录" }
            },
            "required": ["filename", "content_base64"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Storage
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let filename = input
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let content_base64 = input
            .get("content_base64")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let bucket = input
            .get("bucket")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if filename.is_empty() {
            return Ok(ToolResult::error("Error: filename 是必需的"));
        }
        if content_base64.is_empty() {
            return Ok(ToolResult::error("Error: content_base64 是必需的"));
        }

        let decoded = base64_decode(content_base64)?;
        let docs = Path::new("documents");
        let bucket_path = if bucket.is_empty() {
            docs.join(filename)
        } else {
            docs.join(bucket).join(filename)
        };

        if let Some(parent) = bucket_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::execution_failed(format!("创建目录失败: {}", e)))?;
        }
        std::fs::write(&bucket_path, decoded)
            .map_err(|e| ToolError::execution_failed(format!("写入文件失败: {}", e)))?;

        Ok(ToolResult::success(format!(
            "文件 '{}' 已上传到 '{}'",
            filename,
            bucket_path.display()
        )))
    }
}

// ── DownloadStorageFileTool ────────────────────────────────────────────────

pub struct DownloadStorageFileTool;

#[async_trait]
impl Tool for DownloadStorageFileTool {
    fn name(&self) -> &str {
        "DownloadStorageFile"
    }

    fn description(&self) -> &str {
        "从存储系统下载文件，返回 Base64 编码的文件内容。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "要下载的文件路径" }
            },
            "required": ["path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Storage
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if path.is_empty() {
            return Ok(ToolResult::error("Error: path 是必需的"));
        }

        let docs = Path::new("documents");
        let full_path = docs.join(path);

        if !full_path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", path)));
        }

        let content = std::fs::read(&full_path)
            .map_err(|e| ToolError::execution_failed(format!("读取文件失败: {}", e)))?;
        let encoded = Engine::encode(&base64::engine::general_purpose::STANDARD, &content);

        Ok(ToolResult::success(format!("文件 '{}' 内容 (base64):\n{}", path, encoded)))
    }
}

// ── DeleteStorageFileTool ──────────────────────────────────────────────────

pub struct DeleteStorageFileTool;

#[async_trait]
impl Tool for DeleteStorageFileTool {
    fn name(&self) -> &str {
        "DeleteStorageFile"
    }

    fn description(&self) -> &str {
        "从存储系统中删除指定路径的文件。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "要删除的文件路径" }
            },
            "required": ["path"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Storage
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if path.is_empty() {
            return Ok(ToolResult::error("Error: path 是必需的"));
        }

        let docs = Path::new("documents");
        let full_path = docs.join(path);

        if !full_path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", path)));
        }

        std::fs::remove_file(&full_path)
            .map_err(|e| ToolError::execution_failed(format!("删除文件失败: {}", e)))?;

        Ok(ToolResult::success(format!("文件 '{}' 已删除", path)))
    }
}
