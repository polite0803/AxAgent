//! 缓存拦截器契约 — Harness 层缓存抽象
//!
//! 提供 `HarnessCache` trait 和 `LlmCacheKey` 数据结构，
//! 下游 crate 可注入具体实现（如 Redis、内存、磁盘缓存）。

use serde::{Deserialize, Serialize};

/// LLM 请求缓存键
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCacheKey {
    pub model: String,
    pub messages_hash: String,
    pub temperature: Option<f64>,
}

/// Harness 缓存的 trait 抽象
#[async_trait::async_trait]
pub trait HarnessCache: Send + Sync {
    async fn get(&self, key: &LlmCacheKey) -> Option<serde_json::Value>;
    async fn set(&self, key: LlmCacheKey, value: serde_json::Value, ttl_secs: u64);
    async fn invalidate(&self, key: &LlmCacheKey);
}

/// 可序列化的单条向量搜索缓存条目（用于持久化快照）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntrySnapshot {
    pub key: String,
    pub results_json: String,
}
