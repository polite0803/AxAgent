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
pub struct CreateReviewRequest {
    pub marketplace_id: String,
    pub user_id: String,
    pub rating: i32,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReviewRequest {
    pub rating: Option<i32>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResponse {
    pub id: String,
    pub marketplace_id: String,
    pub user_id: String,
    pub rating: i32,
    pub comment: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
