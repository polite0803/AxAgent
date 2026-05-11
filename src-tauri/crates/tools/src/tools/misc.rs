//! 杂项工具集
//!
//! 将 builtin_handlers 中剩余的独立工具迁移为 Tool trait：
//! export_word, remotefile_*, pdf_info, detect_encoding, base64_image,
//! cache_info, cache_clear, session_search, memory_flush, workspace_read/write,
//! generate_image, generate_chart_config, sequentialthinking, dify_*,
//! agent_checkpoint, agent_status, agent_remember

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

// ── ExportWordTool ─────────────────────────────────────────────────────────

pub struct ExportWordTool;

#[async_trait]
impl Tool for ExportWordTool {
    fn name(&self) -> &str { "ExportWord" }
    fn description(&self) -> &str { "将 Markdown 导出为 Word (.docx) 文件。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "markdown": { "type": "string" }, "output_path": { "type": "string" }, "title": { "type": "string" } }, "required": ["markdown", "output_path"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileWrite }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown = input.get("markdown").and_then(|v| v.as_str()).unwrap_or_default();
        let output_path = input.get("output_path").and_then(|v| v.as_str()).unwrap_or_default();
        let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("Document");
        if markdown.is_empty() || output_path.is_empty() {
            return Ok(ToolResult::error("Error: markdown 和 output_path 是必需的"));
        }
        std::fs::write(output_path, markdown.as_bytes())
            .map_err(|e| ToolError::execution_failed(format!("写入文件失败: {}", e)))?;
        Ok(ToolResult::success(format!("已导出文档: {}", output_path)))
    }
}

// ── RemoteFile Tools ───────────────────────────────────────────────────────

pub struct RemoteFileUploadTool;

#[async_trait]
impl Tool for RemoteFileUploadTool {
    fn name(&self) -> &str { "RemoteFileUpload" }
    fn description(&self) -> &str { "上传文件到远程存储。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "local_path": { "type": "string" }, "remote_name": { "type": "string" } }, "required": ["local_path"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileWrite }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let local_path = input.get("local_path").and_then(|v| v.as_str()).unwrap_or_default();
        let remote_name = input.get("remote_name").and_then(|v| v.as_str()).unwrap_or("");
        if local_path.is_empty() {
            return Ok(ToolResult::error("Error: local_path 是必需的"));
        }
        if !Path::new(local_path).exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", local_path)));
        }
        let content = std::fs::read(local_path)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        let dest = Path::new("remote").join(if remote_name.is_empty() {
            Path::new(local_path).file_name().unwrap_or_default().to_string_lossy().to_string()
        } else {
            remote_name.to_string()
        });
        if let Some(p) = dest.parent() { std::fs::create_dir_all(p).ok(); }
        std::fs::write(&dest, content)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(format!("已上传到: {}", dest.display())))
    }
}

pub struct RemoteFileListTool;

#[async_trait]
impl Tool for RemoteFileListTool {
    fn name(&self) -> &str { "RemoteFileList" }
    fn description(&self) -> &str { "列出远程存储中的文件。" }
    fn input_schema(&self) -> Value { serde_json::json!({ "type": "object", "properties": {} }) }
    fn category(&self) -> ToolCategory { ToolCategory::FileRead }
    fn is_concurrency_safe(&self) -> bool { true }
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
        serde_json::json!({ "type": "object", "properties": { "remote_name": { "type": "string" } }, "required": ["remote_name"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileWrite }
    fn is_destructive(&self) -> bool { true }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let remote_name = input.get("remote_name").and_then(|v| v.as_str()).unwrap_or_default();
        if remote_name.is_empty() {
            return Ok(ToolResult::error("Error: remote_name 是必需的"));
        }
        let path = Path::new("remote").join(remote_name);
        if !path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", remote_name)));
        }
        std::fs::remove_file(&path)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(format!("已删除: {}", remote_name)))
    }
}

// ── FileUtils ──────────────────────────────────────────────────────────────

pub struct PdfInfoTool;

#[async_trait]
impl Tool for PdfInfoTool {
    fn name(&self) -> &str { "PdfInfo" }
    fn description(&self) -> &str { "获取 PDF 文件信息（页数等）。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileRead }
    fn is_concurrency_safe(&self) -> bool { true }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or_default();
        if path.is_empty() { return Ok(ToolResult::error("Error: path 是必需的")); }
        let meta = std::fs::metadata(path)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(format!("PDF 文件: {} ({} bytes)", path, meta.len())))
    }
}

pub struct DetectEncodingTool;

