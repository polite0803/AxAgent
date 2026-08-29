// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::memory_providers::service::{MemoryNature, MemoryTier};

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
    pub tier: MemoryTier,
    pub nature: MemoryNature,
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
    pub tier_filter: Option<MemoryTier>,
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
