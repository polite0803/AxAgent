//! OCR 光学字符识别工具
//!
//! 将 builtin_handlers 中的 ocr_image、ocr_detect_langs 迁移为 Tool trait。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

pub struct OcrImageTool;

#[async_trait]
impl Tool for OcrImageTool {
    fn name(&self) -> &str {
        "OcrImage"
    }
    fn description(&self) -> &str {
        "使用 Tesseract OCR 从图片中提取文字。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "file_path": { "type": "string" }, "lang": { "type": "string", "default": "eng" } }, "required": ["file_path"] })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::AiMedia
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let lang = input.get("lang").and_then(|v| v.as_str()).unwrap_or("eng");

        if file_path.is_empty() {
            return Ok(ToolResult::error("Error: file_path 是必需的"));
        }
        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", file_path)));
        }
        let meta = std::fs::metadata(file_path)
            .map_err(|e| ToolError::execution_failed(format!("读取文件元数据失败: {}", e)))?;
        if meta.len() > 50 * 1024 * 1024 {
            return Ok(ToolResult::error("图片过大 (最大 50 MB)"));
        }
        let safe_lang = if lang.is_empty()
            || lang.contains("..")
            || lang.contains("/")
            || lang.contains("\\")
        {
            "eng"
        } else {
            lang
        };

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            tokio::process::Command::new("tesseract")
                .arg(file_path)
                .arg("stdout")
                .arg("-l")
                .arg(safe_lang)
                .output(),
        )
        .await;

        match output {
            Ok(Ok(out)) => {
                let text = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    let detail = if !stderr.is_empty() { format!(" Tesseract stderr: {}", stderr.trim()) } else { String::new() };
                    return Ok(ToolResult::success(format!("OCR 未识别出文字。图片可能不含可识别文本，或语言包 '{}' 未安装。{} 使用 OcrDetectLangs 查看可用语言。", safe_lang, detail)));
                }
                Ok(ToolResult::success(trimmed.to_string()))
            },
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(ToolResult::error("Tesseract 未安装。安装方法:\n  - macOS: brew install tesseract tesseract-lang\n  - Ubuntu: sudo apt install tesseract-ocr\n  - Windows: https://github.com/UB-Mannheim/tesseract/wiki"))
            },
            Ok(Err(e)) => Ok(ToolResult::error(format!("运行 tesseract 失败: {}", e))),
            Err(_) => Ok(ToolResult::error("OCR 超时 (120 秒)")),
        }
    }
}

pub struct OcrDetectLangsTool;

#[async_trait]
impl Tool for OcrDetectLangsTool {
    fn name(&self) -> &str {
        "OcrDetectLangs"
    }
    fn description(&self) -> &str {
        "检测已安装的 Tesseract OCR 语言包。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::AiMedia
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::process::Command::new("tesseract")
                .arg("--list-langs")
                .output(),
        )
        .await;

        match output {
            Ok(Ok(out)) => {
                let text = String::from_utf8_lossy(&out.stdout);
                let langs: Vec<&str> = text.lines().skip(1).filter(|l| !l.is_empty()).collect();
                if langs.is_empty() {
                    Ok(ToolResult::success("未检测到 Tesseract 语言包"))
                } else {
                    Ok(ToolResult::success(format!(
                        "可用 Tesseract 语言 ({}):\n{}",
                        langs.len(),
                        langs.join("\n")
                    )))
                }
            },
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(ToolResult::error("Tesseract 未安装"))
            },
            Ok(Err(e)) => Ok(ToolResult::error(format!("Error: {}", e))),
            Err(_) => Ok(ToolResult::error("检测超时")),
        }
    }
}
