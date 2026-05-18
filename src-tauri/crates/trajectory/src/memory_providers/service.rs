//! Memory service - core working memory and session management
//!
//! Implements a tiered memory architecture with:
//! - P0-1: Memory tiers (ShortTerm, Working, LongTerm, Core)
//! - P0-2: Memory forgetting/decay with effective score computation
//! - P0-3: Memory merge/dedup with content similarity detection

use crate::TrajectoryStorage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ── Memory Tier ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTier {
    ShortTerm,
    Working,
    LongTerm,
    Core,
}

impl MemoryTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ShortTerm => "short_term",
            Self::Working => "working",
            Self::LongTerm => "long_term",
            Self::Core => "core",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "short_term" => Self::ShortTerm,
            "working" => Self::Working,
            "long_term" => Self::LongTerm,
            "core" => Self::Core,
            _ => Self::Working,
        }
    }

    fn prompt_priority(&self) -> u8 {
        match self {
            Self::Core => 4,
            Self::LongTerm => 3,
            Self::Working => 2,
            Self::ShortTerm => 1,
        }
    }

    fn default_capacity(&self) -> usize {
        match self {
            Self::ShortTerm => 20,
            Self::Working => 50,
            Self::LongTerm => 200,
            Self::Core => 30,
        }
    }

    fn default_decay_rate(&self) -> f64 {
        match self {
            Self::ShortTerm => 0.1,
            Self::Working => 0.02,
            Self::LongTerm => 0.005,
            Self::Core => 0.001,
        }
    }

    fn promotion_threshold(&self) -> u64 {
        match self {
            Self::ShortTerm => 3,
            Self::Working => 8,
            Self::LongTerm => 20,
            Self::Core => u64::MAX,
        }
    }

    fn next_tier(&self) -> Option<MemoryTier> {
        match self {
            Self::ShortTerm => Some(MemoryTier::Working),
            Self::Working => Some(MemoryTier::LongTerm),
            Self::LongTerm => Some(MemoryTier::Core),
            Self::Core => None,
        }
    }
}

// ── Memory Nature ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryNature {
    #[default]
    Semantic,
    Episodic,
}

impl MemoryNature {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "episodic" => Self::Episodic,
            _ => Self::Semantic,
        }
    }
}

// ── Memory Provenance ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProvenance {
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub extraction_method: String,
}

// ── Memory Entry ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub tier: MemoryTier,
    pub importance: f64,
    pub access_count: u64,
    pub last_accessed: i64,
    pub decay_rate: f64,
    pub created_at: i64,
    pub updated_at: i64,
    pub expires_at: Option<i64>,
    pub nature: MemoryNature,
    pub provenance: Option<MemoryProvenance>,
    pub tags: Vec<String>,
    pub namespace_id: Option<String>,
}

impl MemoryEntry {
    pub fn effective_score(&self) -> f64 {
        let now = chrono::Utc::now().timestamp();
        let hours_elapsed = ((now - self.last_accessed).max(0) as f64) / 3600.0;
        let time_decay = (-self.decay_rate * hours_elapsed).exp();
        let access_boost = 1.0 + (1.0 + self.access_count as f64).ln();
        let tier_bonus = self.tier.prompt_priority() as f64 * 0.1;
        self.importance * time_decay * access_boost + tier_bonus
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            chrono::Utc::now().timestamp() > expires
        } else {
            false
        }
    }

    pub fn should_promote(&self) -> bool {
        self.access_count >= self.tier.promotion_threshold()
            && !self.is_expired()
            && self.tier.next_tier().is_some()
    }

    pub fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = chrono::Utc::now().timestamp();
    }
}

// ── Working Memory ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkingMemory {
    pub entries: HashMap<String, MemoryEntry>,
}

