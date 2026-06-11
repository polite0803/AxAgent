// SPDX-License-Identifier: AGPL-3.0-only

//! axagent-search — 搜索与 RAG 引擎
//!
//! 包含向量搜索、混合搜索、文件索引、语义缓存、AST 索引、
//! Self-RAG 质量门控、查询增强等模块。

pub mod ast_index;
pub mod file_index;
pub mod hybrid_search;
pub mod incremental_indexer;
pub mod inference;
pub mod model_downloader;
pub mod query_enhancement;
pub mod rag;
pub mod rag_pipeline;
pub mod recall_pipeline;
pub mod reranker;
pub mod search;
pub mod self_rag;
pub mod semantic_cache;
pub mod sources;
pub mod text_chunker;
pub mod vector_cache;
pub mod vector_store;
