// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    Json,
    extract::{
        Extension, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::AuthenticatedKey;
use crate::realtime_ticket::TicketStore;
use crate::server::GatewayAppState;

use std::sync::Arc;
use std::time::Duration;

// --- Client → Server messages ---

#[derive(Deserialize)]
#[serde(tag = "type")]
enum RealtimeClientMessage {
    #[serde(rename = "session.create")]
    SessionCreate { model: String },
    #[serde(rename = "input_audio_buffer.append")]
    AudioAppend { audio: String },
    #[serde(rename = "input_audio_buffer.commit")]
    AudioCommit,
    #[serde(rename = "session.close")]
    SessionClose,
}

// --- Server → Client messages ---

#[derive(Serialize)]
#[serde(tag = "type")]
enum RealtimeServerMessage {
    #[serde(rename = "session.created")]
    SessionCreated { session_id: String },
    #[serde(rename = "response.text.delta")]
    TextDelta { delta: String },
    #[serde(rename = "response.done")]
    ResponseDone,
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Deserialize)]
pub struct RealtimeQuery {
    ticket: Option<String>,
}

/// Build a 401 JSON response.
fn unauth(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": {
                "message": message,
                "type": "invalid_request_error",
                "code": "invalid_api_key"
            }
        })),
    )
        .into_response()
}

/// GET /v1/realtime — WebSocket upgrade with ticket-based auth.
///
/// SECURITY (P0-2.2): the long-lived API key must never appear in the upgrade
/// URL (it would be logged by proxies / Referer / browser history). Clients
/// exchange a Bearer token for a short-lived single-use ticket via
/// `POST /v1/realtime-ticket` first.
pub async fn realtime_handler(
    State(state): State<GatewayAppState>,
    Query(params): Query<RealtimeQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let ticket_id = match params.ticket {
        Some(t) if !t.is_empty() => t,
        _ => return unauth("Missing or invalid ticket query parameter"),
    };

    // Consume the ticket. Single-use and TTL-bounded — replay or expiry both
    // return None and we fall through to the 401.
    let ticket = match state.ticket_store.consume(&ticket_id).await {
        Some(t) => t,
        None => return unauth("Invalid, expired, or already-used ticket"),
    };

    // 二次校验：ticket 已被 consume，证明它在 issue 时绑定到一个有效 key；
    // 但 issue→consume 30s 窗口内 key 可能被禁用（revoked），所以这里仍要
    // 重新查一次 DB 确认 key 仍然存在且 enabled。
    let key = match state.adapter.gateway_keys().get_by_id(&ticket.key_id).await {
        Ok(Some(k)) if k.enabled => k,
        _ => return unauth("API key not found or disabled"),
    };

    // Update last_used_at in background
    let adapter_bg = state.adapter.clone();
    let key_id = key.id.clone();
    tokio::spawn(async move {
        let _ = adapter_bg.gateway_keys().update_last_used(&key_id).await;
    });

    ws.on_upgrade(move |socket| handle_realtime_session(socket, state.db))
}

/// POST /v1/realtime-ticket — issue a short-lived ticket for the WS upgrade.
///
/// Caller must already present a valid Bearer API key (auth_middleware puts
/// the resolved key in `AuthenticatedKey`). The returned ticket can be
/// passed to `/v1/realtime?ticket=...` once.
pub async fn issue_realtime_ticket(
    Extension(store): Extension<Arc<TicketStore>>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> Response {
    let ticket = store.issue(auth.0.id).await;
    (
        StatusCode::OK,
        Json(json!({
            "ticket": ticket.ticket_id,
            "expires_in_secs": TICKET_TTL_SECS,
        })),
    )
        .into_response()
}

/// Lifetime of issued tickets. Long enough for a client to receive the
/// response, read the ticket, and open the WS upgrade — but short enough
/// that a leaked ticket (logs, browser history) is hard to weaponise.
pub const TICKET_TTL_SECS: u64 = 30;

/// Convenience: build a fresh `TicketStore` with the default TTL.
pub fn default_ticket_store() -> Arc<TicketStore> {
    Arc::new(TicketStore::new(Duration::from_secs(TICKET_TTL_SECS)))
}

async fn handle_realtime_session(mut socket: WebSocket, _db: DatabaseConnection) {
    let session_id = uuid::Uuid::new_v4().to_string();
    let mut audio_buffer: Vec<String> = Vec::new();
    let mut _model: Option<String> = None;
    let mut session_created = false;

    while let Some(msg_result) = socket.recv().await {
        let msg = match msg_result {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!("WebSocket recv error: {}", e);
                break;
            },
        };

        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            Message::Ping(data) => {
                if socket.send(Message::Pong(data)).await.is_err() {
                    break;
                }
                continue;
            },
            _ => continue,
        };

        let client_msg: RealtimeClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let _ = send_msg(
                    &mut socket,
                    &RealtimeServerMessage::Error {
                        message: format!("Invalid message: {}", e),
                    },
                )
                .await;
                continue;
            },
        };

        match client_msg {
            RealtimeClientMessage::SessionCreate { model } => {
                _model = Some(model);
                session_created = true;
                if send_msg(
                    &mut socket,
                    &RealtimeServerMessage::SessionCreated {
                        session_id: session_id.clone(),
                    },
                )
                .await
                .is_err()
                {
                    break;
                }
            },

            RealtimeClientMessage::AudioAppend { audio } => {
                if !session_created {
                    let _ = send_msg(
                        &mut socket,
                        &RealtimeServerMessage::Error {
                            message: "Session not created. Send session.create first.".into(),
                        },
                    )
                    .await;
                    continue;
                }
                audio_buffer.push(audio);
            },

            RealtimeClientMessage::AudioCommit => {
                if !session_created {
                    let _ = send_msg(
                        &mut socket,
                        &RealtimeServerMessage::Error {
                            message: "Session not created. Send session.create first.".into(),
                        },
                    )
                    .await;
                    continue;
                }

                // Stub: echo back a text response instead of forwarding to a provider
                audio_buffer.clear();

                let send_ok = send_msg(
                    &mut socket,
                    &RealtimeServerMessage::TextDelta {
                        delta: "Realtime voice is not yet connected to a provider".into(),
                    },
                )
                .await
                .is_ok()
                    && send_msg(&mut socket, &RealtimeServerMessage::ResponseDone)
                        .await
                        .is_ok();

                if !send_ok {
                    break;
                }
            },

            RealtimeClientMessage::SessionClose => {
                let _ = socket.send(Message::Close(None)).await;
                break;
            },
        }
    }

    tracing::debug!("Realtime session {} closed", session_id);
}

async fn send_msg(socket: &mut WebSocket, msg: &RealtimeServerMessage) -> Result<(), axum::Error> {
    let json =
        serde_json::to_string(msg).map_err(|e| axum::Error::new(std::io::Error::other(e)))?;
    socket.send(Message::Text(json.into())).await
}
