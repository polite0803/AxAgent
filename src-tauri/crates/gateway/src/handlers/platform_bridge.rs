// SPDX-License-Identifier: AGPL-3.0-only

//! PlatformBridge — external platform webhook → AI Agent execution pipeline.
//!
//! This module provides the integration point for external platforms (Slack,
//! Discord, custom webhooks, etc.) to trigger AI Agent workflows through the
//! AxAgent gateway.
//!
//! ## Data flow
//!
//! ```text
//! External Platform (e.g. Slack)
//!   │  HTTP POST /api/webhook/{platform}
//!   ▼
//! PlatformBridgeHandler::receive()
//!   │  1. Validate authentication (HMAC-SHA256 or Bearer token)
//!   │  2. Parse platform-specific payload
//!   │  3. Extract message + metadata
//!   ▼
//! SessionRouter::route()
//!   │  Resolve conversation context (session ID, user ID)
//!   ▼
//! AgentDispatcher::dispatch()
//!   │  Forward to target Agent with message + context
//!   ▼
//! AI Agent processes and returns result
//!   │
//!   ▼
//! ResponseFormatter::format_for_platform()
//!   │  Convert Agent output to platform-native format
//!   ▼
//! HTTP 200 { response, handover? }
//! ```

use axum::{
    Extension, Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::server::GatewayAppState;

// ── Platform Types ───────────────────────────────────────────────────

/// Supported external platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// Generic webhook with HMAC-SHA256 verification.
    Webhook,
    /// Slack Events API / slash commands.
    Slack,
    /// Discord Interactions / webhooks.
    Discord,
    /// Microsoft Teams incoming webhook.
    Teams,
    /// Custom HTTP callback.
    Custom,
}

impl Platform {
    pub fn from_path_segment(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "webhook" => Some(Self::Webhook),
            "slack" => Some(Self::Slack),
            "discord" => Some(Self::Discord),
            "teams" => Some(Self::Teams),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

// ── Incoming Message ────────────────────────────────────────────────

/// Unified incoming message structure from any platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMessage {
    /// The text content of the message.
    pub content: String,
    /// Platform-specific identifier for the conversation/channel.
    pub conversation_id: Option<String>,
    /// Platform-specific sender identifier.
    pub sender_id: Option<String>,
    /// Platform-specific sender display name.
    pub sender_name: Option<String>,
    /// Arbitrary platform metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Timestamp of the original message (Unix millis).
    pub timestamp: Option<i64>,
}

// ── PlatformMessage (incoming) ───────────────────────────────────────

/// Incoming webhook payload wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Platform identifier (for routing).
    pub platform: Option<String>,
    /// The message to process.
    pub message: PlatformMessage,
    /// Optional ID of a specific workflow to trigger.
    pub workflow_id: Option<String>,
    /// If true, wait for full Agent response (sync). Default: async fire-and-forget.
    #[serde(default)]
    pub sync: bool,
}

// ── Platform Response ────────────────────────────────────────────────

/// Response returned after processing the platform message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformResponse {
    /// Processing status.
    pub status: PlatformResponseStatus,
    /// Human-readable message.
    pub message: String,
    /// Generated reply content (null for async processing).
    pub reply: Option<String>,
    /// Execution identifier for async tracking.
    pub execution_id: Option<String>,
    /// Structured handover (if applicable).
    pub handover: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformResponseStatus {
    /// Message accepted and queued for async processing.
    Accepted,
    /// Message processed synchronously, result in reply field.
    Completed,
    /// Authentication failed.
    Unauthorized,
    /// Invalid payload.
    InvalidPayload,
    /// Rate limited.
    RateLimited,
    /// Platform not supported.
    UnsupportedPlatform,
}

// ── Webhook Handler ──────────────────────────────────────────────────

