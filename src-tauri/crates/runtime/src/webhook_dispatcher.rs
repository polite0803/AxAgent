use crate::webhook_subscription::{
    WebhookEvent, WebhookPayload, WebhookSubscription, WebhookSubscriptionManager,
};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;

pub struct WebhookDispatcher {
    subscription_manager: Arc<WebhookSubscriptionManager>,
    client: Client,
}

impl WebhookDispatcher {
    pub fn new(subscription_manager: Arc<WebhookSubscriptionManager>) -> Self {
        Self {
            subscription_manager,
            client: Client::new(),
        }
    }

    pub async fn dispatch(
        &self,
        event: WebhookEvent,
        data: HashMap<String, serde_json::Value>,
    ) -> DispatchResult {
        let payload = WebhookPayload {
            id: uuid::Uuid::new_v4().to_string(),
            event,
            timestamp: chrono::Utc::now(),
            data,
        };
        let subscriptions = self
            .subscription_manager
            .get_subscriptions_for_event(payload.event)
            .await;
        if subscriptions.is_empty() {
            return DispatchResult {
                success_count: 0,
                failure_count: 0,
                errors: vec![],
            };
        }
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut errors = Vec::new();
        for subscription in subscriptions {
            let result = self.send_webhook(&subscription, &payload).await;
            match result {
                Ok(_) => {
                    success_count += 1;
                    self.subscription_manager
                        .reset_failures(&subscription.id)
                        .await;
                }
                Err(e) => {
                    failure_count += 1;
                    errors.push(e.clone());
                    self.subscription_manager
                        .increment_failure(&subscription.id)
                        .await;
                    tracing::error!("Webhook dispatch failed for {}: {}", subscription.id, e);
                }
            }
        }
        DispatchResult {
            success_count,
            failure_count,
            errors,
        }
    }

    async fn send_webhook(
        &self,
        subscription: &WebhookSubscription,
        payload: &WebhookPayload,
    ) -> Result<(), String> {
        let json = serde_json::to_string(payload).map_err(|e| e.to_string())?;
        let mut request = self
            .client
            .post(&subscription.url)
            .header("Content-Type", "application/json");
        if let Some(secret) = &subscription.secret {
            let signature = Self::generate_signature(json.as_bytes(), secret);
            request = request.header("X-Webhook-Signature", signature);
        }
        let response = request
            .header("X-Webhook-Event", payload.event.as_str())
            .header("X-Webhook-Delivery", &payload.id)
            .body(json)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("Webhook returned status: {}", response.status()));
        }
        self.subscription_manager
            .update_last_triggered(&subscription.id)
            .await;
        Ok(())
    }

    fn generate_signature(payload: &[u8], secret: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(payload);
        hasher.update(secret.as_bytes());
        let result = hasher.finalize();
        format!("sha256={}", hex::encode(result))
    }
}

#[derive(Debug, Clone)]
pub struct DispatchResult {
    pub success_count: usize,
    pub failure_count: usize,
    pub errors: Vec<String>,
}

pub struct WebhookEventEmitter {
    dispatcher: Arc<WebhookDispatcher>,
}

impl WebhookEventEmitter {
    pub fn new(dispatcher: Arc<WebhookDispatcher>) -> Self {
        Self { dispatcher }
    }

    pub async fn emit_tool_complete(
        &self,
        tool_name: &str,
        args: HashMap<String, serde_json::Value>,
        result: &str,
    ) {
        let mut data = HashMap::new();
        data.insert("tool_name".to_string(), serde_json::json!(tool_name));
        data.insert("arguments".to_string(), serde_json::json!(args));
        data.insert("result".to_string(), serde_json::json!(result));
        self.dispatcher
            .dispatch(WebhookEvent::ToolComplete, data)
            .await;
    }

    pub async fn emit_tool_error(&self, tool_name: &str, error: &str) {
        let mut data = HashMap::new();
        data.insert("tool_name".to_string(), serde_json::json!(tool_name));
        data.insert("error".to_string(), serde_json::json!(error));
        self.dispatcher
            .dispatch(WebhookEvent::ToolError, data)
            .await;
    }

    pub async fn emit_agent_error(&self, session_id: &str, error: &str) {
        let mut data = HashMap::new();
        data.insert("session_id".to_string(), serde_json::json!(session_id));
        data.insert("error".to_string(), serde_json::json!(error));
        self.dispatcher
            .dispatch(WebhookEvent::AgentError, data)
            .await;
    }

    pub async fn emit_agent_end(&self, session_id: &str, outcome: &str) {
        let mut data = HashMap::new();
        data.insert("session_id".to_string(), serde_json::json!(session_id));
        data.insert("outcome".to_string(), serde_json::json!(outcome));
        self.dispatcher.dispatch(WebhookEvent::AgentEnd, data).await;
    }

    pub async fn emit_session_end(&self, session_id: &str) {
        let mut data = HashMap::new();
        data.insert("session_id".to_string(), serde_json::json!(session_id));
        self.dispatcher
            .dispatch(WebhookEvent::SessionEnd, data)
            .await;
    }
}
