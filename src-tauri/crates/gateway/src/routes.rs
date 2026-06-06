use axum::{
    Router, middleware,
    routing::{delete, get, patch, post, put},
};
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use tower_http::cors::CorsLayer;

use crate::auth::{AuthState, auth_middleware};
use crate::handlers::{
    cancel_run, chat_completions, create_job, delete_job, delete_response, detailed_health_check,
    disable_job, enable_job, get_job, get_job_schedule, get_response, get_run, get_run_logs,
    health_check, list_jobs, list_models, list_runs, pause_job, resume_job, retry_run, trigger_job,
    trigger_run, update_job, update_job_schedule,
};
use crate::marketplace_handlers::{
    create_review, delete_review, get_marketplace_stats, get_my_review, get_reviews, update_review,
};
use crate::metrics::metrics_handler;
use crate::middleware::rate_limit_middleware;
use crate::native::{
    anthropic_count_tokens, anthropic_messages, gemini_list_models, gemini_model_operation,
    openai_responses,
};
use crate::realtime::realtime_handler;
use crate::server::GatewayAppState;

pub fn create_router(state: GatewayAppState) -> Router {
    // SECURITY (C9): CORS 白名单收紧：
    // - 删除 `tauri://localhost`（沙箱 iframe 可被滥用跨源）
    // - 删除任意 1419 来源（避免本机任意 webview 跨源调用）
    // - 改为只放行显式配置的 origin，环境变量 `AXAGENT_GATEWAY_ALLOWED_ORIGINS` 可扩展
    let mut allowed: Vec<http::HeaderValue> = Vec::new();
    for default_origin in &[
        // Tauri 桌面 webview 内核使用 https://tauri.localhost
        "https://tauri.localhost",
    ] {
        if let Ok(v) = default_origin.parse() {
            allowed.push(v);
        }
    }
    if let Ok(extra) = std::env::var("AXAGENT_GATEWAY_ALLOWED_ORIGINS") {
        for raw in extra.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if let Ok(v) = raw.parse() {
                allowed.push(v);
            }
        }
    }
    if allowed.is_empty() {
        // 兜底：至少允许一个 tauri 内核回环
        if let Ok(v) = "https://tauri.localhost".parse() {
            allowed.push(v);
        }
    }
    let cors = CorsLayer::new()
        .allow_origin(allowed)
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::PUT,
            http::Method::PATCH,
            http::Method::DELETE,
            http::Method::OPTIONS,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(false)
        .max_age(std::time::Duration::from_secs(600));

    // Protected routes (require auth)
    let protected = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/responses", post(openai_responses))
        .route("/v1/responses/{response_id}", get(get_response))
        .route("/v1/responses/{response_id}", delete(delete_response))
        .route("/v1/messages", post(anthropic_messages))
        .route("/v1/messages/count_tokens", post(anthropic_count_tokens))
        .route("/v1/models", get(list_models))
        .route("/v1beta/models", get(gemini_list_models))
        .route(
            "/v1beta/models/{model_action}",
            post(gemini_model_operation),
        )
        .route("/api/jobs", get(list_jobs))
        .route("/api/jobs", post(create_job))
        .route("/api/jobs/{job_id}", get(get_job))
        .route("/api/jobs/{job_id}", patch(update_job))
        .route("/api/jobs/{job_id}", delete(delete_job))
        .route("/api/jobs/{job_id}/enable", post(enable_job))
        .route("/api/jobs/{job_id}/disable", post(disable_job))
        .route("/api/jobs/{job_id}/pause", post(pause_job))
        .route("/api/jobs/{job_id}/resume", post(resume_job))
        .route("/api/jobs/{job_id}/run", post(trigger_job))
        .route("/api/jobs/{job_id}/schedule", get(get_job_schedule))
        .route("/api/jobs/{job_id}/schedule", put(update_job_schedule))
        .route("/api/jobs/{job_id}/runs", get(list_runs))
        .route("/api/jobs/{job_id}/runs", post(trigger_run))
        .route("/api/jobs/{job_id}/runs/{run_id}", get(get_run))
        .route("/api/jobs/{job_id}/runs/{run_id}/cancel", post(cancel_run))
        .route("/api/jobs/{job_id}/runs/{run_id}/retry", post(retry_run))
        .route("/api/jobs/{job_id}/runs/{run_id}/logs", get(get_run_logs))
        // Marketplace reviews
        .route(
            "/api/marketplace/{marketplace_id}/reviews",
            get(get_reviews),
        )
        .route(
            "/api/marketplace/{marketplace_id}/reviews",
            post(create_review),
        )
        .route(
            "/api/marketplace/{marketplace_id}/reviews/me",
            get(get_my_review),
        )
        .route(
            "/api/marketplace/{marketplace_id}/stats",
            get(get_marketplace_stats),
        )
        .route("/api/reviews/{review_id}", patch(update_review))
        .route("/api/reviews/{review_id}", delete(delete_review))
        .route("/health/detailed", get(detailed_health_check))
        // SECURITY (H3): realtime 之前是"内部鉴权"但未走 auth_middleware，改为受保护路由。
        .route("/v1/realtime", get(realtime_handler))
        .layer(middleware::from_fn_with_state(
            AuthState {
                db: state.db.clone(),
                master_key: state.master_key,
            },
            auth_middleware,
        ));

    // Public routes
    let public = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .layer(middleware::from_fn(rate_limit_middleware));

    Router::new()
        .merge(protected)
        .merge(public)
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_core::db::create_test_pool;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use tower::ServiceExt;

    fn test_state(db: sea_orm::DatabaseConnection) -> GatewayAppState {
        GatewayAppState {
            db,
            master_key: [7u8; 32],
            started_at: 0,
            provider_registry: axagent_harness::test_support::empty_provider_registry(),
        }
    }

    async fn assert_protected_route_exists(method: Method, uri: &str) {
        let handle = create_test_pool().await.unwrap();
        let app = create_router(test_state(handle.conn));
        let response = app
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "expected protected route {} {} to reject missing auth instead of 404/405",
            uri,
            response.status()
        );
    }

    #[tokio::test]
    async fn native_protocol_routes_require_auth() {
        assert_protected_route_exists(Method::POST, "/v1/responses").await;
        assert_protected_route_exists(Method::POST, "/v1/messages").await;
        assert_protected_route_exists(Method::POST, "/v1/messages/count_tokens").await;
        assert_protected_route_exists(Method::GET, "/v1beta/models").await;
        assert_protected_route_exists(
            Method::POST,
            "/v1beta/models/gemini-2.5-pro:generateContent",
        )
        .await;
        assert_protected_route_exists(
            Method::POST,
            "/v1beta/models/gemini-2.5-pro:streamGenerateContent?alt=sse",
        )
        .await;
        assert_protected_route_exists(Method::POST, "/v1beta/models/gemini-2.5-pro:countTokens")
            .await;
    }

    #[tokio::test]
    async fn realtime_requires_auth() {
        // SECURITY (H3): realtime 必须鉴权
        assert_protected_route_exists(Method::GET, "/v1/realtime").await;
    }

    #[test]
    fn cors_strips_tauri_localhost_and_1419() {
        // SECURITY (C9): 不再包含任意 1419 / tauri://localhost
        let mut allowed: Vec<http::HeaderValue> = Vec::new();
        for default_origin in &["https://tauri.localhost"] {
            if let Ok(v) = default_origin.parse() {
                allowed.push(v);
            }
        }
        let serialized: Vec<String> = allowed
            .iter()
            .map(|v| v.to_str().unwrap_or("").to_string())
            .collect();
        assert!(!serialized.iter().any(|s| s.contains("1419")));
        assert!(!serialized.iter().any(|s| s.contains("tauri://")));
        assert!(serialized.iter().any(|s| s == "https://tauri.localhost"));
    }
}
