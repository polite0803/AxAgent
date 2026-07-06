// SPDX-License-Identifier: AGPL-3.0-only

//! QR 绑定路由（WebUI 生成 / 平台端消费）。
//!
//! 基于现有 `TicketStore` 模式，提供 IM 渠道扫码绑定能力：
//! 1. WebUI 调用 `POST /v1/bind/qr-token` 生成绑定令牌
//! 2. 用户在目标平台（Telegram 等）发送二维码中的令牌
//! 3. 平台端调用 `POST /v1/bind/qr-token/{token}` 完成绑定

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;

use crate::auth::AuthenticatedKey;
use crate::server::GatewayAppState;

#[derive(Debug, Deserialize)]
pub struct BindRequest {
    /// 平台名称：telegram / discord 等
    pub platform: String,
    /// 平台侧用户 ID
    pub platform_user_id: String,
}

/// POST /v1/bind/qr-token — 生成 QR 绑定令牌（需要 API Key 认证）。
pub async fn generate_qr_token(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    let ticket = state.qr_bind_store.issue(&auth.0.id);
    (
        StatusCode::OK,
        Json(json!({
            "token": ticket.ticket_id,
            "expires_in_secs": 300,
        })),
    )
        .into_response()
}

/// POST /v1/bind/qr-token/{token} — 消费绑定令牌（平台端调用，无需 API Key）。
///
/// 调用时需携带 platform + platform_user_id，
/// 令牌被消费后将平台用户 ID 与绑定的 API Key 关联。
pub async fn consume_qr_token(
    State(state): State<GatewayAppState>,
    Path(token): Path<String>,
    Json(body): Json<BindRequest>,
) -> impl IntoResponse {
    let ticket = match state.qr_bind_store.consume(&token) {
        Some(t) => t,
        None => {
            return (
                StatusCode::GONE,
                Json(json!({ "error": "Invalid, expired, or already-used QR token" })),
            )
                .into_response();
        },
    };

    // 将平台用户绑定到 API Key
    let result = state
        .adapter
        .gateway_keys()
        .bind_platform_user(&ticket.key_id, &body.platform, &body.platform_user_id)
        .await;

    match result {
        Ok(_) => {
            tracing::info!(
                "QR bind: platform={} user={} → key={}",
                body.platform,
                body.platform_user_id,
                ticket.key_id
            );
            (StatusCode::OK, Json(json!({ "status": "bound" }))).into_response()
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })))
            .into_response(),
    }
}
