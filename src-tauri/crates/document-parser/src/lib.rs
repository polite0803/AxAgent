#![allow(clippy::result_large_err)]
// SPDX-License-Identifier: AGPL-3.0-only

//! Document text extraction — PDF, DOCX, XLSX, PPTX, and plain text.
//!
//! Extracted from `axagent-core` as part of harness architecture refactoring.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use axagent_harness::core_error::{AxAgentError, Result};

/// Extract plain text from a document file based on its MIME type.
pub fn extract_text(file_path: &Path, mime_type: &str) -> Result<String> {
    match mime_type {
        // Plain text files
        "text/plain" | "text/markdown" | "text/csv" | "text/html" | "text/xml"
        | "application/json" | "application/xml" => {
            std::fs::read_to_string(file_path).map_err(|e| {
                AxAgentError::execution_with_source(format!("Failed to read file: {file_path}"), e)
            })
        },

        // PDF
        "application/pdf" => {
            let text = extract_pdf(file_path)?;
            // 扫描版 PDF 文本层为空时回退到 OCR
            if text.trim().is_empty() {
                let ocr_text = ocr_fallback(file_path).unwrap_or_default();
                if !ocr_text.trim().is_empty() {
                    return Ok(ocr_text);
                }
            }
            Ok(text)
        },

        // 图片类型 —— 直接走 OCR
        "image/png" | "image/jpeg" | "image/tiff" | "image/bmp" | "image/webp" => {
            let ocr_text = ocr_fallback(file_path).unwrap_or_default();
            if ocr_text.trim().is_empty() {
                // OCR 无结果时返回错误，便于上层判断
                Err(AxAgentError::Provider(format!("OCR 未识别出文字，MIME 类型 '{}'", mime_type)))
            } else {
                Ok(ocr_text)
            }
        },

        // DOCX — basic XML extraction without external crate
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            extract_docx(file_path)
        },

        // XLSX — extract cell values from shared strings and sheet XML
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            extract_xlsx(file_path)
        },

        // PPTX — extract text from PowerPoint presentations
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
            extract_pptx(file_path)
        },

        _ => {
            // Try reading as plain text as fallback
            std::fs::read_to_string(file_path).map_err(|e| {
                AxAgentError::execution_with_source(
                    format!("Unsupported MIME type '{}' for {}", mime_type, file_path),
                    e,
                )
            })
        },
    }
}

/// OCR 回退：调用系统 tesseract 命令行工具识别图片中的文字。
///
/// - 默认语言 `eng+chi_sim`（英文 + 简体中文）
/// - 超时 120 秒
/// - tesseract 未安装或调用失败时返回 `Ok(String::new())`（空字符串），
///   不向上层抛错，让 RAG 链路可以正常处理空文本
///
/// 注意：document-parser crate 是同步的，所以这里用 `std::process::Command`
/// 而非 `tokio::process::Command`。超时通过 spawn 子线程 + mpsc channel 实现。
///
/// 返回类型显式写 `std::result::Result` 以避免与 crate 内 import 的
/// `axagent_harness::core_error::Result`（type alias，只接受 1 个泛型）冲突。
pub fn ocr_fallback(path: &Path) -> std::result::Result<String, String> {
    let path_owned = path.to_path_buf();
    let (tx, rx) = mpsc::channel::<std::io::Result<std::process::Output>>();

    let worker = std::thread::spawn(move || {
        let result = std::process::Command::new("tesseract")
            .arg(&path_owned)
            .arg("stdout")
            .arg("-l")
            .arg("eng+chi_sim")
            .output();
        // 忽略发送失败（接收端超时后已 drop）
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(120)) {
        Ok(Ok(output)) => {
            // tesseract 成功返回 —— 取 stdout 文本
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            // 等待工作线程退出，避免泄漏
            let _ = worker.join();
            Ok(text)
        },
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            // tesseract 未安装 —— 静默返回空字符串
            let _ = worker.join();
            Ok(String::new())
        },
        Ok(Err(e)) => {
            // tesseract 调用失败 —— 记录日志并返回空字符串
            tracing::warn!(target: "document-parser", "tesseract 调用失败: {}", e);
            let _ = worker.join();
            Ok(String::new())
        },
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // 超时：尽力 kill 已 spawn 的子进程（通过 drop worker 不能 kill child，
            // 这里只能让 worker 线程在后台自然结束，主流程继续）
            tracing::warn!(target: "document-parser", "OCR 超时 (120s)，跳过");
            Err(format!("OCR 超时 (120 秒): {}", path.display()))
        },
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            // 工作线程 panic 或提前结束 —— 返回空字符串
            let _ = worker.join();
            Ok(String::new())
        },
    }
}

