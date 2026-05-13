//! 统一 Markdown 解析层
//!
//! 基于 pulldown-cmark 提供共享的 MD→IR 解析，供所有文档导出工具复用。
//! 替代 misc.rs 中手写的逐行解析器。

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Markdown 解析后的中间表示
#[derive(Debug, Clone)]
pub struct MdDocument {
    pub blocks: Vec<MdBlock>,
}

#[derive(Debug, Clone)]
pub enum MdBlock {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        inlines: Vec<MdInline>,
    },
    CodeBlock {
        language: String,
        code: String,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<MdInline>>,
    },
    Blockquote {
        inlines: Vec<MdInline>,
    },
    Image {
        alt: String,
        path: String,
    },
    HorizontalRule,
}

#[derive(Debug, Clone)]
pub enum MdInline {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Link { text: String, url: String },
    Image { alt: String, path: String },
}

/// 解析 Markdown 文本为结构化 IR
pub fn parse_markdown(text: &str) -> MdDocument {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(text, options);
    let mut doc = MdDocument { blocks: Vec::new() };
    let mut current_block: Option<MdBlock> = None;
    let mut current_inlines: Vec<MdInline> = Vec::new();
    let mut in_table_head = false;
    let mut table_headers: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_cells: Vec<String> = Vec::new();
    let mut list_items: Vec<Vec<MdInline>> = Vec::new();
    let mut current_item_inlines: Vec<MdInline> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_inlines(&mut current_inlines, &mut current_block, &mut doc);
                    current_block = Some(MdBlock::Heading {
                        level: level_to_u8(level),
                        text: String::new(),
                    });
                },
                Tag::Paragraph => {
                    flush_inlines(&mut current_inlines, &mut current_block, &mut doc);
                    current_block = Some(MdBlock::Paragraph {
                        inlines: Vec::new(),
                    });
                },
                Tag::CodeBlock(kind) => {
                    flush_inlines(&mut current_inlines, &mut current_block, &mut doc);
                    current_block = Some(MdBlock::CodeBlock {
                        language: {
                            match kind {
                                pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                                pulldown_cmark::CodeBlockKind::Indented => String::new(),
                            }
                        },
                        code: String::new(),
                    });
                },
                Tag::Table(_) => {
                    flush_inlines(&mut current_inlines, &mut current_block, &mut doc);
                    in_table_head = true;
                    table_headers.clear();
                    table_rows.clear();
                    table_cells.clear();
                },
                Tag::TableHead => {
                    in_table_head = true;
                },
                Tag::TableRow => {
                    table_cells.clear();
                },
                Tag::TableCell => {},
                Tag::List(ordered) => {
                    flush_inlines(&mut current_inlines, &mut current_block, &mut doc);
                    list_items.clear();
                    current_block = Some(MdBlock::List {
                        ordered: ordered.is_some(),
                        items: Vec::new(),
                    });
                },
                Tag::Item => {
                    current_item_inlines.clear();
                },
                Tag::BlockQuote(_) => {
                    flush_inlines(&mut current_inlines, &mut current_block, &mut doc);
                    current_block = Some(MdBlock::Blockquote {
                        inlines: Vec::new(),
                    });
                },
                Tag::Image {
                    dest_url, title, ..
                } => {
                    flush_inlines(&mut current_inlines, &mut current_block, &mut doc);
                    current_block = Some(MdBlock::Image {
                        alt: title.to_string(),
                        path: dest_url.to_string(),
                    });
                },
                _ => {},
            },

            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    if let Some(MdBlock::Heading { text: _, .. }) = &mut current_block {
                        flush_and_store(&mut current_block, &mut doc);
                    }
                },
                TagEnd::Paragraph => {
                    if let Some(MdBlock::Paragraph { .. }) = &current_block {
                        flush_paragraph_block(&mut current_block, &mut current_inlines, &mut doc);
                    }
                },
                TagEnd::CodeBlock => {
                    flush_and_store(&mut current_block, &mut doc);
                },
                TagEnd::Table => {
                    if !table_headers.is_empty() || !table_rows.is_empty() {
                        doc.blocks.push(MdBlock::Table {
                            headers: std::mem::take(&mut table_headers),
                            rows: std::mem::take(&mut table_rows),
                        });
                    }
                    in_table_head = false;
                    current_block = None;
                },
                TagEnd::TableHead => {
                    in_table_head = false;
                },
                TagEnd::TableRow => {
                    if in_table_head {
                        table_headers = std::mem::take(&mut table_cells);
                    } else if !table_cells.is_empty() {
                        table_rows.push(std::mem::take(&mut table_cells));
                    }
                },
                TagEnd::TableCell => {},
                TagEnd::List(_) => {
                    if let Some(MdBlock::List { ordered, .. }) = &mut current_block {
                        let ordered = *ordered;
                        doc.blocks.push(MdBlock::List {
                            ordered,
                            items: std::mem::take(&mut list_items),
                        });
                    }
                    current_block = None;
                },
                TagEnd::Item => {
                    list_items.push(std::mem::take(&mut current_item_inlines));
                },
                TagEnd::BlockQuote(_) => {
                    if let Some(MdBlock::Blockquote { .. }) = &current_block {
                        flush_blockquote_block(&mut current_block, &mut current_inlines, &mut doc);
                    }
                },
                TagEnd::Image => {},
                _ => {},
            },

            Event::Text(text) => {
                let inline = MdInline::Text(text.to_string());
                push_inline(
                    inline,
                    &mut current_block,
                    &mut current_inlines,
                    &mut current_item_inlines,
                    &mut table_cells,
                );
            },

            Event::Code(text) => {
                let inline = MdInline::Code(text.to_string());
                push_inline(
                    inline,
                    &mut current_block,
                    &mut current_inlines,
                    &mut current_item_inlines,
                    &mut table_cells,
                );
            },

            Event::InlineMath(text) | Event::DisplayMath(text) => {
                let inline = MdInline::Code(text.to_string());
                push_inline(
                    inline,
                    &mut current_block,
                    &mut current_inlines,
                    &mut current_item_inlines,
                    &mut table_cells,
                );
            },

            Event::InlineHtml(html) | Event::Html(html) => {
                let inline = MdInline::Text(html.to_string());
                push_inline(
                    inline,
                    &mut current_block,
                    &mut current_inlines,
                    &mut current_item_inlines,
                    &mut table_cells,
                );
            },

            Event::SoftBreak | Event::HardBreak => {
                let inline = MdInline::Text("\n".to_string());
                push_inline(
                    inline,
                    &mut current_block,
                    &mut current_inlines,
                    &mut current_item_inlines,
                    &mut table_cells,
                );
            },

            Event::Rule => {
                flush_inlines(&mut current_inlines, &mut current_block, &mut doc);
                doc.blocks.push(MdBlock::HorizontalRule);
            },

            Event::TaskListMarker(checked) => {
                let marker = if checked { "[x] " } else { "[ ] " };
                let inline = MdInline::Text(marker.to_string());
                push_inline(
                    inline,
                    &mut current_block,
                    &mut current_inlines,
                    &mut current_item_inlines,
                    &mut table_cells,
                );
            },

            Event::FootnoteReference(_) => {},
        }
    }

    // 刷新剩余内容
    flush_inlines(&mut current_inlines, &mut current_block, &mut doc);
    flush_and_store(&mut current_block, &mut doc);

    doc
}

