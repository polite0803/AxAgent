// SPDX-License-Identifier: AGPL-3.0-only

//! axagent-dao — Marketplace / Review 服务
//!
//! SeaORM 数据访问实现。DTO 和 trait 契约在 `axagent_harness::marketplace` 中定义，
//! 本模块实现 `MarketplaceService` trait，通过 harness 暴露给上层。

use async_trait::async_trait;
use sea_orm::*;
use uuid::Uuid;

use axagent_entities::{workflow_marketplace, workflow_marketplace_review};
use axagent_harness::marketplace::{
    CreateReviewRequest, MarketplaceService as MarketplaceServiceTrait, MarketplaceStats,
    ReviewResponse, UpdateReviewRequest,
};

/// Default SeaORM-backed implementation of `MarketplaceService`.
pub struct MarketplaceServiceImpl;

#[async_trait]
impl MarketplaceServiceTrait for MarketplaceServiceImpl {
    async fn create_review(
        &self,
        db: &DatabaseConnection,
        req: CreateReviewRequest,
    ) -> Result<ReviewResponse, String> {
        if req.rating < 1 || req.rating > 5 {
            return Err("Rating must be between 1 and 5".to_string());
        }

        let now = chrono::Utc::now().timestamp();

        let review = workflow_marketplace_review::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            marketplace_id: Set(req.marketplace_id.clone()),
            user_id: Set(req.user_id),
            rating: Set(req.rating),
            comment: Set(req.comment),
            is_hidden: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = review.insert(db).await.map_err(|e| e.to_string())?;

        self.update_marketplace_rating(db, &req.marketplace_id)
            .await?;

        Ok(model_to_response(result))
    }

    async fn get_reviews(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
    ) -> Result<Vec<ReviewResponse>, String> {
        let reviews = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::MarketplaceId.eq(marketplace_id))
            .filter(workflow_marketplace_review::Column::IsHidden.eq(false))
            .order_by_desc(workflow_marketplace_review::Column::CreatedAt)
            .all(db)
            .await
            .map_err(|e| e.to_string())?;

        Ok(reviews.into_iter().map(model_to_response).collect())
    }

    async fn get_user_review(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
        user_id: &str,
    ) -> Result<Option<ReviewResponse>, String> {
        let review = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::MarketplaceId.eq(marketplace_id))
            .filter(workflow_marketplace_review::Column::UserId.eq(user_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        Ok(review.map(model_to_response))
    }

    async fn update_review(
        &self,
        db: &DatabaseConnection,
        review_id: &str,
        req: UpdateReviewRequest,
    ) -> Result<ReviewResponse, String> {
        if let Some(rating) = req.rating
            && !(1..=5).contains(&rating)
        {
            return Err("Rating must be between 1 and 5".to_string());
        }

        let review = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::Id.eq(review_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let marketplace_id = review.marketplace_id.clone();

        let mut active_model: workflow_marketplace_review::ActiveModel = review.into();
        if let Some(rating) = req.rating {
            active_model.rating = Set(rating);
        }
        if let Some(comment) = req.comment {
            active_model.comment = Set(Some(comment));
        }
        active_model.updated_at = Set(chrono::Utc::now().timestamp());

        let result = active_model.update(db).await.map_err(|e| e.to_string())?;

        self.update_marketplace_rating(db, &marketplace_id).await?;

        Ok(model_to_response(result))
    }

    async fn delete_review(&self, db: &DatabaseConnection, review_id: &str) -> Result<(), String> {
        let review = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::Id.eq(review_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let marketplace_id = review.marketplace_id.clone();

        let active_model: workflow_marketplace_review::ActiveModel = review.into();
        active_model.delete(db).await.map_err(|e| e.to_string())?;

        self.update_marketplace_rating(db, &marketplace_id).await?;

        Ok(())
    }

    async fn get_stats(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
    ) -> Result<MarketplaceStats, String> {
        let reviews = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::MarketplaceId.eq(marketplace_id))
            .filter(workflow_marketplace_review::Column::IsHidden.eq(false))
            .all(db)
            .await
            .map_err(|e| e.to_string())?;

        let total_reviews = reviews.len() as i32;
        let rating_average = if total_reviews > 0 {
            let sum: i32 = reviews.iter().map(|r| r.rating).sum();
            sum as f64 / total_reviews as f64
        } else {
            0.0
        };

        Ok(MarketplaceStats {
            marketplace_id: marketplace_id.to_string(),
            total_reviews,
            rating_average,
        })
    }

    async fn get_marketplace_id_for_review(
        &self,
        db: &DatabaseConnection,
        review_id: &str,
    ) -> Result<String, String> {
        let review = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::Id.eq(review_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        Ok(review.marketplace_id)
    }
}

// ── Private helpers ──

fn model_to_response(model: workflow_marketplace_review::Model) -> ReviewResponse {
    ReviewResponse {
        id: model.id,
        marketplace_id: model.marketplace_id,
        user_id: model.user_id,
        rating: model.rating,
        comment: model.comment,
        created_at: model.created_at,
    }
}

impl MarketplaceServiceImpl {
    async fn update_marketplace_rating(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
    ) -> Result<(), String> {
        let stats = <Self as MarketplaceServiceTrait>::get_stats(self, db, marketplace_id).await?;

        let marketplace = workflow_marketplace::Entity::find()
            .filter(workflow_marketplace::Column::Id.eq(marketplace_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Marketplace not found".to_string())?;

        let mut active_model: workflow_marketplace::ActiveModel = marketplace.into();
        active_model.rating_average = Set(stats.rating_average);
        active_model.rating_count = Set(stats.total_reviews);
        active_model.updated_at = Set(chrono::Utc::now().timestamp());

        active_model.update(db).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}

// ── Backward compatibility: type alias for old `MarketplaceService` struct name ──
/// Deprecated alias — use `MarketplaceServiceImpl` directly or the
/// `axagent_harness::marketplace::MarketplaceService` trait.
pub type MarketplaceService = MarketplaceServiceImpl;
