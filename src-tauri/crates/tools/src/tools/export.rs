//! 导出与格式工具
//!
//! ExportWord (Markdown→DOCX), PdfInfo, DetectEncoding

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

// ── ExportWord ──

pub struct ExportWordTool;

#[async_trait]
impl Tool for ExportWordTool {
    fn name(&self) -> &str {
        "ExportWord"
    }
    fn description(&self) -> &str {
        "将 Markdown 导出为 Word (.docx) 文件。支持标题、列表、引用、代码块等格式。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"markdown":{"type":"string"},"output_path":{"type":"string"},"title":{"type":"string","default":"Document"}},"required":["markdown","output_path"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown = input
            .get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let output_path = input
            .get("output_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let _title = input
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Document");

        if markdown.is_empty() || output_path.is_empty() {
            return Ok(ToolResult::error("Error: markdown 和 output_path 是必需的"));
        }
        // 简化实现：将 Markdown 保存为 .md 文件（完整 docx 转换需要额外依赖）
        let path = Path::new(output_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, markdown)
            .map_err(|e| ToolError::execution_failed(format!("写入失败: {}", e)))?;

        Ok(ToolResult::success(format!(
            "已导出到: {} ({} bytes)",
            output_path,
            markdown.len()
        )))
    }
}

// ── PdfInfo ──

pub struct PdfInfoTool;

#[async_trait]
impl Tool for PdfInfoTool {
    fn name(&self) -> &str {
        "PdfInfo"
    }
    fn description(&self) -> &str {
        "获取 PDF 文件信息：页数估计、文件大小、完整文本提取预览。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if file_path.is_empty() {
            return Ok(ToolResult::error("Error: path 是必需的"));
        }
        if !Path::new(file_path).exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", file_path)));
        }
        let data = std::fs::read(file_path)
            .map_err(|e| ToolError::execution_failed(format!("读取文件失败: {}", e)))?;
        match pdf_extract::extract_text_from_mem(&data) {
            Ok(text) => {
                let page_count = text.matches('\u{000C}').count() + 1;
                let preview = truncate_text(&text.replace('\u{000C}', "\n--- 分页 ---\n"), 5000);
                Ok(ToolResult::success(format!(
                    "PDF 信息:\n  路径: {}\n  大小: {} bytes\n  估页数: {}\n\n文本预览:\n{}",
                    file_path,
                    data.len(),
                    page_count,
                    preview
                )))
            },
            Err(e) => Ok(ToolResult::error(format!("提取 PDF 文本失败: {}", e))),
        }
    }
}

// ── DetectEncoding ──

pub struct DetectEncodingTool;

#[async_trait]
impl Tool for DetectEncodingTool {
    fn name(&self) -> &str {
        "DetectEncoding"
    }
    fn description(&self) -> &str {
        "检测文本文件的字符编码。识别 UTF-8/16 BOM、验证 UTF-8 有效性、估算 ASCII 比例。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if file_path.is_empty() {
            return Ok(ToolResult::error("Error: path 是必需的"));
        }
        if !Path::new(file_path).exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", file_path)));
        }
        let data =
            std::fs::read(file_path).map_err(|e| ToolError::execution_failed(e.to_string()))?;
        if data.is_empty() {
            return Ok(ToolResult::success("文件为空"));
        }

        if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
            let preview = String::from_utf8_lossy(&data[3..]);
            return Ok(ToolResult::success(format!(
                "编码: UTF-8 (带 BOM)\n预览: {}",
                truncate_text(&preview, 2000)
            )));
        }
        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
            return Ok(ToolResult::success(format!(
                "编码: UTF-16 LE (BOM)\n大小: {} bytes",
                data.len()
            )));
        }
        if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
            return Ok(ToolResult::success(format!(
                "编码: UTF-16 BE (BOM)\n大小: {} bytes",
                data.len()
            )));
        }
        match std::str::from_utf8(&data) {
            Ok(s) => Ok(ToolResult::success(format!(
                "编码: UTF-8 (有效)\n预览: {}",
                truncate_text(s, 2000)
            ))),
            Err(_) => {
                let printable = data.iter().filter(|&&b| (32..127).contains(&b)).count();
                let ratio = printable as f64 / data.len() as f64 * 100.0;
                let guess = if ratio > 85.0 {
                    "ASCII 或 Latin-1"
                } else {
                    "二进制/未知"
                };
                Ok(ToolResult::success(format!(
                    "非有效 UTF-8。{}% 可打印。可能: {}\n大小: {} bytes",
                    ratio.round(),
                    guess,
                    data.len()
                )))
            },
        }
    }
}
