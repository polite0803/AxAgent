//! Output-side token consumption control pipeline.
//!
//! Implements post-processing of LLM output to reduce token waste through:
//! - Duplicate explanation detection and merging
//! - Redundant comment / blank line stripping in code blocks
//! - Segmented generation with context recycling markers

use std::collections::HashSet;

/// Configuration for the output processing pipeline.
#[derive(Debug, Clone)]
pub struct OutputProcessorConfig {
    /// Whether to detect and merge duplicate explanations.
    pub deduplicate_explanations: bool,
    /// Whether to strip redundant comments from generated code.
    pub strip_redundant_comments: bool,
    /// Whether to strip excessive blank lines from generated code.
    pub strip_blank_lines: bool,
    /// Maximum number of consecutive blank lines to preserve.
    pub max_consecutive_blank_lines: usize,
    /// Whether to emit segment boundaries for long outputs.
    pub segment_output: bool,
    /// Character count threshold above which segmentation markers are inserted.
    pub segment_threshold_chars: usize,
}

impl Default for OutputProcessorConfig {
    fn default() -> Self {
        Self {
            deduplicate_explanations: true,
            strip_redundant_comments: true,
            strip_blank_lines: true,
            max_consecutive_blank_lines: 1,
            segment_output: true,
            segment_threshold_chars: 4000,
        }
    }
}

impl OutputProcessorConfig {
    /// A configuration that applies all optimizations (code mode).
    #[must_use]
    pub fn aggressive() -> Self {
        Self {
            deduplicate_explanations: true,
            strip_redundant_comments: true,
            strip_blank_lines: true,
            max_consecutive_blank_lines: 0,
            segment_output: true,
            segment_threshold_chars: 2000,
        }
    }

    /// A configuration that applies minimal changes (general mode).
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            deduplicate_explanations: false,
            strip_redundant_comments: false,
            strip_blank_lines: false,
            max_consecutive_blank_lines: 3,
            segment_output: false,
            segment_threshold_chars: usize::MAX,
        }
    }
}

/// Result of processing a block of output text.
#[derive(Debug, Clone)]
pub struct ProcessedOutput {
    pub text: String,
    pub original_chars: usize,
    pub processed_chars: usize,
    pub segments: Vec<String>,
    pub deduplications_removed: usize,
}

/// The output processing pipeline.
pub struct OutputProcessor {
    config: OutputProcessorConfig,
}

impl Default for OutputProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputProcessor {
    pub fn new() -> Self {
        Self {
            config: OutputProcessorConfig::default(),
        }
    }

    pub fn with_config(config: OutputProcessorConfig) -> Self {
        Self { config }
    }

    /// Process raw LLM output through the configured pipeline.
    pub fn process(&self, raw: &str) -> ProcessedOutput {
        let original_chars = raw.chars().count();
        let mut text = raw.to_string();
        let mut deduplications_removed = 0;

        if self.config.strip_redundant_comments || self.config.strip_blank_lines {
            text = self.process_code_blocks(&text);
        }

        if self.config.deduplicate_explanations {
            let (deduped, count) = self.deduplicate(&text);
            text = deduped;
            deduplications_removed = count;
        }

        if self.config.strip_blank_lines {
            text = collapse_blank_lines(&text, self.config.max_consecutive_blank_lines);
        }

        let segments = if self.config.segment_output
            && text.chars().count() > self.config.segment_threshold_chars
        {
            segment_text(&text, self.config.segment_threshold_chars)
        } else {
            vec![text.clone()]
        };

        let processed_chars = text.chars().count();

        ProcessedOutput {
            text,
            original_chars,
            processed_chars,
            segments,
            deduplications_removed,
        }
    }

    /// Detect and merge duplicate explanatory paragraphs.
    fn deduplicate(&self, text: &str) -> (String, usize) {
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        if paragraphs.len() < 2 {
            return (text.to_string(), 0);
        }

        let mut seen: HashSet<String> = HashSet::new();
        let mut result: Vec<&str> = Vec::new();
        let mut removed = 0;

        for para in &paragraphs {
            let normalized = para
                .trim()
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>();

            if normalized.len() < 20 {
                result.push(para);
                continue;
            }

            if seen.contains(&normalized) {
                removed += 1;
            } else {
                seen.insert(normalized);
                result.push(para);
            }
        }

        (result.join("\n\n"), removed)
    }