/// 异步版本的文本提取，通过 spawn_blocking 包装同步 I/O，避免阻塞 tokio 运行时。
/// 参数使用 `PathBuf` 和 `String`（owned），因为闭包需要 `'static` 生命周期。
pub async fn extract_text_async(file_path: PathBuf, mime_type: String) -> Result<String> {
    tokio::task::spawn_blocking(move || extract_text(&file_path, &mime_type))
        .await
        .map_err(|e| {
            AxAgentError::execution_with_source(format!("文本提取任务失败: {file_path}"), e)
        })
        .and_then(|r| r)
}

/// Extract text from PDF using pdf-extract crate.
fn extract_pdf(file_path: &Path) -> Result<String> {
    let bytes = std::fs::read(file_path).map_err(|e| {
        AxAgentError::execution_with_source(
            format!("Failed to read PDF file: {}", file_path.display()),
            e,
        )
    })?;

    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| AxAgentError::execution_with_source("Failed to extract PDF text".into(), e))
}

/// Extract text from DOCX by reading the internal XML.
/// DOCX files are ZIP archives containing word/document.xml.
fn extract_docx(file_path: &Path) -> Result<String> {
    let file = std::fs::File::open(file_path).map_err(|e| {
        AxAgentError::execution_with_source(
            format!("Failed to open DOCX file: {}", file_path.display()),
            e,
        )
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        AxAgentError::execution_with_source(
            format!("Failed to read DOCX as ZIP: {}", file_path.display()),
            e,
        )
    })?;

    let mut xml_content = String::new();
    if let Ok(mut entry) = archive.by_name("word/document.xml") {
        use std::io::Read;
        entry.read_to_string(&mut xml_content).map_err(|e| {
            AxAgentError::execution_with_source("Failed to read document.xml".into(), e)
        })?;
    } else {
        return Err(AxAgentError::Provider("DOCX: word/document.xml not found".into()));
    }

    // Simple XML text extraction: find all <w:t> tag contents
    Ok(extract_text_from_xml(&xml_content))
}

/// Simple XML text extraction — pulls text from <w:t> and <w:t xml:space="preserve"> tags.
fn extract_text_from_xml(xml: &str) -> String {
    let mut result = String::new();
    let mut in_paragraph = false;

    for part in xml.split("<w:p") {
        if in_paragraph && !result.is_empty() {
            result.push('\n');
        }
        in_paragraph = true;

        for segment in part.split("<w:t") {
            if let Some(text_start) = segment.find('>') {
                let after_tag = &segment[text_start + 1..];
                if let Some(end) = after_tag.find("</w:t>") {
                    result.push_str(&after_tag[..end]);
                }
            }
        }
    }

    result
}

/// Determine the MIME type from a file extension.
pub fn mime_from_extension(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "txt" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "xml" => "text/xml",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        _ => "text/plain",
    }
}

