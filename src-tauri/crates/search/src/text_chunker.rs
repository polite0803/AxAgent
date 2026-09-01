// SPDX-License-Identifier: AGPL-3.0-only

/// A chunk of text with its position index.
#[derive(Debug, Clone)]
pub struct TextChunk {
    pub index: i32,
    pub content: String,
}

/// Default chunk size in characters (~500 tokens).
pub const DEFAULT_CHUNK_SIZE: usize = 2000;
/// Default overlap in characters (~50 tokens).
pub const DEFAULT_OVERLAP: usize = 200;

/// Code-specific chunk size in characters (~20-37 tokens for 80-150 chars).
pub const CODE_CHUNK_SIZE: usize = 120;
/// Code-specific overlap in characters (~12% overlap).
pub const CODE_OVERLAP: usize = 12;

/// Split text into overlapping chunks, breaking at paragraph/sentence boundaries.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<TextChunk> {
    chunk_text_with_separator(text, chunk_size, overlap, None)
}

/// Split text into overlapping chunks, with optional custom separator support.
///
/// When `separator` is `Some`, the text is first split by the separator,
/// then segments are grouped into chunks that fit within `chunk_size`.
/// When `separator` is `None`, falls back to the default smart chunking.
pub fn chunk_text_with_separator(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    separator: Option<&str>,
) -> Vec<TextChunk> {
    chunk_text_with_separator_and_markdown(text, chunk_size, overlap, separator, false)
}

/// Split text with optional Markdown heading-aware chunking.
///
/// When `is_markdown` is true and no custom separator is provided,
/// the text is first split by Markdown headings (#, ##, ###, etc.)
/// to preserve semantic section boundaries.
pub fn chunk_text_with_separator_and_markdown(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    separator: Option<&str>,
    is_markdown: bool,
) -> Vec<TextChunk> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    // chunk_size 语义为「字符数」（见常量文档），中文等多字节字符按字符计数
    if text.chars().count() <= chunk_size {
        return vec![TextChunk { index: 0, content: text.to_string() }];
    }

    // If a custom separator is provided, use separator-first chunking
    if let Some(sep) = separator
        && !sep.is_empty()
    {
        return chunk_by_separator(text, chunk_size, overlap, sep);
    }

    // If Markdown, use heading-aware chunking
    if is_markdown {
        return chunk_by_markdown_headings(text, chunk_size, overlap);
    }

    // Default smart chunking
    chunk_text_impl(text, chunk_size, overlap)
}

/// Chunk Markdown text by splitting on heading boundaries (#, ##, ###, etc.),
/// then grouping sections into chunks that fit within chunk_size.
fn chunk_by_markdown_headings(text: &str, chunk_size: usize, overlap: usize) -> Vec<TextChunk> {
    // Split text into sections at heading lines
    let mut sections: Vec<String> = Vec::new();
    let mut current_section = String::new();

    for line in text.lines() {
        // Check if line starts with 1-6 # characters followed by a space
        let is_heading = line.starts_with("# ")
            || line.starts_with("## ")
            || line.starts_with("### ")
            || line.starts_with("#### ")
            || line.starts_with("##### ")
            || line.starts_with("###### ");

        if is_heading && !current_section.trim().is_empty() {
            sections.push(std::mem::take(&mut current_section));
        }

        if !current_section.is_empty() {
            current_section.push('\n');
        }
        current_section.push_str(line);
    }

    // Don't forget the last section
    if !current_section.trim().is_empty() {
        sections.push(current_section);
    }

    if sections.is_empty() {
        return vec![];
    }

    // Group sections into chunks that fit within chunk_size
    let mut chunks = Vec::new();
    let mut current_parts: Vec<&str> = Vec::new();
    let mut current_len = 0usize;

    for section in &sections {
        // 按字符计数（chunk_size 语义为字符数）
        let sec_len = section.chars().count();
        let newline_len = if current_parts.is_empty() { 0 } else { 1 };

        // If a single section exceeds chunk_size, split it further with smart chunking
        if sec_len > chunk_size {
            // Flush current buffer first
            if !current_parts.is_empty() {
                chunks.push(TextChunk {
                    index: chunks.len() as i32,
                    content: current_parts.join("\n").trim().to_string(),
                });
                current_parts.clear();
                current_len = 0;
            }
            // Smart-chunk the oversized section
            let sub_chunks = chunk_text_impl(section.trim(), chunk_size, overlap);
            for sub in sub_chunks {
                chunks.push(TextChunk { index: chunks.len() as i32, content: sub.content });
            }
            continue;
        }

        // If adding this section would exceed chunk_size, flush current buffer
        if current_len + newline_len + sec_len > chunk_size && !current_parts.is_empty() {
            chunks.push(TextChunk {
                index: chunks.len() as i32,
                content: current_parts.join("\n").trim().to_string(),
            });
            current_parts.clear();
            current_len = 0;
        }

        current_parts.push(section);
        current_len += newline_len + sec_len;
    }

    // Flush remaining
    if !current_parts.is_empty() {
        chunks.push(TextChunk {
            index: chunks.len() as i32,
            content: current_parts.join("\n").trim().to_string(),
        });
    }

    chunks
}

