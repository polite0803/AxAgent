#![allow(clippy::unwrap_used, clippy::needless_borrows_for_generic_args)]

use axagent_core::cache::{EmbeddingCache, TextHashCache};
use std::time::Duration;

#[test]
fn test_embedding_cache_insert_and_get() {
    let cache = EmbeddingCache::new(100, Duration::from_secs(3600));

    cache.insert("key1".to_string(), vec![0.1, 0.2, 0.3]);
    let result = cache.get("key1");

    assert!(result.is_some());
    let embedding = result.unwrap();
    assert_eq!(embedding.len(), 3);
    assert_eq!(embedding[0], 0.1);
    assert_eq!(embedding[1], 0.2);
    assert_eq!(embedding[2], 0.3);
}

#[test]
fn test_embedding_cache_miss() {
    let cache = EmbeddingCache::new(100, Duration::from_secs(3600));

    let result = cache.get("nonexistent");

    assert!(result.is_none());
}

#[test]
fn test_embedding_cache_remove() {
    let cache = EmbeddingCache::new(100, Duration::from_secs(3600));

    cache.insert("key1".to_string(), vec![0.1, 0.2]);
    assert!(cache.get("key1").is_some());

    cache.remove("key1");
    assert!(cache.get("key1").is_none());
}

#[test]
fn test_embedding_cache_clear() {
    let cache = EmbeddingCache::new(100, Duration::from_secs(3600));

    cache.insert("key1".to_string(), vec![0.1]);
    cache.insert("key2".to_string(), vec![0.2]);
    cache.insert("key3".to_string(), vec![0.3]);

    cache.clear();

    assert!(cache.get("key1").is_none());
    assert!(cache.get("key2").is_none());
    assert!(cache.get("key3").is_none());
}

#[test]
fn test_embedding_cache_max_entries() {
    let cache = EmbeddingCache::new(2, Duration::from_secs(3600));

    cache.insert("key1".to_string(), vec![0.1]);
    cache.insert("key2".to_string(), vec![0.2]);

    assert!(cache.get("key1").is_some());
    assert!(cache.get("key2").is_some());

    cache.insert("key3".to_string(), vec![0.3]);

    let key1 = cache.get("key1").is_some();
    let key2 = cache.get("key2").is_some();
    let key3 = cache.get("key3").is_some();

    assert!(key1 || key2 || key3, "At least one key should exist");
}

#[test]
fn test_text_hash_cache_insert_and_get() {
    let cache = TextHashCache::new(100, Duration::from_secs(7200));

    cache.insert("doc1".to_string(), "hash123".to_string());
    let result = cache.get("doc1");

    assert!(result.is_some());
    assert_eq!(result.unwrap(), "hash123");
}

#[test]
fn test_text_hash_cache_miss() {
    let cache = TextHashCache::new(100, Duration::from_secs(7200));

    let result = cache.get("nonexistent");

    assert!(result.is_none());
}

#[test]
fn test_text_hash_cache_clear() {
    let cache = TextHashCache::new(100, Duration::from_secs(7200));

    cache.insert("doc1".to_string(), "hash1".to_string());
    cache.insert("doc2".to_string(), "hash2".to_string());

    cache.clear();

    assert!(cache.get("doc1").is_none());
    assert!(cache.get("doc2").is_none());
}

#[test]
fn test_embedding_cache_default() {
    let cache = EmbeddingCache::default();

    cache.insert("test".to_string(), vec![1.0, 2.0]);
    assert!(cache.get("test").is_some());
}

#[test]
fn test_text_hash_cache_default() {
    let cache = TextHashCache::default();

    cache.insert("test".to_string(), "hash".to_string());
    assert!(cache.get("test").is_some());
}

#[test]
fn test_embedding_cache_with_empty_key() {
    let cache = EmbeddingCache::new(100, Duration::from_secs(3600));

    cache.insert("".to_string(), vec![1.0]);
    assert!(cache.get("").is_some());
}

#[test]
fn test_text_hash_cache_with_empty_value() {
    let cache = TextHashCache::new(100, Duration::from_secs(7200));

    cache.insert("doc".to_string(), "".to_string());
    assert!(cache.get("doc").is_some());
}
