// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::platform as platform_err;
use axagent_runtime::webhook_subscription::{WebhookEvent, WebhookSubscription};
use tauri::State;

#[derive(Debug, Clone, serde::Serialize)]
pub struct WebhookSubscriptionResponse {
    id: String,
    url: String,
    events: Vec<String>,
    secret: Option<String>,
    enabled: bool,
    created_at: String,
    last_triggered: Option<String>,
    failure_count: u32,
}

impl From<WebhookSubscription> for WebhookSubscriptionResponse {
    fn from(sub: WebhookSubscription) -> Self {
        Self {
            id: sub.id,
            url: sub.url,
            events: sub
                .events
                .iter()
                .map(|e: &WebhookEvent| e.as_str().to_string())
                .collect(),
            secret: sub.secret,
            enabled: sub.enabled,
            created_at: sub.created_at.to_rfc3339(),
            last_triggered: sub.last_triggered.map(|t| t.to_rfc3339()),
            failure_count: sub.failure_count,
        }
    }
}

#[tauri::command]
pub async fn webhook_list_subscriptions(
    state: State<'_, AppState>,
) -> Result<Vec<WebhookSubscriptionResponse>, String> {
    let manager = state.webhook_subscription_manager.as_ref().ok_or_else(|| {
        ErrorResponse::err_with_detail(
            platform_err::WEBHOOK_NOT_CONFIGURED,
            "Webhook subscription manager not initialized",
        )
    })?;
    let subscriptions = manager.list_subscriptions().await;
    Ok(subscriptions.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn webhook_create_subscription(
    state: State<'_, AppState>,
    url: String,
    events: Vec<String>,
    secret: Option<String>,
) -> Result<WebhookSubscriptionResponse, String> {
    {
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err("Webhook URL must use http or https scheme".to_string());
        }
        let after_scheme = url.split("://").nth(1).unwrap_or(&url);
        let host_part = after_scheme.split('/').next().unwrap_or(after_scheme);
        let host = host_part
            .split(':')
            .next()
            .unwrap_or(host_part)
            .to_lowercase();
        if host == "localhost"
            || host == "127.0.0.1"
            || host == "::1"
            || host.starts_with("169.254.")
            || host.starts_with("10.")
            || host.starts_with("192.168.")
            || (host.starts_with("172.")
                && host
                    .split('.')
                    .nth(1)
                    .is_some_and(|o| o.parse::<u8>().is_ok_and(|n| (16..=31).contains(&n))))
            || host.starts_with("fc")
            || host.starts_with("fd")
            || host.starts_with("fe80:")
        {
            return Err("Webhook URL must not point to a private/reserved address".to_string());
        }
    }
    let manager = state.webhook_subscription_manager.as_ref().ok_or_else(|| {
        ErrorResponse::err_with_detail(
            platform_err::WEBHOOK_NOT_CONFIGURED,
            "Webhook subscription manager not initialized",
        )
    })?;
    let webhook_events: Vec<WebhookEvent> = events
        .iter()
        .filter_map(|e| WebhookEvent::from_event_str(e))
        .collect();
    let subscription = manager.subscribe(url, webhook_events, secret).await?;
    Ok(subscription.into())
}

#[tauri::command]
pub async fn webhook_delete_subscription(
    state: State<'_, AppState>,
    subscription_id: String,
) -> Result<(), String> {
    let manager = state.webhook_subscription_manager.as_ref().ok_or_else(|| {
        ErrorResponse::err_with_detail(
            platform_err::WEBHOOK_NOT_CONFIGURED,
            "Webhook subscription manager not initialized",
        )
    })?;
    manager.unsubscribe(&subscription_id).await
}

#[tauri::command]
pub async fn webhook_toggle_subscription(
    state: State<'_, AppState>,
    subscription_id: String,
    enabled: bool,
) -> Result<(), String> {
    let manager = state.webhook_subscription_manager.as_ref().ok_or_else(|| {
        ErrorResponse::err_with_detail(
            platform_err::WEBHOOK_NOT_CONFIGURED,
            "Webhook subscription manager not initialized",
        )
    })?;
    manager.set_enabled(&subscription_id, enabled).await
}

#[tauri::command]
pub async fn webhook_test_subscription(
    state: State<'_, AppState>,
    subscription_id: String,
) -> Result<(), String> {
    let manager = state.webhook_subscription_manager.as_ref().ok_or_else(|| {
        ErrorResponse::err_with_detail(
            platform_err::WEBHOOK_NOT_CONFIGURED,
            "Webhook subscription manager not initialized",
        )
    })?;
    manager.test_subscription(&subscription_id).await
}

#[tauri::command]
pub async fn webhook_reload(state: State<'_, AppState>) -> Result<(), String> {
    let manager = state.webhook_subscription_manager.as_ref().ok_or_else(|| {
        ErrorResponse::err_with_detail(
            platform_err::WEBHOOK_NOT_CONFIGURED,
            "Webhook subscription manager not initialized",
        )
    })?;
    manager.reload().await
}
