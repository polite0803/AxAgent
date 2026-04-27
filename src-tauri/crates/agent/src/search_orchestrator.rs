use crate::research_state::{SearchPlan, SearchQuery, SearchResult, SourceType};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Search provider error: {0}")]
    ProviderError(String),
    #[error("No providers available for source type: {0:?}")]
    NoProviderForSource(SourceType),
    #[error("Query execution failed: {0}")]
    QueryFailed(String),
    #[error("Result deduplication failed: {0}")]
    DeduplicationFailed(String),
    #[error("Timeout exceeded for query: {0}")]
    Timeout(String),
}

#[derive(Clone)]
pub struct SearchOrchestrator {
    max_concurrent: usize,
    timeout_secs: u64,
    use_deduplication: bool,
}

impl Default for SearchOrchestrator {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            timeout_secs: 30,
            use_deduplication: true,
        }
    }
}

impl SearchOrchestrator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_deduplication(mut self, enabled: bool) -> Self {
        self.use_deduplication = enabled;
        self
    }

    pub async fn execute(&self, plan: &SearchPlan) -> Result<Vec<SearchResult>, OrchestratorError> {
        let mut all_results: Vec<SearchResult> = Vec::new();
        let mut query_results: HashMap<String, Vec<SearchResult>> = HashMap::new();

        for group in &plan.parallel_groups {
            let group_results = self.execute_parallel_group(group, plan).await?;
            for (query_id, results) in group_results {
                query_results.insert(query_id, results);
            }
        }

        for (_query_id, results) in query_results {
            all_results.extend(results);
        }

        if self.use_deduplication {
            all_results = self.deduplicate_results(all_results);
        }

        all_results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(all_results)
    }

    async fn execute_parallel_group(
        &self,
        query_ids: &[String],
        plan: &SearchPlan,
    ) -> Result<HashMap<String, Vec<SearchResult>>, OrchestratorError> {
        let mut handles = Vec::new();
        let timeout = self.timeout_secs;

        for query_id in query_ids {
            if let Some(query) = plan.queries.iter().find(|q| &q.id == query_id) {
                let query_clone = query.clone();
                let query_id_clone = query_id.clone();

                let handle = tokio::spawn(async move {
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(timeout),
                        Self::execute_single_query_static(&query_clone),
                    )
                    .await;

                    match result {
                        Ok(Ok(results)) => Ok((query_id_clone.clone(), results)),
                        Ok(Err(e)) => Err(OrchestratorError::QueryFailed(e.to_string())),
                        Err(_) => Err(OrchestratorError::Timeout(query_id_clone.clone())),
                    }
                });

                handles.push(handle);
            }
        }

        let mut results: HashMap<String, Vec<SearchResult>> = HashMap::new();

        for handle in handles {
            match handle.await {
                Ok(Ok((query_id, query_results))) => {
                    results.insert(query_id, query_results);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Query failed: {}", e);
                }
                Err(e) => {
                    tracing::warn!("Task join error: {}", e);
                }
            }
        }

        Ok(results)
    }

    async fn execute_single_query_static(
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, OrchestratorError> {
        let mut results: Vec<SearchResult> = Vec::new();

        for source_type in &query.source_types {
            let source_results = Self::search_source_static(query, *source_type).await?;
            results.extend(source_results);
        }

        results.truncate(query.max_results);
        Ok(results)
    }

    async fn search_source_static(
        query: &SearchQuery,
        source_type: SourceType,
    ) -> Result<Vec<SearchResult>, OrchestratorError> {
        match source_type {
            SourceType::Web => Ok(Self::mock_web_search(query)),
            SourceType::Wikipedia => Ok(Self::mock_wikipedia_search(query)),
            SourceType::Academic => Ok(Self::mock_academic_search(query)),
            SourceType::GitHub => Ok(Self::mock_github_search(query)),
            SourceType::Documentation => Ok(Self::mock_doc_search(query)),
            SourceType::News => Ok(Self::mock_news_search(query)),
            SourceType::Blog => Ok(Vec::new()),
            SourceType::Forum => Ok(Vec::new()),
            SourceType::Unknown => Ok(Vec::new()),
        }
    }

    fn mock_web_search(query: &SearchQuery) -> Vec<SearchResult> {
        vec![SearchResult::new(
            SourceType::Web,
            format!(
                "https://example.com/search?q={}",
                urlencoding::encode(&query.query)
            ),
            format!("Result for: {}", query.query),
            format!(
                "This is a mock search result snippet for the query: {}",
                query.query
            ),
        )
        .with_credibility(SourceType::Web.default_credibility())
        .with_relevance(0.8)]
    }

    fn mock_wikipedia_search(query: &SearchQuery) -> Vec<SearchResult> {
        vec![SearchResult::new(
            SourceType::Wikipedia,
            format!(
                "https://en.wikipedia.org/wiki/{}",
                urlencoding::encode(&query.query.replace(" ", "_"))
            ),
            query.query.clone(),
            format!("Wikipedia article about: {}", query.query),
        )
        .with_credibility(SourceType::Wikipedia.default_credibility())
        .with_relevance(0.95)]
    }

    fn mock_academic_search(query: &SearchQuery) -> Vec<SearchResult> {
        vec![SearchResult::new(
            SourceType::Academic,
            "https://scholar.google.com/search".to_string(),
            format!("Academic paper: {}", query.query),
            format!("Scholarly article discussing: {}", query.query),
        )
        .with_credibility(SourceType::Academic.default_credibility())
        .with_relevance(0.9)]
    }

    fn mock_github_search(query: &SearchQuery) -> Vec<SearchResult> {
        vec![SearchResult::new(
            SourceType::GitHub,
            format!(
                "https://github.com/search?q={}",
                urlencoding::encode(&query.query)
            ),
            format!("GitHub repositories: {}", query.query),
            format!("Open source projects related to: {}", query.query),
        )
        .with_credibility(SourceType::GitHub.default_credibility())
        .with_relevance(0.75)]
    }

    fn mock_doc_search(query: &SearchQuery) -> Vec<SearchResult> {
        vec![SearchResult::new(
            SourceType::Documentation,
            format!(
                "https://docs.example.com/search?q={}",
                urlencoding::encode(&query.query)
            ),
            format!("Documentation: {}", query.query),
            format!("Official documentation for: {}", query.query),
        )
        .with_credibility(SourceType::Documentation.default_credibility())
        .with_relevance(0.85)]
    }

    fn mock_news_search(query: &SearchQuery) -> Vec<SearchResult> {
        vec![SearchResult::new(
            SourceType::News,
            format!(
                "https://news.example.com/search?q={}",
                urlencoding::encode(&query.query)
            ),
            format!("News: {}", query.query),
            format!("Recent news about: {}", query.query),
        )
        .with_credibility(SourceType::News.default_credibility())
        .with_relevance(0.7)]
    }

    fn deduplicate_results(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut deduplicated: Vec<SearchResult> = Vec::new();

        for result in results {
            if seen_urls.contains(&result.url) {
                continue;
            }
            seen_urls.insert(result.url.clone());
            deduplicated.push(result);
        }

        deduplicated
    }

    pub fn calculate_source_distribution(results: &[SearchResult]) -> HashMap<SourceType, usize> {
        let mut distribution: HashMap<SourceType, usize> = HashMap::new();

        for result in results {
            *distribution.entry(result.source_type).or_insert(0) += 1;
        }

        distribution
    }

    pub fn get_high_credibility_results<'a>(
        &self,
        results: &'a [SearchResult],
        threshold: f32,
    ) -> Vec<&'a SearchResult> {
        results
            .iter()
            .filter(|r| r.credibility_score.map(|s| s >= threshold).unwrap_or(false))
            .collect()
    }
}

pub struct SearchOrchestratorBuilder {
    orchestrator: SearchOrchestrator,
}

impl SearchOrchestratorBuilder {
    pub fn new() -> Self {
        Self {
            orchestrator: SearchOrchestrator::new(),
        }
    }

    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.orchestrator.max_concurrent = max;
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.orchestrator.timeout_secs = secs;
        self
    }

    pub fn deduplication(mut self, enabled: bool) -> Self {
        self.orchestrator.use_deduplication = enabled;
        self
    }

    pub fn build(self) -> SearchOrchestrator {
        self.orchestrator
    }
}

impl Default for SearchOrchestratorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
