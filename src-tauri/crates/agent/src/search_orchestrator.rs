use crate::research_state::{SearchPlan, SearchQuery, SearchResult, SourceType};
use crate::search_provider::SearchProvider;
use std::collections::HashMap;
use std::sync::Arc;
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
    providers: HashMap<SourceType, Arc<dyn SearchProvider>>,
}

impl Default for SearchOrchestrator {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            timeout_secs: 30,
            use_deduplication: true,
            providers: HashMap::new(),
        }
    }
}

impl SearchOrchestrator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_provider(mut self, provider: Arc<dyn SearchProvider>) -> Self {
        self.providers.insert(provider.source_type(), provider);
        self
    }

    pub fn with_web_search_provider(mut self, provider: Arc<dyn SearchProvider>) -> Self {
        self.providers.insert(SourceType::Web, provider);
        self
    }

    pub fn with_academic_search_provider(mut self, provider: Arc<dyn SearchProvider>) -> Self {
        self.providers.insert(SourceType::Academic, provider);
        self
    }

    pub fn add_provider(&mut self, provider: Arc<dyn SearchProvider>) -> &mut Self {
        self.providers.insert(provider.source_type(), provider);
        self
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
        let providers = self.providers.clone();

        for query_id in query_ids {
            if let Some(query) = plan.queries.iter().find(|q| &q.id == query_id) {
                let query_clone = query.clone();
                let query_id_clone = query_id.clone();
                let providers_clone = providers.clone();

                let handle = tokio::spawn(async move {
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(timeout),
                        Self::execute_single_query_static(&query_clone, &providers_clone),
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
                },
                Ok(Err(e)) => {
                    tracing::warn!("Query failed: {}", e);
                },
                Err(e) => {
                    tracing::warn!("Task join error: {}", e);
                },
            }
        }

        Ok(results)
    }

    async fn execute_single_query_static(
        query: &SearchQuery,
        providers: &HashMap<SourceType, Arc<dyn SearchProvider>>,
    ) -> Result<Vec<SearchResult>, OrchestratorError> {
        let mut results: Vec<SearchResult> = Vec::new();

        for source_type in &query.source_types {
            let source_results = Self::search_source_static(query, *source_type, providers).await?;
            results.extend(source_results);
        }

        results.truncate(query.max_results);
        Ok(results)
    }

    async fn search_source_static(
        query: &SearchQuery,
        source_type: SourceType,
        providers: &HashMap<SourceType, Arc<dyn SearchProvider>>,
    ) -> Result<Vec<SearchResult>, OrchestratorError> {
        let provider = providers
            .get(&source_type)
            .ok_or(OrchestratorError::NoProviderForSource(source_type))?;

        provider
            .search(query)
            .await
            .map_err(|e| OrchestratorError::ProviderError(e.to_string()))
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

    pub fn add_provider(mut self, provider: Arc<dyn SearchProvider>) -> Self {
        self.orchestrator
            .providers
            .insert(provider.source_type(), provider);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research_state::SearchQuery;
    use crate::search_provider::{ExtractError, ExtractedContent, SearchError, SearchProvider};
    use async_trait::async_trait;

    struct MockSearchProvider {
        source_type: SourceType,
        results: Vec<SearchResult>,
        should_fail: bool,
    }

    impl MockSearchProvider {
        fn new(source_type: SourceType, results: Vec<SearchResult>) -> Self {
            Self {
                source_type,
                results,
                should_fail: false,
            }
        }

        fn failing(source_type: SourceType) -> Self {
            Self {
                source_type,
                results: vec![],
                should_fail: true,
            }
        }
    }

    #[async_trait]
    impl SearchProvider for MockSearchProvider {
        async fn search(&self, _query: &SearchQuery) -> Result<Vec<SearchResult>, SearchError> {
            if self.should_fail {
                Err(SearchError::ApiError("mock error".to_string()))
            } else {
                Ok(self.results.clone())
            }
        }

        async fn extract_content(&self, _url: &str) -> Result<ExtractedContent, ExtractError> {
            Ok(ExtractedContent::new(
                "https://example.com".to_string(),
                "Test".to_string(),
                "Content".to_string(),
            ))
        }

        fn source_type(&self) -> SourceType {
            self.source_type
        }

        fn display_name(&self) -> &str {
            "Mock Provider"
        }

        fn rate_limit(&self) -> Option<std::time::Duration> {
            None
        }
    }

    fn make_result(
        url: &str,
        title: &str,
        source_type: SourceType,
        relevance: f32,
    ) -> SearchResult {
        SearchResult::new(
            source_type,
            url.to_string(),
            title.to_string(),
            format!("Snippet for {}", title),
        )
        .with_relevance(relevance)
    }

    #[test]
    fn test_orchestrator_default() {
        let orch = SearchOrchestrator::new();
        assert_eq!(orch.max_concurrent, 5);
        assert_eq!(orch.timeout_secs, 30);
        assert!(orch.use_deduplication);
        assert!(orch.providers.is_empty());
    }

    #[test]
    fn test_orchestrator_builder_default() {
        let orch = SearchOrchestratorBuilder::new().build();
        assert_eq!(orch.max_concurrent, 5);
        assert_eq!(orch.timeout_secs, 30);
    }

    #[test]
    fn test_orchestrator_with_max_concurrent() {
        let orch = SearchOrchestrator::new().with_max_concurrent(10);
        assert_eq!(orch.max_concurrent, 10);
    }

    #[test]
    fn test_orchestrator_with_timeout() {
        let orch = SearchOrchestrator::new().with_timeout(60);
        assert_eq!(orch.timeout_secs, 60);
    }

    #[test]
    fn test_orchestrator_with_deduplication() {
        let orch = SearchOrchestrator::new().with_deduplication(false);
        assert!(!orch.use_deduplication);
    }

    #[test]
    fn test_orchestrator_builder_max_concurrent() {
        let orch = SearchOrchestratorBuilder::new().max_concurrent(20).build();
        assert_eq!(orch.max_concurrent, 20);
    }

    #[test]
    fn test_orchestrator_builder_timeout() {
        let orch = SearchOrchestratorBuilder::new().timeout(120).build();
        assert_eq!(orch.timeout_secs, 120);
    }

    #[test]
    fn test_orchestrator_builder_deduplication() {
        let orch = SearchOrchestratorBuilder::new()
            .deduplication(false)
            .build();
        assert!(!orch.use_deduplication);
    }

    #[test]
    fn test_orchestrator_with_provider() {
        let provider = Arc::new(MockSearchProvider::new(SourceType::Web, vec![]));
        let orch = SearchOrchestrator::new().with_provider(provider);
        assert!(orch.providers.contains_key(&SourceType::Web));
    }

    #[test]
    fn test_orchestrator_with_web_search_provider() {
        let provider = Arc::new(MockSearchProvider::new(SourceType::Web, vec![]));
        let orch = SearchOrchestrator::new().with_web_search_provider(provider);
        assert!(orch.providers.contains_key(&SourceType::Web));
    }

    #[test]
    fn test_orchestrator_with_academic_search_provider() {
        let provider = Arc::new(MockSearchProvider::new(SourceType::Academic, vec![]));
        let orch = SearchOrchestrator::new().with_academic_search_provider(provider);
        assert!(orch.providers.contains_key(&SourceType::Academic));
    }

    #[test]
    fn test_orchestrator_add_provider() {
        let provider = Arc::new(MockSearchProvider::new(SourceType::Web, vec![]));
        let mut orch = SearchOrchestrator::new();
        orch.add_provider(provider);
        assert!(orch.providers.contains_key(&SourceType::Web));
    }

    #[test]
    fn test_orchestrator_builder_add_provider() {
        let provider = Arc::new(MockSearchProvider::new(SourceType::Web, vec![]));
        let orch = SearchOrchestratorBuilder::new()
            .add_provider(provider)
            .build();
        assert!(orch.providers.contains_key(&SourceType::Web));
    }

    #[test]
    fn test_deduplicate_results_removes_duplicates() {
        let orch = SearchOrchestrator::new();
        let results = vec![
            make_result("https://a.com", "A", SourceType::Web, 0.5),
            make_result("https://a.com", "A dup", SourceType::Web, 0.3),
            make_result("https://b.com", "B", SourceType::Web, 0.4),
        ];
        let deduped = orch.deduplicate_results(results);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].url, "https://a.com");
        assert_eq!(deduped[1].url, "https://b.com");
    }

    #[test]
    fn test_deduplicate_results_preserves_order() {
        let orch = SearchOrchestrator::new();
        let results = vec![
            make_result("https://x.com", "X", SourceType::Web, 0.9),
            make_result("https://y.com", "Y", SourceType::Web, 0.8),
            make_result("https://z.com", "Z", SourceType::Web, 0.7),
        ];
        let deduped = orch.deduplicate_results(results);
        assert_eq!(deduped.len(), 3);
        assert_eq!(deduped[0].url, "https://x.com");
        assert_eq!(deduped[1].url, "https://y.com");
        assert_eq!(deduped[2].url, "https://z.com");
    }

    #[test]
    fn test_deduplicate_results_empty() {
        let orch = SearchOrchestrator::new();
        let deduped = orch.deduplicate_results(vec![]);
        assert!(deduped.is_empty());
    }

    #[test]
    fn test_calculate_source_distribution() {
        let results = vec![
            make_result("https://a.com", "A", SourceType::Web, 0.5),
            make_result("https://b.com", "B", SourceType::Web, 0.4),
            make_result("https://c.com", "C", SourceType::Academic, 0.9),
        ];
        let dist = SearchOrchestrator::calculate_source_distribution(&results);
        assert_eq!(*dist.get(&SourceType::Web).unwrap(), 2);
        assert_eq!(*dist.get(&SourceType::Academic).unwrap(), 1);
        assert!(!dist.contains_key(&SourceType::GitHub));
    }

    #[test]
    fn test_calculate_source_distribution_empty() {
        let dist = SearchOrchestrator::calculate_source_distribution(&[]);
        assert!(dist.is_empty());
    }

    #[test]
    fn test_get_high_credibility_results() {
        let orch = SearchOrchestrator::new();
        let results = vec![
            SearchResult::new(
                SourceType::Web,
                "https://a.com".to_string(),
                "A".to_string(),
                "s".to_string(),
            )
            .with_credibility(0.9),
            SearchResult::new(
                SourceType::Web,
                "https://b.com".to_string(),
                "B".to_string(),
                "s".to_string(),
            )
            .with_credibility(0.5),
            SearchResult::new(
                SourceType::Web,
                "https://c.com".to_string(),
                "C".to_string(),
                "s".to_string(),
            ),
        ];
        let high = orch.get_high_credibility_results(&results, 0.8);
        assert_eq!(high.len(), 1);
        assert_eq!(high[0].title, "A");
    }

    #[test]
    fn test_get_high_credibility_results_none_match() {
        let orch = SearchOrchestrator::new();
        let results = vec![SearchResult::new(
            SourceType::Web,
            "https://a.com".to_string(),
            "A".to_string(),
            "s".to_string(),
        )
        .with_credibility(0.3)];
        let high = orch.get_high_credibility_results(&results, 0.8);
        assert!(high.is_empty());
    }

    #[tokio::test]
    async fn test_execute_with_provider() {
        let results = vec![
            make_result("https://a.com", "Result A", SourceType::Web, 0.9),
            make_result("https://b.com", "Result B", SourceType::Web, 0.7),
        ];
        let provider = Arc::new(MockSearchProvider::new(SourceType::Web, results));
        let orch = SearchOrchestrator::new()
            .with_provider(provider)
            .with_deduplication(true);

        let query = SearchQuery::new("test query".to_string())
            .with_sources(vec![SourceType::Web])
            .with_max_results(10);
        let plan = SearchPlan::new(vec![query]);

        let result = orch.execute(&plan).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].relevance_score >= result[1].relevance_score);
    }

    #[tokio::test]
    async fn test_execute_no_provider_returns_empty() {
        let orch = SearchOrchestrator::new();
        let query = SearchQuery::new("test query".to_string())
            .with_sources(vec![SourceType::Web])
            .with_max_results(10);
        let plan = SearchPlan::new(vec![query]);

        let result = orch.execute(&plan).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_execute_deduplication_enabled() {
        let results = vec![
            make_result("https://same.com", "First", SourceType::Web, 0.9),
            make_result("https://same.com", "Second", SourceType::Web, 0.8),
        ];
        let provider = Arc::new(MockSearchProvider::new(SourceType::Web, results));
        let orch = SearchOrchestrator::new()
            .with_provider(provider)
            .with_deduplication(true);

        let query = SearchQuery::new("test".to_string())
            .with_sources(vec![SourceType::Web])
            .with_max_results(10);
        let plan = SearchPlan::new(vec![query]);

        let result = orch.execute(&plan).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "First");
    }

    #[tokio::test]
    async fn test_execute_deduplication_disabled() {
        let results = vec![
            make_result("https://same.com", "First", SourceType::Web, 0.9),
            make_result("https://same.com", "Second", SourceType::Web, 0.8),
        ];
        let provider = Arc::new(MockSearchProvider::new(SourceType::Web, results));
        let orch = SearchOrchestrator::new()
            .with_provider(provider)
            .with_deduplication(false);

        let query = SearchQuery::new("test".to_string())
            .with_sources(vec![SourceType::Web])
            .with_max_results(10);
        let plan = SearchPlan::new(vec![query]);

        let result = orch.execute(&plan).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_execute_results_sorted_by_relevance() {
        let results = vec![
            make_result("https://low.com", "Low", SourceType::Web, 0.3),
            make_result("https://high.com", "High", SourceType::Web, 0.9),
            make_result("https://mid.com", "Mid", SourceType::Web, 0.6),
        ];
        let provider = Arc::new(MockSearchProvider::new(SourceType::Web, results));
        let orch = SearchOrchestrator::new().with_provider(provider);

        let query = SearchQuery::new("test".to_string())
            .with_sources(vec![SourceType::Web])
            .with_max_results(10);
        let plan = SearchPlan::new(vec![query]);

        let result = orch.execute(&plan).await.unwrap();
        assert_eq!(result[0].title, "High");
        assert_eq!(result[1].title, "Mid");
        assert_eq!(result[2].title, "Low");
    }

    #[tokio::test]
    async fn test_execute_multiple_source_types() {
        let web_results = vec![make_result("https://web.com", "Web", SourceType::Web, 0.8)];
        let academic_results = vec![make_result(
            "https://acad.com",
            "Academic",
            SourceType::Academic,
            0.9,
        )];
        let web_provider = Arc::new(MockSearchProvider::new(SourceType::Web, web_results));
        let academic_provider =
            Arc::new(MockSearchProvider::new(SourceType::Academic, academic_results));
        let orch = SearchOrchestrator::new()
            .with_provider(web_provider)
            .with_provider(academic_provider);

        let query = SearchQuery::new("test".to_string())
            .with_sources(vec![SourceType::Web, SourceType::Academic])
            .with_max_results(10);
        let plan = SearchPlan::new(vec![query]);

        let result = orch.execute(&plan).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_execute_max_results_truncation() {
        let results = vec![
            make_result("https://a.com", "A", SourceType::Web, 0.9),
            make_result("https://b.com", "B", SourceType::Web, 0.8),
            make_result("https://c.com", "C", SourceType::Web, 0.7),
        ];
        let provider = Arc::new(MockSearchProvider::new(SourceType::Web, results));
        let orch = SearchOrchestrator::new().with_provider(provider);

        let query = SearchQuery::new("test".to_string())
            .with_sources(vec![SourceType::Web])
            .with_max_results(2);
        let plan = SearchPlan::new(vec![query]);

        let result = orch.execute(&plan).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_execute_empty_plan() {
        let orch = SearchOrchestrator::new();
        let plan = SearchPlan::new(vec![]);
        let result = orch.execute(&plan).await.unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_orchestrator_error_display() {
        let err = OrchestratorError::ProviderError("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = OrchestratorError::NoProviderForSource(SourceType::Web);
        assert!(err.to_string().contains("Web"));

        let err = OrchestratorError::QueryFailed("q1".to_string());
        assert!(err.to_string().contains("q1"));

        let err = OrchestratorError::DeduplicationFailed("dup".to_string());
        assert!(err.to_string().contains("dup"));

        let err = OrchestratorError::Timeout("t1".to_string());
        assert!(err.to_string().contains("t1"));
    }

    #[tokio::test]
    async fn test_execute_with_failing_provider() {
        let provider = Arc::new(MockSearchProvider::failing(SourceType::Web));
        let orch = SearchOrchestrator::new().with_provider(provider);

        let query = SearchQuery::new("test query".to_string())
            .with_sources(vec![SourceType::Web])
            .with_max_results(10);
        let plan = SearchPlan::new(vec![query]);

        let result = orch.execute(&plan).await;
        assert!(result.is_err() || result.unwrap().is_empty());
    }
}
