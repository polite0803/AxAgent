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
    http_client: reqwest::Client,
}

impl WebSearchProvider {
    pub fn new() -> Self {
        Self::with_config(WebSearchConfig::default())
    }

    pub fn with_config(config: WebSearchConfig) -> Self {
        let timeout = Duration::from_secs(config.timeout_secs);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .unwrap_or_default();
        Self {
            config,
            http_client: client,
        }
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
        if let Some(ref api_key) = self.config.api_key {
            if let Some(ref endpoint) = self.config.endpoint {
                return self.search_via_api(query, endpoint, api_key).await;
            }
        }
        self.search_via_ddg(query).await
    }

    async fn search_via_ddg(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let ddg_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(&query.query)
        );

        let response = self
            .http_client
            .get(&ddg_url)
            .send()
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        let html = response
            .text()
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        self.parse_ddg_html(&html, query)
    }

    fn parse_ddg_html(
        &self,
        html: &str,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let mut results = Vec::new();
        let doc = scraper::Html::parse_document(html);
        let result_selector = scraper::Selector::parse(".result__a")
            .map_err(|e| crate::search_provider::SearchError::ParseError(e.to_string()))?;
        let snippet_selector = scraper::Selector::parse(".result__snippet")
            .map_err(|e| crate::search_provider::SearchError::ParseError(e.to_string()))?;

        for (idx, result_element) in doc.select(&result_selector).enumerate() {
            if idx >= query.max_results {
                break;
            }

            let link = result_element.value().attr("href").unwrap_or("");
            let title = result_element.text().collect::<String>();

            let snippet = result_element
                .select(&snippet_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            if !link.is_empty() {
                let relevance = self.calculate_relevance(&title, &snippet, &query.query);
                let credibility = self.estimate_credibility(link);

                results.push(
                    SearchResult::new(
                        SourceType::Web,
                        link.to_string(),
                        title.trim().to_string(),
                        snippet.trim().to_string(),
                    )
                    .with_credibility(credibility)
                    .with_relevance(relevance),
                );
            }
        }

        if results.is_empty() {
            let wiki_results =
                tokio::runtime::Handle::current().block_on(self.search_via_wikipedia(query));
            return wiki_results;
        }

        Ok(results)
    }

    async fn search_via_wikipedia(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let wiki_url = format!(
            "https://en.wikipedia.org/w/api.php?action=opensearch&search={}&limit={}&format=json",
            urlencoding::encode(&query.query),
            query.max_results
        );

        let response = self
            .http_client
            .get(&wiki_url)
            .send()
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        #[derive(Deserialize)]
        struct WikiSearchResponse {
            #[serde(rename = "[1]")]
            titles: Vec<String>,
            #[serde(rename = "[2]")]
            snippets: Vec<String>,
            #[serde(rename = "[3]")]
            urls: Vec<String>,
        }

        let wiki_response: WikiSearchResponse = response
            .json()
            .await
            .map_err(|e| crate::search_provider::SearchError::ParseError(e.to_string()))?;

        let scorer = RelevanceScorer::new(&query.query);

        let max_results = query.max_results;
        let results: Vec<SearchResult> = wiki_response
            .titles
            .into_iter()
            .zip(wiki_response.snippets.into_iter())
            .zip(wiki_response.urls.into_iter())
            .enumerate()
            .filter(|(idx, _)| *idx < max_results)
            .map(|(_, ((title, snippet), url))| {
                let result = SearchResult::new(SourceType::Wikipedia, url, title, snippet)
                    .with_credibility(SourceType::Wikipedia.default_credibility());
                let relevance = scorer.score(&result);
                result.with_relevance(relevance)
            })
            .collect();

        Ok(results)
    }

    async fn search_via_api(
        &self,
        query: &SearchQuery,
        endpoint: &str,
        _api_key: &str,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let url = if endpoint.contains('?') {
            format!("{}&q={}", endpoint, urlencoding::encode(&query.query))
        } else {
            format!("{}?q={}", endpoint, urlencoding::encode(&query.query))
        };

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        #[derive(Deserialize)]
        struct ApiSearchResult {
            title: String,
            url: String,
            snippet: Option<String>,
        }

        #[derive(Deserialize)]
        struct ApiResponse {
            results: Vec<ApiSearchResult>,
        }

        let api_response: ApiResponse = response
            .json()
            .await
            .map_err(|e| crate::search_provider::SearchError::ParseError(e.to_string()))?;

        let scorer = RelevanceScorer::new(&query.query);

        let results: Vec<SearchResult> = api_response
            .results
            .into_iter()
            .take(query.max_results)
            .map(|r| {
                let snippet = r.snippet.unwrap_or_default();
                let url = r.url.clone();
                let credibility = self.estimate_credibility(&url);
                let result = SearchResult::new(SourceType::Web, r.url, r.title, snippet)
                    .with_credibility(credibility);
                let relevance = scorer.score(&result);
                result.with_relevance(relevance)
            })
            .collect();

        Ok(results)
    }

    fn calculate_relevance(&self, title: &str, snippet: &str, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let title_lower = title.to_lowercase();
        let snippet_lower = snippet.to_lowercase();

        let mut score: f32 = 0.0;
        for word in query_lower.split_whitespace() {
            if title_lower.contains(word) {
                score += 0.3;
            }
            if snippet_lower.contains(word) {
                score += 0.1;
            }
        }

        score.min(1.0)
    }

    fn estimate_credibility(&self, url: &str) -> f32 {
        let domain = url.split('/').nth(2).unwrap_or("");
        let high_credibility = [
            "arxiv.org",
            "github.com",
            "stackoverflow.com",
            "wikipedia.org",
            "doi.org",
            "pubmed.gov",
            "nature.com",
            "science.org",
        ];

        for credible in high_credibility {
            if domain.ends_with(credible) {
                return 0.9;
            }
        }

        if domain.is_empty() {
            0.5
        } else {
            0.7
        }
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
