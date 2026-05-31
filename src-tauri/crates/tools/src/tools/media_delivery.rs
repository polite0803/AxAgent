//! Media Smart Delivery - 智能媒体文件检测与投递
//!
//! MediaDetectTool: 扫描文本中的绝对路径，检测媒体类型，验证文件可读性
//! MediaDeliverTool: 处理投递指令，返回清理后的文本和媒体附件列表
//! MediaPreviewTool: 生成媒体文件的预览信息

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
enum MediaType {
    Image,
    Audio,
    Video,
    Document,
}

impl MediaType {
    fn as_str(&self) -> &'static str {
        match self {
            MediaType::Image => "image",
            MediaType::Audio => "audio",
            MediaType::Video => "video",
            MediaType::Document => "document",
        }
    }
}

fn detect_media_type(ext: &str) -> Option<MediaType> {
    match ext.to_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "tiff" | "tif" => {
            Some(MediaType::Image)
        },
        "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" | "wma" => Some(MediaType::Audio),
        "mp4" | "webm" | "avi" | "mkv" | "mov" | "wmv" | "flv" => Some(MediaType::Video),
        "pdf" | "docx" | "xlsx" | "pptx" | "doc" | "xls" | "ppt" | "odt" | "ods" | "odp" => {
            Some(MediaType::Document)
        },
        _ => None,
    }
}

