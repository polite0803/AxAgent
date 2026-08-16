// SPDX-License-Identifier: AGPL-3.0-only
//! 能力索引层 — 将 CapabilityPassport 向量化 + 标签化存入本地向量库
//!
//! # 索引策略
//! 1. 正向索引：description + tags 拼接后向量化
//! 2. 负向索引：negative_scenarios 逐条向量化，标记 is_negative = true
//! 3. 元数据存储：完整 CapabilityPassportDto 作为 metadata 挂载

use crate::capability::CapabilityPassportDto;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 索引操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexResult {
    pub capability_id: String,
    pub success: bool,
    pub vector_dimensions: usize,
    pub indexed_at_ms: u64,
    pub error: Option<String>,
}

/// 能力索引器 — 负责将能力护照写入向量索引
///
/// # 实现位置
/// harness 层定义 trait，由 tools crate 实现
#[async_trait]
pub trait CapabilityIndexer: Send + Sync {
    /// 索引单个能力护照
    ///
    /// 将 description + tags 拼接嵌入为正向向量，
    /// negative_scenarios 分别嵌入为负向向量。
    async fn index_passport(&self, passport: &CapabilityPassportDto)
    -> Result<IndexResult, String>;

    /// 批量索引能力护照
    async fn index_batch(&self, passports: &[CapabilityPassportDto]) -> Vec<IndexResult>;

    /// 删除指定能力的索引（正向 + 负向）
    async fn remove_index(&self, capability_id: &str) -> Result<(), String>;

    /// 清空整个能力索引
    async fn clear_all(&self) -> Result<(), String>;

    /// 获取索引统计
    async fn get_stats(&self) -> Result<CapabilityIndexStats, String>;

    /// 列出所有已索引的能力 ID
    async fn list_capability_ids(&self) -> Vec<String>;

    /// 根据 ID 获取能力护照
    async fn get_passport(&self, capability_id: &str) -> Option<CapabilityPassportDto>;

    /// 一次性获取所有已索引的能力护照
    ///
    /// 默认实现遍历 `list_capability_ids` + `get_passport`（N 次读锁），
    /// 实现方可重写为单次读锁批量返回，避免元数据过滤时逐个 await 造成的性能瓶颈。
    async fn list_passports(&self) -> Vec<CapabilityPassportDto> {
        let ids = self.list_capability_ids().await;
        let mut passports = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(passport) = self.get_passport(&id).await {
                passports.push(passport);
            }
        }
        passports
    }
}

/// 索引统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityIndexStats {
    pub total_capabilities: u64,
    pub total_vectors: u64,
    pub positive_vectors: u64,
    pub negative_vectors: u64,
    pub last_indexed_at: Option<u64>,
}

/// 能力集合名称（传给 VectorStoreProvider 的 collection 参数）
pub const CAPABILITY_COLLECTION: &str = "capabilities";
pub const CAPABILITY_NEGATIVE_COLLECTION: &str = "capabilities_negative";
