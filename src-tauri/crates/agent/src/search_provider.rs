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
