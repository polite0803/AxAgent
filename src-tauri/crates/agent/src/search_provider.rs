// SPDX-License-Identifier: AGPL-3.0-only

use crate::research_state::{SearchQuery, SearchResult, SourceType};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Timeout")]
    Timeout,
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[derive(Error, Debug)]
pub enum ExtractError {
    #[error("Failed to fetch URL: {0}")]
    FetchError(String),
    #[error("Failed to parse HTML: {0}")]
    ParseError(String),
    #[error("Content too large: {0} bytes")]
    ContentTooLarge(usize),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    pub url: String,
    pub title: String,
    pub text: String,
    pub html: Option<String>,
    pub links: Vec<String>,
    pub images: Vec<String>,
    pub metadata: ContentMetadata,
    pub extracted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContentMetadata {
    pub author: Option<String>,
    pub published_date: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub language: Option<String>,
}

impl ExtractedContent {
    pub fn new(url: String, title: String, text: String) -> Self {
        Self {
            url,
            title,
            text,
            html: None,
            links: Vec::new(),
            images: Vec::new(),
            metadata: ContentMetadata::default(),
            extracted_at: Utc::now(),
        }
    }

    pub fn with_html(mut self, html: String) -> Self {
        self.html = Some(html);
        self
    }

    pub fn with_links(mut self, links: Vec<String>) -> Self {
        self.links = links;
        self
    }

    pub fn with_images(mut self, images: Vec<String>) -> Self {
        self.images = images;
        self
    }

    pub fn with_metadata(mut self, metadata: ContentMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

#[async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SearchError>;
    async fn extract_content(&self, url: &str) -> Result<ExtractedContent, ExtractError>;
    fn source_type(&self) -> SourceType;
    fn display_name(&self) -> &str;
    fn rate_limit(&self) -> Option<std::time::Duration>;
}

pub struct SearchProviderRegistry {
    providers: Vec<Box<dyn SearchProvider>>,
}

impl SearchProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register<P: SearchProvider + 'static>(&mut self, provider: P) {
        self.providers.push(Box::new(provider));
    }

    pub fn get(&self, source_type: SourceType) -> Option<&dyn SearchProvider> {
        self.providers
            .iter()
            .find(|p| p.source_type() == source_type)
            .map(|p| p.as_ref())
    }

    pub fn get_all(&self) -> Vec<&dyn SearchProvider> {
        self.providers.iter().map(|p| p.as_ref()).collect()
    }

    pub fn get_by_types(&self, source_types: &[SourceType]) -> Vec<&dyn SearchProvider> {
        self.providers
            .iter()
            .filter(|p| source_types.contains(&p.source_type()))
            .map(|p| p.as_ref())
            .collect()
    }
}

impl Default for SearchProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SearchQueryBuilder {
    query: String,
    source_types: Vec<SourceType>,
    max_results: usize,
    language: Option<String>,
    date_range: Option<DateRange>,
}

#[derive(Debug, Clone)]
pub struct DateRange {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

impl SearchQueryBuilder {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            source_types: vec![SourceType::Web],
            max_results: 10,
            language: None,
            date_range: None,
        }
    }

    pub fn sources(mut self, sources: Vec<SourceType>) -> Self {
        self.source_types = sources;
        self
    }

    pub fn max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    pub fn date_range(mut self, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Self {
        self.date_range = Some(DateRange { from, to });
        self
    }

    pub fn build(self) -> SearchQuery {
        SearchQuery::new(self.query)
            .with_sources(self.source_types)
            .with_max_results(self.max_results)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchProviderType {
    Web,
    Academic,
    Wikipedia,
    GitHub,
    Documentation,
    News,
}

impl fmt::Display for SearchProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchProviderType::Web => write!(f, "Web Search"),
            SearchProviderType::Academic => write!(f, "Academic Search"),
            SearchProviderType::Wikipedia => write!(f, "Wikipedia"),
            SearchProviderType::GitHub => write!(f, "GitHub"),
            SearchProviderType::Documentation => write!(f, "Documentation"),
            SearchProviderType::News => write!(f, "News"),
        }
    }
}

pub trait SearchResultProcessor: Send + Sync {
    fn process(&self, result: SearchResult) -> SearchResult;
    fn process_batch(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        results.into_iter().map(|r| self.process(r)).collect()
    }
}

pub struct RelevanceScorer {
    query_terms: Vec<String>,
}

