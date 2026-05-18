//! Semantic cache layer for RAG queries to avoid redundant embedding computation and vector search.
//!
//! This module provides a semantic cache that uses cosine similarity to find cached results
//! for semantically similar queries, avoiding expensive re-computation of embeddings and vector searches.
//!
//! # Architecture
//!
//! - [`CacheEntry`]: Individual cache entry with embedding, result, and metadata
//! - [`CacheStats`]: Statistics about cache usage
//! - [`SemanticCache`]: Main cache implementation with TTL and LRU eviction
//!
//! # Example
//!
//! ```
//! use axagent_core::semantic_cache::SemanticCache;
//!
//! let mut cache = SemanticCache::new(0.95, 10000);
//!
//! // Cache a query result
//! let key = cache.insert(
//!     vec![0.1, 0.2, 0.3],
//!     "What is Rust?".to_string(),
//!     "Rust is a systems programming language...".to_string(),
//!     vec!["chunk1".to_string()],
//!     0.92,
//!     3600,
//! );
//!
//! // Search for similar queries
//! if let Some(entry) = cache.search(&[0.1, 0.2, 0.3]) {
//!     println!("Cache hit! Result: {}", entry.result);
//! }
//! ```

use crate::error::{AxAgentError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

/// A single entry in the semantic cache containing the cached query result and metadata.
///
/// # Fields
///
/// - `query_embedding`: The embedding vector of the original query
/// - `query_text`: The original query text
/// - `result`: The cached search result
/// - `chunk_ids`: IDs of the chunks that were returned
/// - `score`: Confidence/relevance score of the result
/// - `created_at`: When this entry was created
/// - `last_accessed`: When this entry was last accessed
/// - `access_count`: How many times this entry was hit
/// - `ttl_secs`: Time-to-live in seconds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub query_embedding: Vec<f32>,
    pub query_text: String,
    pub result: String,
    pub chunk_ids: Vec<String>,
    pub score: f32,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
    pub ttl_secs: u32,
}

impl CacheEntry {
    /// Creates a new cache entry with the given parameters.
    pub fn new(
        query_embedding: Vec<f32>,
        query_text: String,
        result: String,
        chunk_ids: Vec<String>,
        score: f32,
        ttl_secs: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            query_embedding,
            query_text,
            result,
            chunk_ids,
            score,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            ttl_secs,
        }
    }
}