impl WorkingMemory {
    fn sorted_by_score(&self) -> Vec<&MemoryEntry> {
        let mut entries: Vec<&MemoryEntry> = self.entries.values().collect();
        entries.sort_by(|a, b| {
            b.effective_score()
                .partial_cmp(&a.effective_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries
    }
}

// ── Config & Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_memory_entries: usize,
    pub max_user_entries: usize,
    pub token_limit: usize,
    pub dedup_similarity_threshold: f64,
    pub eviction_score_threshold: f64,
    pub decay_tick_interval_secs: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memory_entries: 50,
            max_user_entries: 100,
            token_limit: 4000,
            dedup_similarity_threshold: 0.7,
            eviction_score_threshold: 0.05,
            decay_tick_interval_secs: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub memory_count: usize,
    pub user_count: usize,
    pub total_tokens: usize,
    pub tier_counts: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub session_id: String,
    pub session_title: String,
    pub message_content: String,
    pub match_type: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryActionResult {
    pub success: bool,
    pub message: String,
    pub new_usage: Option<MemoryUsage>,
}

// ── Memory Service ───────────────────────────────────────────────────────────

pub struct MemoryService {
    storage: Arc<TrajectoryStorage>,
    working_memory: RwLock<WorkingMemory>,
    config: MemoryConfig,
}

impl MemoryService {
    pub fn new(storage: Arc<TrajectoryStorage>) -> anyhow::Result<Self> {
        Ok(Self {
            storage,
            working_memory: RwLock::new(WorkingMemory::default()),
            config: MemoryConfig::default(),
        })
    }

    pub fn with_config(mut self, config: MemoryConfig) -> Self {
        self.config = config;
        self
    }

    pub fn storage(&self) -> Arc<TrajectoryStorage> {
        self.storage.clone()
    }

    pub fn initialize(&self) -> anyhow::Result<()> {
        self.storage.init_memory_tables()?;
        self.load_memories_from_storage()
    }

    fn load_memories_from_storage(&self) -> anyhow::Result<()> {
        let memories = self.storage.get_all_memories()?;

        let mut working = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        for memory in memories {
            let entry = MemoryEntry {
                id: memory.id.clone(),
                content: memory.content.clone(),
                memory_type: memory.memory_type.clone(),
                tier: memory.tier,
                importance: memory.importance,
                access_count: memory.access_count,
                last_accessed: memory.last_accessed,
                decay_rate: memory.decay_rate,
                created_at: memory.created_at,
                updated_at: memory.updated_at,
                expires_at: memory.expires_at,
                nature: memory.nature,
                provenance: memory.provenance,
                tags: memory.tags,
                namespace_id: memory.namespace_id.clone(),
            };

            if entry.is_expired() {
                if let Err(e) = self.storage.delete_memory(&entry.id) {
                    tracing::warn!("Failed to delete expired memory {}: {}", entry.id, e);
                }
                continue;
            }

            working.entries.insert(entry.id.clone(), entry);
        }

        Ok(())
    }

    // ── Core CRUD ────────────────────────────────────────────────────────────

    pub fn add_memory(&self, target: &str, content: &str) -> MemoryActionResult {
        self.add_memory_advanced(AddMemoryRequest {
            target: target.to_string(),
            content: content.to_string(),
            tier: MemoryTier::Working,
            importance: 0.5,
            nature: MemoryNature::Semantic,
            provenance: None,
            tags: vec![],
            expires_at: None,
            namespace_id: None,
        })
    }

    pub fn add_memory_advanced(&self, req: AddMemoryRequest) -> MemoryActionResult {
        if req.content.trim().is_empty() {
            return MemoryActionResult {
                success: false,
                message: "内容不能为空".to_string(),
                new_usage: None,
            };
        }

        let dedup_result = self.check_dedup(&req.content);
        if let Some(dedup) = dedup_result {
            return dedup;
        }

        let now = chrono::Utc::now().timestamp();
        let entry = MemoryEntry {
            id: format!("mem_{}_{}", now, &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]),
            content: req.content.clone(),
            memory_type: req.target.clone(),
            tier: req.tier,
            importance: req.importance,
            access_count: 0,
            last_accessed: now,
            decay_rate: req.tier.default_decay_rate(),
            created_at: now,
            updated_at: now,
            expires_at: req.expires_at,
            nature: req.nature,
            provenance: req.provenance,
            tags: req.tags,
            namespace_id: req.namespace_id.clone(),
        };

        if let Err(e) = self.storage.save_memory(&entry) {
            return MemoryActionResult {
                success: false,
                message: format!("保存失败: {}", e),
                new_usage: None,
            };
        }

        if let Err(e) = crate::storage::TrajectoryStorage::block_on(self.storage.index_memory_fts(
            &entry.id,
            &entry.memory_type,
            &entry.content,
            &entry.tags,
        )) {
            tracing::warn!("Failed to sync FTS5 index for new memory: {}", e);
        }

        {
            let mut mem = self.working_memory.write().unwrap_or_else(|e| {
                tracing::warn!("Working memory lock poisoned, recovering: {}", e);
                e.into_inner()
            });
            mem.entries.insert(entry.id.clone(), entry);
        }

        self.enforce_tier_capacity(req.tier);

        MemoryActionResult {
            success: true,
            message: format!("已添加记忆: \"{}\"", &req.content[..req.content.len().min(30)]),
            new_usage: Some(self.get_memory_usage()),
        }
    }

    pub fn replace_memory(
        &self,
        target: &str,
        old_text: &str,
        new_text: &str,
    ) -> MemoryActionResult {
        if old_text.trim().is_empty() || new_text.trim().is_empty() {
            return MemoryActionResult {
                success: false,
                message: "旧文本和新文本都不能为空".to_string(),
                new_usage: None,
            };
        }

        let mut mem = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let found = mem
            .entries
            .values()
            .find(|e| e.memory_type == target && e.content.contains(old_text));

        if let Some(found) = found {
            let id = found.id.clone();
            let now = chrono::Utc::now().timestamp();
            let mut updated = found.clone();
            updated.content = new_text.to_string();
            updated.updated_at = now;
            updated.last_accessed = now;

            if let Err(e) = self.storage.save_memory(&updated) {
                return MemoryActionResult {
                    success: false,
                    message: format!("替换失败: {}", e),
                    new_usage: None,
                };
            }

            if let Err(e) =
                crate::storage::TrajectoryStorage::block_on(self.storage.index_memory_fts(
                    &updated.id,
                    &updated.memory_type,
                    &updated.content,
                    &updated.tags,
                ))
            {
                tracing::warn!("Failed to sync FTS5 index for replaced memory: {}", e);
            }

            mem.entries.insert(id, updated);

            MemoryActionResult {
                success: true,
                message: "已替换记忆".to_string(),
                new_usage: Some(self.get_memory_usage()),
            }
        } else {
            MemoryActionResult {
                success: false,
                message: "未找到要替换的记忆".to_string(),
                new_usage: None,
            }
        }
    }

    pub fn remove_memory(&self, target: &str, text: &str) -> MemoryActionResult {
        if text.trim().is_empty() {
            return MemoryActionResult {
                success: false,
                message: "要删除的文本不能为空".to_string(),
                new_usage: None,
            };
        }

        let mut mem = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let found = mem
            .entries
            .values()
            .find(|e| e.memory_type == target && e.content.contains(text));

        if let Some(found) = found {
            let id = found.id.clone();
            if let Err(e) = self.storage.delete_memory(&id) {
                return MemoryActionResult {
                    success: false,
                    message: format!("删除失败: {}", e),
                    new_usage: None,
                };
            }

            if let Err(e) =
                crate::storage::TrajectoryStorage::block_on(self.storage.delete_memory_fts(&id))
            {
                tracing::warn!("Failed to remove memory from FTS5 index: {}", e);
            }

            mem.entries.remove(&id);

            MemoryActionResult {
                success: true,
                message: "已删除记忆".to_string(),
                new_usage: Some(self.get_memory_usage()),
            }
        } else {
            MemoryActionResult {
                success: false,
                message: "未找到要删除的记忆".to_string(),
                new_usage: None,
            }
        }
    }

    // ── Tier Management ──────────────────────────────────────────────────────

    pub fn promote_memory(&self, id: &str) -> MemoryActionResult {
        let mut mem = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        if let Some(entry) = mem.entries.get_mut(id) {
            if let Some(next_tier) = entry.tier.next_tier() {
                entry.tier = next_tier;
                entry.decay_rate = next_tier.default_decay_rate();
                entry.updated_at = chrono::Utc::now().timestamp();

                if let Err(e) = self.storage.save_memory(entry) {
                    return MemoryActionResult {
                        success: false,
                        message: format!("晋升保存失败: {}", e),
                        new_usage: None,
                    };
                }

                return MemoryActionResult {
                    success: true,
                    message: format!("记忆已晋升到 {} 层", next_tier.as_str()),
                    new_usage: Some(self.get_memory_usage()),
                };
            } else {
                return MemoryActionResult {
                    success: false,
                    message: "记忆已在最高层，无法继续晋升".to_string(),
                    new_usage: None,
                };
            }
        }

        MemoryActionResult {
            success: false,
            message: "未找到指定记忆".to_string(),
            new_usage: None,
        }
    }

    pub fn demote_memory(&self, id: &str) -> MemoryActionResult {
        let mut mem = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        if let Some(entry) = mem.entries.get_mut(id) {
            let lower_tier = match entry.tier {
                MemoryTier::Core => MemoryTier::LongTerm,
                MemoryTier::LongTerm => MemoryTier::Working,
                MemoryTier::Working => MemoryTier::ShortTerm,
                MemoryTier::ShortTerm => {
                    return MemoryActionResult {
                        success: false,
                        message: "记忆已在最低层".to_string(),
                        new_usage: None,
                    };
                },
            };

            entry.tier = lower_tier;
            entry.decay_rate = lower_tier.default_decay_rate();
            entry.updated_at = chrono::Utc::now().timestamp();

            if let Err(e) = self.storage.save_memory(entry) {
                return MemoryActionResult {
                    success: false,
                    message: format!("降级保存失败: {}", e),
                    new_usage: None,
                };
            }

            return MemoryActionResult {
                success: true,
                message: format!("记忆已降级到 {} 层", lower_tier.as_str()),
                new_usage: Some(self.get_memory_usage()),
            };
        }

        MemoryActionResult {
            success: false,
            message: "未找到指定记忆".to_string(),
            new_usage: None,
        }
    }

    fn enforce_tier_capacity(&self, tier: MemoryTier) {
        let capacity = tier.default_capacity();
        let mut mem = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let tier_entries: Vec<(String, f64)> = mem
            .entries
            .iter()
            .filter(|(_, e)| e.tier == tier)
            .map(|(id, e)| (id.clone(), e.effective_score()))
            .collect();

        if tier_entries.len() > capacity {
            let mut sorted = tier_entries;
            sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            let to_evict = sorted.len() - capacity;
            for (id, score) in sorted.iter().take(to_evict) {
                if *score < self.config.eviction_score_threshold {
                    tracing::info!("Evicting low-score memory {} (score={:.4})", id, score);
                    if let Err(e) = self.storage.delete_memory(id) {
                        tracing::warn!("Failed to evict memory {}: {}", id, e);
                    }
                    if let Err(e) = crate::storage::TrajectoryStorage::block_on(
                        self.storage.delete_memory_fts(id),
                    ) {
                        tracing::warn!("Failed to remove evicted memory from FTS5: {}", e);
                    }
                    mem.entries.remove(id);
                }
            }
        }
    }

    // ── Decay & Auto-Promotion ───────────────────────────────────────────────

    pub fn apply_decay_tick(&self) -> usize {
        let mut mem = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let now = chrono::Utc::now().timestamp();
        let mut evicted = 0;
        let mut to_promote = Vec::new();

        let expired_ids: Vec<String> = mem
            .entries
            .iter()
            .filter(|(_, e)| e.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired_ids {
            if let Err(e) = self.storage.delete_memory(id) {
                tracing::warn!("Failed to delete expired memory {}: {}", id, e);
            }
            if let Err(e) =
                crate::storage::TrajectoryStorage::block_on(self.storage.delete_memory_fts(id))
            {
                tracing::warn!("Failed to remove expired memory from FTS5: {}", e);
            }
            mem.entries.remove(id);
            evicted += 1;
        }

        for entry in mem.entries.values_mut() {
            let hours_since_access = ((now - entry.last_accessed).max(0) as f64) / 3600.0;
            let decay_factor = (-entry.decay_rate * hours_since_access).exp();
            entry.importance *= decay_factor.max(0.01);

            if entry.importance < self.config.eviction_score_threshold {
                if let Err(e) = self.storage.delete_memory(&entry.id) {
                    tracing::warn!("Failed to delete decayed memory {}: {}", entry.id, e);
                }
                if let Err(e) = crate::storage::TrajectoryStorage::block_on(
                    self.storage.delete_memory_fts(&entry.id),
                ) {
                    tracing::warn!("Failed to remove decayed memory from FTS5: {}", e);
                }
                evicted += 1;
            } else if entry.should_promote() {
                to_promote.push(entry.id.clone());
            }
        }

        let evicted_ids: Vec<String> = mem
            .entries
            .iter()
            .filter(|(_, e)| e.importance < self.config.eviction_score_threshold)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &evicted_ids {
            mem.entries.remove(id);
        }

        drop(mem);

        for id in to_promote {
            let _ = self.promote_memory(&id);
        }

        evicted
    }

    // ── Dedup & Merge ────────────────────────────────────────────────────────

    fn check_dedup(&self, content: &str) -> Option<MemoryActionResult> {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let content_lower = content.to_lowercase();
        let content_words: Vec<&str> = content_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        if content_words.is_empty() {
            return None;
        }

        for entry in mem.entries.values() {
            let entry_lower = entry.content.to_lowercase();
            let entry_words: Vec<&str> = entry_lower
                .split_whitespace()
                .filter(|w| w.len() > 2)
                .collect();

            if entry_words.is_empty() {
                continue;
            }

            let intersection = content_words
                .iter()
                .filter(|w| entry_words.contains(w))
                .count();
            let union = content_words.len() + entry_words.len() - intersection;
            let similarity = if union > 0 {
                intersection as f64 / union as f64
            } else {
                0.0
            };

            if similarity >= self.config.dedup_similarity_threshold {
                return Some(MemoryActionResult {
                    success: false,
                    message: format!(
                        "检测到相似记忆（相似度 {:.0}%），已跳过重复添加: \"{}\"",
                        similarity * 100.0,
                        &entry.content[..entry.content.len().min(50)]
                    ),
                    new_usage: None,
                });
            }
        }

        None
    }

    pub fn add_memory_with_dedup(&self, target: &str, content: &str) -> MemoryActionResult {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let content_chars: Vec<char> = content.to_lowercase().chars().collect();
        let content_bigrams: std::collections::HashSet<String> = content_chars
            .windows(2)
            .map(|w| w.iter().collect::<String>())
            .collect();

        let mut best_match: Option<(String, f64, String)> = None;

        if !content_bigrams.is_empty() {
            for entry in mem.entries.values() {
                if entry.memory_type != target {
                    continue;
                }
                let entry_chars: Vec<char> = entry.content.to_lowercase().chars().collect();
                let entry_bigrams: std::collections::HashSet<String> = entry_chars
                    .windows(2)
                    .map(|w| w.iter().collect::<String>())
                    .collect();

                if entry_bigrams.is_empty() {
                    continue;
                }

                let intersection = content_bigrams.intersection(&entry_bigrams).count();
                let union = content_bigrams.union(&entry_bigrams).count();
                let similarity = if union > 0 {
                    intersection as f64 / union as f64
                } else {
                    0.0
                };

                if similarity >= self.config.dedup_similarity_threshold {
                    match &best_match {
                        Some((_, best_sim, _)) if similarity > *best_sim => {
                            best_match =
                                Some((entry.id.clone(), similarity, entry.content.clone()));
                        },
                        None => {
                            best_match =
                                Some((entry.id.clone(), similarity, entry.content.clone()));
                        },
                        _ => {},
                    }
                }
            }
        }

        drop(mem);

        if let Some((_existing_id, _similarity, existing_content)) = best_match {
            let merged = self.merge_content(&existing_content, content);
            return self.replace_memory(target, &existing_content, &merged);
        }

        self.add_memory(target, content)
    }

    fn merge_content(&self, existing: &str, new: &str) -> String {
        if existing.contains(new) {
            return existing.to_string();
        }
        if new.contains(existing) {
            return new.to_string();
        }

        let existing_len = existing.len();
        let new_len = new.len();

        if new_len > existing_len * 2 {
            return new.to_string();
        }

        format!("{}; {}", existing, new)
    }

    // ── Retrieval & Prompt Formatting ────────────────────────────────────────

    pub fn get_memory_usage(&self) -> MemoryUsage {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let mut tier_counts: HashMap<String, usize> = HashMap::new();
        for entry in mem.entries.values() {
            *tier_counts
                .entry(entry.tier.as_str().to_string())
                .or_insert(0) += 1;
        }

        let memory_count = mem
            .entries
            .values()
            .filter(|e| e.memory_type == "memory")
            .count();
        let user_count = mem
            .entries
            .values()
            .filter(|e| e.memory_type == "user")
            .count();

        let total_tokens: usize = mem.entries.values().map(|e| e.content.len() / 4).sum();

        MemoryUsage {
            memory_count,
            user_count,
            total_tokens,
            tier_counts,
        }
    }

    pub fn get_working_memory(&self) -> WorkingMemory {
        self.working_memory.read().unwrap().clone()
    }

    pub fn get_all_entries_for_sync(&self) -> Vec<(String, String, String)> {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        mem.entries
            .values()
            .map(|entry| (entry.id.clone(), entry.content.clone(), entry.memory_type.clone()))
            .collect()
    }

    pub fn format_for_prompt(&self) -> String {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let sorted = mem.sorted_by_score();

        let mut sections: Vec<String> = Vec::new();
        let mut current_tier: Option<MemoryTier> = None;
        let mut token_count = 0usize;

        for entry in sorted.iter() {
            if entry.is_expired() {
                continue;
            }

            if token_count + entry.content.len() / 4 > self.config.token_limit {
                break;
            }

            if current_tier != Some(entry.tier) {
                if !sections.is_empty() {
                    sections.push(String::new());
                }
                let tier_header = match entry.tier {
                    MemoryTier::Core => "## Core Memory (always relevant)\n",
                    MemoryTier::LongTerm => "## Long-Term Memory\n",
                    MemoryTier::Working => "## Working Memory\n",
                    MemoryTier::ShortTerm => "## Recent Context\n",
                };
                sections.push(tier_header.to_string());
                current_tier = Some(entry.tier);
            }

            let nature_tag = match entry.nature {
                MemoryNature::Episodic => " [episodic]",
                MemoryNature::Semantic => "",
            };
            let time_age = {
                let now = chrono::Utc::now().timestamp();
                let hours = (now - entry.created_at).max(0) / 3600;
                if hours < 1 {
                    " [just now]".to_string()
                } else if hours < 24 {
                    format!(" [{}h ago]", hours)
                } else {
                    format!(" [{}d ago]", hours / 24)
                }
            };
            sections.push(format!("- {}{}{}", entry.content, nature_tag, time_age));
            token_count += entry.content.len() / 4;
        }

        if let Ok(entities) = self.storage.get_all_entities() {
            let top_entities: Vec<_> = entities
                .iter()
                .filter(|e| e.mention_count >= 2 || e.confidence >= 0.8)
                .take(15)
                .collect();
            if !top_entities.is_empty() && token_count < self.config.token_limit {
                sections.push(String::new());
                sections.push("## Known Entities\n".to_string());
                for entity in top_entities {
                    if token_count >= self.config.token_limit {
                        break;
                    }
                    let rels = self
                        .storage
                        .get_relationships_by_entity(&entity.id)
                        .unwrap_or_default();
                    let rel_summary: Vec<String> =
                        rels.iter()
                            .take(3)
                            .filter_map(|r| {
                                let other_id = if r.source_id == entity.id {
                                    &r.target_id
                                } else {
                                    &r.source_id
                                };
                                self.storage.get_entity(other_id).ok().flatten().map(|e| {
                                    format!("{} {} {}", entity.name, r.relation_type, e.name)
                                })
                            })
                            .collect();
                    if rel_summary.is_empty() {
                        sections.push(format!(
                            "- {} ({}: mentioned {}x)",
                            entity.name, entity.entity_type, entity.mention_count
                        ));
                    } else {
                        sections.push(format!("- {}", rel_summary.join("; ")));
                    }
                    token_count += 10;
                }
            }
        }

        sections.join("\n")
    }

    pub fn search_memories(&self, query: &str, limit: usize) -> Vec<MemoryEntry> {
        let explained = self.search_memories_explained(query, limit);
        explained.into_iter().map(|r| r.entry).collect()
    }

    pub fn search_memories_explained(
        &self,
        query: &str,
        limit: usize,
    ) -> Vec<ExplainedSearchResult> {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|w| w.len() > 1)
            .collect();

        let now = chrono::Utc::now().timestamp();
        let recency_boost = |last_accessed: i64| -> f64 {
            let hours = ((now - last_accessed).max(0) as f64) / 3600.0;
            if hours < 1.0 {
                0.3
            } else if hours < 24.0 {
                0.2
            } else if hours < 168.0 {
                0.1
            } else {
                0.0
            }
        };

        let mut scored: Vec<ExplainedSearchResult> = mem
            .entries
            .values()
            .filter(|e| !e.is_expired())
            .filter_map(|entry| {
                let content_lower = entry.content.to_lowercase();
                let matched_words: Vec<&str> = query_words
                    .iter()
                    .filter(|w| content_lower.contains(*w))
                    .copied()
                    .collect();

                if matched_words.is_empty() {
                    return None;
                }

                let relevance = matched_words.len() as f64 / query_words.len() as f64;
                let effective = entry.effective_score();
                let recency = recency_boost(entry.last_accessed);
                let total_score = relevance * 0.5 + effective * 0.3 + recency;

                let explanation = SearchExplanation {
                    matched_keywords: matched_words.iter().map(|s| s.to_string()).collect(),
                    relevance_score: relevance,
                    effective_score: effective,
                    recency_score: recency,
                    total_score,
                    reason: format!(
                        "关键词匹配 {:.0}% + 有效分 {:.2} + 时效加成 {:.2}",
                        relevance * 100.0,
                        effective,
                        recency
                    ),
                };

                Some(ExplainedSearchResult {
                    entry: entry.clone(),
                    explanation,
                })
            })
            .collect();

        scored.sort_by(|a, b| {
            b.explanation
                .total_score
                .partial_cmp(&a.explanation.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let results: Vec<ExplainedSearchResult> = scored.into_iter().take(limit).collect();

        drop(mem);

        let mut touched = Vec::new();
        for r in &results {
            touched.push(r.entry.id.clone());
        }

        if !touched.is_empty() {
            let mut mem = self.working_memory.write().unwrap_or_else(|e| {
                tracing::warn!("Working memory lock poisoned, recovering: {}", e);
                e.into_inner()
            });
            for id in &touched {
                if let Some(entry) = mem.entries.get_mut(id) {
                    entry.touch();
                }
            }
        }

        results
    }

    pub fn search_memories_by_time_range(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: usize,
    ) -> Vec<MemoryEntry> {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let mut results: Vec<MemoryEntry> = mem
            .entries
            .values()
            .filter(|e| !e.is_expired())
            .filter(|e| e.created_at >= start_ts && e.created_at <= end_ts)
            .cloned()
            .collect();

        results.sort_by(|a, b| {
            b.effective_score()
                .partial_cmp(&a.effective_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.into_iter().take(limit).collect()
    }

    pub fn get_memories_grouped_by_time(&self) -> TimeGroupedMemories {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let now = chrono::Utc::now().timestamp();
        let one_day = 86400i64;
        let one_week = 604800i64;
        let one_month = 2592000i64;

        let mut groups = TimeGroupedMemories::default();

        for entry in mem.entries.values() {
            if entry.is_expired() {
                continue;
            }
            let age = now - entry.created_at;
            if age < one_day {
                groups.today.push(entry.clone());
            } else if age < one_week {
                groups.this_week.push(entry.clone());
            } else if age < one_month {
                groups.this_month.push(entry.clone());
            } else {
                groups.older.push(entry.clone());
            }
        }

        let sort_entries = |entries: &mut Vec<MemoryEntry>| {
            entries.sort_by(|a, b| {
                b.effective_score()
                    .partial_cmp(&a.effective_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        };
        sort_entries(&mut groups.today);
        sort_entries(&mut groups.this_week);
        sort_entries(&mut groups.this_month);
        sort_entries(&mut groups.older);

        groups
    }

    pub fn update_importance(&self, id: &str, delta: f64) -> MemoryActionResult {
        let mut mem = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        if let Some(entry) = mem.entries.get_mut(id) {
            entry.importance = (entry.importance + delta).clamp(0.0, 1.0);
            entry.updated_at = chrono::Utc::now().timestamp();

            if let Err(e) = self.storage.save_memory(entry) {
                return MemoryActionResult {
                    success: false,
                    message: format!("更新重要性失败: {}", e),
                    new_usage: None,
                };
            }

            MemoryActionResult {
                success: true,
                message: format!("重要性已更新为 {:.2}", entry.importance),
                new_usage: Some(self.get_memory_usage()),
            }
        } else {
            MemoryActionResult {
                success: false,
                message: "未找到指定记忆".to_string(),
                new_usage: None,
            }
        }
    }

    pub fn graph_enhanced_search(&self, query: &str, limit: usize) -> Vec<GraphEnhancedResult> {
        let base_results = self.search_memories(query, limit);
        let entities = self.storage.search_entities(query, 10).unwrap_or_default();

        let entity_ids: Vec<String> = entities.iter().map(|e| e.id.clone()).collect();
        let mut related_entity_ids = std::collections::HashSet::new();
        for eid in &entity_ids {
            if let Ok(rels) = self.storage.get_relationships_by_entity(eid) {
                for rel in &rels {
                    if &rel.source_id == eid {
                        related_entity_ids.insert(rel.target_id.clone());
                    } else {
                        related_entity_ids.insert(rel.source_id.clone());
                    }
                }
            }
        }

        let mut entity_contents: Vec<String> = Vec::new();
        for entity in &entities {
            entity_contents.push(format!(
                "[entity:{}:{}] {}",
                entity.entity_type, entity.name, entity.mention_count
            ));
        }

        for eid in &related_entity_ids {
            if let Ok(Some(entity)) = self.storage.get_entity(eid)
                && !entities.iter().any(|e| e.id == entity.id)
            {
                entity_contents.push(format!(
                    "[related:{}:{}] {}",
                    entity.entity_type, entity.name, entity.mention_count
                ));
            }
        }

        base_results
            .into_iter()
            .map(|entry| {
                let entity_boost = entity_contents.len() as f64 * 0.05;
                GraphEnhancedResult {
                    entry,
                    related_entities: entity_contents.clone(),
                    graph_boost: entity_boost,
                }
            })
            .collect()
    }

    pub fn disambiguate_entities(&self) -> DisambiguationResult {
        let entities = match self.storage.get_all_entities() {
            Ok(e) => e,
            Err(_) => {
                return DisambiguationResult {
                    merged: 0,
                    total: 0,
                };
            },
        };

        let total = entities.len();
        let mut merged = 0;
        let mut processed = std::collections::HashSet::new();

        for i in 0..entities.len() {
            if processed.contains(&entities[i].id) {
                continue;
            }
            for j in (i + 1)..entities.len() {
                if processed.contains(&entities[j].id) {
                    continue;
                }

                let name_a = entities[i].name.to_lowercase().trim().to_string();
                let name_b = entities[j].name.to_lowercase().trim().to_string();

                let is_same = name_a == name_b
                    || entities[i]
                        .aliases
                        .iter()
                        .any(|a| a.to_lowercase() == name_b)
                    || entities[j]
                        .aliases
                        .iter()
                        .any(|a| a.to_lowercase() == name_a);

                if is_same && entities[i].entity_type == entities[j].entity_type {
                    let keep_id = if entities[i].mention_count >= entities[j].mention_count {
                        &entities[i]
                    } else {
                        &entities[j]
                    };
                    let remove_id = if entities[i].mention_count >= entities[j].mention_count {
                        &entities[j]
                    } else {
                        &entities[i]
                    };

                    if let Ok(rels) = self.storage.get_relationships_by_entity(&remove_id.id) {
                        for rel in &rels {
                            let new_source = if rel.source_id == remove_id.id {
                                keep_id.id.clone()
                            } else {
                                rel.source_id.clone()
                            };
                            let new_target = if rel.target_id == remove_id.id {
                                keep_id.id.clone()
                            } else {
                                rel.target_id.clone()
                            };

                            let mut new_rel = rel.clone();
                            new_rel.source_id = new_source;
                            new_rel.target_id = new_target;
                            new_rel.weight = (new_rel.weight + rel.weight) / 2.0;
                            let _ = self.storage.save_relationship(&new_rel);
                        }
                    }

                    let _ = self.storage.delete_entity(&remove_id.id);
                    processed.insert(remove_id.id.clone());
                    merged += 1;
                }
            }
        }

        DisambiguationResult { merged, total }
    }

    pub fn find_similar_clusters(&self, similarity_threshold: f64) -> Vec<MemoryCluster> {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        let entries: Vec<&MemoryEntry> = mem.entries.values().filter(|e| !e.is_expired()).collect();

        let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut clusters: Vec<MemoryCluster> = Vec::new();

        let bigram_sets: Vec<std::collections::HashSet<String>> = entries
            .iter()
            .map(|e| {
                let chars: Vec<char> = e.content.to_lowercase().chars().collect();
                let mut set = std::collections::HashSet::new();
                for w in chars.windows(2) {
                    set.insert(w.iter().collect::<String>());
                }
                set
            })
            .collect();

        for i in 0..entries.len() {
            if assigned.contains(&entries[i].id) {
                continue;
            }

            let mut cluster_ids = vec![entries[i].id.clone()];
            assigned.insert(entries[i].id.clone());

            if bigram_sets[i].is_empty() {
                continue;
            }

            for j in (i + 1)..entries.len() {
                if assigned.contains(&entries[j].id) {
                    continue;
                }

                if bigram_sets[j].is_empty() {
                    continue;
                }

                let intersection = bigram_sets[i].intersection(&bigram_sets[j]).count();
                let union = bigram_sets[i].union(&bigram_sets[j]).count();
                let similarity = if union > 0 {
                    intersection as f64 / union as f64
                } else {
                    0.0
                };

                if similarity >= similarity_threshold {
                    cluster_ids.push(entries[j].id.clone());
                    assigned.insert(entries[j].id.clone());
                }
            }

            if cluster_ids.len() > 1 {
                let cluster_entries: Vec<MemoryEntry> = cluster_ids
                    .iter()
                    .filter_map(|id| mem.entries.get(id).cloned())
                    .collect();

                let combined_content: Vec<String> =
                    cluster_entries.iter().map(|e| e.content.clone()).collect();

                let avg_importance = cluster_entries.iter().map(|e| e.importance).sum::<f64>()
                    / cluster_entries.len() as f64;

                let best_tier = cluster_entries
                    .iter()
                    .map(|e| e.tier.prompt_priority())
                    .max()
                    .unwrap_or(1);

                clusters.push(MemoryCluster {
                    ids: cluster_ids,
                    contents: combined_content,
                    avg_importance,
                    best_tier_priority: best_tier,
                });
            }
        }

        clusters
    }

    pub fn apply_user_feedback(&self, memory_id: &str, feedback: &str) -> MemoryActionResult {
        let mut mem = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        if let Some(entry) = mem.entries.get_mut(memory_id) {
            match feedback {
                "useful" | "positive" => {
                    entry.importance = (entry.importance + 0.15).min(1.0);
                    entry.access_count += 2;
                },
                "not_useful" | "negative" => {
                    entry.importance = (entry.importance - 0.2).max(0.0);
                },
                "outdated" => {
                    entry.importance = (entry.importance - 0.3).max(0.0);
                    entry.expires_at = Some(chrono::Utc::now().timestamp() + 86400);
                },
                _ => {
                    return MemoryActionResult {
                        success: false,
                        message: format!("未知反馈类型: {}", feedback),
                        new_usage: None,
                    };
                },
            }

            entry.updated_at = chrono::Utc::now().timestamp();

            if let Err(e) = self.storage.save_memory(entry) {
                return MemoryActionResult {
                    success: false,
                    message: format!("反馈保存失败: {}", e),
                    new_usage: None,
                };
            }

            let action_desc = match feedback {
                "useful" | "positive" => "重要性提升",
                "not_useful" | "negative" => "重要性降低",
                "outdated" => "标记过期",
                _ => "已处理",
            };

            MemoryActionResult {
                success: true,
                message: format!("反馈已应用: {} (重要性={:.2})", action_desc, entry.importance),
                new_usage: Some(self.get_memory_usage()),
            }
        } else {
            MemoryActionResult {
                success: false,
                message: "未找到指定记忆".to_string(),
                new_usage: None,
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEnhancedResult {
    pub entry: MemoryEntry,
    pub related_entities: Vec<String>,
    pub graph_boost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisambiguationResult {
    pub merged: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeGroupedMemories {
    pub today: Vec<MemoryEntry>,
    pub this_week: Vec<MemoryEntry>,
    pub this_month: Vec<MemoryEntry>,
    pub older: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchExplanation {
    pub matched_keywords: Vec<String>,
    pub relevance_score: f64,
    pub effective_score: f64,
    pub recency_score: f64,
    pub total_score: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainedSearchResult {
    pub entry: MemoryEntry,
    pub explanation: SearchExplanation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCluster {
    pub ids: Vec<String>,
    pub contents: Vec<String>,
    pub avg_importance: f64,
    pub best_tier_priority: u8,
}

// ── Add Memory Request ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemoryRequest {
    pub target: String,
    pub content: String,
    #[serde(default = "default_tier")]
    pub tier: MemoryTier,
    #[serde(default = "default_importance")]
    pub importance: f64,
    #[serde(default)]
    pub nature: MemoryNature,
    pub provenance: Option<MemoryProvenance>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub expires_at: Option<i64>,
    pub namespace_id: Option<String>,
}

fn default_tier() -> MemoryTier {
    MemoryTier::Working
}

fn default_importance() -> f64 {
    0.5
}
