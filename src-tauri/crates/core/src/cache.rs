//! Caching utilities for AxAgent core functionality.
//!
//! This module provides thread-safe in-memory caches for embedding vectors
//! and text hashes to reduce redundant computation and improve performance.
//!
//! # Architecture
//!
//! - [`EmbeddingCache`]: Cache for storing pre-computed embedding vectors
//! - [`TextHashCache`]: Cache for storing text hash values for deduplication
//!
//! Both caches use `quick_cache` under the hood for lock-free concurrent access.

use quick_cache::sync::Cache;
use std::time::Duration;

/// Thread-safe cache for storing pre-computed embedding vectors.
///
/// Embeddings are typically expensive to compute (involving neural network inference).
/// This cache stores recently computed embeddings to avoid redundant computation.
///
/// # Type Parameters
///
/// - `String`: The cache key (typically a hash of the input text)
/// - `Vec<f32>`: The embedding vector
///
/// # Example
///
/// ```
/// use axagent_core::cache::EmbeddingCache;
///
/// let cache = EmbeddingCache::new(1000, Duration::from_secs(3600));
/// cache.insert("text_hash".to_string(), vec![0.1, 0.2, 0.3]);
///
/// if let Some(embedding) = cache.get("text_hash") {
///     println!("Found embedding with {} dimensions", embedding.len());
/// }
/// ```
pub struct EmbeddingCache {
    cache: Cache<String, Vec<f32>>,
}

impl EmbeddingCache {
    /// Creates a new embedding cache with the specified maximum entries and TTL.
    ///
    /// # Parameters
    ///
    /// - `max_entries`: Maximum number of embeddings to store
    /// - `_ttl`: Time-to-live duration (currently unused, reserved for future)
    pub fn new(max_entries: usize, _ttl: Duration) -> Self {
        Self {
            cache: Cache::new(max_entries),
        }
    }

    /// Retrieves an embedding from the cache by key.
    ///
    /// # Parameters
    ///
    /// - `key`: The cache key to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(Vec<f32>)` if found, `None` otherwise.
    pub fn get(&self, key: &str) -> Option<Vec<f32>> {
        self.cache.get(key)
    }

    /// Inserts an embedding into the cache.
    ///
    /// # Parameters
    ///
    /// - `key`: The cache key (typically a hash of the input)
    /// - `value`: The embedding vector to store
    pub fn insert(&self, key: String, value: Vec<f32>) {
        self.cache.insert(key, value);
    }

    /// Removes an entry from the cache by key.
    ///
    /// # Parameters
    ///
    /// - `key`: The cache key to remove
    pub fn remove(&self, key: &str) {
        self.cache.remove(key);
    }

    /// Clears all entries from the cache.
    pub fn clear(&self) {
        self.cache.clear();
    }
}

impl Default for EmbeddingCache {
    fn default() -> Self {
        Self::new(1000, Duration::from_secs(3600))
    }
}

/// Thread-safe cache for storing text hash values.
///
/// Text hashes are used for deduplication and change detection.
/// This cache avoids recomputing hashes for unchanged text.
///
/// # Example
///
/// ```
/// use axagent_core::cache::TextHashCache;
///
/// let cache = TextHashCache::new(500, Duration::from_secs(7200));
/// cache.insert("document_id".to_string(), "hash_value".to_string());
///
/// if let Some(hash) = cache.get("document_id") {
///     println!("Found hash: {}", hash);
/// }
/// ```
pub struct TextHashCache {
    cache: Cache<String, String>,
}

impl TextHashCache {
    /// Creates a new text hash cache with the specified maximum entries and TTL.
    ///
    /// # Parameters
    ///
    /// - `max_entries`: Maximum number of hashes to store
    /// - `_ttl`: Time-to-live duration (currently unused, reserved for future)
    pub fn new(max_entries: usize, _ttl: Duration) -> Self {
        Self {
            cache: Cache::new(max_entries),
        }
    }

    /// Retrieves a text hash from the cache by key.
    ///
    /// # Parameters
    ///
    /// - `key`: The cache key to look up
    ///
    /// # Returns
    ///
    /// Returns `Some(String)` if found, `None` otherwise.
    pub fn get(&self, key: &str) -> Option<String> {
        self.cache.get(key)
    }

    /// Inserts a text hash into the cache.
    ///
    /// # Parameters
    ///
    /// - `key`: The cache key (typically a document ID)
    /// - `value`: The hash value to store
    pub fn insert(&self, key: String, value: String) {
        self.cache.insert(key, value);
    }

    /// Clears all entries from the cache.
    pub fn clear(&self) {
        self.cache.clear();
    }
}

impl Default for TextHashCache {
    fn default() -> Self {
        Self::new(500, Duration::from_secs(7200))
    }
}