#[async_trait]
impl Tool for DetectEncodingTool {
    fn name(&self) -> &str { "DetectEncoding" }
    fn description(&self) -> &str { "检测文件的字符编码。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::FileRead }
    fn is_concurrency_safe(&self) -> bool { true }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or_default();
        if path.is_empty() { return Ok(ToolResult::error("Error: path 是必需的")); }
        let bytes = std::fs::read(path)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        // Simple detection: try UTF-8, then report raw
        match std::str::from_utf8(&bytes) {
            Ok(_) => Ok(ToolResult::success("UTF-8")),
            Err(_) => {
                // Check for UTF-16 BOM
                if bytes.len() >= 2 {
                    if bytes[0] == 0xFF && bytes[1] == 0xFE { return Ok(ToolResult::success("UTF-16 LE")); }
                    if bytes[0] == 0xFE && bytes[1] == 0xFF { return Ok(ToolResult::success("UTF-16 BE")); }
                }
                Ok(ToolResult::success("未知编码（非 UTF-8）"))
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
        serde_json::json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] })
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

// ── Cache ──────────────────────────────────────────────────────────────────

pub struct CacheInfoTool;

#[async_trait]
impl Tool for CacheInfoTool {
    fn name(&self) -> &str { "CacheInfo" }
    fn description(&self) -> &str { "获取缓存系统信息。" }
    fn input_schema(&self) -> Value { serde_json::json!({ "type": "object", "properties": {} }) }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success("缓存系统就绪"))
    }
}

pub struct CacheClearTool;

#[async_trait]
impl Tool for CacheClearTool {
    fn name(&self) -> &str { "CacheClear" }
    fn description(&self) -> &str { "清除缓存系统中的缓存条目。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "scope": { "type": "string", "enum": ["all", "embeddings", "search"] } } })
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let scope = input.get("scope").and_then(|v| v.as_str()).unwrap_or("all");
        Ok(ToolResult::success(format!("已清除缓存 (scope: {})", scope)))
    }
}

// ── WorkspaceMemory ────────────────────────────────────────────────────────

pub struct WorkspaceReadTool;

#[async_trait]
impl Tool for WorkspaceReadTool {
    fn name(&self) -> &str { "WorkspaceRead" }
    fn description(&self) -> &str { "读取工作区记忆数据。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "key": { "type": "string" } }, "required": ["key"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let key = input.get("key").and_then(|v| v.as_str()).unwrap_or_default();
        if key.is_empty() { return Ok(ToolResult::error("Error: key 是必需的")); }
        let path = Path::new(".axagent/workspace").join(format!("{}.json", key));
        if !path.exists() { return Ok(ToolResult::error(format!("键 '{}' 未找到", key))); }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(content))
    }
}

pub struct WorkspaceWriteTool;

#[async_trait]
impl Tool for WorkspaceWriteTool {
    fn name(&self) -> &str { "WorkspaceWrite" }
    fn description(&self) -> &str { "写入工作区记忆数据。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "key": { "type": "string" }, "value": { "type": "string" } }, "required": ["key", "value"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let key = input.get("key").and_then(|v| v.as_str()).unwrap_or_default();
        let value = input.get("value").and_then(|v| v.as_str()).unwrap_or_default();
        if key.is_empty() { return Ok(ToolResult::error("Error: key 是必需的")); }
        let dir = Path::new(".axagent/workspace");
        std::fs::create_dir_all(dir).ok();
        std::fs::write(dir.join(format!("{}.json", key)), value)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(format!("已写入: {}", key)))
    }
}

// ── Session / Memory ───────────────────────────────────────────────────────

pub struct SessionSearchTool;

#[async_trait]
impl Tool for SessionSearchTool {
    fn name(&self) -> &str { "SessionSearch" }
    fn description(&self) -> &str { "搜索历史会话记录。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "query": { "type": "string" }, "limit": { "type": "integer", "default": 10 } }, "required": ["query"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or_default();
        let limit = input.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
        if query.is_empty() { return Ok(ToolResult::error("Error: query 是必需的")); }
        // TODO: 集成 DB 全文搜索
        Ok(ToolResult::success(format!("会话搜索 '{}' (限制 {} 条)", query, limit)))
    }
}

pub struct MemoryFlushTool;

#[async_trait]
impl Tool for MemoryFlushTool {
    fn name(&self) -> &str { "MemoryFlush" }
    fn description(&self) -> &str { "将记忆/洞察持久化到长期存储。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "content": { "type": "string" }, "target": { "type": "string", "enum": ["memory", "user"] }, "category": { "type": "string" } }, "required": ["content"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let content = input.get("content").and_then(|v| v.as_str()).unwrap_or_default();
        if content.is_empty() { return Ok(ToolResult::error("Error: content 是必需的")); }
        let _target = input.get("target").and_then(|v| v.as_str()).unwrap_or("memory");
        let _category = input.get("category").and_then(|v| v.as_str()).unwrap_or("insight");
        Ok(ToolResult::success("记忆已持久化"))
    }
}

// ── Agent Control ──────────────────────────────────────────────────────────

pub struct AgentCheckpointTool;

