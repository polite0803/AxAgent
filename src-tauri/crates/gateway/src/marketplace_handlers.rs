// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use axagent_harness::marketplace::{CreateReviewRequest, UpdateReviewRequest};

use crate::auth::AuthenticatedKey;
use crate::server::GatewayAppState;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CreateReviewPayload {
    pub marketplace_id: String,
    pub rating: i32,
    pub comment: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateReviewPayload {
    pub rating: Option<i32>,
    pub comment: Option<String>,
}

pub async fn create_review(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Json(payload): Json<CreateReviewPayload>,
) -> impl IntoResponse {
    if payload.rating < 1 || payload.rating > 5 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Rating must be between 1 and 5" })),
        ));
    }

    let user_id = auth.0.id.clone();

    let req = CreateReviewRequest {
        marketplace_id: payload.marketplace_id,
        user_id,
        rating: payload.rating,
        comment: payload.comment,
    };

    match state
        .marketplace_service
        .create_review(&state.db, req)
        .await
    {
        Ok(review) => Ok((StatusCode::CREATED, Json(review))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))),
    }
}

pub async fn get_reviews(
    State(state): State<GatewayAppState>,
    Path(marketplace_id): Path<String>,
) -> impl IntoResponse {
    match state
        .marketplace_service
        .get_reviews(&state.db, &marketplace_id)
        .await
    {
        Ok(reviews) => Ok(Json(reviews)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))),
    }
}

pub async fn get_my_review(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(marketplace_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.0.id.clone();
    match state
        .marketplace_service
        .get_user_review(&state.db, &marketplace_id, &user_id)
        .await
    {
        Ok(review) => Ok(Json(review)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))),
    }
}

pub async fn update_review(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(review_id): Path<String>,
    Json(payload): Json<UpdateReviewPayload>,
) -> impl IntoResponse {
    let user_id = auth.0.id.clone();

    let marketplace_id = match state
        .marketplace_service
        .get_marketplace_id_for_review(&state.db, &review_id)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e }))));
        },
    };

    let existing = match state
        .marketplace_service
        .get_user_review(&state.db, &marketplace_id, &user_id)
        .await
    {
        Ok(rev) => rev,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            ));
        },
    };

    match existing {
        Some(r) if r.id != review_id => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": "Cannot update another user's review" })),
            ));
        },
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Review not found" })),
            ));
        },
        _ => {},
    }

    let req = UpdateReviewRequest {
        rating: payload.rating,
        comment: payload.comment,
    };

    match state
        .marketplace_service
        .update_review(&state.db, &review_id, req)
        .await
    {
        Ok(review) => Ok(Json(review)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))),
    }
}

pub async fn delete_review(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(review_id): Path<String>,
) -> impl IntoResponse {
    let user_id = auth.0.id.clone();

    let marketplace_id = match state
        .marketplace_service
        .get_marketplace_id_for_review(&state.db, &review_id)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e }))));
        },
    };

    let existing = match state
        .marketplace_service
        .get_user_review(&state.db, &marketplace_id, &user_id)
        .await
    {
        Ok(rev) => rev,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Database error" })),
            ));
        },
    };

    match existing {
        Some(r) if r.id == review_id => {},
        _ => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": "Cannot delete another user's review" })),
            ));
        },
    }

    match state
        .marketplace_service
        .delete_review(&state.db, &review_id)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({ "success": true }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))),
    }
}

pub async fn get_marketplace_stats(
    State(state): State<GatewayAppState>,
    Path(marketplace_id): Path<String>,
) -> impl IntoResponse {
    match state
        .marketplace_service
        .get_stats(&state.db, &marketplace_id)
        .await
    {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))),
    }
}