/// POST /api/webhook/:platform
///
/// Receives an external platform message, validates it, and routes to
/// the appropriate AI Agent for processing.
///
/// ## Authentication
///
/// - `X-Webhook-Signature`: HMAC-SHA256 hex digest of the request body,
///   keyed with the gateway master key. Used for generic webhooks.
/// - `Authorization: Bearer <token>`: Standard API key auth (used for
///   Slack/Discord bots that send via HTTP).
///
/// ## Routing
///
/// The platform is extracted from the URL path segment. The message
/// payload is parsed, validated, and dispatched to the session router
/// which resolves the target Agent.
pub async fn receive_webhook(
    State(state): State<GatewayAppState>,
    Extension(platform): Extension<Platform>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, (StatusCode, Json<PlatformResponse>)> {
    // 1. 用原始 body 字节做 HMAC 校验（不能用 serde 解析后的 payload）
    let verified = verify_webhook_auth(&headers, &body, &state).await;
    if !verified {
        let resp = PlatformResponse {
            status: PlatformResponseStatus::Unauthorized,
            message: "Invalid webhook signature or missing authorization".to_string(),
            reply: None,
            execution_id: None,
            handover: None,
        };
        return Err((StatusCode::UNAUTHORIZED, Json(resp)));
    }

    // 校验通过后再反序列化 payload
    let payload: WebhookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            let resp = PlatformResponse {
                status: PlatformResponseStatus::InvalidPayload,
                message: format!("Invalid JSON payload: {}", e),
                reply: None,
                execution_id: None,
                handover: None,
            };
            return Err((StatusCode::BAD_REQUEST, Json(resp)));
        },
    };

    // 2. Validate payload
    if payload.message.content.is_empty() {
        let resp = PlatformResponse {
            status: PlatformResponseStatus::InvalidPayload,
            message: "Message content cannot be empty".to_string(),
            reply: None,
            execution_id: None,
            handover: None,
        };
        return Err((StatusCode::BAD_REQUEST, Json(resp)));
    }

    // 3. Generate execution ID for tracking
    let execution_id = uuid::Uuid::new_v4().to_string();

    // 4. Process the message
    let result = process_platform_message(&state, platform, &payload, &execution_id).await;

    match result {
        Ok(reply) => {
            let resp = PlatformResponse {
                status: if payload.sync {
                    PlatformResponseStatus::Completed
                } else {
                    PlatformResponseStatus::Accepted
                },
                message: "Message processed".to_string(),
                reply: if payload.sync { Some(reply) } else { None },
                execution_id: Some(execution_id),
                handover: None,
            };
            Ok((StatusCode::OK, Json(resp)))
        },
        Err(e) => {
            tracing::error!(error = %e, execution_id, "platform message processing failed");
            let resp = PlatformResponse {
                status: PlatformResponseStatus::InvalidPayload,
                message: format!("Processing failed: {}", e),
                reply: None,
                execution_id: Some(execution_id),
                handover: None,
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(resp)))
        },
    }
}

/// POST /api/platform/message
///
/// Direct message endpoint: bypass URL-based platform routing, specify
/// platform in the JSON body. Useful for programmatic API clients.
pub async fn direct_message(
    State(state): State<GatewayAppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, (StatusCode, Json<PlatformResponse>)> {
    // 提前解析一次以拿到 platform 字段；后续再交给 receive_webhook 用同一份 bytes 做 HMAC
    // 校验（注意：HMAC 是针对原始 body 字节，不能用重序列化后的 JSON，否则签名对不上）。
    let probe: WebhookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            let resp = PlatformResponse {
                status: PlatformResponseStatus::InvalidPayload,
                message: format!("Invalid JSON payload: {}", e),
                reply: None,
                execution_id: None,
                handover: None,
            };
            return Err((StatusCode::BAD_REQUEST, Json(resp)));
        },
    };

    let platform_str = probe.platform.as_deref().unwrap_or("webhook");
    let platform = Platform::from_path_segment(platform_str).ok_or_else(|| {
        let resp = PlatformResponse {
            status: PlatformResponseStatus::UnsupportedPlatform,
            message: format!("Unsupported platform: {}", platform_str),
            reply: None,
            execution_id: None,
            handover: None,
        };
        (StatusCode::BAD_REQUEST, Json(resp))
    })?;

    // Re-use the webhook handler with resolved platform — 复用原始 bytes
    receive_webhook(State(state), Extension(platform), headers, body).await
}

/// GET /api/platform/health
///
/// Returns the list of supported platforms and their reachability status.
pub async fn platform_health(State(_state): State<GatewayAppState>) -> impl IntoResponse {
    let platforms = vec![
        serde_json::json!({
            "platform": "webhook",
            "status": "active",
            "description": "Generic HMAC-SHA256 signed webhook"
        }),
        serde_json::json!({
            "platform": "slack",
            "status": "active",
            "description": "Slack Events API / slash commands"
        }),
        serde_json::json!({
            "platform": "discord",
            "status": "active",
            "description": "Discord Interactions / webhooks"
        }),
        serde_json::json!({
            "platform": "teams",
            "status": "active",
            "description": "Microsoft Teams incoming webhook"
        }),
        serde_json::json!({
            "platform": "custom",
            "status": "active",
            "description": "Custom HTTP callback"
        }),
    ];

    Json(serde_json::json!({
        "status": "ok",
        "platforms": platforms
    }))
}

// ── Authentication ──────────────────────────────────────────────────