/// Markdown → HTML 渲染
pub fn render_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<style>\n");
    html.push_str("body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;max-width:800px;margin:0 auto;padding:2em;line-height:1.6;color:#333}\n");
    html.push_str("h1{font-size:2em;border-bottom:2px solid #4a90d9;padding-bottom:.3em}\n");
    html.push_str("h2{font-size:1.5em;border-bottom:1px solid #ddd;padding-bottom:.2em}\n");
    html.push_str("h3{font-size:1.25em}\n");
    html.push_str("code{background:#f5f5f5;padding:2px 6px;border-radius:3px;font-family:Consolas,monospace;font-size:.9em}\n");
    html.push_str("pre{background:#f5f5f5;padding:1em;border-radius:4px;overflow-x:auto}\n");
    html.push_str("pre code{background:none;padding:0}\n");
    html.push_str("table{border-collapse:collapse;width:100%}\n");
    html.push_str("th,td{border:1px solid #ddd;padding:8px 12px;text-align:left}\n");
    html.push_str("th{background:#4a90d9;color:#fff}\n");
    html.push_str("tr:nth-child(even){background:#f9f9f9}\n");
    html.push_str("blockquote{border-left:4px solid #4a90d9;margin:0;padding:0 1em;color:#666}\n");
    html.push_str("img{max-width:100%}\n");
    html.push_str("a{color:#0563c1}\n");
    html.push_str("</style>\n</head>\n<body>\n");
    pulldown_cmark::html::push_html(&mut html, parser);
    html.push_str("</body>\n</html>");
    html
}

/// 从 MdDocument 提取所有表格
pub fn extract_tables(doc: &MdDocument) -> Vec<&MdBlock> {
    doc.blocks
        .iter()
        .filter(|b| matches!(b, MdBlock::Table { .. }))
        .collect()
}

