use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleVector {
    pub dimensions: StyleDimensions,
    pub source_confidence: f32,
    pub learned_at: DateTime<Utc>,
    pub sample_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleDimensions {
    pub naming_score: f32,
    pub density_score: f32,
    pub comment_ratio: f32,
    pub abstraction_level: f32,
    pub formality_score: f32,
    pub structure_score: f32,
    pub technical_depth: f32,
    pub explanation_length: f32,
}

impl Default for StyleDimensions {
    fn default() -> Self {
        Self {
            naming_score: 0.5,
            density_score: 0.5,
            comment_ratio: 0.5,
            abstraction_level: 0.5,
            formality_score: 0.5,
            structure_score: 0.5,
            technical_depth: 0.5,
            explanation_length: 0.5,
        }
    }
}

impl StyleVector {
    pub fn new(dimensions: StyleDimensions, source_confidence: f32, sample_count: u32) -> Self {
        Self {
            dimensions,
            source_confidence,
            learned_at: Utc::now(),
            sample_count,
        }
    }

    pub fn default_style() -> Self {
        Self {
            dimensions: StyleDimensions::default(),
            source_confidence: 0.0,
            learned_at: Utc::now(),
            sample_count: 0,
        }
    }

    pub fn to_embedding(&self) -> Vec<f32> {
        vec![
            self.dimensions.naming_score,
            self.dimensions.density_score,
            self.dimensions.comment_ratio,
            self.dimensions.abstraction_level,
            self.dimensions.formality_score,
            self.dimensions.structure_score,
            self.dimensions.technical_depth,
            self.dimensions.explanation_length,
            self.source_confidence,
        ]
    }

    pub fn from_embedding(embedding: &[f32]) -> Option<Self> {
        if embedding.len() < 9 {
            return None;
        }
        Some(Self {
            dimensions: StyleDimensions {
                naming_score: embedding[0],
                density_score: embedding[1],
                comment_ratio: embedding[2],
                abstraction_level: embedding[3],
                formality_score: embedding[4],
                structure_score: embedding[5],
                technical_depth: embedding[6],
                explanation_length: embedding[7],
            },
            source_confidence: embedding[8],
            learned_at: Utc::now(),
            sample_count: 0,
        })
    }

    pub fn similarity(&self, other: &StyleVector) -> f32 {
        let emb1 = self.to_embedding();
        let emb2 = other.to_embedding();
        cosine_similarity(&emb1, &emb2)
    }
}

pub struct StyleVectorizer {
    min_samples_for_confidence: u32,
}

impl StyleVectorizer {
    pub fn new() -> Self {
        Self {
            min_samples_for_confidence: 5,
        }
    }

    pub fn from_coding_samples(&self, samples: &[CodeSample]) -> StyleVector {
        if samples.is_empty() {
            return StyleVector::default_style();
        }

        let naming_score = self.analyze_naming_patterns(samples);
        let density_score = self.analyze_density(samples);
        let comment_ratio = self.analyze_comment_ratio(samples);
        let abstraction_level = self.analyze_abstraction(samples);
        let source_confidence = self.calculate_confidence(samples);

        StyleVector::new(
            StyleDimensions {
                naming_score,
                density_score,
                comment_ratio,
                abstraction_level,
                formality_score: 0.5,
                structure_score: 0.5,
                technical_depth: 0.5,
                explanation_length: 0.5,
            },
            source_confidence,
            samples.len() as u32,
        )
    }

    pub fn from_messages(&self, messages: &[MessageSample]) -> StyleVector {
        if messages.is_empty() {
            return StyleVector::default_style();
        }

        let formality_score = self.analyze_formality(messages);
        let structure_score = self.analyze_message_structure(messages);
        let technical_depth = self.analyze_technical_depth(messages);
        let explanation_length = self.analyze_explanation_length(messages);
        let source_confidence = self.calculate_message_confidence(messages);

        StyleVector::new(
            StyleDimensions {
                naming_score: 0.5,
                density_score: 0.5,
                comment_ratio: 0.5,
                abstraction_level: 0.5,
                formality_score,
                structure_score,
                technical_depth,
                explanation_length,
            },
            source_confidence,
            messages.len() as u32,
        )
    }

    fn analyze_naming_patterns(&self, samples: &[CodeSample]) -> f32 {
        let mut snake_count = 0;
        let mut camel_count = 0;
        let mut pascal_count = 0;
        let mut kebab_count = 0;
        let mut total = 0;

        for sample in samples {
            let funcs = extract_function_names(&sample.code);
            for func in funcs {
                total += 1;
                if func.contains('_') && !func.contains('-') {
                    snake_count += 1;
                } else if func
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
                    && func.chars().any(|c| c.is_uppercase())
                {
                    camel_count += 1;
                } else if func
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                    && func.chars().skip(1).any(|c| c.is_uppercase())
                {
                    pascal_count += 1;
                } else if func.contains('-') {
                    kebab_count += 1;
                }
            }

            let vars = extract_variable_names(&sample.code);
            for var in vars {
                total += 1;
                if var.contains('_') && !var.contains('-') {
                    snake_count += 1;
                } else if var
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
                    && var.chars().any(|c| c.is_uppercase())
                {
                    camel_count += 1;
                }
            }
        }

        if total == 0 {
            return 0.5;
        }

        let snake_ratio = snake_count as f32 / total as f32;
        let camel_ratio = camel_count as f32 / total as f32;
        let pascal_ratio = pascal_count as f32 / total as f32;
        let kebab_ratio = kebab_count as f32 / total as f32;

        (camel_ratio * 0.3 + pascal_ratio * 0.5 + snake_ratio * 0.1 + kebab_ratio * 0.8).min(1.0)
    }

    fn analyze_density(&self, samples: &[CodeSample]) -> f32 {
        if samples.is_empty() {
            return 0.5;
        }

        let mut total_lines = 0;
        let mut non_empty_lines = 0;

        for sample in samples {
            for line in sample.code.lines() {
                total_lines += 1;
                if !line.trim().is_empty() {
                    non_empty_lines += 1;
                }
            }
        }

        if total_lines == 0 {
            return 0.5;
        }

        (non_empty_lines as f32 / total_lines as f32).min(1.0)
    }

    fn analyze_comment_ratio(&self, samples: &[CodeSample]) -> f32 {
        if samples.is_empty() {
            return 0.5;
        }

        let mut total_lines = 0;
        let mut comment_lines = 0;

        let multi_line_start = Regex::new(r"/\*").unwrap();
        let multi_line_end = Regex::new(r"\*/").unwrap();

        for sample in samples {
            let mut in_multiline = false;
            for line in sample.code.lines() {
                total_lines += 1;
                let trimmed = line.trim();

                if trimmed.starts_with("//") {
                    comment_lines += 1;
                } else if in_multiline {
                    comment_lines += 1;
                    if multi_line_end.is_match(trimmed) {
                        in_multiline = false;
                    }
                } else if multi_line_start.is_match(trimmed) {
                    in_multiline = true;
                    comment_lines += 1;
                }
            }
        }

        if total_lines == 0 {
            return 0.5;
        }

        (comment_lines as f32 / total_lines as f32).min(1.0)
    }

    fn analyze_abstraction(&self, samples: &[CodeSample]) -> f32 {
        if samples.is_empty() {
            return 0.5;
        }

        let mut abstract_count = 0;
        let mut concrete_count = 0;

        for sample in samples {
            let code_lower = sample.code.to_lowercase();

            abstract_count += code_lower.matches("abstract").count();
            abstract_count += code_lower.matches("interface").count();
            abstract_count += code_lower.matches("trait").count();
            abstract_count += code_lower.matches("protocol").count();

            concrete_count += code_lower.matches("class ").count();
            concrete_count += code_lower.matches("struct ").count();
            concrete_count += code_lower.matches("enum ").count();
        }

        let total = abstract_count + concrete_count;
        if total == 0 {
            return 0.5;
        }

        (abstract_count as f32 / total as f32).min(1.0)
    }

    pub fn calculate_confidence(&self, samples: &[CodeSample]) -> f32 {
        let count = samples.len() as f32;
        let min_samples = self.min_samples_for_confidence as f32;

        if count >= min_samples {
            1.0
        } else {
            count / min_samples
        }
    }

    fn analyze_formality(&self, messages: &[MessageSample]) -> f32 {
        if messages.is_empty() {
            return 0.5;
        }

        let mut formal_indicators = 0;
        let mut informal_indicators = 0;

        let formal_words = [
            "therefore",
            "hence",
            "consequently",
            "furthermore",
            "moreover",
            "thus",
            "accordingly",
            "subsequently",
        ];
        let informal_words = [
            "btw", "lol", "omg", "gonna", "wanna", "gotta", "kinda", "sorta",
        ];

        for msg in messages {
            let content_lower = msg.content.to_lowercase();

            for word in &formal_words {
                formal_indicators += content_lower.matches(word).count();
            }
            for word in &informal_words {
                informal_indicators += content_lower.matches(word).count();
            }
        }

        let total = formal_indicators + informal_indicators;
        if total == 0 {
            return 0.5;
        }

        (formal_indicators as f32 / total as f32).min(1.0)
    }

    fn analyze_message_structure(&self, messages: &[MessageSample]) -> f32 {
        if messages.is_empty() {
            return 0.5;
        }

        let mut structured_count = 0;

        for msg in messages {
            let has_headers = msg.content.contains("# ")
                || (msg.content.contains("\n## ") && msg.content.contains("\n### "));
            let has_lists = msg.content.contains("- ") || msg.content.contains("* ");
            let has_numbered = msg.content.matches(char::is_numeric).count() > 2;

            if has_headers || has_lists || has_numbered {
                structured_count += 1;
            }
        }

        (structured_count as f32 / messages.len() as f32).min(1.0)
    }

    fn analyze_technical_depth(&self, messages: &[MessageSample]) -> f32 {
        if messages.is_empty() {
            return 0.5;
        }

        let technical_terms = [
            "algorithm",
            "architecture",
            "optimization",
            "implementation",
            "interface",
            "abstraction",
            "polymorphism",
            "inheritance",
            "encapsulation",
            "refactoring",
            "performance",
            "scalability",
        ];

        let mut total_terms = 0;

        for msg in messages {
            let content_lower = msg.content.to_lowercase();
            for term in &technical_terms {
                total_terms += content_lower.matches(term).count();
            }
        }

        let avg_terms = total_terms as f32 / messages.len() as f32;
        (avg_terms / 10.0).min(1.0)
    }

    fn analyze_explanation_length(&self, messages: &[MessageSample]) -> f32 {
        if messages.is_empty() {
            return 0.5;
        }

        let total_words: usize = messages
            .iter()
            .map(|m| m.content.split_whitespace().count())
            .sum();
        let avg_words = total_words as f32 / messages.len() as f32;

        (avg_words / 200.0).min(1.0)
    }

    fn calculate_message_confidence(&self, _messages: &[MessageSample]) -> f32 {
        self.calculate_confidence(&[])
    }
}

impl Default for StyleVectorizer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSample {
    pub code: String,
    pub language: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSample {
    pub content: String,
    pub role: String,
    pub timestamp: DateTime<Utc>,
}

fn extract_function_names(code: &str) -> Vec<String> {
    let mut names = Vec::new();

    let func_patterns = [
        r"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)",
        r"function\s+(\w+)",
        r"def\s+(\w+)",
        r"class\s+(\w+)",
        r"impl\s+(?:\w+\s+for\s+)?(\w+)",
    ];

    for pattern in &func_patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(code) {
                if let Some(name) = cap.get(1) {
                    names.push(name.as_str().to_string());
                }
            }
        }
    }

    names
}

fn extract_variable_names(code: &str) -> Vec<String> {
    let mut names = Vec::new();

    let var_patterns = [
        r"let\s+(?:mut\s+)?(\w+)",
        r"var\s+(\w+)",
        r"(\w+)\s*=",
        r"(\w+)\s*:\s*\w+",
    ];

    for pattern in &var_patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(code) {
                if let Some(name) = cap.get(1) {
                    let name_str = name.as_str();
                    if !is_keyword(name_str) && !name_str.starts_with(|c: char| c.is_uppercase()) {
                        names.push(name_str.to_string());
                    }
                }
            }
        }
    }

    names
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "let"
            | "var"
            | "const"
            | "static"
            | "fn"
            | "function"
            | "def"
            | "class"
            | "impl"
            | "struct"
            | "enum"
            | "if"
            | "else"
            | "for"
            | "while"
            | "loop"
            | "return"
            | "true"
            | "false"
            | "nil"
            | "null"
            | "None"
            | "void"
    )
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    dot_product / (mag_a * mag_b)
}
