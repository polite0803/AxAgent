// SPDX-License-Identifier: AGPL-3.0-only

//! POST /v1/images/generations — OpenAI 兼容的图像生成端点。
//!
//! `ImageGenProvider` trait 定义在 `axagent-providers` crate 中，而 gateway
//! 作为 consumer crate 仅依赖 `axagent-harness`（架构铁律），无法直接引用
//! `ImageGenProvider`。当前 gateway 的 `GatewayAppState` 也未注入图像生成
//! 能力。因此该端点返回 501 Not Implemented，但路由已注册以避免 404。
//!
//! 如需启用，应在 wiring 层（`axagent-runtime`）把 `ImageGenProvider` 注入
//! `GatewayAppState`，并在本 handler 中调用。

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::error_response;
use crate::server::GatewayAppState;

/// POST /v1/images/generations — 图像生成端点。
///
/// 当前返回 501 Not Implemented，因为 `ImageGenProvider` 未注入 gateway。
pub async fn create_image(
    State(_state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    error_response(
        StatusCode::NOT_IMPLEMENTED,
        "Image generation is not available via gateway. No ImageGenProvider injected.",
    )
}
