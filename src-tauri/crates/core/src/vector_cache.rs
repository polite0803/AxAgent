use crate::vector_store::VectorSearchResult;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
struct CacheEntry {
    results: Vec<VectorSearchResult>,
    timestamp: Instant,
    query_hash: u64,
}

pub struct VectorSearchCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_entries: usize,
    ttl: Duration,
    hits: Arc<std::sync::atomic::AtomicU64>,
    misses: Arc<std::sync::atomic::AtomicU64>,
}

impl VectorSearchCache {
    pub fn new(max_entries: usize, ttl_secs: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            ttl: Duration::from_secs(ttl_secs),
            hits: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            misses: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn get(&self, key: &str, query_hash: u64) -> Option<Vec<VectorSearchResult>> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if entry.timestamp.elapsed() < self.ttl && entry.query_hash == query_hash {
                self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Some(entry.results.clone());
            }
        }
        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }

    pub async fn insert(&self, key: String, query_hash: u64, results: Vec<VectorSearchResult>) {
        let mut cache = self.cache.write().await;

        if cache.len() >= self.max_entries && !cache.contains_key(&key) {
            self.evict_oldest(&mut cache).await;
        }

        cache.insert(
            key,
            CacheEntry {
                results,
                timestamp: Instant::now(),
                query_hash,
            },
        );
    }

    async fn evict_oldest(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some((oldest_key, _)) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.timestamp)
            .map(|(k, v)| (k.clone(), v.timestamp))
        {
            cache.remove(&oldest_key);
        }
    }

    pub async fn invalidate(&self, key: &str) {
        let mut cache = self.cache.write().await;
        cache.retain(|k, _| k != key);
    }

    pub async fn invalidate_all(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.misses.load(std::sync::atomic::Ordering::Relaxed),
            size: self.cache.blocking_read().len(),
            max_entries: self.max_entries,
        }
    }

    pub fn compute_query_hash(knowledge_base_id: &str, query_embedding: &[f32], top_k: usize) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        knowledge_base_id.hash(&mut hasher);
        for &v in query_embedding.iter().take(10) {
            v.to_bits().hash(&mut hasher);
        }
        top_k.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    pub max_entries: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Serializable representation of a single vector search cache entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntrySnapshot {
    pub key: String,
    pub results_json: String,
}

impl VectorSearchCache {
    /// Export all currently valid cache entries for persistence.
    ///
    /// Returns entries whose TTL has not yet expired, serialized as
    /// `CacheEntrySnapshot` values that can be included in a
    /// `CacheSnapshot`.
    pub async fn export_snapshot(&self) -> Vec<CacheEntrySnapshot> {
        let cache = self.cache.read().await;
        cache
            .iter()
            .filter(|(_, entry)| entry.timestamp.elapsed() < self.ttl)
            .map(|(key, entry)| {
                let results_json =
                    serde_json::to_string(&entry.results).unwrap_or_default();
                CacheEntrySnapshot {
                    key: key.clone(),
                    results_json,
                }
            })
            .collect()
    }

    /// Restore cached entries from a persisted snapshot.
    ///
    /// Entry timestamps are reset to `Instant::now()` so TTL starts fresh
    /// after application restart.
    pub async fn restore_from_snapshot(&self, entries: Vec<CacheEntrySnapshot>) {
        let mut cache = self.cache.write().await;
        for entry in entries {
            let results: Vec<crate::vector_store::VectorSearchResult> =
                match serde_json::from_str(&entry.results_json) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
            cache.insert(
                entry.key,
                CacheEntry {
                    results,
                    timestamp: Instant::now(),
                    query_hash: 0,
                },
            );
        }
    }
}

impl Default for VectorSearchCache {
    fn default() -> Self {
        Self::new(1000, 300)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache = VectorSearchCache::new(10, 60);
        let results = vec![VectorSearchResult {
            id: "test".to_string(),
            document_id: "doc1".to_string(),
            chunk_index: 0,
            content: "test content".to_string(),
            score: 0.5,
            has_embedding: true,
        }];

        let hash = VectorSearchCache::compute_query_hash("kb1", &[0.1, 0.2], 5);
        cache.insert("kb1".to_string(), hash, results.clone()).await;

        let cached = cache.get("kb1", hash).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let cache = VectorSearchCache::new(10, 0);
        let results = vec![VectorSearchResult {
            id: "test".to_string(),
            document_id: "doc1".to_string(),
            chunk_index: 0,
            content: "test content".to_string(),
            score: 0.5,
            has_embedding: true,
        }];

        let hash = VectorSearchCache::compute_query_hash("kb1", &[0.1, 0.2], 5);
        cache.insert("kb1".to_string(), hash, results).await;

        tokio::time::sleep(Duration::from_millis(10)).await;

        let cached = cache.get("kb1", hash).await;
        assert!(cached.is_none());
    }
}
