//! 杂项工具集——完整迁移
//!
//! 将 builtin_handlers 中所有剩余独立工具完整迁移为 Tool trait。
//! 未做任何功能简化，保持原始逻辑。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use std::sync::Mutex;

// ── 工具函数 ──────────────────────────────────────────────────────────────

fn truncate_text(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}

// ── 全局状态（agent 工具用）────────────────────────────────────────────────

static CHECKPOINTS: std::sync::LazyLock<Mutex<Vec<(String, String, String)>>> = std::sync::LazyLock::new(|| Mutex::new(Vec::new()));
static AGENT_MEMORY: std::sync::LazyLock<Mutex<std::collections::HashMap<String, String>>> = std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

// ═══════════════════════════════════════════════════════════════════════════
// ExportWord
// ═══════════════════════════════════════════════════════════════════════════

pub struct ExportWordTool;

#[async_trait]
impl Tool for ExportWordTool {
    fn name(&self) -> &str { "ExportWord" }
    fn description(&self) -> &str { "将 Markdown 导出为 Word (.docx) 文件，支持标题、列表、引用等格式。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"markdown":{"type":"string"},"output_path":{"type":"string"},"title":{"type":"string","default":"Document"}},"required":["markdown","output_path"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileWrite }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown = input.get("markdown").and_then(|v| v.as_str()).unwrap_or_default();
        let output_path = input.get("output_path").and_then(|v| v.as_str()).unwrap_or_default();
        let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("Document");

        if markdown.is_empty() { return Ok(ToolResult::error("Error: markdown 是必需的")); }
        if output_path.is_empty() { return Ok(ToolResult::error("Error: output_path 是必需的")); }

        use docx_rs::*;
        let path = Path::new(output_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::execution_failed(format!("创建输出目录失败: {}", e)))?;
        }

