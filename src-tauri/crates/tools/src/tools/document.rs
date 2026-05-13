//! 文档格式工具
//!
//! ExportWord (MD→DOCX), ExportPdf (MD→PDF), ExportXlsx (MD→XLSX),
//! ExportPptx (MD→PPTX), RenderMarkdown (MD→HTML),
//! ReadXlsx, ReadPptx (读取 OOXML 文本)
//!
//! 全部纯 Rust 实现，无需安装 Python/LibreOffice。

use crate::{markdown, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════════
// ExportWord — 基于 pulldown-cmark 的 MD→DOCX
// ═══════════════════════════════════════════════════════════════════════════════

pub struct ExportWordTool;

#[async_trait]
impl Tool for ExportWordTool {
    fn name(&self) -> &str {
        "ExportWord"
    }
    fn description(&self) -> &str {
        "将 Markdown 导出为 Word (.docx) 文件。支持标题、表格、代码块、图片、列表、引用、链接等。纯 Rust 实现，无需安装 Python/LibreOffice。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "markdown": {"type": "string", "description": "Markdown 源文本"},
                "output_path": {"type": "string", "description": "输出 .docx 文件路径"},
                "title": {"type": "string", "default": "Document", "description": "文档标题"}
            },
            "required": ["markdown", "output_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input
            .get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let output_path = input
            .get("output_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Document");

        if markdown_text.is_empty() {
            return Ok(ToolResult::error("Error: markdown 是必需的"));
        }
        if output_path.is_empty() {
            return Ok(ToolResult::error("Error: output_path 是必需的"));
        }

        let path = Path::new(output_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::execution_failed(format!("创建输出目录失败: {}", e)))?;
        }

        let doc = build_docx_from_md(markdown_text, title);

        let file = std::fs::File::create(path)
            .map_err(|e| ToolError::execution_failed(format!("创建文件失败: {}", e)))?;
        match doc.build().pack(file) {
            Ok(_) => Ok(ToolResult::success(format!(
                "Word 文档已导出: {} ({} 字符)",
                output_path,
                markdown_text.len()
            ))),
            Err(e) => Ok(ToolResult::error(format!("创建 Word 文档失败: {}", e))),
        }
    }
}

