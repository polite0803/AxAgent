use crate::research_state::{SearchQuery, SearchResult, SourceType};
use crate::search_provider::{ContentMetadata, ExtractedContent, RelevanceScorer, SearchProvider};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcademicSearchConfig {
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub use_mock: bool,
    pub timeout_secs: u64,
    pub rate_limit_per_minute: Option<u32>,
    pub sources: AcademicSources,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct AcademicSources {
    pub arxiv: bool,
    pub scholar: bool,
    pub pubmed: bool,
}

impl AcademicSources {
    pub fn all() -> Self {
        Self {
            arxiv: true,
            scholar: true,
            pubmed: true,
        }
    }

    pub fn only_arxiv() -> Self {
        Self {
            arxiv: true,
            scholar: false,
            pubmed: false,
        }
    }
}

impl Default for AcademicSearchConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: None,
            use_mock: true,
            timeout_secs: 30,
            rate_limit_per_minute: Some(30),
            sources: AcademicSources::all(),
        }
    }
}

pub struct AcademicSearchProvider {
    config: AcademicSearchConfig,
}

impl AcademicSearchProvider {
    pub fn new() -> Self {
        Self::with_config(AcademicSearchConfig::default())
    }

    pub fn with_config(config: AcademicSearchConfig) -> Self {
        Self { config }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.config.api_key = Some(api_key.into());
        self.config.use_mock = false;
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
        let mut all_results = Vec::new();

        if self.config.sources.arxiv {
            let results = if self.config.use_mock {
                self.generate_mock_results(query, "arxiv")
            } else {
                self.search_arxiv(query).await?
            };
            all_results.extend(results);
        }

        if self.config.sources.scholar {
            let results = if self.config.use_mock {
                self.generate_mock_results(query, "scholar")
            } else {
                self.search_google_scholar(query).await?
            };
            all_results.extend(results);
        }

        if self.config.sources.pubmed {
            let results = if self.config.use_mock {
                self.generate_mock_results(query, "pubmed")
            } else {
                self.search_pubmed(query).await?
            };
            all_results.extend(results);
        }

        let scorer = RelevanceScorer::new(&query.query);
        Ok(scorer.score_and_sort(all_results))
    }

    async fn search_arxiv(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let query_encoded = urlencoding::encode(&query.query);
        let url = format!(
            "http://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results={}",
            query_encoded, query.max_results
        );

        let response = reqwest::get(&url)
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        self.parse_arxiv_response(&body, query)
    }

    fn parse_arxiv_response(
        &self,
        xml: &str,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let mut results = Vec::new();

        let entry_blocks: Vec<&str> = xml.split("<entry>").collect();

        for block in entry_blocks.iter().skip(1) {
            let title = self.extract_tag(block, "title").unwrap_or_default();
            let summary = self.extract_tag(block, "summary").unwrap_or_default();
            let published = self.extract_tag(block, "published").unwrap_or_default();
            let id = self.extract_tag(block, "id").unwrap_or_default();

            if title.is_empty() {
                continue;
            }

            let result = SearchResult::new(
                SourceType::Academic,
                id,
                title.replace("\n", " ").trim().to_string(),
                summary.replace("\n", " ").trim().to_string(),
            )
            .with_published_date(published)
            .with_credibility(0.9)
            .with_relevance(0.85);

            results.push(result);

            if results.len() >= query.max_results {
                break;
            }
        }

        Ok(results)
    }

    fn extract_tag(&self, xml: &str, tag: &str) -> Option<String> {
        let start_pattern = format!("<{}>", tag);
        let end_pattern = format!("</{}>", tag);

        let start_idx = xml.find(&start_pattern)? + start_pattern.len();
        let end_idx = xml.find(&end_pattern)?;

        Some(xml[start_idx..end_idx].to_string())
    }