/// Extract text from XLSX by reading shared strings and sheet XML.
fn extract_xlsx(file_path: &Path) -> Result<String> {
    let file = std::fs::File::open(file_path)
        .map_err(|e| AxAgentError::Provider(format!("Failed to open XLSX file: {e}")))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AxAgentError::Provider(format!("Failed to read XLSX as ZIP: {e}")))?;

    use std::io::Read;

    let mut shared_strings: Vec<String> = Vec::new();
    if let Ok(mut entry) = archive.by_name("xl/sharedStrings.xml") {
        let mut xml = String::new();
        entry.read_to_string(&mut xml).map_err(|e| {
            AxAgentError::Provider(format!("Failed to read sharedStrings.xml: {e}"))
        })?;
        for segment in xml.split("<t") {
            if let Some(text_start) = segment.find('>') {
                let after_tag = &segment[text_start + 1..];
                if let Some(end) = after_tag.find("</t>") {
                    shared_strings.push(after_tag[..end].to_string());
                }
            }
        }
    }

    let mut result = String::new();
    let mut sheet_index = 1;
    loop {
        let sheet_path = format!("xl/worksheets/sheet{}.xml", sheet_index);
        let mut xml = String::new();
        match archive.by_name(&sheet_path) {
            Ok(mut entry) => {
                entry.read_to_string(&mut xml).map_err(|e| {
                    AxAgentError::Provider(format!("Failed to read {}: {e}", sheet_path))
                })?;
            },
            Err(_) => break,
        }

        if sheet_index > 1 {
            result.push_str("\n\n");
        }
        result.push_str(&format!("--- Sheet {} ---\n", sheet_index));

        for row_part in xml.split("<row") {
            let mut row_values: Vec<String> = Vec::new();

            for cell_part in row_part.split("<c") {
                let is_shared_string = cell_part.contains("t=\"s\"");
                let value = if let Some(v_start) = cell_part.find("<v>") {
                    let after_v = &cell_part[v_start + 3..];
                    if let Some(v_end) = after_v.find("</v>") {
                        let v_content = &after_v[..v_end];
                        if is_shared_string {
                            if let Ok(idx) = v_content.parse::<usize>() {
                                if idx < shared_strings.len() {
                                    shared_strings[idx].clone()
                                } else {
                                    v_content.to_string()
                                }
                            } else {
                                v_content.to_string()
                            }
                        } else {
                            v_content.to_string()
                        }
                    } else {
                        continue;
                    }
                } else {
                    let mut inline_str = String::new();
                    if let Some(is_start) = cell_part.find("<is>") {
                        let after_is = &cell_part[is_start..];
                        for seg in after_is.split("<t") {
                            if let Some(t_start) = seg.find('>') {
                                let after_t = &seg[t_start + 1..];
                                if let Some(t_end) = after_t.find("</t>") {
                                    inline_str = after_t[..t_end].to_string();
                                    break;
                                }
                            }
                        }
                    }
                    if inline_str.is_empty() {
                        continue;
                    }
                    inline_str
                };

                if !value.is_empty() {
                    row_values.push(value);
                }
            }

            if !row_values.is_empty() {
                result.push_str(&row_values.join("\t"));
                result.push('\n');
            }
        }

        sheet_index += 1;
    }

    Ok(result)
}

/// Extract text from PPTX (PowerPoint) by reading slide XML files.
fn extract_pptx(file_path: &Path) -> Result<String> {
    let file = std::fs::File::open(file_path)
        .map_err(|e| AxAgentError::Provider(format!("Failed to open PPTX file: {e}")))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AxAgentError::Provider(format!("Failed to read PPTX as ZIP: {e}")))?;

    let mut result = String::new();
    let mut slide_index = 1;

    loop {
        let slide_path = format!("ppt/slides/slide{}.xml", slide_index);
        let mut xml_content = String::new();

        match archive.by_name(&slide_path) {
            Ok(mut entry) => {
                use std::io::Read;
                entry.read_to_string(&mut xml_content).map_err(|e| {
                    AxAgentError::Provider(format!("Failed to read {}: {e}", slide_path))
                })?;

                if !result.is_empty() {
                    result.push_str("\n\n");
                }
                result.push_str(&format!("=== Slide {} ===\n", slide_index));

                let slide_text = extract_text_from_pptx_xml(&xml_content);
                if !slide_text.is_empty() {
                    result.push_str(&slide_text);
                }
            },
            Err(_) => break,
        }

        slide_index += 1;
    }

    if result.is_empty() {
        return Err(AxAgentError::Provider("No slides found in PPTX file".into()));
    }

    Ok(result)
}

fn extract_text_from_pptx_xml(xml: &str) -> String {
    let mut result = String::new();
    let mut current_shape_text = String::new();

    for part in xml.split("<p:sp") {
        if !current_shape_text.is_empty() && !result.is_empty() {
            result.push('\n');
        }
        current_shape_text.clear();

        for segment in part.split("<a:t") {
            if let Some(text_start) = segment.find('>') {
                let after_tag = &segment[text_start + 1..];
                if let Some(end) = after_tag.find("</a:t>") {
                    let text = &after_tag[..end];
                    if !text.is_empty() {
                        current_shape_text.push_str(text);
                    }
                }
            }
        }

        if !current_shape_text.trim().is_empty() {
            if !result.is_empty() && !result.ends_with('\n') {
                result.push(' ');
            }
            result.push_str(current_shape_text.trim());
        }
    }

    result
}

// ── trait 默认实现 ──
pub mod parser_impl;