/// Statistics about cache usage for monitoring and optimization.
///
/// # Fields
///
/// - `total_entries`: Current number of entries in the cache
/// - `hits`: Total number of cache hits
/// - `misses`: Total number of cache misses
/// - `hit_rate`: Ratio of hits to total lookups (0.0 to 1.0)
/// - `avg_access_count`: Average number of times entries have been accessed
/// - `expired_count`: Number of entries that have expired
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub avg_access_count: f64,
    pub expired_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheBucket {
    centroid_embedding: Vec<f32>,
    entry_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApproximateIndex {
    buckets: Vec<CacheBucket>,
    num_buckets: usize,
}

impl ApproximateIndex {
    fn new(num_buckets: usize) -> Self {
        Self {
            buckets: Vec::with_capacity(num_buckets),
            num_buckets,
        }
    }

    fn assign_bucket(&self, embedding: &[f32]) -> usize {
        if self.buckets.is_empty() {
            return 0;
        }
        let mut best_bucket = 0;
        let mut best_sim = -1.0f32;
        for (i, bucket) in self.buckets.iter().enumerate() {
            let sim = cosine_similarity(embedding, &bucket.centroid_embedding);
            if sim > best_sim {
                best_sim = sim;
                best_bucket = i;
            }
        }
        best_bucket
    }

    fn add_entry(&mut self, key: &str, embedding: &[f32]) {
        if self.buckets.is_empty() || self.buckets.len() < self.num_buckets {
            self.buckets.push(CacheBucket {
                centroid_embedding: embedding.to_vec(),
                entry_keys: vec![key.to_string()],
            });
            return;
        }

        let bucket_idx = self.assign_bucket(embedding);
        if let Some(bucket) = self.buckets.get_mut(bucket_idx) {
            for (i, v) in embedding.iter().enumerate() {
                if i < bucket.centroid_embedding.len() {
                    let n = bucket.entry_keys.len() as f32;
                    bucket.centroid_embedding[i] =
                        bucket.centroid_embedding[i] * (n - 1.0) / n + v / n;
                }
            }
            bucket.entry_keys.push(key.to_string());
        }
    }

    fn remove_entry(&mut self, key: &str) {
        for bucket in &mut self.buckets {
            bucket.entry_keys.retain(|k| k != key);
        }
    }

    fn get_candidate_keys(&self, embedding: &[f32], max_buckets: usize) -> Vec<String> {
        if self.buckets.is_empty() {
            return Vec::new();
        }

        let mut scored_buckets: Vec<(usize, f32)> = self
            .buckets
            .iter()
            .enumerate()
            .map(|(i, b)| (i, cosine_similarity(embedding, &b.centroid_embedding)))
            .collect();
        scored_buckets.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored_buckets
            .iter()
            .take(max_buckets)
            .flat_map(|(idx, _)| {
                self.buckets
                    .get(*idx)
                    .map(|b| b.entry_keys.clone())
                    .unwrap_or_default()
            })
            .collect()
    }

    fn clear(&mut self) {
        self.buckets.clear();
    }
}

/// A semantic cache for RAG query results that uses cosine similarity
/// to match similar queries and return cached results.
///
/// # Features
///
/// - Semantic similarity matching using cosine similarity
/// - Configurable similarity threshold
/// - TTL-based expiration
/// - LRU eviction when cache is full
/// - Disk persistence for cache warm-up
/// - Statistics tracking for monitoring
#[derive(Debug)]
pub struct SemanticCache {
    entries: HashMap<String, CacheEntry>,
    similarity_threshold: f32,
    max_entries: usize,
    enabled: bool,
    hits: u64,
    misses: u64,
    index: ApproximateIndex,
}

impl SemanticCache {
    /// Creates a new semantic cache with the specified parameters.
    ///
    /// # Parameters
    ///
    /// - `similarity_threshold`: Minimum cosine similarity (0.0-1.0) to consider a cache hit.
    ///   Default is 0.95 for very strict matching.
    /// - `max_entries`: Maximum number of entries to store. Default is 10000.
    pub fn new(similarity_threshold: f32, max_entries: usize) -> Self {
        info!(
            similarity_threshold = %similarity_threshold,
            max_entries = %max_entries,
            "Initializing semantic cache"
        );
        Self {
            entries: HashMap::new(),
            similarity_threshold,
            max_entries,
            enabled: true,
            hits: 0,
            misses: 0,
            index: ApproximateIndex::new(16),
        }
    }

    pub fn with_index_buckets(mut self, num_buckets: usize) -> Self {
        self.index = ApproximateIndex::new(num_buckets);
        self
    }

    /// Searches for a cache entry with similar embedding above the similarity threshold.
    ///
    /// This performs a linear scan of all entries, computing cosine similarity
    /// against each cached embedding. Returns the first entry that exceeds the threshold.
    ///
    /// # Parameters
    ///
    /// - `query_embedding`: The embedding vector to search for
    ///
    /// # Returns
    ///
    /// Returns `Some(&CacheEntry)` if a match is found, `None` otherwise.
    /// Updates access statistics on hit.
    pub fn search(&mut self, query_embedding: &[f32]) -> Option<&CacheEntry> {
        if !self.enabled {
            debug!("Semantic cache is disabled");
            self.misses += 1;
            return None;
        }

        if self.entries.is_empty() {
            debug!("Semantic cache is empty");
            self.misses += 1;
            return None;
        }

        let candidate_keys = self.index.get_candidate_keys(query_embedding, 3);

        let mut best_sim = 0.0f32;
        let mut matching_key: Option<String> = None;

        if candidate_keys.is_empty() {
            for (k, entry) in &self.entries {
                if is_expired(entry) {
                    continue;
                }
                let sim = cosine_similarity(query_embedding, &entry.query_embedding);
                if sim >= self.similarity_threshold && sim > best_sim {
                    best_sim = sim;
                    matching_key = Some(k.clone());
                }
            }
        } else {
            for k in &candidate_keys {
                let entry = match self.entries.get(k) {
                    Some(e) => e,
                    None => continue,
                };
                if is_expired(entry) {
                    continue;
                }
                let sim = cosine_similarity(query_embedding, &entry.query_embedding);
                if sim >= self.similarity_threshold && sim > best_sim {
                    best_sim = sim;
                    matching_key = Some(k.clone());
                }
            }
        }

        if let Some(key) = matching_key
            && let Some(entry) = self.entries.get_mut(&key)
        {
            entry.last_accessed = Utc::now();
            entry.access_count += 1;
            self.hits += 1;
            debug!(
                key = %key,
                similarity = %best_sim,
                access_count = %entry.access_count,
                "Semantic cache hit"
            );
            return Some(entry);
        }

        debug!("Semantic cache miss");
        self.misses += 1;
        None
    }

    /// Inserts a new entry into the cache.
    ///
    /// If the cache is at capacity, the least recently used entry is evicted.
    /// Returns the cache key for the inserted entry.
    ///
    /// # Parameters
    ///
    /// - `query_embedding`: The embedding vector of the query
    /// - `query_text`: The original query text
    /// - `result`: The search result to cache
    /// - `chunk_ids`: IDs of the chunks that were returned
    /// - `score`: Confidence/relevance score
    /// - `ttl_secs`: Time-to-live in seconds
    ///
    /// # Returns
    ///
    /// Returns the cache key (SHA-256 hash of the embedding).
    pub fn insert(
        &mut self,
        query_embedding: Vec<f32>,
        query_text: String,
        result: String,
        chunk_ids: Vec<String>,
        score: f32,
        ttl_secs: u32,
    ) -> String {
        if !self.enabled {
            debug!("Semantic cache is disabled, skipping insert");
            return String::new();
        }

        let key = embedding_hash(&query_embedding);

        self.index.add_entry(&key, &query_embedding);

        let entry =
            CacheEntry::new(query_embedding, query_text, result, chunk_ids, score, ttl_secs);

        if self.entries.len() >= self.max_entries {
            let evicted = self.invalidate_least_used(1);
            debug!(evicted = %evicted, "Evicted entries to make room for new cache entry");
        }

        debug!(
            key = %key,
            ttl_secs = %ttl_secs,
            "Inserting into semantic cache"
        );
        self.entries.insert(key.clone(), entry);
        key
    }

    /// Removes all expired entries from the cache.
    ///
    /// # Returns
    ///
    /// Returns the number of entries that were removed.
    pub fn invalidate_expired(&mut self) -> usize {
        let before = self.entries.len();
        let expired_keys: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| is_expired(entry))
            .map(|(k, _)| k.clone())
            .collect();
        for key in &expired_keys {
            debug!(key = %key, "Removing expired cache entry");
            self.entries.remove(key);
            self.index.remove_entry(key);
        }
        let removed = before - self.entries.len();
        if removed > 0 {
            info!(removed = %removed, "Invalidated expired cache entries");
        }
        removed
    }

    /// Evicts the least recently used entries from the cache.
    ///
    /// Entries are sorted by `last_accessed` timestamp and the oldest ones are removed.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of entries to evict
    ///
    /// # Returns
    ///
    /// Returns the number of entries that were actually evicted.
    pub fn invalidate_least_used(&mut self, count: usize) -> usize {
        if count == 0 || self.entries.is_empty() {
            return 0;
        }

        let mut entries: Vec<_> = self
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.last_accessed))
            .collect();

        entries.sort_by_key(|(_, accessed)| *accessed);

        let to_evict = count.min(entries.len());
        for (key, _) in entries.iter().take(to_evict) {
            debug!(key = %key, "Evicting least used cache entry");
            self.entries.remove(key);
            self.index.remove_entry(key);
        }

        info!(evicted = %to_evict, "Evicted least used cache entries");
        to_evict
    }

    /// Clears all entries from the cache and resets statistics.
    pub fn clear(&mut self) {
        let count = self.entries.len();
        self.entries.clear();
        self.index.clear();
        self.hits = 0;
        self.misses = 0;
        info!(cleared = %count, "Cleared semantic cache");
    }

    /// Returns statistics about the cache usage.
    ///
    /// Computes current statistics including hit rate, average access count,
    /// and expired entry count.
    pub fn stats(&self) -> CacheStats {
        let total_lookups = self.hits + self.misses;
        let hit_rate = if total_lookups > 0 {
            self.hits as f64 / total_lookups as f64
        } else {
            0.0
        };

        let avg_access_count = if self.entries.is_empty() {
            0.0
        } else {
            let total_accesses: u64 = self.entries.values().map(|e| e.access_count as u64).sum();
            total_accesses as f64 / self.entries.len() as f64
        };

        let expired_count = self.entries.values().filter(|e| is_expired(e)).count();

        CacheStats {
            total_entries: self.entries.len(),
            hits: self.hits,
            misses: self.misses,
            hit_rate,
            avg_access_count,
            expired_count,
        }
    }

    /// Returns whether the cache is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enables or disables the cache.
    ///
    /// When disabled, `search()` always returns None and `insert()` is a no-op.
    pub fn set_enabled(&mut self, enabled: bool) {
        info!(enabled = %enabled, "Semantic cache enabled state changed");
        self.enabled = enabled;
    }

    pub fn search_by_text(&mut self, query_text: &str) -> Option<&CacheEntry> {
        if !self.enabled {
            self.misses += 1;
            return None;
        }

        let mut best_entry: Option<&CacheEntry> = None;
        let mut best_similarity = 0.0f32;

        for entry in self.entries.values() {
            if is_expired(entry) {
                continue;
            }
            let text_sim = text_similarity(query_text, &entry.query_text);
            if text_sim >= self.similarity_threshold && text_sim > best_similarity {
                best_similarity = text_sim;
                best_entry = Some(entry);
            }
        }

        if best_entry.is_some() {
            self.hits += 1;
        } else {
            self.misses += 1;
        }

        best_entry
    }

    pub fn insert_by_text(
        &mut self,
        query_text: String,
        result: String,
        chunk_ids: Vec<String>,
        score: f32,
        ttl_secs: u32,
    ) -> String {
        let embedding = vec![0.0f32];
        self.insert(embedding, query_text, result, chunk_ids, score, ttl_secs)
    }

    /// Saves the cache to disk for persistence.
    ///
    /// Serializes all cache entries to JSON and writes to the specified path.
    ///
    /// # Parameters
    ///
    /// - `path`: File path to save the cache to
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file writing fails.
    pub fn save_to_disk(&self, path: &Path) -> Result<()> {
        info!(path = ?path, "Saving semantic cache to disk");

        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| AxAgentError::Internal(format!("Failed to serialize cache: {}", e)))?;

        std::fs::write(path, json)
            .map_err(|e| AxAgentError::Io(std::io::Error::other(e.to_string())))?;

        info!(entries = %self.entries.len(), path = ?path, "Semantic cache saved to disk");
        Ok(())
    }

    /// Loads the cache from disk.
    ///
    /// Reads and deserializes cache entries from the specified path.
    /// Merges with existing entries (disk entries overwrite existing ones with same key).
    ///
    /// # Parameters
    ///
    /// - `path`: File path to load the cache from
    ///
    /// # Errors
    ///
    /// Returns an error if file reading or deserialization fails.
    pub fn load_from_disk(&mut self, path: &Path) -> Result<()> {
        info!(path = ?path, "Loading semantic cache from disk");

        if !path.exists() {
            debug!(path = ?path, "Cache file does not exist, skipping load");
            return Ok(());
        }

        let json = std::fs::read_to_string(path)
            .map_err(|e| AxAgentError::Io(std::io::Error::other(e.to_string())))?;

        let entries: HashMap<String, CacheEntry> = serde_json::from_str(&json)
            .map_err(|e| AxAgentError::Internal(format!("Failed to deserialize cache: {}", e)))?;

        let loaded = entries.len();
        self.entries.extend(entries);

        info!(loaded = %loaded, total = %self.entries.len(), "Semantic cache loaded from disk");
        Ok(())
    }
}

