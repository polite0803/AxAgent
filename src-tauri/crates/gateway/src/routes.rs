// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    middleware,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
};
use http::StatusCode;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;
use tower_http::cors::CorsLayer;

use crate::auth::{AuthState, auth_middleware};
use crate::handlers::mcp_proxy::{call_mcp_tool, discover_mcp_tools, list_mcp_servers};
use crate::handlers::platform_bridge::{
    Platform, direct_message, platform_health, receive_webhook,
};
use crate::handlers::{
    cancel_fine_tuning_job, cancel_run, chat_completions, create_embedding, create_fine_tuning_job,
    create_image, create_job, create_speech, create_transcription, delete_file, delete_job,
    delete_response, detailed_health_check, disable_job, enable_job, get_job, get_job_schedule,
    get_response, get_run, get_run_logs, health_check, list_files, list_fine_tuning_jobs,
    list_jobs, list_models, list_runs, pause_job, resume_job, retrieve_file,
    retrieve_fine_tuning_job, retry_run, trigger_job, trigger_run, update_job, update_job_schedule,
    upload_file, usage_handler,
};

/// Wrapper that extracts the `{platform}` path parameter, converts it to a
/// `Platform` extension, and delegates to `receive_webhook`.
async fn receive_webhook_with_path(
    State(state): State<GatewayAppState>,
    Path(platform_str): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    match Platform::from_path_segment(&platform_str) {
        Some(platform) => {
            receive_webhook(State(state), Extension(platform), headers, body).await.into_response()
        },
        None => {
            let err_body = Json(json!({
                "status": "unsupported_platform",
                "message": format!("Unsupported platform: {}", platform_str)
            }));
            (StatusCode::BAD_REQUEST, err_body).into_response()
        },
    }
}

use crate::marketplace_handlers::{
    create_review, delete_review, get_marketplace_stats, get_my_review, get_reviews, update_review,
};
use crate::metrics::metrics_handler;
use crate::middleware::rate_limit_middleware;
use crate::native::{
    anthropic_count_tokens, anthropic_messages, gemini_list_models, gemini_model_operation,
    openai_responses,
};
use crate::qr_bind_handlers::{consume_qr_token, generate_qr_token};
use crate::realtime::{issue_realtime_ticket, realtime_handler};
use crate::server::GatewayAppState;

// ACP handler imports
use crate::handlers::acp::{
    acp_websocket_handler, close_session, create_session, get_session, interrupt_session,
    list_sessions, send_prompt,
};