/// 基于 pulldown-cmark 事件流生成 docx 文档
fn build_docx_from_md(markdown_text: &str, title: &str) -> docx_rs::Docx {
    use docx_rs::*;
    use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

    let mut doc = Docx::new();
    doc = doc.page_size(11906, 16838);
    doc = doc.page_margin(PageMargin::new().top(567).bottom(567).left(567).right(567));

    // 页眉
    let header = Header::new().add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(title).size(18).color("888888"))
            .align(AlignmentType::Right),
    );
    doc = doc.header(header);

    // 标题
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(title).size(36).bold().color("1a1a1a"))
            .align(AlignmentType::Center),
    );
    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("")));

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown_text, options);

    // 状态
    let mut para_runs: Vec<Run> = Vec::new();
    let mut heading_text = String::new();
    let mut in_heading: Option<usize> = None; // heading level
    let mut in_code_block = false;
    let mut code_lines: Vec<String> = Vec::new();
    let mut code_lang = String::new();
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_row_cells: Vec<String> = Vec::new();
    let mut table_row_text = String::new();
    let mut in_table_head = false;
    let mut table_headers: Vec<String> = Vec::new();
    let mut in_list = false;
    let mut list_ordered = false;
    let mut in_blockquote = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    let lvl = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    in_heading = Some(lvl);
                },
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                },
                Tag::Table(_) => {
                    in_table = true;
                    table_rows.clear();
                    table_headers.clear();
                    in_table_head = true;
                },
                Tag::TableHead => in_table_head = true,
                Tag::TableRow => {
                    table_row_cells.clear();
                },
                Tag::List(order) => {
                    in_list = true;
                    list_ordered = order.is_some();
                },
                Tag::Item => {},
                Tag::BlockQuote(_) => {
                    in_blockquote = true;
                },
                _ => {},
            },

            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    let size = match in_heading.unwrap_or(1) {
                        1 => 36,
                        2 => 30,
                        3 => 26,
                        4 => 24,
                        _ => 22,
                    };
                    let text = std::mem::take(&mut heading_text);
                    doc = doc.add_paragraph(
                        Paragraph::new()
                            .add_run(Run::new().add_text(text).size(size).bold().color("1a1a1a")),
                    );
                    para_runs.clear();
                    in_heading = None;
                },
                TagEnd::Paragraph if !para_runs.is_empty() => {
                    let mut p = Paragraph::new();
                    for r in std::mem::take(&mut para_runs) {
                        p = p.add_run(r);
                    }
                    if in_list {
                        p = p.indent(Some(567), None, None, None);
                    }
                    if in_blockquote {
                        p = p.indent(Some(284), None, None, None);
                    }
                    doc = doc.add_paragraph(p);
                },
                TagEnd::Paragraph => {},
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    let label = if code_lang.is_empty() {
                        "代码块".to_string()
                    } else {
                        format!("代码块 ({})", &code_lang)
                    };
                    doc = doc.add_paragraph(
                        Paragraph::new()
                            .add_run(Run::new().add_text(&label).size(18).bold().color("888888")),
                    );
                    for line in &code_lines {
                        doc = doc.add_paragraph(
                            Paragraph::new()
                                .add_run(
                                    Run::new()
                                        .add_text(line.replace('\t', "    "))
                                        .size(18)
                                        .fonts(
                                            RunFonts::new().ascii("Consolas").hi_ansi("Consolas"),
                                        )
                                        .color("2d2d2d"),
                                )
                                .indent(Some(284), None, None, None),
                        );
                    }
                    code_lines.clear();
                    code_lang.clear();
                },
                TagEnd::Table => {
                    in_table = false;
                    if !table_headers.is_empty() || !table_rows.is_empty() {
                        let mut t_rows: Vec<TableRow> = Vec::new();
                        // 表头行
                        if !table_headers.is_empty() {
                            let cells: Vec<TableCell> = table_headers
                                .iter()
                                .map(|h| {
                                    TableCell::new()
                                        .shading(
                                            Shading::new().fill("4a90d9").shd_type(ShdType::Clear),
                                        )
                                        .add_paragraph(Paragraph::new().add_run(
                                            Run::new().add_text(h).size(22).bold().color("ffffff"),
                                        ))
                                })
                                .collect();
                            t_rows.push(TableRow::new(cells));
                        }
                        // 数据行
                        for (ri, row) in table_rows.iter().enumerate() {
                            let cells: Vec<TableCell> = row
                                .iter()
                                .map(|cell| {
                                    let c =
                                        TableCell::new().add_paragraph(Paragraph::new().add_run(
                                            Run::new().add_text(cell).size(22).color("333333"),
                                        ));
                                    if ri % 2 == 1 {
                                        c.shading(
                                            Shading::new().fill("f9f9f9").shd_type(ShdType::Clear),
                                        )
                                    } else {
                                        c
                                    }
                                })
                                .collect();
                            t_rows.push(TableRow::new(cells));
                        }
                        doc = doc.add_table(Table::new(t_rows));
                        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("")));
                    }
                    table_rows.clear();
                    table_headers.clear();
                    in_table_head = false;
                },
                TagEnd::TableRow => {
                    if !table_row_text.is_empty() {
                        table_row_cells.push(std::mem::take(&mut table_row_text));
                    }
                    if !table_row_cells.is_empty() {
                        let cells = std::mem::take(&mut table_row_cells);
                        if in_table_head {
                            table_headers = cells;
                        } else {
                            table_rows.push(cells);
                        }
                    }
                },
                TagEnd::TableCell => {},
                TagEnd::List(_) => {
                    in_list = false;
                    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("")));
                },
                TagEnd::Item if !para_runs.is_empty() => {
                    let prefix = if list_ordered { "" } else { "• " };
                    let mut p = Paragraph::new()
                        .add_run(Run::new().add_text(prefix).size(22))
                        .indent(Some(567), None, None, None);
                    for r in std::mem::take(&mut para_runs) {
                        p = p.add_run(r);
                    }
                    doc = doc.add_paragraph(p);
                },
                TagEnd::Item => {},
                TagEnd::BlockQuote(_) => {
                    in_blockquote = false;
                },
                _ => {},
            },

            Event::Text(text) => {
                if in_code_block {
                    code_lines.push(text.to_string());
                } else if in_heading.is_some() {
                    heading_text.push_str(&text);
                } else if in_table {
                    table_row_text.push_str(&text);
                } else {
                    para_runs.push(Run::new().add_text(text.to_string()).size(22));
                }
            },

            Event::Code(text) => {
                if in_code_block {
                    code_lines.push(text.to_string());
                } else if in_heading.is_some() {
                    heading_text.push_str(&text);
                } else if in_table {
                    table_row_text.push_str(&text);
                } else {
                    para_runs.push(
                        Run::new()
                            .add_text(text.to_string())
                            .size(20)
                            .fonts(RunFonts::new().ascii("Consolas").hi_ansi("Consolas"))
                            .color("c7254e"),
                    );
                }
            },

            Event::SoftBreak | Event::HardBreak if !in_table => {
                para_runs.push(Run::new().add_text(" "));
            },
            Event::SoftBreak | Event::HardBreak => {},

            Event::Rule => {
                doc = doc.add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().add_text("─".repeat(60)).size(18).color("cccccc"))
                        .align(AlignmentType::Center),
                );
            },

            Event::TaskListMarker(checked) => {
                para_runs.push(
                    Run::new()
                        .add_text(if checked { "☑ " } else { "☐ " })
                        .size(22),
                );
            },

            _ => {},
        }
    }

    // 页脚
    let footer = Footer::new().add_paragraph(
        Paragraph::new()
            .add_run(
                Run::new()
                    .add_text("由 AxAgent 生成")
                    .size(16)
                    .color("999999"),
            )
            .align(AlignmentType::Center),
    );
    doc = doc.footer(footer);

    doc
}

