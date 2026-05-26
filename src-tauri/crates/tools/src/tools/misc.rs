//! 杂项工具集
//!
//! 远程文件管理, 缓存管理

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════
// RemoteFile
// ═══════════════════════════════════════════════════════════════════════════

pub struct RemoteFileUploadTool;

#[async_trait]
impl Tool for RemoteFileUploadTool {
    fn name(&self) -> &str {
        "RemoteFileUpload"
    }
    fn description(&self) -> &str {
        "上传文件到远程存储（基于 HTTP API）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"local_path":{"type":"string"},"remote_name":{"type":"string"},"provider":{"type":"string"},"api_key":{"type":"string"}},"required":["local_path"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let local_path = input
            .get("local_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let remote_name = input
            .get("remote_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if local_path.is_empty() {
            return Ok(ToolResult::error("Error: local_path 是必需的"));
        }
        if !Path::new(local_path).exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", local_path)));
        }
        if remote_name.contains("..") || remote_name.contains(std::path::MAIN_SEPARATOR) {
            return Err(ToolError::permission_denied(
                "RemoteFileUpload",
                "remote_name 包含非法路径字符",
            ));
        }

        let content =
            std::fs::read(local_path).map_err(|e| ToolError::execution_failed(e.to_string()))?;
        let dest_name = if remote_name.is_empty() {
            Path::new(local_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            remote_name.to_string()
        };

        let remote_dir = Path::new("remote");
        std::fs::create_dir_all(remote_dir).ok();
        let dest_path = remote_dir.join(&dest_name);
        if dest_path.exists() {
            let canonical_remote = remote_dir
                .canonicalize()
                .unwrap_or_else(|_| remote_dir.to_path_buf());
            let canonical_dest = dest_path
                .canonicalize()
                .unwrap_or_else(|_| dest_path.clone());
            if !canonical_dest.starts_with(&canonical_remote) {
                return Err(ToolError::permission_denied(
                    "RemoteFileUpload",
                    "路径遍历攻击已阻止：目标路径超出远程存储目录",
                ));
            }
        }
        std::fs::write(&dest_path, &content)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;

        Ok(ToolResult::success(format!("已上传: {} -> remote/{}", local_path, dest_name)))
    }
}

pub struct RemoteFileListTool;

#[async_trait]
impl Tool for RemoteFileListTool {
    fn name(&self) -> &str {
        "RemoteFileList"
    }
    fn description(&self) -> &str {
        "列出远程存储中的文件。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let remote = Path::new("remote");
        if !remote.exists() {
            return Ok(ToolResult::success("远程存储为空"));
        }
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(remote) {
            for e in entries.filter_map(|e| e.ok()) {
                let name = e.file_name().to_string_lossy().to_string();
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                files.push(format!("  {} ({} bytes)", name, size));
            }
        }
        if files.is_empty() {
            Ok(ToolResult::success("远程存储为空"))
        } else {
            Ok(ToolResult::success(format!(
                "远程文件 ({}):\n{}",
                files.len(),
                files.join("\n")
            )))
        }
    }
}

pub struct RemoteFileDeleteTool;

#[async_trait]
impl Tool for RemoteFileDeleteTool {
    fn name(&self) -> &str {
        "RemoteFileDelete"
    }
    fn description(&self) -> &str {
        "删除远程存储中的文件。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"remote_name":{"type":"string"}},"required":["remote_name"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = input
            .get("remote_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if name.is_empty() {
            return Ok(ToolResult::error("Error: remote_name 是必需的"));
        }
        if name.contains("..") || name.contains(std::path::MAIN_SEPARATOR) || name.contains('/') {
            return Err(ToolError::permission_denied(
                "RemoteFileDelete",
                "remote_name 包含非法路径字符",
            ));
        }
        let remote_dir = Path::new("remote");
        let path = remote_dir.join(name);
        if !path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", name)));
        }
        let canonical_remote = remote_dir
            .canonicalize()
            .unwrap_or_else(|_| remote_dir.to_path_buf());
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !canonical_path.starts_with(&canonical_remote) {
            return Err(ToolError::permission_denied(
                "RemoteFileDelete",
                "路径遍历攻击已阻止：文件路径超出远程存储目录",
            ));
        }
        std::fs::remove_file(&path)
            .map_err(|e| ToolError::execution_failed(format!("删除失败: {}", e)))?;
        Ok(ToolResult::success(format!("已删除: {}", name)))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Cache
// ═══════════════════════════════════════════════════════════════════════════

pub struct CacheInfoTool;

#[async_trait]
impl Tool for CacheInfoTool {
    fn name(&self) -> &str {
        "CacheInfo"
    }
    fn description(&self) -> &str {
        "获取缓存系统信息（条目数、总大小）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let cache_dir = Path::new(".axagent/cache");
        if !cache_dir.exists() {
            return Ok(ToolResult::success("缓存目录不存在或为空"));
        }
        let mut count = 0u64;
        let mut size = 0u64;
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for e in entries.filter_map(|e| e.ok()) {
                if e.path().is_file() {
                    count += 1;
                    size += e.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
        Ok(ToolResult::success(format!(
            "缓存信息:\n  文件数: {}\n  总大小: {} bytes",
            count, size
        )))
    }
}

pub struct CacheClearTool;

#[async_trait]
impl Tool for CacheClearTool {
    fn name(&self) -> &str {
        "CacheClear"
    }
    fn description(&self) -> &str {
        "清除缓存系统中的缓存条目。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"scope":{"type":"string","enum":["all","embeddings","search"],"default":"all"}}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let scope = input.get("scope").and_then(|v| v.as_str()).unwrap_or("all");
        let cache_dir = Path::new(".axagent/cache");
        if !cache_dir.exists() {
            return Ok(ToolResult::success("缓存目录为空"));
        }
        let mut removed = 0u64;
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for e in entries.filter_map(|e| e.ok()) {
                let path = e.path();
                if !path.is_file() {
                    continue;
                }
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                let should_remove = match scope {
                    "embeddings" => {
                        file_name.starts_with("emb_") || file_name.starts_with("embedding_")
                    },
                    "search" => file_name.starts_with("search_") || file_name.starts_with("idx_"),
                    _ => true,
                };
                if should_remove && std::fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            }
        }
        Ok(ToolResult::success(format!("已清除 {} 个缓存文件 (scope: {})", removed, scope)))
    }
}
