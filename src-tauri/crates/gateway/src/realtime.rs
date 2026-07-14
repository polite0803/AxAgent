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
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite;

use axagent_harness::ProviderRequestContext;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::provider_type_to_str;
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
    #[allow(dead_code)]
    AudioAppend { audio: String },
    #[serde(rename = "input_audio_buffer.commit")]
    #[allow(dead_code)]
    AudioCommit,
    #[serde(rename = "session.close")]
    SessionClose,
}

// --- Server → Client messages ---

#[derive(Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum RealtimeServerMessage {
    #[serde(rename = "session.created")]
    SessionCreated { session_id: String },
    #[serde(rename = "response.text.delta")]
    TextDelta { delta: String },
    #[serde(rename = "response.audio.delta")]
    AudioDelta { delta: String },
    #[serde(rename = "response.audio.done")]
    AudioDone,
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
    let ws =
        ws.max_message_size(REALTIME_MAX_MESSAGE_BYTES).max_frame_size(REALTIME_MAX_MESSAGE_BYTES);

    ws.on_upgrade(move |socket| handle_realtime_session(socket, state))
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

/// LLM 提供商 WebSocket 消息通道容量
const LLM_CHANNEL_SIZE: usize = 64;

/// LLM 提供商 → 前端（String 是 JSON 文本）
type LlmToFrontend = mpsc::Receiver<String>;
/// 前端 → LLM 提供商（String 是 JSON 文本）
type FrontendToLlm = mpsc::Sender<String>;