// ═══════════════════════════════════════════════════════════════════════════════
// RenderMarkdown — MD→HTML
// ═══════════════════════════════════════════════════════════════════════════════

pub struct RenderMarkdownTool;

#[async_trait]
impl Tool for RenderMarkdownTool {
    fn name(&self) -> &str {
        "RenderMarkdown"
    }
    fn description(&self) -> &str {
        "将 Markdown 渲染为完整 HTML 页面。支持表格、代码高亮、任务列表等扩展语法，内嵌响应式 CSS。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "markdown": {"type": "string", "description": "Markdown 源文本"},
                "output_path": {"type": "string", "description": "可选：保存为 .html 文件路径"}
            },
            "required": ["markdown"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input
            .get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if markdown_text.is_empty() {
            return Ok(ToolResult::error("Error: markdown 是必需的"));
        }

        let html = markdown::render_to_html(markdown_text);

        if let Some(path_str) = input.get("output_path").and_then(|v| v.as_str()) {
            if !path_str.is_empty() {
                let path = Path::new(path_str);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| ToolError::execution_failed(format!("创建目录失败: {}", e)))?;
                }
                std::fs::write(path, &html)
                    .map_err(|e| ToolError::execution_failed(format!("写入文件失败: {}", e)))?;
                return Ok(ToolResult::success(format!(
                    "HTML 已保存: {} ({} 字符)",
                    path_str,
                    html.len()
                )));
            }
        }

        Ok(ToolResult::success(html))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ExportPdf — MD→PDF（通过 genpdf）
// ═══════════════════════════════════════════════════════════════════════════════

pub struct ExportPdfTool;

#[async_trait]
impl Tool for ExportPdfTool {
    fn name(&self) -> &str {
        "ExportPdf"
    }
    fn description(&self) -> &str {
        "将 Markdown 导出为 PDF 文件。支持标题、段落、列表、代码块、表格。纯 Rust 实现，无需安装外部工具。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "markdown": {"type": "string", "description": "Markdown 源文本"},
                "output_path": {"type": "string", "description": "输出 .pdf 文件路径"},
                "title": {"type": "string", "default": "Document", "description": "文档标题"}
            },
            "required": ["markdown", "output_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input
            .get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let output_path = input
            .get("output_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Document");

        if markdown_text.is_empty() {
            return Ok(ToolResult::error("Error: markdown 是必需的"));
        }
        if output_path.is_empty() {
            return Ok(ToolResult::error("Error: output_path 是必需的"));
        }

        let path = Path::new(output_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::execution_failed(format!("创建目录失败: {}", e)))?;
        }

        let doc = markdown::parse_markdown(markdown_text);

