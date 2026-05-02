use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platforms::{wechat, whatsapp};
use crate::webhook_subscription::{WebhookEvent, WebhookSubscription, WebhookSubscriptionManager};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct WebhookServerState {
    pub subscription_manager: Arc<WebhookSubscriptionManager>,
    /// WhatsApp webhook 处理所需的平台配置
    pub platform_config: Option<Arc<tokio::sync::RwLock<PlatformConfig>>>,
}

pub struct WebhookServer {
    state: WebhookServerState,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl WebhookServer {
    pub fn new(subscription_manager: Arc<WebhookSubscriptionManager>) -> Self {
        let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
        Self {
            state: WebhookServerState {
                subscription_manager,
                platform_config: None,
            },
            shutdown_tx,
        }
    }

    /// 设置 WhatsApp webhook 所需的平台配置
    pub fn set_platform_config(&mut self, config: Arc<tokio::sync::RwLock<PlatformConfig>>) {
        self.state.platform_config = Some(config);
    }

    pub async fn start(self, listener: tokio::net::TcpListener) -> Result<(), String> {
        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/subscriptions", get(list_subscriptions_handler))
            .route("/subscriptions", post(create_subscription_handler))
            .route(
                "/subscriptions/:id",
                axum::routing::delete(delete_subscription_handler),
            )
            // WhatsApp webhook 路由
            .route("/whatsapp/webhook", get(whatsapp_verify_handler))
            .route("/whatsapp/webhook", post(whatsapp_notification_handler))
            // WeChat 官方公众号 webhook 路由
            .route("/wechat/portal", get(wechat_verify_handler))
            .route("/wechat/portal", post(wechat_message_handler))
            .with_state(Arc::new(self.state));
        tracing::info!(
            "Webhook server listening on port {}",
            listener.local_addr().unwrap().port()
        );
        axum::serve(listener, app).await.map_err(|e| e.to_string())
    }

    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
    }
}

async fn health_handler() -> &'static str {
    "OK"
}

// ── WhatsApp webhook handlers ──

/// WhatsApp Webhook 验证端点 (GET)
/// Meta 在配置 webhook 时发送 GET 请求验证服务器
#[derive(serde::Deserialize)]
struct WhatsAppVerifyQuery {
    #[serde(rename = "hub.mode")]
    hub_mode: String,
    #[serde(rename = "hub.verify_token")]
    hub_verify_token: String,
    #[serde(rename = "hub.challenge")]
    hub_challenge: String,
}

async fn whatsapp_verify_handler(
    State(state): State<Arc<WebhookServerState>>,
    axum::extract::Query(query): axum::extract::Query<WhatsAppVerifyQuery>,
) -> Result<String, StatusCode> {
    let config_guard = state
        .platform_config
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let config = config_guard.read().await;

    match whatsapp::verify_webhook_challenge(
        &config,
        &query.hub_mode,
        &query.hub_verify_token,
        &query.hub_challenge,
    ) {
        Ok(challenge) => Ok(challenge),
        Err(e) => {
            tracing::warn!("WhatsApp webhook verification failed: {}", e);
            Err(StatusCode::FORBIDDEN)
        },
    }
}

/// WhatsApp Webhook 通知端点 (POST)
/// Meta 在有新消息时发送 POST 通知
async fn whatsapp_notification_handler(
    State(state): State<Arc<WebhookServerState>>,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let config_guard = match state.platform_config.as_ref() {
        Some(c) => c,
        None => return StatusCode::SERVICE_UNAVAILABLE,
    };
    let config = config_guard.read().await;

    match whatsapp::handle_webhook_notification(&config, &body).await {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            tracing::error!("WhatsApp webhook notification error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        },
    }
}

// ── WeChat official_account webhook handlers ──

#[derive(serde::Deserialize)]
struct WeChatVerifyQuery {
    signature: String,
    timestamp: String,
    nonce: String,
    echostr: String,
}

async fn wechat_verify_handler(
    State(state): State<Arc<WebhookServerState>>,
    axum::extract::Query(query): axum::extract::Query<WeChatVerifyQuery>,
) -> Result<String, StatusCode> {
    let config_guard = state
        .platform_config
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let config = config_guard.read().await;

    let token = config.wechat_token.as_deref().unwrap_or("");

    match wechat::verify_server(
        token,
        &query.signature,
        &query.timestamp,
        &query.nonce,
        &query.echostr,
    ) {
        Ok(echostr) => Ok(echostr),
        Err(e) => {
            tracing::warn!("WeChat server verification failed: {}", e);
            Err(StatusCode::FORBIDDEN)
        },
    }
}

async fn wechat_message_handler(
    State(state): State<Arc<WebhookServerState>>,
    body: axum::body::Bytes,
) -> Result<String, StatusCode> {
    let config_guard = state
        .platform_config
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let config = config_guard.read().await;

    let xml_body = String::from_utf8_lossy(&body).to_string();

    match wechat::handle_official_account_message(&config, &xml_body).await {
        Ok(reply) => Ok(reply),
        Err(e) => {
            tracing::error!("WeChat message handling error: {}", e);
            // 返回 success 避免微信重复推送
            Ok("success".to_string())
        },
    }
}

// ── subscription handlers ──

async fn list_subscriptions_handler(
    State(state): State<Arc<WebhookServerState>>,
) -> Json<Vec<WebhookSubscription>> {
    Json(state.subscription_manager.list_subscriptions().await)
}

#[derive(serde::Deserialize)]
pub struct CreateSubscriptionRequest {
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
}

async fn create_subscription_handler(
    State(state): State<Arc<WebhookServerState>>,
    Json(req): Json<CreateSubscriptionRequest>,
) -> Result<Json<WebhookSubscription>, StatusCode> {
    let events: Vec<WebhookEvent> = req
        .events
        .iter()
        .filter_map(|e| WebhookEvent::from_event_str(e))
        .collect();
    if events.is_empty() && !req.events.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let subscription = state
        .subscription_manager
        .subscribe(req.url, events, req.secret)
        .await;
    Ok(Json(subscription))
}

async fn delete_subscription_handler(
    State(state): State<Arc<WebhookServerState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, StatusCode> {
    state
        .subscription_manager
        .unsubscribe(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(StatusCode::NO_CONTENT)
}