impl Default for SemanticCache {
    fn default() -> Self {
        Self::new(0.95, 10000)
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Computes the cosine similarity between two embedding vectors.
///
/// # Parameters
///
/// - `a`: First embedding vector
/// - `b`: Second embedding vector
///
/// # Returns
///
/// Returns a value between 0.0 and 1.0 representing the cosine similarity.
/// Returns 0.0 if vectors have different lengths or are empty.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot_product = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (va, vb) in a.iter().zip(b.iter()) {
        dot_product += va * vb;
        norm_a += va * va;
        norm_b += vb * vb;
    }

    let magnitude = norm_a.sqrt() * norm_b.sqrt();
    if magnitude < 1e-10 {
        return 0.0;
    }

    dot_product / magnitude
}

/// Computes a SHA-256 hash of an embedding vector for use as a cache key.
///
/// # Parameters
///
/// - `embedding`: The embedding vector to hash
///
/// # Returns
///
/// Returns a hex-encoded SHA-256 hash string.
pub fn embedding_hash(embedding: &[f32]) -> String {
    let mut hasher = Sha256::new();
    for value in embedding {
        hasher.update(value.to_le_bytes());
    }
    let result = hasher.finalize();
    hex::encode(result)
}

/// Checks if a cache entry has expired based on its TTL.
///
/// # Parameters
///
/// - `entry`: The cache entry to check
///
/// # Returns
///
/// Returns `true` if the entry has expired, `false` otherwise.
pub fn is_expired(entry: &CacheEntry) -> bool {
    let now = Utc::now();
    let age = now.signed_duration_since(entry.created_at);
    age.num_seconds() >= entry.ttl_secs as i64
}

