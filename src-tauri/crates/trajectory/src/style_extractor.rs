use crate::style_vectorizer::{CodeSample, MessageSample};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedCodePatterns {
    pub function_patterns: Vec<FunctionPattern>,
    pub naming_patterns: Vec<NamingPattern>,
    pub structure_patterns: Vec<StructurePattern>,
    pub comment_patterns: Vec<CommentPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionPattern {
    pub name: String,
    pub param_count: usize,
    pub has_return_type: bool,
    pub is_async: bool,
    pub visibility: Visibility,
    pub line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingPattern {
    pub pattern_type: NamingPatternType,
    pub example: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NamingPatternType {
    Snake,
    Camel,
    Pascal,
    Kebab,
    Upper,
    Lower,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructurePattern {
    pub pattern_type: StructurePatternType,
    pub description: String,
    pub frequency: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StructurePatternType {
    EarlyReturn,
    GuardClauses,
    NestedCallbacks,
    BuilderPattern,
    ChainMethod,
    StrategyPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentPattern {
    pub style: CommentStyle,
    pub frequency: f32,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommentStyle {
    SingleLine,
    MultiLine,
    Documentation,
    Inline,
}

pub struct StyleExtractor {
    min_pattern_frequency: u32,
}

impl StyleExtractor {
    pub fn new() -> Self {
        Self {
            min_pattern_frequency: 2,
        }
    }

    pub fn extract_from_code(&self, samples: &[CodeSample]) -> ExtractedCodePatterns {
        let function_patterns = self.extract_function_patterns(samples);
        let naming_patterns = self.extract_naming_patterns(samples);
        let structure_patterns = self.extract_structure_patterns(samples);
        let comment_patterns = self.extract_comment_patterns(samples);

        ExtractedCodePatterns {
            function_patterns,
            naming_patterns,
            structure_patterns,
            comment_patterns,
        }
    }

    pub fn extract_from_messages(&self, messages: &[MessageSample]) -> DocumentStyleProfile {
        DocumentStyleProfile {
            formality_level: self.estimate_formality(messages),
            structure_level: self.estimate_structure_level(messages),
            technical_vocabulary_ratio: self.estimate_technical_vocabulary(messages),
            explanation_detail_level: self.estimate_explanation_detail(messages),
            preferred_format: self.detect_preferred_format(messages),
        }
    }

    pub fn extract_naming_conventions(&self, samples: &[CodeSample]) -> Vec<NamingPattern> {
        self.extract_naming_patterns(samples)
    }

    pub fn extract_formatting_preferences(&self, samples: &[CodeSample]) -> FormattingPreferences {
        FormattingPreferences {
            indent_size: self.detect_indent_size(samples),
            indent_style: self.detect_indent_style(samples),
            line_ending: self.detect_line_ending(samples),
            max_line_length: self.estimate_max_line_length(samples),
            trailing_whitespace: self.estimate_trailing_whitespace(samples),
        }
    }

    fn extract_function_patterns(&self, samples: &[CodeSample]) -> Vec<FunctionPattern> {
        let mut all_patterns: Vec<FunctionPattern> = Vec::new();
        let mut seen_signatures: HashMap<String, u32> = HashMap::new();

        for sample in samples {
            let code_lower = sample.code.to_lowercase();
            let lines: Vec<&str> = sample.code.lines().collect();

            let fn_regexes = [
                (
                    r"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*(\w+))?",
                    "rust",
                ),
                (
                    r"function\s+(\w+)\s*\(([^)]*)\)\s*(?::\s*(\w+))?",
                    "typescript",
                ),
                (r"def\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*(\w+))?:", "python"),
            ];

            for (pattern, _lang) in &fn_regexes {
                if let Ok(re) = Regex::new(pattern) {
                    for cap in re.captures_iter(&sample.code) {
                        if let Some(name_match) = cap.get(1) {
                            let name = name_match.as_str().to_string();
                            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                            let return_type = cap.get(3).is_some();
                            let param_count = if params.is_empty() {
                                0
                            } else {
                                params.split(',').filter(|p| !p.trim().is_empty()).count()
                            };

                            let is_async = code_lower.contains("async fn")
                                || code_lower.contains("async function");
                            let visibility = if sample.code.contains("pub fn")
                                || sample.code.contains("pub async fn")
                            {
                                Visibility::Public
                            } else {
                                Visibility::Private
                            };

                            let signature = format!("{}_{}_{}", name, param_count, return_type);
                            *seen_signatures.entry(signature.clone()).or_insert(0) += 1;

                            if seen_signatures.get(&signature).copied().unwrap_or(0)
                                >= self.min_pattern_frequency
                            {
                                all_patterns.push(FunctionPattern {
                                    name,
                                    param_count,
                                    has_return_type: return_type,
                                    is_async,
                                    visibility,
                                    line_count: lines.len(),
                                });
                            }
                        }
                    }
                }
            }
        }

        all_patterns
    }

    fn extract_naming_patterns(&self, samples: &[CodeSample]) -> Vec<NamingPattern> {
        let mut patterns: Vec<NamingPattern> = Vec::new();
        let mut snake_count = 0u32;
        let mut camel_count = 0u32;
        let mut pascal_count = 0u32;
        let mut kebab_count = 0u32;

        let mut last_snake = String::new();
        let mut last_camel = String::new();
        let mut last_pascal = String::new();
        let mut last_kebab = String::new();

        for sample in samples {
            let identifiers = extract_identifiers(&sample.code);

            for id in identifiers {
                if id.contains('_') && !id.contains('-') && !id.contains("__") {
                    snake_count += 1;
                    last_snake = id;
                } else if id.chars().next().map(|c| c.is_lowercase()).unwrap_or(false)
                    && id.chars().any(|c| c.is_uppercase())
                {
                    camel_count += 1;
                    last_camel = id;
                } else if id.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && id.len() > 1
                    && id.chars().skip(1).any(|c| c.is_uppercase())
                {
                    pascal_count += 1;
                    last_pascal = id;
                } else if id.contains('-') {
                    kebab_count += 1;
                    last_kebab = id;
                }
            }
        }

        if snake_count >= self.min_pattern_frequency {
            patterns.push(NamingPattern {
                pattern_type: NamingPatternType::Snake,
                example: last_snake,
                count: snake_count,
            });
        }
        if camel_count >= self.min_pattern_frequency {
            patterns.push(NamingPattern {
                pattern_type: NamingPatternType::Camel,
                example: last_camel,
                count: camel_count,
            });
        }
        if pascal_count >= self.min_pattern_frequency {
            patterns.push(NamingPattern {
                pattern_type: NamingPatternType::Pascal,
                example: last_pascal,
                count: pascal_count,
            });
        }
        if kebab_count >= self.min_pattern_frequency {
            patterns.push(NamingPattern {
                pattern_type: NamingPatternType::Kebab,
                example: last_kebab,
                count: kebab_count,
            });
        }

        patterns
    }

    fn extract_structure_patterns(&self, samples: &[CodeSample]) -> Vec<StructurePattern> {
        let mut patterns: Vec<StructurePattern> = Vec::new();

        let mut early_return_count = 0u32;
        let mut guard_clause_count = 0u32;
        let mut nested_callback_count = 0u32;
        let mut builder_count = 0u32;

        for sample in samples {
            let code = &sample.code;
            let code_lower = code.to_lowercase();

            if code_lower.contains("return")
                && code_lower.contains("if")
                && code.lines().take(10).any(|l| l.contains("return"))
            {
                early_return_count += 1;
            }

            if (code_lower.contains("if") && code_lower.contains("return"))
                || (code_lower.contains("if") && code_lower.contains("throw"))
            {
                guard_clause_count += 1;
            }

            if code_lower.contains(".then(")
                || code_lower.contains(".then(")
                || code_lower.contains("callback")
            {
                nested_callback_count += 1;
            }

            if code_lower.contains(".build()")
                || code_lower.contains("builder")
                || code_lower.contains(".with_")
            {
                builder_count += 1;
            }
        }

        let total = samples.len() as f32;

        if early_return_count as f32 / total > 0.3 {
            patterns.push(StructurePattern {
                pattern_type: StructurePatternType::EarlyReturn,
                description: "Uses early return patterns".to_string(),
                frequency: early_return_count as f32 / total,
            });
        }
        if guard_clause_count as f32 / total > 0.3 {
            patterns.push(StructurePattern {
                pattern_type: StructurePatternType::GuardClauses,
                description: "Uses guard clause patterns".to_string(),
                frequency: guard_clause_count as f32 / total,
            });
        }
        if nested_callback_count as f32 / total > 0.2 {
            patterns.push(StructurePattern {
                pattern_type: StructurePatternType::NestedCallbacks,
                description: "Uses nested callbacks or promises".to_string(),
                frequency: nested_callback_count as f32 / total,
            });
        }
        if builder_count as f32 / total > 0.2 {
            patterns.push(StructurePattern {
                pattern_type: StructurePatternType::BuilderPattern,
                description: "Uses builder pattern".to_string(),
                frequency: builder_count as f32 / total,
            });
        }

        patterns
    }

    fn extract_comment_patterns(&self, samples: &[CodeSample]) -> Vec<CommentPattern> {
        let mut patterns: Vec<CommentPattern> = Vec::new();

        let mut single_line_count = 0u32;
        let mut multi_line_count = 0u32;
        let mut doc_count = 0u32;
        let mut inline_count = 0u32;

        let mut single_line_examples = Vec::new();
        let mut multi_line_examples = Vec::new();
        let mut doc_examples = Vec::new();

        for sample in samples {
            let lines: Vec<&str> = sample.code.lines().collect();

            for line in &lines {
                let trimmed = line.trim();
                if trimmed.starts_with("//") {
                    single_line_count += 1;
                    if single_line_examples.len() < 3 {
                        single_line_examples.push(line.to_string());
                    }
                } else if trimmed.starts_with("/*") {
                    multi_line_count += 1;
                    if multi_line_examples.len() < 3 {
                        multi_line_examples.push(line.to_string());
                    }
                } else if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                    doc_count += 1;
                    if doc_examples.len() < 3 {
                        doc_examples.push(line.to_string());
                    }
                } else if trimmed.contains("//") && !trimmed.starts_with("//") {
                    inline_count += 1;
                }
            }
        }

        let total = samples.len() as f32;

        if single_line_count as f32 / total > 0.5 {
            patterns.push(CommentPattern {
                style: CommentStyle::SingleLine,
                frequency: single_line_count as f32 / total,
                examples: single_line_examples,
            });
        }
        if multi_line_count as f32 / total > 0.3 {
            patterns.push(CommentPattern {
                style: CommentStyle::MultiLine,
                frequency: multi_line_count as f32 / total,
                examples: multi_line_examples,
            });
        }
        if doc_count as f32 / total > 0.2 {
            patterns.push(CommentPattern {
                style: CommentStyle::Documentation,
                frequency: doc_count as f32 / total,
                examples: doc_examples,
            });
        }
        if inline_count > 0 {
            patterns.push(CommentPattern {
                style: CommentStyle::Inline,
                frequency: inline_count as f32 / total,
                examples: Vec::new(),
            });
        }

        patterns
    }

    fn detect_indent_size(&self, samples: &[CodeSample]) -> usize {
        let mut sizes: Vec<usize> = Vec::new();
        let indent_regex = Regex::new(r"^(\s+)\S").unwrap();

        for sample in samples {
            for line in sample.code.lines() {
                if let Some(cap) = indent_regex.captures(line) {
                    if let Some(indent) = cap.get(1) {
                        let spaces = indent.as_str().chars().filter(|&c| c == ' ').count();
                        let tabs = indent.as_str().chars().filter(|&c| c == '\t').count();
                        if spaces > 0 {
                            sizes.push(spaces);
                        } else if tabs > 0 {
                            sizes.push(tabs * 4);
                        }
                    }
                }
            }
        }

        if sizes.is_empty() {
            return 4;
        }

        let avg: f32 = sizes.iter().sum::<usize>() as f32 / sizes.len() as f32;
        if avg < 2.0 {
            2
        } else if avg < 4.0 {
            4
        } else {
            8
        }
    }

    fn detect_indent_style(&self, samples: &[CodeSample]) -> IndentStyle {
        let mut space_count = 0;
        let mut tab_count = 0;

        for sample in samples {
            for line in sample.code.lines() {
                if line.starts_with("    ") {
                    space_count += 1;
                } else if line.starts_with("\t") {
                    tab_count += 1;
                }
            }
        }

        if space_count > tab_count {
            IndentStyle::Spaces(4)
        } else {
            IndentStyle::Tabs
        }
    }

    fn detect_line_ending(&self, samples: &[CodeSample]) -> LineEnding {
        let mut crlf_count = 0;
        let mut lf_count = 0;

        for sample in samples {
            if sample.code.contains("\r\n") {
                crlf_count += 1;
            } else if sample.code.contains('\n') {
                lf_count += 1;
            }
        }

        if crlf_count > lf_count {
            LineEnding::Crlf
        } else {
            LineEnding::Lf
        }
    }

    fn estimate_max_line_length(&self, samples: &[CodeSample]) -> usize {
        let mut total_length = 0;
        let mut line_count = 0;

        for sample in samples {
            for line in sample.code.lines() {
                total_length += line.len();
                line_count += 1;
            }
        }

        if line_count == 0 {
            return 100;
        }

        let avg = total_length as f32 / line_count as f32;
        (avg * 1.5) as usize
    }

    fn estimate_trailing_whitespace(&self, samples: &[CodeSample]) -> f32 {
        let mut trailing_count = 0;
        let mut total_count = 0;

        for sample in samples {
            for line in sample.code.lines() {
                total_count += 1;
                if line.ends_with(' ') || line.ends_with('\t') {
                    trailing_count += 1;
                }
            }
        }

        if total_count == 0 {
            return 0.0;
        }

        trailing_count as f32 / total_count as f32
    }

    fn estimate_formality(&self, messages: &[MessageSample]) -> f32 {
        if messages.is_empty() {
            return 0.5;
        }
        let formal_words = [
            "therefore",
            "hence",
            "consequently",
            "furthermore",
            "moreover",
            "thus",
            "accordingly",
            "subsequently",
            "hereby",
            "whereas",
        ];
        let informal_words = ["btw", "lol", "omg", "gonna", "wanna", "gotta", "kinda"];

        let mut formal_count = 0;
        let mut informal_count = 0;

        for msg in messages {
            let content_lower = msg.content.to_lowercase();
            for word in &formal_words {
                formal_count += content_lower.matches(word).count();
            }
            for word in &informal_words {
                informal_count += content_lower.matches(word).count();
            }
        }

        let total = formal_count + informal_count;
        if total == 0 {
            return 0.5;
        }

        formal_count as f32 / total as f32
    }

    fn estimate_structure_level(&self, messages: &[MessageSample]) -> f32 {
        if messages.is_empty() {
            return 0.5;
        }

        let mut structured_count = 0;

        for msg in messages {
            let content = &msg.content;
            let has_headers =
                content.contains("# ") || content.contains("\n## ") || content.contains("\n### ");
            let has_lists =
                content.contains("- ") || content.contains("* ") || content.contains("1. ");
            let has_tables = content.contains("|") && content.contains("---");

            if has_headers || has_lists || has_tables {
                structured_count += 1;
            }
        }

        structured_count as f32 / messages.len() as f32
    }

    fn estimate_technical_vocabulary(&self, messages: &[MessageSample]) -> f32 {
        if messages.is_empty() {
            return 0.5;
        }

        let tech_terms = [
            "algorithm",
            "architecture",
            "optimization",
            "implementation",
            "interface",
            "abstraction",
            "polymorphism",
            "refactoring",
            "performance",
            "scalability",
            "throughput",
            "latency",
        ];

        let mut term_count = 0;
        let mut total_words = 0;

        for msg in messages {
            let words: Vec<&str> = msg.content.split_whitespace().collect();
            total_words += words.len();

            let content_lower = msg.content.to_lowercase();
            for term in &tech_terms {
                term_count += content_lower.matches(term).count();
            }
        }

        if total_words == 0 {
            return 0.5;
        }

        (term_count as f32 / total_words as f32 * 100.0).min(1.0)
    }

    fn estimate_explanation_detail(&self, messages: &[MessageSample]) -> f32 {
        if messages.is_empty() {
            return 0.5;
        }

        let total_words: usize = messages
            .iter()
            .map(|m| m.content.split_whitespace().count())
            .sum();

        let avg_words = total_words as f32 / messages.len() as f32;

        if avg_words < 50.0 {
            0.2
        } else if avg_words < 100.0 {
            0.4
        } else if avg_words < 200.0 {
            0.6
        } else if avg_words < 500.0 {
            0.8
        } else {
            1.0
        }
    }

    fn detect_preferred_format(&self, messages: &[MessageSample]) -> DocumentFormat {
        if messages.is_empty() {
            return DocumentFormat::PlainText;
        }

        let mut markdown_score = 0;
        let mut structured_score = 0;

        for msg in messages {
            let content = &msg.content;
            if content.contains("# ") || content.contains("**") || content.contains("`") {
                markdown_score += 1;
            }
            if content.contains("```") || content.contains("    ") || content.contains("\n- ") {
                structured_score += 1;
            }
        }

        let total = messages.len();
        if markdown_score > total / 2 {
            DocumentFormat::Markdown
        } else if structured_score > total / 2 {
            DocumentFormat::Structured
        } else {
            DocumentFormat::PlainText
        }
    }
}

impl Default for StyleExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingPreferences {
    pub indent_size: usize,
    pub indent_style: IndentStyle,
    pub line_ending: LineEnding,
    pub max_line_length: usize,
    pub trailing_whitespace: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndentStyle {
    Spaces(usize),
    Tabs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentFormat {
    PlainText,
    Markdown,
    Structured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStyleProfile {
    pub formality_level: f32,
    pub structure_level: f32,
    pub technical_vocabulary_ratio: f32,
    pub explanation_detail_level: f32,
    pub preferred_format: DocumentFormat,
}

fn extract_identifiers(code: &str) -> Vec<String> {
    let mut identifiers = Vec::new();

    let patterns = [
        r"\blet\s+(?:mut\s+)?(\w+)",
        r"\bvar\s+(\w+)",
        r"\bconst\s+(\w+)",
        r"\bfn\s+(\w+)",
        r"\bclass\s+(\w+)",
        r"\bstruct\s+(\w+)",
        r"\benum\s+(\w+)",
        r"\btrait\s+(\w+)",
        r"\bimpl\s+(\w+)",
        r"\btype\s+(\w+)",
    ];

    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(code) {
                if let Some(name) = cap.get(1) {
                    let name_str = name.as_str();
                    if !is_rust_keyword(name_str) {
                        identifiers.push(name_str.to_string());
                    }
                }
            }
        }
    }

    identifiers
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
    )
}