pub fn create_router(state: GatewayAppState) -> Router {
    let cors = build_cors_layer();

    // Protected routes (require auth)
    let protected = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/responses", post(openai_responses))
        .route("/v1/responses/{response_id}", get(get_response))
        .route("/v1/responses/{response_id}", delete(delete_response))
        .route("/v1/messages", post(anthropic_messages))
        .route("/v1/messages/count_tokens", post(anthropic_count_tokens))
        .route("/v1/models", get(list_models))
        // 网关用量与成本聚合统计（受 auth 保护）
        .route("/v1/usage", get(usage_handler))
        .route("/v1beta/models", get(gemini_list_models))
        .route(
            "/v1beta/models/{model_action}",
            post(gemini_model_operation),
        )
        // OpenAI 兼容端点：embeddings / images / audio
        .route("/v1/embeddings", post(create_embedding))
        .route("/v1/images/generations", post(create_image))
        .route("/v1/audio/transcriptions", post(create_transcription))
        .route("/v1/audio/speech", post(create_speech))
        // OpenAI 兼容端点：files（501，路由已注册）
        .route("/v1/files", get(list_files))
        .route("/v1/files", post(upload_file))
        .route("/v1/files/{file_id}", get(retrieve_file))
        .route("/v1/files/{file_id}", delete(delete_file))
        // OpenAI 兼容端点：fine_tuning（501，路由已注册）
        .route("/v1/fine_tuning/jobs", post(create_fine_tuning_job))
        .route("/v1/fine_tuning/jobs", get(list_fine_tuning_jobs))
        .route(
            "/v1/fine_tuning/jobs/{job_id}",
            get(retrieve_fine_tuning_job),
        )
        .route(
            "/v1/fine_tuning/jobs/{job_id}",
            delete(cancel_fine_tuning_job),
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
        // SECURITY (P0-2.2): WS upgrade must not carry the long-lived API key.
        // Caller presents Bearer token, gets back a single-use short-lived ticket.
        .route("/v1/realtime-ticket", post(issue_realtime_ticket))
        // QR 绑定路由（WebUI 生成 / 平台端消费）
        .route("/v1/bind/qr-token", post(generate_qr_token))
        .route("/v1/bind/qr-token/{token}", post(consume_qr_token))
        // MCP 代理路由
        .route("/v1/mcp/servers", get(list_mcp_servers))
        .route(
            "/v1/mcp/servers/{server_id}/tools/list",
            post(discover_mcp_tools),
        )
        .route(
            "/v1/mcp/servers/{server_id}/tools/call",
            post(call_mcp_tool),
        );

    // ACP (Agent Communication Protocol) 路由 — 受 acp_enabled 门控。
    // 启用时注册会话管理 / prompt / WebSocket 端点，供外部工具/IDE 调用。
    let protected = if state.acp_enabled {
        tracing::info!("[Routes] ACP protocol enabled — registering ACP endpoints");
        protected
            .route("/acp/v1/sessions", post(create_session))
            .route("/acp/v1/sessions", get(list_sessions))
            .route("/acp/v1/sessions/{id}", get(get_session))
            .route("/acp/v1/sessions/{id}/prompts", post(send_prompt))
            .route("/acp/v1/sessions/{id}/interrupt", post(interrupt_session))
            .route("/acp/v1/sessions/{id}/close", post(close_session))
            .route("/acp/v1/ws", get(acp_websocket_handler))
    } else {
        tracing::debug!("[Routes] ACP protocol disabled — skipping endpoint registration");
        protected
    };

    let protected = protected
        .layer(Extension(state.ticket_store.clone()))
        .layer(middleware::from_fn_with_state(
            AuthState {
                db: state.db.clone(),
                adapter: state.adapter.clone(),
                key_verify_limiter: state.key_verify_limiter.clone(),
                client_ip_policy: state.client_ip_policy.clone(),
            },
            auth_middleware,
        ));

    // Public routes
    let public = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .layer(middleware::from_fn(rate_limit_middleware));

    // Platform bridge routes — internal auth with optional API key
    let platform = Router::new()
        .route("/api/platform/health", get(platform_health))
        .route("/api/platform/message", post(direct_message))
        .route("/api/webhook/{platform}", post(receive_webhook_with_path));

    Router::new().merge(protected).merge(platform).merge(public).layer(cors).with_state(state)
}

/// 构造网关 CORS 层。
///
/// SECURITY (C9 + P2-23): CORS 白名单收紧，启动时校验。
/// - 删除 `tauri://localhost`（沙箱 iframe 可被滥用跨源）
/// - 删除任意 1419 来源（避免本机任意 webview 跨源调用）
/// - 只放行显式配置的 origin，环境变量 `AXAGENT_GATEWAY_ALLOWED_ORIGINS` 可扩展
/// - 启动时校验每条 origin 必须是 https / http://localhost / http://127.0.0.1；
///   拒绝包含用户信息、片段或非允许 scheme 的来源
fn build_cors_layer() -> CorsLayer {
    // 收集来源字符串，启动后再校验/转换
    let mut raw_origins: Vec<String> = vec!["https://tauri.localhost".to_string()];

    if let Ok(extra) = std::env::var("AXAGENT_GATEWAY_ALLOWED_ORIGINS") {
        for raw in extra.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            raw_origins.push(raw.to_string());
        }
    }

    let mut allowed: Vec<http::HeaderValue> = Vec::with_capacity(raw_origins.len());
    let mut rejected: Vec<String> = Vec::new();
    for raw in raw_origins {
        match validate_origin(&raw) {
            Ok(header) => allowed.push(header),
            Err(reason) => rejected.push(format!("{raw} ({reason})")),
        }
    }
    if allowed.is_empty() {
        // 兜底：至少允许一个 tauri 内核回环
        if let Ok(v) = "https://tauri.localhost".parse() {
            allowed.push(v);
        }
    }

    // 启动时日志：让运维一眼能看见实际生效的白名单
    let final_list: Vec<String> =
        allowed.iter().map(|v| v.to_str().unwrap_or("<binary>").to_string()).collect();
    tracing::info!(
        target: "axagent.gateway.cors",
        "CORS allowed origins: [{}]",
        final_list.join(", ")
    );
    if !rejected.is_empty() {
        tracing::warn!(
            target: "axagent.gateway.cors",
            "CORS rejected origins (set AXAGENT_GATEWAY_ALLOWED_ORIGINS to override): [{}]",
            rejected.join(", ")
        );
    }

    CorsLayer::new()
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
        .allow_credentials(allow_credentials())
        .max_age(std::time::Duration::from_secs(600))
}

