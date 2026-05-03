//! FileReadTool - 文件读取工具
//!
//! 支持文本文件（行范围）、图片、PDF、IDocument 的读取。

use crate::{PermissionResult, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

/// 设备文件黑名单
const DEVICE_FILE_BLACKLIST: &[&str] = &[
    "/dev/zero",
    "/dev/random",
    "/dev/urandom",
    "/dev/full",
    "/dev/tty",
    "/dev/stdin",
    "/dev/stdout",
    "/dev/stderr",
];

/// 大文件阈值（超过则不允许全量读取）
const LARGE_FILE_THRESHOLD_MB: u64 = 50;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "FileRead"
    }
    fn description(&self) -> &str {
        "读取文件内容。支持文本文件（可指定行范围）、图片、PDF。支持偏移量和行数限制。文件路径必须是绝对路径。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "要读取的文件绝对路径"
                },
                "offset": {
                    "type": "integer",
                    "description": "从第几行开始读取（0 表示从开头）"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多读取多少行（默认 2000）"
                }
            },
            "required": ["file_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ToolError> {
        let path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileRead", "缺少 file_path 参数"))?;

        if !Path::new(path).is_absolute() {
            return Err(ToolError::invalid_input_for(
                "FileRead",
                "file_path 必须是绝对路径",
            ));
        }

        // 设备文件检查
        for dev in DEVICE_FILE_BLACKLIST {
            if path.starts_with(dev) {
                return Err(ToolError::permission_denied(
                    "FileRead",
                    &format!("禁止读取设备文件: {}", dev),
                ));
            }
        }

        Ok(())
    }

    fn check_permissions(&self, _input: &Value, _ctx: &ToolContext) -> PermissionResult {
        PermissionResult::Allow
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input["file_path"].as_str().unwrap();
        let offset: usize = input.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit: usize = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;

        let path = Path::new(file_path);

        // 检查文件存在
        if !path.exists() {
            return Err(ToolError::invalid_input(format!(
                "文件不存在: {}",
                file_path
            )));
        }

        if !path.is_file() {
            return Err(ToolError::invalid_input(format!("不是文件: {}", file_path)));
        }

        // 设备文件二次检查
        let path_str = file_path.to_lowercase();
        for dev in DEVICE_FILE_BLACKLIST {
            if path_str.contains(dev) {
                return Err(ToolError::permission_denied(
                    "FileRead",
                    &format!("禁止读取设备文件: {}", dev),
                ));
            }
        }

        // 大小检查
        let metadata = std::fs::metadata(path)
            .map_err(|e| ToolError::execution_failed(format!("无法获取文件信息: {}", e)))?;
        if metadata.len() > LARGE_FILE_THRESHOLD_MB * 1024 * 1024 {
            return Err(ToolError::invalid_input(format!(
                "文件过大 ({} MB)，最大允许 {} MB。请使用 offset/limit 分段读取。",
                metadata.len() / 1024 / 1024,
                LARGE_FILE_THRESHOLD_MB
            )));
        }

        // 根据扩展名判断文件类型
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" => read_image(file_path),
            "pdf" => read_pdf(file_path),
            "ipynb" => read_notebook(file_path),
            _ => read_text(file_path, offset, limit),
        }
    }
}

fn read_text(path: &str, offset: usize, limit: usize) -> Result<ToolResult, ToolError> {
    // 检测编码（尝试 UTF-8，失败则尝试其他编码）
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            let bytes = std::fs::read(path)
                .map_err(|e| ToolError::execution_failed(format!("读取失败: {}", e)))?;

            // 简单处理：跳过非 UTF-8 字节
            String::from_utf8_lossy(&bytes).to_string()
        },
    };

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if offset >= total_lines {
        return Ok(ToolResult::success(format!(
            "文件共 {} 行 (offset={offset} 超出范围)",
            total_lines
        )));
    }

    let end = std::cmp::min(offset + limit, total_lines);
    let selected: Vec<&str> = lines[offset..end].to_vec();

    let mut output = String::new();
    for (i, line) in selected.iter().enumerate() {
        let line_no = offset + i + 1;
        output.push_str(&format!("{:>6}\t{}\n", line_no, line));
    }

    if end < total_lines {
        output.push_str(&format!(
            "\n[显示 {offset}-{end} / {total_lines} 行，可增加 limit 或调整 offset 读取更多]"
        ));
    }

    if content.len() > 200_000 {
        return Ok(ToolResult::truncated(output, 200_000));
    }

    Ok(ToolResult::success(output))
}

fn read_image(path: &str) -> Result<ToolResult, ToolError> {
    // 图片文件以 base64 编码返回
    let bytes = std::fs::read(path)
        .map_err(|e| ToolError::execution_failed(format!("读取图片失败: {}", e)))?;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let ext = Path::new(path).extension().unwrap().to_str().unwrap();

    Ok(ToolResult::success(format!(
        "[图片] {} ({:.1} KB)\ndata:image/{};base64,{}",
        path,
        bytes.len() as f64 / 1024.0,
        ext,
        b64
    )))
}

fn read_pdf(path: &str) -> Result<ToolResult, ToolError> {
    // 使用 pdf-extract 或回退到原始文本提取
    match pdf_extract::extract_text(path) {
        Ok(text) => {
            let text: String = text;
            if text.len() > 200_000 {
                Ok(ToolResult::truncated(text, 200_000))
            } else {
                Ok(ToolResult::success(text))
            }
        },
        Err(e) => Err(ToolError::execution_failed(format!("PDF 读取失败: {}", e))),
    }
}

fn read_notebook(path: &str) -> Result<ToolResult, ToolError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ToolError::execution_failed(format!("读取 Notebook 失败: {}", e)))?;

    let nb: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| ToolError::execution_failed(format!("Notebook JSON 解析失败: {}", e)))?;

    let mut output = String::new();
    output.push_str(&format!("# Notebook: {}\n\n", path));

    if let Some(cells) = nb["cells"].as_array() {
        for (i, cell) in cells.iter().enumerate() {
            let cell_type = cell["cell_type"].as_str().unwrap_or("unknown");
            let source = cell["source"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();

            output.push_str(&format!("## Cell {} [{}]\n{}\n\n", i, cell_type, source));
        }
    }

    Ok(ToolResult::success(output))
}
