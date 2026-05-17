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

/// 基于 pulldown-cmark 事件流生成专业 docx 文档
///
/// 使用 docx-rs 样式系统、表格边框、列表编号、页码、行内格式等完整能力。
fn build_docx_from_md(markdown_text: &str, title: &str) -> docx_rs::Docx {
    use docx_rs::*;
    use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

    let mut doc = Docx::new();

    // ── 页面设置 ──
    doc = doc.page_size(11906, 16838); // A4
    doc = doc.page_margin(
        PageMargin::new()
            .top(1440)    // 1 inch
            .bottom(1440)
            .left(1440)
            .right(1440),
    );

    // ── 文档样式 ──
    let default_font = RunFonts::new()
        .ascii("Calibri")
        .hi_ansi("Calibri")
        .east_asia("微软雅黑")
        .cs("Calibri");

    let heading_font = RunFonts::new()
        .ascii("Calibri Light")
        .hi_ansi("Calibri Light")
        .east_asia("微软雅黑");

    let styles = Styles::new()
        .default_fonts(default_font)
        .default_size(22) // 11pt
        .default_line_spacing(
            LineSpacing::new()
                .line_rule(LineSpacingType::Auto)
                .line(276)  // 1.15x
                .after(120), // 段后 6pt
        )
        .add_style(
            Style::new("Heading1", StyleType::Paragraph)
                .name("Heading 1")
                .based_on("Normal")
                .q_format(true)
                .size(32)
                .bold()
                .color("1F3864")
                .fonts(heading_font.clone())
                .line_spacing(
                    LineSpacing::new()
                        .before(240)
                        .after(120)
                        .line_rule(LineSpacingType::Auto)
                        .line(276),
                )
                
                .outline_lvl(0),
        )
        .add_style(
            Style::new("Heading2", StyleType::Paragraph)
                .name("Heading 2")
                .based_on("Normal")
                .q_format(true)
                .size(26)
                .bold()
                .color("2E75B6")
                .fonts(heading_font.clone())
                .line_spacing(
                    LineSpacing::new()
                        .before(200)
                        .after(80)
                        .line_rule(LineSpacingType::Auto)
                        .line(276),
                )
                
                .outline_lvl(1),
        )
        .add_style(
            Style::new("Heading3", StyleType::Paragraph)
                .name("Heading 3")
                .based_on("Normal")
                .q_format(true)
                .size(24)
                .bold()
                .color("2E75B6")
                .fonts(heading_font)
                .line_spacing(
                    LineSpacing::new()
                        .before(160)
                        .after(60)
                        .line_rule(LineSpacingType::Auto)
                        .line(276),
                )
                
                .outline_lvl(2),
        );
    doc = doc.styles(styles);

    // ── 列表编号定义 ──
    let abs_ordered = AbstractNumbering::new(1).add_level(
        Level::new(
            0,
            Start::new(1),
            NumberFormat::new("decimal"),
            LevelText::new("%1."),
            LevelJc::new("left"),
        )
        .indent(Some(567), Some(SpecialIndentType::Hanging(284)), None, None)
        .size(22),
    );
    let num_ordered = Numbering::new(10, 1);

    let abs_bullet = AbstractNumbering::new(2).add_level(
        Level::new(
            0,
            Start::new(1),
            NumberFormat::new("bullet"),
            LevelText::new("\u{2022}"),
            LevelJc::new("left"),
        )
        .indent(Some(567), Some(SpecialIndentType::Hanging(284)), None, None)
        .size(22),
    );
    let num_bullet = Numbering::new(20, 2);
    doc = doc
        .add_abstract_numbering(abs_ordered)
        .add_numbering(num_ordered)
        .add_abstract_numbering(abs_bullet)
        .add_numbering(num_bullet);

    // ── 页眉 ──
    let header = Header::new().add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(title).size(18).color("888888"))
            .align(AlignmentType::Right),
    );
    doc = doc.header(header);

    // ── 封面标题 ──
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(title).size(36).bold().color("1a1a1a"))
            .align(AlignmentType::Center)
            .line_spacing(
                LineSpacing::new()
                    .after(240)
                    .line_rule(LineSpacingType::Auto)
                    .line(276),
            ),
    );

    // ── 解析器设置 ──
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown_text, options);

    // ── 状态变量 ──
    let mut para_runs: Vec<Run> = Vec::new();
    let mut heading_text = String::new();
    let mut in_heading: Option<usize> = None;
    let mut in_code_block = false;
    let mut code_lines: Vec<String> = Vec::new();
    let mut code_lang = String::new();
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_row_cells: Vec<String> = Vec::new();
    let mut table_row_text = String::new();
    let mut in_table_head = false;
    let mut table_headers: Vec<String> = Vec::new();
    let mut list_ordered = false;
    let mut in_blockquote = false;
    // 行内格式跟踪
    let mut bold_depth: u32 = 0;
    let mut italic_depth: u32 = 0;
    let mut strike_depth: u32 = 0;
    let mut link_url: Option<String> = None;
    // 累积文本（延迟到获取格式信息后创建 Run）
    let mut text_buf: String = String::new();

    // 辅助：刷新 text_buf 为 Run 并推入 para_runs
    let flush_text = |para_runs: &mut Vec<Run>,
                      text_buf: &mut String,
                      in_heading: &mut Option<usize>,
                      heading_text: &mut String,
                      in_table: bool,
                      table_row_text: &mut String,
                      bold_depth: u32,
                      italic_depth: u32,
                      strike_depth: u32,
                      link_url: &Option<String>,
                      in_code_block: bool| {
        if text_buf.is_empty() {
            return;
        }
        let text = std::mem::take(text_buf);
        if in_code_block {
            return; // handled separately
        }
        if in_heading.is_some() {
            heading_text.push_str(&text);
            return;
        }
        if in_table {
            table_row_text.push_str(&text);
            return;
        }
        let mut run = Run::new().add_text(text).size(22);
        if bold_depth > 0 {
            run = run.bold();
        }
        if italic_depth > 0 {
            run = run.italic();
        }
        if strike_depth > 0 {
            run = run.strike();
        }
        if let Some(_url) = link_url {
            run = run.color("0563C1").underline("single");
        }
        para_runs.push(run);
    };

    for event in parser {
        match event {
            // ── Start Tags ──
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
                    flush_text(
                        &mut para_runs,
                        &mut text_buf,
                        &mut in_heading,
                        &mut heading_text,
                        in_table,
                        &mut table_row_text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &link_url,
                        in_code_block,
                    );
                    in_heading = Some(lvl);
                },
                Tag::Strong => bold_depth += 1,
                Tag::Emphasis => italic_depth += 1,
                Tag::Strikethrough => strike_depth += 1,
                Tag::Link { dest_url, .. } => {
                    flush_text(
                        &mut para_runs,
                        &mut text_buf,
                        &mut in_heading,
                        &mut heading_text,
                        in_table,
                        &mut table_row_text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &link_url,
                        in_code_block,
                    );
                    link_url = Some(dest_url.to_string());
                },
                Tag::Image {
                    dest_url, title, ..
                } => {
                    flush_text(
                        &mut para_runs,
                        &mut text_buf,
                        &mut in_heading,
                        &mut heading_text,
                        in_table,
                        &mut table_row_text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &link_url,
                        in_code_block,
                    );
                    // 刷新当前段落
                    if !para_runs.is_empty() {
                        let p = Paragraph::new();
                        let p = add_runs_to_para(p, std::mem::take(&mut para_runs), in_blockquote);
                        doc = doc.add_paragraph(p);
                    }
                    doc = embed_image(doc, &title, &dest_url);
                },
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                },
                Tag::Table(_) => {
                    flush_text(
                        &mut para_runs,
                        &mut text_buf,
                        &mut in_heading,
                        &mut heading_text,
                        in_table,
                        &mut table_row_text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &link_url,
                        in_code_block,
                    );
                    // 刷新当前段落
                    if !para_runs.is_empty() {
                        let p = Paragraph::new();
                        let p = add_runs_to_para(p, std::mem::take(&mut para_runs), in_blockquote);
                        doc = doc.add_paragraph(p);
                    }
                    in_table = true;
                    table_rows.clear();
                    table_headers.clear();
                    in_table_head = true;
                },
                Tag::TableHead => in_table_head = true,
                Tag::TableRow => table_row_cells.clear(),
                Tag::List(order) => {
                    flush_text(
                        &mut para_runs,
                        &mut text_buf,
                        &mut in_heading,
                        &mut heading_text,
                        in_table,
                        &mut table_row_text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &link_url,
                        in_code_block,
                    );
                    list_ordered = order.is_some();
                },
                Tag::Item => {},
                Tag::BlockQuote(_) => in_blockquote = true,
                _ => {},
            },

            // ── End Tags ──
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    let heading_style = match in_heading.unwrap_or(1) {
                        1 => "Heading1",
                        2 => "Heading2",
                        _ => "Heading3",
                    };
                    let text = std::mem::take(&mut heading_text);
                    doc = doc.add_paragraph(
                        Paragraph::new()
                            .add_run(Run::new().add_text(text))
                            .style(heading_style),
                    );
                    para_runs.clear();
                    in_heading = None;
                },
                TagEnd::Strong => bold_depth = bold_depth.saturating_sub(1),
                TagEnd::Emphasis => italic_depth = italic_depth.saturating_sub(1),
                TagEnd::Strikethrough => strike_depth = strike_depth.saturating_sub(1),
                TagEnd::Link => {
                    flush_text(
                        &mut para_runs,
                        &mut text_buf,
                        &mut in_heading,
                        &mut heading_text,
                        in_table,
                        &mut table_row_text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &link_url,
                        in_code_block,
                    );
                    link_url = None;
                },
                TagEnd::Paragraph if !para_runs.is_empty() => {
                    flush_text(
                        &mut para_runs,
                        &mut text_buf,
                        &mut in_heading,
                        &mut heading_text,
                        in_table,
                        &mut table_row_text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &link_url,
                        in_code_block,
                    );
                    let p = Paragraph::new();
                    let p = add_runs_to_para(p, std::mem::take(&mut para_runs), in_blockquote);
                    if in_blockquote {
                        doc = doc.add_paragraph(
                            p.indent(Some(284), None, None, None).line_spacing(
                                LineSpacing::new()
                                    .line_rule(LineSpacingType::Auto)
                                    .line(276),
                            ),
                        );
                    } else {
                        doc = doc.add_paragraph(p);
                    }
                },
                TagEnd::Paragraph => {},
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    if !code_lines.is_empty() {
                        let label = if code_lang.is_empty() {
                            "代码".to_string()
                        } else {
                            format!("{}", &code_lang)
                        };
                        doc = doc.add_paragraph(
                            Paragraph::new()
                                .add_run(
                                    Run::new().add_text(&label).size(16).bold().color("888888"),
                                )
                                .indent(Some(284), None, None, None),
                        );
                        for line in &code_lines {
                            doc = doc.add_paragraph(
                                Paragraph::new()
                                    .add_run(
                                        Run::new()
                                            .add_text(line.replace('\t', "    "))
                                            .size(18)
                                            .fonts(
                                                RunFonts::new()
                                                    .ascii("Consolas")
                                                    .hi_ansi("Consolas"),
                                            )
                                            .color("2D2D2D"),
                                    )
                                    .indent(Some(284), None, None, None)
                                    .line_spacing(
                                        LineSpacing::new()
                                            .line_rule(LineSpacingType::Exact)
                                            .line(280),
                                    ),
                            );
                        }
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
                                            Shading::new().fill("1F3864").shd_type(ShdType::Clear),
                                        )
                                        .vertical_align(VAlignType::Center)
                                        .add_paragraph(
                                            Paragraph::new()
                                                .add_run(
                                                    Run::new()
                                                        .add_text(h)
                                                        .size(20)
                                                        .bold()
                                                        .color("FFFFFF"),
                                                )
                                                .line_spacing(
                                                    docx_rs::LineSpacing::new()
                                                        .line_rule(docx_rs::LineSpacingType::Auto)
                                                        .line(240),
                                                ),
                                        )
                                })
                                .collect();
                            t_rows.push(TableRow::new(cells));
                        }

                        // 数据行
                        for (ri, row) in table_rows.iter().enumerate() {
                            let cells: Vec<TableCell> = row
                                .iter()
                                .map(|cell| {
                                    let c = TableCell::new()
                                        .vertical_align(VAlignType::Center)
                                        .add_paragraph(
                                            Paragraph::new()
                                                .add_run(
                                                    Run::new()
                                                        .add_text(cell)
                                                        .size(20)
                                                        .color("333333"),
                                                )
                                                .line_spacing(
                                                    docx_rs::LineSpacing::new()
                                                        .line_rule(docx_rs::LineSpacingType::Auto)
                                                        .line(240),
                                                ),
                                        );
                                    if ri % 2 == 1 {
                                        c.shading(
                                            Shading::new().fill("F2F7FB").shd_type(ShdType::Clear),
                                        )
                                    } else {
                                        c
                                    }
                                })
                                .collect();
                            t_rows.push(TableRow::new(cells));
                        }

                        // 表格边框：外框粗线，内线细线
                        let table_borders = TableBorders::new()
                            .set(
                                TableBorder::new(TableBorderPosition::Top)
                                    .size(8)
                                    .color("1F3864"),
                            )
                            .set(
                                TableBorder::new(TableBorderPosition::Bottom)
                                    .size(8)
                                    .color("1F3864"),
                            )
                            .set(
                                TableBorder::new(TableBorderPosition::Left)
                                    .size(4)
                                    .color("D0D0D0"),
                            )
                            .set(
                                TableBorder::new(TableBorderPosition::Right)
                                    .size(4)
                                    .color("D0D0D0"),
                            )
                            .clear(TableBorderPosition::InsideH)
                            .clear(TableBorderPosition::InsideV);

                        doc = doc.add_table(
                            Table::new(t_rows)
                                .set_borders(table_borders)
                                .width(5000, WidthType::Pct),
                        );
                        doc = doc.add_paragraph(
                            Paragraph::new()
                                .add_run(Run::new().add_text(""))
                                .line_spacing(
                                    docx_rs::LineSpacing::new()
                                        .line_rule(docx_rs::LineSpacingType::Auto)
                                        .line(240),
                                ),
                        );
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
                    doc = doc.add_paragraph(
                        Paragraph::new()
                            .add_run(Run::new().add_text(""))
                            .line_spacing(
                                docx_rs::LineSpacing::new()
                                    .line_rule(docx_rs::LineSpacingType::Auto)
                                    .line(240),
                            ),
                    );
                },
                TagEnd::Item if !para_runs.is_empty() => {
                    flush_text(
                        &mut para_runs,
                        &mut text_buf,
                        &mut in_heading,
                        &mut heading_text,
                        in_table,
                        &mut table_row_text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &link_url,
                        in_code_block,
                    );
                    let mut p = Paragraph::new().line_spacing(
                        docx_rs::LineSpacing::new()
                            .line_rule(docx_rs::LineSpacingType::Auto)
                            .line(240),
                    );
                    for r in std::mem::take(&mut para_runs) {
                        p = p.add_run(r);
                    }
                    if list_ordered {
                        p = p.numbering(NumberingId::new(10), IndentLevel::new(0));
                    } else {
                        p = p.numbering(NumberingId::new(20), IndentLevel::new(0));
                    }
                    doc = doc.add_paragraph(p);
                },
                TagEnd::Item => {},
                TagEnd::BlockQuote(_) => in_blockquote = false,
                TagEnd::Image => {},
                _ => {},
            },

            // ── Text / Code events ──
            Event::Text(text) => {
                if in_code_block {
                    code_lines.push(text.to_string());
                } else {
                    text_buf.push_str(&text);
                }
            },
            Event::Code(text) => {
                if in_code_block {
                    code_lines.push(text.to_string());
                } else if !in_table {
                    flush_text(
                        &mut para_runs,
                        &mut text_buf,
                        &mut in_heading,
                        &mut heading_text,
                        in_table,
                        &mut table_row_text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &link_url,
                        in_code_block,
                    );
                    para_runs.push(
                        Run::new()
                            .add_text(text.to_string())
                            .size(20)
                            .fonts(RunFonts::new().ascii("Consolas").hi_ansi("Consolas"))
                            .color("C7254E"),
                    );
                } else {
                    table_row_text.push_str(&text);
                }
            },

            // ── Break events ──
            Event::SoftBreak | Event::HardBreak if !in_table => {
                text_buf.push(' ');
            },
            Event::SoftBreak | Event::HardBreak => {},

            // ── Rule ──
            Event::Rule => {
                flush_text(
                    &mut para_runs,
                    &mut text_buf,
                    &mut in_heading,
                    &mut heading_text,
                    in_table,
                    &mut table_row_text,
                    bold_depth,
                    italic_depth,
                    strike_depth,
                    &link_url,
                    in_code_block,
                );
                if !para_runs.is_empty() {
                    let p = Paragraph::new();
                    let p = add_runs_to_para(p, std::mem::take(&mut para_runs), in_blockquote);
                    doc = doc.add_paragraph(p);
                }
                doc = doc.add_paragraph(
                    Paragraph::new().align(AlignmentType::Center).line_spacing(
                        LineSpacing::new()
                            .line_rule(LineSpacingType::Exact)
                            .line(40)
                            .before(120)
                            .after(120),
                    ),
                );
            },

            // ── Task list marker ──
            Event::TaskListMarker(checked) => {
                text_buf.push_str(if checked { "☑ " } else { "☐ " });
            },

            _ => {},
        }
    }

    // 刷新残留内容
    flush_text(
        &mut para_runs,
        &mut text_buf,
        &mut in_heading,
        &mut heading_text,
        in_table,
        &mut table_row_text,
        bold_depth,
        italic_depth,
        strike_depth,
        &link_url,
        in_code_block,
    );
    if !para_runs.is_empty() {
        let p = Paragraph::new();
        let p = add_runs_to_para(p, std::mem::take(&mut para_runs), in_blockquote);
        doc = doc.add_paragraph(p);
    }
    if !heading_text.is_empty() {
        let lvl = in_heading.unwrap_or(1);
        let heading_style = match lvl {
            1 => "Heading1",
            2 => "Heading2",
            _ => "Heading3",
        };
        doc = doc.add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text(std::mem::take(&mut heading_text)))
                .style(heading_style),
        );
    }

    // ── 页脚（含页码） ──
    let footer = Footer::new().add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text("第 ").size(16).color("888888"))
            .add_page_num(PageNum::new())
            .add_run(Run::new().add_text(" 页，共 ").size(16).color("888888"))
            .add_num_pages(NumPages::new())
            .add_run(Run::new().add_text(" 页").size(16).color("888888"))
            .align(AlignmentType::Center),
    );
    doc = doc.footer(footer);

    doc
}

