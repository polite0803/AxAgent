use crate::proactive_assistant::{ContextPrediction, PredictedIntent};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchResult {
    pub prefetch_type: PrefetchType,
    pub resource_id: String,
    pub data: Option<String>,
    pub ready: bool,
    pub estimated_prepare_time_ms: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrefetchType {
    CodeCompletion,
    SearchResults,
    Documentation,
    ContextAnalysis,
    ToolCache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchResults {
    pub results: Vec<PrefetchResult>,
    pub total_estimated_time_ms: u32,
    pub critical_path: Vec<String>,
}

impl PrefetchResults {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            total_estimated_time_ms: 0,
            critical_path: Vec::new(),
        }
    }

    pub fn add(&mut self, result: PrefetchResult) {
        self.total_estimated_time_ms += result.estimated_prepare_time_ms;
        self.results.push(result);
    }

    pub fn get_ready_results(&self) -> Vec<&PrefetchResult> {
        self.results.iter().filter(|r| r.ready).collect()
    }

    pub fn is_ready(&self) -> bool {
        self.results.iter().all(|r| r.ready)
    }
}

impl Default for PrefetchResults {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TaskPrefetcher {
    config: PrefetcherConfig,
    cache: HashMap<String, PrefetchResult>,
    #[allow(dead_code)]
    pending_prefetches: HashMap<String, PrefetchTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetcherConfig {
    pub enabled: bool,
    pub max_cache_size: usize,
    pub cache_ttl_seconds: i64,
    pub parallel_prefetch: bool,
    pub prioritize_critical_path: bool,
}

impl Default for PrefetcherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_cache_size: 100,
            cache_ttl_seconds: 300,
            parallel_prefetch: true,
            prioritize_critical_path: true,
        }
    }
}

impl Default for TaskPrefetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskPrefetcher {
    pub fn new() -> Self {
        Self {
            config: PrefetcherConfig::default(),
            cache: HashMap::new(),
            pending_prefetches: HashMap::new(),
        }
    }

    pub fn with_config(config: PrefetcherConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
            pending_prefetches: HashMap::new(),
        }
    }

    pub fn get_config(&self) -> &PrefetcherConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: PrefetcherConfig) {
        self.config = config;
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    pub fn prefetch(&self, predictions: &[ContextPrediction]) -> PrefetchResults {
        let mut results = PrefetchResults::new();

        for prediction in predictions {
            let prefetch_result = self.prefetch_for_prediction(prediction);
            if let Some(result) = prefetch_result {
                results.add(result);
            }
        }

        results
    }

    fn prefetch_for_prediction(&self, prediction: &ContextPrediction) -> Option<PrefetchResult> {
        match &prediction.predicted_intent {
            PredictedIntent::CodeCompletion { language, context } => {
                self.prefetch_code_context(language, context)
            },
            PredictedIntent::Search { query_type } => self.prefetch_search_results(query_type),
            PredictedIntent::Documentation { topic } => self.prefetch_documentation(topic),
            PredictedIntent::Refactoring { target } => self.prefetch_refactor_context(target),
            PredictedIntent::TestGeneration { target } => self.prefetch_test_context(target),
            PredictedIntent::Debug { error } => self.prefetch_debug_context(error),
            PredictedIntent::Unknown => None,
        }
    }

    fn prefetch_code_context(&self, language: &str, context: &str) -> Option<PrefetchResult> {
        let cache_key = format!("completion_{}_{}", language, context);

        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached.clone());
        }

        Some(PrefetchResult {
            prefetch_type: PrefetchType::CodeCompletion,
            resource_id: cache_key,
            data: None,
            ready: false,
            estimated_prepare_time_ms: self.estimate_completion_time(language),
            created_at: Utc::now(),
        })
    }

    fn prefetch_search_results(&self, query_type: &str) -> Option<PrefetchResult> {
        let cache_key = format!("search_{}", query_type);

        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached.clone());
        }

        Some(PrefetchResult {
            prefetch_type: PrefetchType::SearchResults,
            resource_id: cache_key,
            data: None,
            ready: false,
            estimated_prepare_time_ms: 200,
            created_at: Utc::now(),
        })
    }

    fn prefetch_documentation(&self, topic: &str) -> Option<PrefetchResult> {
        let cache_key = format!("doc_{}", topic);

        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached.clone());
        }

        Some(PrefetchResult {
            prefetch_type: PrefetchType::Documentation,
            resource_id: cache_key,
            data: None,
            ready: false,
            estimated_prepare_time_ms: 500,
            created_at: Utc::now(),
        })
    }

    fn prefetch_refactor_context(&self, target: &str) -> Option<PrefetchResult> {
        let cache_key = format!("refactor_{}", target);

        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached.clone());
        }

        Some(PrefetchResult {
            prefetch_type: PrefetchType::ContextAnalysis,
            resource_id: cache_key,
            data: None,
            ready: false,
            estimated_prepare_time_ms: 800,
            created_at: Utc::now(),
        })
    }

    fn prefetch_test_context(&self, target: &str) -> Option<PrefetchResult> {
        let cache_key = format!("test_{}", target);

        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached.clone());
        }

        Some(PrefetchResult {
            prefetch_type: PrefetchType::ContextAnalysis,
            resource_id: cache_key,
            data: None,
            ready: false,
            estimated_prepare_time_ms: 600,
            created_at: Utc::now(),
        })
    }

    fn prefetch_debug_context(&self, error: &str) -> Option<PrefetchResult> {
        let cache_key = format!("debug_{}", error);

        if let Some(cached) = self.cache.get(&cache_key) {
            return Some(cached.clone());
        }

        Some(PrefetchResult {
            prefetch_type: PrefetchType::ContextAnalysis,
            resource_id: cache_key,
            data: None,
            ready: false,
            estimated_prepare_time_ms: 300,
            created_at: Utc::now(),
        })
    }

    fn estimate_completion_time(&self, language: &str) -> u32 {
        match language.to_lowercase().as_str() {
            "typescript" | "javascript" => 150,
            "python" => 200,
            "rust" => 300,
            "go" => 250,
            "java" => 280,
            _ => 200,
        }
    }

    pub fn cache_result(&mut self, result: PrefetchResult) {
        if self.cache.len() >= self.config.max_cache_size {
            self.evict_oldest();
        }
        self.cache.insert(result.resource_id.clone(), result);
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .cache
            .iter()
            .min_by_key(|(_, v)| v.created_at)
            .map(|(k, _)| k.clone())
        {
            self.cache.remove(&oldest_key);
        }
    }

    pub fn get_cached(&self, resource_id: &str) -> Option<&PrefetchResult> {
        self.cache.get(resource_id)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();
        let ttl = chrono::Duration::seconds(self.config.cache_ttl_seconds);
        self.cache.retain(|_, v| now - v.created_at < ttl);
    }

    pub fn mark_ready(&mut self, resource_id: &str) -> Option<&PrefetchResult> {
        if let Some(result) = self.cache.get_mut(resource_id) {
            result.ready = true;
        }
        self.cache.get(resource_id)
    }

    pub fn update_data(&mut self, resource_id: &str, data: String) -> Option<&PrefetchResult> {
        if let Some(result) = self.cache.get_mut(resource_id) {
            result.data = Some(data);
            result.ready = true;
        }
        self.cache.get(resource_id)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PrefetchTask {
    resource_id: String,
    prefetch_type: PrefetchType,
    started_at: DateTime<Utc>,
    priority: u32,
}
