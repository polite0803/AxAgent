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
            },
            shutdown_tx,
        }
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