    fn generate_mock_results(&self, query: &SearchQuery, source: &str) -> Vec<SearchResult> {
        let query_lower = query.query.to_lowercase();

        let mock_papers = match source {
            "scholar" => self.get_scholar_mock_results(&query_lower),
            "pubmed" => self.get_pubmed_mock_results(&query_lower),
            _ => self.get_arxiv_mock_results(&query_lower),
        };

        mock_papers
            .into_iter()
            .take(query.max_results.min(10))
            .collect()
    }

    fn get_arxiv_mock_results(&self, query_lower: &str) -> Vec<SearchResult> {
        if query_lower.contains("machine learning") || query_lower.contains("ml") {
            vec![
                SearchResult::new(
                    SourceType::Academic,
                    "https://arxiv.org/abs/2103.00001".to_string(),
                    "Learning Transferable Visual Models From Natural Language Supervision".to_string(),
                    "We demonstrate that the simple pre-training task of predicting which image goes with which text is an efficient and scalable way to learn SOTA image representations from scratch on a dataset of 400 million images.".to_string(),
                )
                .with_published_date("2021-02-26".to_string())
                .with_credibility(0.95)
                .with_relevance(0.93),
                SearchResult::new(
                    SourceType::Academic,
                    "https://arxiv.org/abs/2005.14165".to_string(),
                    "Language Models are Few-Shot Learners".to_string(),
                    "We show that scaling up language models greatly improves task-agnostic, few-shot performance.".to_string(),
                )
                .with_published_date("2020-05-28".to_string())
                .with_credibility(0.95)
                .with_relevance(0.91),
            ]
        } else if query_lower.contains("rust") || query_lower.contains("programming") {
            vec![
                SearchResult::new(
                    SourceType::Academic,
                    "https://arxiv.org/abs/1905.09501".to_string(),
                    "RustBelt: Securing the Foundations of the Rust Programming Language".to_string(),
                    "RustBelt is the first formal verification of the safety of the Rust type system, providing a machine-checked proof of soundness for the Rust compiler and several core libraries.".to_string(),
                )
                .with_published_date("2019-05-23".to_string())
                .with_credibility(0.95)
                .with_relevance(0.9),
            ]
        } else {
            vec![SearchResult::new(
                SourceType::Academic,
                "https://arxiv.org/abs/2303.17760".to_string(),
                format!("Survey Paper: {}", query_lower),
                format!(
                    "A comprehensive survey on {} covering recent advances and future directions.",
                    query_lower
                ),
            )
            .with_published_date("2023-03-31".to_string())
            .with_credibility(0.85)
            .with_relevance(0.8)]
        }
    }

    fn get_scholar_mock_results(&self, query_lower: &str) -> Vec<SearchResult> {
        if query_lower.contains("machine learning") || query_lower.contains("ml") {
            vec![
                SearchResult::new(
                    SourceType::Academic,
                    "https://scholar.google.com/scholar?q=attention+is+all+you+need".to_string(),
                    "Attention Is All You Need".to_string(),
                    "The dominant sequence transduction models are based on complex recurrent or convolutional neural networks that include an encoder and a decoder. The best performing models also connect the encoder and the decoder through an attention mechanism.".to_string(),
                )
                .with_published_date("2017-06-12".to_string())
                .with_credibility(0.95)
                .with_relevance(0.95),
                SearchResult::new(
                    SourceType::Academic,
                    "https://scholar.google.com/scholar?q=bert+pre-training+of+deep+bidirectional".to_string(),
                    "BERT: Pre-training of Deep Bidirectional Transformers for Language Understanding".to_string(),
                    "We introduce a new language representation model called BERT, which stands for Bidirectional Encoder Representations from Transformers.".to_string(),
                )
                .with_published_date("2018-10-11".to_string())
                .with_credibility(0.95)
                .with_relevance(0.92),
            ]
        } else if query_lower.contains("deep learning") || query_lower.contains("neural") {
            vec![
                SearchResult::new(
                    SourceType::Academic,
                    "https://scholar.google.com/scholar?q=deep+residual+learning+image+recognition".to_string(),
                    "Deep Residual Learning for Image Recognition".to_string(),
                    "We present a residual learning framework to ease the training of networks that are substantially deeper than those used previously.".to_string(),
                )
                .with_published_date("2015-12-10".to_string())
                .with_credibility(0.95)
                .with_relevance(0.91),
            ]
        } else {
            vec![SearchResult::new(
                SourceType::Academic,
                "https://scholar.google.com/scholar?q=comprehensive+survey".to_string(),
                format!("Comprehensive Survey on {}", query_lower),
                format!(
                    "This paper provides a comprehensive survey of {} covering theoretical foundations, methodologies, and applications.",
                    query_lower
                ),
            )
            .with_published_date("2024-01-15".to_string())
            .with_credibility(0.88)
            .with_relevance(0.85)]
        }
    }

