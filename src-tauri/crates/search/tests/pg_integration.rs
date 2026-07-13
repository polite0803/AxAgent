//! End-to-end pgvector validation for the PostgreSQL vector-store backend.
//!
//! This test is **env-gated**: it only runs when `AXAGENT_TEST_PG_URL` points at
//! a reachable pgvector database. On machines without PostgreSQL (or without the
//! env var) it returns immediately, so `cargo test -p axagent-search` stays
//! green everywhere and the SQLite path is always exercised.
//!
//! Run against a local Docker pgvector:
//!
//! ```bash
//! docker run --name axagent-pgvec -e POSTGRES_PASSWORD=secret \
//!   -p 5432:5432 -d pgvector/pgvector:pg17
//! # then create the database:
//! docker exec -i axagent-pgvec psql -U postgres -c "CREATE DATABASE axagent;"
//!
//! AXAGENT_TEST_PG_URL="postgres://postgres:secret@127.0.0.1:5432/axagent" \
//!   cargo test -p axagent-search --test pg_integration
//! ```
//!
//! Or point it at an existing pgvector instance (e.g. 192.168.0.235/236):
//!
//! ```bash
//! AXAGENT_TEST_PG_URL="postgres://postgres:secret@192.168.0.235:5432/axagent" \
//!   cargo test -p axagent-search --test pg_integration
//! ```
//!
//! The `vector` extension is created automatically (`CREATE EXTENSION vector`).

use axagent_search::hybrid_search::{HybridSearchOptions, HybridSearcher};
use axagent_search::vector_store::{EmbeddingRecord, VectorStore};
use sea_orm::{ConnectionTrait, Database};

#[tokio::test]
async fn pg_vector_and_hybrid_search_e2e() {
    let url = match std::env::var("AXAGENT_TEST_PG_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("AXAGENT_TEST_PG_URL not set — skipping pgvector e2e test");
            return;
        },
    };

    let db = Database::connect(&url)
        .await
        .expect("failed to connect to PostgreSQL (check AXAGENT_TEST_PG_URL)");

    db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS vector")
        .await
        .expect("failed to create pgvector extension");

    // Unique per-run collection so parallel/CI runs don't collide.
    let collection = format!("pg_itest_{}", std::process::id());
    let store = VectorStore::new(db.clone());
    let searcher = HybridSearcher::new(db.clone());

    // 4-dimensional embeddings for deterministic similarity.
    store.ensure_collection(&collection, 4).await.expect("ensure_collection failed");
    store.ensure_fts5_index(&collection).await.expect("ensure_fts5_index (GIN) failed");

    let records = vec![
        EmbeddingRecord {
            id: "c1".into(),
            document_id: "doc1".into(),
            chunk_index: 0,
            content: "rust programming language systems".into(),
            embedding: vec![1.0, 0.0, 0.0, 0.0],
        },
        EmbeddingRecord {
            id: "c2".into(),
            document_id: "doc1".into(),
            chunk_index: 1,
            content: "postgresql database vector search".into(),
            embedding: vec![0.0, 1.0, 0.0, 0.0],
        },
        EmbeddingRecord {
            id: "c3".into(),
            document_id: "doc2".into(),
            chunk_index: 0,
            content: "machine learning neural network training".into(),
            embedding: vec![0.0, 0.0, 1.0, 0.0],
        },
    ];

    store.upsert_embeddings(&collection, records).await.expect("upsert_embeddings failed");

    // ── Vector search: query near c2 should rank c2 first ──
    let vec_results = store
        .search(&collection, vec![0.05, 0.95, 0.0, 0.0], 3)
        .await
        .expect("vector search failed");
    assert!(!vec_results.is_empty(), "vector search returned nothing");
    assert_eq!(vec_results[0].id, "c2", "closest vector should be c2");

    // ── Hybrid (keyword) search: 'postgresql' must surface c2 ──
    let hybrid = searcher
        .hybrid_search(
            &collection,
            "postgresql database",
            vec![0.0, 1.0, 0.0, 0.0],
            HybridSearchOptions { top_k: 5, ..Default::default() },
        )
        .await
        .expect("hybrid search failed");
    assert!(!hybrid.is_empty(), "hybrid search returned nothing");
    assert!(
        hybrid.iter().any(|r| r.id == "c2"),
        "keyword 'postgresql' should match c2 (got {:?})",
        hybrid.iter().map(|r| &r.id).collect::<Vec<_>>()
    );

    // Best-effort cleanup so the database is not polluted across runs.
    let _ = store.delete_collection(&collection).await;
}