fn text_similarity(a: &str, b: &str) -> f32 {
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();

    if a_words.is_empty() || b_words.is_empty() {
        return 0.0;
    }

    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();

    if union == 0 {
        return 0.0;
    }
    intersection as f32 / union as f32
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    fn create_test_embedding(seed: f32) -> Vec<f32> {
        vec![seed, seed + 0.1, seed + 0.2, seed + 0.3, seed + 0.4]
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = create_test_embedding(0.5);
        let b = create_test_embedding(0.5);
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_different() {
        let a = create_test_embedding(0.0);
        let b = create_test_embedding(1.0);
        let sim = cosine_similarity(&a, &b);
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_embedding_hash_consistency() {
        let embedding = create_test_embedding(0.5);
        let hash1 = embedding_hash(&embedding);
        let hash2 = embedding_hash(&embedding);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_embedding_hash_uniqueness() {
        let a = create_test_embedding(0.5);
        let b = create_test_embedding(0.6);
        assert_ne!(embedding_hash(&a), embedding_hash(&b));
    }

    #[test]
    fn test_is_expired() {
        let mut entry = CacheEntry::new(
            create_test_embedding(0.5),
            "test".to_string(),
            "result".to_string(),
            vec!["chunk1".to_string()],
            0.9,
            1,
        );

        assert!(!is_expired(&entry));

        entry.created_at = Utc::now() - chrono::Duration::seconds(2);
        assert!(is_expired(&entry));
    }

    #[test]
    fn test_cache_insert_and_search() {
        let mut cache = SemanticCache::new(0.95, 100);
        let embedding = create_test_embedding(0.5);

        let key = cache.insert(
            embedding.clone(),
            "What is Rust?".to_string(),
            "Rust is a systems programming language".to_string(),
            vec!["chunk1".to_string()],
            0.92,
            3600,
        );

        assert!(!key.is_empty());

        let result = cache.search(&embedding);
        assert!(result.is_some());
        assert_eq!(result.unwrap().query_text, "What is Rust?");
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = SemanticCache::new(0.95, 100);
        let embedding1 = create_test_embedding(0.5);
        let embedding2 = create_test_embedding(-0.5);

        cache.insert(embedding1, "query1".to_string(), "result1".to_string(), vec![], 0.9, 3600);

        let result = cache.search(&embedding2);
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_disabled() {
        let mut cache = SemanticCache::new(0.95, 100);
        cache.set_enabled(false);

        let embedding = create_test_embedding(0.5);
        cache.insert(
            embedding.clone(),
            "query".to_string(),
            "result".to_string(),
            vec![],
            0.9,
            3600,
        );

        assert!(cache.search(&embedding).is_none());
    }

    #[test]
    fn test_cache_invalidate_expired() {
        let mut cache = SemanticCache::new(0.95, 100);

        cache.insert(
            create_test_embedding(0.5),
            "q1".to_string(),
            "r1".to_string(),
            vec![],
            0.9,
            1,
        );

        cache.insert(
            create_test_embedding(0.6),
            "q2".to_string(),
            "r2".to_string(),
            vec![],
            0.9,
            3600,
        );

        std::thread::sleep(Duration::from_secs(2));

        let removed = cache.invalidate_expired();
        assert_eq!(removed, 1);
        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn test_cache_invalidate_least_used() {
        let mut cache = SemanticCache::new(0.95, 100);

        cache.insert(
            create_test_embedding(0.1),
            "q1".to_string(),
            "r1".to_string(),
            vec![],
            0.9,
            3600,
        );

        std::thread::sleep(Duration::from_millis(10));

        cache.insert(
            create_test_embedding(0.2),
            "q2".to_string(),
            "r2".to_string(),
            vec![],
            0.9,
            3600,
        );

        cache.search(&create_test_embedding(0.2));

        let evicted = cache.invalidate_least_used(1);
        assert_eq!(evicted, 1);
        assert_eq!(cache.entries.len(), 1);
        assert!(
            cache
                .entries
                .contains_key(&embedding_hash(&create_test_embedding(0.2)))
        );
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = SemanticCache::new(0.95, 100);

        cache.insert(
            create_test_embedding(0.5),
            "q1".to_string(),
            "r1".to_string(),
            vec![],
            0.9,
            3600,
        );

        cache.search(&create_test_embedding(0.5));

        cache.clear();
        assert_eq!(cache.entries.len(), 0);

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = SemanticCache::new(0.95, 100);

        let embedding = create_test_embedding(0.5);
        cache.insert(embedding.clone(), "q1".to_string(), "r1".to_string(), vec![], 0.9, 3600);

        cache.search(&embedding);
        cache.search(&create_test_embedding(-0.5));

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_cache_persistence() {
        let mut cache = SemanticCache::new(0.95, 100);

        let embedding = create_test_embedding(0.5);
        cache.insert(
            embedding.clone(),
            "test query".to_string(),
            "test result".to_string(),
            vec!["chunk1".to_string()],
            0.9,
            3600,
        );

        let temp_file = NamedTempFile::new().unwrap();
        cache.save_to_disk(temp_file.path()).unwrap();

        let mut cache2 = SemanticCache::new(0.95, 100);
        cache2.load_from_disk(temp_file.path()).unwrap();

        assert_eq!(cache2.entries.len(), 1);
        let result = cache2.search(&embedding);
        assert!(result.is_some());
        assert_eq!(result.unwrap().query_text, "test query");
    }

    #[test]
    fn test_cache_max_entries_eviction() {
        let mut cache = SemanticCache::new(0.95, 3);

        cache.insert(
            create_test_embedding(0.1),
            "q1".to_string(),
            "r1".to_string(),
            vec![],
            0.9,
            3600,
        );

        std::thread::sleep(Duration::from_millis(10));

        cache.insert(
            create_test_embedding(0.2),
            "q2".to_string(),
            "r2".to_string(),
            vec![],
            0.9,
            3600,
        );

        std::thread::sleep(Duration::from_millis(10));

        cache.insert(
            create_test_embedding(0.3),
            "q3".to_string(),
            "r3".to_string(),
            vec![],
            0.9,
            3600,
        );

        cache.insert(
            create_test_embedding(0.4),
            "q4".to_string(),
            "r4".to_string(),
            vec![],
            0.9,
            3600,
        );

        assert_eq!(cache.entries.len(), 3);
    }

    #[test]
    fn test_access_count_tracking() {
        let mut cache = SemanticCache::new(0.95, 100);
        let embedding = create_test_embedding(0.5);

        cache.insert(embedding.clone(), "q1".to_string(), "r1".to_string(), vec![], 0.9, 3600);

        cache.search(&embedding);
        cache.search(&embedding);
        cache.search(&embedding);

        let entry = cache.entries.values().next().unwrap();
        assert_eq!(entry.access_count, 3);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let mut cache = SemanticCache::new(0.95, 100);
        let result = cache.load_from_disk(Path::new("/nonexistent/path/cache.json"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_similarity_identical() {
        let sim = text_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_text_similarity_partial() {
        let sim = text_similarity("hello world foo", "hello world bar");
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn test_text_similarity_no_overlap() {
        let sim = text_similarity("aaa bbb", "ccc ddd");
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_text_similarity_empty() {
        assert_eq!(text_similarity("", "hello"), 0.0);
        assert_eq!(text_similarity("hello", ""), 0.0);
        assert_eq!(text_similarity("", ""), 0.0);
    }

    #[test]
    fn test_search_by_text_hit() {
        let mut cache = SemanticCache::new(0.5, 100);
        cache.insert_by_text(
            "how to read a file in Rust".to_string(),
            "use std::fs::read_to_string".to_string(),
            vec!["chunk1".to_string()],
            0.9,
            3600,
        );

        let result = cache.search_by_text("how to read a file in Rust");
        assert!(result.is_some());
        assert_eq!(result.unwrap().query_text, "how to read a file in Rust");
    }

    #[test]
    fn test_search_by_text_similar_hit() {
        let mut cache = SemanticCache::new(0.5, 100);
        cache.insert_by_text(
            "how to read a file in Rust".to_string(),
            "use std::fs::read_to_string".to_string(),
            vec!["chunk1".to_string()],
            0.9,
            3600,
        );

        let result = cache.search_by_text("how to read file Rust");
        assert!(result.is_some());
    }

    #[test]
    fn test_search_by_text_miss() {
        let mut cache = SemanticCache::new(0.5, 100);
        cache.insert_by_text(
            "how to read a file in Rust".to_string(),
            "use std::fs::read_to_string".to_string(),
            vec!["chunk1".to_string()],
            0.9,
            3600,
        );

        let result = cache.search_by_text("completely different topic about networking");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_by_text_disabled() {
        let mut cache = SemanticCache::new(0.5, 100);
        cache.set_enabled(false);
        cache.insert_by_text(
            "how to read a file".to_string(),
            "result".to_string(),
            vec![],
            0.9,
            3600,
        );

        let result = cache.search_by_text("how to read a file");
        assert!(result.is_none());
    }

    #[test]
    fn test_insert_by_text_returns_key() {
        let mut cache = SemanticCache::new(0.5, 100);
        let key = cache.insert_by_text(
            "test query".to_string(),
            "test result".to_string(),
            vec!["chunk1".to_string()],
            0.8,
            3600,
        );
        assert!(!key.is_empty());
    }

    #[test]
    fn test_approximate_index_new() {
        let index = ApproximateIndex::new(8);
        assert_eq!(index.buckets.len(), 0);
        assert_eq!(index.num_buckets, 8);
    }

    #[test]
    fn test_approximate_index_add_entry() {
        let mut index = ApproximateIndex::new(4);
        let emb = vec![1.0, 0.0, 0.0];
        index.add_entry("key1", &emb);
        assert_eq!(index.buckets.len(), 1);
        assert_eq!(index.buckets[0].entry_keys, vec!["key1"]);

        let emb2 = vec![0.9, 0.1, 0.0];
        index.add_entry("key2", &emb2);
        assert_eq!(index.buckets.len(), 2);

        let emb3 = vec![0.0, 1.0, 0.0];
        index.add_entry("key3", &emb3);
        assert_eq!(index.buckets.len(), 3);

        let emb4 = vec![0.0, 0.0, 1.0];
        index.add_entry("key4", &emb4);
        assert_eq!(index.buckets.len(), 4);

        let emb5 = vec![0.95, 0.05, 0.0];
        index.add_entry("key5", &emb5);
        assert_eq!(index.buckets.len(), 4);
    }

    #[test]
    fn test_approximate_index_assign_bucket() {
        let mut index = ApproximateIndex::new(3);
        index.add_entry("k1", &[1.0, 0.0, 0.0]);
        index.add_entry("k2", &[0.0, 1.0, 0.0]);
        index.add_entry("k3", &[0.0, 0.0, 1.0]);

        let bucket = index.assign_bucket(&[0.9, 0.1, 0.0]);
        assert_eq!(bucket, 0);

        let bucket = index.assign_bucket(&[0.1, 0.9, 0.0]);
        assert_eq!(bucket, 1);

        let bucket = index.assign_bucket(&[0.0, 0.1, 0.9]);
        assert_eq!(bucket, 2);
    }

    #[test]
    fn test_approximate_index_remove_entry() {
        let mut index = ApproximateIndex::new(4);
        index.add_entry("k1", &[1.0, 0.0]);
        index.add_entry("k2", &[0.0, 1.0]);

        index.remove_entry("k1");
        assert!(index.buckets[0].entry_keys.is_empty() || index.buckets[1].entry_keys.is_empty());
    }

    #[test]
    fn test_approximate_index_get_candidate_keys() {
        let mut index = ApproximateIndex::new(4);
        index.add_entry("k1", &[1.0, 0.0, 0.0]);
        index.add_entry("k2", &[0.0, 1.0, 0.0]);
        index.add_entry("k3", &[0.0, 0.0, 1.0]);
        index.add_entry("k4", &[0.99, 0.01, 0.0]);

        let candidates = index.get_candidate_keys(&[0.95, 0.05, 0.0], 2);
        assert!(!candidates.is_empty());
        assert!(candidates.contains(&"k1".to_string()) || candidates.contains(&"k4".to_string()));
    }

    #[test]
    fn test_approximate_index_clear() {
        let mut index = ApproximateIndex::new(4);
        index.add_entry("k1", &[1.0, 0.0]);
        index.add_entry("k2", &[0.0, 1.0]);
        assert_eq!(index.buckets.len(), 2);

        index.clear();
        assert_eq!(index.buckets.len(), 0);
    }

    #[test]
    fn test_approximate_index_empty_candidates() {
        let index = ApproximateIndex::new(4);
        let candidates = index.get_candidate_keys(&[1.0, 0.0], 3);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_cache_with_index_buckets() {
        let cache = SemanticCache::new(0.95, 100).with_index_buckets(8);
        assert_eq!(cache.index.num_buckets, 8);
    }

    #[test]
    fn test_cache_search_uses_index() {
        let mut cache = SemanticCache::new(0.95, 100).with_index_buckets(4);
        let emb = vec![0.5, 0.6, 0.7, 0.8, 0.9];
        cache.insert(emb.clone(), "q1".to_string(), "r1".to_string(), vec![], 0.9, 3600);

        let result = cache.search(&emb);
        assert!(result.is_some());
        assert_eq!(result.unwrap().query_text, "q1");
    }
}
