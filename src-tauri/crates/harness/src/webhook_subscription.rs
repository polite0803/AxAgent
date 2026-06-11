// SPDX-License-Identifier: AGPL-3.0-only

//! Webhook 订阅服务契约 + 共享 DTO。
//!
//! 提供 Webhook 订阅的注册、查询、事件派发能力，
//! 以及 `WebhookEvent` / `WebhookSubscription` / `WebhookPayload` 等纯数据 DTO。
//! 实现方（`axagent-rt-messaging`）管理 Webhook 订阅的生命周期。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ── 事件枚举 ─────────────────────────────────────────────

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

// ── Webhook 订阅 DTO ─────────────────────────────────────

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub event: WebhookEvent,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResult {
    pub success_count: usize,
    pub failure_count: usize,
    pub errors: Vec<String>,
}

// ── 服务契约 ─────────────────────────────────────────────

/// Webhook 订阅管理服务契约
#[async_trait::async_trait]
pub trait WebhookSubscriptionService: fmt::Debug + Send + Sync {
    /// 订阅 webhook
    async fn subscribe(
        &self,
        url: String,
        event: &str,
        secret: Option<String>,
    ) -> Result<WebhookSubscriptionInfo, String>;

    /// 获取指定事件类型的订阅列表
    async fn get_subscriptions_for_event(&self, event: &str) -> Vec<WebhookSubscriptionInfo>;

    /// 取消订阅
    async fn unsubscribe(&self, subscription_id: &str) -> Result<(), String>;

    /// 重置订阅的失败计数
    async fn reset_failures(&self, subscription_id: &str);

    /// 增加订阅的失败计数
    async fn increment_failure(&self, subscription_id: &str);

    /// 更新最后触发时间
    async fn update_last_triggered(&self, subscription_id: &str);

    /// 列出所有订阅
    async fn list_subscriptions(&self) -> Vec<WebhookSubscriptionInfo>;
}

/// Webhook 订阅信息（纯数据 DTO）
#[derive(Debug, Clone)]
pub struct WebhookSubscriptionInfo {
    pub id: String,
    pub url: String,
    pub secret: Option<String>,
    pub event: String,
    pub enabled: bool,
}

// ── 空实现 — 什么也不做 ────────────────────────────────

#[derive(Debug)]
pub struct NoopWebhookSubscriptionService;

#[async_trait::async_trait]
impl WebhookSubscriptionService for NoopWebhookSubscriptionService {
    async fn subscribe(
        &self,
        _url: String,
        _event: &str,
        _secret: Option<String>,
    ) -> Result<WebhookSubscriptionInfo, String> {
        Err("Webhook subscription service is not configured".to_string())
    }

    async fn get_subscriptions_for_event(&self, _event: &str) -> Vec<WebhookSubscriptionInfo> {
        Vec::new()
    }

    async fn unsubscribe(&self, _subscription_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn reset_failures(&self, _subscription_id: &str) {}

    async fn increment_failure(&self, _subscription_id: &str) {}

    async fn update_last_triggered(&self, _subscription_id: &str) {}

    async fn list_subscriptions(&self) -> Vec<WebhookSubscriptionInfo> {
        Vec::new()
    }
}