async fn handle_realtime_session(socket: WebSocket, state: GatewayAppState) {
    // SECURITY (P1-9): max_message_size / max_frame_size 已在 realtime_handler
    // 通过 WebSocketUpgrade 配置，axum 0.8 的 WebSocket 不再支持 with_config。
    let mut socket = socket;

    let session_id = uuid::Uuid::new_v4().to_string();
    let model: String;
    let mut last_activity = std::time::Instant::now();
    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + REALTIME_HEARTBEAT_INTERVAL,
        REALTIME_HEARTBEAT_INTERVAL,
    );
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // ── Phase 1: 等待 session.create ──────────────────────────────

    loop {
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
                return;
            }
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(Bytes::new())).await.is_err() {
                    return;
                }
            }
            msg_result = socket.recv() => {
                let msg = match msg_result {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        tracing::debug!(error = %e, "WebSocket recv error");
                        return;
                    }
                    None => return, // 客户端断开
                };

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
                            RealtimeClientMessage::SessionCreate { model: m } => {
                                model = m;
                                break; // 退出 Phase 1，进入 Phase 2
                            },
                            _ => {
                                let _ = send_msg(
                                    &mut socket,
                                    &RealtimeServerMessage::Error {
                                        message: "Send session.create first".into(),
                                    },
                                ).await;
                            },
                        }
                    }
                    Message::Ping(data) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            return;
                        }
                    }
                    Message::Pong(_) | Message::Binary(_) => {}
                    Message::Close(_) => return,
                }
            }
        }
    }

    // ── Phase 2: 解析提供商，连接 LLM Realtime API ────────────────

    let (provider_config, provider_key, resolved_model) = match state
        .adapter
        .providers()
        .resolve_model_for_node(Some(&model), None, None, None)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(session_id = %session_id, model = %model, error = %e, "Failed to resolve model");
            let _ = send_msg(
                &mut socket,
                &RealtimeServerMessage::Error {
                    message: format!("Failed to resolve model '{}': {}", model, e),
                },
            )
            .await;
            return;
        },
    };

    let adapter =
        match state.provider_registry.get(provider_type_to_str(&provider_config.provider_type)) {
            Some(a) => a,
            None => {
                let _ = send_msg(
                    &mut socket,
                    &RealtimeServerMessage::Error {
                        message: format!(
                            "Provider type '{}' not found in registry",
                            provider_type_to_str(&provider_config.provider_type)
                        ),
                    },
                )
                .await;
                return;
            },
        };

    let decrypted_key = match state.adapter.crypto().decrypt_key(&provider_key.key_encrypted) {
        Ok(k) => k,
        Err(e) => {
            tracing::error!(error = %e, "Failed to decrypt provider key");
            let _ = send_msg(
                &mut socket,
                &RealtimeServerMessage::Error { message: "Failed to decrypt provider key".into() },
            )
            .await;
            return;
        },
    };

    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: provider_key.id.clone(),
        provider_id: provider_config.id.clone(),
        base_url: Some(provider_config.api_host.clone()),
        api_path: provider_config.api_path.clone(),
        proxy_config: provider_config.proxy_config.clone(),
        custom_headers: provider_config
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let realtime_config = match adapter.realtime_config(&ctx, &resolved_model).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "Provider does not support realtime voice");
            let _ = send_msg(
                &mut socket,
                &RealtimeServerMessage::Error {
                    message: format!("Realtime voice not supported: {}", e),
                },
            )
            .await;
            return;
        },
    };

    tracing::info!(
        session_id = %session_id,
        ws_url = %realtime_config.ws_url,
        provider = %provider_type_to_str(&provider_config.provider_type),
        "Connecting to LLM Realtime API"
    );

    let llm_ws = match tokio_tungstenite::connect_async(&realtime_config.ws_url).await {
        Ok((ws, _)) => ws,
        Err(e) => {
            tracing::error!(error = %e, "Failed to connect to LLM Realtime API");
            let _ = send_msg(
                &mut socket,
                &RealtimeServerMessage::Error {
                    message: format!("Failed to connect to LLM provider: {}", e),
                },
            )
            .await;
            return;
        },
    };

    let (mut llm_write, mut llm_read) = llm_ws.split();

    // ── Phase 3: 双向桥接 ────────────────────────────────────────

    // 通道：前端 → LLM
    let (frontend_to_llm_tx, mut frontend_to_llm_rx) = mpsc::channel::<String>(LLM_CHANNEL_SIZE);
    // 通道：LLM → 前端
    let (llm_to_frontend_tx, llm_to_frontend_rx) = mpsc::channel::<String>(LLM_CHANNEL_SIZE);

    // 将 session.create 转发给 LLM 提供商
    let session_create_json = json!({
        "type": "session.create",
        "model": resolved_model,
    })
    .to_string();
    if let Err(e) = llm_write.send(tungstenite::Message::Text(session_create_json.into())).await {
        tracing::error!(error = %e, "Failed to send session.create to LLM provider");
        return;
    }

    // 后台任务：LLM 提供商 → 前端通道
    let llm_read_task = tokio::spawn(async move {
        while let Some(msg) = llm_read.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    tracing::debug!(error = %e, "LLM WS read error");
                    break;
                },
            };
            match msg {
                tungstenite::Message::Text(t) => {
                    if llm_to_frontend_tx.send(t.to_string()).await.is_err() {
                        break; // 前端已关闭
                    }
                },
                tungstenite::Message::Close(_) => break,
                tungstenite::Message::Ping(data) => {
                    // tungstenite 自动回复 pong，这里不需要处理
                    let _ = data;
                },
                _ => {}, // 忽略 binary / pong
            }
        }
    });

    // 后台任务：前端通道 → LLM 提供商
    let llm_write_task = tokio::spawn(async move {
        while let Some(msg) = frontend_to_llm_rx.recv().await {
            if llm_write.send(tungstenite::Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
        // 优雅关闭 LLM 连接
        let _ = llm_write.close().await;
    });

    // 主循环：前端 ↔ 通道桥接
    let mut llm_to_frontend_rx: Option<LlmToFrontend> = Some(llm_to_frontend_rx);
    let frontend_to_llm_tx: Option<FrontendToLlm> = Some(frontend_to_llm_tx);

    loop {
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
                if socket.send(Message::Ping(Bytes::new())).await.is_err() {
                    break;
                }
            }

            // LLM → 前端
            llm_msg = async {
                match llm_to_frontend_rx.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                match llm_msg {
                    Some(text) => {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    None => {
                        // LLM 通道关闭，说明 LLM 连接已断开
                        tracing::info!(session_id = %session_id, "LLM provider disconnected");
                        // 告诉前端 LLM 响应结束
                        let _ = send_msg(&mut socket, &RealtimeServerMessage::ResponseDone).await;
                        break;
                    }
                }
            }

            // 前端消息
            msg_result = socket.recv() => {
                let msg = match msg_result {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        tracing::debug!(error = %e, "WebSocket recv error");
                        break;
                    }
                    None => break, // 前端断开
                };

                if !matches!(msg, Message::Ping(_) | Message::Pong(_)) {
                    last_activity = std::time::Instant::now();
                }

                match msg {
                    Message::Text(t) => {
                        let text = t;

                        // 快速检查是否为 session.close（不完整解析，只检查 type 字段）
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text)
                            && v.get("type").and_then(|t| t.as_str()) == Some("session.close")
                        {
                            let _ = socket.send(Message::Close(None)).await;
                            break;
                        }

                        // 转发到 LLM 提供商
                        if let Some(ref tx) = frontend_to_llm_tx
                            && tx.send(text.to_string()).await.is_err()
                        {
                            // LLM 通道已关闭
                            let _ = send_msg(
                                &mut socket,
                                &RealtimeServerMessage::Error {
                                    message: "LLM provider connection lost".into(),
                                },
                            ).await;
                            break;
                        }
                    }
                    Message::Binary(_b) => {
                        let _ = send_msg(
                            &mut socket,
                            &RealtimeServerMessage::Error {
                                message: "Binary messages are not supported on this endpoint".into(),
                            },
                        ).await;
                    }
                    Message::Ping(data) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Message::Pong(_) => {}
                    Message::Close(_) => break,
                }
            }
        }
    }

    // 清理：取消后台任务
    llm_read_task.abort();
    llm_write_task.abort();

    tracing::debug!(session_id = %session_id, "Realtime session closed");
}

