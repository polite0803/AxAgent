use crate::style_extractor::{
    DocumentFormat, DocumentStyleProfile, FormattingPreferences, IndentStyle, LineEnding,
};
use crate::style_vectorizer::StyleVector;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStyleTemplate {
    pub name: String,
    pub patterns: Vec<StylePattern>,
    pub templates: Vec<CodeTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylePattern {
    pub pattern_type: StylePatternType,
    pub original: String,
    pub transformed: String,
    pub context: String,
    pub usage_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StylePatternType {
    Naming,
    Formatting,
    Structure,
    Comment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTemplate {
    pub name: String,
    pub template: String,
    pub description: String,
}

pub struct StyleApplier {
    indent_size: usize,
    indent_style: IndentStyle,
    line_ending: LineEnding,
    max_line_length: usize,
}

impl StyleApplier {
    pub fn new() -> Self {
        Self {
            indent_size: 4,
            indent_style: IndentStyle::Spaces(4),
            line_ending: LineEnding::Lf,
            max_line_length: 100,
        }
    }

    pub fn with_preferences(prefs: &FormattingPreferences) -> Self {
        Self {
            indent_size: prefs.indent_size,
            indent_style: prefs.indent_style.clone(),
            line_ending: prefs.line_ending.clone(),
            max_line_length: prefs.max_line_length,
        }
    }

    pub fn apply_code_style(&self, code: &str, target_style: &StyleVector) -> String {
        let mut result = code.to_string();

        result = self.apply_naming_transforms(&result, target_style);
        result = self.apply_formatting_transforms(&result, target_style);
        result = self.apply_comment_transforms(&result, target_style);

        result
    }

    pub fn apply_document_style(&self, content: &str, target_style: &StyleVector) -> String {
        let mut result = content.to_string();

        result = self.apply_document_formatting(&result, target_style);
        result = self.adjust_document_structure(&result, target_style);

        result
    }

    fn apply_naming_transforms(&self, code: &str, style: &StyleVector) -> String {
        let mut result = code.to_string();

        let naming_score = style.dimensions.naming_score;

        if naming_score < 0.3 {
            result = self.to_snake_case(&result);
        } else if naming_score > 0.7 {
            result = self.to_camel_case(&result);
        }

        result
    }

    fn to_snake_case(&self, code: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;
        let mut prev_underscore = true;

        for (i, c) in code.chars().enumerate() {
            if c.is_uppercase() && i > 0 && !prev_underscore && !prev_upper {
                result.push('_');
            }

            if c.is_uppercase() {
                result.push(c.to_lowercase().next().unwrap_or(c));
                prev_upper = true;
            } else {
                result.push(c);
                prev_upper = false;
            }

            prev_underscore = c == '_';
        }

        result
    }

    fn to_camel_case(&self, code: &str) -> String {
        let mut result = String::new();
        let mut next_upper = false;

        for c in code.chars() {
            if c == '_' {
                next_upper = true;
            } else if next_upper {
                result.push(c.to_uppercase().next().unwrap_or(c));
                next_upper = false;
            } else {
                result.push(c);
            }
        }

        result
    }

    fn apply_formatting_transforms(&self, code: &str, style: &StyleVector) -> String {
        let mut result = code.to_string();

        let density_score = style.dimensions.density_score;
        result = self.apply_density_transform(&result, density_score);

        let indent_str = match &self.indent_style {
            IndentStyle::Spaces(_n) => " ".repeat(self.indent_size),
            IndentStyle::Tabs => "\t".to_string(),
        };
        result = self.apply_indentation(&result, &indent_str);
        result = self.enforce_max_line_length(&result);

        result = self.apply_line_ending(&result);

        result
    }

    fn apply_density_transform(&self, code: &str, density: f32) -> String {
        let lines: Vec<&str> = code.lines().collect();
        let mut result = Vec::new();

        if density < 0.4 {
            for line in lines {
                result.push(line.trim_end().to_string());
            }
        } else if density > 0.6 {
            let mut prev_empty = false;
            for line in lines {
                let is_empty = line.trim().is_empty();
                if is_empty && !prev_empty {
                    result.push(String::new());
                } else if !is_empty {
                    result.push(line.to_string());
                }
                prev_empty = is_empty;
            }
        } else {
            return code.to_string();
        }

        result.join(&self.line_ending_str())
    }

    fn apply_indentation(&self, code: &str, indent_str: &str) -> String {
        let mut result = Vec::new();
        let base_indent = self.detect_base_indent(code);

        for line in code.lines() {
            if line.trim().is_empty() {
                result.push(String::new());
                continue;
            }

            let leading_spaces = line.len() - line.trim_start().len();
            let indent_level = if base_indent > 0 {
                leading_spaces.checked_div(base_indent).unwrap_or(0)
            } else {
                0
            };

            let new_indent = indent_str.repeat(indent_level);
            result.push(format!("{}{}", new_indent, line.trim()));
        }

        result.join(&self.line_ending_str())
    }

    fn detect_base_indent(&self, code: &str) -> usize {
        let mut min_indent = usize::MAX;

        for line in code.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let leading_spaces = line.len() - line.trim_start().len();
            if leading_spaces > 0 && leading_spaces < min_indent {
                min_indent = leading_spaces;
            }
        }

        if min_indent == usize::MAX {
            4
        } else {
            min_indent
        }
    }

    fn apply_line_ending(&self, code: &str) -> String {
        let le_str = self.line_ending_str();
        if le_str == "\n" {
            code.replace("\r\n", "\n")
        } else {
            code.replace("\n", "\r\n")
        }
    }

    fn enforce_max_line_length(&self, code: &str) -> String {
        let max_len = self.max_line_length;
        if max_len == 0 {
            return code.to_string();
        }

        code.lines()
            .map(|line| {
                if line.len() <= max_len {
                    line.to_string()
                } else {
                    let mut result = String::new();
                    let mut current_len = 0;
                    for word in line.split_whitespace() {
                        if current_len + word.len() + 1 > max_len && current_len > 0 {
                            result.push('\n');
                            current_len = 0;
                        }
                        if current_len > 0 {
                            result.push(' ');
                            current_len += 1;
                        }
                        result.push_str(word);
                        current_len += word.len();
                    }
                    result
                }
            })
            .collect::<Vec<_>>()
            .join(&self.line_ending_str())
    }

    fn line_ending_str(&self) -> String {
        match self.line_ending {
            LineEnding::Lf => "\n".to_string(),
            LineEnding::Crlf => "\r\n".to_string(),
        }
    }

    fn apply_comment_transforms(&self, code: &str, style: &StyleVector) -> String {
        let mut result = code.to_string();

        let comment_ratio = style.dimensions.comment_ratio;

        if comment_ratio < 0.1 {
            result = self.remove_comments(&result);
        } else if comment_ratio > 0.3 {
            result = self.ensure_comments(&result, comment_ratio);
        }

        result
    }

    fn remove_comments(&self, code: &str) -> String {
        let single_line = Regex::new(r"//[^\n]*").unwrap();
        let multi_line = Regex::new(r"/\*[\s\S]*?\*/").unwrap();

        let mut result = code.to_string();
        result = single_line.replace_all(&result, "").to_string();
        result = multi_line.replace_all(&result, "").to_string();

        result
    }

    fn ensure_comments(&self, code: &str, ratio: f32) -> String {
        let comment_count = code.lines().filter(|l| l.trim().starts_with("//")).count();
        let total_lines = code.lines().count();

        if total_lines == 0 {
            return code.to_string();
        }

        let current_ratio = comment_count as f32 / total_lines as f32;

        if current_ratio >= ratio {
            return code.to_string();
        }

        code.to_string()
    }

    fn apply_document_formatting(&self, content: &str, style: &StyleVector) -> String {
        let mut result = content.to_string();

        let formality = style.dimensions.formality_score;
        result = self.apply_formality_transform(&result, formality);

        let structure = style.dimensions.structure_score;
        result = self.apply_structure_markup(&result, structure);

        result
    }

    fn apply_formality_transform(&self, content: &str, formality: f32) -> String {
        if formality < 0.3 {
            let informal_replacements = [
                ("because", "cuz"),
                ("therefore", "so"),
                ("however", "but"),
                ("furthermore", "also"),
                ("consequently", "so"),
            ];

            let mut result = content.to_string();
            for (formal, informal) in informal_replacements {
                let re = Regex::new(&format!(r"\b{}\b", formal)).unwrap();
                result = re.replace_all(&result, informal).to_string();
            }
            result
        } else if formality > 0.7 {
            let formal_replacements = [
                ("cuz", "because"),
                ("so ", "therefore "),
                ("but ", "however "),
                ("also ", "furthermore "),
            ];

            let mut result = content.to_string();
            for (informal, formal) in formal_replacements {
                let re = Regex::new(&format!(r"\b{}\b", informal)).unwrap();
                result = re.replace_all(&result, formal).to_string();
            }
            result
        } else {
            content.to_string()
        }
    }

    fn apply_structure_markup(&self, content: &str, structure: f32) -> String {
        if structure < 0.3 {
            let re = Regex::new(r"^#+\s+").unwrap();
            return re.replace_all(content, "").to_string();
        }

        content.to_string()
    }

    fn adjust_document_structure(&self, content: &str, style: &StyleVector) -> String {
        let explanation = style.dimensions.explanation_length;
        let mut result = content.to_string();

        if explanation < 0.3 {
            result = self.shorten_explanations(&result);
        } else if explanation > 0.7 {
            result = self.expand_explanations(&result);
        }

        result
    }

    fn shorten_explanations(&self, content: &str) -> String {
        let mut result = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.len() > 80 {
                let words: Vec<&str> = trimmed.split_whitespace().collect();
                if words.len() > 15 {
                    let shortened: String =
                        words.iter().take(12).copied().collect::<Vec<_>>().join(" ");
                    result.push(format!("{}...", shortened));
                    continue;
                }
            }
            result.push(line.to_string());
        }

        result.join("\n")
    }

    fn expand_explanations(&self, content: &str) -> String {
        let mut result = Vec::new();

        for line in content.lines() {
            result.push(line.to_string());

            let trimmed = line.trim();
            if (trimmed.ends_with(':') || trimmed.ends_with('：') || trimmed.ends_with('?'))
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("- ")
                && !trimmed.starts_with("* ")
            {
                result.push(String::new());
            }
        }

        result.join("\n")
    }

    pub fn create_template(&self, name: &str, patterns: Vec<StylePattern>) -> CodeStyleTemplate {
        CodeStyleTemplate {
            name: name.to_string(),
            patterns,
            templates: Vec::new(),
        }
    }

    pub fn apply_template(&self, code: &str, template: &CodeStyleTemplate) -> String {
        let mut result = code.to_string();

        for pattern in &template.patterns {
            result = result.replace(&pattern.original, &pattern.transformed);
        }

        result
    }

    pub fn learn_pattern(
        &self,
        pattern_type: StylePatternType,
        original: &str,
        transformed: &str,
        context: &str,
    ) -> StylePattern {
        StylePattern {
            pattern_type,
            original: original.to_string(),
            transformed: transformed.to_string(),
            context: context.to_string(),
            usage_count: 1,
        }
    }
}

impl Default for StyleApplier {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DocumentStyleApplicator {
    pub profile: DocumentStyleProfile,
}

impl DocumentStyleApplicator {
    pub fn new(profile: DocumentStyleProfile) -> Self {
        Self { profile }
    }

    pub fn apply_to_message(&self, message: &str) -> String {
        let mut result = message.to_string();

        result = self.apply_formatting(&result);
        result = self.apply_structure(&result);

        result
    }

    fn apply_formatting(&self, content: &str) -> String {
        match self.profile.preferred_format {
            DocumentFormat::Markdown => self.apply_markdown_formatting(content),
            DocumentFormat::Structured => self.apply_structured_formatting(content),
            DocumentFormat::PlainText => content.to_string(),
        }
    }

    fn apply_markdown_formatting(&self, content: &str) -> String {
        let mut result = content.to_string();

        if self.profile.formality_level > 0.7 && !result.contains("**") {
            result = self.make_bold_headers(&result);
        }

        result
    }

    fn apply_structured_formatting(&self, content: &str) -> String {
        let mut result = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('-') && !trimmed.starts_with('*') {
                result.push(format!("- {}", trimmed));
            } else {
                result.push(line.to_string());
            }
        }

        result.join("\n")
    }

    fn apply_structure(&self, content: &str) -> String {
        if self.profile.structure_level < 0.3 {
            return content.to_string();
        }

        let lines: Vec<&str> = content.lines().collect();

        if lines.len() > 3 && !lines[0].starts_with('#') {
            let title = lines[0];
            let formatted_title = format!("# {}", title);
            return format!("{}\n{}", formatted_title, content);
        }

        content.to_string()
    }

    fn make_bold_headers(&self, content: &str) -> String {
        let re = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();

        re.replace_all(content, |caps: &regex::Captures| {
            let hashes = &caps[1];
            let title = &caps[2];
            format!("{} **{}**", hashes, title)
        })
        .to_string()
    }
}