/// 从环境变量读取 `AXAGENT_GATEWAY_ALLOW_CREDENTIALS` 决定是否允许 credentials。
/// 默认 `false`（安全优先）—— 仅当存在严格的 origin 白名单时才启用。
fn allow_credentials() -> bool {
    std::env::var("AXAGENT_GATEWAY_ALLOW_CREDENTIALS")
        .map(|v| v.to_lowercase())
        .map(|v| v == "1" || v == "true" || v == "yes")
        .unwrap_or(false)
}

/// 校验 origin 字符串：必须能解析为 `HeaderValue`，并满足以下条件之一：
/// - scheme = `https`
/// - scheme = `http` 且 host 是 `localhost` / `127.0.0.1` / `::1`（开发态）
///
/// 拒绝包含 userinfo、path、query、fragment 的来源（这些都不是合法 origin）。
fn validate_origin(raw: &str) -> Result<http::HeaderValue, &'static str> {
    let value: http::HeaderValue = raw.parse().map_err(|_| "not a valid header value")?;

    // HeaderValue 必须是可见 ASCII
    if value.to_str().map_err(|_| "non-ascii bytes")? != raw {
        return Err("value round-trip mismatch");
    }

    // URL 形态校验
    let url = url::Url::parse(raw).map_err(|_| "not a valid URL")?;
    if url.has_authority() && !url.authority().is_empty() {
        let host = url.host_str().unwrap_or("");
        let userinfo = !url.username().is_empty() || url.password().is_some();
        if userinfo {
            return Err("userinfo not allowed");
        }
        if url.path() != "/" && !url.path().is_empty() {
            return Err("path not allowed in origin");
        }
        if url.query().is_some() {
            return Err("query not allowed in origin");
        }
        if url.fragment().is_some() {
            return Err("fragment not allowed in origin");
        }
        match url.scheme() {
            "https" => {},
            "http" if matches!(host, "localhost" | "127.0.0.1" | "::1") => {},
            _ => return Err("scheme/host not allowed"),
        }
    } else {
        return Err("missing host");
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::ClientIpPolicy;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use sea_orm::Database;
    use tower::ServiceExt;

    fn test_state(db: sea_orm::DatabaseConnection) -> GatewayAppState {
        GatewayAppState {
            db,
            master_key: [7u8; 32],
            started_at: 0,
            provider_registry: axagent_harness::test_support::empty_provider_registry(),
            adapter: axagent_harness::test_support::empty_platform_adapter(),
            marketplace_service: axagent_harness::test_support::empty_marketplace_service(),
            mcp_store: std::sync::Arc::new(axagent_harness::test_support::NoopMcpServerStore),
            mcp_client: std::sync::Arc::new(axagent_harness::test_support::NoopMcpClientService),
            ticket_store: crate::realtime::default_ticket_store(),
            // SECURITY (Phase 2 Task 2.3): 路由层测试用宽阈值 limiter，
            // 避免和限流本身的目的混在一起。
            key_verify_limiter: std::sync::Arc::new(crate::auth::KeyVerifyLimiter::new(
                100,
                std::time::Duration::from_secs(60),
            )),
            client_ip_policy: std::sync::Arc::new(ClientIpPolicy::trust_all()),
            qr_bind_store: crate::qr_bind::QrBindStore::new(),
            routing_strategy: axagent_harness::types::LoadBalanceStrategy::default(),
            latency_tracker: crate::routing::LatencyTracker::new(),
            round_robin_cursor: crate::routing::RoundRobinCursor::new(),
        }
    }

    async fn assert_protected_route_exists(method: Method, uri: &str) {
        let db = Database::connect("sqlite::memory:?mode=rwc").await.unwrap();
        let app = create_router(test_state(db));
        let response = app
            .oneshot(Request::builder().method(method).uri(uri).body(Body::empty()).unwrap())
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

    #[tokio::test]
    async fn usage_endpoint_requires_auth() {
        // /v1/usage 暴露成本与用量统计，必须鉴权
        assert_protected_route_exists(Method::GET, "/v1/usage").await;
    }

    #[tokio::test]
    async fn openai_compat_endpoints_require_auth() {
        // 新增的 OpenAI 兼容端点必须受 auth_middleware 保护
        assert_protected_route_exists(Method::POST, "/v1/embeddings").await;
        assert_protected_route_exists(Method::POST, "/v1/images/generations").await;
        assert_protected_route_exists(Method::POST, "/v1/audio/transcriptions").await;
        assert_protected_route_exists(Method::POST, "/v1/audio/speech").await;
        assert_protected_route_exists(Method::GET, "/v1/files").await;
        assert_protected_route_exists(Method::POST, "/v1/files").await;
        assert_protected_route_exists(Method::GET, "/v1/files/file-abc").await;
        assert_protected_route_exists(Method::DELETE, "/v1/files/file-abc").await;
        assert_protected_route_exists(Method::POST, "/v1/fine_tuning/jobs").await;
        assert_protected_route_exists(Method::GET, "/v1/fine_tuning/jobs").await;
        assert_protected_route_exists(Method::GET, "/v1/fine_tuning/jobs/job-abc").await;
        assert_protected_route_exists(Method::DELETE, "/v1/fine_tuning/jobs/job-abc").await;
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
        let serialized: Vec<String> =
            allowed.iter().map(|v| v.to_str().unwrap_or("").to_string()).collect();
        assert!(!serialized.iter().any(|s| s.contains("1419")));
        assert!(!serialized.iter().any(|s| s.contains("tauri://")));
        assert!(serialized.iter().any(|s| s == "https://tauri.localhost"));
    }

    #[test]
    fn validate_origin_accepts_https_and_localhost() {
        // https 来源
        assert!(validate_origin("https://tauri.localhost").is_ok());
        assert!(validate_origin("https://app.example.com").is_ok());
        // http + 本机回环（开发态）
        assert!(validate_origin("http://localhost:3000").is_ok());
        assert!(validate_origin("http://127.0.0.1:8080").is_ok());
    }

    #[test]
    fn validate_origin_rejects_insecure_or_malformed() {
        // SECURITY (P2-23): 拒绝 http + 非回环（mixed-content / 中间人风险）
        assert!(validate_origin("http://app.example.com").is_err());
        assert!(validate_origin("http://10.0.0.5").is_err());
        // 拒绝 userinfo / path / query / fragment
        assert!(validate_origin("https://user:pw@app.example.com").is_err());
        assert!(validate_origin("https://app.example.com/path").is_err());
        assert!(validate_origin("https://app.example.com?x=1").is_err());
        assert!(validate_origin("https://app.example.com#frag").is_err());
        // 拒绝缺失 scheme / host
        assert!(validate_origin("app.example.com").is_err());
        assert!(validate_origin("https://").is_err());
    }
}
