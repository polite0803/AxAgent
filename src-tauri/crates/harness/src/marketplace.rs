// SPDX-License-Identifier: AGPL-3.0-only

//! Marketplace service trait and DTOs.
//!
//! Defines the contract for marketplace review CRUD operations,
//! allowing upper layers (gateway) to depend on harness rather than
//! dao/entities/kit directly.

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReviewRequest {
    pub marketplace_id: String,
    pub user_id: String,
    pub rating: i32,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReviewRequest {
    pub rating: Option<i32>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResponse {
    pub id: String,
    pub marketplace_id: String,
    pub user_id: String,
    pub rating: i32,
    pub comment: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceStats {
    pub marketplace_id: String,
    pub total_reviews: i32,
    pub rating_average: f64,
}

/// Marketplace review service contract.
///
/// Implemented by `axagent-dao` using SeaORM, injected into gateway
/// via `GatewayAppState`.
#[async_trait]
pub trait MarketplaceService: Send + Sync {
    async fn create_review(
        &self,
        db: &DatabaseConnection,
        req: CreateReviewRequest,
    ) -> Result<ReviewResponse, String>;

    async fn get_reviews(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
    ) -> Result<Vec<ReviewResponse>, String>;

    async fn get_user_review(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
        user_id: &str,
    ) -> Result<Option<ReviewResponse>, String>;

    async fn update_review(
        &self,
        db: &DatabaseConnection,
        review_id: &str,
        req: UpdateReviewRequest,
    ) -> Result<ReviewResponse, String>;

    async fn delete_review(&self, db: &DatabaseConnection, review_id: &str) -> Result<(), String>;

    async fn get_stats(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
    ) -> Result<MarketplaceStats, String>;

    /// Resolve a review_id to its parent marketplace_id.
    ///
    /// Replaces direct `workflow_marketplace_review::Entity::find()` calls
    /// in upper layers, eliminating the gateway→entities dependency.
    async fn get_marketplace_id_for_review(
        &self,
        db: &DatabaseConnection,
        review_id: &str,
    ) -> Result<String, String>;
}

// ── 目录浏览/搜索/安装/发布契约 ─────────────────────────────────────
//
// 单用户桌面场景下「市场」= 本地已导入的工作流模板 + 本地技能目录的统一浏览。
// 与评论/评分 CRUD（上方 `MarketplaceService`）正交，故定义为独立 trait。
// 实现位于 `axagent-dao::marketplace_service::MarketplaceCatalogServiceImpl`。

/// 目录条目类型，取值为 `"workflow_template"` 或 `"skill"`。
pub const CATALOG_ITEM_TYPE_TEMPLATE: &str = "workflow_template";
/// 目录条目类型——技能。
pub const CATALOG_ITEM_TYPE_SKILL: &str = "skill";

/// 市场目录条目 DTO。
///
/// 同时承载工作流模板与本地技能的元数据，前端按 `item_type` 分流展示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub id: String,
    /// `"workflow_template"` 或 `"skill"`，参考 `CATALOG_ITEM_TYPE_*` 常量
    pub item_type: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub version: Option<String>,
    /// 是否已安装到本地（模板：是否在 marketplace 表中标记 is_public；技能：恒为 true）
    pub installed: bool,
    pub rating_average: Option<f64>,
    pub download_count: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 市场目录查询参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CatalogQuery {
    /// 关键词：在 name/description/tags 中模糊匹配
    pub keyword: Option<String>,
    /// 分类过滤
    pub category: Option<String>,
    /// 类型过滤：`workflow_template` 或 `skill`，None 表示两者都返回
    pub item_type: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// 市场目录分页结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPage {
    pub items: Vec<CatalogItem>,
    pub total: u64,
    pub offset: u32,
    pub limit: u32,
}

/// 市场目录服务契约。
///
/// 由 `axagent-dao` 实现，命令层 (`src/commands/marketplace.rs`) 通过
/// `AppState` 拿到 db 连接后调用本 trait。
#[async_trait]
pub trait MarketplaceCatalogService: Send + Sync {
    /// 浏览市场目录：合并 `workflow_marketplace` 表与本地 skills 目录。
    async fn list_catalog(
        &self,
        db: &DatabaseConnection,
        query: CatalogQuery,
    ) -> Result<CatalogPage, String>;

    /// 按 ID + 类型查单个目录条目。
    async fn get_catalog_item(
        &self,
        db: &DatabaseConnection,
        item_id: &str,
        item_type: &str,
    ) -> Result<Option<CatalogItem>, String>;

    /// 安装模板：在 `workflow_marketplace` 表 upsert 记录并置 `is_public = true`。
    async fn install_template(
        &self,
        db: &DatabaseConnection,
        template_id: &str,
    ) -> Result<(), String>;

    /// 卸载模板：置 `workflow_marketplace.is_public = false`。
    async fn uninstall_template(
        &self,
        db: &DatabaseConnection,
        template_id: &str,
    ) -> Result<(), String>;

    /// 发布模板：更新 `workflow_marketplace` 表的 `category` 和 `tags` 字段。
    async fn publish_template(
        &self,
        db: &DatabaseConnection,
        template_id: &str,
        category: Option<String>,
        tags: Vec<String>,
    ) -> Result<(), String>;
}
