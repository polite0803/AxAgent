// SPDX-License-Identifier: AGPL-3.0-only
//! 内容过滤器契约
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterAction {
    Allow,
    Block { reason: String },
    Modify { modified: String, reason: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    UserInput,
    LlmOutput,
    ExternalData,
    FileContent,
}
#[derive(Debug, Clone)]
pub struct ContentFilterConfig {
    pub detect_pii: bool,
    pub detect_sensitive: bool,
    pub detect_code_injection: bool,
    pub custom_words: Vec<String>,
}
impl Default for ContentFilterConfig {
    fn default() -> Self {
        Self {
            detect_pii: true,
            detect_sensitive: true,
            detect_code_injection: true,
            custom_words: Vec::new(),
        }
    }
}

#[async_trait]
pub trait ContentFilter: Send + Sync {
    async fn filter(
        &self,
        content: &str,
        content_type: ContentType,
    ) -> Result<FilterAction, String>;
    async fn is_safe(&self, content: &str, content_type: ContentType) -> Result<bool, String>;
}
#[derive(Default)]
pub struct NoopContentFilter;
#[async_trait]
impl ContentFilter for NoopContentFilter {
    async fn filter(&self, _: &str, _: ContentType) -> Result<FilterAction, String> {
        Ok(FilterAction::Allow)
    }
    async fn is_safe(&self, _: &str, _: ContentType) -> Result<bool, String> {
        Ok(true)
    }
}