fn extension_to_mime(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "tiff" | "tif" => "image/tiff",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        "wma" => "audio/x-ms-wma",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "doc" => "application/msword",
        "xls" => "application/vnd.ms-excel",
        "ppt" => "application/vnd.ms-powerpoint",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn detect_mime_from_bytes(data: &[u8], fallback_ext: &str) -> String {
    if let Some(kind) = infer::get(data) {
        return kind.mime_type().to_string();
    }
    extension_to_mime(fallback_ext)
}

fn extract_absolute_paths(text: &str) -> Vec<String> {
    let re = regex::Regex::new(r#"(?:(?:[A-Za-z]:[/\\])|/)[^\s"'<>\]\)}，。；：！？、]+"#).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut paths = Vec::new();
    for cap in re.captures_iter(text) {
        let p = cap[0].to_string();
        let cleaned = p.trim_end_matches(|c: char| c == '.' || c == ',' || c == ';' || c == ':');
        if seen.insert(cleaned.to_string()) {
            paths.push(cleaned.to_string());
        }
    }
    paths
}

fn scan_media_from_text(text: &str) -> Vec<Value> {
    let paths = extract_absolute_paths(text);
    let mut results = Vec::new();
    for path_str in paths {
        let path = Path::new(&path_str);
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => continue,
        };
        let media_type = match detect_media_type(ext) {
            Some(mt) => mt,
            None => continue,
        };
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !metadata.is_file() {
            continue;
        }
        let file_size = metadata.len();
        let mime_type = match std::fs::read(path) {
            Ok(data) if !data.is_empty() => detect_mime_from_bytes(&data, ext),
            _ => extension_to_mime(ext),
        };
        results.push(serde_json::json!({
            "path": path_str,
            "media_type": media_type.as_str(),
            "file_size": file_size,
            "mime_type": mime_type,
        }));
    }
    results
}

// ── MediaDetectTool ──

pub struct MediaDetectTool;

#[async_trait]
impl Tool for MediaDetectTool {
    fn name(&self) -> &str {
        "MediaDetect"
    }
    fn description(&self) -> &str {
        "扫描文本中的绝对文件路径，检测媒体类型（图片/音频/视频/文档），验证文件存在且可读。返回结构化的媒体列表：path, media_type, file_size, mime_type。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "要扫描的文本内容（通常是 Agent 响应文本）"
                }
            },
            "required": ["text"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::AiMedia
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let text = input
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if text.is_empty() {
            return Ok(ToolResult::error("Error: text 是必需的"));
        }
        let media_list = scan_media_from_text(text);
        if media_list.is_empty() {
            return Ok(ToolResult {
                content: "未检测到媒体文件".to_string(),
                is_error: false,
                truncated: false,
                metadata: Some(serde_json::json!({
                    "media_count": 0,
                    "media": []
                })),
                duration_ms: None,
                progress: Vec::new(),
            });
        }
        let count = media_list.len();
        let mut summary_parts = Vec::new();
        for m in &media_list {
            let path = m["path"].as_str().unwrap_or("?");
            let mt = m["media_type"].as_str().unwrap_or("?");
            let size = m["file_size"].as_u64().unwrap_or(0);
            let size_str = if size < 1024 {
                format!("{}B", size)
            } else if size < 1024 * 1024 {
                format!("{:.1}KB", size as f64 / 1024.0)
            } else {
                format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
            };
            summary_parts.push(format!("- [{}] {} ({})", mt, path, size_str));
        }
        Ok(ToolResult {
            content: format!("检测到 {} 个媒体文件:\n{}", count, summary_parts.join("\n")),
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "media_count": count,
                "media": media_list,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

// ── MediaDeliverTool ──

pub struct MediaDeliverTool;

#[async_trait]
impl Tool for MediaDeliverTool {
    fn name(&self) -> &str {
        "MediaDeliver"
    }
    fn description(&self) -> &str {
        "处理响应文本中的媒体文件投递。支持投递指令：[[audio_as_voice]] 将音频提升为语音消息气泡；[[as_document]] 将所有媒体作为文档附件投递（避免有损压缩）。指令文本会从响应中移除。返回清理后的文本和媒体附件列表。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Agent 响应文本（可能包含投递指令和文件路径）"
                }
            },
            "required": ["text"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::AiMedia
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let text = input
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if text.is_empty() {
            return Ok(ToolResult::error("Error: text 是必需的"));
        }
        let audio_as_voice = text.contains("[[audio_as_voice]]");
        let as_document = text.contains("[[as_document]]");
        let cleaned = text
            .replace("[[audio_as_voice]]", "")
            .replace("[[as_document]]", "");
        let cleaned = cleaned.trim().to_string();
        let media_list = scan_media_from_text(text);
        let mut attachments = Vec::new();
        for m in &media_list {
            let media_type_str = m["media_type"].as_str().unwrap_or("document");
            let mut delivery_mode = "native";
            if as_document {
                delivery_mode = "document";
            } else if audio_as_voice && media_type_str == "audio" {
                delivery_mode = "voice";
            }
            attachments.push(serde_json::json!({
                "path": m["path"],
                "media_type": m["media_type"],
                "file_size": m["file_size"],
                "mime_type": m["mime_type"],
                "delivery_mode": delivery_mode,
            }));
        }
        let attach_count = attachments.len();
        let mut mode_desc = Vec::new();
        if audio_as_voice {
            mode_desc.push("audio_as_voice");
        }
        if as_document {
            mode_desc.push("as_document");
        }
        let mode_info = if mode_desc.is_empty() {
            "native".to_string()
        } else {
            mode_desc.join(", ")
        };
        let content = if attach_count == 0 {
            format!("无媒体附件 | 投递模式: {}", mode_info)
        } else {
            format!("投递模式: {} | 媒体附件: {} 个", mode_info, attach_count)
        };
        Ok(ToolResult {
            content,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "cleaned_text": cleaned,
                "delivery_mode": mode_info,
                "attachment_count": attach_count,
                "attachments": attachments,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

// ── MediaPreviewTool ──

pub struct MediaPreviewTool;

#[async_trait]
impl Tool for MediaPreviewTool {
    fn name(&self) -> &str {
        "MediaPreview"
    }
    fn description(&self) -> &str {
        "生成媒体文件的预览信息。图片: 尺寸/格式/大小；音频: 时长/格式/大小；文档: 页数/格式/大小。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "媒体文件的绝对路径"
                }
            },
            "required": ["path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::AiMedia
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if path_str.is_empty() {
            return Ok(ToolResult::error("Error: path 是必需的"));
        }
        let path = Path::new(path_str);
        if !path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", path.display())));
        }
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => return Ok(ToolResult::error(format!("无法读取文件元数据: {}", e))),
        };
        if !metadata.is_file() {
            return Ok(ToolResult::error(format!("不是文件: {}", path.display())));
        }
        let file_size = metadata.len();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        let media_type = match detect_media_type(&ext) {
            Some(mt) => mt,
            None => return Ok(ToolResult::error(format!("无法识别的媒体类型: .{}", ext))),
        };
        let file_data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => return Ok(ToolResult::error(format!("无法读取文件: {}", e))),
        };
        let mime_type = if !file_data.is_empty() {
            detect_mime_from_bytes(&file_data, &ext)
        } else {
            extension_to_mime(&ext)
        };
        let mut preview = serde_json::json!({
            "path": path_str,
            "media_type": media_type.as_str(),
            "format": ext.to_lowercase(),
            "file_size": file_size,
            "mime_type": mime_type,
        });
        match media_type {
            MediaType::Image => {
                if let Ok(img_reader) =
                    image::ImageReader::new(std::io::Cursor::new(&file_data)).with_guessed_format()
                {
                    if let Ok(dims) = img_reader.into_dimensions() {
                        preview["width"] = Value::Number(dims.0.into());
                        preview["height"] = Value::Number(dims.1.into());
                    }
                }
            },
            MediaType::Audio => {
                let duration_secs = estimate_audio_duration(&file_data, &ext, file_size);
                if let Some(dur) = duration_secs {
                    preview["duration_secs"] = Value::Number(
                        serde_json::Number::from_f64(dur).unwrap_or(serde_json::Number::from(0)),
                    );
                }
            },
            MediaType::Document => {
                let page_count = estimate_document_pages(&file_data, &ext);
                if let Some(pages) = page_count {
                    preview["page_count"] = Value::Number(pages.into());
                }
            },
            MediaType::Video => {},
        }
        let size_str = format_size(file_size);
        let mut content_parts = vec![format!(
            "[{}] {} | {} | {}",
            media_type.as_str(),
            path_str,
            ext.to_uppercase(),
            size_str
        )];
        if let Some(w) = preview.get("width").and_then(|v| v.as_u64()) {
            if let Some(h) = preview.get("height").and_then(|v| v.as_u64()) {
                content_parts.push(format!("尺寸: {}x{}", w, h));
            }
        }
        if let Some(d) = preview.get("duration_secs").and_then(|v| v.as_f64()) {
            content_parts.push(format!("时长: {:.1}s", d));
        }
        if let Some(p) = preview.get("page_count").and_then(|v| v.as_u64()) {
            content_parts.push(format!("页数: {}", p));
        }
        Ok(ToolResult {
            content: content_parts.join(" | "),
            is_error: false,
            truncated: false,
            metadata: Some(preview),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn estimate_audio_duration(data: &[u8], ext: &str, file_size: u64) -> Option<f64> {
    match ext.to_lowercase().as_str() {
        "mp3" => {
            let bitrate = estimate_mp3_bitrate(data);
            if bitrate > 0 {
                Some((file_size as f64 * 8.0) / (bitrate as f64 * 1000.0))
            } else {
                None
            }
        },
        "wav" => {
            if data.len() > 28 {
                let channels = u16::from_le_bytes([data[22], data[23]]) as u32;
                let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
                let bits_per_sample = u16::from_le_bytes([data[34], data[35]]) as u32;
                let byte_rate = channels * sample_rate * (bits_per_sample / 8);
                if byte_rate > 0 {
                    Some(file_size as f64 / byte_rate as f64)
                } else {
                    None
                }
            } else {
                None
            }
        },
        "ogg" | "flac" | "aac" | "m4a" | "wma" => None,
        _ => None,
    }
}

fn estimate_mp3_bitrate(data: &[u8]) -> u32 {
    let bitrate_table_v1 = [
        [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
        ],
        [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
        ],
        [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
        ],
    ];
    for i in 0..data.len().saturating_sub(4) {
        if data[i] == 0xFF && (data[i + 1] & 0xE0) == 0xE0 {
            let version = (data[i + 1] >> 3) & 0x03;
            let layer = (data[i + 1] >> 1) & 0x03;
            let bitrate_idx = (data[i + 2] >> 4) & 0x0F;
            if version == 0x03 && layer == 0x01 && (bitrate_idx as usize) < 15 {
                return bitrate_table_v1[0][bitrate_idx as usize];
            }
        }
    }
    128
}

fn estimate_document_pages(data: &[u8], ext: &str) -> Option<u64> {
    match ext.to_lowercase().as_str() {
        "pdf" => estimate_pdf_pages(data),
        "docx" => estimate_docx_pages(data),
        _ => None,
    }
}

fn estimate_pdf_pages(data: &[u8]) -> Option<u64> {
    let text = String::from_utf8_lossy(data);
    let mut count = 0u64;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("/Type") && trimmed.contains("/Page") && !trimmed.contains("/Pages")
        {
            count += 1;
        }
    }
    if count > 0 {
        return Some(count);
    }
    let re = regex::Regex::new(r#"/Type\s*/Page[^s]"#).ok()?;
    Some(re.find_iter(&text).count() as u64)
}

fn estimate_docx_pages(data: &[u8]) -> Option<u64> {
    let reader = std::io::Cursor::new(data);
    let mut archive = match zip::ZipArchive::new(reader) {
        Ok(a) => a,
        Err(_) => return None,
    };
    let mut char_count = 0u64;
    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let name = file.name().to_string();
        if name.ends_with("document.xml") {
            let mut content = String::new();
            if let Err(_) = std::io::Read::read_to_string(&mut file, &mut content) {
                continue;
            }
            let text_re = regex::Regex::new(r"<w:t[^>]*>([^<]+)</w:t>").ok()?;
            for cap in text_re.captures_iter(&content) {
                char_count += cap[1].len() as u64;
            }
            break;
        }
    }
    if char_count > 0 {
        Some((char_count / 3000).max(1))
    } else {
        None
    }
}
