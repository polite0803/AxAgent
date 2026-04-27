use sea_orm::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::{workflow_marketplace, workflow_marketplace_review};

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

impl From<workflow_marketplace_review::Model> for ReviewResponse {
    fn from(model: workflow_marketplace_review::Model) -> Self {
        Self {
            id: model.id,
            marketplace_id: model.marketplace_id,
            user_id: model.user_id,
            rating: model.rating,
            comment: model.comment,
            created_at: model.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStats {
    pub marketplace_id: String,
    pub total_reviews: i32,
    pub rating_average: f64,
}

pub struct MarketplaceService;

impl MarketplaceService {
    pub async fn create_review(
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

        Self::update_marketplace_rating(db, &req.marketplace_id).await?;

        Ok(ReviewResponse::from(result))
    }

    pub async fn get_reviews(
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

        Ok(reviews.into_iter().map(ReviewResponse::from).collect())
    }

    pub async fn get_user_review(
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

        Ok(review.map(ReviewResponse::from))
    }

    pub async fn update_review(
        db: &DatabaseConnection,
        review_id: &str,
        req: UpdateReviewRequest,
    ) -> Result<ReviewResponse, String> {
        if let Some(rating) = req.rating {
            if rating < 1 || rating > 5 {
                return Err("Rating must be between 1 and 5".to_string());
            }
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

        Self::update_marketplace_rating(db, &marketplace_id).await?;

        Ok(ReviewResponse::from(result))
    }

    pub async fn delete_review(db: &DatabaseConnection, review_id: &str) -> Result<(), String> {
        let review = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::Id.eq(review_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let marketplace_id = review.marketplace_id.clone();

        let active_model: workflow_marketplace_review::ActiveModel = review.into();
        active_model.delete(db).await.map_err(|e| e.to_string())?;

        Self::update_marketplace_rating(db, &marketplace_id).await?;

        Ok(())
    }

    pub async fn get_stats(
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

    async fn update_marketplace_rating(
        db: &DatabaseConnection,
        marketplace_id: &str,
    ) -> Result<(), String> {
        let stats = Self::get_stats(db, marketplace_id).await?;

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
