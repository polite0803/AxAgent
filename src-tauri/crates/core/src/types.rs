//! 共享类型定义
//!
//! 所有类型由 `axagent-harness` 提供，本模块仅做 re-export。

pub use axagent_harness::types::*;

// NoteSearchResult / RAGPipelineConfig / Note 等类型由 harness::rag_config 提供。
// DAO Note → Harness Note 的 From 转换定义在 axagent-dao::repo::note 中。
