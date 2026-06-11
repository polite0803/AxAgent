use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sea_orm::DatabaseConnection;
use serde_json::json;
use std::sync::Arc;

use axagent_harness::platform_adapter::PlatformAdapter;
use axagent_harness::types::GatewayKey;

/// Authenticated key injected into request extensions after auth middleware.
#[derive(Clone, Debug)]
pub struct AuthenticatedKey(pub GatewayKey);

/// 鉴权中间件需要的运行时状态（adapter）。由 routes.rs 用 `from_fn_with_state` 注入。
#[derive(Clone)]
pub struct AuthState {
    /// 数据库连接，update_last_used 后台任务用
    pub db: DatabaseConnection,
    pub adapter: Arc<dyn PlatformAdapter>,
}

/// Auth middleware: extracts Bearer token, verifies against gateway_keys, updates last_used_at.
pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": {
                        "message": "Missing or invalid Authorization header. Expected: Bearer <api-key>",
                        "type": "invalid_request_error",
                        "code": "invalid_api_key"
                    }
                })),
            )
                .into_response();
        },
    };

    match state.adapter.gateway_keys().verify_key(token).await {
        Ok(Some(key)) => {
            // Update last_used_at in background (non-blocking)
            let adapter_bg = state.adapter.clone();
            let key_id = key.id.clone();
            tokio::spawn(async move {
                if let Err(e) = adapter_bg.gateway_keys().update_last_used(&key_id).await {
                    tracing::warn!(%e, "Failed to update gateway key last_used");
                }
            });

            request.extensions_mut().insert(AuthenticatedKey(key));
            next.run(request).await
        },
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": {
                    "message": "Invalid or disabled API key",
                    "type": "invalid_request_error",
                    "code": "invalid_api_key"
                }
            })),
        )
            .into_response(),
    }
}
