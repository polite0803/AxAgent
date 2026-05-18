use crate::research_state::{SearchQuery, SearchResult, SourceType};
use crate::search_provider::{ContentMetadata, ExtractedContent, RelevanceScorer, SearchProvider};
use async_trait::async_trait;
use axagent_core::html_cleaner::HtmlCleaner;
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
        if let Some(ref api_key) = self.config.api_key
            && let Some(ref endpoint) = self.config.endpoint
        {
            return self.search_via_api(query, endpoint, api_key).await;
        }
        self.search_via_ddg(query).await
    }

    async fn search_via_ddg(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let ddg_url =
            format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(&query.query));

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
            .zip(wiki_response.snippets)
            .zip(wiki_response.urls)
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

        if domain.is_empty() { 0.5 } else { 0.7 }
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

        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| crate::search_provider::ExtractError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::search_provider::ExtractError::FetchError(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let html = response
            .text()
            .await
            .map_err(|e| crate::search_provider::ExtractError::FetchError(e.to_string()))?;

        if !content_type.contains("text/html") && !content_type.contains("application/xhtml") {
            let text = if html.len() > 50_000 {
                format!("{}...\n[Content truncated]", &html[..50_000])
            } else {
                html
            };
            return Ok(ExtractedContent::new(
                url.to_string(),
                url.split('/').nth(2).unwrap_or("unknown").to_string(),
                text,
            ));
        }

        let cleaner = HtmlCleaner::new();
        let (title, body_text, links) = cleaner.extract_readability(&html);
        let lang = HtmlCleaner::detect_language(&body_text);

        Ok(ExtractedContent::new(url.to_string(), title, body_text)
            .with_links(links)
            .with_metadata(ContentMetadata {
                author: None,
                published_date: None,
                description: None,
                keywords: Vec::new(),
                language: Some(lang.to_string()),
            }))
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

    #[test]
    fn test_web_search_config_default() {
        let config = WebSearchConfig::default();
        assert!(config.api_key.is_none());
        assert!(config.endpoint.is_none());
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.rate_limit_per_minute, Some(60));
    }

    #[test]
    fn test_web_search_provider_new() {
        let provider = WebSearchProvider::new();
        assert_eq!(provider.source_type(), SourceType::Web);
        assert_eq!(provider.display_name(), "Web Search");
    }

    #[test]
    fn test_web_search_provider_default() {
        let provider = WebSearchProvider::default();
        assert_eq!(provider.source_type(), SourceType::Web);
    }

    #[test]
    fn test_web_search_provider_with_api_key() {
        let provider = WebSearchProvider::new().with_api_key("test-key-123");
        assert_eq!(provider.config.api_key, Some("test-key-123".to_string()));
    }

    #[test]
    fn test_web_search_provider_with_endpoint() {
        let provider = WebSearchProvider::new().with_endpoint("https://api.example.com/search");
        assert_eq!(provider.config.endpoint, Some("https://api.example.com/search".to_string()));
    }

    #[test]
    fn test_web_search_provider_with_config() {
        let config = WebSearchConfig {
            api_key: Some("key".to_string()),
            endpoint: Some("https://api.test.com".to_string()),
            timeout_secs: 10,
            rate_limit_per_minute: Some(30),
        };
        let provider = WebSearchProvider::with_config(config);
        assert_eq!(provider.config.timeout_secs, 10);
        assert_eq!(provider.config.rate_limit_per_minute, Some(30));
    }

    #[test]
    fn test_calculate_relevance_title_match() {
        let provider = WebSearchProvider::new();
        let score = provider.calculate_relevance("Rust Programming Guide", "some snippet", "rust");
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_calculate_relevance_snippet_match() {
        let provider = WebSearchProvider::new();
        let score = provider.calculate_relevance("Some Title", "learn rust programming", "rust");
        assert!(score > 0.0);
    }

    #[test]
    fn test_calculate_relevance_both_match() {
        let provider = WebSearchProvider::new();
        let score = provider.calculate_relevance("Rust Guide", "rust programming language", "rust");
        let title_only = provider.calculate_relevance("Rust Guide", "unrelated content", "rust");
        assert!(score > title_only);
    }

    #[test]
    fn test_calculate_relevance_no_match() {
        let provider = WebSearchProvider::new();
        let score = provider.calculate_relevance("Python Tutorial", "learn python", "rust");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_calculate_relevance_capped_at_one() {
        let provider = WebSearchProvider::new();
        let score = provider.calculate_relevance(
            "rust rust rust rust rust",
            "rust rust rust rust rust",
            "rust",
        );
        assert!(score <= 1.0);
    }

    #[test]
    fn test_calculate_relevance_multi_word_query() {
        let provider = WebSearchProvider::new();
        let score = provider.calculate_relevance(
            "Rust Programming Language",
            "Learn rust programming",
            "rust programming",
        );
        assert!(score > 0.3);
    }

    #[test]
    fn test_calculate_relevance_case_insensitive() {
        let provider = WebSearchProvider::new();
        let score = provider.calculate_relevance("RUST PROGRAMMING", "RUST SNIPPET", "rust");
        assert!(score > 0.0);
    }

    #[test]
    fn test_estimate_credibility_high_credibility_arxiv() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("https://arxiv.org/abs/2103.00001");
        assert_eq!(score, 0.9);
    }

    #[test]
    fn test_estimate_credibility_high_credibility_github() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("https://github.com/user/repo");
        assert_eq!(score, 0.9);
    }

    #[test]
    fn test_estimate_credibility_high_credibility_stackoverflow() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("https://stackoverflow.com/questions/123");
        assert_eq!(score, 0.9);
    }

    #[test]
    fn test_estimate_credibility_high_credibility_wikipedia() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("https://en.wikipedia.org/wiki/Rust");
        assert_eq!(score, 0.9);
    }

    #[test]
    fn test_estimate_credibility_high_credibility_pubmed() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("https://pubmed.gov/123456/");
        assert_eq!(score, 0.9);
    }

    #[test]
    fn test_estimate_credibility_high_credibility_nature() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("https://nature.com/articles/s41586");
        assert_eq!(score, 0.9);
    }

    #[test]
    fn test_estimate_credibility_high_credibility_science() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("https://science.org/doi/10.1126");
        assert_eq!(score, 0.9);
    }

    #[test]
    fn test_estimate_credibility_high_credibility_doi() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("https://doi.org/10.1234/test");
        assert_eq!(score, 0.9);
    }

    #[test]
    fn test_estimate_credibility_normal_domain() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("https://example.com/page");
        assert_eq!(score, 0.7);
    }

    #[test]
    fn test_estimate_credibility_empty_domain() {
        let provider = WebSearchProvider::new();
        let score = provider.estimate_credibility("no-slash-url");
        assert_eq!(score, 0.5);
    }

    #[test]
    fn test_parse_ddg_html_with_results() {
        let provider = WebSearchProvider::new();
        let html = r#"
        <html><body>
        <div class="result__a" href="https://example.com/rust">Rust Programming</div>
        <div class="result__snippet">Learn Rust programming language</div>
        <div class="result__a" href="https://github.com/rust-lang/rust">Rust Language Repo</div>
        <div class="result__snippet">The Rust programming language repository</div>
        </body></html>
        "#;
        let query = SearchQuery::new("rust programming".to_string()).with_max_results(5);
        let results = provider.parse_ddg_html(html, &query);
        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Rust Programming");
        assert_eq!(results[0].url, "https://example.com/rust");
        assert_eq!(results[1].url, "https://github.com/rust-lang/rust");
    }

    #[test]
    fn test_parse_ddg_html_respects_max_results() {
        let provider = WebSearchProvider::new();
        let html = r#"
        <html><body>
        <div class="result__a" href="https://a.com">Result A</div>
        <div class="result__snippet">Snippet A</div>
        <div class="result__a" href="https://b.com">Result B</div>
        <div class="result__snippet">Snippet B</div>
        <div class="result__a" href="https://c.com">Result C</div>
        <div class="result__snippet">Snippet C</div>
        </body></html>
        "#;
        let query = SearchQuery::new("test".to_string()).with_max_results(2);
        let results = provider.parse_ddg_html(html, &query);
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 2);
    }

    #[test]
    fn test_parse_ddg_html_skips_empty_links() {
        let provider = WebSearchProvider::new();
        let html = r#"
        <html><body>
        <div class="result__a" href="">Empty Link</div>
        <div class="result__snippet">Snippet</div>
        <div class="result__a" href="https://example.com">Valid Link</div>
        <div class="result__snippet">Valid Snippet</div>
        </body></html>
        "#;
        let query = SearchQuery::new("test".to_string()).with_max_results(10);
        let results = provider.parse_ddg_html(html, &query);
        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.com");
    }

    #[test]
    fn test_parse_ddg_html_no_snippet() {
        let provider = WebSearchProvider::new();
        let html = r#"
        <html><body>
        <div class="result__a" href="https://example.com">No Snippet Result</div>
        </body></html>
        "#;
        let query = SearchQuery::new("test".to_string()).with_max_results(10);
        let results = provider.parse_ddg_html(html, &query);
        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].snippet, "");
    }

    #[test]
    fn test_extract_readability_basic_html() {
        let html = r#"
        <html><head><title>Test Page</title></head>
        <body><main><p>Hello world example content</p></main></body></html>
        "#;
        let cleaner = HtmlCleaner::new();
        let (title, body_text, links) = cleaner.extract_readability(html);
        assert_eq!(title, "Test Page");
        assert!(body_text.contains("Hello world"));
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_readability_extracts_links() {
        let html = r#"
        <html><head><title>Link Page</title></head>
        <body><article>
            <a href="https://example.com/a">Link A</a>
            <a href="https://example.com/b">Link B</a>
            <a href="/relative">Relative</a>
        </article></body></html>
        "#;
        let cleaner = HtmlCleaner::new();
        let (title, _, links) = cleaner.extract_readability(html);
        assert_eq!(title, "Link Page");
        assert_eq!(links.len(), 2);
        assert!(links.contains(&"https://example.com/a".to_string()));
        assert!(links.contains(&"https://example.com/b".to_string()));
    }

    #[test]
    fn test_extract_readability_no_title() {
        let html = r#"<html><body><p>No title page</p></body></html>"#;
        let cleaner = HtmlCleaner::new();
        let (title, body_text, _) = cleaner.extract_readability(html);
        assert_eq!(title, "");
        assert!(body_text.contains("No title page"));
    }

    #[test]
    fn test_detect_language_english() {
        let text = "This is a sample English text for language detection testing purposes";
        assert_eq!(HtmlCleaner::detect_language(text), "en");
    }

    #[test]
    fn test_detect_language_chinese() {
        let text = "这是一段用于语言检测的中文文本内容测试";
        assert_eq!(HtmlCleaner::detect_language(text), "zh");
    }

    #[tokio::test]
    async fn test_extract_content_empty_url() {
        let provider = WebSearchProvider::new();
        let result = provider.extract_content("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_content_invalid_url() {
        let provider = WebSearchProvider::new();
        let result = provider.extract_content("not-a-url").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_with_config() {
        let provider = WebSearchProvider::new();
        let rate_limit = provider.rate_limit();
        assert!(rate_limit.is_some());
        let duration = rate_limit.unwrap();
        assert_eq!(duration.as_secs(), 60);
    }

    #[test]
    fn test_rate_limit_none_when_not_set() {
        let config = WebSearchConfig {
            rate_limit_per_minute: None,
            ..Default::default()
        };
        let provider = WebSearchProvider::with_config(config);
        assert!(provider.rate_limit().is_none());
    }

    #[test]
    fn test_rate_limit_custom() {
        let config = WebSearchConfig {
            rate_limit_per_minute: Some(120),
            ..Default::default()
        };
        let provider = WebSearchProvider::with_config(config);
        let rate_limit = provider.rate_limit();
        assert!(rate_limit.is_some());
        assert_eq!(rate_limit.unwrap().as_secs(), 30);
    }

    #[test]
    fn test_source_type_is_web() {
        let provider = WebSearchProvider::new();
        assert_eq!(provider.source_type(), SourceType::Web);
    }

    #[test]
    fn test_display_name() {
        let provider = WebSearchProvider::new();
        assert_eq!(provider.display_name(), "Web Search");
    }

    #[test]
    fn test_builder_new() {
        let builder = WebSearchProviderBuilder::new();
        let provider = builder.build();
        assert_eq!(provider.source_type(), SourceType::Web);
    }

    #[test]
    fn test_builder_default() {
        let builder = WebSearchProviderBuilder::default();
        let provider = builder.build();
        assert_eq!(provider.source_type(), SourceType::Web);
    }

    #[test]
    fn test_builder_with_api_key() {
        let provider = WebSearchProviderBuilder::new()
            .api_key("my-api-key")
            .build();
        assert_eq!(provider.config.api_key, Some("my-api-key".to_string()));
    }

    #[test]
    fn test_builder_with_endpoint() {
        let provider = WebSearchProviderBuilder::new()
            .endpoint("https://search.api.com")
            .build();
        assert_eq!(provider.config.endpoint, Some("https://search.api.com".to_string()));
    }

    #[test]
    fn test_builder_with_timeout() {
        let provider = WebSearchProviderBuilder::new().timeout(60).build();
        assert_eq!(provider.config.timeout_secs, 60);
    }

    #[test]
    fn test_builder_with_rate_limit() {
        let provider = WebSearchProviderBuilder::new().rate_limit(100).build();
        assert_eq!(provider.config.rate_limit_per_minute, Some(100));
    }

    #[test]
    fn test_builder_full_configuration() {
        let provider = WebSearchProviderBuilder::new()
            .api_key("key123")
            .endpoint("https://api.example.com")
            .timeout(45)
            .rate_limit(30)
            .build();
        assert_eq!(provider.config.api_key, Some("key123".to_string()));
        assert_eq!(provider.config.endpoint, Some("https://api.example.com".to_string()));
        assert_eq!(provider.config.timeout_secs, 45);
        assert_eq!(provider.config.rate_limit_per_minute, Some(30));
    }

    #[test]
    fn test_web_search_config_serialization() {
        let config = WebSearchConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WebSearchConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.timeout_secs, config.timeout_secs);
        assert_eq!(deserialized.rate_limit_per_minute, config.rate_limit_per_minute);
    }
}