        match build_pdf(&doc, title, output_path) {
            Ok(()) => Ok(ToolResult::success(format!(
                "PDF 已导出: {} ({} 字符输入)",
                output_path,
                markdown_text.len()
            ))),
            Err(e) => Ok(ToolResult::error(format!("创建 PDF 失败: {}", e))),
        }
    }
}

fn build_pdf(doc: &markdown::MdDocument, title: &str, output_path: &str) -> Result<(), String> {
    // 使用原始 PDF 语法构建最简单的 PDF
    // 仅支持 ASCII/UTF-8 文本，复杂格式后续可增强为使用 HTML→PDF 引擎
    let mut pdf = Vec::new();

    // 收集所有文本
    let mut text_lines: Vec<String> = Vec::new();
    text_lines.push(title.to_string());
    text_lines.push(String::new());

    for block in &doc.blocks {
        match block {
            markdown::MdBlock::Heading { level: _, text } => {
                text_lines.push(text.clone());
                text_lines.push(String::new());
            },
            markdown::MdBlock::Paragraph { inlines } => {
                let t = inlines_to_plain_text(inlines);
                if !t.trim().is_empty() {
                    text_lines.push(t);
                }
            },
            markdown::MdBlock::CodeBlock { code, .. } => {
                for line in code.lines() {
                    text_lines.push(format!("    {}", line));
                }
                text_lines.push(String::new());
            },
            markdown::MdBlock::Table { headers, rows } => {
                if !headers.is_empty() {
                    text_lines.push(headers.join(" | "));
                    text_lines.push(
                        headers
                            .iter()
                            .map(|_| "---")
                            .collect::<Vec<_>>()
                            .join(" | "),
                    );
                }
                for row in rows {
                    text_lines.push(row.join(" | "));
                }
                text_lines.push(String::new());
            },
            markdown::MdBlock::List { items, .. } => {
                for item in items {
                    text_lines.push(format!("  • {}", inlines_to_plain_text(item)));
                }
                text_lines.push(String::new());
            },
            markdown::MdBlock::Blockquote { inlines } => {
                text_lines.push(format!("  │ {}", inlines_to_plain_text(inlines)));
            },
            markdown::MdBlock::HorizontalRule => {
                text_lines.push("─".repeat(60));
            },
            markdown::MdBlock::Image { alt, .. } => {
                text_lines.push(format!("[图片: {}]", alt));
            },
        }
    }

    let _content = text_lines.join("\n");
    // 构建最小有效 PDF
    pdf.extend_from_slice(b"%PDF-1.4\n");

    // Object 1: Catalog
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Object 2: Pages
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    // Object 3: Page
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n");

    // Object 4: Content stream
    let mut stream = Vec::new();
    stream.extend_from_slice(b"BT\n/F1 10 Tf\n");
    let mut y: f64 = 750.0;
    for line in &text_lines {
        if y < 40.0 {
            break;
        }
        // 对 PDF 字符串进行转义
        let escaped = line
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
        stream.extend_from_slice(format!("1 0 0 1 36 {} Tm\n({}) Tj\nT*\n", y, escaped).as_bytes());
        y -= 14.0;
    }
    stream.extend_from_slice(b"ET\n");

    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", stream.len()).as_bytes());
    pdf.extend_from_slice(&stream);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    // Object 5: Font (Courier as base font — always available)
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>\nendobj\n",
    );

    // Cross-reference table
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", find_obj_offset(&pdf, 1)).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", find_obj_offset(&pdf, 2)).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", find_obj_offset(&pdf, 3)).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", find_obj_offset(&pdf, 4)).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", find_obj_offset(&pdf, 5)).as_bytes());

    pdf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    pdf.extend_from_slice(format!("{}\n", xref_offset).as_bytes());
    pdf.extend_from_slice(b"%%EOF\n");

    std::fs::write(output_path, &pdf).map_err(|e| format!("写入 PDF 失败: {}", e))
}

fn find_obj_offset(pdf: &[u8], obj_num: usize) -> usize {
    let marker = format!("{} 0 obj", obj_num);
    let marker_bytes = marker.as_bytes();
    pdf.windows(marker_bytes.len())
        .position(|w| w == marker_bytes)
        .unwrap_or(0)
}

