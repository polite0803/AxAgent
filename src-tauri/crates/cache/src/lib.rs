//! axagent-cache — 热缓存层
//!
//! 内存缓存（EmbeddingCache, TextHashCache）及其持久化（快照/恢复）。

pub mod cache;
pub mod cache_persister;
pub mod cache_snapshot;
