// SPDX-License-Identifier: AGPL-3.0-only

use crate::proactive_assistant::{ContextPrediction, PredictedIntent};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchResult {
    pub prefetch_type: PrefetchType,
    pub resource_id: String,
    pub data: Option<String>,
    pub ready: bool,
    pub estimated_prepare_time_ms: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PrefetchType {
    CodeCompletion,
    SearchResults,
    Documentation,
    ContextAnalysis,
    ToolCache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchResults {
    pub results: Vec<PrefetchResult>,
    pub total_estimated_time_ms: u32,
    pub critical_path: Vec<String>,
}

impl PrefetchResults {
    pub fn new() -> Self {
        Self { results: Vec::new(), total_estimated_time_ms: 0, critical_path: Vec::new() }
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
    /// 因果层观测到的真实延迟提示，key 为实体 ID（如 `intent:search`）
    delay_hints: HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefetcherConfig {
    pub enabled: bool,
    pub max_cache_size: usize,
    pub cache_ttl_seconds: i64,
    pub parallel_prefetch: bool,
    pub prioritize_critical_path: bool,
    /// 是否用因果层观测到的真实延迟覆盖硬编码准备耗时估算。
    /// 需配合 [`TaskPrefetcher::set_causal_delay_hints`] 注入提示表。
    #[serde(default)]
    pub use_causal_hints: bool,
}

impl Default for PrefetcherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_cache_size: 100,
            cache_ttl_seconds: 300,
            parallel_prefetch: true,
            prioritize_critical_path: true,
            use_causal_hints: false,
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
            delay_hints: HashMap::new(),
        }
    }

    pub fn with_config(config: PrefetcherConfig) -> Self {
        Self { config, cache: HashMap::new(), delay_hints: HashMap::new() }
    }

    /// 注入因果层观测到的真实延迟提示。
    ///
    /// key 为实体 ID（见 [`crate::causal::build_delay_hints`] 的返回），
    /// value 为毫秒。命中时覆盖硬编码的准备耗时估算。
    pub fn set_causal_delay_hints(&mut self, hints: HashMap<String, i64>) {
        self.delay_hints = hints;
    }

    /// 清空延迟提示，回退到硬编码估算
    pub fn clear_causal_delay_hints(&mut self) {
        self.delay_hints.clear();
    }

    /// 当前生效的延迟提示条数
    pub fn causal_delay_hint_count(&self) -> usize {
        self.delay_hints.len()
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
        let mut result = match &prediction.predicted_intent {
            PredictedIntent::CodeCompletion { language, context } => {
                self.prefetch_code_context(language, context)
            },
            PredictedIntent::Search { query_type } => self.prefetch_search_results(query_type),
            PredictedIntent::Documentation { topic } => self.prefetch_documentation(topic),
            PredictedIntent::Refactoring { target } => self.prefetch_refactor_context(target),
            PredictedIntent::TestGeneration { target } => self.prefetch_test_context(target),
            PredictedIntent::Debug { error } => self.prefetch_debug_context(error),
            PredictedIntent::Unknown => None,
        }?;

        // 因果层观测到的真实延迟优先于硬编码估算
        if self.config.use_causal_hints {
            let key = crate::causal::prediction_intent_entity(&prediction.predicted_intent);
            if let Some(&observed) = self.delay_hints.get(&key) {
                result.estimated_prepare_time_ms =
                    u32::try_from(observed.max(0)).unwrap_or(u32::MAX);
            }
        }

        Some(result)
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
        if let Some(oldest_key) =
            self.cache.iter().min_by_key(|(_, v)| v.created_at).map(|(k, _)| k.clone())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proactive_assistant::{ContextWindow, SuggestedAction};
    use chrono::Utc;

    fn prediction(intent: PredictedIntent) -> ContextPrediction {
        ContextPrediction {
            predicted_intent: intent,
            confidence: 0.9,
            reasoning: String::new(),
            suggested_actions: vec![SuggestedAction {
                action_type: "prefetch".to_string(),
                title: "t".to_string(),
                description: "d".to_string(),
                priority: crate::proactive_assistant::Priority::Medium,
            }],
            context_window: ContextWindow {
                files: Vec::new(),
                recent_actions: Vec::new(),
                current_language: None,
                project_type: None,
            },
            created_at: Utc::now(),
        }
    }

    fn search_prediction() -> ContextPrediction {
        prediction(PredictedIntent::Search { query_type: "symbol".to_string() })
    }

    #[test]
    fn hint_key_maps_every_intent_variant() {
        assert_eq!(
            crate::causal::prediction_intent_entity(&PredictedIntent::CodeCompletion {
                language: "rust".to_string(),
                context: String::new(),
            }),
            "intent:code_completion"
        );
        assert_eq!(
            crate::causal::prediction_intent_entity(&PredictedIntent::Search {
                query_type: "x".to_string()
            }),
            "intent:search"
        );
        assert_eq!(crate::causal::prediction_intent_entity(&PredictedIntent::Unknown), "");
    }

    #[test]
    fn delay_hint_overrides_hardcoded_estimate() {
        let mut p = TaskPrefetcher::new();
        let mut config = p.get_config().clone();
        config.use_causal_hints = true;
        p.update_config(config);

        let mut hints = HashMap::new();
        hints.insert("intent:search".to_string(), 1_800_i64);
        p.set_causal_delay_hints(hints);
        assert_eq!(p.causal_delay_hint_count(), 1);

        let results = p.prefetch(&[search_prediction()]);
        assert_eq!(results.results.len(), 1);
        // 硬编码值为 200，注入提示后应采用观测到的 1800
        assert_eq!(results.results[0].estimated_prepare_time_ms, 1_800);
    }

    #[test]
    fn delay_hint_ignored_when_config_disabled() {
        let mut p = TaskPrefetcher::new();
        assert!(!p.get_config().use_causal_hints, "默认必须关闭");

        let mut hints = HashMap::new();
        hints.insert("intent:search".to_string(), 1_800_i64);
        p.set_causal_delay_hints(hints);

        let results = p.prefetch(&[search_prediction()]);
        assert_eq!(results.results[0].estimated_prepare_time_ms, 200, "关闭时回退到硬编码值");
    }

    #[test]
    fn delay_hint_ignored_on_unknown_intent() {
        let mut p = TaskPrefetcher::new();
        let mut config = p.get_config().clone();
        config.use_causal_hints = true;
        p.update_config(config);
        p.set_causal_delay_hints(HashMap::from([("intent:search".to_string(), 1_800_i64)]));

        // Unknown 不产生任何预取结果，且空 key 不会误命中
        let results = p.prefetch(&[prediction(PredictedIntent::Unknown)]);
        assert!(results.results.is_empty());
    }

    #[test]
    fn negative_delay_clamps_to_zero() {
        let mut p = TaskPrefetcher::new();
        let mut config = p.get_config().clone();
        config.use_causal_hints = true;
        p.update_config(config);
        p.set_causal_delay_hints(HashMap::from([("intent:search".to_string(), -500_i64)]));

        let results = p.prefetch(&[search_prediction()]);
        assert_eq!(results.results[0].estimated_prepare_time_ms, 0);
    }

    #[test]
    fn clear_hints_restores_hardcoded_estimate() {
        let mut p = TaskPrefetcher::new();
        let mut config = p.get_config().clone();
        config.use_causal_hints = true;
        p.update_config(config);
        p.set_causal_delay_hints(HashMap::from([("intent:search".to_string(), 1_800_i64)]));

        p.clear_causal_delay_hints();
        assert_eq!(p.causal_delay_hint_count(), 0);

        let results = p.prefetch(&[search_prediction()]);
        assert_eq!(results.results[0].estimated_prepare_time_ms, 200);
    }
}
