// SPDX-License-Identifier: AGPL-3.0-only
//! Style Transfer 契约
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedCodePatterns {
    pub function_patterns: Vec<FunctionPattern>,
    pub naming_patterns: Vec<NamingPattern>,
    pub structural_patterns: Vec<StructurePattern>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionPattern {
    pub name: String,
    pub avg_length: f64,
    pub parameter_count: u8,
    pub has_doc: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingPattern {
    pub convention: String,
    pub frequency: f64,
    pub examples: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructurePattern {
    pub pattern_type: String,
    pub code: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentStyleProfile {
    pub format: String,
    pub line_length: u16,
    pub heading_depth: u8,
    pub uses_tables: bool,
    pub uses_lists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleVector {
    pub dimensions: [f64; 8],
}
impl StyleVector {
    pub fn similarity(&self, other: &StyleVector) -> f64 {
        let dot: f64 = self.dimensions.iter().zip(&other.dimensions).map(|(a, b)| a * b).sum();
        let ma: f64 = self.dimensions.iter().map(|d| d * d).sum::<f64>().sqrt();
        let mb: f64 = other.dimensions.iter().map(|d| d * d).sum::<f64>().sqrt();
        if ma == 0.0 || mb == 0.0 {
            0.0
        } else {
            dot / (ma * mb)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeStyleTemplate {
    pub name: String,
    pub patterns: Vec<StylePattern>,
    pub code_sample: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StylePattern {
    pub pattern_type: StylePatternType,
    pub content: String,
    pub weight: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StylePatternType {
    Naming,
    Structure,
    Comment,
    Formatting,
    Idiom,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSample {
    pub language: String,
    pub code: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSample {
    pub role: String,
    pub content: String,
}

#[async_trait]
pub trait StyleExtractor: Send + Sync {
    async fn extract_from_code(
        &self,
        samples: &[CodeSample],
    ) -> Result<ExtractedCodePatterns, String>;
    async fn extract_from_messages(
        &self,
        samples: &[MessageSample],
    ) -> Result<DocumentStyleProfile, String>;
}
#[async_trait]
pub trait StyleApplier: Send + Sync {
    async fn apply_style(&self, code: &str, template: &CodeStyleTemplate)
    -> Result<String, String>;
    fn active_template(&self) -> Option<CodeStyleTemplate>;
}
#[async_trait]
pub trait StyleVectorizer: Send + Sync {
    async fn vectorize_code(&self, sample: &CodeSample) -> Result<StyleVector, String>;
    async fn vectorize_message(&self, sample: &MessageSample) -> Result<StyleVector, String>;
    fn similarity(&self, a: &StyleVector, b: &StyleVector) -> f64 {
        a.similarity(b)
    }
}
