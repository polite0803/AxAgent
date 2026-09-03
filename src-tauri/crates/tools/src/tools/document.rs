// SPDX-License-Identifier: AGPL-3.0-only

//! 文档格式工具
//!
//! ExportWord (MD→DOCX), ExportPdf (MD→PDF), ExportXlsx (MD→XLSX),
//! ExportPptx (MD→PPTX), RenderMarkdown (MD→HTML),
//! ReadXlsx, ReadPptx (读取 OOXML 文本)
//!
//! 全部纯 Rust 实现，无需安装 Python/LibreOffice。

use crate::{Tool, ToolCategory, ToolContext, ToolDomain, ToolError, ToolResult, markdown};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::BTreeMap;
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
                "title": {"type": "string", "default": "Document", "description": "文档标题"},
                "image_max_width_pt": {"type": "number", "default": 540, "description": "图片最大宽度（pt，默认 540 ≈ 7.5 inch），超出会等比例缩放"},
                "image_align": {"type": "string", "enum": ["left", "center", "right"], "default": "center", "description": "图片对齐方式"}
            },
            "required": ["markdown", "output_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input.get("markdown").and_then(|v| v.as_str()).unwrap_or_default();
        let output_path = input.get("output_path").and_then(|v| v.as_str()).unwrap_or_default();
        let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("Document");
        let image_max_width_pt = input.get("image_max_width_pt").and_then(|v| v.as_f64());
        let image_align = input.get("image_align").and_then(|v| v.as_str().map(String::from));

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

        let doc = build_docx_from_md(markdown_text, title, image_max_width_pt, image_align);

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
/// Markdown 转 Word 文档（公开给 Tauri 命令等外部调用）
pub fn build_docx_from_md(
    markdown_text: &str,
    title: &str,
    image_max_width_pt: Option<f64>,
    image_align: Option<String>,
) -> docx_rs::Docx {
    use docx_rs::*;
    use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

    let mut doc = Docx::new();

    // ── 页面设置 ──
    doc = doc.page_size(11906, 16838); // A4
    doc = doc.page_margin(PageMargin::new()
            .top(1440)    // 1 inch
            .bottom(1440)
            .left(1440)
            .right(1440));

    // ── 文档样式 ──
    let default_font =
        RunFonts::new().ascii("Calibri").hi_ansi("Calibri").east_asia("微软雅黑").cs("Calibri");

    let heading_font =
        RunFonts::new().ascii("Calibri Light").hi_ansi("Calibri Light").east_asia("微软雅黑");

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
            .line_spacing(LineSpacing::new().after(240).line_rule(LineSpacingType::Auto).line(276)),
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
                Tag::Image { dest_url, title, .. } => {
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
                    doc = embed_image(
                        doc,
                        &title,
                        &dest_url,
                        image_max_width_pt.unwrap_or(540.0),
                        match image_align.as_deref() {
                            Some("left") => AlignmentType::Left,
                            Some("right") => AlignmentType::Right,
                            _ => AlignmentType::Center,
                        },
                    );
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
                Tag::TableRow => {
                    // 数据行开始：重置 in_table_head（表头行已由 TableHead 处理）
                    in_table_head = false;
                    table_row_cells.clear();
                },
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
                        Paragraph::new().add_run(Run::new().add_text(text)).style(heading_style),
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
                        doc =
                            doc.add_paragraph(p.indent(Some(284), None, None, None).line_spacing(
                                LineSpacing::new().line_rule(LineSpacingType::Auto).line(276),
                            ));
                    } else {
                        doc = doc.add_paragraph(p);
                    }
                },
                TagEnd::Paragraph => {},
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    if !code_lines.is_empty() {
                        // Mermaid 流程图：解析 + 渲染为 Unicode 框线字符文本
                        let (label, content_lines): (String, Vec<String>) = if code_lang
                            .trim()
                            .eq_ignore_ascii_case("mermaid")
                        {
                            let raw = code_lines.join("\n");
                            let graph = crate::mermaid::parse_mermaid(&raw);
                            let rendered = crate::mermaid::render_to_text(&graph);
                            (
                                "流程图".to_string(),
                                rendered.lines().map(|s| s.to_string()).collect(),
                            )
                        } else {
                            let label = if code_lang.is_empty() {
                                "代码".to_string()
                            } else {
                                code_lang.to_string()
                            };
                            (label, code_lines.iter().map(|s| s.replace('\t', "    ")).collect())
                        };
                        doc = doc.add_paragraph(
                            Paragraph::new()
                                .add_run(
                                    Run::new().add_text(&label).size(16).bold().color("888888"),
                                )
                                .indent(Some(284), None, None, None),
                        );
                        for line in &content_lines {
                            doc = doc.add_paragraph(
                                Paragraph::new()
                                    .add_run(
                                        Run::new()
                                            .add_text(line)
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
                            .set(TableBorder::new(TableBorderPosition::Top).size(8).color("1F3864"))
                            .set(
                                TableBorder::new(TableBorderPosition::Bottom)
                                    .size(8)
                                    .color("1F3864"),
                            )
                            .set(
                                TableBorder::new(TableBorderPosition::Left).size(4).color("D0D0D0"),
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
                            Paragraph::new().add_run(Run::new().add_text("")).line_spacing(
                                docx_rs::LineSpacing::new()
                                    .line_rule(docx_rs::LineSpacingType::Auto)
                                    .line(240),
                            ),
                        );

                        // 表格若含数字列，追加 Unicode 块字符条形图段落
                        if let Some(val_col) = find_numeric_column(&table_headers, &table_rows) {
                            for para in
                                make_docx_chart_paragraphs(&table_headers, &table_rows, val_col)
                            {
                                doc = doc.add_paragraph(para);
                            }
                        }
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
                TagEnd::TableHead => {
                    // 表头行结束：把累积的 cells 赋给 table_headers
                    if !text_buf.is_empty() {
                        table_row_cells.push(std::mem::take(&mut text_buf));
                    } else if !table_row_text.is_empty() {
                        table_row_cells.push(std::mem::take(&mut table_row_text));
                    }
                    if !table_row_cells.is_empty() && table_headers.is_empty() {
                        table_headers = std::mem::take(&mut table_row_cells);
                    }
                },
                TagEnd::TableCell => {
                    // 单元格结束：把 text_buf 的内容推入 table_row_cells
                    if !text_buf.is_empty() {
                        table_row_cells.push(std::mem::take(&mut text_buf));
                    } else if !table_row_text.is_empty() {
                        table_row_cells.push(std::mem::take(&mut table_row_text));
                    }
                },
                TagEnd::List(_) => {
                    doc = doc.add_paragraph(
                        Paragraph::new().add_run(Run::new().add_text("")).line_spacing(
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
fn embed_image(
    mut doc: docx_rs::Docx,
    alt: &str,
    path: &str,
    max_width_pt: f64,
    align: docx_rs::AlignmentType,
) -> docx_rs::Docx {
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
            // 用 image crate 解码尺寸以便按 max_width_pt 缩放
            let (w_pt, h_pt) = compute_image_size_pt(&bytes, max_width_pt)
                .unwrap_or((max_width_pt, max_width_pt * 0.75)); // fallback 4:3
            // EMU = pt * 12700
            let w_emu = (w_pt * 12700.0) as u32;
            let h_emu = (h_pt * 12700.0) as u32;

            let pic = Pic::new(&bytes).size(w_emu, h_emu);
            doc =
                doc.add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic)).align(align));
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
                        .align(align),
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

/// 从图片字节中解码尺寸（PNG/JPEG/GIF/BMP/WebP）并按 max_width_pt 等比例缩放，返回 (w_pt, h_pt)。
/// 解码失败时返回 None，调用方应使用 fallback 尺寸。
fn compute_image_size_pt(bytes: &[u8], max_width_pt: f64) -> Option<(f64, f64)> {
    use std::io::Cursor;
    let reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok()?;
    let dims = reader.into_dimensions().ok()?;
    let (w_px, h_px) = dims;
    if w_px == 0 || h_px == 0 {
        return None;
    }
    // 假设图片 96 DPI：1 px = 0.75 pt（OpenXML/Word 默认 96 DPI）
    let w_pt = w_px as f64 * 0.75;
    let h_pt = h_px as f64 * 0.75;
    if w_pt <= max_width_pt {
        return Some((w_pt, h_pt));
    }
    // 等比例缩放
    let scale = max_width_pt / w_pt;
    Some((max_width_pt, h_pt * scale))
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input.get("markdown").and_then(|v| v.as_str()).unwrap_or_default();

        if markdown_text.is_empty() {
            return Ok(ToolResult::error("Error: markdown 是必需的"));
        }

        let html = markdown::render_to_html(markdown_text);

        if let Some(path_str) = input.get("output_path").and_then(|v| v.as_str())
            && !path_str.is_empty()
        {
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
                "title": {"type": "string", "default": "Document", "description": "文档标题"},
                "image_max_width_pt": {
                    "type": "number",
                    "default": 540.0,
                    "description": "图片最大宽度（pt），超过则等比例缩放。1 pt = 1/72 inch，PDF A4 正文区约 540 pt 宽"
                },
                "image_align": {
                    "type": "string",
                    "enum": ["left", "center", "right"],
                    "default": "center",
                    "description": "图片水平对齐方式"
                },
                "image_base_dir": {
                    "type": "string",
                    "description": "图片相对路径的基准目录（可选）。不传则使用调用方 working_dir 的相对路径"
                },
                "subtitle": {
                    "type": "string",
                    "default": "",
                    "description": "封面副标题（默认封面模板里会用到）"
                },
                "author": {
                    "type": "string",
                    "default": "",
                    "description": "文档作者（默认封面模板里会用到）"
                },
                "cover_template": {
                    "type": "string",
                    "description": "封面 MiniJinja 模板。变量：title/subtitle/date/author"
                },
                "header_template": {
                    "type": "string",
                    "description": "页眉 MiniJinja 模板。变量：title/date/author"
                },
                "footer_template": {
                    "type": "string",
                    "description": "页脚 MiniJinja 模板。变量：title/date/author/page_no/total_pages"
                },
                "enable_toc": {
                    "type": "boolean",
                    "default": false,
                    "description": "是否在封面后生成目录页（从 Markdown 标题自动提取）"
                }
            },
            "required": ["markdown", "output_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input.get("markdown").and_then(|v| v.as_str()).unwrap_or_default();
        let output_path = input.get("output_path").and_then(|v| v.as_str()).unwrap_or_default();
        let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("Document");
        let image_max_width_pt =
            input.get("image_max_width_pt").and_then(|v| v.as_f64()).unwrap_or(540.0);
        let image_align = input.get("image_align").and_then(|v| v.as_str()).unwrap_or("center");
        let image_base_dir = input.get("image_base_dir").and_then(|v| v.as_str());
        let subtitle = input.get("subtitle").and_then(|v| v.as_str()).unwrap_or("");
        let author = input.get("author").and_then(|v| v.as_str()).unwrap_or("");
        let cover_template = input.get("cover_template").and_then(|v| v.as_str());
        let header_template = input.get("header_template").and_then(|v| v.as_str());
        let footer_template = input.get("footer_template").and_then(|v| v.as_str());
        let enable_toc = input.get("enable_toc").and_then(|v| v.as_bool()).unwrap_or(false);

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

        // 解析图片基准目录：优先用 image_base_dir，否则用 ctx.working_dir
        let base_dir_owned: String =
            image_base_dir.map(|s| s.to_string()).unwrap_or_else(|| ctx.working_dir.clone());

        match build_pdf(
            &doc,
            title,
            subtitle,
            author,
            output_path,
            image_max_width_pt,
            image_align,
            &base_dir_owned,
            cover_template,
            header_template,
            footer_template,
            enable_toc,
        ) {
            Ok(()) => Ok(ToolResult::success(format!(
                "PDF 已导出: {} ({} 字符输入)",
                output_path,
                markdown_text.len()
            ))),
            Err(e) => Ok(ToolResult::error(format!("创建 PDF 失败: {}", e))),
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Markdown 转 PDF（公开给 Tauri 命令等外部调用）
pub fn build_pdf(
    doc: &markdown::MdDocument,
    title: &str,
    subtitle: &str,
    author: &str,
    output_path: &str,
    image_max_width_pt: f64,
    image_align: &str,
    image_base_dir: &str,
    cover_template: Option<&str>,
    header_template: Option<&str>,
    footer_template: Option<&str>,
    enable_toc: bool,
) -> Result<(), String> {
    use lopdf::{Document, Object, ObjectId, Stream, dictionary};

    // ── 尝试加载 CJK 字体（首次调用时执行文件系统查找 + leak TTF 字节）──
    let cjk_font = crate::cjk_font::cjk_font();

    // 内容中是否含 CJK，决定是否需要注册 CIDFont
    let content_needs_cjk = doc.blocks.iter().any(block_needs_cjk);

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
    let mut font_res = dictionary! { "F1" => fid_h, "F2" => fid_hb, "F3" => fid_c };

    // ── 页面参数（A4, pt）──
    let pw: f64 = 595.0;
    let ph: f64 = 842.0;
    let ml: f64 = 72.0;
    let mr: f64 = pw - 60.0;
    let mt: f64 = 720.0;
    let mb: f64 = 60.0;
    let tw: f64 = mr - ml;
    let lh: f64 = 14.0;

    // ── 收集文本/图片行（PageOp 枚举同时承载文本和图片）──
    enum PageOp {
        Text { text: String, font: &'static str, size: f64, x: f64, gap: f64 },
        Image { xobj_id: ObjectId, w_pt: f64, h_pt: f64, gap: f64 },
    }
    let mut blocks: Vec<Vec<PageOp>> = Vec::new();
    let mut cur: Vec<PageOp> = Vec::new();
    let push = |cur: &mut Vec<PageOp>, blocks: &mut Vec<Vec<PageOp>>| {
        if !cur.is_empty() {
            blocks.push(std::mem::take(cur));
        }
    };

    // 渲染封面（MiniJinja 模板 + 多行居中显示）
    let cover_ctx = crate::templates::TemplateContext {
        title: title.to_string(),
        subtitle: subtitle.to_string(),
        date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        author: author.to_string(),
        page_no: 0,
        total_pages: 0,
        items: vec![],
    };
    let cover_text = crate::templates::render_cover(&cover_ctx, cover_template)
        .unwrap_or_else(|_| title.to_string());
    for (i, line) in cover_text.lines().enumerate() {
        let (font, size) = match i {
            0 => ("F2", 24.0), // 标题行
            1 => ("F1", 14.0), // 副标题行
            2 => ("F1", 10.0), // 日期/作者行
            _ => ("F1", 10.0),
        };
        let gap = if i == 0 { lh * 2.0 } else { lh * 1.4 };
        cur.push(PageOp::Text { text: line.to_string(), font, size, x: ml, gap });
    }
    cur.push(PageOp::Text { text: String::new(), font: "F1", size: 4.0, x: ml, gap: 4.0 });
    push(&mut cur, &mut blocks);

    // 可选：插入目录页（从 Markdown 标题提取）
    if enable_toc {
        let toc_items = crate::templates::extract_toc_from_md(doc);
        if !toc_items.is_empty() {
            cur.push(PageOp::Text {
                text: "目录".to_string(),
                font: "F2",
                size: 18.0,
                x: ml,
                gap: lh * 2.0,
            });
            let toc_text = crate::templates::render_toc(&toc_items, None).unwrap_or_default();
            for line in toc_text.lines() {
                cur.push(PageOp::Text {
                    text: line.to_string(),
                    font: "F1",
                    size: 10.0,
                    x: ml,
                    gap: lh * 0.9,
                });
            }
            cur.push(PageOp::Text { text: String::new(), font: "F1", size: 4.0, x: ml, gap: 4.0 });
        }
        push(&mut cur, &mut blocks);
    }

    for block in &doc.blocks {
        match block {
            markdown::MdBlock::Heading { level, text } => {
                let (font, size) = match level {
                    1 => ("F2", 16.0),
                    2 => ("F2", 13.0),
                    _ => ("F2", 11.0),
                };
                cur.push(PageOp::Text {
                    text: String::new(),
                    font: "F1",
                    size: 6.0,
                    x: ml,
                    gap: 6.0,
                });
                cur.push(PageOp::Text {
                    text: text.clone(),
                    font,
                    size,
                    x: ml,
                    gap: if *level <= 2 { lh * 1.6 } else { lh },
                });
                cur.push(PageOp::Text {
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
                for line in wrap_lines_measured(&text, tw, 9.0, cjk_font) {
                    cur.push(PageOp::Text { text: line, font: "F1", size: 9.0, x: ml, gap: lh });
                }
                cur.push(PageOp::Text {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::CodeBlock { language, code } => {
                // Mermaid 流程图：解析 + 渲染为 Unicode 框线字符文本
                let lines: Vec<String> = if language.trim().eq_ignore_ascii_case("mermaid") {
                    let graph = crate::mermaid::parse_mermaid(code);
                    let rendered = crate::mermaid::render_to_text(&graph);
                    rendered.lines().map(|s| s.to_string()).collect()
                } else {
                    code.lines().map(|s| s.replace('\t', "    ")).collect()
                };
                for line in &lines {
                    cur.push(PageOp::Text {
                        text: line.clone(),
                        font: "F3",
                        size: 8.0,
                        x: ml + 12.0,
                        gap: lh * 0.85,
                    });
                }
                cur.push(PageOp::Text {
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
                cur.push(PageOp::Text {
                    text: format_row(headers, cw),
                    font: "F2",
                    size: 8.0,
                    x: ml,
                    gap: lh,
                });
                for row in rows {
                    cur.push(PageOp::Text {
                        text: format_row(row, cw),
                        font: "F1",
                        size: 8.0,
                        x: ml,
                        gap: lh * 0.85,
                    });
                }
                cur.push(PageOp::Text {
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
                    cur.push(PageOp::Text {
                        text: format!("  • {}", text),
                        font: "F1",
                        size: 9.0,
                        x: ml + 14.0,
                        gap: lh,
                    });
                }
                cur.push(PageOp::Text {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::Blockquote { inlines } => {
                let text = inlines_to_plain_text(inlines);
                cur.push(PageOp::Text {
                    text: format!("▎ {}", text),
                    font: "F1",
                    size: 8.0,
                    x: ml + 14.0,
                    gap: lh * 1.1,
                });
                cur.push(PageOp::Text {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::HorizontalRule => {
                cur.push(PageOp::Text {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
                cur.push(PageOp::Text {
                    text: "─".repeat(60),
                    font: "F1",
                    size: 7.0,
                    x: ml,
                    gap: lh,
                });
                cur.push(PageOp::Text {
                    text: String::new(),
                    font: "F1",
                    size: 4.0,
                    x: ml,
                    gap: 4.0,
                });
            },
            markdown::MdBlock::Image { alt, path } => {
                // 解析图片：相对路径用 image_base_dir 拼接，读字节 → JPEG → 注册 XObject
                let resolved = if Path::new(path).is_absolute() {
                    path.clone()
                } else if !image_base_dir.is_empty() {
                    Path::new(image_base_dir).join(path).to_string_lossy().to_string()
                } else {
                    path.clone()
                };
                match load_and_register_image(&mut pdf, &resolved, image_max_width_pt) {
                    Some((xobj_id, w_pt, h_pt)) => {
                        // 顶部空行
                        cur.push(PageOp::Text {
                            text: String::new(),
                            font: "F1",
                            size: 4.0,
                            x: ml,
                            gap: 4.0,
                        });
                        cur.push(PageOp::Image { xobj_id, w_pt, h_pt, gap: lh * 0.4 });
                        if !alt.is_empty() {
                            cur.push(PageOp::Text {
                                text: format!("图：{}", alt),
                                font: "F1",
                                size: 8.0,
                                x: ml,
                                gap: lh * 0.5,
                            });
                        }
                        cur.push(PageOp::Text {
                            text: String::new(),
                            font: "F1",
                            size: 4.0,
                            x: ml,
                            gap: 4.0,
                        });
                    },
                    None => {
                        // 图片加载失败：降级为占位文本
                        cur.push(PageOp::Text {
                            text: format!("[图片缺失: {} ({})]", alt, path),
                            font: "F1",
                            size: 8.0,
                            x: ml,
                            gap: lh,
                        });
                        cur.push(PageOp::Text {
                            text: String::new(),
                            font: "F1",
                            size: 4.0,
                            x: ml,
                            gap: 4.0,
                        });
                    },
                }
            },
        }
    }
    push(&mut cur, &mut blocks);

    // ── CJK 复合字体（Identity-H + CIDFontType2，CIDToGIDMap = Identity）──
    // 必须在收集全部文本（blocks）之后注册：先扫描所有将走 F4 的字符，建立
    // GID→Unicode 的 ToUnicode CMap，保证 PDF 阅读器/ pdf-extract 能正确提取中文与符号。
    let mut used_glyphs: BTreeMap<u16, char> = BTreeMap::new();
    if content_needs_cjk {
        for blk in &blocks {
            for op in blk {
                if let PageOp::Text { text, .. } = op {
                    for c in text.chars() {
                        if crate::cjk_font::needs_cid_font(c) {
                            let gid = cjk_font.and_then(|cj| cj.glyph_index(c)).unwrap_or(0);
                            used_glyphs.insert(gid, c);
                        }
                    }
                }
            }
        }
    }
    let fid_cjk: Option<ObjectId> = if content_needs_cjk {
        match cjk_font {
            Some(cjk) => {
                let cjk_id = register_cjk_font(&mut pdf, cjk, &used_glyphs);
                font_res.set("F4", Object::Reference(cjk_id));
                Some(cjk_id)
            },
            None => {
                tracing::warn!(
                    "ExportPdf: 文档含 CJK 字符但未找到 CJK 字体，CJK 文本将回退为拉丁字体导致乱码。\
                     请将 TTF 放到 $AXAGENT_FONT_DIR 或 ./fonts/，或安装系统 CJK 字体。"
                );
                None
            },
        }
    } else {
        None
    };

    // ── 页面分配 ──
    // 第一遍：扫描所有 Image Op 收集其 xobj_id + 对应名字（Im1/Im2/...）
    // 为了让所有页都能 do 任意图片，每页 Resources.XObject 都包含全部 XObject
    let mut all_xobjs: Vec<(ObjectId, &'static str)> = Vec::new();
    let mut img_counter: usize = 0;
    for blk in &blocks {
        for op in blk {
            if let PageOp::Image { xobj_id, .. } = op {
                img_counter += 1;
                let name: &'static str = Box::leak(format!("Im{}", img_counter).into_boxed_str());
                all_xobjs.push((*xobj_id, name));
            }
        }
    }

    // 第二遍：分页
    // 关键：图片不能跨页（防止"剩余空间不足时把图撕开"）
    let mut pages: Vec<Vec<u8>> = Vec::new();
    let mut buf: Vec<u8> = Vec::new();
    let mut y = mt;
    // 本页已注册的图片名（按 do 顺序决定）
    let mut page_used_imgs: Vec<&'static str> = Vec::new();

    for blk in &blocks {
        for op in blk {
            let need = match op {
                PageOp::Text { gap, .. } => gap + 4.0,
                PageOp::Image { h_pt, gap, .. } => h_pt + gap + 4.0,
            };
            // 图片必须整页放：剩余空间放不下就换页
            let fits = match op {
                PageOp::Text { .. } => y - need >= mb,
                PageOp::Image { h_pt, .. } => y - h_pt >= mb,
            };
            let must_break = (!fits || y < mb + lh * 3.0) && !pages.is_empty();
            if must_break {
                pages.push(std::mem::take(&mut buf));
                y = mt;
                page_used_imgs.clear();
            }
            match op {
                PageOp::Text { text, font, size, x, .. } => {
                    if !text.is_empty() {
                        emit_text(&mut buf, text, *x, y, *size, font, fid_cjk);
                    }
                },
                PageOp::Image { xobj_id, w_pt, h_pt, .. } => {
                    // 找到该 xobj_id 的名字
                    let name = all_xobjs
                        .iter()
                        .find(|(id, _)| id == xobj_id)
                        .map(|(_, n)| *n)
                        .unwrap_or("Im?");
                    // 决定 x：根据 image_align
                    let x = match image_align {
                        "left" => ml,
                        "right" => mr - w_pt,
                        _ => ml + (tw - w_pt) / 2.0, // center
                    };
                    // PDF 坐标：图片底边 y_pt = y - h_pt（因为 y 是基线）
                    let y_bottom = y - h_pt;
                    // cm 矩阵：a b c d e f → [w_pt 0 0 h_pt x y_bottom]
                    // 这里 cm 矩阵把 1×1 单位矩阵的 XObject 缩放到 w_pt × h_pt，平移到 (x, y_bottom)
                    buf.extend_from_slice(
                        format!(
                            "q\n{} 0 0 {} {} {} cm\n/{} Do\nQ\n",
                            w_pt, h_pt, x, y_bottom, name
                        )
                        .as_bytes(),
                    );
                    page_used_imgs.push(name);
                },
            }
            y -= need;
        }
    }
    pages.push(buf);

    // ── 构建 PDF 对象（每页 Resources 包含本 PDF 全部图片 XObject）──
    // 构造 XObject 字典：{"Im1" => id1, "Im2" => id2, ...}
    let mut xobj_dict = lopdf::Dictionary::new();
    for (id, name) in &all_xobjs {
        xobj_dict.set(*name, Object::Reference(*id));
    }
    // ProcSet 必须包含 Image 才能正常显示（PDF/A 要求；老阅读器更兼容）
    let xobj_dict = if all_xobjs.is_empty() {
        lopdf::Dictionary::new()
    } else {
        xobj_dict
    };

    let mut page_ids: Vec<ObjectId> = Vec::new();
    let total_pages = pages.len();
    let ctx_for_tmpl = crate::templates::TemplateContext {
        title: title.to_string(),
        subtitle: subtitle.to_string(),
        date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        author: author.to_string(),
        page_no: 0,
        total_pages: total_pages as u32,
        items: vec![],
    };
    for (pi, body) in pages.iter().enumerate() {
        let mut content = body.clone();
        // 页眉（顶部居中）— 用户未配置则不画
        if let Ok(header_str) = crate::templates::render_header(&ctx_for_tmpl, header_template)
            && !header_str.trim().is_empty()
        {
            // PDF 字符串字面量需转义 \ ( ) 三种字符
            let escaped = pdf_escape_string(&header_str);
            content.extend_from_slice(
                format!(
                    "BT\n/F1 7 Tf\n1 0 0 1 {} {} Tm\n({}) Tj\nET\n",
                    ml,
                    ph - mt + 8.0,
                    escaped
                )
                .as_bytes(),
            );
        }
        // 页脚（底部居中，模板渲染 + 默认带页码）
        let mut ctx_pn = ctx_for_tmpl.clone();
        ctx_pn.page_no = (pi + 1) as u32;
        let footer_str = crate::templates::render_footer(&ctx_pn, footer_template)
            .unwrap_or_else(|_| format!("{} / {}", pi + 1, total_pages));
        if !footer_str.trim().is_empty() {
            let escaped = pdf_escape_string(&footer_str);
            content.extend_from_slice(
                format!("BT\n/F1 7 Tf\n1 0 0 1 {} 28 Tm\n({}) Tj\nET\n", ml, escaped).as_bytes(),
            );
        }
        let sid = pdf.add_object(Stream::new(lopdf::Dictionary::new(), content));
        let pid = pdf.new_object_id();
        let mut resources = dictionary! { "Font" => font_res.clone() };
        if !all_xobjs.is_empty() {
            resources.set("XObject", Object::Dictionary(xobj_dict.clone()));
        }
        pdf.objects.insert(pid, Object::Dictionary(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(pw as f32), Object::Real(ph as f32)],
            "Contents" => vec![sid.into()],
            "Resources" => resources,
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
    pdf.objects
        .insert(cat_id, Object::Dictionary(dictionary! { "Type" => "Catalog", "Pages" => pt_id }));
    pdf.trailer.set("Root", Object::Reference(cat_id));

    pdf.save(output_path).map(|_| ()).map_err(|e| format!("保存 PDF 失败: {}", e))
}

/// 读取图片文件，解码后编码为 JPEG，再注册为 PDF Image XObject。
/// 返回 (XObject ObjectId, pt 宽度, pt 高度)。
fn load_and_register_image(
    pdf: &mut lopdf::Document,
    path: &str,
    max_width_pt: f64,
) -> Option<(lopdf::ObjectId, f64, f64)> {
    use std::io::Cursor;
    let bytes = std::fs::read(path).ok()?;
    // 用 guess_format 直接读字节头，不依赖 ImageReader 的 format() 返回
    let format = image::guess_format(&bytes).ok()?;
    let (w_px, h_px, jpeg_bytes) = match format {
        image::ImageFormat::Jpeg => {
            // JPEG fast path：直接拿原始字节，不重新解码
            let reader = image::ImageReader::new(Cursor::new(&bytes)).with_guessed_format().ok()?;
            let (w, h) = reader.into_dimensions().ok()?;
            (w, h, bytes)
        },
        _ => {
            // PNG / WebP / GIF / BMP / TIFF：解码 → 编码为 JPEG
            let reader = image::ImageReader::new(Cursor::new(&bytes)).with_guessed_format().ok()?;
            let decoded = reader.decode().ok()?;
            let (w, h) = (decoded.width(), decoded.height());
            let rgb = decoded.to_rgb8();
            let mut jpeg = Vec::new();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 88);
            encoder
                .encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)
                .ok()?;
            (w, h, jpeg)
        },
    };
    if w_px == 0 || h_px == 0 {
        return None;
    }
    let w_pt = w_px as f64 * 0.75;
    let h_pt = h_px as f64 * 0.75;
    let (w_pt, h_pt) = if w_pt <= max_width_pt {
        (w_pt, h_pt)
    } else {
        let scale = max_width_pt / w_pt;
        (max_width_pt, h_pt * scale)
    };

    // 注册 XObject：DCTDecode 直接接收 JPEG 字节流
    let stream = lopdf::Stream::new(
        lopdf::Dictionary::from_iter(vec![
            ("Type", lopdf::Object::Name(b"XObject".to_vec())),
            ("Subtype", lopdf::Object::Name(b"Image".to_vec())),
            ("Width", lopdf::Object::Integer(w_px as i64)),
            ("Height", lopdf::Object::Integer(h_px as i64)),
            ("ColorSpace", lopdf::Object::Name(b"DeviceRGB".to_vec())),
            ("BitsPerComponent", lopdf::Object::Integer(8)),
            ("Filter", lopdf::Object::Name(b"DCTDecode".to_vec())),
        ]),
        jpeg_bytes,
    );
    let xobj_id = pdf.add_object(stream);
    Some((xobj_id, w_pt, h_pt))
}

/// PDF 字符串字面量转义：\ ( ) 三种字符必须转义，否则 PDF 解析失败
fn pdf_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            _ => out.push(ch),
        }
    }
    out
}

/// 测量字符串宽度（pt）。CJK 字符走真实度量，拉丁字符按 0.5em 估算。
fn measure_text(text: &str, size_pt: f64, cjk: Option<&crate::cjk_font::CjkFont>) -> f64 {
    if let Some(cjk) = cjk {
        let mut total = 0.0;
        for c in text.chars() {
            if crate::cjk_font::needs_cid_font(c) {
                total += cjk.measure(&c.to_string(), size_pt);
            } else {
                total += size_pt * 0.5;
            }
        }
        total
    } else {
        text.chars().count() as f64 * size_pt * 0.5
    }
}

/// 按真实宽度换行（用 cjk_font 度量 CJK 字符）。
fn wrap_lines_measured(
    text: &str,
    max_w_pt: f64,
    size_pt: f64,
    cjk: Option<&crate::cjk_font::CjkFont>,
) -> Vec<String> {
    let measure = |s: &str| measure_text(s, size_pt, cjk);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        if measure(&cur) + measure(word) > max_w_pt && !cur.is_empty() {
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

/// 块是否包含 CJK 字符（仅检查粗粒度文本）。
fn block_needs_cjk(block: &markdown::MdBlock) -> bool {
    match block {
        markdown::MdBlock::Heading { text, .. } => crate::cjk_font::needs_cjk_font(text),
        markdown::MdBlock::Paragraph { inlines } => {
            inlines.iter().any(|i| crate::cjk_font::needs_cjk_font(&inline_to_text(i)))
        },
        markdown::MdBlock::CodeBlock { code, .. } => crate::cjk_font::needs_cjk_font(code),
        markdown::MdBlock::Blockquote { inlines } => {
            inlines.iter().any(|i| crate::cjk_font::needs_cjk_font(&inline_to_text(i)))
        },
        markdown::MdBlock::List { items, .. } => items
            .iter()
            .any(|item| item.iter().any(|i| crate::cjk_font::needs_cjk_font(&inline_to_text(i)))),
        markdown::MdBlock::Table { headers, rows } => {
            headers.iter().any(|h| crate::cjk_font::needs_cjk_font(h))
                || rows.iter().any(|r| r.iter().any(|c| crate::cjk_font::needs_cjk_font(c)))
        },
        markdown::MdBlock::Image { alt, .. } => crate::cjk_font::needs_cjk_font(alt),
        markdown::MdBlock::HorizontalRule => false,
    }
}

fn inline_to_text(i: &markdown::MdInline) -> String {
    match i {
        markdown::MdInline::Text(s)
        | markdown::MdInline::Bold(s)
        | markdown::MdInline::Italic(s)
        | markdown::MdInline::Code(s) => s.clone(),
        markdown::MdInline::Link { text, .. } => text.clone(),
        markdown::MdInline::Image { alt, .. } => alt.clone(),
        markdown::MdInline::Math { latex, .. } => {
            let segs = crate::math::parse_latex(latex);
            crate::math::segments_to_plain(&segs)
        },
    }
}

/// 写文本到 PDF 内容流。按字符切 Latin/CJK 段，分别用 base_font / F4 输出。
/// `base_font` 形如 "F1"（用于拉丁段），CJK 段固定用 F4。
/// `fid_cjk` 为 None 时全部走 base_font（CJK 会渲染成乱码，但不会崩）。
fn emit_text(
    buf: &mut Vec<u8>,
    text: &str,
    x: f64,
    y: f64,
    size: f64,
    base_font: &str,
    fid_cjk: Option<lopdf::ObjectId>,
) {
    if text.is_empty() {
        return;
    }
    if fid_cjk.is_none() {
        // 纯拉丁快速路径
        let escaped = text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
        buf.extend_from_slice(
            format!("/{} {} Tf\n1 0 0 1 {} {} Tm\n({}) Tj\n", base_font, size, x, y, escaped)
                .as_bytes(),
        );
        return;
    }
    // 混合输出：按字符切 Latin/CID 段（CID 段使用复合字体渲染）
    let cjk_ref = crate::cjk_font::cjk_font().expect("fid_cjk.is_some() 必有 cjk_font");
    let mut current = String::new();
    let mut current_needs_cid = false;
    let mut cursor_x = x;
    for c in text.chars() {
        let needs_cid = crate::cjk_font::needs_cid_font(c);
        if current.is_empty() {
            current_needs_cid = needs_cid;
            current.push(c);
            continue;
        }
        if needs_cid == current_needs_cid {
            current.push(c);
        } else {
            // flush current
            cursor_x += write_text_segment(
                buf,
                &current,
                cursor_x,
                y,
                size,
                base_font,
                current_needs_cid,
                cjk_ref,
            );
            current.clear();
            current.push(c);
            current_needs_cid = needs_cid;
        }
    }
    if !current.is_empty() {
        let _ = write_text_segment(
            buf,
            &current,
            cursor_x,
            y,
            size,
            base_font,
            current_needs_cid,
            cjk_ref,
        );
    }
}

/// 写一段（同种字符：Latin 或 CJK），返回该段宽度（pt）以让调用方累加 cursor_x。
#[allow(clippy::too_many_arguments)]
fn write_text_segment(
    buf: &mut Vec<u8>,
    seg: &str,
    x: f64,
    y: f64,
    size: f64,
    base_font: &str,
    is_cjk: bool,
    cjk: &crate::cjk_font::CjkFont,
) -> f64 {
    let advance = if is_cjk {
        cjk.measure(seg, size)
    } else {
        seg.chars().count() as f64 * size * 0.5
    };
    if is_cjk {
        let hex = cjk.encode_cid_hex(seg);
        buf.extend_from_slice(
            format!("/F4 {} Tf\n1 0 0 1 {:.2} {:.2} Tm\n<{}> Tj\n", size, x, y, hex).as_bytes(),
        );
    } else {
        let escaped = seg.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
        buf.extend_from_slice(
            format!("/{} {} Tf\n1 0 0 1 {:.2} {:.2} Tm\n({}) Tj\n", base_font, size, x, y, escaped)
                .as_bytes(),
        );
    }
    advance
}

/// 在 PDF 文档中注册 CIDFontType2 复合字体（Identity-H 编码），返回 Type0 字体 ObjectId。
///
/// `used_glyphs` 为本文档实际使用的 `<GID, Unicode 字符>` 映射（已去重，按 GID 升序）。
/// 因 `CIDToGIDMap = Identity`，内容流里的 CID 即 GID，故 ToUnicode 必须显式把每个 GID
/// 映射回真实 Unicode，否则 `pdf-extract` / 阅读器会把 GID 当作 Unicode 而提取出乱码。
fn register_cjk_font(
    pdf: &mut lopdf::Document,
    cjk: &crate::cjk_font::CjkFont,
    used_glyphs: &BTreeMap<u16, char>,
) -> lopdf::ObjectId {
    use lopdf::{Dictionary, Object, Stream, dictionary};

    // 1. FontFile2 stream — TTF 原始字节
    let font_file_id = pdf.add_object(Stream::new(Dictionary::new(), cjk.bytes().to_vec()));

    // 2. 度量（归一化到 1/1000 em，符合 PDF 惯例）
    let upem = cjk.units_per_em() as f64;
    let to_pdf_units = |v: f64| v * 1000.0 / upem;
    let ascent = to_pdf_units(cjk.ascent() as f64);
    let descent = to_pdf_units(cjk.descent() as f64); // 负值
    let cap_height = ascent * 0.7;

    // 3. FontDescriptor
    let fd_id = pdf.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => Object::Name(b"AxAgentCJK".to_vec()),
        "Flags" => 4,
        "FontBBox" => vec![
            Object::Real(0.0),
            Object::Real(descent as f32),
            Object::Real(1000.0),
            Object::Real(ascent as f32),
        ],
        "ItalicAngle" => 0,
        "Ascent" => ascent,
        "Descent" => descent,
        "CapHeight" => cap_height,
        "StemV" => 80,
        "FontFile2" => font_file_id,
    });

    // 4. CIDFont (CIDFontType2 = TrueType)
    let cid_id = pdf.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType2",
        "BaseFont" => Object::Name(b"AxAgentCJK".to_vec()),
        "CIDSystemInfo" => dictionary! {
            "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"),
            "Supplement" => 0,
        },
        "FontDescriptor" => fd_id,
        "CIDToGIDMap" => Object::Name(b"Identity".to_vec()),
        "DW" => 1000,
    });

    // 5. ToUnicode CMap：GID → Unicode（逐字符 bfchar），保证文本可提取 / 搜索 / 复制
    let mut bfchar = String::new();
    for (&gid, &ch) in used_glyphs {
        bfchar.push_str(&format!("<{:04X}> <{}>\n", gid, char_to_utf16_hex(ch)));
    }
    let cmap_text = format!(
        "\
/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /AxAgent-CJK-ToUnicode def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<0000> <FFFF>\n\
endcodespacerange\n\
{} beginbfchar\n\
{}\
endbfchar\n\
endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n",
        used_glyphs.len(),
        bfchar,
    );
    let cmap_id = pdf.add_object(Stream::new(
        dictionary! { "Length" => cmap_text.len() as i64 },
        cmap_text.as_bytes().to_vec(),
    ));

    // 6. Type0 复合字体
    pdf.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => Object::Name(b"AxAgentCJK".to_vec()),
        "Encoding" => Object::Name(b"Identity-H".to_vec()),
        "DescendantFonts" => vec![Object::Reference(cid_id)],
        "ToUnicode" => cmap_id,
    })
}

/// 字符 → UTF-16BE 十六进制（BMP 为 4 位，非 BMP 为 surrogate pair 8 位），用于 ToUnicode CMap 值。
fn char_to_utf16_hex(c: char) -> String {
    let cp = c as u32;
    if cp <= 0xFFFF {
        format!("{:04X}", cp)
    } else {
        let v = cp - 0x10000;
        let hi = 0xD800 + (v >> 10);
        let lo = 0xDC00 + (v & 0x3FF);
        format!("{:04X}{:04X}", hi, lo)
    }
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
            if i == 0 { s } else { format!("  {}", s) }
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
                "sheet_name": {"type": "string", "default": "Sheet1", "description": "工作表名称"},
                "enable_chart": {
                    "type": "boolean",
                    "default": true,
                    "description": "是否在每个表格旁自动生成图表（要求至少一个字符串列 + 一个数字列）"
                },
                "chart_type": {
                    "type": "string",
                    "enum": ["bar", "column", "line", "pie"],
                    "default": "column",
                    "description": "图表类型：bar=横向条形图, column=纵向柱状图, line=折线图, pie=饼图"
                }
            },
            "required": ["markdown", "output_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input.get("markdown").and_then(|v| v.as_str()).unwrap_or_default();
        let output_path = input.get("output_path").and_then(|v| v.as_str()).unwrap_or_default();
        let sheet_name = input.get("sheet_name").and_then(|v| v.as_str()).unwrap_or("Sheet1");
        let enable_chart = input.get("enable_chart").and_then(|v| v.as_bool()).unwrap_or(true);
        let chart_type = input.get("chart_type").and_then(|v| v.as_str()).unwrap_or("column");

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

        match build_xlsx(markdown_text, sheet_name, output_path, enable_chart, chart_type) {
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

fn build_xlsx(
    markdown_text: &str,
    sheet_name: &str,
    output_path: &str,
    enable_chart: bool,
    chart_type: &str,
) -> Result<usize, String> {
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
        #[allow(unused_mut)]
        let mut worksheet = workbook.add_worksheet().set_name(&name).map_err(|e| e.to_string())?;

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
        worksheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;

        // 自动筛选
        worksheet
            .autofilter(0, 0, (num_rows - 1) as u32, num_cols - 1)
            .map_err(|e| e.to_string())?;

        // 自动列宽（基于表头和数据内容）
        for ci in 0..num_cols {
            let mut max_width: f64 =
                headers.get(ci as usize).map(|h| char_width_estimate(h)).unwrap_or(8.0);
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
            worksheet.set_column_width(ci, col_width).map_err(|e| e.to_string())?;
        }

        // 表头行高
        worksheet.set_row_height(0, 24).map_err(|e| e.to_string())?;

        if enable_chart && rows.len() >= 2 {
            insert_chart_for_table(worksheet, &name, headers, rows, chart_type)?;
        }
    }

    workbook.save(output_path).map_err(|e| e.to_string())?;
    Ok(tables.len())
}

/// 给单个表格 sheet 插入图表：
/// - 找到第一个"含非数字值的列"作为类别轴（categories）
/// - 每个"全数字列"作为一个数据系列
/// - 图表位置：数据右侧、距顶 1 行的位置
fn insert_chart_for_table(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    sheet_name: &str,
    headers: &[String],
    rows: &[Vec<String>],
    chart_type: &str,
) -> Result<(), String> {
    use rust_xlsxwriter::{Chart, ChartLegendPosition, ChartType};

    if headers.is_empty() || rows.is_empty() {
        return Ok(());
    }

    // 识别类别列：第一个"至少有一行含非数字"或"全部非数字"的列；否则取第 0 列
    let num_cols = headers.len();
    let category_col: usize = (0..num_cols)
        .find(|&c| {
            rows.iter().any(|row| {
                row.get(c).map(|cell| cell.trim().parse::<f64>().is_err()).unwrap_or(true)
            })
        })
        .unwrap_or(0);

    // 识别值列：每个"全部为数字"的列
    let value_cols: Vec<usize> = (0..num_cols)
        .filter(|&c| {
            c != category_col
                && rows.iter().all(|row| {
                    row.get(c).map(|cell| cell.trim().parse::<f64>().is_ok()).unwrap_or(false)
                })
        })
        .collect();

    if value_cols.is_empty() {
        // 没有数字列，跳过图表
        return Ok(());
    }

    // 把 category_col 转成 Excel 列字母（手动实现，避免引入 col name 工具）
    let col_letter = |idx: usize| -> String {
        let mut s = String::new();
        let mut n = idx;
        loop {
            s.insert(0, (b'A' + (n % 26) as u8) as char);
            if n < 26 {
                break;
            }
            n = n / 26 - 1;
        }
        s
    };

    let cat_letter = col_letter(category_col);
    let last_row = rows.len() as u32; // 0..=last_row 含表头

    // 选择 ChartType
    let ct = match chart_type {
        "bar" => ChartType::Bar,
        "line" => ChartType::Line,
        "pie" => ChartType::Pie,
        _ => ChartType::Column,
    };
    let mut chart = Chart::new(ct);
    // 标题：取工作表名
    chart.title().set_name(sheet_name);
    // 图例放底部
    chart.legend().set_position(ChartLegendPosition::Bottom);

    // 类别范围：=Sheet!$A$2:$A$N
    let cat_range = format!("'{}'!${}$2:${}${}", sheet_name, cat_letter, cat_letter, last_row + 1);

    // 为每个值列添加 series
    for &vc in &value_cols {
        let v_letter = col_letter(vc);
        let val_range = format!("'{}'!${}$2:${}${}", sheet_name, v_letter, v_letter, last_row + 1);
        let header_name = headers.get(vc).cloned().unwrap_or_default();
        let series = chart.add_series();
        series.set_name(&header_name).set_values(&val_range).set_categories(&cat_range);
    }

    // 图表位置：数据右侧、距顶 1 行
    let insert_col = (num_cols as u32 + 1) as u16; // 紧贴数据右
    let insert_row = 1u32; // 距顶 1 行
    worksheet
        .insert_chart(insert_row, insert_col, &chart)
        .map_err(|e| format!("插入图表失败: {}", e))?;
    Ok(())
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
                "title": {"type": "string", "default": "Presentation", "description": "演示文稿标题"},
                "enable_chart": {"type": "boolean", "default": true, "description": "是否将含数字列的 Markdown 表格自动渲染为图表幻灯片"}
            },
            "required": ["markdown", "output_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let markdown_text = input.get("markdown").and_then(|v| v.as_str()).unwrap_or_default();
        let output_path = input.get("output_path").and_then(|v| v.as_str()).unwrap_or_default();
        let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("Presentation");
        let enable_chart = input.get("enable_chart").and_then(|v| v.as_bool()).unwrap_or(true);

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

        match build_pptx(markdown_text, title, output_path, enable_chart) {
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
fn build_pptx(
    markdown_text: &str,
    title: &str,
    output_path: &str,
    enable_chart: bool,
) -> Result<usize, String> {
    use std::io::Write;

    let doc = markdown::parse_markdown(markdown_text);

    // 按标题分割为幻灯片
    let mut slides: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_title = title.to_string();
    let mut current_bullets: Vec<String> = Vec::new();

    // 收集表格用于图表生成
    let mut table_charts: Vec<(usize, Vec<String>, Vec<Vec<String>>)> = Vec::new();

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
            markdown::MdBlock::CodeBlock { language, code } => {
                // Mermaid 流程图：解析 + 渲染为 Unicode 框线字符文本
                let lines: Vec<String> = if language.trim().eq_ignore_ascii_case("mermaid") {
                    let graph = crate::mermaid::parse_mermaid(code);
                    let rendered = crate::mermaid::render_to_text(&graph);
                    rendered.lines().map(|s| s.to_string()).collect()
                } else {
                    code.lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(|s| format!("  {}", s))
                        .collect()
                };
                for line in lines {
                    current_bullets.push(line);
                }
            },
            markdown::MdBlock::Table { headers, rows } => {
                // 表格：推到 bullets 作为文本，同时记录用于图表
                current_bullets.push(format!("表: {}", headers.join(" | ")));
                for row in rows {
                    current_bullets.push(format!("  {}", row.join(" | ")));
                }
                if enable_chart {
                    // 检查是否有数字列
                    if find_numeric_column(headers, rows).is_some() {
                        table_charts.push((slides.len(), headers.clone(), rows.clone()));
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

    // 追加图表幻灯片（每个含数字列的表格一张）
    let mut chart_slide_map: Vec<(usize, Vec<String>, Vec<Vec<String>>)> = Vec::new();
    for (_, headers, rows) in &table_charts {
        let chart_slide_idx = slides.len();
        let chart_title = format!("图表: {}", headers.join(" | "));
        slides.push((chart_title, vec![]));
        chart_slide_map.push((chart_slide_idx, headers.clone(), rows.clone()));
    }

    let slide_count = slides.len();
    let chart_count = chart_slide_map.len();

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
         <Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>\n\
         <Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>\n\
         <Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/>\n\
         <Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/>\n",
    );
    for i in 1..=slide_count {
        content_types.push_str(&format!(
            "<Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>\n\
             <Override PartName=\"/ppt/slides/_rels/slide{}.xml.rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\n",
            i, i
        ));
    }
    // 图表 content type
    for i in 1..=chart_count {
        content_types.push_str(&format!(
            "<Override PartName=\"/ppt/charts/chart{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/>\n",
            i
        ));
    }
    content_types.push_str("</Types>");

    zip_writer.start_file("[Content_Types].xml", zip_options).map_err(|e| e.to_string())?;
    zip_writer.write_all(content_types.as_bytes()).map_err(|e| e.to_string())?;

    // _rels/.rels
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
                <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
                <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/>\n\
                </Relationships>";
    zip_writer.start_file("_rels/.rels", zip_options).map_err(|e| e.to_string())?;
    zip_writer.write_all(rels.as_bytes()).map_err(|e| e.to_string())?;

    // ppt/_rels/presentation.xml.rels — 加 theme + slideMaster + slideLayout
    let mut pres_rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
         <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"theme/theme1.xml\"/>\n\
         <Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"slideMasters/slideMaster1.xml\"/>\n",
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
    zip_writer.write_all(pres_rels.as_bytes()).map_err(|e| e.to_string())?;

    // ppt/presentation.xml
    let mut pres_xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <p:presentation xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"\
         xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\n\
         <p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId2\"/></p:sldMasterIdLst>\n\
         <p:sldIdLst>\n",
    );
    for i in 1..=slide_count {
        pres_xml.push_str(&format!("<p:sldId id=\"{}\" r:id=\"rId{}\"/>\n", i + 255, i + 10));
    }
    pres_xml.push_str("</p:sldIdLst>\n<p:sldSz cx=\"9144000\" cy=\"6858000\"/>\n</p:presentation>");

    zip_writer.start_file("ppt/presentation.xml", zip_options).map_err(|e| e.to_string())?;
    zip_writer.write_all(pres_xml.as_bytes()).map_err(|e| e.to_string())?;

    // ppt/theme/theme1.xml
    zip_writer.start_file("ppt/theme/theme1.xml", zip_options).map_err(|e| e.to_string())?;
    zip_writer.write_all(make_pptx_theme_xml().as_bytes()).map_err(|e| e.to_string())?;

    // ppt/slideMasters/slideMaster1.xml
    zip_writer
        .start_file("ppt/slideMasters/slideMaster1.xml", zip_options)
        .map_err(|e| e.to_string())?;
    zip_writer.write_all(make_pptx_slide_master_xml().as_bytes()).map_err(|e| e.to_string())?;

    // ppt/slideMasters/_rels/slideMaster1.xml.rels — 关联 theme1
    zip_writer
        .start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", zip_options)
        .map_err(|e| e.to_string())?;
    zip_writer.write_all(MASTER_RELS_XML.as_bytes()).map_err(|e| e.to_string())?;

    // ppt/slideLayouts/slideLayout1.xml
    zip_writer
        .start_file("ppt/slideLayouts/slideLayout1.xml", zip_options)
        .map_err(|e| e.to_string())?;
    zip_writer.write_all(make_pptx_slide_layout_xml().as_bytes()).map_err(|e| e.to_string())?;

    // ppt/slideLayouts/_rels/slideLayout1.xml.rels — 关联 slideMaster1
    zip_writer
        .start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", zip_options)
        .map_err(|e| e.to_string())?;
    zip_writer.write_all(LAYOUT_RELS_XML.as_bytes()).map_err(|e| e.to_string())?;

    // 每张幻灯片 — 专业布局
    for (si, (slide_title, bullets)) in slides.iter().enumerate() {
        let slide_num = si + 1;
        let escaped_title = xml_escape(slide_title);

        // 检查是否为图表幻灯片
        let chart_info = chart_slide_map
            .iter()
            .enumerate()
            .find(|(_, (idx, _, _))| *idx == si)
            .map(|(ci, (_, h, r))| (ci + 1, h, r));

        // 标题: 顶部居中, 大号深蓝粗体, 底部带金色装饰线
        let mut slide_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
             <p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"\
             xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"\
             xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"\n\
             xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\">\n\
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

        if let Some((chart_num, _, _)) = chart_info {
            // 图表幻灯片：嵌入 graphicFrame 引用 chart{chart_num}.xml
            slide_xml.push_str(&format!(
                "<p:graphicFrame><p:nvGraphicFramePr>\
                 <p:cNvPr id=\"200\" name=\"Chart{}\"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp=\"1\"/></p:cNvGraphicFramePr>\
                 <p:nvPr/></p:nvGraphicFramePr>\
                 <p:xfrm><a:off x=\"685800\" y=\"1554400\"/><a:ext cx=\"7772400\" cy=\"4572000\"/></a:xfrm>\
                 <a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/chart\">\
                 <c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" r:id=\"rIdChart{}\"/>\
                 </a:graphicData></a:graphic></p:graphicFrame>\n",
                chart_num, chart_num
            ));
        } else {
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
        zip_writer.start_file(&slide_path, zip_options).map_err(|e| e.to_string())?;
        zip_writer.write_all(slide_xml.as_bytes()).map_err(|e| e.to_string())?;

        // slide{N}.xml.rels — 关联 slideLayout1（图表幻灯片额外关联 chart）
        let slide_rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);
        zip_writer.start_file(&slide_rels_path, zip_options).map_err(|e| e.to_string())?;
        if let Some((chart_num, _, _)) = chart_info {
            let chart_rels = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
                 <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
                 <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/>\n\
                 <Relationship Id=\"rIdChart{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart\" Target=\"../charts/chart{}.xml\"/>\n\
                 </Relationships>",
                chart_num, chart_num
            );
            zip_writer.write_all(chart_rels.as_bytes()).map_err(|e| e.to_string())?;
        } else {
            zip_writer.write_all(SLIDE_RELS_XML.as_bytes()).map_err(|e| e.to_string())?;
        }
    }

    // 写入图表 XML 文件
    for (ci, (_, headers, rows)) in chart_slide_map.iter().enumerate() {
        let chart_num = ci + 1;
        let chart_xml = make_pptx_chart_xml(chart_num, headers, rows);
        let chart_path = format!("ppt/charts/chart{}.xml", chart_num);
        zip_writer.start_file(&chart_path, zip_options).map_err(|e| e.to_string())?;
        zip_writer.write_all(chart_xml.as_bytes()).map_err(|e| e.to_string())?;
    }

    zip_writer.finish().map_err(|e| e.to_string())?;
    Ok(slide_count)
}

// ── PPTX 主题 / 母版 / 版式 ─────────────────────────────────────────────────

/// slideMaster1.xml.rels：关联 theme1
const MASTER_RELS_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"../theme/theme1.xml\"/>\n\
<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/>\n\
</Relationships>";

/// slideLayout1.xml.rels：关联 slideMaster1
const LAYOUT_RELS_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"../slideMasters/slideMaster1.xml\"/>\n\
</Relationships>";

/// slide{N}.xml.rels：关联 slideLayout1
const SLIDE_RELS_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\n\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/>\n\
</Relationships>";

/// 查找表格中第一个"全数字"列的索引（用于图表值轴）
fn find_numeric_column(headers: &[String], rows: &[Vec<String>]) -> Option<usize> {
    if rows.is_empty() || headers.is_empty() {
        return None;
    }
    for col_idx in 0..headers.len() {
        let all_numeric = rows
            .iter()
            .all(|row| col_idx < row.len() && row[col_idx].trim().parse::<f64>().is_ok());
        if all_numeric {
            return Some(col_idx);
        }
    }
    None
}

/// 生成 DOCX Unicode 块字符条形图段落（无需 DrawingML Chart，纯文本可视化）
///
/// 每行格式：`类别  ████████  数值`，最大值对应 30 个 █，其他按比例。
/// 第一个非数字列作为类别列；val_col 指定的列作为值列。
fn make_docx_chart_paragraphs(
    headers: &[String],
    rows: &[Vec<String>],
    val_col: usize,
) -> Vec<docx_rs::Paragraph> {
    use docx_rs::*;

    let mut paras: Vec<Paragraph> = Vec::new();

    // 图表标题
    let chart_title = if val_col < headers.len() {
        format!("图: {}", headers[val_col])
    } else {
        "图".to_string()
    };
    paras.push(
        Paragraph::new()
            .add_run(Run::new().add_text(&chart_title).size(22).bold().color("1F3864"))
            .line_spacing(
                LineSpacing::new().before(160).after(80).line_rule(LineSpacingType::Auto).line(276),
            ),
    );

    // 类别列：第一个非 val_col 的列；若没有则用行号
    let cat_col = (0..headers.len()).find(|&i| i != val_col).unwrap_or(0);

    // 最大值（绝对值，用于归一化）
    let max_abs = rows
        .iter()
        .filter_map(|r| {
            if val_col < r.len() {
                r[val_col].trim().parse::<f64>().ok()
            } else {
                None
            }
        })
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max);

    if max_abs <= 0.0 {
        // 全为 0 或解析失败：仅输出文字
        for (i, row) in rows.iter().enumerate() {
            let cat = if cat_col < row.len() {
                row[cat_col].clone()
            } else {
                format!("#{}", i + 1)
            };
            let val = if val_col < row.len() {
                row[val_col].clone()
            } else {
                "0".to_string()
            };
            paras.push(
                Paragraph::new()
                    .add_run(
                        Run::new().add_text(format!("{}  {}", cat, val)).size(20).color("333333"),
                    )
                    .line_spacing(LineSpacing::new().line_rule(LineSpacingType::Auto).line(240)),
            );
        }
        return paras;
    }

    const MAX_BLOCKS: usize = 30;
    for (i, row) in rows.iter().enumerate() {
        let cat = if cat_col < row.len() {
            row[cat_col].clone()
        } else {
            format!("#{}", i + 1)
        };
        let val: f64 = if val_col < row.len() {
            row[val_col].trim().parse().unwrap_or(0.0)
        } else {
            0.0
        };

        // 块数 = round(|val| / max_abs * MAX_BLOCKS)，至少 1（非零值）
        let blocks = if val.abs() < 1e-9 {
            0
        } else {
            ((val.abs() / max_abs) * MAX_BLOCKS as f64).round() as usize
        };
        let bar: String = "█".repeat(blocks.max(1));

        paras.push(
            Paragraph::new()
                .add_run(Run::new().add_text(format!("{:<10}", cat)).size(20).color("333333"))
                .add_run(Run::new().add_text(format!(" {} ", bar)).size(20).color("2E75B6"))
                .add_run(Run::new().add_text(format!("{}", val)).size(20).color("333333"))
                .line_spacing(LineSpacing::new().line_rule(LineSpacingType::Auto).line(240)),
        );
    }

    paras
}

/// 生成 PPTX DrawingML Chart XML（柱状图）
fn make_pptx_chart_xml(chart_num: usize, headers: &[String], rows: &[Vec<String>]) -> String {
    // 找类别列（第一个非数字列）和值列（第一个全数字列）
    let cat_col = headers
        .iter()
        .position(|_| true)
        .filter(|&i| !rows.iter().all(|r| i < r.len() && r[i].trim().parse::<f64>().is_ok()))
        .unwrap_or(0);

    let val_col = find_numeric_column(headers, rows).unwrap_or(if cat_col == 0 { 1 } else { 0 });

    let n = rows.len();
    let mut cat_pts = String::new();
    let mut val_pts = String::new();
    for (i, row) in rows.iter().enumerate() {
        let cat = if cat_col < row.len() {
            xml_escape(&row[cat_col])
        } else {
            String::new()
        };
        let val: f64 = if val_col < row.len() {
            row[val_col].trim().parse().unwrap_or(0.0)
        } else {
            0.0
        };
        cat_pts.push_str(&format!("<c:pt idx=\"{}\"><c:v>{}</c:v></c:pt>\n", i, cat));
        val_pts.push_str(&format!("<c:pt idx=\"{}\"><c:v>{}</c:v></c:pt>\n", i, val));
    }

    let title = if val_col < headers.len() {
        xml_escape(&headers[val_col])
    } else {
        format!("Chart {}", chart_num)
    };

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\"\n\
         xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"\n\
         xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\n\
         <c:chart>\n\
         <c:title><c:tx><c:rich>\n\
         <a:bodyPr/><a:lstStyle/><a:p><a:pPr algn=\"ctr\"/>\n\
         <a:r><a:rPr lang=\"zh-CN\" sz=\"1400\" b=\"1\"><a:solidFill><a:srgbClr val=\"1F3864\"/></a:solidFill></a:rPr>\n\
         <a:t>{}</a:t></a:r></a:p></c:rich></c:tx></c:title>\n\
         <c:plotArea><c:layout/>\n\
         <c:barChart>\n\
         <c:barDir val=\"col\"/>\n\
         <c:grouping val=\"clustered\"/>\n\
         <c:varyColors val=\"1\"/>\n\
         <c:ser>\n\
         <c:idx val=\"0\"/>\n\
         <c:order val=\"0\"/>\n\
         <c:tx><c:v>{}</c:v></c:tx>\n\
         <c:spPr><a:solidFill><a:srgbClr val=\"2E75B6\"/></a:solidFill></c:spPr>\n\
         <c:cat>\n\
         <c:strRef>\n\
         <c:f>Sheet1!$A$2:$A${}</c:f>\n\
         <c:strCache><c:ptCount val=\"{}\"/>\n{}</c:strCache>\n\
         </c:strRef>\n\
         </c:cat>\n\
         <c:val>\n\
         <c:numRef>\n\
         <c:f>Sheet1!$B$2:$B${}</c:f>\n\
         <c:numCache><c:formatCode>General</c:formatCode><c:ptCount val=\"{}\"/>\n{}</c:numCache>\n\
         </c:numRef>\n\
         </c:val>\n\
         </c:ser>\n\
         <c:axId val=\"1\"/>\n\
         <c:axId val=\"2\"/>\n\
         </c:barChart>\n\
         <c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling>\n\
         <c:delete val=\"0\"/><c:axPos val=\"b\"/><c:crossAx val=\"2\"/></c:catAx>\n\
         <c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling>\n\
         <c:delete val=\"0\"/><c:axPos val=\"l\"/><c:crossAx val=\"1\"/></c:valAx>\n\
         </c:plotArea>\n\
         </c:chart>\n\
         </c:chartSpace>",
        title,
        title,
        n + 1,
        n,
        cat_pts,
        n + 1,
        n,
        val_pts
    )
}

/// theme1.xml — 商务蓝主题（与现有 build_pptx 颜色一致）
fn make_pptx_theme_xml() -> String {
    // fillStyle / lnStyle / effectStyle / bgFillStyle 各列出 3 项（OOXML 要求最少 3 项）
    let fill_style_lst = "\
<a:fillStyleLst>\
<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
<a:solidFill><a:schemeClr val=\"phClr\"><a:alpha val=\"40000\"/></a:schemeClr></a:solidFill>\
<a:solidFill><a:schemeClr val=\"phClr\"><a:alpha val=\"10000\"/></a:schemeClr></a:solidFill>\
</a:fillStyleLst>";

    let ln_style_lst = "\
<a:lnStyleLst>\
<a:ln w=\"6350\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>\
<a:ln w=\"12700\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>\
<a:ln w=\"19050\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>\
</a:lnStyleLst>";

    let effect_style_lst = "\
<a:effectStyleLst>\
<a:effectLst/>\
<a:effectLst/>\
<a:effectLst><a:outerShdw blurRad=\"40000\" dist=\"20000\" dir=\"5400000\" algn=\"ctr\" rotWithShape=\"0\"><a:srgbClr val=\"000000\"><a:alpha val=\"38000\"/></a:srgbClr></a:outerShdw></a:effectLst>\
</a:effectStyleLst>";

    let bg_fill_style_lst = "\
<a:bgFillStyleLst>\
<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
<a:solidFill><a:schemeClr val=\"phClr\"><a:tint val=\"95000\"/><a:satMod val=\"170000\"/></a:schemeClr></a:solidFill>\
<a:gradFill rotWithShape=\"1\"><a:gsLst><a:gs pos=\"0\"><a:schemeClr val=\"phClr\"><a:tint val=\"93000\"/><a:satMod val=\"150000\"/><a:shade val=\"98000\"/><a:alpha val=\"50000\"/></a:schemeClr></a:gs><a:gs pos=\"50000\"><a:schemeClr val=\"phClr\"><a:tint val=\"98000\"/><a:satMod val=\"130000\"/><a:shade val=\"90000\"/><a:alpha val=\"30000\"/></a:schemeClr></a:gs><a:gs pos=\"100000\"><a:schemeClr val=\"phClr\"><a:shade val=\"63000\"/><a:satMod val=\"120000\"/></a:schemeClr></a:gs></a:gsLst><a:lin ang=\"5400000\" scaled=\"0\"/></a:gradFill>\
</a:bgFillStyleLst>";

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"AxAgent 主题\">\
<a:themeElements>\
<a:clrScheme name=\"AxAgent\">\
<a:dk1><a:sysClr val=\"windowText\" lastClr=\"000000\"/></a:dk1>\
<a:lt1><a:sysClr val=\"window\" lastClr=\"FFFFFF\"/></a:lt1>\
<a:dk2><a:srgbClr val=\"1F3864\"/></a:dk2>\
<a:lt2><a:srgbClr val=\"E7E6E6\"/></a:lt2>\
<a:accent1><a:srgbClr val=\"1F3864\"/></a:accent1>\
<a:accent2><a:srgbClr val=\"2E75B6\"/></a:accent2>\
<a:accent3><a:srgbClr val=\"9E480E\"/></a:accent3>\
<a:accent4><a:srgbClr val=\"636363\"/></a:accent4>\
<a:accent5><a:srgbClr val=\"997300\"/></a:accent5>\
<a:accent6><a:srgbClr val=\"255E91\"/></a:accent6>\
<a:hlink><a:srgbClr val=\"0563C1\"/></a:hlink>\
<a:folHlink><a:srgbClr val=\"954F72\"/></a:folHlink>\
</a:clrScheme>\
<a:fontScheme name=\"AxAgent\">\
<a:majorFont><a:latin typeface=\"Calibri Light\" panose=\"020F0302020204030204\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont>\
<a:minorFont><a:latin typeface=\"Calibri\" panose=\"020F0502020204030204\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont>\
</a:fontScheme>\
<a:fmtScheme name=\"AxAgent\">\
{fill_style_lst}\
{ln_style_lst}\
{effect_style_lst}\
{bg_fill_style_lst}\
</a:fmtScheme>\
</a:themeElements>\
</a:theme>",
        fill_style_lst = fill_style_lst,
        ln_style_lst = ln_style_lst,
        effect_style_lst = effect_style_lst,
        bg_fill_style_lst = bg_fill_style_lst,
    )
}

/// slideMaster1.xml — 母版：占位符定义、背景、文本样式
fn make_pptx_slide_master_xml() -> String {
    // 注：txStyles 给出 titleStyle / bodyStyle / otherStyle 三个默认文本样式
    // bodyStyle 用 lvl1pPr 含项目符号样式（lvl1/2/3 都有）
    let body_style = "\
<p:bodyStyle>\
<a:lvl1pPr marL=\"342900\" indent=\"-342900\" algn=\"t\"><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr><a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"•\"/></a:lvl1pPr>\
<a:lvl2pPr marL=\"742950\" indent=\"-285750\" algn=\"t\"><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr><a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"•\"/></a:lvl2pPr>\
<a:lvl3pPr marL=\"1143000\" indent=\"-228600\" algn=\"t\"><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr><a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"•\"/></a:lvl3pPr>\
<a:lvl4pPr marL=\"1600200\" indent=\"-228600\" algn=\"t\"><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr><a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"•\"/></a:lvl4pPr>\
<a:lvl5pPr marL=\"2057400\" indent=\"-228600\" algn=\"t\"><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr><a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"•\"/></a:lvl5pPr>\
<a:lvl6pPr marL=\"2514600\" indent=\"-228600\" algn=\"t\"><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr><a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"•\"/></a:lvl6pPr>\
<a:lvl7pPr marL=\"2971800\" indent=\"-228600\" algn=\"t\"><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr><a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"•\"/></a:lvl7pPr>\
<a:lvl8pPr marL=\"3429000\" indent=\"-228600\" algn=\"t\"><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr><a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"•\"/></a:lvl8pPr>\
<a:lvl9pPr marL=\"3886200\" indent=\"-228600\" algn=\"t\"><a:buClr><a:srgbClr val=\"2E75B6\"/></a:buClr><a:buFont typeface=\"Arial\" panose=\"020B0604020202020204\" pitchFamily=\"34\" charset=\"0\"/><a:buChar char=\"•\"/></a:lvl9pPr>\
</p:bodyStyle>";

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<p:sldMaster xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"\
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"\
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">\
<p:cSld>\
<p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"lt1\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>\
<p:spTree>\
<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
<p:sp>\
<p:nvSpPr><p:cNvPr id=\"2\" name=\"Title Placeholder\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr>\
<p:spPr><a:xfrm><a:off x=\"685800\" y=\"274320\"/><a:ext cx=\"7772400\" cy=\"1142640\"/></a:xfrm></p:spPr>\
<p:txBody><a:bodyPr/><a:lstStyle/>\
<a:p><a:r><a:rPr lang=\"zh-CN\" sz=\"3200\" b=\"1\"><a:solidFill><a:srgbClr val=\"1F3864\"/></a:solidFill></a:rPr><a:t></a:t></a:r></a:p>\
</p:txBody>\
</p:sp>\
<p:sp>\
<p:nvSpPr><p:cNvPr id=\"3\" name=\"Body Placeholder\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr>\
<p:spPr><a:xfrm><a:off x=\"685800\" y=\"1428480\"/><a:ext cx=\"7772400\" cy=\"4851360\"/></a:xfrm></p:spPr>\
<p:txBody><a:bodyPr/><a:lstStyle/>\
<a:p><a:r><a:rPr lang=\"zh-CN\" sz=\"2000\"><a:solidFill><a:srgbClr val=\"333333\"/></a:solidFill></a:rPr><a:t></a:t></a:r></a:p>\
</p:txBody>\
</p:sp>\
<p:sp>\
<p:nvSpPr><p:cNvPr id=\"4\" name=\"Slide Number\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"sldNum\" sz=\"quarter\" idx=\"2\"/></p:nvPr></p:nvSpPr>\
<p:spPr><a:xfrm><a:off x=\"8229600\" y=\"6400800\"/><a:ext cx=\"685800\" cy=\"365760\"/></a:xfrm></p:spPr>\
<p:txBody><a:bodyPr/><a:lstStyle/>\
<a:p><a:pPr algn=\"r\"/><a:fld id=\"{{00000000-0000-0000-0000-000000000000}}\" type=\"slidenum\"><a:rPr lang=\"en-US\" sz=\"1200\"><a:solidFill><a:srgbClr val=\"999999\"/></a:solidFill></a:rPr><a:t>‹#›</a:t></a:fld></a:p>\
</p:txBody>\
</p:sp>\
</p:spTree>\
</p:cSld>\
<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>\
<p:sldLayoutIdLst><p:sldLayoutId id=\"1\" r:id=\"rId2\"/></p:sldLayoutIdLst>\
<p:txStyles>\
<p:titleStyle>\
<a:lvl1pPr algn=\"l\" defTabSz=\"914400\"><a:defRPr lang=\"zh-CN\" sz=\"3200\" b=\"1\"><a:solidFill><a:srgbClr val=\"1F3864\"/></a:solidFill><a:latin typeface=\"Calibri Light\"/></a:defRPr></a:lvl1pPr>\
</p:titleStyle>\
{body_style}\
<p:otherStyle><a:lvl1pPr><a:defRPr lang=\"zh-CN\" sz=\"1800\"/></a:lvl1pPr></p:otherStyle>\
</p:txStyles>\
</p:sldMaster>",
        body_style = body_style
    )
}

/// slideLayout1.xml — Title Slide 版式（标题 + 内容）
fn make_pptx_slide_layout_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<p:sldLayout xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"\
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"\
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" type=\"title\" preserve=\"1\">\
<p:cSld name=\"AxAgent 标题幻灯片\">\
<p:spTree>\
<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
<p:sp>\
<p:nvSpPr><p:cNvPr id=\"2\" name=\"Title\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr>\
<p:spPr/>\
<p:txBody><a:bodyPr/><a:lstStyle/>\
<a:p><a:r><a:rPr lang=\"zh-CN\"/><a:t></a:t></a:r></a:p>\
</p:txBody>\
</p:sp>\
<p:sp>\
<p:nvSpPr><p:cNvPr id=\"3\" name=\"Content\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph idx=\"1\"/></p:nvPr></p:nvSpPr>\
<p:spPr/>\
<p:txBody><a:bodyPr/><a:lstStyle/>\
<a:p><a:r><a:rPr lang=\"zh-CN\"/><a:t></a:t></a:r></a:p>\
</p:txBody>\
</p:sp>\
</p:spTree>\
</p:cSld>\
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>\
</p:sldLayout>"
        .to_string()
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
            // 数学公式：LaTeX → Unicode 文本
            markdown::MdInline::Math { latex, .. } => {
                let segs = crate::math::parse_latex(latex);
                crate::math::segments_to_plain(&segs)
            },
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input.get("path").and_then(|v| v.as_str()).unwrap_or_default();

        if file_path.is_empty() {
            return Ok(ToolResult::error("Error: path 是必需的"));
        }
        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", file_path)));
        }

        match crate::parser::parser()
            .extract_text(path, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        {
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

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let file_path = input.get("path").and_then(|v| v.as_str()).unwrap_or_default();

        if file_path.is_empty() {
            return Ok(ToolResult::error("Error: path 是必需的"));
        }
        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", file_path)));
        }

        match crate::parser::parser().extract_text(
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

// ── 集成测试：ExportPdf 端到端生成含中文 PDF + pdf-extract 反向解析验证 ──────

#[cfg(test)]
mod cjk_pdf_integration {
    use super::*;
    use std::path::PathBuf;

    fn tmp_out(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("axagent-cjk-pdf-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        dir.join(name)
    }

    /// Windows + msyh.ttc 系统字体可用时端到端验证中文 PDF 渲染。
    /// 其它平台或无系统字体时跳过（仅静默 return 通过）。
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn export_pdf_renders_chinese() {
        if !std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc").exists() {
            eprintln!("跳过：msyh.ttc 不存在");
            return;
        }

        let out = tmp_out("chinese_test.pdf");
        let tool = ExportPdfTool;
        let ctx = crate::ToolContext {
            working_dir: std::env::temp_dir().to_string_lossy().to_string(),
            conversation_id: None,
            message_id: None,
            allow_write: true,
            allow_execute: true,
            allow_network: false,
            abort_signal: None,
            extra: std::collections::HashMap::new(),
            permissions: None,
            output_sanitizer: None,
            ask_user_bridge: None,
            rollback_stack: None,
            agent_id: None,
            dynamic_tools: None,
            sandbox: None,
            approval_policy: None,
        };
        let md = "# 中文标题\n\n这是一段包含中文的段落。**粗体中文** 和 `代码中文` 与 English mixed 都应正常显示。\n\n## 列表\n\n- 第一项：用 CJK 字体渲染\n- Second item: 混合排版\n\n```rust\nfn 中文变量() { println!(\"中文日志\"); }\n```\n";
        let input = serde_json::json!({
            "markdown": md,
            "output_path": out.to_string_lossy(),
            "title": "CJK 测试"
        });
        let result = tool.call(input, &ctx).await.expect("测试：异步操作应成功");
        assert!(!result.is_error, "ExportPdf 失败: {}", result.content);

        let bytes = std::fs::read(&out).expect("PDF 文件应存在");
        assert!(bytes.starts_with(b"%PDF-"), "应以 %PDF- 开头");

        // pdf-extract 反向解析
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("pdf-extract 解析失败");
        assert!(text.contains("中文标题"), "PDF 应保留中文标题，实际: {}", text);
        assert!(text.contains("中文"), "PDF 应保留中文段落");
        assert!(text.contains("CJK"), "拉丁字符应保留");
        assert!(text.contains("English"), "英文应保留");

        let preview: String = text.chars().take(500).collect();
        eprintln!("pdf-extract 提取的文本（前 500 字符）:\n{}", preview);
        let _ = std::fs::remove_file(&out);
    }

    /// 验证 build_pdf 直接调用 + pdf-extract 端到端。
    /// 这个测试不依赖系统字体 — 它测试"无 CJK 字体"降级路径（拉丁文应正常输出）。
    #[tokio::test]
    async fn build_pdf_latin_only_fallback() {
        let out = tmp_out("latin_test.pdf");
        let doc = markdown::parse_markdown("# Hello World\n\nThis is English content with `code`.");
        build_pdf(
            &doc,
            "Latin Test",
            "",
            "",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            None,
            None,
            None,
            false,
        )
        .expect("拉丁文 PDF 生成应成功");

        let bytes = std::fs::read(&out).expect("PDF 文件应存在");
        assert!(bytes.starts_with(b"%PDF-"));
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("pdf-extract 解析失败");
        assert!(text.contains("Hello World"));
        assert!(text.contains("English content"));
        let _ = std::fs::remove_file(&out);
    }
}

// ── 集成测试：数学公式（LaTeX → Unicode 文本注入 PDF）─────────────────────

#[cfg(test)]
mod pdf_math_test {
    use super::*;
    use crate::markdown;
    use std::path::PathBuf;

    fn tmp_out(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("axagent-pdf-math-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        dir.join(name)
    }

    /// 行内数学：$x^2$ 应在 PDF 文本中显示为 x^2 形式（保留 sup 标记以反映结构）
    /// 注：运行时检查 msyh.ttc 存在后再测，避免并行测试中 OnceLock 被毒化时误报。
    #[tokio::test]
    async fn inline_math_appears_in_pdf() {
        if !std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc").exists() {
            eprintln!("跳过：msyh.ttc 不存在");
            return;
        }
        let out = tmp_out("inline_math.pdf");
        let md = "勾股定理：$a^2 + b^2 = c^2$。";
        let doc = markdown::parse_markdown(md);
        build_pdf(
            &doc,
            "Math Test",
            "",
            "",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            None,
            None,
            None,
            false,
        )
        .expect("PDF 生成应成功");

        let bytes = std::fs::read(&out).expect("PDF 文件应存在");
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("pdf-extract 解析失败");
        // Greek/Unicode 字符直接出现在 PDF 文本中
        assert!(text.contains("勾股定理"), "中文应保留");
        // 上下标用 ^/_ 标记输出（结构化保留）
        assert!(text.contains("a^2") || text.contains("a²"), "上标应出现");
        assert!(text.contains("b^2") || text.contains("b²"), "上标应出现");
        assert!(text.contains("c^2") || text.contains("c²"), "上标应出现");
        let _ = std::fs::remove_file(&out);
    }

    /// 块级数学：$$...$$ 包含 LaTeX 命令，应展开为 Unicode 字符
    /// 注：运行时检查 msyh.ttc 存在后再测，避免并行测试中 OnceLock 被毒化时误报。
    #[tokio::test]
    async fn display_math_unfolds_latex() {
        if !std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc").exists() {
            eprintln!("跳过：msyh.ttc 不存在");
            return;
        }
        let out = tmp_out("display_math.pdf");
        let md = "## 公式\n\n$$\\alpha + \\beta = \\gamma$$\n\n行内 $\\sum_{i=1}^n i$ 求和。";
        let doc = markdown::parse_markdown(md);
        build_pdf(
            &doc,
            "Display Math",
            "",
            "",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            None,
            None,
            None,
            false,
        )
        .expect("PDF 生成应成功");

        let bytes = std::fs::read(&out).expect("PDF 文件应存在");
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("pdf-extract 解析失败");
        // pdf-extract 对 CIDFont 中 Unicode 希腊字母提取有限，
        // 改为验证 LaTeX 命令已被转换（不再以 \command 形式出现）
        assert!(!text.contains("\\alpha"), "LaTeX \\alpha 命令应被转换");
        assert!(!text.contains("\\beta"), "LaTeX \\beta 命令应被转换");
        assert!(!text.contains("\\gamma"), "LaTeX \\gamma 命令应被转换");
        assert!(!text.contains("\\sum"), "LaTeX \\sum 命令应被转换");
        assert!(text.contains("公式"), "中文标题应保留");
        assert!(text.contains("求和"), "中文应保留");
        let _ = std::fs::remove_file(&out);
    }

    /// 不等式符号：\leq, \geq, \neq
    /// 注：运行时检查 msyh.ttc 存在后再测，避免并行测试中 OnceLock 被毒化时误报。
    #[tokio::test]
    async fn inequality_symbols() {
        if !std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc").exists() {
            eprintln!("跳过：msyh.ttc 不存在");
            return;
        }
        let out = tmp_out("inequality.pdf");
        let md = "约束：$a \\leq b$ 且 $b \\neq c$。";
        let doc = markdown::parse_markdown(md);
        build_pdf(
            &doc,
            "Inequality",
            "",
            "",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            None,
            None,
            None,
            false,
        )
        .expect("PDF 生成应成功");

        let bytes = std::fs::read(&out).expect("PDF 文件应存在");
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("pdf-extract 解析失败");
        eprintln!("PDF 提取内容: {}", text);
        assert!(text.contains("约束"), "中文应保留");
        assert!(text.contains("b"), "字母 b 应保留");
        // 验证原始 LaTeX 命令被替换为 Unicode
        assert!(!text.contains("\\leq"), "LaTeX 命令应被转换");
        assert!(!text.contains("\\neq"), "LaTeX 命令应被转换");
        let _ = std::fs::remove_file(&out);
    }
}

// ── 集成测试：ExportPptx PowerPoint 兼容性（必须包含 theme/master/layout）───

#[cfg(test)]
mod pptx_compat {
    use super::*;
    use std::collections::HashSet;
    use std::io::Read;
    use std::path::PathBuf;

    fn tmp_out(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("axagent-pptx-compat-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        dir.join(name)
    }

    /// 端到端生成 PPTX + 验证 OPC 包结构完整（PowerPoint 兼容性要求）。
    /// 必须包含：theme、master、layout、每张 slide 的 .rels。
    #[tokio::test]
    async fn export_pptx_has_complete_opc_structure() {
        let out = tmp_out("compat_test.pptx");
        let tool = ExportPptxTool;
        let ctx = crate::ToolContext {
            working_dir: std::env::temp_dir().to_string_lossy().to_string(),
            conversation_id: None,
            message_id: None,
            allow_write: true,
            allow_execute: true,
            allow_network: false,
            abort_signal: None,
            extra: std::collections::HashMap::new(),
            permissions: None,
            output_sanitizer: None,
            ask_user_bridge: None,
            rollback_stack: None,
            agent_id: None,
            dynamic_tools: None,
            sandbox: None,
            approval_policy: None,
        };
        let md = "# 第一页\n\n内容一\n\n# 第二页\n\n内容二\n\n# 第三页\n\n内容三\n";
        let input = serde_json::json!({
            "markdown": md,
            "output_path": out.to_string_lossy(),
            "title": "兼容性测试"
        });
        let result = tool.call(input, &ctx).await.expect("测试：异步操作应成功");
        assert!(!result.is_error, "ExportPptx 失败: {}", result.content);

        // 用 zip crate 解压并列出所有部件
        let bytes = std::fs::read(&out).expect("PPTX 文件应存在");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析失败");

        let mut parts: HashSet<String> = HashSet::new();
        for i in 0..archive.len() {
            let f = archive.by_index(i).expect("测试：by_index 应成功");
            parts.insert(f.name().to_string());
        }

        // 必需部件清单（PowerPoint 兼容性硬要求）
        let required = [
            "[Content_Types].xml",
            "_rels/.rels",
            "ppt/presentation.xml",
            "ppt/_rels/presentation.xml.rels",
            "ppt/theme/theme1.xml",
            "ppt/slideMasters/slideMaster1.xml",
            "ppt/slideMasters/_rels/slideMaster1.xml.rels",
            "ppt/slideLayouts/slideLayout1.xml",
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
            "ppt/slides/slide1.xml",
            "ppt/slides/_rels/slide1.xml.rels",
            "ppt/slides/slide2.xml",
            "ppt/slides/_rels/slide2.xml.rels",
            "ppt/slides/slide3.xml",
            "ppt/slides/_rels/slide3.xml.rels",
        ];
        let missing: Vec<&str> =
            required.iter().filter(|p| !parts.contains(**p)).copied().collect();
        assert!(missing.is_empty(), "PPTX 缺少数 {} 个必需 OPC 部件：{:?}", missing.len(), missing);

        // 验证 Content_Types 包含 theme/master/layout override
        let mut ct_xml = String::new();
        archive
            .by_name("[Content_Types].xml")
            .expect("测试：by_name 应成功")
            .read_to_string(&mut ct_xml)
            .unwrap();
        assert!(ct_xml.contains("theme+xml"), "Content_Types 缺 theme override");
        assert!(ct_xml.contains("slideMaster+xml"), "Content_Types 缺 slideMaster override");
        assert!(ct_xml.contains("slideLayout+xml"), "Content_Types 缺 slideLayout override");

        // 验证 theme1.xml 引用合法的颜色方案
        let mut theme_xml = String::new();
        archive
            .by_name("ppt/theme/theme1.xml")
            .expect("测试：by_name 应成功")
            .read_to_string(&mut theme_xml)
            .unwrap();
        assert!(theme_xml.contains("clrScheme"), "theme 缺 clrScheme");
        assert!(theme_xml.contains("fontScheme"), "theme 缺 fontScheme");
        assert!(theme_xml.contains("fmtScheme"), "theme 缺 fmtScheme");

        // 验证 slideMaster 引用 slideLayout
        let mut master_xml = String::new();
        archive
            .by_name("ppt/slideMasters/slideMaster1.xml")
            .expect("测试应成功")
            .read_to_string(&mut master_xml)
            .expect("测试应成功");
        assert!(master_xml.contains("sldLayoutIdLst"), "master 缺 sldLayoutIdLst");
        assert!(master_xml.contains("txStyles"), "master 缺 txStyles");

        // 验证每张 slide 的 .rels 引用 slideLayout1
        for i in 1..=3 {
            let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", i);
            let mut rels = String::new();
            archive
                .by_name(&rels_path)
                .expect("测试：by_name 应成功")
                .read_to_string(&mut rels)
                .unwrap();
            assert!(rels.contains("slideLayout1.xml"), "slide{}.xml.rels 未引用 slideLayout1", i);
        }

        // 验证 presentation.xml 引用 slideMaster
        let mut pres_xml = String::new();
        archive
            .by_name("ppt/presentation.xml")
            .expect("测试：by_name 应成功")
            .read_to_string(&mut pres_xml)
            .unwrap();
        assert!(pres_xml.contains("sldMasterIdLst"), "presentation 缺 sldMasterIdLst");
        assert!(pres_xml.contains("rId2"), "presentation 未引用 slideMaster (rId2)");

        let _ = std::fs::remove_file(&out);
    }

    /// 验证含数字列表格的 Markdown 会触发图表幻灯片生成：
    /// - chart{N}.xml 文件存在
    /// - chart XML 含 c:barChart / c:ser / c:cat / c:val
    /// - Content_Types 含 chart override
    /// - 对应 slide rels 含 chart 关系
    #[tokio::test]
    async fn export_pptx_generates_chart_slide_for_numeric_table() {
        let out = tmp_out("chart_test.pptx");
        let tool = ExportPptxTool;
        let ctx = crate::ToolContext {
            working_dir: std::env::temp_dir().to_string_lossy().to_string(),
            conversation_id: None,
            message_id: None,
            allow_write: true,
            allow_execute: true,
            allow_network: false,
            abort_signal: None,
            extra: std::collections::HashMap::new(),
            permissions: None,
            output_sanitizer: None,
            ask_user_bridge: None,
            rollback_stack: None,
            agent_id: None,
            dynamic_tools: None,
            sandbox: None,
            approval_policy: None,
        };
        // 含数字列的表格 → 应触发图表生成
        let md = "# 销售数据\n\n| 季度 | 销售额 |\n| --- | --- |\n| Q1 | 100 |\n| Q2 | 250 |\n| Q3 | 180 |\n";
        let input = serde_json::json!({
            "markdown": md,
            "output_path": out.to_string_lossy(),
            "title": "图表测试",
            "enable_chart": true
        });
        let result = tool.call(input, &ctx).await.expect("测试：异步操作应成功");
        assert!(!result.is_error, "ExportPptx 失败: {}", result.content);

        let bytes = std::fs::read(&out).expect("PPTX 文件应存在");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析失败");

        // 1. 验证 chart1.xml 存在且结构合法
        let mut chart_xml = String::new();
        archive
            .by_name("ppt/charts/chart1.xml")
            .expect("应有 chart1.xml")
            .read_to_string(&mut chart_xml)
            .expect("测试应成功");
        assert!(chart_xml.contains("c:barChart"), "chart XML 缺 c:barChart");
        assert!(chart_xml.contains("<c:ser>"), "chart XML 缺 c:ser");
        assert!(chart_xml.contains("<c:cat>"), "chart XML 缺 c:cat");
        assert!(chart_xml.contains("<c:val>"), "chart XML 缺 c:val");
        // 类别应为 Q1/Q2/Q3，值应为 100/250/180
        assert!(chart_xml.contains("Q1"), "chart XML 缺 Q1 类别");
        assert!(chart_xml.contains("250"), "chart XML 缺值 250");

        // 2. 验证 Content_Types 含 chart override
        let mut ct_xml = String::new();
        archive
            .by_name("[Content_Types].xml")
            .expect("测试：by_name 应成功")
            .read_to_string(&mut ct_xml)
            .unwrap();
        assert!(ct_xml.contains("drawingml.chart+xml"), "Content_Types 缺 chart override");

        // 3. 遍历所有 slide rels，找到含 chart 关系的那张
        //    （图表幻灯片编号不固定：标题幻灯片 + 表格幻灯片 + 图表幻灯片 = slide3）
        let mut chart_slide_num: Option<usize> = None;
        for i in 1..=10 {
            let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", i);
            if let Ok(mut f) = archive.by_name(&rels_path) {
                let mut rels = String::new();
                use std::io::Read;
                let _ = f.read_to_string(&mut rels);
                if rels.contains("rIdChart1") {
                    chart_slide_num = Some(i);
                    break;
                }
            }
        }
        let chart_slide_num = chart_slide_num.expect("应存在含 rIdChart1 的 slide rels");

        // 4. 验证该图表幻灯片 XML 含 graphicFrame 引用
        let mut slide_xml = String::new();
        let slide_path = format!("ppt/slides/slide{}.xml", chart_slide_num);
        archive
            .by_name(&slide_path)
            .expect("测试：by_name 应成功")
            .read_to_string(&mut slide_xml)
            .unwrap();
        assert!(slide_xml.contains("p:graphicFrame"), "图表幻灯片应含 graphicFrame");
        assert!(slide_xml.contains("r:id=\"rIdChart1\""), "图表幻灯片应引用 rIdChart1");

        let _ = std::fs::remove_file(&out);
    }

    /// 验证 enable_chart=false 时不生成图表幻灯片
    #[tokio::test]
    async fn export_pptx_disables_chart_when_flag_off() {
        let out = tmp_out("no_chart_test.pptx");
        let tool = ExportPptxTool;
        let ctx = crate::ToolContext {
            working_dir: std::env::temp_dir().to_string_lossy().to_string(),
            conversation_id: None,
            message_id: None,
            allow_write: true,
            allow_execute: true,
            allow_network: false,
            abort_signal: None,
            extra: std::collections::HashMap::new(),
            permissions: None,
            output_sanitizer: None,
            ask_user_bridge: None,
            rollback_stack: None,
            agent_id: None,
            dynamic_tools: None,
            sandbox: None,
            approval_policy: None,
        };
        let md = "# 销售\n\n| 季度 | 销售额 |\n| --- | --- |\n| Q1 | 100 |\n| Q2 | 250 |\n";
        let input = serde_json::json!({
            "markdown": md,
            "output_path": out.to_string_lossy(),
            "title": "无图表",
            "enable_chart": false
        });
        let result = tool.call(input, &ctx).await.expect("测试：异步操作应成功");
        assert!(!result.is_error, "ExportPptx 失败: {}", result.content);

        let bytes = std::fs::read(&out).expect("PPTX 文件应存在");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析失败");

        // 不应有 chart1.xml
        let chart_result = archive.by_name("ppt/charts/chart1.xml");
        assert!(chart_result.is_err(), "enable_chart=false 时不应生成 chart1.xml");
        drop(chart_result);

        // Content_Types 不应含 chart override
        let mut ct_xml = String::new();
        archive
            .by_name("[Content_Types].xml")
            .expect("测试：by_name 应成功")
            .read_to_string(&mut ct_xml)
            .unwrap();
        assert!(
            !ct_xml.contains("drawingml.chart+xml"),
            "enable_chart=false 时 Content_Types 不应含 chart override"
        );

        let _ = std::fs::remove_file(&out);
    }
}

// ── 集成测试：ExportWord 图片缩放/对齐 ─────────────────────────────────────

#[cfg(test)]
mod word_image_test {
    use super::*;

    /// 验证 ExportWord 图片尺寸缩放 / 对齐：构造一张 2000x1500 PNG，用 max_width_pt=300 应缩放到 300x225 pt
    /// （按 96 DPI 换算，1 px = 0.75 pt，所以原图 2000x1500 = 1500x1125 pt，需缩到 300x225）。
    /// 同时验证 .docx 内部 [Content_Types].xml 包含 image/png。
    #[test]
    fn export_word_image_sizing_and_align() {
        // 构造一张 2000x1500 的纯色 PNG（蓝色）
        let mut img = image::RgbImage::new(2000, 1500);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgb([0x1F, 0x38, 0x64]);
        }
        let dir = std::env::temp_dir().join("axagent-word-img-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        let img_path = dir.join("test.png");
        img.save(&img_path).expect("保存测试图片");
        let img_path_str = img_path.to_string_lossy().to_string();

        let out = dir.join("img_test.docx");
        let doc = build_docx_from_md(
            &format!("# 文档\n\n![测试图片]({})\n", img_path_str),
            "图片测试",
            Some(300.0), // max_width_pt
            Some("left".to_string()),
        );
        let file = std::fs::File::create(&out).expect("创建 docx");
        doc.build().pack(file).expect("打包 docx");

        // 验证尺寸换算：2000px = 1500pt（@96dpi），300pt 是 400px
        let dims =
            compute_image_size_pt(&std::fs::read(&img_path).expect("测试：读取文件应成功"), 300.0)
                .expect("compute_image_size_pt");
        assert!((dims.0 - 300.0).abs() < 0.1, "宽度应缩放到 300pt, 实际 {}", dims.0);
        assert!((dims.1 - 225.0).abs() < 0.1, "高度应等比缩到 225pt, 实际 {}", dims.1);

        // 验证 .docx 内含 image/png
        let bytes = std::fs::read(&out).expect("docx 文件");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析");
        let mut ct = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("[Content_Types].xml").expect("测试应成功"),
            &mut ct,
        )
        .expect("测试应成功");
        assert!(ct.contains("image/png"), "Content_Types 应含 image/png override");
        assert!(ct.contains("wordprocessingml.document.main+xml"));

        // 验证 .docx 内 word/media/ 目录含 png
        let mut has_png = false;
        for i in 0..archive.len() {
            let f = archive.by_index(i).expect("测试：by_index 应成功");
            if f.name().starts_with("word/media/") && f.name().ends_with(".png") {
                has_png = true;
                break;
            }
        }
        assert!(has_png, "docx 内 word/media/ 目录应包含 PNG 图片");

        eprintln!("图片测试通过：300x225 pt, align=left");
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(&img_path);
    }
}

// ── 集成测试：ExportWord 表格图表（Unicode 块字符条形图）─────────────────────

#[cfg(test)]
mod word_chart_test {
    use super::*;

    /// 验证含数字列的表格会在 .docx 中追加 Unicode 块字符条形图段落：
    /// - 图表标题段落 "图: 销售额"
    /// - 每行含 █ 字符和数值
    /// - 最大值（250）对应 30 个 █，其他按比例
    #[test]
    fn export_word_appends_chart_for_numeric_table() {
        let dir = std::env::temp_dir().join("axagent-word-chart-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        let out = dir.join("chart.docx");

        let md = "# 销售报告\n\n| 季度 | 销售额 |\n| --- | --- |\n| Q1 | 100 |\n| Q2 | 250 |\n| Q3 | 180 |\n";
        let doc = build_docx_from_md(md, "图表测试", None, None);
        let file = std::fs::File::create(&out).expect("创建 docx");
        doc.build().pack(file).expect("打包 docx");

        // 解压 .docx，读取 word/document.xml
        let bytes = std::fs::read(&out).expect("docx 文件");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析");
        let mut doc_xml = String::new();
        use std::io::Read;
        archive
            .by_name("word/document.xml")
            .expect("测试：by_name 应成功")
            .read_to_string(&mut doc_xml)
            .unwrap();

        // 1. 应含图表标题 "图: 销售额"
        assert!(doc_xml.contains("图: 销售额"), "docx 应含图表标题段落 '图: 销售额'");

        // 2. 应含 █ 块字符（U+2588，UTF-8 编码 0xE2 0x96 0x88）
        assert!(doc_xml.contains('\u{2588}'), "docx 应含 Unicode 块字符 █ (U+2588)");

        // 3. 应含所有类别标签 Q1/Q2/Q3 和值 100/250/180
        for s in ["Q1", "Q2", "Q3", "100", "250", "180"] {
            assert!(doc_xml.contains(s), "docx 应含表格数据 '{}'", s);
        }

        // 4. 验证最大值 250 对应 30 个 █（docx-rs 把每行的块字符放在一个 <w:t> 中）
        //    统计 docx 中所有 █ 字符总数，应 >= 30（最大值对应 30，其他按比例）
        let total_blocks = doc_xml.chars().filter(|&c| c == '\u{2588}').count();
        assert!(
            total_blocks >= 30,
            "最大值 250 应对应 30 个 █，总 █ 数应 >= 30，实际 {}",
            total_blocks
        );

        let _ = std::fs::remove_file(&out);
    }

    /// 验证纯文本表格（无数字列）不会追加图表段落
    #[test]
    fn export_word_no_chart_for_text_only_table() {
        let dir = std::env::temp_dir().join("axagent-word-chart-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        let out = dir.join("no_chart.docx");

        let md = "# 名单\n\n| 姓名 | 部门 |\n| --- | --- |\n| 张三 | 销售 |\n| 李四 | 研发 |\n";
        let doc = build_docx_from_md(md, "无图表", None, None);
        let file = std::fs::File::create(&out).expect("创建 docx");
        doc.build().pack(file).expect("打包 docx");

        let bytes = std::fs::read(&out).expect("docx 文件");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析");
        let mut doc_xml = String::new();
        use std::io::Read;
        archive
            .by_name("word/document.xml")
            .expect("测试：by_name 应成功")
            .read_to_string(&mut doc_xml)
            .unwrap();

        // 不应含图表标题前缀 "图: "
        assert!(!doc_xml.contains("图: "), "纯文本表格不应追加图表段落");
        // 不应含 █ 字符
        assert!(!doc_xml.contains('\u{2588}'), "纯文本表格不应含 █ 字符");

        let _ = std::fs::remove_file(&out);
    }
}

// ── 集成测试：ExportPdf 图片嵌入（XObject / DCTDecode JPEG）────────────────

#[cfg(test)]
mod pdf_image_test {
    use super::*;
    use std::path::PathBuf;

    fn tmp_out(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("axagent-pdf-image-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        dir.join(name)
    }

    fn make_test_png(w: u32, h: u32, color: [u8; 3]) -> PathBuf {
        let dir = std::env::temp_dir().join("axagent-pdf-image-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        let path = dir.join(format!("test_{}x{}.png", w, h));
        let mut img = image::RgbImage::new(w, h);
        for px in img.pixels_mut() {
            *px = image::Rgb(color);
        }
        img.save(&path).expect("保存测试 PNG");
        path
    }

    /// 验证 ExportPdf 把 PNG 嵌入 PDF：XObject 字典 + DCTDecode JPEG 数据流 + Page Resources 引用 + do 操作符。
    #[test]
    fn export_pdf_embeds_png_as_jpeg_xobject() {
        let img_path = make_test_png(800, 600, [0x33, 0x99, 0xCC]);
        let img_path_str = img_path.to_string_lossy().to_string();

        let out = tmp_out("img_test.pdf");
        let md = format!("# 测试\n\n![图]({})\n", img_path_str);
        let doc = markdown::parse_markdown(&md);

        build_pdf(
            &doc,
            "Image Test",
            "",
            "",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            None,
            None,
            None,
            false,
        )
        .expect("PDF 生成应成功");

        let bytes = std::fs::read(&out).expect("PDF 文件应存在");
        assert!(bytes.starts_with(b"%PDF-"));

        // 检查 PDF 字节流：应包含 /Subtype /Image（DCTDecode XObject 标记）和 /DCTDecode 滤镜
        let raw = String::from_utf8_lossy(&bytes);
        assert!(raw.contains("/Subtype"), "PDF 应含 /Subtype 定义");
        assert!(raw.contains("/Image"), "PDF 应含 /Image XObject");
        assert!(raw.contains("/DCTDecode"), "PDF 应使用 DCTDecode 滤镜承载 JPEG");
        assert!(raw.contains("/XObject"), "Page Resources 应引用 XObject 字典");
        assert!(raw.contains(" Do"), "内容流应含 Do 操作符（绘制 XObject）");

        // 校验 PDF 至少有一页
        let page_count = raw.matches("/Type /Page").count() + raw.matches("/Type/Page").count();
        assert!(page_count >= 1, "PDF 应至少有一页");

        eprintln!("PDF 图片嵌入验证通过：含 XObject + DCTDecode + Do 操作符");
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(&img_path);
    }

    /// 验证 ExportPdf 处理图片缺失：优雅降级为占位文本，不崩溃。
    #[test]
    fn export_pdf_handles_missing_image() {
        let out = tmp_out("missing_img_test.pdf");
        let md = "# 测试\n\n![不存在的图](Z:/nonexistent/path/missing_xyz_12345.png)\n";
        let doc = markdown::parse_markdown(md);

        build_pdf(
            &doc,
            "Missing Image Test",
            "",
            "",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            None,
            None,
            None,
            false,
        )
        .expect("缺图片时应优雅降级，不应失败");

        let bytes = std::fs::read(&out).expect("PDF 文件应存在");
        assert!(bytes.starts_with(b"%PDF-"));
        eprintln!("缺失图片降级测试通过");
        let _ = std::fs::remove_file(&out);
    }

    /// 验证 ExportPdf 不同对齐方式：左/中/右。生成 3 个 PDF 验证文件有效即可（PDF 字面量层面
    /// 验证 x 坐标差异较复杂，这里只验证三张图都能成功嵌入）。
    #[test]
    fn export_pdf_image_aligns_left_center_right() {
        let img_path = make_test_png(400, 300, [0xAA, 0xBB, 0xCC]);
        let img_path_str = img_path.to_string_lossy().to_string();

        for align in ["left", "center", "right"] {
            let out = tmp_out(&format!("align_{}.pdf", align));
            let md = format!("# 对齐测试 {}\n\n![图]({})\n", align, img_path_str);
            let doc = markdown::parse_markdown(&md);
            build_pdf(
                &doc,
                &format!("Align {}", align),
                "",
                "",
                out.to_str().expect("测试：路径转字符串应成功"),
                540.0,
                align,
                "",
                None,
                None,
                None,
                false,
            )
            .unwrap_or_else(|e| panic!("align={} 生成失败: {}", align, e));
            let bytes = std::fs::read(&out).expect("PDF 文件应存在");
            assert!(bytes.starts_with(b"%PDF-"), "align={} 应生成有效 PDF", align);
            assert!(
                bytes.windows(8).any(|w| w == b"/DCTDeco"),
                "align={} 应嵌入 JPEG XObject",
                align
            );
            let _ = std::fs::remove_file(&out);
        }
        eprintln!("三种对齐方式测试通过");
        let _ = std::fs::remove_file(&img_path);
    }
}

// ── 集成测试：ExportXlsx 图表（柱状图 / 折线图 / 饼图）────────────────────

#[cfg(test)]
mod xlsx_chart_test {
    use super::*;
    use std::io::Read;
    use std::path::PathBuf;

    fn tmp_out(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("axagent-xlsx-chart-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        dir.join(name)
    }

    /// 验证 ExportXlsx 默认开启图表：生成的 .xlsx 包内应含 xl/charts/ 目录和 chart1.xml。
    #[test]
    fn export_xlsx_embeds_column_chart() {
        let out = tmp_out("column_test.xlsx");
        let md = "| 月份 | 销量 | 收入 |\n| ---- | ---- | ---- |\n| 1月 | 100 | 2000 |\n| 2月 | 150 | 3000 |\n| 3月 | 180 | 3600 |\n| 4月 | 200 | 4000 |\n";
        let result =
            build_xlsx(md, "销售", out.to_str().expect("测试：路径转字符串应成功"), true, "column");
        assert!(result.is_ok(), "ExportXlsx 失败: {:?}", result.err());

        let bytes = std::fs::read(&out).expect("XLSX 文件应存在");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析失败");

        let mut has_chart_xml = false;
        let mut has_drawing_xml = false;
        let mut has_chart_rels = false;
        for i in 0..archive.len() {
            let f = archive.by_index(i).expect("测试：by_index 应成功");
            let name = f.name();
            if name.starts_with("xl/charts/chart") && name.ends_with(".xml") {
                has_chart_xml = true;
            }
            if name.starts_with("xl/drawings/drawing") && name.ends_with(".xml") {
                has_drawing_xml = true;
            }
            // rust_xlsxwriter 把 chart 关系放在 xl/drawings/_rels/ 而非 xl/charts/_rels/
            if name.starts_with("xl/drawings/_rels/") && name.ends_with(".xml.rels") {
                has_chart_rels = true;
            }
        }
        assert!(has_chart_xml, "XLSX 应含 xl/charts/chart1.xml");
        assert!(has_drawing_xml, "XLSX 应含 xl/drawings/drawing1.xml");
        assert!(has_chart_rels, "XLSX 应含 chart 关系文件");

        // 校验 chart1.xml 内含 /c:barChart / c:lineChart / c:pieChart 之一 + categories + values 引用
        let mut chart_xml = String::new();
        archive
            .by_name("xl/charts/chart1.xml")
            .expect("chart1.xml 应存在")
            .read_to_string(&mut chart_xml)
            .expect("测试应成功");
        assert!(
            chart_xml.contains("<c:barChart")
                || chart_xml.contains("<c:lineChart")
                || chart_xml.contains("<c:pieChart"),
            "chart1.xml 应含图表类型定义"
        );
        // 公式引用（sheet 名 + 单元格范围）
        assert!(chart_xml.contains("<c:f>"), "chart1.xml 应含公式 <c:f> 引用");
        // 类别列引用：A 列 (月份)
        assert!(chart_xml.contains("$A$"), "chart1.xml 应含 categories 引用 A 列");
        // 值列引用：B 列 (销量) 或 C 列 (收入)
        assert!(
            chart_xml.contains("$B$") || chart_xml.contains("$C$"),
            "chart1.xml 应含 values 引用 B 或 C 列"
        );

        eprintln!("柱状图测试通过：chart1.xml + drawing1.xml + 关系文件齐全");
        let _ = std::fs::remove_file(&out);
    }

    /// 验证 4 种图表类型：bar / column / line / pie。生成的 .xlsx 各自含正确图表类型标签。
    #[test]
    fn export_xlsx_supports_all_chart_types() {
        let md = "\
| 月份 | 销量 |
| ---- | ---- |
| 1月 | 100 |
| 2月 | 150 |
| 3月 | 180 |
";
        for (ct, expected_tag) in [
            ("bar", "<c:barChart"),
            ("column", "<c:barChart"),
            ("line", "<c:lineChart"),
            ("pie", "<c:pieChart"),
        ] {
            let out = tmp_out(&format!("chart_{}.xlsx", ct));
            let result = build_xlsx(md, "Data", out.to_str().expect("路径编码无效"), true, ct);
            assert!(result.is_ok(), "chart_type={} 失败: {:?}", ct, result.err());

            let bytes = std::fs::read(&out).expect("XLSX 文件应存在");
            let cursor = std::io::Cursor::new(bytes);
            let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析");
            let mut chart_xml = String::new();
            archive
                .by_name("xl/charts/chart1.xml")
                .expect("测试应成功")
                .read_to_string(&mut chart_xml)
                .expect("测试应成功");
            assert!(
                chart_xml.contains(expected_tag),
                "chart_type={} 应在 chart1.xml 中包含 {}，实际内容片段: {}",
                ct,
                expected_tag,
                &chart_xml[..chart_xml.len().min(200)]
            );
            let _ = std::fs::remove_file(&out);
        }
        eprintln!("4 种图表类型全部通过");
    }

    /// 验证 enable_chart=false 时不生成图表。
    #[test]
    fn export_xlsx_disables_chart() {
        let md = "\
| 月份 | 销量 |
| ---- | ---- |
| 1月 | 100 |
| 2月 | 150 |
";
        let out = tmp_out("no_chart.xlsx");
        let result =
            build_xlsx(md, "NoChart", out.to_str().expect("路径编码无效"), false, "column");
        assert!(result.is_ok());

        let bytes = std::fs::read(&out).expect("XLSX 文件应存在");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析");
        let mut has_chart = false;
        for i in 0..archive.len() {
            let f = archive.by_index(i).expect("测试应成功");
            if f.name().starts_with("xl/charts/") && f.name().ends_with(".xml") {
                has_chart = true;
                break;
            }
        }
        assert!(!has_chart, "enable_chart=false 时不应生成图表文件");
        eprintln!("disable_chart 测试通过");
        let _ = std::fs::remove_file(&out);
    }

    /// 验证无数字列时优雅降级（不生成图表，不崩溃）。
    #[test]
    fn export_xlsx_no_numeric_columns_skips_chart() {
        let md = "\
| 姓名 | 城市 |
| ---- | ---- |
| 张三 | 北京 |
| 李四 | 上海 |
| 王五 | 广州 |
";
        let out = tmp_out("no_numeric.xlsx");
        let result =
            build_xlsx(md, "NoNumeric", out.to_str().expect("路径编码无效"), true, "column");
        assert!(result.is_ok(), "无数字列时应优雅降级: {:?}", result.err());

        let bytes = std::fs::read(&out).expect("XLSX 文件应存在");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析");
        let mut has_chart = false;
        for i in 0..archive.len() {
            let f = archive.by_index(i).expect("测试应成功");
            if f.name().starts_with("xl/charts/") && f.name().ends_with(".xml") {
                has_chart = true;
                break;
            }
        }
        assert!(!has_chart, "无数字列时不应生成图表");
        eprintln!("无数字列降级测试通过");
        let _ = std::fs::remove_file(&out);
    }
}

// ── 集成测试：ExportPdf 模板（cover/header/footer/TOC）────────────────────

#[cfg(test)]
mod pdf_template_test {
    use super::*;
    use std::path::PathBuf;

    fn tmp_out(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("axagent-pdf-template-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        dir.join(name)
    }

    /// 验证自定义 cover_template 渲染到 PDF。
    /// 注：运行时检查 msyh.ttc 存在后再测，避免并行测试中 OnceLock 被毒化时误报。
    #[test]
    fn custom_cover_template_renders() {
        if !std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc").exists() {
            eprintln!("跳过：msyh.ttc 不存在");
            return;
        }
        let out = tmp_out("custom_cover.pdf");
        let md = "# 第一章\n\n正文内容\n";
        let doc = markdown::parse_markdown(md);
        let cover_tpl = "【{{ title }}】\n~~{{ subtitle }}~~\n-- {{ author }} --";
        let result = build_pdf(
            &doc,
            "我的报告",
            "2025 年度",
            "张三",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            Some(cover_tpl),
            None,
            None,
            false,
        );
        assert!(result.is_ok(), "导出失败: {:?}", result.err());
        let bytes = std::fs::read(&out).expect("PDF 应存在");
        assert!(bytes.starts_with(b"%PDF-"));
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("提取文本");
        assert!(text.contains("我的报告"), "PDF 文本应含 cover 标题");
        assert!(text.contains("2025 年度"), "PDF 文本应含 cover 副标题");
        assert!(text.contains("张三"), "PDF 文本应含 cover 作者");
        eprintln!("自定义封面模板测试通过");
        let _ = std::fs::remove_file(&out);
    }

    /// 验证 header_template / footer_template 渲染到每页。
    #[test]
    fn header_footer_templates_per_page() {
        let out = tmp_out("header_footer.pdf");
        let md = "# 标题\n\n## 第一节\n\n## 第二节\n\n## 第三节\n\n## 第四节\n\n";
        let doc = markdown::parse_markdown(md);
        let header_tpl = "=={{ title }}==";
        let footer_tpl = "[P{{ page_no }} of {{ total_pages }}]";
        let result = build_pdf(
            &doc,
            "Doc",
            "",
            "",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            None,
            Some(header_tpl),
            Some(footer_tpl),
            false,
        );
        assert!(result.is_ok(), "导出失败: {:?}", result.err());
        let bytes = std::fs::read(&out).expect("PDF 应存在");
        let raw = String::from_utf8_lossy(&bytes);
        // 检查 raw bytes 内含 header / footer 字符串
        assert!(raw.contains("Doc"), "header_template 应渲染 title");
        assert!(raw.contains("P1 of"), "footer_template 第 1 页应含 P1");
        eprintln!("header/footer 模板测试通过");
        let _ = std::fs::remove_file(&out);
    }

    /// 验证 enable_toc 自动生成目录页。
    /// 注：运行时检查 msyh.ttc 存在后再测，避免并行测试中 OnceLock 被毒化时误报。
    #[test]
    fn enable_toc_generates_table_of_contents() {
        if !std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc").exists() {
            eprintln!("跳过：msyh.ttc 不存在");
            return;
        }
        let out = tmp_out("toc.pdf");
        let md = "# 第一章\n\n## 1.1 节\n\n# 第二章\n\n## 2.1 节\n\n# 第三章\n";
        let doc = markdown::parse_markdown(md);
        let result = build_pdf(
            &doc,
            "目录测试",
            "",
            "",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            None,
            None,
            None,
            true,
        );
        assert!(result.is_ok(), "导出失败: {:?}", result.err());
        let bytes = std::fs::read(&out).expect("PDF 应存在");
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("提取文本");
        assert!(text.contains("目录"), "PDF 应有目录页头");
        assert!(text.contains("第一章"), "PDF 目录应含第一章");
        assert!(text.contains("1.1"), "PDF 目录应含 1.1 标题");
        assert!(text.contains("第二章"), "PDF 目录应含第二章");
        eprintln!("TOC 目录测试通过");
        let _ = std::fs::remove_file(&out);
    }

    /// 验证 PDF 字符串字面量转义：标题含 ( ) \\ 时不破坏 PDF 结构。
    #[test]
    fn pdf_escape_special_chars_in_templates() {
        let s = "title (with) \\ backslash and) parenthesis";
        let escaped = pdf_escape_string(s);
        assert_eq!(escaped, "title \\(with\\) \\\\ backslash and\\) parenthesis");
        eprintln!("PDF 转义测试通过");
    }
}

// ── 集成测试：Mermaid 流程图渲染（DOCX/PDF/PPTX）────────────────────────────

#[cfg(test)]
mod mermaid_integration_test {
    use super::*;
    use std::path::PathBuf;

    fn tmp_out(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("axagent-mermaid-test");
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        dir.join(name)
    }

    /// 测试用 Mermaid 流程图：含节点形状 + 边 + 标签
    const TEST_MERMAID: &str =
        "graph LR\n    A[开始] --> B{成功?}\n    B -->|是| C[结束]\n    B -->|否| D[重试]\n";

    /// 验证 DOCX 导出含 mermaid 代码块时，渲染为 Unicode 框线字符文本段落
    #[test]
    fn export_docx_renders_mermaid_flowchart() {
        let out = tmp_out("mermaid.docx");
        let md = format!("# 流程图测试\n\n```mermaid\n{}\n```\n", TEST_MERMAID);
        let doc = build_docx_from_md(&md, "Mermaid 测试", None, None);
        let file = std::fs::File::create(&out).expect("创建 docx");
        doc.build().pack(file).expect("打包 docx");

        let bytes = std::fs::read(&out).expect("docx 文件");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析");
        let mut doc_xml = String::new();
        use std::io::Read;
        archive
            .by_name("word/document.xml")
            .expect("测试：by_name 应成功")
            .read_to_string(&mut doc_xml)
            .unwrap();

        // 1. 应含"流程图"标签
        assert!(doc_xml.contains("流程图"), "docx 应含 '流程图' 标签");
        // 2. 应含节点文本（开始/结束/重试）
        for s in ["开始", "结束", "重试"] {
            assert!(doc_xml.contains(s), "docx 应含 mermaid 节点文本 '{}'", s);
        }
        // 3. 应含边标签（是/否）
        assert!(doc_xml.contains("是"), "docx 应含边标签 '是'");
        // 4. 不应含原始 mermaid 语法（graph LR）
        assert!(!doc_xml.contains("graph LR"), "docx 不应含原始 mermaid 语法");

        let _ = std::fs::remove_file(&out);
    }

    /// 验证 PDF 导出含 mermaid 代码块时，渲染为 Unicode 框线字符文本
    /// 需要系统 CJK 字体（msyh.ttc）支持中文渲染，否则跳过。
    #[cfg(target_os = "windows")]
    #[test]
    fn export_pdf_renders_mermaid_flowchart() {
        if !std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc").exists() {
            eprintln!("跳过：msyh.ttc 不存在");
            return;
        }
        let out = tmp_out("mermaid.pdf");
        let md = format!("# 流程图测试\n\n```mermaid\n{}\n```\n", TEST_MERMAID);
        let doc = markdown::parse_markdown(&md);
        build_pdf(
            &doc,
            "Mermaid Test",
            "",
            "",
            out.to_str().expect("测试：路径转字符串应成功"),
            540.0,
            "center",
            "",
            None,
            None,
            None,
            false,
        )
        .expect("PDF 生成应成功");

        let bytes = std::fs::read(&out).expect("PDF 文件应存在");
        assert!(bytes.starts_with(b"%PDF-"), "应生成有效 PDF");
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("pdf-extract 解析失败");
        // 应含节点文本
        assert!(text.contains("开始"), "PDF 应含 mermaid 节点 '开始'，实际: {}", text);
        assert!(text.contains("结束"), "PDF 应含 mermaid 节点 '结束'，实际: {}", text);

        let _ = std::fs::remove_file(&out);
    }

    /// 验证 PPTX 导出含 mermaid 代码块时，渲染为 Unicode 框线字符文本（作为 bullet）
    #[tokio::test]
    async fn export_pptx_renders_mermaid_flowchart() {
        let out = tmp_out("mermaid.pptx");
        let tool = ExportPptxTool;
        let ctx = crate::ToolContext {
            working_dir: std::env::temp_dir().to_string_lossy().to_string(),
            conversation_id: None,
            message_id: None,
            allow_write: true,
            allow_execute: true,
            allow_network: false,
            abort_signal: None,
            extra: std::collections::HashMap::new(),
            permissions: None,
            output_sanitizer: None,
            ask_user_bridge: None,
            rollback_stack: None,
            agent_id: None,
            dynamic_tools: None,
            sandbox: None,
            approval_policy: None,
        };
        let md = format!("# 流程图\n\n```mermaid\n{}\n```\n", TEST_MERMAID);
        let input = serde_json::json!({
            "markdown": md,
            "output_path": out.to_string_lossy(),
            "title": "Mermaid 测试",
            "enable_chart": false
        });
        let result = tool.call(input, &ctx).await.expect("测试：异步操作应成功");
        assert!(!result.is_error, "ExportPptx 失败: {}", result.content);

        let bytes = std::fs::read(&out).expect("PPTX 文件应存在");
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("zip 解析失败");

        // 遍历所有 slide，合并 XML 后检查（mermaid 内容可能在任意 slide）
        use std::io::Read;
        let mut all_slide_xml = String::new();
        for i in 1..=10 {
            let path = format!("ppt/slides/slide{}.xml", i);
            if let Ok(mut f) = archive.by_name(&path) {
                let mut s = String::new();
                f.read_to_string(&mut s).expect("测试：read_to_string 应成功");
                all_slide_xml.push_str(&s);
            }
        }

        // 应含节点文本（开始/结束/重试）
        assert!(all_slide_xml.contains("开始"), "pptx slides 应含 '开始'");
        assert!(all_slide_xml.contains("结束"), "pptx slides 应含 '结束'");

        let _ = std::fs::remove_file(&out);
    }
}
