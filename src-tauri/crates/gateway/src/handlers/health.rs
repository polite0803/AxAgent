use axum::extract::{Extension, State};
use axum::response::{IntoResponse, Json};
use serde_json::json;

use crate::auth::AuthenticatedKey;
use crate::server::GatewayAppState;

/// GET /health — unauthenticated health check
pub async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

/// GET /health/detailed — detailed health check with system info (requires authentication)
pub async fn detailed_health_check(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> axum::response::Response {
    let AuthenticatedKey(_gateway_key) = auth;

    let db_status = match state.adapter.providers().list_providers().await {
        Ok(_) => "connected",
        Err(e) => {
            tracing::warn!("Database health check failed: {}", e);
            "disconnected"
        },
    };

    let providers_count = match state.adapter.providers().list_providers().await {
        Ok(p) => p.len(),
        Err(_) => 0,
    };

    let active_keys_count = match state.adapter.gateway_keys().list_gateway_keys().await {
        Ok(keys) => keys.iter().filter(|k| k.enabled).count(),
        Err(_) => 0,
    };

    let uptime = axagent_harness::util_fns::now_ts() - state.started_at;
    let uptime = if uptime > 0 { uptime as u64 } else { 0 };

    Json(json!({
        "status": "ok",
        "uptime_seconds": uptime,
        "database": db_status,
        "providers_count": providers_count,
        "active_keys_count": active_keys_count,
        "version": env!("CARGO_PKG_VERSION"),
    }))
    .into_response()
}
