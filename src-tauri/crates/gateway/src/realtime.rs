// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    Json,
    body::Bytes,
    extract::{
        Extension, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket, close_code},
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

use std::collections::VecDeque;
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

    // SECURITY (P1-9): 在 upgrade extractor 上设置 max_message_size / max_frame_size，
    // axum 0.8 不再支持 WebSocket::with_config()，必须在 WebSocketUpgrade 上配置。
    let ws = ws
        .max_message_size(REALTIME_MAX_MESSAGE_BYTES)
        .max_frame_size(REALTIME_MAX_MESSAGE_BYTES);

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

/// P1-9: Realtime 会话硬性上限 —— 60s 心跳超时 + 单条消息上限 + audio_buffer 容量上限。
/// 防止：
/// 1. 客户端发半截消息然后卡住，server 永久等待
/// 2. 攻击者发巨大单条消息耗尽内存
/// 3. 客户端不断 `input_audio_buffer.append` 不 commit，buffer 无限增长
pub const REALTIME_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// 单条 WebSocket 消息字节上限（base64 编码的 PCM 单帧 ~ 250KB 已够大）
pub const REALTIME_MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024; // 4MB
/// 音频 buffer 容量上限（约 60 秒 24kHz mono 16-bit ≈ 2.88MB，按 base64 体积再 / 0.75 约 4MB；
/// 这里按"段数"计算，假设每段 5s 音频）
pub const REALTIME_MAX_AUDIO_CHUNKS: usize = 64;
/// P1-9: 客户端心跳间隔 —— 30s 没收到任何消息就主动 ping 一次
pub const REALTIME_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

async fn handle_realtime_session(socket: WebSocket, _db: DatabaseConnection) {
    // SECURITY (P1-9): max_message_size / max_frame_size 已在 realtime_handler
    // 通过 WebSocketUpgrade 配置，axum 0.8 的 WebSocket 不再支持 with_config。
    let mut socket = socket;

    let session_id = uuid::Uuid::new_v4().to_string();
    // P1-9: 改用 VecDeque + 容量上限，避免恶意客户端无限 append
    let mut audio_buffer: VecDeque<String> = VecDeque::new();
    let mut _model: Option<String> = None;
    let mut session_created = false;
    let mut last_activity = std::time::Instant::now();
    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + REALTIME_HEARTBEAT_INTERVAL,
        REALTIME_HEARTBEAT_INTERVAL,
    );
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        // P1-9: select! 监听 socket + heartbeat + idle timeout 三路
        tokio::select! {
            biased;
            _ = tokio::time::sleep_until(
                tokio::time::Instant::from_std(last_activity + REALTIME_IDLE_TIMEOUT)
            ) => {
                tracing::warn!(
                    session_id = %session_id,
                    "Realtime session idle timeout ({}s) - closing",
                    REALTIME_IDLE_TIMEOUT.as_secs()
                );
                let _ = socket.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: close_code::POLICY,
                    reason: "idle timeout".into(),
                }))).await;
                break;
            }
            _ = heartbeat.tick() => {
                // 主动 ping 客户端（axum 自动回复 Pong）
                if socket.send(Message::Ping(Bytes::new())).await.is_err() {
                    break;
                }
            }
            msg_result = socket.recv() => {
                let msg = match msg_result {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        tracing::debug!(error = %e, "WebSocket recv error");
                        break;
                    }
                    None => break, // 客户端断开
                };

                // P1-9: 任何消息都刷新 idle 计时（Ping 单独处理以避免长连接误判）
                if !matches!(msg, Message::Ping(_) | Message::Pong(_)) {
                    last_activity = std::time::Instant::now();
                }

                match msg {
                    Message::Text(t) => {
                        let text = t;
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
                                // P1-9: 容量上限保护
                                if audio_buffer.len() >= REALTIME_MAX_AUDIO_CHUNKS {
                                    // 丢弃最老的 chunk（FIFO），并向客户端报错
                                    audio_buffer.pop_front();
                                    let _ = send_msg(
                                        &mut socket,
                                        &RealtimeServerMessage::Error {
                                            message: format!(
                                                "audio buffer overflow (max {REALTIME_MAX_AUDIO_CHUNKS} chunks); oldest dropped"
                                            ),
                                        },
                                    )
                                    .await;
                                }
                                audio_buffer.push_back(audio);
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
                    Message::Binary(_b) => {
                        let _ = send_msg(
                            &mut socket,
                            &RealtimeServerMessage::Error {
                                message: "Binary messages are not supported on this endpoint".into(),
                            },
                        )
                        .await;
                    }
                    Message::Ping(data) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Message::Pong(_) => {
                        // 客户端回复 Pong — 心跳正常
                    }
                    Message::Close(_close_frame) => break,
                }
            }
        }
    }

    tracing::debug!(session_id = %session_id, "Realtime session closed");
}

async fn send_msg(socket: &mut WebSocket, msg: &RealtimeServerMessage) -> Result<(), axum::Error> {
    let json =
        serde_json::to_string(msg).map_err(|e| axum::Error::new(std::io::Error::other(e)))?;
    socket.send(Message::Text(json.into())).await
}