/// Chunk text by first splitting on a custom separator, then grouping
/// segments into chunks that fit within chunk_size.
fn chunk_by_separator(text: &str, chunk_size: usize, overlap: usize, sep: &str) -> Vec<TextChunk> {
    let segments: Vec<&str> = text.split(sep).filter(|s| !s.trim().is_empty()).collect();
    if segments.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut current_parts: Vec<&str> = Vec::new();
    let mut current_len = 0usize;

    for segment in &segments {
        // 按字符计数（chunk_size 语义为字符数）
        let seg_len = segment.chars().count();
        let sep_len = if current_parts.is_empty() {
            0
        } else {
            sep.chars().count()
        };

        // If a single segment exceeds chunk_size, split it further with smart chunking
        if seg_len > chunk_size {
            // Flush current buffer first
            if !current_parts.is_empty() {
                chunks.push(TextChunk {
                    index: chunks.len() as i32,
                    content: current_parts.join(sep).trim().to_string(),
                });
                current_parts.clear();
                current_len = 0;
            }
            // Smart-chunk the oversized segment
            let sub_chunks = chunk_text_impl(segment.trim(), chunk_size, overlap);
            for sub in sub_chunks {
                chunks.push(TextChunk { index: chunks.len() as i32, content: sub.content });
            }
            continue;
        }

        // If adding this segment would exceed chunk_size, flush current buffer
        if current_len + sep_len + seg_len > chunk_size && !current_parts.is_empty() {
            chunks.push(TextChunk {
                index: chunks.len() as i32,
                content: current_parts.join(sep).trim().to_string(),
            });
            current_parts.clear();
            current_len = 0;
        }

        current_parts.push(segment);
        current_len += sep_len + seg_len;
    }

    // Flush remaining
    if !current_parts.is_empty() {
        chunks.push(TextChunk {
            index: chunks.len() as i32,
            content: current_parts.join(sep).trim().to_string(),
        });
    }

    chunks
}

/// 字符索引 → 字节偏移对照表（含末尾哨兵 text.len()，长度 = 字符数 + 1）。
fn char_byte_offsets(text: &str) -> Vec<usize> {
    text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len())).collect()
}

/// 将 `find_break_point` 系列返回的字节偏移换算为字符索引。
///
/// 返回值均落在字符边界上；理论上应精确命中对照表，兜底向下取整并保证
/// 至少前进 1 个字符（避免死循环）。
fn byte_offset_to_char_index(offsets: &[usize], byte_pos: usize, min: usize) -> usize {
    let idx = match offsets.binary_search(&byte_pos) {
        Ok(i) => i,
        // 非边界兜底：向下取整（保证不超过切分目标）
        Err(i) => i.saturating_sub(1),
    };
    idx.max(min + 1).min(offsets.len() - 1)
}

