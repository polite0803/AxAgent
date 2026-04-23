#![allow(clippy::unwrap_used)]

use axagent_core::vector_store::{EmbeddingRecord, HnswConfig, VectorSearchResult};

#[test]
fn test_hnsw_config_default() {
    let config = HnswConfig::default();

    assert_eq!(config.ef_construction, 100);
    assert_eq!(config.m, 16);
    assert_eq!(config.ef_search, 50);
}

#[test]
fn test_hnsw_config_custom() {
    let config = HnswConfig {
        ef_construction: 200,
        m: 32,
        ef_search: 100,
    };

    assert_eq!(config.ef_construction, 200);
    assert_eq!(config.m, 32);
    assert_eq!(config.ef_search, 100);
}

#[test]
fn test_hnsw_config_small_collection() {
    let config = HnswConfig {
        ef_construction: 100,
        m: 12,
        ef_search: 50,
    };

    assert!(config.ef_construction <= 100);
    assert!(config.m <= 16);
}

#[test]
fn test_hnsw_config_large_collection() {
    let config = HnswConfig {
        ef_construction: 200,
        m: 16,
        ef_search: 100,
    };

    assert!(config.ef_construction >= 200);
    assert!(config.m >= 16);
    assert!(config.ef_search >= 100);
}

#[test]
fn test_embedding_record_creation() {
    let record = EmbeddingRecord {
        id: "chunk_1".to_string(),
        document_id: "doc_1".to_string(),
        chunk_index: 0,
        content: "Test content".to_string(),
        embedding: vec![0.1, 0.2, 0.3, 0.4],
    };

    assert_eq!(record.id, "chunk_1");
    assert_eq!(record.document_id, "doc_1");
    assert_eq!(record.chunk_index, 0);
    assert_eq!(record.content, "Test content");
    assert_eq!(record.embedding.len(), 4);
}

#[test]
fn test_embedding_record_with_high_dimensions() {
    let dims = 1536;
    let embedding: Vec<f32> = (0..dims).map(|i| i as f32 * 0.01).collect();

    let record = EmbeddingRecord {
        id: "high_dim_chunk".to_string(),
        document_id: "doc_1".to_string(),
        chunk_index: 5,
        content: "High dimensional content".to_string(),
        embedding,
    };

    assert_eq!(record.embedding.len(), 1536);
    assert!((record.embedding[100] - 1.0).abs() < 0.01);
}

#[test]
fn test_vector_search_result_creation() {
    let result = VectorSearchResult {
        id: "result_1".to_string(),
        document_id: "doc_1".to_string(),
        chunk_index: 0,
        content: "Search result content".to_string(),
        score: 0.95,
        has_embedding: true,
    };

    assert_eq!(result.id, "result_1");
    assert_eq!(result.score, 0.95);
    assert!(result.has_embedding);
}

#[test]
fn test_vector_search_result_serialization() {
    let result = VectorSearchResult {
        id: "result_1".to_string(),
        document_id: "doc_1".to_string(),
        chunk_index: 0,
        content: "Test content".to_string(),
        score: 0.5,
        has_embedding: false,
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: VectorSearchResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, result.id);
    assert_eq!(deserialized.score, result.score);
}

#[test]
fn test_hnsw_config_debug_trait() {
    let config = HnswConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("HnswConfig"));
    assert!(debug_str.contains("ef_construction"));
    assert!(debug_str.contains("m"));
    assert!(debug_str.contains("ef_search"));
}

#[test]
fn test_hnsw_config_clone() {
    let config = HnswConfig::default();
    let cloned = config.clone();

    assert_eq!(cloned.ef_construction, config.ef_construction);
    assert_eq!(cloned.m, config.m);
    assert_eq!(cloned.ef_search, config.ef_search);
}
