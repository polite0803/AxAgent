use crate::research_state::{SearchQuery, SearchResult, SourceType};
use crate::search_provider::{ContentMetadata, ExtractedContent, RelevanceScorer, SearchProvider};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub timeout_secs: u64,
    pub rate_limit_per_minute: Option<u32>,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: None,
            timeout_secs: 30,
            rate_limit_per_minute: Some(60),
        }
    }
}

pub struct WebSearchProvider {
    config: WebSearchConfig,
}

impl WebSearchProvider {
    pub fn new() -> Self {
        Self::with_config(WebSearchConfig::default())
    }

    pub fn with_config(config: WebSearchConfig) -> Self {
        Self { config }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.config.api_key = Some(api_key.into());
        self
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.config.endpoint = Some(endpoint.into());
        self
    }

    async fn perform_search(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let scorer = RelevanceScorer::new(&query.query);
        let mock_results = self.generate_mock_results(query);
        Ok(scorer.score_and_sort(mock_results))
    }

    fn generate_mock_results(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let query_lower = query.query.to_lowercase();
        let base_results = match query_lower.as_str() {
            _ if query_lower.contains("rust") || query_lower.contains("programming") => vec![
                SearchResult::new(
                    SourceType::Web,
                    "https://doc.rust-lang.org/book/".to_string(),
                    "The Rust Programming Language - The Rust Book".to_string(),
                    "The official book for learning Rust, covering everything from basic syntax to advanced concurrency patterns.".to_string(),
                )
                .with_credibility(0.9)
                .with_relevance(0.95),
                SearchResult::new(
                    SourceType::Web,
                    "https://crates.io/".to_string(),
                    "crates.io: Rust Package Registry".to_string(),
                    "The official registry for Rust packages. Find, publish, and manage Rust dependencies.".to_string(),
                )
                .with_credibility(0.85)
                .with_relevance(0.9),
                SearchResult::new(
                    SourceType::Blog,
                    "https://blog.rust-lang.org/".to_string(),
                    "The Rust Programming Language Blog".to_string(),
                    "Official updates, announcements, and articles from the Rust team.".to_string(),
                )
                .with_credibility(0.9)
                .with_relevance(0.85),
            ],
            _ if query_lower.contains("machine learning") || query_lower.contains("ai") => vec![
                SearchResult::new(
                    SourceType::Web,
                    "https://paperswithcode.com/".to_string(),
                    "Papers with Code - The Latest in Machine Learning".to_string(),
                    "Free papers, benchmarks, and evaluation methods for machine learning research.".to_string(),
                )
                .with_credibility(0.85)
                .with_relevance(0.92),
                SearchResult::new(
                    SourceType::Academic,
                    "https://arxiv.org/list/cs.AI/recent".to_string(),
                    "arXiv: Artificial Intelligence".to_string(),
                    "Recent papers on artificial intelligence from arXiv preprint server.".to_string(),
                )
                .with_credibility(0.9)
                .with_relevance(0.88),
            ],
            _ => vec![
                SearchResult::new(
                    SourceType::Web,
                    format!("https://en.wikipedia.org/wiki/Special:Search?search={}", urlencoding::encode(&query.query)),
                    format!("Search results for: {}", query.query),
                    format!("Wikipedia article about {}. Click to read more on Wikipedia.", query.query),
                )
                .with_credibility(0.75)
                .with_relevance(0.7),
                SearchResult::new(
                    SourceType::Web,
                    format!("https://www.google.com/search?q={}", urlencoding::encode(&query.query)),
                    format!("Google Search: {}", query.query),
                    format!("Web search results for {}. Google is the most popular search engine.", query.query),
                )
                .with_credibility(0.6)
                .with_relevance(0.65),
            ],
        };

        base_results
            .into_iter()
            .take(query.max_results.min(10))
            .collect()
    }
}

impl Default for WebSearchProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchProvider for WebSearchProvider {
    async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        self.perform_search(query).await
    }

    async fn extract_content(
        &self,
        url: &str,
    ) -> Result<ExtractedContent, crate::search_provider::ExtractError> {
        if url.is_empty() {
            return Err(crate::search_provider::ExtractError::InvalidUrl(
                "URL is empty".to_string(),
            ));
        }

        let domain = url.split('/').nth(2).unwrap_or("unknown").to_string();

        let content = ExtractedContent::new(
            url.to_string(),
            format!("Page: {}", domain),
            format!("Content extracted from {}. This is a mock implementation for demonstration purposes.", url),
        )
        .with_metadata(ContentMetadata {
            author: None,
            published_date: None,
            description: Some("Mock extracted content".to_string()),
            keywords: vec!["demo".to_string(), "mock".to_string()],
            language: Some("en".to_string()),
        });

        Ok(content)
    }

    fn source_type(&self) -> SourceType {
        SourceType::Web
    }

    fn display_name(&self) -> &str {
        "Web Search"
    }

    fn rate_limit(&self) -> Option<Duration> {
        self.config
            .rate_limit_per_minute
            .map(|rpm| Duration::from_secs(60 * 60 / rpm as u64))
    }
}

pub struct WebSearchProviderBuilder {
    provider: WebSearchProvider,
}

impl WebSearchProviderBuilder {
    pub fn new() -> Self {
        Self {
            provider: WebSearchProvider::new(),
        }
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.provider = self.provider.with_api_key(key);
        self
    }

    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.provider = self.provider.with_endpoint(endpoint);
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.provider.config.timeout_secs = secs;
        self
    }

    pub fn rate_limit(mut self, per_minute: u32) -> Self {
        self.provider.config.rate_limit_per_minute = Some(per_minute);
        self
    }

    pub fn build(self) -> WebSearchProvider {
        self.provider
    }
}

impl Default for WebSearchProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_search() {
        let provider = WebSearchProvider::new();
        let query = SearchQuery::new("rust programming".to_string());

        let results = provider.search(&query).await;
        assert!(results.is_ok());
        let results = results.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_extract_content() {
        let provider = WebSearchProvider::new();
        let url = "https://example.com/test";

        let content = provider.extract_content(url).await;
        assert!(content.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_url() {
        let provider = WebSearchProvider::new();
        let url = "";

        let content = provider.extract_content(url).await;
        assert!(content.is_err());
    }
}