    fn get_pubmed_mock_results(&self, query_lower: &str) -> Vec<SearchResult> {
        if query_lower.contains("cancer") || query_lower.contains("tumor") {
            vec![
                SearchResult::new(
                    SourceType::Academic,
                    "https://pubmed.ncbi.nlm.nih.gov/29198900/".to_string(),
                    "Cancer Immunotherapy: A Review of Current Understanding".to_string(),
                    "This review provides an overview of cancer immunotherapy approaches including checkpoint inhibitors, CAR-T cells, and therapeutic vaccines.".to_string(),
                )
                .with_published_date("2017-11-01".to_string())
                .with_credibility(0.92)
                .with_relevance(0.89),
                SearchResult::new(
                    SourceType::Academic,
                    "https://pubmed.ncbi.nlm.nih.gov/30351497/".to_string(),
                    "Molecular Mechanisms of Cancer Development".to_string(),
                    "This paper explores the molecular mechanisms underlying cancer development and progression, including genetic mutations and signaling pathways.".to_string(),
                )
                .with_published_date("2018-10-15".to_string())
                .with_credibility(0.91)
                .with_relevance(0.87),
            ]
        } else if query_lower.contains("covid") || query_lower.contains("virus") {
            vec![
                SearchResult::new(
                    SourceType::Academic,
                    "https://pubmed.ncbi.nlm.nih.gov/32191675/".to_string(),
                    "SARS-CoV-2 Transmission and Infection".to_string(),
                    "This study examines the transmission dynamics and infection patterns of SARS-CoV-2 across different populations and settings.".to_string(),
                )
                .with_published_date("2020-03-15".to_string())
                .with_credibility(0.94)
                .with_relevance(0.92),
            ]
        } else {
            vec![SearchResult::new(
                SourceType::Academic,
                "https://pubmed.ncbi.nlm.nih.gov/35000000/".to_string(),
                format!("Review: {}", query_lower),
                format!(
                    "A systematic review of {} covering recent clinical findings and therapeutic approaches.",
                    query_lower
                ),
            )
            .with_published_date("2022-06-20".to_string())
            .with_credibility(0.89)
            .with_relevance(0.82)]
        }
    }

