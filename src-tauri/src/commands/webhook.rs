// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::platform as platform_err;
use axagent_runtime::webhook_subscription::{WebhookEvent, WebhookSubscription, assert_url_safe};
use tauri::State;
use agent_macro::agent_command;

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
            events: sub.events.iter().map(|e: &WebhookEvent| e.as_str().to_string()).collect(),
            secret: sub.secret,
            enabled: sub.enabled,
            created_at: sub.created_at.to_rfc3339(),
            last_triggered: sub.last_triggered.map(|t| t.to_rfc3339()),
            failure_count: sub.failure_count,
        }
    }
}

#[agent_command(domain = webhook, safety = Safe, call_mode = StateOnly, description = "列出 Webhook 订阅")]
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

#[agent_command(domain = webhook, safety = Caution, call_mode = StateInput, description = "创建 Webhook 订阅")]
#[tauri::command]
pub async fn webhook_create_subscription(
    state: State<'_, AppState>,
    url: String,
    events: Vec<String>,
    secret: Option<String>,
) -> Result<WebhookSubscriptionResponse, String> {
    {
        // P0-5: 用 DNS 解析 + IpAddr 黑名单做严格 SSRF 校验，覆盖 link-local/云元数据/私网
        // subscription 路径在 manager.subscribe 内部会再次校验（双保险）。
        assert_url_safe(&url, false).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }
    let manager = state.webhook_subscription_manager.as_ref().ok_or_else(|| {
        ErrorResponse::err_with_detail(
            platform_err::WEBHOOK_NOT_CONFIGURED,
            "Webhook subscription manager not initialized",
        )
    })?;
    let webhook_events: Vec<WebhookEvent> =
        events.iter().filter_map(|e| WebhookEvent::from_event_str(e)).collect();
    let subscription = manager.subscribe(url, webhook_events, secret).await?;
    Ok(subscription.into())
}

#[agent_command(domain = webhook, safety = Dangerous, call_mode = StateInput, description = "删除 Webhook 订阅")]
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

#[agent_command(domain = webhook, safety = Caution, call_mode = StateInput, description = "切换 Webhook 订阅启用状态")]
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

#[agent_command(domain = webhook, safety = Safe, call_mode = StateInput, description = "测试 Webhook 订阅")]
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

#[agent_command(domain = webhook, safety = Caution, call_mode = StateOnly, description = "重新加载 Webhook 订阅")]
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
