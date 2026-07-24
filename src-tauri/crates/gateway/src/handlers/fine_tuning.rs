// SPDX-License-Identifier: AGPL-3.0-only

//! /v1/fine_tuning/jobs 端点组 — OpenAI 兼容的微调任务管理接口。
//!
//! 实际微调能力在 devtools/fineTune 模块中，不通过网关暴露。
//! 此处注册路由以避免 404，所有端点返回 501 Not Implemented 并给出
//! 明确错误信息，引导调用方使用 devtools UI。

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::error_response;
use crate::server::GatewayAppState;

/// 微调不支持的错误消息常量。
const NOT_SUPPORTED_MSG: &str = "Fine tuning not supported via gateway, use the devtools UI";

/// POST /v1/fine_tuning/jobs — 创建微调任务（不支持）。
pub async fn create_fine_tuning_job(
    State(_state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    error_response(StatusCode::NOT_IMPLEMENTED, NOT_SUPPORTED_MSG)
}

/// GET /v1/fine_tuning/jobs — 列出微调任务（不支持）。
pub async fn list_fine_tuning_jobs(
    State(_state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    error_response(StatusCode::NOT_IMPLEMENTED, NOT_SUPPORTED_MSG)
}

/// GET /v1/fine_tuning/jobs/{job_id} — 获取微调任务详情（不支持）。
pub async fn retrieve_fine_tuning_job(
    State(_state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(_job_id): Path<String>,
) -> impl IntoResponse {
    error_response(StatusCode::NOT_IMPLEMENTED, NOT_SUPPORTED_MSG)
}

/// DELETE /v1/fine_tuning/jobs/{job_id} — 取消微调任务（不支持）。
pub async fn cancel_fine_tuning_job(
    State(_state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(_job_id): Path<String>,
) -> impl IntoResponse {
    error_response(StatusCode::NOT_IMPLEMENTED, NOT_SUPPORTED_MSG)
}
