// SPDX-License-Identifier: AGPL-3.0-only

use crate::memory_provider::{MemoryEntry, MemoryProvider, MemoryQuery, MemoryQueryResult};
use crate::memory_providers::service::{MemoryNature, MemoryTier};
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

        if self.config.api_key.is_some()
            && let Err(e) = self.sync_to_remote(session_id, &entries).await
        {
            tracing::warn!("Honcho remote sync failed, falling back to local cache: {}", e);
        }

        let cache_key = format!("{}:{}", self.config.user_id, session_id);
        self.local_cache
            .write()
            .await
            .insert(cache_key.clone(), entries);
        tracing::debug!("Synced memory entries for session {} via Honcho", session_id);
        Ok(())
    }

    async fn prefetch(
        &self,
        session_id: &str,
        query: &MemoryQuery,
    ) -> Result<MemoryQueryResult, String> {
        if self.config.api_key.is_some() {
            match self.search_remote(session_id, query).await {
                Ok(result) if !result.entries.is_empty() => return Ok(result),
                Ok(_) => {},
                Err(e) => {
                    tracing::warn!(
                        "Honcho remote search failed, falling back to local cache: {}",
                        e
                    );
                },
            }
        }

        let cache_key = format!("{}:{}", self.config.user_id, session_id);
        let cached = self
            .local_cache
            .read()
            .await
            .get(&cache_key)
            .cloned()
            .unwrap_or_default();
        let filtered: Vec<MemoryEntry> = cached
            .into_iter()
            .filter(|e| {
                if let Some(types) = &query.memory_types
                    && !types.contains(&e.memory_type)
                {
                    return false;
                }
                if let Some(tags) = &query.tags
                    && !tags.iter().any(|t| e.tags.contains(t))
                {
                    return false;
                }
                if let Some(min_imp) = query.min_importance
                    && e.importance < min_imp
                {
                    return false;
                }
                if let Some(tier) = &query.tier_filter
                    && e.tier != *tier
                {
                    return false;
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
        "2.0.0"
    }
}

impl HonchoProvider {
    async fn sync_to_remote(
        &self,
        _session_id: &str,
        entries: &[MemoryEntry],
    ) -> Result<(), String> {
        let api_key = self.config.api_key.as_ref().ok_or("No API key")?;
        let url = format!(
            "{}/apps/{}/users/{}/memories",
            self.config.api_url, self.config.app_id, self.config.user_id
        );

        let client = reqwest::Client::new();
        for entry in entries {
            let body = serde_json::json!({
                "content": entry.content,
                "metadata": {
                    "memory_type": entry.memory_type.as_str(),
                    "tier": entry.tier.as_str(),
                    "nature": entry.nature.as_str(),
                    "importance": entry.importance,
                    "tags": entry.tags,
                }
            });

            let resp = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Honcho API request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::warn!("Honcho API returned {}: {}", status, body);
            }
        }

        Ok(())
    }

    async fn search_remote(
        &self,
        _session_id: &str,
        query: &MemoryQuery,
    ) -> Result<MemoryQueryResult, String> {
        let api_key = self.config.api_key.as_ref().ok_or("No API key")?;
        let url = format!(
            "{}/apps/{}/users/{}/memories/search",
            self.config.api_url, self.config.app_id, self.config.user_id
        );

        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "query": query.query,
            "limit": query.limit,
        });

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Honcho search API request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Honcho search API returned {}", resp.status()));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Honcho response: {}", e))?;

        let memories = json.as_array().cloned().unwrap_or_default();

        let entries: Vec<MemoryEntry> = memories
            .iter()
            .filter_map(|m| {
                let content = m.get("content")?.as_str()?.to_string();
                let id = m
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let metadata = m.get("metadata").cloned();
                let memory_type_str = metadata
                    .as_ref()
                    .and_then(|m| m.get("memory_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("fact");
                let tier_str = metadata
                    .as_ref()
                    .and_then(|m| m.get("tier"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("working");
                let nature_str = metadata
                    .as_ref()
                    .and_then(|m| m.get("nature"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("semantic");
                let importance = metadata
                    .as_ref()
                    .and_then(|m| m.get("importance"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5);
                let tags: Vec<String> = metadata
                    .as_ref()
                    .and_then(|m| m.get("tags"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                Some(MemoryEntry {
                    id,
                    content,
                    memory_type: match memory_type_str {
                        "conversation" => crate::memory_provider::MemoryType::Conversation,
                        "preference" => crate::memory_provider::MemoryType::Preference,
                        "skill" => crate::memory_provider::MemoryType::Skill,
                        "project" => crate::memory_provider::MemoryType::Project,
                        "user" => crate::memory_provider::MemoryType::User,
                        _ => crate::memory_provider::MemoryType::Fact,
                    },
                    importance,
                    tags,
                    created_at: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    tier: MemoryTier::from_str(tier_str),
                    nature: MemoryNature::from_str(nature_str),
                })
            })
            .take(query.limit)
            .collect();

        let total = entries.len();
        Ok(MemoryQueryResult {
            entries,
            scores: vec![1.0; total],
            total,
        })
    }
}
