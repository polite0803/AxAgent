use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: f64,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    Conversation,
    Fact,
    Preference,
    Skill,
    Project,
    User,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Conversation => "conversation",
            Self::Fact => "fact",
            Self::Preference => "preference",
            Self::Skill => "skill",
            Self::Project => "project",
            Self::User => "user",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub query: String,
    pub memory_types: Option<Vec<MemoryType>>,
    pub tags: Option<Vec<String>>,
    pub limit: usize,
    pub min_importance: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResult {
    pub entries: Vec<MemoryEntry>,
    pub scores: Vec<f64>,
    pub total: usize,
}

#[async_trait]
pub trait MemoryProvider: Send + Sync {
    async fn sync_turn(&self, session_id: &str, entries: Vec<MemoryEntry>) -> Result<(), String>;
    async fn prefetch(
        &self,
        session_id: &str,
        query: &MemoryQuery,
    ) -> Result<MemoryQueryResult, String>;
    async fn shutdown(&self) -> Result<(), String>;
    fn provider_name(&self) -> &'static str;
    fn provider_version(&self) -> &'static str;
}

pub struct MemoryProviderRegistry {
    providers: HashMap<String, Box<dyn MemoryProvider>>,
    active_provider: String,
}

impl Default for MemoryProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            active_provider: "internal".to_string(),
        }
    }

    pub fn register(&mut self, name: String, provider: Box<dyn MemoryProvider>) {
        tracing::info!("Registering memory provider: {}", name);
        self.providers.insert(name, provider);
    }

    pub fn set_active(&mut self, name: &str) -> Result<(), String> {
        if !self.providers.contains_key(name) {
            return Err(format!("Provider '{}' not found", name));
        }
        self.active_provider = name.to_string();
        Ok(())
    }

    pub async fn sync_turn(
        &self,
        session_id: &str,
        entries: Vec<MemoryEntry>,
    ) -> Result<(), String> {
        let provider = self
            .providers
            .get(&self.active_provider)
            .ok_or_else(|| format!("Active provider '{}' not found", self.active_provider))?;
        provider.sync_turn(session_id, entries).await
    }

    pub async fn prefetch(
        &self,
        session_id: &str,
        query: &MemoryQuery,
    ) -> Result<MemoryQueryResult, String> {
        let provider = self
            .providers
            .get(&self.active_provider)
            .ok_or_else(|| format!("Active provider '{}' not found", self.active_provider))?;
        provider.prefetch(session_id, query).await
    }

    pub async fn shutdown(&self) -> Result<(), String> {
        for (name, provider) in &self.providers {
            tracing::info!("Shutting down memory provider: {}", name);
            provider.shutdown().await?;
        }
        Ok(())
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub fn active_provider_name(&self) -> &str {
        &self.active_provider
    }
}
