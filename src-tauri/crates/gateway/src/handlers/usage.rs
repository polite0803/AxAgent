// SPDX-License-Identifier: AGPL-3.0-only
//! `GET /v1/usage` — 暴露网关用量与成本统计。
//!
//! 复用 `GatewayKeyRepository::get_metrics`（dao 层单条 SQL 聚合），
//! 返回全量 + 今日两个维度的请求数、token 数与估算美元成本。
//! 成本字段（`total_cost_usd` / `today_cost_usd`）由 chat / native
//! handler 在 `record_usage` 时基于 `ModelPricing` 换算后落库，
//! 此端点只做读取，不参与定价计算。

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};

use crate::auth::AuthenticatedKey;
use crate::handlers::error::error_response;
use crate::server::GatewayAppState;

/// GET /v1/usage — 返回网关累计与今日的请求数、token 数、估算美元成本。
///
/// 受 `auth_middleware` 保护：调用方必须携带有效的 gateway API key。
/// 失败时返回 500 + 标准 JSON 错误信封，与其它 handler 保持一致。
pub async fn usage_handler(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> axum::response::Response {
    // 鉴权通过即放行；当前 handler 不区分 key 维度，统一返回聚合指标。
    // 若后续需要 per-key 视图，可基于 `auth.0.id` 走 get_usage_by_key。
    let AuthenticatedKey(_gateway_key) = auth;

    match state.adapter.gateway_keys().get_metrics().await {
        Ok(metrics) => Json(metrics).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "Failed to fetch gateway usage metrics");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch usage metrics")
        },
    }
}