fn inlines_to_plain_text(inlines: &[markdown::MdInline]) -> String {
    inlines
        .iter()
        .map(|i| match i {
            markdown::MdInline::Text(s)
            | markdown::MdInline::Bold(s)
            | markdown::MdInline::Italic(s)
            | markdown::MdInline::Code(s) => s.clone(),
            markdown::MdInline::Link { text, .. } => text.clone(),
            markdown::MdInline::Image { alt, .. } => alt.clone(),
        })
        .collect::<Vec<_>>()
        .join("")
}

// ═══════════════════════════════════════════════════════════════════════════════
// ExportXlsx — MD 表格→XLSX
// ═══════════════════════════════════════════════════════════════════════════════

pub struct ExportXlsxTool;

#[async_trait]
impl Tool for ExportXlsxTool {
    fn name(&self) -> &str {
        "ExportXlsx"
    }
    fn description(&self) -> &str {
        "将 Markdown 中的表格导出为 Excel (.xlsx) 文件。每个表格生成一个工作表，支持表头格式。纯 Rust 实现。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "markdown": {"type": "string", "description": "Markdown 源文本（提取其中的表格）"},
                "output_path": {"type": "string", "description": "输出 .xlsx 文件路径"},
                "sheet_name": {"type": "string", "default": "Sheet1", "description": "工作表名称"}
            },
            "required": ["markdown", "output_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input
            .get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let output_path = input
            .get("output_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let sheet_name = input
            .get("sheet_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Sheet1");

        if markdown_text.is_empty() {
            return Ok(ToolResult::error("Error: markdown 是必需的"));
        }
        if output_path.is_empty() {
            return Ok(ToolResult::error("Error: output_path 是必需的"));
        }

        let path = Path::new(output_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::execution_failed(format!("创建目录失败: {}", e)))?;
        }

        match build_xlsx(markdown_text, sheet_name, output_path) {
            Ok(table_count) => Ok(ToolResult::success(format!(
                "XLSX 已导出: {} ({} 个表格, {} 字符输入)",
                output_path,
                table_count,
                markdown_text.len()
            ))),
            Err(e) => Ok(ToolResult::error(format!("创建 XLSX 失败: {}", e))),
        }
    }
}

