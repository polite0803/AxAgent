// SPDX-License-Identifier: AGPL-3.0-only
//! Dream Consolidation 契约
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamConsolidationConfig { pub min_interval_secs: u64, pub max_experiences_per_run: usize, pub temperature: f64 }
impl Default for DreamConsolidationConfig { fn default() -> Self { Self { min_interval_secs: 300, max_experiences_per_run: 50, temperature: 0.3 } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceRecord { pub id: String, pub timestamp: i64, pub summary: String, pub tokens_used: u64, pub outcome: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySample { pub experience_id: String, pub replay_count: u32, pub last_replayed: i64 }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledKnowledge { pub topic: String, pub summary: String, pub confidence: f64, pub source_count: usize }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamConsolidationResult { pub experiences_processed: usize, pub knowledge_distilled: Vec<DistilledKnowledge>, pub replays_generated: usize, pub duration_secs: f64 }

#[async_trait]
pub trait ConsolidationDataProvider: Send + Sync {
    async fn fetch_new_experiences(&self, limit: usize) -> Result<Vec<ExperienceRecord>, String>;
    async fn fetch_replay_samples(&self, limit: usize) -> Result<Vec<ReplaySample>, String>;
    async fn store_distilled(&self, knowledge: DistilledKnowledge) -> Result<(), String>;
}

#[async_trait]
pub trait DreamConsolidator: Send + Sync {
    async fn consolidate(&self) -> Result<DreamConsolidationResult, String>;
    async fn should_consolidate(&self) -> Result<bool, String>;
    fn config(&self) -> DreamConsolidationConfig;
}

#[derive(Default)]
pub struct NoopConsolidationDataProvider;
#[async_trait]
impl ConsolidationDataProvider for NoopConsolidationDataProvider {
    async fn fetch_new_experiences(&self, _limit: usize) -> Result<Vec<ExperienceRecord>, String> { Ok(Vec::new()) }
    async fn fetch_replay_samples(&self, _limit: usize) -> Result<Vec<ReplaySample>, String> { Ok(Vec::new()) }
    async fn store_distilled(&self, _knowledge: DistilledKnowledge) -> Result<(), String> { Ok(()) }
}

#[derive(Default)]
pub struct NoopDreamConsolidator;
#[async_trait]
impl DreamConsolidator for NoopDreamConsolidator {
    async fn consolidate(&self) -> Result<DreamConsolidationResult, String> { Err("dream consolidator not configured".to_string()) }
    async fn should_consolidate(&self) -> Result<bool, String> { Ok(false) }
    fn config(&self) -> DreamConsolidationConfig { DreamConsolidationConfig::default() }
}
