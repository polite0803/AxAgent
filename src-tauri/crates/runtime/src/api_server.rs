// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platform_manager::PlatformManager;

static API_TOKEN: std::sync::OnceLock<String> = std::sync::OnceLock::new();

fn get_api_token() -> &'static str {
    API_TOKEN.get_or_init(|| {
        std::env::var("AXAGENT_API_TOKEN").unwrap_or_else(|_| {
            let token = uuid::Uuid::new_v4().to_string();
            tracing::info!(
                "Generated random API token (set AXAGENT_API_TOKEN env var to customize)"
            );
            token
        })
    })
}

#[derive(Clone)]
pub struct ApiServerState {
    pub platform_config: Arc<tokio::sync::RwLock<PlatformConfig>>,
    pub platform_manager: Arc<PlatformManager>,
}

pub struct ApiServer {
    state: ApiServerState,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl ApiServer {
    pub fn new(
        platform_config: Arc<tokio::sync::RwLock<PlatformConfig>>,
        platform_manager: Arc<PlatformManager>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        Self {
            state: ApiServerState {
                platform_config,
                platform_manager,
            },
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
        }
    }

    pub async fn start(mut self, port: u16) -> Result<(), String> {
        let token = get_api_token().to_string();

        let cors = CorsLayer::new()
            .allow_origin([
                "http://localhost"
                    .parse::<axum::http::HeaderValue>()
                    .expect("hardcoded localhost header value is valid"),
                "http://127.0.0.1"
                    .parse::<axum::http::HeaderValue>()
                    .expect("hardcoded 127.0.0.1 header value is valid"),
            ])
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
            ]);

        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/api/chat", post(chat_handler))
            .layer(axum::extract::DefaultBodyLimit::max(1_048_576))
            .layer(axum::middleware::from_fn(move |req, next| {
                let t = token.clone();
                async move { auth_middleware(req, next, &t).await }
            }))
            .layer(cors)
            .with_state(Arc::new(self.state));

        let addr = format!("127.0.0.1:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("API Server bind failed on {}: {}", addr, e))?;

        tracing::info!("API Server listening on {}", addr);

        let shutdown_rx = self
            .shutdown_rx
            .take()
            .ok_or("shutdown_rx already consumed")?;
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
    }
}

async fn auth_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
    token: &str,
) -> Result<axum::response::Response, StatusCode> {
    if req.uri().path() == "/health" {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header == format!("Bearer {}", token) => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().timestamp_millis()
    }))
}

#[derive(serde::Deserialize)]
struct ChatRequest {
    message: String,
    platform: Option<String>,
    user_id: Option<String>,
}

#[derive(serde::Serialize)]
struct ChatResponse {
    reply: Option<String>,
    error: Option<String>,
}

async fn chat_handler(
    State(state): State<Arc<ApiServerState>>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    if req.message.trim().is_empty() {
        return Ok(Json(ChatResponse {
            reply: None,
            error: Some("消息内容不能为空".to_string()),
        }));
    }

    let platform = req.platform.as_deref().unwrap_or("api_server");
    let user_id = req.user_id.as_deref().unwrap_or("api_user");

    let adapter = state.platform_manager.get_adapter(platform).await;
    if let Some(adapter) = adapter {
        let config_guard = state.platform_config.read().await;
        match adapter
            .send_message(&config_guard, user_id, &req.message, None)
            .await
        {
            Ok(()) => Ok(Json(ChatResponse {
                reply: Some("消息已发送".to_string()),
                error: None,
            })),
            Err(e) => Ok(Json(ChatResponse {
                reply: None,
                error: Some(format!("发送失败: {}", e)),
            })),
        }
    } else {
        Ok(Json(ChatResponse {
            reply: None,
            error: Some(format!("未知平台: {}", platform)),
        }))
    }
}