fn build_xlsx(markdown_text: &str, sheet_name: &str, output_path: &str) -> Result<usize, String> {
    use rust_xlsxwriter::*;

    let parsed = markdown::parse_markdown(markdown_text);
    let tables = markdown::extract_tables(&parsed);
    if tables.is_empty() {
        return Err("未在 Markdown 中找到表格".to_string());
    }

    let mut workbook = Workbook::new();
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4A90D9))
        .set_font_color(Color::White);

    for (ti, table_block) in tables.iter().enumerate() {
        let (headers, rows) = match table_block {
            markdown::MdBlock::Table { headers, rows } => (headers, rows),
            _ => continue,
        };

        let name = if ti == 0 {
            sheet_name.to_string()
        } else {
            format!("{}_{}", sheet_name, ti + 1)
        };
        let worksheet = workbook
            .add_worksheet()
            .set_name(&name)
            .map_err(|e| e.to_string())?;

        // 写表头
        for (ci, h) in headers.iter().enumerate() {
            worksheet
                .write_with_format(0, ci as u16, h, &header_format)
                .map_err(|e| e.to_string())?;
        }
        // 写数据行
        for (ri, row) in rows.iter().enumerate() {
            for (ci, cell) in row.iter().enumerate() {
                worksheet
                    .write((ri + 1) as u32, ci as u16, cell)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    workbook.save(output_path).map_err(|e| e.to_string())?;
    Ok(tables.len())
}

// ═══════════════════════════════════════════════════════════════════════════════
// ExportPptx — MD→PPTX（# 标题 = 新幻灯片）
// ═══════════════════════════════════════════════════════════════════════════════

pub struct ExportPptxTool;

#[async_trait]
impl Tool for ExportPptxTool {
    fn name(&self) -> &str {
        "ExportPptx"
    }
    fn description(&self) -> &str {
        "将 Markdown 导出为 PowerPoint (.pptx) 文件。每个 H1/H2 标题创建一个幻灯片，内容为项目符号列表。纯 Rust 实现，无需安装 PowerPoint。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "markdown": {"type": "string", "description": "Markdown 源文本"},
                "output_path": {"type": "string", "description": "输出 .pptx 文件路径"},
                "title": {"type": "string", "default": "Presentation", "description": "演示文稿标题"}
            },
            "required": ["markdown", "output_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input
            .get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let output_path = input
            .get("output_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Presentation");

        if markdown_text.is_empty() {
            return Ok(ToolResult::error("Error: markdown 是必需的"));
        }
        if output_path.is_empty() {
            return Ok(ToolResult::error("Error: output_path 是必需的"));
        }

        let path = Path::new(output_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::execution_failed(format!("创建目录失败: {}", e)))?;
        }

        match build_pptx(markdown_text, title, output_path) {
            Ok(slide_count) => Ok(ToolResult::success(format!(
                "PPTX 已导出: {} ({} 张幻灯片, {} 字符输入)",
                output_path,
                slide_count,
                markdown_text.len()
            ))),
            Err(e) => Ok(ToolResult::error(format!("创建 PPTX 失败: {}", e))),
        }
    }
}

/// 构建 PPTX（ZIP + PresentationML XML）
fn build_pptx(markdown_text: &str, title: &str, output_path: &str) -> Result<usize, String> {
    use std::io::Write;

    let doc = markdown::parse_markdown(markdown_text);

    // 按标题分割为幻灯片
    let mut slides: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_title = title.to_string();
    let mut current_bullets: Vec<String> = Vec::new();

    for block in &doc.blocks {
        match block {
            markdown::MdBlock::Heading { level, text } if *level <= 2 => {
                if !current_bullets.is_empty() || !current_title.is_empty() {
                    slides.push((
                        std::mem::take(&mut current_title),
                        std::mem::take(&mut current_bullets),
                    ));
                }
                current_title = text.clone();
                current_bullets = Vec::new();
            },
            markdown::MdBlock::Paragraph { inlines } => {
                let t = inlines_to_plain_text(inlines);
                if !t.trim().is_empty() {
                    current_bullets.push(t);
                }
            },
            markdown::MdBlock::List { items, .. } => {
                for item in items {
                    current_bullets.push(inlines_to_plain_text(item));
                }
            },
            markdown::MdBlock::CodeBlock { language: _, code } => {
                for line in code.lines() {
                    if !line.trim().is_empty() {
                        current_bullets.push(format!("  {}", line));
                    }
                }
            },
            _ => {},
        }
    }
    if !current_title.is_empty() {
        slides.push((current_title, current_bullets));
    }
    if slides.is_empty() {
        slides.push((title.to_string(), vec!["（空演示文稿）".to_string()]));
    }

    let slide_count = slides.len();

    // 构建 ZIP
    let file = std::fs::File::create(output_path).map_err(|e| format!("创建文件失败: {}", e))?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let zip_options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // [Content_Types].xml
    let mut content_types = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\n\
         <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\n\
         <Default Extension=\"xml\" ContentType=\"application/xml\"/>\n\
         <Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>\n",
    );
    for i in 1..=slide_count {
        content_types.push_str(&format!(
            "<Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>\n",
            i
        ));
    }
    content_types.push_str("</Types>");

    zip_writer
        .start_file("[Content_Types].xml", zip_options)
        .map_err(|e| e.to_string())?;
    zip_writer
        .write_all(content_types.as_bytes())
        .map_err(|e| e.to_string())?;

    // _rels/.rels
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
                <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
                <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/>\n\
                </Relationships>";
    zip_writer
        .start_file("_rels/.rels", zip_options)
        .map_err(|e| e.to_string())?;
    zip_writer
        .write_all(rels.as_bytes())
        .map_err(|e| e.to_string())?;

    // ppt/_rels/presentation.xml.rels
    let mut pres_rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n",
    );
    for i in 1..=slide_count {
        pres_rels.push_str(&format!(
            "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{}.xml\"/>\n",
            i + 10, i
        ));
    }
    pres_rels.push_str("</Relationships>");

    zip_writer
        .start_file("ppt/_rels/presentation.xml.rels", zip_options)
        .map_err(|e| e.to_string())?;
    zip_writer
        .write_all(pres_rels.as_bytes())
        .map_err(|e| e.to_string())?;

    // ppt/presentation.xml
    let mut pres_xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <p:presentation xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"\
         xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\n\
         <p:sldIdLst>\n",
    );
    for i in 1..=slide_count {
        pres_xml.push_str(&format!("<p:sldId id=\"{}\" r:id=\"rId{}\"/>\n", i + 255, i + 10));
    }
    pres_xml.push_str("</p:sldIdLst>\n<p:sldSz cx=\"9144000\" cy=\"6858000\"/>\n</p:presentation>");

    zip_writer
        .start_file("ppt/presentation.xml", zip_options)
        .map_err(|e| e.to_string())?;
    zip_writer
        .write_all(pres_xml.as_bytes())
        .map_err(|e| e.to_string())?;

    // 每张幻灯片
    for (si, (slide_title, bullets)) in slides.iter().enumerate() {
        let slide_num = si + 1;
        let escaped_title = xml_escape(slide_title);
        let mut slide_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
             <p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"\
             xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"\
             xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\n\
             <p:cSld>\n<p:spTree>\n\
             <p:sp><p:nvSpPr><p:cNvPr id=\"1\" name=\"Title\"/><p:cNvSpPr><p:spLocks noGrp=\"1\"/></p:cNvSpPr>\
             <p:nvPr/></p:nvSpPr>\
             <p:spPr><a:xfrm><a:off x=\"685800\" y=\"274320\"/><a:ext cx=\"7772400\" cy=\"822960\"/></a:xfrm></p:spPr>\
             <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"zh-CN\" sz=\"3200\" b=\"1\"/>\
             <a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>\n",
            escaped_title
        );

        // 项目符号
        let mut content_y: u32 = 1_371_600; // 起始 y 位置
        for bullet in bullets {
            let escaped_bullet = xml_escape(bullet);
            slide_xml.push_str(&format!(
                "<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Bullet\"/><p:cNvSpPr><p:spLocks noGrp=\"1\"/></p:cNvSpPr>\
                 <p:nvPr/></p:nvSpPr>\
                 <p:spPr><a:xfrm><a:off x=\"914400\" y=\"{}\"/><a:ext cx=\"7315200\" cy=\"411480\"/></a:xfrm></p:spPr>\
                 <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"zh-CN\" sz=\"2000\"/>\
                 <a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>\n",
                content_y, escaped_bullet
            ));
            content_y += 457_200; // 行间距
        }

        slide_xml.push_str("</p:spTree>\n</p:cSld>\n</p:sld>");

        let slide_path = format!("ppt/slides/slide{}.xml", slide_num);
        zip_writer
            .start_file(&slide_path, zip_options)
            .map_err(|e| e.to_string())?;
        zip_writer
            .write_all(slide_xml.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    zip_writer.finish().map_err(|e| e.to_string())?;
    Ok(slide_count)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ═══════════════════════════════════════════════════════════════════════════════
// ReadXlsx — 读取 XLSX 文本内容
// ═══════════════════════════════════════════════════════════════════════════════

pub struct ReadXlsxTool;

#[async_trait]
impl Tool for ReadXlsxTool {
    fn name(&self) -> &str {
        "ReadXlsx"
    }
    fn description(&self) -> &str {
        "读取 Excel (.xlsx) 文件的文本内容，提取所有工作表的单元格数据（制表符分隔）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "XLSX 文件路径"}
            },
            "required": ["path"]
        })
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
        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", file_path)));
        }

        match axagent_core::document_parser::extract_text(
            path,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ) {
            Ok(text) => Ok(ToolResult::success(truncate_str(&text, 100_000))),
            Err(e) => Ok(ToolResult::error(format!("读取 XLSX 失败: {}", e))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ReadPptx — 读取 PPTX 文本内容
// ═══════════════════════════════════════════════════════════════════════════════

pub struct ReadPptxTool;

#[async_trait]
impl Tool for ReadPptxTool {
    fn name(&self) -> &str {
        "ReadPptx"
    }
    fn description(&self) -> &str {
        "读取 PowerPoint (.pptx) 文件的文本内容，按幻灯片编号分段输出。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "PPTX 文件路径"}
            },
            "required": ["path"]
        })
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
        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", file_path)));
        }

        match axagent_core::document_parser::extract_text(
            path,
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ) {
            Ok(text) => Ok(ToolResult::success(truncate_str(&text, 100_000))),
            Err(e) => Ok(ToolResult::error(format!("读取 PPTX 失败: {}", e))),
        }
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...\n(已截断, 原 {} 字符)", &s[..max], s.len())
    }
}