/// Verify webhook request authenticity using HMAC-SHA256.
///
/// SECURITY (P0-1): 严格校验，杜绝"先返回 true"兜底。
/// 1. `Authorization: Bearer <token>`：与 master_key 派生密钥做常量时间比对
/// 2. `X-Webhook-Signature: hex(hmac_sha256(secret, body))`：用 master_key 计算
///    HMAC，再与签名头做常量时间比对
async fn verify_webhook_auth(headers: &HeaderMap, body: &[u8], state: &GatewayAppState) -> bool {
    // Bearer token：常量时间比对预期密钥
    if let Some(auth_header) = headers.get("authorization")
        && let Ok(auth_str) = auth_header.to_str()
        && let Some(token) = auth_str.strip_prefix("Bearer ")
        && !token.is_empty()
    {
        // 期望密钥：用 master_key 派生一段长为 32 字节的子密钥
        let mut expected = [0u8; 32];
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&state.master_key)
            .expect("HMAC accepts any key length");
        mac.update(b"bearer-token-v1");
        let bytes = mac.finalize().into_bytes();
        expected.copy_from_slice(&bytes);

        // 客户端 token 先 hex 解析 → 字节级常量时间比对
        if let Ok(token_bytes) = hex::decode(token.trim()) {
            return bool::from(token_bytes.ct_eq(&expected));
        }
        return false;
    }

    // HMAC 签名：必须用 master_key 真实计算 HMAC
    if let Some(sig_header) = headers.get("x-webhook-signature")
        && let Ok(sig_hex) = sig_header.to_str()
    {
        if sig_hex.len() != 64 || !sig_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
        let Ok(sig_bytes) = hex::decode(sig_hex) else {
            return false;
        };

        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&state.master_key)
            .expect("HMAC accepts any key length");
        mac.update(body);
        let computed = mac.finalize().into_bytes();

        return bool::from(computed.ct_eq(&sig_bytes));
    }

    false
}

// ── Message Processing Pipeline ──────────────────────────────────────

/// Process a platform message through the Agent execution pipeline.
async fn process_platform_message(
    state: &GatewayAppState,
    platform: Platform,
    payload: &WebhookPayload,
    execution_id: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(
        platform = ?platform,
        execution_id,
        content_len = payload.message.content.len(),
        sync = payload.sync,
        "processing platform message"
    );

    // Build the system prompt for the executing Agent
    let system_prompt = match platform {
        Platform::Slack => format!(
            "You are an AI assistant responding to a Slack message. \
             Sender: {}. Conversation: {}. Reply concisely in Slack-friendly format.",
            payload.message.sender_name.as_deref().unwrap_or("unknown"),
            payload
                .message
                .conversation_id
                .as_deref()
                .unwrap_or("unknown"),
        ),
        Platform::Discord => format!(
            "You are an AI assistant responding to a Discord message. \
             Sender: {}. Channel: {}. Use Discord markdown formatting.",
            payload.message.sender_name.as_deref().unwrap_or("unknown"),
            payload
                .message
                .conversation_id
                .as_deref()
                .unwrap_or("unknown"),
        ),
        _ => format!(
            "You are an AI assistant responding to a message from platform {:?}. \
             Reply directly and helpfully.",
            platform
        ),
    };

    // Dispatch to the conversation system through the platform adapter
    // The adapter resolves: model selection → prompt construction → LLM call
    let response = state
        .adapter
        .chat_completion(axagent_harness::platform_adapter::ChatCompletionParams {
            system_prompt,
            message: payload.message.content.clone(),
            platform: format!("{:?}", platform),
            workflow_id: payload.workflow_id.clone(),
        })
        .await
        .map_err(|e| format!("Agent dispatch failed: {}", e))?;

    tracing::info!(
        execution_id,
        response_len = response.len(),
        "platform message processed successfully"
    );

    Ok(response)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_from_path_segment() {
        assert_eq!(Platform::from_path_segment("slack"), Some(Platform::Slack));
        assert_eq!(Platform::from_path_segment("discord"), Some(Platform::Discord));
        assert_eq!(Platform::from_path_segment("webhook"), Some(Platform::Webhook));
        assert_eq!(Platform::from_path_segment("teams"), Some(Platform::Teams));
        assert_eq!(Platform::from_path_segment("unknown"), None);
        assert_eq!(Platform::from_path_segment("SLACK"), Some(Platform::Slack));
    }

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            platform: Some("slack".to_string()),
            message: PlatformMessage {
                content: "Hello world".to_string(),
                conversation_id: Some("C123".to_string()),
                sender_id: Some("U456".to_string()),
                sender_name: Some("Alice".to_string()),
                metadata: serde_json::json!({"channel": "general"}),
                timestamp: Some(1700000000000),
            },
            workflow_id: None,
            sync: false,
        };

        let json = serde_json::to_string(&payload).unwrap();
        let parsed: WebhookPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.message.content, "Hello world");
        assert_eq!(parsed.platform, Some("slack".to_string()));
        assert!(!parsed.sync);
    }

    #[test]
    fn test_valid_signature_format() {
        // A valid hex HMAC-SHA256 is 64 hex chars
        let valid_sig = "a".repeat(64);
        assert!(valid_sig.len() == 64);
        assert!(valid_sig.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