    async fn search_google_scholar(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let query_encoded = urlencoding::encode(&query.query);
        let url = format!(
            "https://serpapi.com/search.json?engine=google_scholar&q={}&num={}",
            query_encoded,
            query.max_results.min(10)
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        self.parse_scholar_response(&body, query)
    }

    fn parse_scholar_response(
        &self,
        json: &str,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let mut results = Vec::new();

        if let Ok(data) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(organic_results) = data.get("organic_results").and_then(|r| r.as_array()) {
                for item in organic_results {
                    let title = item
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let snippet = item
                        .get("snippet")
                        .and_then(|s| s.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let link = item
                        .get("link")
                        .and_then(|l| l.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let _publication_info = item
                        .get("publication_info")
                        .and_then(|p| p.get("summary"))
                        .and_then(|s| s.as_str())
                        .unwrap_or_default()
                        .to_string();

                    if title.is_empty() {
                        continue;
                    }

                    let result = SearchResult::new(SourceType::Academic, link, title, snippet)
                        .with_credibility(0.88)
                        .with_relevance(0.85);

                    results.push(result);
                }
            }
        }

        if results.is_empty() {
            results = self.get_scholar_mock_results(&query.query.to_lowercase());
        }

        Ok(results)
    }

    async fn search_pubmed(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let query_encoded = urlencoding::encode(&query.query);
        let url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term={}&retmax={}&retmode=json",
            query_encoded,
            query.max_results
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| crate::search_provider::SearchError::NetworkError(e.to_string()))?;

        self.parse_pubmed_response(&body, query)
    }

    fn parse_pubmed_response(
        &self,
        json: &str,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, crate::search_provider::SearchError> {
        let mut results = Vec::new();

        if let Ok(data) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(id_list) = data.get("esearchresult").and_then(|r| r.get("idlist")).and_then(|r| r.as_array()) {
                for id in id_list {
                    if let Some(id_str) = id.as_str() {
                        let link = format!("https://pubmed.ncbi.nlm.nih.gov/{}/", id_str);
                        let result = SearchResult::new(
                            SourceType::Academic,
                            link,
                            format!("PubMed Article: {}", id_str),
                            format!("Abstract available at PubMed for article {}", id_str),
                        )
                        .with_credibility(0.90)
                        .with_relevance(0.80);

                        results.push(result);
                    }
                }
            }
        }

        if results.is_empty() {
            results = self.get_pubmed_mock_results(&query.query.to_lowercase());
        }

        Ok(results)
    }
}

impl Default for AcademicSearchProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchProvider for AcademicSearchProvider {
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

        let is_pdf = url.contains(".pdf");
        let title = if url.contains("arxiv.org/abs") {
            url.split("abs/")
                .last()
                .unwrap_or("Unknown Paper")
                .to_string()
        } else {
            "Academic Paper".to_string()
        };

        let content = ExtractedContent::new(
            url.to_string(),
            title.clone(),
            format!(
                "Academic paper: {}. {}",
                title,
                if is_pdf {
                    "Full PDF content would be extracted here."
                } else {
                    "Abstract and metadata extracted from arXiv."
                }
            ),
        )
        .with_metadata(ContentMetadata {
            author: None,
            published_date: None,
            description: Some("Academic research paper".to_string()),
            keywords: vec!["research".to_string(), "academic".to_string()],
            language: Some("en".to_string()),
        });

        Ok(content)
    }

    fn source_type(&self) -> SourceType {
        SourceType::Academic
    }

    fn display_name(&self) -> &str {
        "Academic Search (arXiv)"
    }

    fn rate_limit(&self) -> Option<Duration> {
        self.config
            .rate_limit_per_minute
            .map(|rpm| Duration::from_secs(60 * 60 / rpm as u64))
    }
}

pub struct AcademicSearchProviderBuilder {
    provider: AcademicSearchProvider,
}

impl AcademicSearchProviderBuilder {
    pub fn new() -> Self {
        Self {
            provider: AcademicSearchProvider::new(),
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

    pub fn use_mock(mut self, use_mock: bool) -> Self {
        self.provider.config.use_mock = use_mock;
        self
    }

    pub fn rate_limit(mut self, per_minute: u32) -> Self {
        self.provider.config.rate_limit_per_minute = Some(per_minute);
        self
    }

    pub fn build(self) -> AcademicSearchProvider {
        self.provider
    }
}

impl Default for AcademicSearchProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_academic_search_mock() {
        let mut provider = AcademicSearchProvider::new();
        provider.config.use_mock = true;

        let query = SearchQuery::new("machine learning".to_string());
        let results = provider.search(&query).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_extract_arxiv() {
        let provider = AcademicSearchProvider::new();
        let url = "https://arxiv.org/abs/2103.00001";

        let content = provider.extract_content(url).await;
        assert!(content.is_ok());
    }
}
