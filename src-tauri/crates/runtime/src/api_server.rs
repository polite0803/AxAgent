use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platform_manager::PlatformManager;

#[derive(Clone)]
pub struct ApiServerState {
    pub platform_config: Arc<tokio::sync::RwLock<PlatformConfig>>,
    pub platform_manager: Arc<PlatformManager>,
}

pub struct ApiServer {
    state: ApiServerState,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl ApiServer {
    pub fn new(
        platform_config: Arc<tokio::sync::RwLock<PlatformConfig>>,
        platform_manager: Arc<PlatformManager>,
    ) -> Self {
        let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
        Self {
            state: ApiServerState {
                platform_config,
                platform_manager,
            },
            shutdown_tx,
        }
    }

    pub async fn start(self, port: u16) -> Result<(), String> {
        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/api/chat", post(chat_handler))
            .with_state(Arc::new(self.state));

        let addr = format!("127.0.0.1:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("API Server bind failed on {}: {}", addr, e))?;

        tracing::info!("API Server listening on {}", addr);
        axum::serve(listener, app).await.map_err(|e| e.to_string())
    }

    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
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

    // 通过 PlatformManager 的消息回调机制处理
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
