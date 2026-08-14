// SPDX-License-Identifier: AGPL-3.0-only

//! 能力发现系统的真实嵌入提供者
//!
//! 能力发现系统（tools crate 的 CapabilityIndexer / CapabilityRetriever）通过
//! harness 层的 `EmbeddingProvider` trait 生成嵌入向量。早期实现使用
//! `MockEmbeddingProvider`（FNV 哈希伪随机向量，无语义含义）。
//!
//! 本模块在 **wiring 层**（src/）提供真实实现：内部复用 `indexing::generate_embeddings`
//! （与 RAG 知识库同源的嵌入链路），通过 DB 中的 embedding provider 配置 +
//! master_key + provider registry 调用真实 LLM 嵌入服务。
//!
//! # 架构定位
//! 能力发现系统作为"基座"（tools crate）不应持有具体 provider 配置（API key、
//! provider id 等），因此真实实现的依赖注入（db / master_key / registry /
//! embedding_provider 字符串）由 wiring 层完成，harness 与 tools 均不感知。

use std::sync::Arc;

use async_trait::async_trait;
use axagent_harness::rag_provider::EmbeddingProvider;
use axagent_harness::registry::ProviderRegistry;
use axagent_harness::types::ModelType;
use sea_orm::DatabaseConnection;

use crate::indexing::generate_embeddings;

/// 真实嵌入提供者
///
/// 复用 RAG 系统的 `generate_embeddings`（含 token 预算分片、过大自动二分、重试退避），
/// 保证任意长度的文本都能稳定生成指定维度的向量。
#[derive(Clone)]
pub struct CapabilityEmbeddingProvider {
    db: DatabaseConnection,
    master_key: [u8; 32],
    provider_registry: Arc<dyn ProviderRegistry>,
    /// `"providerId::model_id"` 格式的嵌入模型配置
    embedding_provider: String,
    /// 固定向量维度（由启动时探测得到，保证与既有向量库维度一致）
    dimensions: usize,
}

#[async_trait]
impl EmbeddingProvider for CapabilityEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let resp = generate_embeddings(
            &self.db,
            &self.master_key,
            &self.provider_registry,
            &self.embedding_provider,
            vec![text.to_string()],
            Some(self.dimensions),
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(resp.embeddings.into_iter().next().unwrap_or_default())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let resp = generate_embeddings(
            &self.db,
            &self.master_key,
            &self.provider_registry,
            &self.embedding_provider,
            texts.to_vec(),
            Some(self.dimensions),
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(resp.embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimensions
    }
}

/// 创建能力发现系统的嵌入提供者
///
/// 自动发现系统中第一个启用的 embedding 类型 provider + model，拼接成
/// `"providerId::model_id"` 并探测真实向量维度。未配置任何 embedding provider
/// （或探测失败）时回退 `MockEmbeddingProvider`，保证能力发现系统始终可用
/// （语义检索质量受限，启动时打 warning 提示）。
pub async fn create_capability_embedding_provider(
    sea_db: &DatabaseConnection,
    master_key: &[u8; 32],
    harness: &axagent_runtime::harness::RuntimeHarness,
) -> Arc<dyn EmbeddingProvider> {
    let registry = harness.provider_registry().clone();
    match discover_system_embedding_provider(sea_db, master_key, &registry).await {
        Some((embedding_provider, dimensions)) => {
            tracing::info!(
                "[capability] 使用真实嵌入服务 {}（维度 {}）",
                embedding_provider,
                dimensions
            );
            Arc::new(CapabilityEmbeddingProvider {
                db: sea_db.clone(),
                master_key: *master_key,
                provider_registry: registry,
                embedding_provider,
                dimensions,
            })
        },
        None => {
            tracing::warn!(
                "[capability] 未发现可用的 embedding provider，回退 MockEmbeddingProvider，语义检索质量受限"
            );
            Arc::new(axagent_tools::MockEmbeddingProvider::new(1536))
        },
    }
}

/// 从 providers 表发现第一个启用的 embedding provider + model，
/// 拼接 `"providerId::model_id"` 并通过一次探测调用确定真实向量维度。
///
/// 按顺序尝试：第一个 provider 探测失败（缺 key / 网络不可达）时继续尝试下一个，
/// 全部失败返回 `None`。
async fn discover_system_embedding_provider(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    provider_registry: &Arc<dyn ProviderRegistry>,
) -> Option<(String, usize)> {
    let providers = axagent_dao::repo::provider::list_providers_merged(db).await.ok()?;

    for provider in &providers {
        if !provider.enabled {
            continue;
        }
        let Some(model) =
            provider.models.iter().find(|m| m.model_type == ModelType::Embedding && m.enabled)
        else {
            // 当前 provider 无启用的 embedding 模型，尝试下一个
            continue;
        };
        let embedding_provider = format!("{}::{}", provider.id, model.model_id);

        // 传 None 让服务端返回默认维度，探测真实向量维度
        match generate_embeddings(
            db,
            master_key,
            provider_registry,
            &embedding_provider,
            vec!["capability-dimension-probe".to_string()],
            None,
        )
        .await
        {
            Ok(resp) if resp.dimensions > 0 => {
                return Some((embedding_provider, resp.dimensions));
            },
            Ok(_) => {
                tracing::warn!(
                    "[capability] embedding provider {} 探测无维度信息，尝试下一个",
                    embedding_provider
                );
            },
            Err(e) => {
                tracing::warn!(
                    "[capability] embedding provider {} 探测失败: {}",
                    embedding_provider,
                    e
                );
            },
        }
    }
    None
}