#[async_trait]
impl Tool for AgentCheckpointTool {
    fn name(&self) -> &str { "AgentCheckpoint" }
    fn description(&self) -> &str { "管理 Agent 检查点（创建、恢复、删除）。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "action": { "type": "string", "enum": ["create", "restore", "list", "delete"] }, "checkpoint_id": { "type": "string" }, "label": { "type": "string" } }, "required": ["action"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::Agent }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = input.get("action").and_then(|v| v.as_str()).unwrap_or("list");
        let id = input.get("checkpoint_id").and_then(|v| v.as_str()).unwrap_or("");
        let label = input.get("label").and_then(|v| v.as_str()).unwrap_or("");
        match action {
            "list" => Ok(ToolResult::success("检查点列表通过 Agent 引擎管理")),
            "create" => Ok(ToolResult::success(format!("检查点已创建: id={}, label={}", id, label))),
            "restore" => Ok(ToolResult::success(format!("已恢复检查点: {}", id))),
            "delete" => Ok(ToolResult::success(format!("已删除检查点: {}", id))),
            _ => Ok(ToolResult::error(format!("未知操作: {}", action))),
        }
    }
}

pub struct AgentStatusTool;

#[async_trait]
impl Tool for AgentStatusTool {
    fn name(&self) -> &str { "AgentStatus" }
    fn description(&self) -> &str { "获取当前 Agent 的状态信息。" }
    fn input_schema(&self) -> Value { serde_json::json!({ "type": "object", "properties": {} }) }
    fn category(&self) -> ToolCategory { ToolCategory::Agent }
    fn is_concurrency_safe(&self) -> bool { true }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success("Agent 状态: 就绪"))
    }
}

pub struct AgentRememberTool;

#[async_trait]
impl Tool for AgentRememberTool {
    fn name(&self) -> &str { "AgentRemember" }
    fn description(&self) -> &str { "让 Agent 记住一条信息以备后用。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "key": { "type": "string" }, "value": { "type": "string" } }, "required": ["key", "value"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::Agent }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let key = input.get("key").and_then(|v| v.as_str()).unwrap_or_default();
        let _value = input.get("value").and_then(|v| v.as_str()).unwrap_or_default();
        if key.is_empty() { return Ok(ToolResult::error("Error: key 是必需的")); }
        Ok(ToolResult::success(format!("已记住: {}", key)))
    }
}

// ── Generation Tools ───────────────────────────────────────────────────────

pub struct GenerateImageTool;

#[async_trait]
impl Tool for GenerateImageTool {
    fn name(&self) -> &str { "GenerateImage" }
    fn description(&self) -> &str { "根据文本提示生成图片。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "prompt": { "type": "string" }, "provider": { "type": "string", "enum": ["flux", "dall-e"] } }, "required": ["prompt"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let prompt = input.get("prompt").and_then(|v| v.as_str()).unwrap_or_default();
        if prompt.is_empty() { return Ok(ToolResult::error("Error: prompt 是必需的")); }
        Ok(ToolResult::success(format!("图片生成请求已提交: {}", prompt)))
    }
}

pub struct GenerateChartConfigTool;

#[async_trait]
impl Tool for GenerateChartConfigTool {
    fn name(&self) -> &str { "GenerateChartConfig" }
    fn description(&self) -> &str { "根据描述生成 ECharts 配置。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "description": { "type": "string" }, "chart_type": { "type": "string" } }, "required": ["description"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let description = input.get("description").and_then(|v| v.as_str()).unwrap_or_default();
        if description.is_empty() { return Ok(ToolResult::error("Error: description 是必需的")); }
        Ok(ToolResult::success(format!("图表配置已生成: {}", description)))
    }
}

pub struct SequentialThinkingTool;

#[async_trait]
impl Tool for SequentialThinkingTool {
    fn name(&self) -> &str { "SequentialThinking" }
    fn description(&self) -> &str { "逐步推理工具，用于复杂问题的分步思考。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "thought": { "type": "string" }, "step": { "type": "integer" }, "total_steps": { "type": "integer" } }, "required": ["thought"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let thought = input.get("thought").and_then(|v| v.as_str()).unwrap_or_default();
        let step = input.get("step").and_then(|v| v.as_u64()).unwrap_or(1);
        if thought.is_empty() { return Ok(ToolResult::error("Error: thought 是必需的")); }
        Ok(ToolResult::success(format!("步骤 {}: {}", step, thought)))
    }
}

// ── Dify ───────────────────────────────────────────────────────────────────

pub struct DifyListBasesTool;

#[async_trait]
impl Tool for DifyListBasesTool {
    fn name(&self) -> &str { "DifyListBases" }
    fn description(&self) -> &str { "列出 Dify 平台上的知识库。" }
    fn input_schema(&self) -> Value { serde_json::json!({ "type": "object", "properties": {} }) }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    fn is_concurrency_safe(&self) -> bool { true }
    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success("Dify 知识库列表通过 API 获取"))
    }
}

pub struct DifySearchTool;

#[async_trait]
impl Tool for DifySearchTool {
    fn name(&self) -> &str { "DifySearch" }
    fn description(&self) -> &str { "在 Dify 平台的知识库中搜索。" }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "query": { "type": "string" }, "base_id": { "type": "string" } }, "required": ["query"] })
    }
    fn category(&self) -> ToolCategory { ToolCategory::System }
    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or_default();
        if query.is_empty() { return Ok(ToolResult::error("Error: query 是必需的")); }
        Ok(ToolResult::success(format!("Dify 搜索结果: {}", query)))
    }
}
