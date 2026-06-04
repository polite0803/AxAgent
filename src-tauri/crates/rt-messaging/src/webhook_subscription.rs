//! Webhook 订阅管理 — DTO 由 `axagent-harness` 提供。
//!
//! 纯数据 DTO（WebhookEvent / WebhookSubscription / WebhookPayload / DispatchResult）
//! 定义在 `axagent-harness::webhook_subscription`，此处仅做 re-export。
//! `WebhookDispatch` trait 向下兼容，新代码请直接使用 `axagent_harness::*`。

pub use axagent_harness::{
    DispatchResult, WebhookEvent, WebhookPayload, WebhookSubscription,
};

use std::sync::Arc;
use tokio::sync::RwLock;

/// Webhook 事件派发 trait（纯数据 DTO 已迁至 harness）
#[async_trait::async_trait]
pub trait WebhookDispatch: Send + Sync {
    async fn dispatch(
        &self,
        event: WebhookEvent,
        data: std::collections::HashMap<String, serde_json::Value>,
    );
}

/// Webhook 订阅管理器 — 管理生命周期和事件派发
#[derive(Debug)]
pub struct WebhookSubscriptionManager {
    subscriptions: Arc<RwLock<std::collections::HashMap<String, WebhookSubscription>>>,
}

impl Default for WebhookSubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookSubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn subscribe(
        &self,
        url: String,
        events: Vec<WebhookEvent>,
        secret: Option<String>,
    ) -> Result<WebhookSubscription, String> {
        let parsed_url =
            url::Url::parse(&url).map_err(|e| format!("Invalid webhook URL: {}", e))?;
        if parsed_url.scheme() != "https" {
            return Err("Webhook URL must use HTTPS".to_string());
        }
        let host = parsed_url.host_str().unwrap_or("");
        if host == "localhost" || host == "127.0.0.1" || host == "::1" {
            return Err("Webhook URL cannot point to localhost".to_string());
        }
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            let is_restricted = match ip {
                std::net::IpAddr::V4(v4) => {
                    v4.is_loopback() || v4.is_private() || v4.is_link_local()
                },
                std::net::IpAddr::V6(v6) => v6.is_loopback(),
            };
            if is_restricted {
                return Err("Webhook URL cannot point to a private/internal address".to_string());
            }
        }

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
        Ok(subscription)
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
            tracing::info!("Webhook {} {}", id, if enabled { "enabled" } else { "disabled" });
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

// ── Harness WebhookSubscriptionService trait 实现 ──

#[async_trait::async_trait]
impl axagent_harness::WebhookSubscriptionService for WebhookSubscriptionManager {
    async fn subscribe(
        &self,
        url: String,
        event: &str,
        secret: Option<String>,
    ) -> Result<axagent_harness::WebhookSubscriptionInfo, String> {
        let event_enum = WebhookEvent::from_event_str(event)
            .ok_or_else(|| format!("Unknown webhook event: {event}"))?;
        let sub = self.subscribe(url, vec![event_enum], secret).await?;
        Ok(axagent_harness::WebhookSubscriptionInfo {
            id: sub.id,
            url: sub.url,
            secret: sub.secret,
            event: event.to_string(),
            enabled: sub.enabled,
        })
    }

    async fn get_subscriptions_for_event(&self, event: &str) -> Vec<axagent_harness::WebhookSubscriptionInfo> {
        let event_enum = WebhookEvent::from_event_str(event);
        let Some(event_enum) = event_enum else { return Vec::new(); };
        self.get_subscriptions_for_event(event_enum)
            .await
            .into_iter()
            .map(|s| axagent_harness::WebhookSubscriptionInfo {
                id: s.id,
                url: s.url,
                secret: s.secret.clone(),
                event: event.to_string(),
                enabled: s.enabled,
            })
            .collect()
    }

    async fn unsubscribe(&self, subscription_id: &str) -> Result<(), String> {
        self.unsubscribe(subscription_id).await
    }

    async fn reset_failures(&self, subscription_id: &str) {
        self.reset_failures(subscription_id).await;
    }

    async fn increment_failure(&self, subscription_id: &str) {
        self.increment_failure(subscription_id).await;
    }

    async fn update_last_triggered(&self, subscription_id: &str) {
        self.update_last_triggered(subscription_id).await;
    }

    async fn list_subscriptions(&self) -> Vec<axagent_harness::WebhookSubscriptionInfo> {
        self.list_subscriptions()
            .await
            .into_iter()
            .map(|s| axagent_harness::WebhookSubscriptionInfo {
                id: s.id,
                url: s.url,
                secret: s.secret.clone(),
                event: s.events.first().map(|e| e.as_str().to_string()).unwrap_or_default(),
                enabled: s.enabled,
            })
            .collect()
    }
}