    /// Process code blocks within text: strip redundant comments and blank lines.
    fn process_code_blocks(&self, text: &str) -> String {
        let mut result = String::new();
        let mut in_code_block = false;
        let mut code_buffer = String::new();
        let mut fence_line = String::new();

        for line in text.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("```") {
                if in_code_block {
                    // End of code block: process buffer and flush
                    let processed = self.strip_code_content(&code_buffer);
                    result.push_str(&fence_line);
                    result.push('\n');
                    result.push_str(&processed);
                    result.push_str(line);
                    result.push('\n');
                    code_buffer.clear();
                    fence_line.clear();
                    in_code_block = false;
                } else {
                    // Start of code block
                    fence_line = line.to_string();
                    in_code_block = true;
                }
            } else if in_code_block {
                code_buffer.push_str(line);
                code_buffer.push('\n');
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Handle unclosed code block
        if in_code_block && !code_buffer.is_empty() {
            result.push_str(&fence_line);
            result.push('\n');
            result.push_str(&code_buffer);
        }

        result
    }

    /// Strip redundant content from within a code block.
    fn strip_code_content(&self, code: &str) -> String {
        let mut result = String::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // Skip lines that are purely redundant comments
            if self.config.strip_redundant_comments {
                if trimmed == "//" || trimmed == "#" {
                    continue;
                }
                if trimmed == "// TODO: implement" || trimmed == "# TODO: implement" {
                    continue;
                }
                if trimmed.starts_with("// This is") || trimmed.starts_with("# This is") {
                    continue;
                }
                // Keep doc comments and meaningful comments
                if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                    result.push_str(line);
                    result.push('\n');
                    continue;
                }
            }

            result.push_str(line);
            result.push('\n');
        }

        result
    }
}

/// Collapse consecutive blank lines down to a maximum count.
fn collapse_blank_lines(text: &str, max_blank: usize) -> String {
    let mut result = String::new();
    let mut blank_count = 0;

    for line in text.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= max_blank {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Segment a long text output into chunks with context markers.
///
/// Each segment includes a `[Segment N/M]` marker so the downstream
/// consumer can individually process and recycle context after each
/// segment.
fn segment_text(text: &str, threshold: usize) -> Vec<String> {
    let total_chars = text.chars().count();
    if total_chars <= threshold {
        return vec![text.to_string()];
    }

    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let _segment_count = total_chars.div_ceil(threshold).max(2);

    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_len = 0;

    for para in &paragraphs {
        let para_len = para.chars().count();

        if current_len + para_len > threshold && !current.is_empty() {
            segments.push(std::mem::take(&mut current));
            current_len = 0;
        }

        if !current.is_empty() {
            current.push_str("\n\n");
            current_len += 2;
        }
        current.push_str(para);
        current_len += para_len;
    }

    if !current.is_empty() {
        segments.push(current);
    }

    let total_segments = segments.len();
    segments
        .into_iter()
        .enumerate()
        .map(|(i, seg)| {
            format!(
                "[Segment {}/{}]\n{seg}",
                i + 1,
                total_segments
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplicate_explanations() {
        let processor = OutputProcessor::with_config(OutputProcessorConfig::aggressive());
        let input = "This is a long explanation that repeats.\n\nThis is a long explanation that repeats.\n\nSomething else.";
        let output = processor.process(input);
        assert!(output.deduplications_removed > 0);
        assert!(output.processed_chars < output.original_chars);
    }

    #[test]
    fn test_strip_redundant_comments() {
        let processor = OutputProcessor::with_config(OutputProcessorConfig::aggressive());
        let input = "```rust\n//\n// TODO: implement\nfn main() {}\n// This is a comment\n```";
        let output = processor.process(input);
        assert!(output.text.contains("fn main()"));
        assert!(!output.text.contains("TODO: implement"));
    }

    #[test]
    fn test_collapse_blank_lines() {
        let result = collapse_blank_lines("a\n\n\n\nb\n\n\nc", 1);
        let blank_count = result.lines().filter(|l| l.trim().is_empty()).count();
        assert_eq!(blank_count, 2);
    }

    #[test]
    fn test_segment_long_text() {
        let text = "A\n\nB\n\nC\n\nD\n\nE\n\nF\n\nG\n\nH\n\nI\n\nJ";
        let segments = segment_text(text, 10);
        assert!(segments.len() > 1);
        assert!(segments[0].contains("[Segment 1/"));
    }

    #[test]
    fn test_minimal_config_preserves_content() {
        let processor = OutputProcessor::with_config(OutputProcessorConfig::minimal());
        let input = "Hello\n\n\n\nWorld";
        let output = processor.process(input);
        assert_eq!(output.text.trim(), input.trim());
        assert_eq!(output.deduplications_removed, 0);
    }
}