/// Core smart chunking implementation (no separator).
///
/// `chunk_size` / `overlap` 单位为字符（非字节），中文等多字节文本不再被压缩成
/// 1/3 长度；内部仍以字节偏移做切片，通过对照表换算。
fn chunk_text_impl(text: &str, chunk_size: usize, overlap: usize) -> Vec<TextChunk> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    let total_chars = text.chars().count();
    if total_chars <= chunk_size {
        return vec![TextChunk { index: 0, content: text.to_string() }];
    }

    let offsets = char_byte_offsets(text);
    let mut chunks = Vec::new();
    let mut start = 0usize; // 字符索引

    while start < total_chars {
        let end = (start + chunk_size).min(total_chars);

        let actual_end = if end >= total_chars {
            total_chars
        } else {
            let bp = find_break_point(text, offsets[start], offsets[end]);
            byte_offset_to_char_index(&offsets, bp, start)
        };

        let chunk_content = text[offsets[start]..offsets[actual_end]].trim();
        if !chunk_content.is_empty() {
            chunks
                .push(TextChunk { index: chunks.len() as i32, content: chunk_content.to_string() });
        }

        // Move start forward by (chunk_size - overlap) chars, but at least 1 char
        let advance = if actual_end - start > overlap {
            actual_end - start - overlap
        } else {
            actual_end - start
        };
        start += advance.max(1);

        // If remaining text is tiny, it's already covered by the last chunk's overlap
        if start >= total_chars || total_chars - start < overlap {
            break;
        }
    }

    chunks
}

/// Find a good break point near `target` position, searching backwards from target.
/// Prefers: paragraph break (\n\n) > line break (\n) > sentence end (. ! ? 。！？；) > space/CJK space
fn find_break_point(text: &str, start: usize, target: usize) -> usize {
    let search_range = &text[start..target];
    let min_chunk = (target - start) / 2; // Don't break before half the chunk

    // Try paragraph break (double newline, handles both \n\n and \r\n\r\n)
    for marker in ["\n\n", "\r\n\r\n"] {
        if let Some(pos) = search_range.rfind(marker)
            && pos >= min_chunk
        {
            return start + pos + marker.len();
        }
    }

    // Try horizontal rule / separator lines
    for marker in ["\n---\n", "\n***\n", "\n___\n"] {
        if let Some(pos) = search_range.rfind(marker)
            && pos >= min_chunk
        {
            return start + pos + marker.len();
        }
    }

    // Try line break
    if let Some(pos) = search_range.rfind('\n')
        && pos >= min_chunk
    {
        return start + pos + 1;
    }

    // Try sentence end (ASCII + CJK punctuation)
    // Search character-by-character backwards for CJK punctuation
    let chars: Vec<(usize, char)> = search_range.char_indices().collect();
    for (pos, ch) in chars.iter().rev() {
        if *pos < min_chunk {
            break;
        }
        if matches!(ch, '。' | '！' | '？' | '；' | '!' | '?' | ';' | '.') {
            let byte_end = pos + ch.len_utf8();
            if byte_end <= search_range.len() {
                let next_char = search_range[byte_end..].chars().next();
                let is_sentence_end = match next_char {
                    Some(nc) => {
                        nc.is_whitespace()
                            || nc == '"'
                            || nc == '\''
                            || nc == '”'
                            || nc == '’'
                            || nc == '）'
                            || nc == '】'
                    },
                    None => true,
                };
                if is_sentence_end {
                    return start + byte_end;
                }
            }
        }
    }

    // Try word break (space) — for English text
    if let Some(pos) = search_range.rfind(' ')
        && pos >= min_chunk
    {
        return start + pos + 1;
    }

    // For CJK text without spaces, try breaking after common suffix particles
    // (的、了、吗、呢、啊、吧 等) — but only as a last resort
    for (pos, ch) in chars.iter().rev() {
        if *pos < min_chunk {
            break;
        }
        if matches!(ch, '，' | '、' | '：' | '）' | '】' | '」' | '』') {
            return start + pos + ch.len_utf8();
        }
    }

    // No good break found, just cut at target
    target
}

