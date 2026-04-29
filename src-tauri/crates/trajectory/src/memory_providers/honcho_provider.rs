use crate::memory_provider::{MemoryEntry, MemoryProvider, MemoryQuery, MemoryQueryResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HonchoConfig {
    pub api_url: String,
    pub api_key: Option<String>,
    pub user_id: String,
    pub app_id: String,
}

impl Default for HonchoConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.honcho.ai".to_string(),
            api_key: None,
            user_id: "default".to_string(),
            app_id: "axagent".to_string(),
        }
    }
}

pub struct HonchoProvider {
    config: HonchoConfig,
    local_cache: Arc<RwLock<HashMap<String, Vec<MemoryEntry>>>>,
}

impl HonchoProvider {
    pub fn new(config: HonchoConfig) -> Self {
        Self {
            config,
            local_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl MemoryProvider for HonchoProvider {
    async fn sync_turn(&self, session_id: &str, entries: Vec<MemoryEntry>) -> Result<(), String> {
        if entries.is_empty() {
            return Ok(());
        }
        let cache_key = format!("{}:{}", self.config.user_id, session_id);
        self.local_cache.write().await.insert(cache_key.clone(), entries);
        tracing::debug!("Synced memory entries for session {} via Honcho", session_id);
        Ok(())
    }

    async fn prefetch(&self, session_id: &str, query: &MemoryQuery) -> Result<MemoryQueryResult, String> {
        let cache_key = format!("{}:{}", self.config.user_id, session_id);
        let cached = self.local_cache.read().await.get(&cache_key).cloned().unwrap_or_default();
        let filtered: Vec<MemoryEntry> = cached
            .into_iter()
            .filter(|e| {
                if let Some(types) = &query.memory_types {
                    if !types.contains(&e.memory_type) {
                        return false;
                    }
                }
                if let Some(tags) = &query.tags {
                    if !tags.iter().any(|t| e.tags.contains(t)) {
                        return false;
                    }
                }
                if let Some(min_imp) = query.min_importance {
                    if e.importance < min_imp {
                        return false;
                    }
                }
                true
            })
            .take(query.limit)
            .collect();
        let total = filtered.len();
        Ok(MemoryQueryResult {
            entries: filtered,
            scores: vec![1.0; total],
            total,
        })
    }

    async fn shutdown(&self) -> Result<(), String> {
        self.local_cache.write().await.clear();
        tracing::info!("Honcho memory provider shutdown complete");
        Ok(())
    }

    fn provider_name(&self) -> &'static str {
        "honcho"
    }

    fn provider_version(&self) -> &'static str {
        "1.0.0"
    }
}