fn add_runs_to_para(
    p: docx_rs::Paragraph,
    runs: Vec<docx_rs::Run>,
    _in_blockquote: bool,
) -> docx_rs::Paragraph {
    let mut p = p;
    for r in runs {
        p = p.add_run(r);
    }
    p
}

/// 嵌入图片到文档
fn embed_image(mut doc: docx_rs::Docx, alt: &str, path: &str) -> docx_rs::Docx {
    use docx_rs::*;
    use std::path::Path;

    let resolved = Path::new(path);
    if !resolved.exists() {
        return doc.add_paragraph(
            Paragraph::new().add_run(
                Run::new()
                    .add_text(format!("[图片: {} ({})]", alt, path))
                    .size(20)
                    .italic()
                    .color("999999"),
            ),
        );
    }

    match std::fs::read(resolved) {
        Ok(bytes) => {
            let pic = Pic::new(&bytes);
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_image(pic))
                    .align(AlignmentType::Center),
            );
            if !alt.is_empty() {
                doc = doc.add_paragraph(
                    Paragraph::new()
                        .add_run(
                            Run::new()
                                .add_text(format!("图：{}", alt))
                                .size(18)
                                .italic()
                                .color("888888"),
                        )
                        .align(AlignmentType::Center),
                );
            }
        },
        Err(e) => {
            doc = doc.add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(format!("[图片读取失败: {} — {}]", path, e))
                        .size(20)
                        .italic()
                        .color("CC0000"),
                ),
            );
        },
    }

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
    use lopdf::{dictionary, Document, Object, ObjectId, Stream};

    let mut pdf = Document::new();

    // ── 字体（内置 Type1，无需嵌入）──
    let fid_h = pdf.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" },
    );
    let fid_hb = pdf.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica-Bold" },
    );
    let fid_c = pdf.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier" },
    );
    let font_res = dictionary! { "F1" => fid_h, "F2" => fid_hb, "F3" => fid_c };

    // ── 页面参数（A4, pt）──
    let pw: f64 = 595.0;
    let ph: f64 = 842.0;
    let ml: f64 = 72.0;
    let mr: f64 = pw - 60.0;
    let mt: f64 = 720.0;
    let mb: f64 = 60.0;
    let tw: f64 = mr - ml;
    let lh: f64 = 14.0;

    // ── 收集文本行 ──
    struct Line {
        text: String,
        font: &'static str,
        size: f64,
        x: f64,
        gap: f64,
    }
    let mut blocks: Vec<Vec<Line>> = Vec::new();
    let mut cur: Vec<Line> = Vec::new();
    let push = |cur: &mut Vec<Line>, blocks: &mut Vec<Vec<Line>>| {
        if !cur.is_empty() {
            blocks.push(std::mem::take(cur));
        }
    };

    // 封面
    cur.push(Line {
        text: title.to_string(),
        font: "F2",
        size: 24.0,
        x: ml,
        gap: lh * 2.0,
    });
    cur.push(Line {
        text: format!("由 AxAgent 生成 | {}", chrono::Local::now().format("%Y-%m-%d")),
        font: "F1",
        size: 10.0,
        x: ml,
        gap: lh * 3.0,
    });
    push(&mut cur, &mut blocks);

    for block in &doc.blocks {
        match block {
            markdown::MdBlock::Heading { level, text } => {
                let (font, size) = match level {
                    1 => ("F2", 16.0),
                    2 => ("F2", 13.0),
                    _ => ("F2", 11.0),
                };
                cur.push(Line {
                    text: String::new(),
                    font: "F1",
                    size: 6.0,
                    x: ml,
                    gap: 6.0,
                });
                cur.push(Line {
                    text: text.clone(),
                    font,
                    size,
                    x: ml,
                    gap: if *level <= 2 { lh * 1.6 } else { lh },
                });
                cur.push(Line {
                    text: "─".repeat((tw / 5.0) as usize),
                    font: "F1",
                    size: 1.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::Paragraph { inlines } => {
                let text = inlines_to_plain_text(inlines);
                if text.trim().is_empty() {
                    continue;
                }
                for line in wrap_lines(&text, tw, 9.0) {
                    cur.push(Line {
                        text: line,
                        font: "F1",
                        size: 9.0,
                        x: ml,
                        gap: lh,
                    });
                }
                cur.push(Line {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::CodeBlock { code, .. } => {
                for line in code.lines() {
                    cur.push(Line {
                        text: line.replace('\t', "    "),
                        font: "F3",
                        size: 8.0,
                        x: ml + 12.0,
                        gap: lh * 0.85,
                    });
                }
                cur.push(Line {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::Table { headers, rows } => {
                let cols = headers.len().max(1) as f64;
                let cw = tw / cols;
                cur.push(Line {
                    text: format_row(&headers, cw),
                    font: "F2",
                    size: 8.0,
                    x: ml,
                    gap: lh,
                });
                for row in rows {
                    cur.push(Line {
                        text: format_row(&row, cw),
                        font: "F1",
                        size: 8.0,
                        x: ml,
                        gap: lh * 0.85,
                    });
                }
                cur.push(Line {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::List { items, .. } => {
                for item in items {
                    let text = inlines_to_plain_text(item);
                    cur.push(Line {
                        text: format!("  • {}", text),
                        font: "F1",
                        size: 9.0,
                        x: ml + 14.0,
                        gap: lh,
                    });
                }
                cur.push(Line {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::Blockquote { inlines } => {
                let text = inlines_to_plain_text(inlines);
                cur.push(Line {
                    text: format!("▎ {}", text),
                    font: "F1",
                    size: 8.0,
                    x: ml + 14.0,
                    gap: lh * 1.1,
                });
                cur.push(Line {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::HorizontalRule => {
                cur.push(Line {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
                cur.push(Line {
                    text: "─".repeat(60),
                    font: "F1",
                    size: 7.0,
                    x: ml,
                    gap: lh,
                });
                cur.push(Line {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::Image { alt, .. } => {
                cur.push(Line {
                    text: format!("[图片: {}]", alt),
                    font: "F1",
                    size: 8.0,
                    x: ml,
                    gap: lh,
                });
                cur.push(Line {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
        }
    }
    push(&mut cur, &mut blocks);

    // ── 页面分配 ──
    let mut pages: Vec<Vec<u8>> = Vec::new();
    let mut buf: Vec<u8> = Vec::new();
    let mut y = mt;

    for blk in &blocks {
        for line in blk {
            let need = line.gap + 4.0;
            if y < mb + lh * 3.0 || (y - need < mb && !pages.is_empty()) {
                pages.push(std::mem::take(&mut buf));
                y = mt;
            }
            if !line.text.is_empty() {
                let escaped = line
                    .text
                    .replace('\\', "\\\\")
                    .replace('(', "\\(")
                    .replace(')', "\\)");
                buf.extend_from_slice(
                    format!(
                        "/{} {} Tf\n1 0 0 1 {} {} Tm\n({}) Tj\n",
                        line.font, line.size, line.x, y, escaped
                    )
                    .as_bytes(),
                );
            }
            y -= need;
        }
    }
    pages.push(buf);

    // ── 构建 PDF 对象 ──
    let mut page_ids: Vec<ObjectId> = Vec::new();
    for (pi, body) in pages.iter().enumerate() {
        let mut content = body.clone();
        // 页码
        content.extend_from_slice(
            format!(
                "BT\n/F1 7 Tf\n1 0 0 1 {} 28 Tm\n({} / {}) Tj\nET\n",
                pw / 2.0 - 15.0,
                pi + 1,
                pages.len()
            )
            .as_bytes(),
        );
        let sid = pdf.add_object(Stream::new(lopdf::Dictionary::new(), content));
        let pid = pdf.new_object_id();
        pdf.objects.insert(pid, Object::Dictionary(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(pw as f32), Object::Real(ph as f32)],
            "Contents" => vec![sid.into()],
            "Resources" => dictionary! { "Font" => font_res.clone() },
        }));
        page_ids.push(pid);
    }

    let pt_id = pdf.new_object_id();
    let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
    pdf.objects.insert(
        pt_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => kids, "Count" => Object::Integer(page_ids.len() as i64),
        }),
    );
    for pid in &page_ids {
        if let Ok(d) = pdf.get_dictionary_mut(*pid) {
            d.set("Parent", Object::Reference(pt_id));
        }
    }
    let cat_id = pdf.new_object_id();
    pdf.objects.insert(
        cat_id,
        Object::Dictionary(dictionary! { "Type" => "Catalog", "Pages" => pt_id }),
    );
    pdf.trailer.set("Root", Object::Reference(cat_id));

    pdf.save(output_path)
        .map(|_| ())
        .map_err(|e| format!("保存 PDF 失败: {}", e))
}

fn wrap_lines(text: &str, max_w_pt: f64, font_pt: f64) -> Vec<String> {
    let max_chars = (max_w_pt / (font_pt * 0.5)) as usize;
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        if cur.chars().count() + word.chars().count() > max_chars && !cur.is_empty() {
            lines.push(cur.trim_end().to_string());
            cur = String::new();
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur.trim_end().to_string());
    }
    if lines.is_empty() {
        lines.push(text.to_string());
    }
    lines
}

fn format_row(cells: &[String], col_w: f64) -> String {
    let max_chars = (col_w / 5.0) as usize;
    cells
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let s = if c.chars().count() > max_chars {
                format!("{}…", &c[..max_chars.saturating_sub(1)])
            } else {
                c.clone()
            };
            if i == 0 {
                s
            } else {
                format!("  {}", s)
            }
        })
        .collect::<Vec<_>>()
        .join("")
}
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

    // 专业格式定义
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x1F3864))
        .set_font_color(Color::White)
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::RGB(0x1F3864))
        .set_text_wrap();

    let data_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::RGB(0xD0D0D0));

    let alt_row_format = Format::new()
        .set_font_size(10)
        .set_background_color(Color::RGB(0xF2F7FB))
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::RGB(0xD0D0D0));

    let number_format = Format::new()
        .set_font_size(10)
        .set_border(FormatBorder::Thin)
        .set_border_color(Color::RGB(0xD0D0D0))
        .set_num_format("#,##0.00");

    for (ti, table_block) in tables.iter().enumerate() {
        let (headers, rows) = match table_block {
            markdown::MdBlock::Table { headers, rows } => (headers, rows),
            _ => continue,
        };

        let name = if ti == 0 {
            sanitize_sheet_name(sheet_name)
        } else {
            sanitize_sheet_name(&format!("{}_{}", sheet_name, ti + 1))
        };
        let worksheet = workbook
            .add_worksheet()
            .set_name(&name)
            .map_err(|e| e.to_string())?;

        let num_cols = headers.len().max(1) as u16;
        let num_rows = rows.len() + 1;

        // 写表头
        for (ci, h) in headers.iter().enumerate() {
            worksheet
                .write_with_format(0, ci as u16, h, &header_format)
                .map_err(|e| e.to_string())?;
        }

        // 写数据行
        for (ri, row) in rows.iter().enumerate() {
            let row_idx = (ri + 1) as u32;
            let fmt = if ri % 2 == 1 {
                &alt_row_format
            } else {
                &data_format
            };
            for (ci, cell) in row.iter().enumerate() {
                // 尝试检测数字
                let cell_trimmed = cell.trim();
                let cell_fmt = if cell_trimmed.parse::<f64>().is_ok() {
                    &number_format
                } else {
                    fmt
                };
                worksheet
                    .write_with_format(row_idx, ci as u16, cell_trimmed, cell_fmt)
                    .map_err(|e| e.to_string())?;
            }
        }

        // 冻结表头行
        worksheet
            .set_freeze_panes(1, 0)
            .map_err(|e| e.to_string())?;

        // 自动筛选
        worksheet
            .autofilter(0, 0, (num_rows - 1) as u32, num_cols - 1)
            .map_err(|e| e.to_string())?;

        // 自动列宽（基于表头和数据内容）
        for ci in 0..num_cols {
            let mut max_width: f64 = headers
                .get(ci as usize)
                .map(|h| char_width_estimate(h))
                .unwrap_or(8.0);
            for row in rows.iter() {
                if let Some(cell) = row.get(ci as usize) {
                    let w = char_width_estimate(cell);
                    if w > max_width {
                        max_width = w;
                    }
                }
            }
            // 限制列宽范围：8~40 字符宽度
            let col_width = max_width.clamp(8.0, 40.0) + 2.0;
            worksheet
                .set_column_width(ci, col_width)
                .map_err(|e| e.to_string())?;
        }

        // 表头行高
        worksheet.set_row_height(0, 24).map_err(|e| e.to_string())?;
    }

    workbook.save(output_path).map_err(|e| e.to_string())?;
    Ok(tables.len())
}

/// 估算字符串的字符宽度（CJK 字符 ≈ 2，ASCII ≈ 1）
fn char_width_estimate(s: &str) -> f64 {
    let mut width = 0.0f64;
    for c in s.chars() {
        if c > '\u{2E80}' {
            width += 2.0; // CJK / 全角
        } else {
            width += 1.0;
        }
    }
    width
}

fn sanitize_sheet_name(name: &str) -> String {
    let s: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == ' ')
        .take(31)
        .collect();
    if s.is_empty() {
        "Sheet1".to_string()
    } else {
        s
    }
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

    // 每张幻灯片 — 专业布局
    for (si, (slide_title, bullets)) in slides.iter().enumerate() {
        let slide_num = si + 1;
        let escaped_title = xml_escape(slide_title);

        // 标题: 顶部居中, 大号深蓝粗体, 底部带金色装饰线
        let mut slide_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
             <p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"\
             xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"\
             xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\n\
             <p:cSld>\n<p:spTree>\n\
             // 顶部装饰条\n\
             <p:sp><p:nvSpPr><p:cNvPr id=\"4\" name=\"AccentBar\"/><p:cNvSpPr><p:spLocks noGrp=\"1\"/></p:cNvSpPr>\
             <p:nvPr/></p:nvSpPr>\
             <p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"9144000\" cy=\"73152\"/></a:xfrm>\
             <a:solidFill><a:srgbClr val=\"1F3864\"/></a:solidFill></p:spPr></p:sp>\n\
             // 标题\n\
             <p:sp><p:nvSpPr><p:cNvPr id=\"1\" name=\"Title\"/><p:cNvSpPr><p:spLocks noGrp=\"1\"/></p:cNvSpPr>\
             <p:nvPr/></p:nvSpPr>\
             <p:spPr><a:xfrm><a:off x=\"685800\" y=\"274320\"/><a:ext cx=\"7772400\" cy=\"914400\"/></a:xfrm></p:spPr>\
             <p:txBody><a:bodyPr/><a:lstStyle/>\
             <a:p><a:pPr algn=\"l\"/>\
             <a:r><a:rPr lang=\"zh-CN\" sz=\"3200\" b=\"1\">\
             <a:solidFill><a:srgbClr val=\"1F3864\"/></a:solidFill></a:rPr>\
             <a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>\n\
             // 标题下划线\n\
             <p:sp><p:nvSpPr><p:cNvPr id=\"5\" name=\"TitleLine\"/><p:cNvSpPr><p:spLocks noGrp=\"1\"/></p:cNvSpPr>\
             <p:nvPr/></p:nvSpPr>\
             <p:spPr><a:xfrm><a:off x=\"685800\" y=\"1219200\"/><a:ext cx=\"1371600\" cy=\"27432\"/></a:xfrm>\
             <a:solidFill><a:srgbClr val=\"2E75B6\"/></a:solidFill></p:spPr></p:sp>\n",
            escaped_title
        );

        // 内容区域: 项目符号列表
        let mut content_y: u32 = 1_554_000;
        for (bi, bullet) in bullets.iter().enumerate() {
            if content_y > 6_200_000 {
                break;
            } // 超出幻灯片区域
            let escaped_bullet = xml_escape(bullet);
            // 项目符号（蓝色圆点）+ 文本
            slide_xml.push_str(&format!(
                "<p:sp><p:nvSpPr><p:cNvPr id=\"{}\" name=\"Content{}\"/><p:cNvSpPr><p:spLocks noGrp=\"1\"/></p:cNvSpPr>\
                 <p:nvPr/></p:nvSpPr>\
                 <p:spPr><a:xfrm><a:off x=\"914400\" y=\"{}\"/><a:ext cx=\"7315200\" cy=\"365760\"/></a:xfrm></p:spPr>\
                 <p:txBody><a:bodyPr/>\
                 <a:lstStyle><a:lvl1pPr marL=\"285750\" indent=\"-285750\">\
                 <a:buChar char=\"•\"/><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr>\
                 <a:buSzTx/><a:buFont typeface=\"Calibri\"/></a:lvl1pPr></a:lstStyle>\
                 <a:p><a:pPr marL=\"285750\" indent=\"-285750\"/>\
                 <a:r><a:rPr lang=\"zh-CN\" sz=\"2200\">\
                 <a:solidFill><a:srgbClr val=\"333333\"/></a:solidFill></a:rPr>\
                 <a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>\n",
                100 + bi, bi, content_y, escaped_bullet
            ));
            content_y += 411_480;
        }

        // 幻灯片编号（右下角）
        slide_xml.push_str(&format!(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"SlideNum\"/><p:cNvSpPr><p:spLocks noGrp=\"1\"/></p:cNvSpPr>\
             <p:nvPr/></p:nvSpPr>\
             <p:spPr><a:xfrm><a:off x=\"8229600\" y=\"6400800\"/><a:ext cx=\"685800\" cy=\"365760\"/></a:xfrm></p:spPr>\
             <p:txBody><a:bodyPr/>\
             <a:p><a:pPr algn=\"r\"/><a:r><a:rPr lang=\"en-US\" sz=\"1200\">\
             <a:solidFill><a:srgbClr val=\"999999\"/></a:solidFill></a:rPr>\
             <a:t>{} / {}</a:t></a:r></a:p></p:txBody></p:sp>\n",
            slide_num, slide_count
        ));

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