/// Find a good break point for code, preferring function/class boundaries and newlines.
fn find_code_break_point(text: &str, start: usize, target: usize) -> usize {
    let search_range = &text[start..target];
    let min_chunk = (target - start) / 3;

    if let Some(pos) = search_range.rfind("\n\n")
        && pos >= min_chunk
    {
        return start + pos + 2;
    }

    if let Some(pos) = search_range.rfind('\n')
        && pos >= min_chunk
    {
        return start + pos + 1;
    }

    // Prefer breaking after semicolons (statement end) or closing braces
    let bytes = search_range.as_bytes();
    for i in (min_chunk..bytes.len()).rev() {
        if matches!(bytes[i], b';' | b'}') {
            return start + i + 1;
        }
    }

    target
}

/// Chunk code text with code-optimized parameters (80-150 char chunks, ~10% overlap).
///
/// Uses `CODE_CHUNK_SIZE` (120) and `CODE_OVERLAP` (12) as defaults, but allows
/// the caller to override via the optional `config` parameter.
pub fn chunk_for_code(
    text: &str,
    chunk_size: Option<usize>,
    overlap: Option<usize>,
) -> Vec<TextChunk> {
    let chunk_size = chunk_size.unwrap_or(CODE_CHUNK_SIZE);
    let overlap = overlap.unwrap_or(CODE_OVERLAP);
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    let total_chars = text.chars().count();
    if total_chars <= chunk_size {
        return vec![TextChunk { index: 0, content: text.to_string() }];
    }

    let offsets = char_byte_offsets(text);
    let mut chunks = Vec::new();
    let mut start = 0usize; // 字符索引

    while start < total_chars {
        let end = (start + chunk_size).min(total_chars);

        let actual_end = if end >= total_chars {
            total_chars
        } else {
            let bp = find_code_break_point(text, offsets[start], offsets[end]);
            byte_offset_to_char_index(&offsets, bp, start)
        };

        let chunk_content = text[offsets[start]..offsets[actual_end]].trim();
        if !chunk_content.is_empty() {
            chunks
                .push(TextChunk { index: chunks.len() as i32, content: chunk_content.to_string() });
        }

        let advance = if actual_end - start > overlap {
            actual_end - start - overlap
        } else {
            actual_end - start
        };
        start += advance.max(1);

        if start >= total_chars || total_chars - start < overlap {
            break;
        }
    }

    chunks
}