async fn send_msg(socket: &mut WebSocket, msg: &RealtimeServerMessage) -> Result<(), axum::Error> {
    let json =
        serde_json::to_string(msg).map_err(|e| axum::Error::new(std::io::Error::other(e)))?;
    socket.send(Message::Text(json.into())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── RealtimeClientMessage 反序列化 ─────────────────────────

    #[test]
    fn deserialize_session_create() {
        let json = r#"{"type":"session.create","model":"gpt-4o-realtime-preview"}"#;
        let msg: RealtimeClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            RealtimeClientMessage::SessionCreate { model } => {
                assert_eq!(model, "gpt-4o-realtime-preview");
            },
            _ => panic!("expected SessionCreate"),
        }
    }

    #[test]
    fn deserialize_audio_append() {
        let json = r#"{"type":"input_audio_buffer.append","audio":"AAAA"}"#;
        let msg: RealtimeClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            RealtimeClientMessage::AudioAppend { audio } => {
                assert_eq!(audio, "AAAA");
            },
            _ => panic!("expected AudioAppend"),
        }
    }

    #[test]
    fn deserialize_audio_commit() {
        let json = r#"{"type":"input_audio_buffer.commit"}"#;
        let msg: RealtimeClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, RealtimeClientMessage::AudioCommit));
    }

    #[test]
    fn deserialize_session_close() {
        let json = r#"{"type":"session.close"}"#;
        let msg: RealtimeClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, RealtimeClientMessage::SessionClose));
    }

    #[test]
    fn deserialize_unknown_type_is_error() {
        let json = r#"{"type":"unknown_type","foo":"bar"}"#;
        let result: Result<RealtimeClientMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_invalid_json_is_error() {
        let result: Result<RealtimeClientMessage, _> = serde_json::from_str("not json");
        assert!(result.is_err());
    }

    // ── RealtimeServerMessage 序列化 ─────────────────────────

    #[test]
    fn serialize_session_created() {
        let msg = RealtimeServerMessage::SessionCreated { session_id: "sess-1".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "session.created");
        assert_eq!(v["session_id"], "sess-1");
    }

    #[test]
    fn serialize_text_delta() {
        let msg = RealtimeServerMessage::TextDelta { delta: "Hello".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "response.text.delta");
        assert_eq!(v["delta"], "Hello");
    }

    #[test]
    fn serialize_audio_delta() {
        let msg = RealtimeServerMessage::AudioDelta { delta: "AAAA".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "response.audio.delta");
        assert_eq!(v["delta"], "AAAA");
    }

    #[test]
    fn serialize_audio_done() {
        let msg = RealtimeServerMessage::AudioDone;
        let json = serde_json::to_string(&msg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "response.audio.done");
    }

    #[test]
    fn serialize_response_done() {
        let msg = RealtimeServerMessage::ResponseDone;
        let json = serde_json::to_string(&msg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "response.done");
    }

    #[test]
    fn serialize_error() {
        let msg = RealtimeServerMessage::Error { message: "Something went wrong".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "error");
        assert_eq!(v["message"], "Something went wrong");
    }

    // ── 常量 ────────────────────────────────────────────────

    #[test]
    fn constants_bounds() {
        assert!(REALTIME_IDLE_TIMEOUT.as_secs() >= 10);
        assert!(REALTIME_MAX_MESSAGE_BYTES >= 1024 * 1024);
        assert!(REALTIME_MAX_AUDIO_CHUNKS >= 8);
        assert_eq!(TICKET_TTL_SECS, 30);
        assert_eq!(REALTIME_HEARTBEAT_INTERVAL.as_secs(), 30);
        assert_eq!(LLM_CHANNEL_SIZE, 64);
    }
}
