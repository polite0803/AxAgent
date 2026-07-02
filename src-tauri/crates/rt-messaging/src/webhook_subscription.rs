// SPDX-License-Identifier: AGPL-3.0-only

//! Webhook 订阅管理 — DTO 由 `axagent-harness` 提供。
//!
//! 纯数据 DTO（WebhookEvent / WebhookSubscription / WebhookPayload / DispatchResult）
//! 定义在 `axagent-harness::webhook_subscription`，此处仅做 re-export。
//! `WebhookDispatch` trait 向下兼容，新代码请直接使用 `axagent_harness::*`。

pub use axagent_harness::{DispatchResult, WebhookEvent, WebhookPayload, WebhookSubscription};

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use tokio::net::lookup_host;
use tokio::sync::RwLock;

/// P0-5: SSRF 校验 —— 检查 URL 主机名解析出的 IP 是否在黑名单中。
/// 拒绝：loopback、私网、link-local（含 169.254/16 云元数据）、IPv6 link-local、
/// unique local、multicast、未指定地址。
pub fn is_blocked_ip(ip: &IpAddr) -> bool {
    if ip.is_loopback() {
        return true;
    }
    match ip {
        IpAddr::V4(v4) => {
            if v4.is_private()
                || v4.is_link_local() // 169.254.0.0/16 — 含 169.254.169.254
                || v4.is_unspecified()
                || v4.is_multicast()
                || v4.is_broadcast()
            {
                return true;
            }
        },
        IpAddr::V6(v6) => {
            if v6.is_unicast_link_local()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // fc00::/7
                || v6.is_unspecified()
                || v6.is_multicast()
            {
                return true;
            }
            // IPv4-mapped IPv6 (::ffff:0:0/96) 也要校验内嵌的 v4
            if let Some(v4) = ipv4_mapped_from_v6(v6)
                && is_blocked_ip(&IpAddr::V4(v4))
            {
                return true;
            }
        },
    }
    false
}

fn ipv4_mapped_from_v6(v6: &Ipv6Addr) -> Option<Ipv4Addr> {
    let seg = v6.segments();
    if seg[0] == 0 && seg[1] == 0 && seg[2] == 0 && seg[3] == 0 && seg[4] == 0 && seg[5] == 0xffff {
        let octets = v6.octets();
        return Some(Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]));
    }
    None
}

/// P0-5: 严格 SSRF 校验：解析 URL → 校验 scheme → DNS lookup → 检查所有解析 IP。
/// 异步版本，避免阻塞调用方。
pub async fn assert_url_safe(url: &str, require_https: bool) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;
    let scheme = parsed.scheme();
    if require_https {
        if scheme != "https" {
            return Err("URL must use HTTPS".to_string());
        }
    } else if scheme != "http" && scheme != "https" {
        return Err(format!("URL scheme '{scheme}' is not allowed (only http/https)"));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL has no host".to_string())?;
    if host.is_empty() {
        return Err("URL has empty host".to_string());
    }

    // DNS 解析所有地址，任一在黑名单就拒绝
    let addrs: Vec<IpAddr> = lookup_host((host, 0u16))
        .await
        .map_err(|e| format!("DNS lookup failed for {host}: {e}"))?
        .map(|sa| sa.ip())
        .collect();
    if addrs.is_empty() {
        return Err(format!("DNS lookup for {host} returned no addresses"));
    }
    for ip in &addrs {
        if is_blocked_ip(ip) {
            return Err(format!(
                "URL points to blocked IP {ip} (loopback/private/link-local/cloud-metadata/multicast)"
            ));
        }
    }
    Ok(())
}

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
        // P0-5: 用 DNS 解析 + IpAddr 黑名单做严格 SSRF 校验，覆盖 link-local/云元数据
        assert_url_safe(&url, true).await?;

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

    async fn get_subscriptions_for_event(
        &self,
        event: &str,
    ) -> Vec<axagent_harness::WebhookSubscriptionInfo> {
        let event_enum = WebhookEvent::from_event_str(event);
        let Some(event_enum) = event_enum else {
            return Vec::new();
        };
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
                event: s
                    .events
                    .first()
                    .map(|e| e.as_str().to_string())
                    .unwrap_or_default(),
                enabled: s.enabled,
            })
            .collect()
    }
}
