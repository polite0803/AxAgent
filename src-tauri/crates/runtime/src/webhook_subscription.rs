use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: String,
    pub url: String,
    pub events: Vec<WebhookEvent>,
    pub secret: Option<String>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_triggered: Option<chrono::DateTime<chrono::Utc>>,
    pub failure_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebhookEvent {
    ToolComplete,
    ToolError,
    AgentError,
    AgentStart,
    AgentEnd,
    SessionStart,
    SessionEnd,
    MessageReceived,
    MessageSent,
}

impl WebhookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ToolComplete => "tool_complete",
            Self::ToolError => "tool_error",
            Self::AgentError => "agent_error",
            Self::AgentStart => "agent_start",
            Self::AgentEnd => "agent_end",
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
            Self::MessageReceived => "message_received",
            Self::MessageSent => "message_sent",
        }
    }

    pub fn from_event_str(s: &str) -> Option<Self> {
        match s {
            "tool_complete" => Some(Self::ToolComplete),
            "tool_error" => Some(Self::ToolError),
            "agent_error" => Some(Self::AgentError),
            "agent_start" => Some(Self::AgentStart),
            "agent_end" => Some(Self::AgentEnd),
            "session_start" => Some(Self::SessionStart),
            "session_end" => Some(Self::SessionEnd),
            "message_received" => Some(Self::MessageReceived),
            "message_sent" => Some(Self::MessageSent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub event: WebhookEvent,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: HashMap<String, serde_json::Value>,
}

pub struct WebhookSubscriptionManager {
    subscriptions: Arc<RwLock<HashMap<String, WebhookSubscription>>>,
}

impl Default for WebhookSubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookSubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(
        &self,
        url: String,
        events: Vec<WebhookEvent>,
        secret: Option<String>,
    ) -> WebhookSubscription {
        let subscription = WebhookSubscription {
            id: uuid::Uuid::new_v4().to_string(),
            url,
            events,
            secret,
            enabled: true,
            created_at: chrono::Utc::now(),
            last_triggered: None,
            failure_count: 0,
        };
        self.subscriptions
            .write()
            .await
            .insert(subscription.id.clone(), subscription.clone());
        tracing::info!(
            "Webhook subscribed: {} for {} events",
            subscription.id,
            subscription.events.len()
        );
        subscription
    }

    pub async fn unsubscribe(&self, id: &str) -> Result<(), String> {
        if self.subscriptions.write().await.remove(id).is_some() {
            tracing::info!("Webhook unsubscribed: {}", id);
            Ok(())
        } else {
            Err(format!("Subscription '{}' not found", id))
        }
    }

    pub async fn get_subscription(&self, id: &str) -> Option<WebhookSubscription> {
        self.subscriptions.read().await.get(id).cloned()
    }

    pub async fn list_subscriptions(&self) -> Vec<WebhookSubscription> {
        self.subscriptions.read().await.values().cloned().collect()
    }

    pub async fn get_subscriptions_for_event(
        &self,
        event: WebhookEvent,
    ) -> Vec<WebhookSubscription> {
        self.subscriptions
            .read()
            .await
            .values()
            .filter(|s| s.enabled && s.events.contains(&event))
            .cloned()
            .collect()
    }

    pub async fn update_last_triggered(&self, id: &str) {
        if let Some(sub) = self.subscriptions.write().await.get_mut(id) {
            sub.last_triggered = Some(chrono::Utc::now());
        }
    }

    pub async fn increment_failure(&self, id: &str) {
        if let Some(sub) = self.subscriptions.write().await.get_mut(id) {
            sub.failure_count += 1;
            if sub.failure_count >= 5 {
                sub.enabled = false;
                tracing::warn!("Webhook {} disabled due to repeated failures", id);
            }
        }
    }

    pub async fn reset_failures(&self, id: &str) {
        if let Some(sub) = self.subscriptions.write().await.get_mut(id) {
            sub.failure_count = 0;
        }
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        if let Some(sub) = self.subscriptions.write().await.get_mut(id) {
            sub.enabled = enabled;
            tracing::info!(
                "Webhook {} {}",
                id,
                if enabled { "enabled" } else { "disabled" }
            );
            Ok(())
        } else {
            Err(format!("Subscription '{}' not found", id))
        }
    }

    pub async fn test_subscription(&self, id: &str) -> Result<(), String> {
        if let Some(sub) = self.subscriptions.read().await.get(id) {
            tracing::info!("Testing webhook subscription: {} at {}", id, sub.url);
            Ok(())
        } else {
            Err(format!("Subscription '{}' not found", id))
        }
    }

    pub async fn reload(&self) -> Result<(), String> {
        tracing::info!("Reloading webhook subscriptions");
        Ok(())
    }
}