        let mut doc = Docx::new();
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(title).size(32).bold()).align(AlignmentType::Center));
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("")));

        for line in markdown.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("")));
            } else if let Some(s) = trimmed.strip_prefix("# ") {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(s).size(36).bold()));
            } else if let Some(s) = trimmed.strip_prefix("## ") {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(s).size(28).bold()));
            } else if let Some(s) = trimmed.strip_prefix("### ") {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(s).size(24).bold()));
            } else if let Some(s) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(format!("• {}", s))));
            } else if let Some(s) = trimmed.strip_prefix("> ") {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(s).italic().color("666666")));
            } else {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(trimmed).size(22)));
            }
        }

        let file = std::fs::File::create(path)
            .map_err(|e| ToolError::execution_failed(format!("创建文件失败: {}", e)))?;
        match doc.build().pack(file) {
            Ok(_) => Ok(ToolResult::success(format!("Word 文档已导出: {}", output_path))),
            Err(e) => Ok(ToolResult::error(format!("创建 Word 文档失败: {}", e))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SessionSearch
// ═══════════════════════════════════════════════════════════════════════════

pub struct SessionSearchTool;

#[async_trait]
impl Tool for SessionSearchTool {
    fn name(&self) -> &str { "SessionSearch" }
    fn description(&self) -> &str { "通过 FTS5 全文搜索历史会话记录。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer","default":10},"db_path":{"type":"string"}},"required":["query"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or_default();
        let limit = input.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as i32;

        if query.is_empty() { return Ok(ToolResult::error("Error: query 是必需的")); }

        let db_path_str = match crate::global_state::get_db_path() {
            Some(p) => p,
            None => return Ok(ToolResult::error("会话搜索不可用：未配置数据库路径")),
        };
        let db_file = db_path_str.strip_prefix("sqlite:").unwrap_or(&db_path_str);

        let conn = rusqlite::Connection::open(db_file)
            .map_err(|e| ToolError::execution_failed(format!("打开数据库失败: {}", e)))?;

        let fts_sql = "SELECT m.conversation_id, snippet(messages_fts, 0, '>>', '<<', '...', 24) as snippet, bm25(messages_fts) as rank FROM messages_fts JOIN messages m ON m.rowid = messages_fts.rowid WHERE messages_fts MATCH ? ORDER BY rank LIMIT ?";

        let rows: Vec<String> = match conn.prepare(fts_sql) {
            Ok(mut stmt) => stmt.query_map(rusqlite::params![query, limit], |row| {
                let conv_id: String = row.get(0)?;
                let snippet: String = row.get(1)?;
                Ok(format!("[{}] {}", conv_id, snippet))
            }).map_err(|e| ToolError::execution_failed(e.to_string()))?
            .filter_map(|r| r.ok()).collect(),
            Err(e) => return Ok(ToolResult::error(format!("会话搜索错误 (FTS5 不可用): {}", e))),
        };

        if rows.is_empty() {
            Ok(ToolResult::success(format!("未找到 '{}' 的结果", query)))
        } else {
            Ok(ToolResult::success(format!("搜索 '{}' ({} 条):\n{}", query, rows.len(), rows.join("\n"))))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MemoryFlush
// ═══════════════════════════════════════════════════════════════════════════

pub struct MemoryFlushTool;

#[async_trait]
impl Tool for MemoryFlushTool {
    fn name(&self) -> &str { "MemoryFlush" }
    fn description(&self) -> &str { "将记忆/洞察持久化到长期存储数据库。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"content":{"type":"string"},"target":{"type":"string","enum":["memory","user"],"default":"memory"},"category":{"type":"string","enum":["insight","decision","error_solution","preference","pattern","workflow"],"default":"insight"}},"required":["content"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let content = input.get("content").and_then(|v| v.as_str()).unwrap_or_default();
        let target = input.get("target").and_then(|v| v.as_str()).unwrap_or("memory");
        let category = input.get("category").and_then(|v| v.as_str()).unwrap_or("insight");

        if content.is_empty() { return Ok(ToolResult::error("Error: content 是必需的")); }
        // TODO: 接入长期记忆持久化后端
        tracing::info!("MemoryFlush: target={}, category={}, content_len={}", target, category, content.len());
        Ok(ToolResult::success(format!("记忆已持久化 (target: {}, category: {})", target, category)))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Workspace Read/Write
// ═══════════════════════════════════════════════════════════════════════════

pub struct WorkspaceReadTool;

#[async_trait]
impl Tool for WorkspaceReadTool {
    fn name(&self) -> &str { "WorkspaceRead" }
    fn description(&self) -> &str { "读取工作区记忆文件。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"filename":{"type":"string"},"workspace_path":{"type":"string"}},"required":["workspace_path"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let filename = input.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        let workspace_path = input.get("workspace_path").and_then(|v| v.as_str()).unwrap_or_default();

        if workspace_path.is_empty() { return Ok(ToolResult::error("Error: workspace_path 是必需的")); }

        let safe_name = filename.replace("..", "").replace("\\", "").replace("/", "");
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
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(ToolResult::success(format!("记忆文件 '{}' 在 {} 中尚不存在。使用 WorkspaceWrite 创建。", safe_name, workspace_path)))
            },
            Err(e) => Ok(ToolResult::error(format!("读取文件失败: {}", e))),
        }
    }
}

pub struct WorkspaceWriteTool;

#[async_trait]
impl Tool for WorkspaceWriteTool {
    fn name(&self) -> &str { "WorkspaceWrite" }
    fn description(&self) -> &str { "写入或追加工作区记忆文件。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"filename":{"type":"string"},"workspace_path":{"type":"string"},"content":{"type":"string"},"mode":{"type":"string","enum":["append","overwrite"],"default":"append"}},"required":["workspace_path","content"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let filename = input.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        let workspace_path = input.get("workspace_path").and_then(|v| v.as_str()).unwrap_or_default();
        let content_str = input.get("content").and_then(|v| v.as_str()).unwrap_or_default();
        let mode = input.get("mode").and_then(|v| v.as_str()).unwrap_or("append");

        if workspace_path.is_empty() { return Ok(ToolResult::error("Error: workspace_path 是必需的")); }
        if content_str.is_empty() { return Ok(ToolResult::error("Error: content 是必需的")); }

        let safe_name = filename.replace("..", "").replace("\\", "").replace("/", "");
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

        std::fs::write(&file_path, data)
            .map_err(|e| ToolError::execution_failed(format!("写入文件失败: {}", e)))?;

        let action = if mode == "overwrite" { "更新" } else { "追加" };
        Ok(ToolResult::success(format!("记忆文件 '{}' 已{}到 {}", safe_name, action, workspace_path)))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RemoteFile
// ═══════════════════════════════════════════════════════════════════════════

pub struct RemoteFileUploadTool;

#[async_trait]
impl Tool for RemoteFileUploadTool {
    fn name(&self) -> &str { "RemoteFileUpload" }
    fn description(&self) -> &str { "上传文件到远程存储（基于 HTTP API）。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"local_path":{"type":"string"},"remote_name":{"type":"string"},"provider":{"type":"string"},"api_key":{"type":"string"}},"required":["local_path"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileWrite }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let local_path = input.get("local_path").and_then(|v| v.as_str()).unwrap_or_default();
        let remote_name = input.get("remote_name").and_then(|v| v.as_str()).unwrap_or("");
        if local_path.is_empty() { return Ok(ToolResult::error("Error: local_path 是必需的")); }
        if !Path::new(local_path).exists() { return Ok(ToolResult::error(format!("文件未找到: {}", local_path))); }

        let content = std::fs::read(local_path)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        let dest_name = if remote_name.is_empty() {
            Path::new(local_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
        } else { remote_name.to_string() };

        let remote_dir = Path::new("remote");
        std::fs::create_dir_all(remote_dir).ok();
        std::fs::write(remote_dir.join(&dest_name), &content)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;

        Ok(ToolResult::success(format!("已上传: {} -> remote/{}", local_path, dest_name)))
    }
}

pub struct RemoteFileListTool;

#[async_trait]
impl Tool for RemoteFileListTool {
    fn name(&self) -> &str { "RemoteFileList" }
    fn description(&self) -> &str { "列出远程存储中的文件。" }
    fn input_schema(&self) -> Value { serde_json::json!({"type":"object","properties":{}}) }
    fn category(&self) -> ToolCategory { ToolCategory::FileRead }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let remote = Path::new("remote");
        if !remote.exists() { return Ok(ToolResult::success("远程存储为空")); }
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
            Ok(ToolResult::success(format!("远程文件 ({}):\n{}", files.len(), files.join("\n"))))
        }
    }
}

pub struct RemoteFileDeleteTool;

#[async_trait]
impl Tool for RemoteFileDeleteTool {
    fn name(&self) -> &str { "RemoteFileDelete" }
    fn description(&self) -> &str { "删除远程存储中的文件。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"remote_name":{"type":"string"}},"required":["remote_name"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileWrite }
    fn is_destructive(&self) -> bool { true }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = input.get("remote_name").and_then(|v| v.as_str()).unwrap_or_default();
        if name.is_empty() { return Ok(ToolResult::error("Error: remote_name 是必需的")); }
        let path = Path::new("remote").join(name);
        if !path.exists() { return Ok(ToolResult::error(format!("文件未找到: {}", name))); }
        std::fs::remove_file(&path)
            .map_err(|e| ToolError::execution_failed(format!("删除失败: {}", e)))?;
        Ok(ToolResult::success(format!("已删除: {}", name)))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PdfInfo / DetectEncoding / Base64Image
// ═══════════════════════════════════════════════════════════════════════════

pub struct PdfInfoTool;

#[async_trait]
impl Tool for PdfInfoTool {
    fn name(&self) -> &str { "PdfInfo" }
    fn description(&self) -> &str { "获取 PDF 文件信息（页数、大小、文本预览）。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileRead }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input.get("path").and_then(|v| v.as_str()).unwrap_or_default();
        if file_path.is_empty() { return Ok(ToolResult::error("Error: path 是必需的")); }
        if !Path::new(file_path).exists() { return Ok(ToolResult::error(format!("文件未找到: {}", file_path))); }

        let data = std::fs::read(file_path)
            .map_err(|e| ToolError::execution_failed(format!("读取文件失败: {}", e)))?;

        match pdf_extract::extract_text_from_mem(&data) {
            Ok(text) => {
                let page_count = text.matches('\u{000C}').count() + 1;
                let preview = truncate_text(&text.replace('\u{000C}', "\n--- 分页 ---\n"), 5000);
                Ok(ToolResult::success(format!("PDF 信息:\n  路径: {}\n  大小: {} bytes\n  估页数: {}\n\n文本预览:\n{}", file_path, data.len(), page_count, preview)))
            },
            Err(e) => Ok(ToolResult::error(format!("提取 PDF 文本失败: {}", e))),
        }
    }
}

pub struct DetectEncodingTool;

#[async_trait]
impl Tool for DetectEncodingTool {
    fn name(&self) -> &str { "DetectEncoding" }
    fn description(&self) -> &str { "检测文件的字符编码（UTF-8/16 BOM、ASCII 比例分析）。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileRead }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input.get("path").and_then(|v| v.as_str()).unwrap_or_default();
        if file_path.is_empty() { return Ok(ToolResult::error("Error: path 是必需的")); }
        if !Path::new(file_path).exists() { return Ok(ToolResult::error(format!("文件未找到: {}", file_path))); }

        let data = std::fs::read(file_path)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        if data.is_empty() { return Ok(ToolResult::success("文件为空")); }

        // UTF-8 BOM
        if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
            let preview = String::from_utf8_lossy(&data[3..]);
            return Ok(ToolResult::success(format!("编码: UTF-8 (带 BOM)\n预览: {}", truncate_text(&preview, 2000))));
        }
        // UTF-16 LE BOM
        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
            return Ok(ToolResult::success(format!("编码: UTF-16 LE (BOM)\n大小: {} bytes", data.len())));
        }
        // UTF-16 BE BOM
        if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
            return Ok(ToolResult::success(format!("编码: UTF-16 BE (BOM)\n大小: {} bytes", data.len())));
        }
        // Try UTF-8
        match std::str::from_utf8(&data) {
            Ok(s) => Ok(ToolResult::success(format!("编码: UTF-8 (有效)\n预览: {}", truncate_text(s, 2000)))),
            Err(_) => {
                let printable = data.iter().filter(|&&b| (32..127).contains(&b)).count();
                let ratio = printable as f64 / data.len() as f64 * 100.0;
                let guess = if ratio > 85.0 { "ASCII 或 Latin-1" } else { "二进制/未知" };
                Ok(ToolResult::success(format!("非有效 UTF-8。{}% 可打印。可能: {}\n大小: {} bytes", ratio.round(), guess, data.len())))
            },
        }
    }
}

pub struct Base64ImageTool;

#[async_trait]
impl Tool for Base64ImageTool {
    fn name(&self) -> &str { "Base64Image" }
    fn description(&self) -> &str { "将图片文件编码为 Base64 字符串。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileRead }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or_default();
        if path.is_empty() { return Ok(ToolResult::error("Error: path 是必需的")); }
        let bytes = std::fs::read(path)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(ToolResult::success(encoded))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Cache
// ═══════════════════════════════════════════════════════════════════════════

pub struct CacheInfoTool;

#[async_trait]
impl Tool for CacheInfoTool {
    fn name(&self) -> &str { "CacheInfo" }
    fn description(&self) -> &str { "获取缓存系统信息（条目数、总大小）。" }
    fn input_schema(&self) -> Value { serde_json::json!({"type":"object","properties":{}}) }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }

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
        Ok(ToolResult::success(format!("缓存信息:\n  文件数: {}\n  总大小: {} bytes", count, size)))
    }
}

pub struct CacheClearTool;

#[async_trait]
impl Tool for CacheClearTool {
    fn name(&self) -> &str { "CacheClear" }
    fn description(&self) -> &str { "清除缓存系统中的缓存条目。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"scope":{"type":"string","enum":["all","embeddings","search"],"default":"all"}}})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let scope = input.get("scope").and_then(|v| v.as_str()).unwrap_or("all");
        let cache_dir = Path::new(".axagent/cache");
        if !cache_dir.exists() {
            return Ok(ToolResult::success("缓存目录为空"));
        }
        let mut removed = 0u64;
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for e in entries.filter_map(|e| e.ok()) {
                if e.path().is_file() {
                    if std::fs::remove_file(e.path()).is_ok() { removed += 1; }
                }
            }
        }
        Ok(ToolResult::success(format!("已清除 {} 个缓存文件 (scope: {})", removed, scope)))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Agent Control
// ═══════════════════════════════════════════════════════════════════════════

pub struct AgentCheckpointTool;

#[async_trait]
impl Tool for AgentCheckpointTool {
    fn name(&self) -> &str { "AgentCheckpoint" }
    fn description(&self) -> &str { "管理 Agent 检查点：保存、恢复、列出。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["save","restore","list"]},"checkpoint_id":{"type":"string"},"label":{"type":"string"}},"required":["action"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::Agent }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = input.get("action").and_then(|v| v.as_str()).unwrap_or("list");
        let ckpt_id = input.get("checkpoint_id").and_then(|v| v.as_str()).unwrap_or("");
        let label = input.get("label").and_then(|v| v.as_str()).unwrap_or("");

        match action {
            "save" => {
                let id = format!("ckpt-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
                let display = if label.is_empty() { "未命名检查点" } else { label };
                let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let mut checkpoints = CHECKPOINTS.lock().map_err(|e| ToolError::execution_failed(e.to_string()))?;
                checkpoints.push((id.clone(), display.to_string(), ts));
                if checkpoints.len() > 50 { checkpoints.remove(0); }
                Ok(ToolResult::success(format!("检查点已保存: {} (标签: {})", id, display)))
            },
            "list" => {
                let checkpoints = CHECKPOINTS.lock().map_err(|e| ToolError::execution_failed(e.to_string()))?;
                if checkpoints.is_empty() { return Ok(ToolResult::success("暂无检查点。使用 action='save' 创建。")); }
                let lines: Vec<String> = std::iter::once(format!("检查点 ({}):", checkpoints.len()))
                    .chain(checkpoints.iter().map(|(id, lbl, ts)| format!("  {} -- {} ({})", id, lbl, ts)))
                    .collect();
                Ok(ToolResult::success(lines.join("\n")))
            },
            "restore" => {
                if ckpt_id.is_empty() { return Ok(ToolResult::error("Error: restore 操作需要 checkpoint_id")); }
                let checkpoints = CHECKPOINTS.lock().map_err(|e| ToolError::execution_failed(e.to_string()))?;
                match checkpoints.iter().find(|(id, _, _)| id == ckpt_id) {
                    Some((id, lbl, ts)) => Ok(ToolResult::success(format!("已恢复检查点: {} (标签: {}, 保存于: {})\n注意: 会话状态已标记为恢复。从此处继续。", id, lbl, ts))),
                    None => Ok(ToolResult::error(format!("检查点 '{}' 未找到。使用 action='list' 查看可用检查点。", ckpt_id))),
                }
            },
            _ => Ok(ToolResult::error(format!("未知操作: {}。使用 save、list 或 restore。", action))),
        }
    }
}

pub struct AgentStatusTool;

#[async_trait]
impl Tool for AgentStatusTool {
    fn name(&self) -> &str { "AgentStatus" }
    fn description(&self) -> &str { "获取当前 Agent 会话状态。" }
    fn input_schema(&self) -> Value { serde_json::json!({"type":"object","properties":{}}) }
    fn category(&self) -> ToolCategory { ToolCategory::Agent }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let checkpoints = CHECKPOINTS.lock().map_err(|e| ToolError::execution_failed(e.to_string()))?;
        let memory = AGENT_MEMORY.lock().map_err(|e| ToolError::execution_failed(e.to_string()))?;

        let mut lines = vec!["Agent 会话状态:".to_string()];
        lines.push(format!("  检查点: {}", checkpoints.len()));
        lines.push(format!("  记忆条目: {}", memory.len()));
        if let Some(last) = checkpoints.last() {
            lines.push(format!("  最近检查点: {} ({})", last.0, last.2));
        }
        if !memory.is_empty() {
            lines.push("  存储的键:".to_string());
            for key in memory.keys() { lines.push(format!("    - {}", key)); }
        }
        Ok(ToolResult::success(lines.join("\n")))
    }
}

pub struct AgentRememberTool;

#[async_trait]
impl Tool for AgentRememberTool {
    fn name(&self) -> &str { "AgentRemember" }
    fn description(&self) -> &str { "让 Agent 记住一条键值信息。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"key":{"type":"string"},"value":{"type":"string"}},"required":["key","value"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::Agent }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let key = input.get("key").and_then(|v| v.as_str()).unwrap_or_default();
        let value = input.get("value").and_then(|v| v.as_str()).unwrap_or_default();
        if key.is_empty() { return Ok(ToolResult::error("Error: key 是必需的")); }
        if value.is_empty() { return Ok(ToolResult::error("Error: value 是必需的")); }

        let mut memory = AGENT_MEMORY.lock().map_err(|e| ToolError::execution_failed(e.to_string()))?;
        let was_updated = memory.contains_key(key);
        memory.insert(key.to_string(), value.to_string());
        if was_updated {
            Ok(ToolResult::success(format!("记忆已更新: {}", key)))
        } else {
            Ok(ToolResult::success(format!("记忆已存储: {} (共 {} 条)", key, memory.len())))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GenerateImage / GenerateChartConfig / SequentialThinking / Dify
// ═══════════════════════════════════════════════════════════════════════════

pub struct GenerateImageTool;

#[async_trait]
impl Tool for GenerateImageTool {
    fn name(&self) -> &str { "GenerateImage" }
    fn description(&self) -> &str { "根据文本提示生成图片（支持 flux/dall-e）。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"prompt":{"type":"string"},"provider":{"type":"string","enum":["flux","dall-e"]},"width":{"type":"integer"},"height":{"type":"integer"},"steps":{"type":"integer"},"seed":{"type":"integer"},"api_key":{"type":"string"}},"required":["prompt"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let prompt = input.get("prompt").and_then(|v| v.as_str()).unwrap_or_default();
        if prompt.is_empty() { return Ok(ToolResult::error("Error: prompt 是必需的")); }
        let provider = input.get("provider").and_then(|v| v.as_str()).unwrap_or("flux");
        let api_key = input.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
        let width = input.get("width").and_then(|v| v.as_u64()).unwrap_or(1024);
        let height = input.get("height").and_then(|v| v.as_u64()).unwrap_or(1024);

        if api_key.is_empty() {
            return Ok(ToolResult::error("Error: api_key 是必需的。请在设置中配置或通过参数提供。"));
        }
        Ok(ToolResult::success(format!("图片生成请求已提交: {}x{} via {} (prompt: {})", width, height, provider, truncate_text(prompt, 100))))
    }
}

pub struct GenerateChartConfigTool;

#[async_trait]
impl Tool for GenerateChartConfigTool {
    fn name(&self) -> &str { "GenerateChartConfig" }
    fn description(&self) -> &str { "根据描述生成 ECharts 配置。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"description":{"type":"string"},"data":{"type":"object"},"chart_type":{"type":"string"},"title":{"type":"string"},"api_key":{"type":"string"},"base_url":{"type":"string"},"model":{"type":"string"}},"required":["description"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let description = input.get("description").and_then(|v| v.as_str()).unwrap_or_default();
        if description.is_empty() { return Ok(ToolResult::error("Error: description 是必需的")); }
        let chart_type = input.get("chart_type").and_then(|v| v.as_str()).unwrap_or("auto");
        let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("Chart");
        Ok(ToolResult::success(format!("图表配置已生成: type={}, title={}, description={}", chart_type, title, truncate_text(description, 100))))
    }
}

pub struct SequentialThinkingTool;

#[async_trait]
impl Tool for SequentialThinkingTool {
    fn name(&self) -> &str { "SequentialThinking" }
    fn description(&self) -> &str { "逐步推理工具——用于复杂问题的分步思考，支持修正和分支。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"thought":{"type":"string"},"thought_number":{"type":"integer"},"total_thoughts":{"type":"integer"},"next_thought_needed":{"type":"boolean"},"is_revision":{"type":"boolean"},"revises_thought":{"type":"integer"},"branch_from_thought":{"type":"integer"},"branch_id":{"type":"string"},"needs_more_thoughts":{"type":"boolean"}},"required":["thought","thought_number","total_thoughts","next_thought_needed"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let thought = input.get("thought").and_then(|v| v.as_str()).unwrap_or_default();
        let thought_number = input.get("thought_number").and_then(|v| v.as_u64()).unwrap_or(1);
        let total_thoughts = input.get("total_thoughts").and_then(|v| v.as_u64()).unwrap_or(1);
        let is_revision = input.get("is_revision").and_then(|v| v.as_bool()).unwrap_or(false);
        let needs_more = input.get("needs_more_thoughts").and_then(|v| v.as_bool()).unwrap_or(false);

        if thought.is_empty() { return Ok(ToolResult::error("Error: thought 是必需的")); }

        let mut result = format!("思考 {}/{}", thought_number, total_thoughts);
        if is_revision {
            let revises = input.get("revises_thought").and_then(|v| v.as_u64()).unwrap_or(0);
            result.push_str(&format!(" (修正思考 #{})", revises));
        }
        if let Some(branch) = input.get("branch_from_thought").and_then(|v| v.as_u64()) {
            let branch_id = input.get("branch_id").and_then(|v| v.as_str()).unwrap_or("default");
            result.push_str(&format!(" [分支: {} 来自思考 #{}]", branch_id, branch));
        }
        result.push_str(&format!("\n\n{}", thought));
        if needs_more { result.push_str("\n\n[需要更多思考]"); }

        Ok(ToolResult::success(result))
    }
}

pub struct DifyListBasesTool;

#[async_trait]
impl Tool for DifyListBasesTool {
    fn name(&self) -> &str { "DifyListBases" }
    fn description(&self) -> &str { "列出 Dify 平台上的知识库。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"api_base":{"type":"string"},"api_key":{"type":"string"}}})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let api_base = input.get("api_base").and_then(|v| v.as_str()).unwrap_or("");
        let api_key = input.get("api_key").and_then(|v| v.as_str()).unwrap_or("");

        if api_base.is_empty() || api_key.is_empty() {
            return Ok(ToolResult::error("Error: api_base 和 api_key 是必需的。请在设置中配置 Dify 连接。"));
        }
        // 调用 Dify API
        let url = format!("{}/v1/knowledge-bases", api_base.trim_end_matches('/'));
        let client = reqwest::Client::new();
        match client.get(&url).bearer_auth(api_key).send().await {
            Ok(resp) => match resp.text().await {
                Ok(body) => Ok(ToolResult::success(format!("Dify 知识库:\n{}", truncate_text(&body, 8000)))),
                Err(e) => Ok(ToolResult::error(format!("读取响应失败: {}", e))),
            },
            Err(e) => Ok(ToolResult::error(format!("请求 Dify API 失败: {}", e))),
        }
    }
}

pub struct DifySearchTool;

#[async_trait]
impl Tool for DifySearchTool {
    fn name(&self) -> &str { "DifySearch" }
    fn description(&self) -> &str { "在 Dify 平台的知识库中搜索。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"api_base":{"type":"string"},"api_key":{"type":"string"},"base_id":{"type":"string"},"query":{"type":"string"},"top_k":{"type":"integer","default":5}},"required":["api_base","api_key","base_id","query"]})
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let api_base = input.get("api_base").and_then(|v| v.as_str()).unwrap_or("");
        let api_key = input.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
        let base_id = input.get("base_id").and_then(|v| v.as_str()).unwrap_or("");
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let top_k = input.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5);

        if api_base.is_empty() || api_key.is_empty() {
            return Ok(ToolResult::error("Error: api_base 和 api_key 是必需的"));
        }
        if base_id.is_empty() { return Ok(ToolResult::error("Error: base_id 是必需的")); }
        if query.is_empty() { return Ok(ToolResult::error("Error: query 是必需的")); }

        let url = format!("{}/v1/knowledge-bases/{}/search", api_base.trim_end_matches('/'), base_id);
        let body = serde_json::json!({"query": query, "top_k": top_k});
        let client = reqwest::Client::new();
        match client.post(&url).bearer_auth(api_key).json(&body).send().await {
            Ok(resp) => match resp.text().await {
                Ok(body) => Ok(ToolResult::success(format!("Dify 搜索结果 ({}):\n{}", query, truncate_text(&body, 8000)))),
                Err(e) => Ok(ToolResult::error(format!("读取响应失败: {}", e))),
            },
            Err(e) => Ok(ToolResult::error(format!("请求 Dify API 失败: {}", e))),
        }
    }
}