/// 从 MdDocument 提取所有代码块
pub fn extract_code_blocks(doc: &MdDocument) -> Vec<(&str, &str)> {
    doc.blocks
        .iter()
        .filter_map(|b| {
            if let MdBlock::CodeBlock { language, code } = b {
                Some((language.as_str(), code.as_str()))
            } else {
                None
            }
        })
        .collect()
}

// ── 内部辅助 ─────────────────────────────────────────────────────────────────

fn level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn flush_inlines(
    inlines: &mut Vec<MdInline>,
    current_block: &mut Option<MdBlock>,
    _doc: &mut MdDocument,
) {
    if inlines.is_empty() {
        return;
    }
    let taken = std::mem::take(inlines);
    match current_block {
        Some(MdBlock::Heading { text, .. }) => {
            *text = inlines_to_string(&taken);
        },
        Some(MdBlock::Paragraph { inlines: p_inlines }) => {
            p_inlines.extend(taken);
        },
        Some(MdBlock::CodeBlock { code, .. }) => {
            code.push_str(&inlines_to_string(&taken));
        },
        Some(MdBlock::Blockquote { inlines: b_inlines }) => {
            b_inlines.extend(taken);
        },
        _ => {},
    }
}

fn flush_and_store(current_block: &mut Option<MdBlock>, doc: &mut MdDocument) {
    if let Some(block) = current_block.take() {
        // 跳过空段落
        if let MdBlock::Paragraph { ref inlines } = block {
            if inlines.is_empty()
                || inlines
                    .iter()
                    .all(|i| matches!(i, MdInline::Text(t) if t.trim().is_empty()))
            {
                return;
            }
        }
        doc.blocks.push(block);
    }
}

fn flush_paragraph_block(
    current_block: &mut Option<MdBlock>,
    inlines: &mut Vec<MdInline>,
    doc: &mut MdDocument,
) {
    let mut p_inlines = Vec::new();
    if let Some(MdBlock::Paragraph { inlines: pis }) = current_block {
        p_inlines = std::mem::take(pis);
    }
    if !inlines.is_empty() {
        p_inlines.extend(std::mem::take(inlines));
    }
    if !p_inlines.is_empty() {
        doc.blocks.push(MdBlock::Paragraph { inlines: p_inlines });
    }
    *current_block = None;
}

fn flush_blockquote_block(
    current_block: &mut Option<MdBlock>,
    inlines: &mut Vec<MdInline>,
    doc: &mut MdDocument,
) {
    let mut b_inlines = Vec::new();
    if let Some(MdBlock::Blockquote { inlines: bis }) = current_block {
        b_inlines = std::mem::take(bis);
    }
    if !inlines.is_empty() {
        b_inlines.extend(std::mem::take(inlines));
    }
    if !b_inlines.is_empty() {
        doc.blocks.push(MdBlock::Blockquote { inlines: b_inlines });
    }
    *current_block = None;
}

fn push_inline(
    inline: MdInline,
    current_block: &mut Option<MdBlock>,
    current_inlines: &mut Vec<MdInline>,
    current_item_inlines: &mut Vec<MdInline>,
    table_cells: &mut Vec<String>,
) {
    let text = inline_to_string(&inline);
    // 表格单元格
    if matches!(current_block, Some(MdBlock::Table { .. }))
        || !table_cells.is_empty()
        || !current_item_inlines.is_empty()
    {
        if !current_item_inlines.is_empty() || matches!(current_block, Some(MdBlock::List { .. })) {
            current_item_inlines.push(inline);
        } else {
            // 在表格内
            table_cells.push(text);
        }
        return;
    }
    // 标题/代码块 → 刷新到 block 文本
    if matches!(current_block, Some(MdBlock::Heading { .. }) | Some(MdBlock::CodeBlock { .. })) {
        let text = inline_to_string(&inline);
        match current_block {
            Some(MdBlock::Heading { text: t, .. }) => t.push_str(&text),
            Some(MdBlock::CodeBlock { code, .. }) => code.push_str(&text),
            _ => {},
        }
        return;
    }
    current_inlines.push(inline);
}

fn inline_to_string(inline: &MdInline) -> String {
    match inline {
        MdInline::Text(s) => s.clone(),
        MdInline::Bold(s) => s.clone(),
        MdInline::Italic(s) => s.clone(),
        MdInline::Code(s) => s.clone(),
        MdInline::Link { text, .. } => text.clone(),
        MdInline::Image { alt, .. } => alt.clone(),
    }
}

fn inlines_to_string(inlines: &[MdInline]) -> String {
    inlines
        .iter()
        .map(inline_to_string)
        .collect::<Vec<_>>()
        .join("")
}
