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

/// Find the nearest char boundary at or before the given byte position.
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        let mut i = index;
        while i > 0 && !s.is_char_boundary(i) {
            i -= 1;
        }
        i
    }
}

/// Find the nearest char boundary at or after the given byte position.
fn ceil_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        let mut i = index;
        while i < s.len() && !s.is_char_boundary(i) {
            i += 1;
        }
        i
    }
}

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
    if text.len() <= chunk_size {
        return vec![TextChunk {
            index: 0,
            content: text.to_string(),
        }];
    }

    // If a custom separator is provided, use separator-first chunking
    if let Some(sep) = separator {
        if !sep.is_empty() {
            return chunk_by_separator(text, chunk_size, overlap, sep);
        }
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
        let sec_len = section.len();
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
                chunks.push(TextChunk {
                    index: chunks.len() as i32,
                    content: sub.content,
                });
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
        let seg_len = segment.len();
        let sep_len = if current_parts.is_empty() { 0 } else { sep.len() };

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
                chunks.push(TextChunk {
                    index: chunks.len() as i32,
                    content: sub.content,
                });
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

/// Core smart chunking implementation (no separator).
fn chunk_text_impl(text: &str, chunk_size: usize, overlap: usize) -> Vec<TextChunk> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    if text.len() <= chunk_size {
        return vec![TextChunk {
            index: 0,
            content: text.to_string(),
        }];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < text.len() {
        // Snap to char boundary to avoid slicing inside multi-byte chars (e.g. CJK)
        let end = floor_char_boundary(text, (start + chunk_size).min(text.len()));

        // Find a good break point near `end`
        let actual_end = if end >= text.len() {
            text.len()
        } else {
            find_break_point(text, start, end)
        };

        let chunk_content = text[start..actual_end].trim();
        if !chunk_content.is_empty() {
            chunks.push(TextChunk {
                index: chunks.len() as i32,
                content: chunk_content.to_string(),
            });
        }

        // Move start forward by (chunk_size - overlap), but at least 1 char
        let advance = if actual_end - start > overlap {
            actual_end - start - overlap
        } else {
            actual_end - start
        };

        // Snap new start to a char boundary
        start = ceil_char_boundary(text, start + advance.max(1));

        // If remaining text is tiny, it's already covered by the last chunk's overlap
        if start >= text.len() || text.len() - start < overlap {
            break;
        }
    }

    chunks
}

/// Find a good break point near `target` position, searching backwards from target.
/// Prefers: paragraph break (\n\n) > line break (\n) > sentence end (. ! ?) > space
fn find_break_point(text: &str, start: usize, target: usize) -> usize {
    let search_range = &text[start..target];
    let min_chunk = (target - start) / 2; // Don't break before half the chunk

    // Try paragraph break
    if let Some(pos) = search_range.rfind("\n\n") {
        if pos >= min_chunk {
            return start + pos + 2; // After the double newline
        }
    }

    // Try line break
    if let Some(pos) = search_range.rfind('\n') {
        if pos >= min_chunk {
            return start + pos + 1;
        }
    }

    // Try sentence end
    let bytes = search_range.as_bytes();
    for i in (min_chunk..bytes.len()).rev() {
        if matches!(bytes[i], b'.' | b'!' | b'?') {
            // Check it's followed by a space or end
            if i + 1 >= bytes.len() || bytes[i + 1] == b' ' || bytes[i + 1] == b'\n' {
                return start + i + 1;
            }
        }
    }

    // Try word break (space)
    if let Some(pos) = search_range.rfind(' ') {
        if pos >= min_chunk {
            return start + pos + 1;
        }
    }

    // No good break found, just cut at target
    target
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
    fn test_chunking_mixed_ascii_cjk() {
        let text = "Hello世界！这是一段混合中英文的测试文本。".repeat(50);
        let chunks = chunk_text(&text, 150, 30);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
        }
    }
}