impl RelevanceScorer {
    pub fn new(query: &str) -> Self {
        let query_terms: Vec<String> = query.split_whitespace().map(|s| s.to_lowercase()).collect();

        Self { query_terms }
    }

    pub fn score(&self, result: &SearchResult) -> f32 {
        let title_lower = result.title.to_lowercase();
        let snippet_lower = result.snippet.to_lowercase();

        let mut score: f32 = 0.0;

        for term in &self.query_terms {
            if title_lower.contains(term) {
                score += 0.4;
            }
            if snippet_lower.contains(term) {
                score += 0.2;
            }
        }

        score.min(1.0)
    }

    pub fn score_and_sort(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut scored: Vec<(SearchResult, f32)> = results
            .into_iter()
            .map(|mut r| {
                let score = self.score(&r);
                r.relevance_score = score;
                (r, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().map(|(r, _)| r).collect()
    }
}

impl SearchResultProcessor for RelevanceScorer {
    fn process(&self, result: SearchResult) -> SearchResult {
        let mut r = result;
        r.relevance_score = self.score(&r);
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research_state::SourceType;

    struct MockProvider {
        source: SourceType,
        name: &'static str,
        rate: Option<std::time::Duration>,
    }

    #[async_trait]
    impl SearchProvider for MockProvider {
        async fn search(&self, _query: &SearchQuery) -> Result<Vec<SearchResult>, SearchError> {
            Ok(vec![SearchResult::new(
                self.source,
                "https://example.com".to_string(),
                "Mock Result".to_string(),
                "Mock snippet".to_string(),
            )])
        }

        async fn extract_content(&self, _url: &str) -> Result<ExtractedContent, ExtractError> {
            Ok(ExtractedContent::new(
                "https://example.com".to_string(),
                "Mock".to_string(),
                "Content".to_string(),
            ))
        }

        fn source_type(&self) -> SourceType {
            self.source
        }

        fn display_name(&self) -> &str {
            self.name
        }

        fn rate_limit(&self) -> Option<std::time::Duration> {
            self.rate
        }
    }

    #[test]
    fn test_search_error_variants() {
        let e = SearchError::NetworkError("timeout".to_string());
        assert!(e.to_string().contains("timeout"));

        let e = SearchError::ApiError("bad request".to_string());
        assert!(e.to_string().contains("bad request"));

        let e = SearchError::RateLimitExceeded;
        assert!(e.to_string().contains("Rate limit"));

        let e = SearchError::InvalidQuery("empty".to_string());
        assert!(e.to_string().contains("empty"));

        let e = SearchError::Timeout;
        assert!(e.to_string().contains("Timeout"));

        let e = SearchError::ParseError("json".to_string());
        assert!(e.to_string().contains("json"));
    }

    #[test]
    fn test_extract_error_variants() {
        let e = ExtractError::FetchError("404".to_string());
        assert!(e.to_string().contains("404"));

        let e = ExtractError::ParseError("bad html".to_string());
        assert!(e.to_string().contains("bad html"));

        let e = ExtractError::ContentTooLarge(9999);
        assert!(e.to_string().contains("9999"));

        let e = ExtractError::InvalidUrl("nope".to_string());
        assert!(e.to_string().contains("nope"));
    }

    #[test]
    fn test_extracted_content_new() {
        let content = ExtractedContent::new(
            "https://example.com".to_string(),
            "Title".to_string(),
            "Body text".to_string(),
        );
        assert_eq!(content.url, "https://example.com");
        assert_eq!(content.title, "Title");
        assert_eq!(content.text, "Body text");
        assert!(content.html.is_none());
        assert!(content.links.is_empty());
        assert!(content.images.is_empty());
    }

    #[test]
    fn test_extracted_content_builder_pattern() {
        let content = ExtractedContent::new(
            "https://example.com".to_string(),
            "Title".to_string(),
            "Text".to_string(),
        )
        .with_html("<p>html</p>".to_string())
        .with_links(vec!["https://link1.com".to_string()])
        .with_images(vec!["https://img.com/a.png".to_string()])
        .with_metadata(ContentMetadata {
            author: Some("Author".to_string()),
            published_date: Some("2024-01-01".to_string()),
            description: Some("Desc".to_string()),
            keywords: vec!["rust".to_string()],
            language: Some("en".to_string()),
        });

        assert_eq!(content.html.as_deref(), Some("<p>html</p>"));
        assert_eq!(content.links.len(), 1);
        assert_eq!(content.images.len(), 1);
        assert_eq!(content.metadata.author.as_deref(), Some("Author"));
        assert_eq!(content.metadata.keywords.len(), 1);
    }

    #[test]
    fn test_content_metadata_default() {
        let meta = ContentMetadata::default();
        assert!(meta.author.is_none());
        assert!(meta.published_date.is_none());
        assert!(meta.description.is_none());
        assert!(meta.keywords.is_empty());
        assert!(meta.language.is_none());
    }

    #[test]
    fn test_search_provider_registry_new() {
        let registry = SearchProviderRegistry::new();
        assert!(registry.get_all().is_empty());
    }

    #[test]
    fn test_search_provider_registry_default() {
        let registry = SearchProviderRegistry::default();
        assert!(registry.get_all().is_empty());
    }

    #[test]
    fn test_search_provider_registry_register_and_get() {
        let mut registry = SearchProviderRegistry::new();
        registry.register(MockProvider {
            source: SourceType::Web,
            name: "Web Provider",
            rate: None,
        });

        assert!(registry.get(SourceType::Web).is_some());
        assert!(registry.get(SourceType::Academic).is_none());
    }

    #[test]
    fn test_search_provider_registry_get_all() {
        let mut registry = SearchProviderRegistry::new();
        registry.register(MockProvider {
            source: SourceType::Web,
            name: "Web",
            rate: None,
        });
        registry.register(MockProvider {
            source: SourceType::Academic,
            name: "Academic",
            rate: None,
        });

        let all = registry.get_all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_search_provider_registry_get_by_types() {
        let mut registry = SearchProviderRegistry::new();
        registry.register(MockProvider {
            source: SourceType::Web,
            name: "Web",
            rate: None,
        });
        registry.register(MockProvider {
            source: SourceType::Academic,
            name: "Academic",
            rate: None,
        });
        registry.register(MockProvider {
            source: SourceType::GitHub,
            name: "GitHub",
            rate: None,
        });

        let filtered = registry.get_by_types(&[SourceType::Web, SourceType::Academic]);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_search_provider_registry_get_by_types_empty() {
        let mut registry = SearchProviderRegistry::new();
        registry.register(MockProvider {
            source: SourceType::Web,
            name: "Web",
            rate: None,
        });

        let filtered = registry.get_by_types(&[SourceType::GitHub]);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_search_query_builder_new() {
        let builder = SearchQueryBuilder::new("rust programming");
        assert_eq!(builder.query, "rust programming");
        assert_eq!(builder.source_types, vec![SourceType::Web]);
        assert_eq!(builder.max_results, 10);
        assert!(builder.language.is_none());
        assert!(builder.date_range.is_none());
    }

    #[test]
    fn test_search_query_builder_sources() {
        let builder = SearchQueryBuilder::new("test")
            .sources(vec![SourceType::Academic, SourceType::Wikipedia]);
        assert_eq!(builder.source_types, vec![SourceType::Academic, SourceType::Wikipedia]);
    }

    #[test]
    fn test_search_query_builder_max_results() {
        let builder = SearchQueryBuilder::new("test").max_results(25);
        assert_eq!(builder.max_results, 25);
    }

    #[test]
    fn test_search_query_builder_language() {
        let builder = SearchQueryBuilder::new("test").language("en");
        assert_eq!(builder.language.as_deref(), Some("en"));
    }

    #[test]
    fn test_search_query_builder_date_range() {
        let from = chrono::Utc::now();
        let to = chrono::Utc::now();
        let builder = SearchQueryBuilder::new("test").date_range(Some(from), Some(to));
        assert!(builder.date_range.is_some());
        let range = builder.date_range.unwrap();
        assert!(range.from.is_some());
        assert!(range.to.is_some());
    }

    #[test]
    fn test_search_query_builder_build() {
        let query = SearchQueryBuilder::new("rust")
            .sources(vec![SourceType::Web, SourceType::Academic])
            .max_results(5)
            .build();
        assert_eq!(query.query, "rust");
        assert_eq!(query.source_types, vec![SourceType::Web, SourceType::Academic]);
        assert_eq!(query.max_results, 5);
    }

    #[test]
    fn test_search_provider_type_display() {
        assert_eq!(SearchProviderType::Web.to_string(), "Web Search");
        assert_eq!(SearchProviderType::Academic.to_string(), "Academic Search");
        assert_eq!(SearchProviderType::Wikipedia.to_string(), "Wikipedia");
        assert_eq!(SearchProviderType::GitHub.to_string(), "GitHub");
        assert_eq!(SearchProviderType::Documentation.to_string(), "Documentation");
        assert_eq!(SearchProviderType::News.to_string(), "News");
    }

    #[test]
    fn test_relevance_scorer_new() {
        let scorer = RelevanceScorer::new("rust programming language");
        assert_eq!(scorer.query_terms, vec!["rust", "programming", "language"]);
    }

    #[test]
    fn test_relevance_scorer_score_title_match() {
        let scorer = RelevanceScorer::new("rust");
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Rust Programming Guide".to_string(),
            "Some other text".to_string(),
        );
        let score = scorer.score(&result);
        assert!(score > 0.0);
    }

    #[test]
    fn test_relevance_scorer_score_snippet_match() {
        let scorer = RelevanceScorer::new("rust");
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Programming Guide".to_string(),
            "Learn rust programming".to_string(),
        );
        let score = scorer.score(&result);
        assert!(score > 0.0);
    }

    #[test]
    fn test_relevance_scorer_score_no_match() {
        let scorer = RelevanceScorer::new("python");
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Java Tutorial".to_string(),
            "Learn Java programming".to_string(),
        );
        let score = scorer.score(&result);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_relevance_scorer_score_max_one() {
        let scorer = RelevanceScorer::new("a b c d e");
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "a b c d e".to_string(),
            "a b c d e".to_string(),
        );
        let score = scorer.score(&result);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_relevance_scorer_score_and_sort() {
        let scorer = RelevanceScorer::new("rust");
        let results = vec![
            SearchResult::new(
                SourceType::Web,
                "https://a.com".to_string(),
                "Java Guide".to_string(),
                "Java stuff".to_string(),
            ),
            SearchResult::new(
                SourceType::Web,
                "https://b.com".to_string(),
                "Rust Guide".to_string(),
                "Rust stuff".to_string(),
            ),
            SearchResult::new(
                SourceType::Web,
                "https://c.com".to_string(),
                "Python Guide".to_string(),
                "Python stuff".to_string(),
            ),
        ];
        let sorted = scorer.score_and_sort(results);
        assert_eq!(sorted[0].title, "Rust Guide");
        assert!(sorted[0].relevance_score > sorted[1].relevance_score);
    }

    #[test]
    fn test_relevance_scorer_as_processor() {
        let scorer = RelevanceScorer::new("rust");
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Rust Guide".to_string(),
            "Learn rust".to_string(),
        );
        let processed = scorer.process(result);
        assert!(processed.relevance_score > 0.0);
    }

    #[test]
    fn test_search_result_processor_process_batch() {
        let scorer = RelevanceScorer::new("rust");
        let results = vec![
            SearchResult::new(
                SourceType::Web,
                "https://a.com".to_string(),
                "Rust A".to_string(),
                "rust".to_string(),
            ),
            SearchResult::new(
                SourceType::Web,
                "https://b.com".to_string(),
                "Java B".to_string(),
                "java".to_string(),
            ),
        ];
        let processed = scorer.process_batch(results);
        assert_eq!(processed.len(), 2);
        assert!(processed[0].relevance_score > processed[1].relevance_score);
    }

    #[tokio::test]
    async fn test_mock_provider_search() {
        let provider = MockProvider {
            source: SourceType::Web,
            name: "Test",
            rate: None,
        };
        let query = SearchQuery::new("test".to_string());
        let results = provider.search(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_type, SourceType::Web);
    }

    #[tokio::test]
    async fn test_mock_provider_extract() {
        let provider = MockProvider {
            source: SourceType::Web,
            name: "Test",
            rate: None,
        };
        let content = provider
            .extract_content("https://example.com")
            .await
            .unwrap();
        assert_eq!(content.url, "https://example.com");
    }

    #[tokio::test]
    async fn test_mock_provider_source_type() {
        let provider = MockProvider {
            source: SourceType::Academic,
            name: "Academic",
            rate: None,
        };
        assert_eq!(provider.source_type(), SourceType::Academic);
        assert_eq!(provider.display_name(), "Academic");
    }

    #[tokio::test]
    async fn test_mock_provider_rate_limit() {
        let provider = MockProvider {
            source: SourceType::Web,
            name: "Test",
            rate: Some(std::time::Duration::from_secs(1)),
        };
        assert_eq!(provider.rate_limit(), Some(std::time::Duration::from_secs(1)));
    }
}
