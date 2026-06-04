//! Webhook subscription and dispatch system.
//!
//! 订阅管理通过 `axagent-harness::WebhookSubscriptionService` 契约解耦，
//! 具体实现（`WebhookSubscriptionManager`）由 `axagent-rt-messaging` 提供。

pub mod webhook_dispatcher;
pub mod webhook_server;

/// Re-export the harness trait for callers that need to inject the service.
pub use axagent_harness::WebhookSubscriptionService;
