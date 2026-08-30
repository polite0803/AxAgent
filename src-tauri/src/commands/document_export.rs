// SPDX-License-Identifier: AGPL-3.0-only

//! 文档导出命令：将 Markdown 内容导出为 Word / PDF 文件。
//!
//! 复用 `axagent-tools` crate 已有的 MD→DOCX / MD→PDF 纯 Rust 转换能力，
//! 暴露为 Tauri 命令供前端消息气泡 footer 的"保存为"按钮调用。

use axagent_tools::markdown::parse_markdown;
use axagent_tools::tools::document::{build_docx_from_md, build_pdf};
use serde::Deserialize;

/// 支持的导出格式
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentExportFormat {
    Docx,
    Pdf,
}

/// 将 Markdown 内容导出为指定格式的文档文件。
///
/// 前端流程：先通过 save dialog 让用户选保存路径（或用默认路径），
/// 再调用本命令完成实际的 MD→目标格式转换。
///
/// # 参数（Tauri v2 自动 rename_all=camelCase）
/// - markdown: String — Markdown 源文本
/// - output_path: String — 输出文件绝对路径
/// - format: "docx" | "pdf"
/// - title: Option<String> — 文档标题（默认 "Document"）
#[tauri::command]
pub async fn export_content(
    markdown: String,
    output_path: String,
    format: String,
    title: Option<String>,
) -> Result<bool, String> {
    if markdown.trim().is_empty() {
        return Err(String::from("内容为空"));
    }
    if output_path.trim().is_empty() {
        return Err(String::from("输出路径为空"));
    }

    let fmt: DocumentExportFormat = match format.to_lowercase().as_str() {
        "docx" | "word" => DocumentExportFormat::Docx,
        "pdf" => DocumentExportFormat::Pdf,
        other => {
            return Err(format!("不支持的格式: {}，仅支持 docx / pdf", other));
        },
    };
    let doc_title = title.unwrap_or_else(|| String::from("Document"));

    // 确保父目录存在
    let path = std::path::Path::new(&output_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
        }
    }

    match fmt {
        DocumentExportFormat::Docx => {
            let doc = build_docx_from_md(&markdown, &doc_title, None, None);
            let file =
                std::fs::File::create(path).map_err(|e| format!("创建 DOCX 文件失败: {}", e))?;
            doc.build().pack(file).map_err(|e| format!("生成 DOCX 文档失败: {}", e))?;
        },
        DocumentExportFormat::Pdf => {
            let md_doc = parse_markdown(&markdown);
            build_pdf(
                &md_doc,
                &doc_title,
                "", // subtitle
                "", // author
                &output_path,
                540.0,
                "center",
                "",
                None,  // cover_template
                None,  // header_template
                None,  // footer_template
                false, // enable_toc
            )
            .map_err(|e| format!("生成 PDF 文档失败: {}", e))?;
        },
    }

    Ok(true)
}
