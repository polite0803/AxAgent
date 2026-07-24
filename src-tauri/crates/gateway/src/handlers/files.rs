// SPDX-License-Identifier: AGPL-3.0-only

//! /v1/files 端点组 — OpenAI 兼容的文件管理接口。
//!
//! 文件存储与管理不在 gateway 的职责范围内（应由 RAG / 知识库模块处理）。
//! 此处注册路由以避免 404，所有端点返回 501 Not Implemented 并给出明确
//! 错误信息，引导调用方使用正确的 UI / API。

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::error_response;
use crate::server::GatewayAppState;

/// 文件管理不支持的错误消息常量。
const NOT_SUPPORTED_MSG: &str =
    "File management is not supported via gateway. Use the knowledge base / RAG API instead.";

/// GET /v1/files — 列出文件（不支持）。
pub async fn list_files(
    State(_state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    error_response(StatusCode::NOT_IMPLEMENTED, NOT_SUPPORTED_MSG)
}

/// POST /v1/files — 上传文件（不支持）。
pub async fn upload_file(
    State(_state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    error_response(StatusCode::NOT_IMPLEMENTED, NOT_SUPPORTED_MSG)
}

/// GET /v1/files/{file_id} — 获取文件元数据（不支持）。
pub async fn retrieve_file(
    State(_state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(_file_id): Path<String>,
) -> impl IntoResponse {
    error_response(StatusCode::NOT_IMPLEMENTED, NOT_SUPPORTED_MSG)
}

/// DELETE /v1/files/{file_id} — 删除文件（不支持）。
pub async fn delete_file(
    State(_state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(_file_id): Path<String>,
) -> impl IntoResponse {
    error_response(StatusCode::NOT_IMPLEMENTED, NOT_SUPPORTED_MSG)
}
