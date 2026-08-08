// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use async_trait::async_trait;

use axagent_entities::opc_content_assets;

use super::error::OpcResult;

/// 内容资产 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAsset {
    pub id: String,
    pub title: String,
    pub content_type: String,
    pub body: String,
    pub tags: Vec<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContentAssetInput {
    pub title: String,
    pub content_type: String,
    pub body: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContentAssetInput {
    pub title: Option<String>,
    pub content_type: Option<String>,
    pub body: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
}

// ── Entity ↔ DTO 转换 ──────────────────────────────────────────

pub(crate) fn content_asset_entity_to_dto(e: opc_content_assets::Model) -> ContentAsset {
    let tags: Vec<String> = serde_json::from_str(&e.tags_json).unwrap_or_default();
    ContentAsset {
        id: e.id,
        title: e.title,
        content_type: e.content_type,
        body: e.body,
        tags,
        status: e.status,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}

// ── ContentAssetService trait ─────────────────────────────────────

/// ContentAsset 相关方法
#[async_trait]
pub trait ContentAssetService: Send + Sync {
    async fn create_content_asset(&self, input: CreateContentAssetInput)
        -> OpcResult<ContentAsset>;
    async fn get_content_asset(&self, id: &str) -> OpcResult<ContentAsset>;
    async fn list_content_assets(&self) -> OpcResult<Vec<ContentAsset>>;
    async fn update_content_asset(
        &self,
        id: &str,
        input: UpdateContentAssetInput,
    ) -> OpcResult<ContentAsset>;
    async fn delete_content_asset(&self, id: &str) -> OpcResult<()>;
}