/// Chunk text by MIME type, automatically selecting the optimal strategy.
///
/// - Code MIME types (text/x-*, application/javascript, etc.) use code-optimized chunking
/// - Markdown (text/markdown, text/x-markdown) uses heading-aware chunking
/// - Plain text and everything else uses smart sentence/paragraph chunking
pub fn chunk_by_mime_type(
    text: &str,
    mime_type: &str,
    chunk_size: Option<usize>,
    overlap: Option<usize>,
) -> Vec<TextChunk> {
    let lower_mime = mime_type.to_lowercase();

    let is_code = lower_mime.starts_with("text/x-")
        || matches!(
            lower_mime.as_str(),
            "application/javascript"
                | "application/typescript"
                | "application/x-typescript"
                | "application/json"
                | "application/xml"
                | "application/x-yaml"
                | "text/javascript"
                | "text/typescript"
        )
        || lower_mime.contains("rust")
        || lower_mime.contains("python")
        || lower_mime.contains("java")
        || lower_mime.contains("csharp")
        || lower_mime.contains("cpp")
        || lower_mime.contains("go-source")
        || lower_mime.contains("php");

    if is_code {
        return chunk_for_code(text, chunk_size, overlap);
    }

    let is_markdown =
        matches!(lower_mime.as_str(), "text/markdown" | "text/x-markdown" | "text/markdown+github")
            || lower_mime.ends_with(".md")
            || text.starts_with("# ")
            || text.starts_with("## ");

    let chunk_size = chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE);
    let overlap = overlap.unwrap_or(DEFAULT_OVERLAP);

    if is_markdown {
        return chunk_text_with_separator_and_markdown(text, chunk_size, overlap, None, true);
    }

    chunk_text(text, chunk_size, overlap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        assert!(chunk_text("", 100, 20).is_empty());
    }

    #[test]
    fn test_short_text() {
        let chunks = chunk_text("Hello world", 100, 20);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Hello world");
    }

    #[test]
    fn test_chunking_preserves_content() {
        let text = "A".repeat(500);
        let chunks = chunk_text(&text, 200, 50);
        assert!(chunks.len() > 1);
        // First chunk should be roughly 200 chars
        assert!(chunks[0].content.len() <= 200);
    }

    #[test]
    fn test_chunking_cjk_no_panic() {
        // CJK characters are 3 bytes each in UTF-8.
        // A chunk_size of 100 bytes lands inside a multi-byte char → must not panic.
        let text = "中".repeat(200); // 600 bytes
        let chunks = chunk_text(&text, 100, 20);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            // Every chunk must be valid UTF-8 (no partial chars)
            assert!(chunk.content.is_char_boundary(0));
            assert!(chunk.content.is_char_boundary(chunk.content.len()));
        }
    }

    #[test]
    fn test_chunk_size_counts_chars_not_bytes() {
        // 5000 个汉字 ≈ 15000 字节：chunk_size=2000（字符）应只产生 ~3 块；
        // 旧的字节计数实现会产生 ~8 块（每块 ~667 字）。
        let text = "中".repeat(5000);
        let chunks = chunk_text(&text, 2000, 200);
        assert!(
            chunks.len() <= 4,
            "字符计数下 5000 字 / 2000 应为 ~3 块，实际 {} 块",
            chunks.len()
        );
        for chunk in &chunks {
            assert!(chunk.content.chars().count() <= 2000);
        }
    }

    #[test]
    fn test_mixed_cjk_ascii_char_semantics() {
        // 中英混合：每个片段 ~10 字节汉字 + 10 字节 ASCII，保证无 panic 且内容无损
        let text = "中文内容abc def。".repeat(300);
        let chunks = chunk_text(&text, 500, 50);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.content.chars().count() <= 500);
            assert!(chunk.content.is_char_boundary(0));
            assert!(chunk.content.is_char_boundary(chunk.content.len()));
        }
    }

    #[test]
    fn test_chunking_mixed_ascii_cjk() {
        let text = "Hello世界！这是一段混合中英文的测试文本。".repeat(50);
        let chunks = chunk_text(&text, 150, 30);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_code_chunking_small_chunks() {
        let code = "fn main() {\n    let x = 1;\n    let y = 2;\n    println!(\"{}\", x + y);\n}\n";
        let repeated = code.repeat(30);
        let chunks = chunk_for_code(&repeated, None, None);
        assert!(chunks.len() > 5);
        for chunk in &chunks {
            assert!(chunk.content.len() <= CODE_CHUNK_SIZE + 50);
        }
    }

    #[test]
    fn test_code_chunking_preserves_statements() {
        let code = "fn hello() { println!(\"hello\"); }\nfn world() { println!(\"world\"); }\n";
        let chunks = chunk_for_code(code, None, None);
        let joined: String = chunks.iter().map(|c| c.content.as_str()).collect();
        assert!(joined.contains("fn hello"));
        assert!(joined.contains("fn world"));
    }
}
