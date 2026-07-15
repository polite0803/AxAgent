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
use axagent_harness::ProviderAdapter;
use axagent_harness::speech::{AudioChunkStream, SpeakRequest, SpeechInput};
use axagent_harness::types::{AudioFormat, AudioEncoding, ChatContent, ChatMessage, ChatRequest};
use axagent_harness::ProviderRequestContext;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::provider_type_to_str;
use crate::realtime_ticket::TicketStore;
use crate::server::GatewayAppState;

// --- Client → Server messages ---

#[derive(Deserialize)]
#[serde(tag = "type")]
enum RealtimeClientMessage {
    #[serde(rename = "session.create")]
    SessionCreate {
        model: String,
        voice: Option<String>,
        stt_provider: Option<String>,
        tts_provider: Option<String>,
    },
    /// 前端 VAD 检测到静音后发送：把当前缓冲的音频提交做 STT→LLM→TTS 一轮。
    #[serde(rename = "input_audio_buffer.commit")]
    AudioCommit,
    /// 用户主动打断（开始说话 / 点击打断）：中止当前生成。
    #[serde(rename = "response.cancel")]
    ResponseCancel,
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
    /// 用户侧语音识别结果（字幕）
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    InputTranscript { transcript: String },
    /// AI 文本增量（字幕）
    #[serde(rename = "response.text.delta")]
    TextDelta { delta: String },
    /// AI 音频增量（base64 PCM16）
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
pub const REALTIME_MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
/// 音频 buffer 累计字节上限（约 60 秒 24kHz mono 16-bit ≈ 2.88MB）
pub const REALTIME_MAX_AUDIO_BYTES: usize = 6 * 1024 * 1024;
/// P1-9: 客户端心跳间隔 —— 30s 没收到任何消息就主动 ping 一次
pub const REALTIME_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// LLM 提供商 → 前端（String 是 JSON 文本）
const LLM_CHANNEL_SIZE: usize = 64;

/// 一次语音回合所需的全部依赖（按会话选定的 provider 能力注入）。
///
/// 通过 harness `ProviderAdapter` 的 `transcribe` / `chat_stream` / `speech`
/// 三个能力方法串起本地语音闭环，不绑定任何具体厂商。
struct VoicePipelineDeps {
    stt: Arc<dyn ProviderAdapter>,
    llm: Arc<dyn ProviderAdapter>,
    tts: Arc<dyn ProviderAdapter>,
    stt_ctx: ProviderRequestContext,
    llm_ctx: ProviderRequestContext,
    tts_ctx: ProviderRequestContext,
    chat_model: String,
    voice: Option<String>,
    audio_format: AudioFormat,
    history: Arc<Mutex<Vec<ChatMessage>>>,
    generation: Arc<AtomicU64>,
    tts_tx: mpsc::Sender<String>,
    transcript_tx: mpsc::Sender<String>,
    done_tx: mpsc::Sender<()>,
}

/// 本地语音回合：STT → LLM 流式 → TTS 流式，结果经通道推回主循环。
///
/// `my_generation` 用于「打断」语义：若会话已开启新一轮（generation 自增），
/// 本回合视为过期，不再写入历史、不再推送完成信号，避免旧回合污染新上下文。
async fn run_voice_turn(
    deps: VoicePipelineDeps,
    audio: Vec<u8>,
    cancel: Arc<AtomicBool>,
    my_generation: u64,
) {
    let session_gen = || deps.generation.load(Ordering::SeqCst);

    // ── 1. STT：音频 → 文本 ──
    let transcript = match deps
        .stt
        .transcribe(
            &deps.stt_ctx,
            SpeechInput {
                data: audio,
                format: deps.audio_format.clone(),
            },
        )
        .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "STT failed");
            if session_gen() == my_generation {
                let _ = deps.done_tx.send(()).await;
            }
            return;
        },
    };

    if transcript.trim().is_empty() {
        if session_gen() == my_generation {
            let _ = deps.done_tx.send(()).await;
        }
        return;
    }

    // 写入历史（仅当本回合仍是最新）
    {
        let mut h = deps.history.lock().await;
        if session_gen() == my_generation {
            h.push(ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(transcript.clone()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            });
        }
    }

    // 把用户侧识别文本作为字幕回传
    if session_gen() == my_generation {
        let _ = deps.transcript_tx.send(transcript.clone()).await;
    }

    // ── 2. LLM 流式响应 ──
    let messages = {
        let h = deps.history.lock().await;
        h.clone()
    };
    let req = ChatRequest {
        model: deps.chat_model.clone(),
        messages,
        stream: true,
        temperature: Some(0.8),
        top_p: None,
        max_tokens: None,
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
    };

    let mut stream = deps
        .llm
        .chat_stream(&deps.llm_ctx, req, Some(cancel.clone()));
    let mut acc = String::new();
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        if let Ok(c) = chunk
            && let Some(text) = c.content
        {
            acc.push_str(&text);
        }
    }

    if cancel.load(Ordering::SeqCst) || session_gen() != my_generation {
        // 被打断或过期：不合成、不落库、不发完成信号
        return;
    }

    // 写入 assistant 历史
    {
        let mut h = deps.history.lock().await;
        if session_gen() == my_generation {
            h.push(ChatMessage {
                role: "assistant".to_string(),
                content: ChatContent::Text(acc.clone()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            });
        }
    }

    // ── 3. TTS：文本 → 音频流 ──
    if acc.trim().is_empty() {
        let _ = deps.done_tx.send(()).await;
        return;
    }

    let speak_req = SpeakRequest {
        text: acc,
        voice: deps.voice.clone(),
        format: AudioFormat {
            sample_rate: deps.audio_format.sample_rate,
            channels: deps.audio_format.channels,
            encoding: AudioEncoding::Pcm16,
        },
        model: None,
    };

    let mut audio_stream: AudioChunkStream = match deps.tts.speech(&deps.tts_ctx, speak_req).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "TTS failed");
            let _ = deps.done_tx.send(()).await;
            return;
        },
    };

    while let Some(chunk) = audio_stream.next().await {
        if cancel.load(Ordering::SeqCst) || session_gen() != my_generation {
            break;
        }
        match chunk {
            Ok(bytes) => {
                let b64 = BASE64_STANDARD.encode(&bytes);
                if deps.tts_tx.send(b64).await.is_err() {
                    break; // 前端已断开
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "TTS chunk error");
                break;
            },
        }
    }

    if session_gen() == my_generation {
        let _ = deps.done_tx.send(()).await;
    }
}

async fn handle_realtime_session(socket: WebSocket, state: GatewayAppState) {
    let mut socket = socket;

    let session_id = uuid::Uuid::new_v4().to_string();
    let mut last_activity = std::time::Instant::now();
    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + REALTIME_HEARTBEAT_INTERVAL,
        REALTIME_HEARTBEAT_INTERVAL,
    );
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // ── Phase 1: 等待 session.create ──────────────────────────────

    let (resolved_model, voice_opt, stt_provider_opt, tts_provider_opt) = loop {
        tokio::select! {
            biased;
            _ = tokio::time::sleep_until(
                tokio::time::Instant::from_std(last_activity + REALTIME_IDLE_TIMEOUT)
            ) => {
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
                    },
                    None => return, // 客户端断开
                };

                if !matches!(msg, Message::Ping(_) | Message::Pong(_)) {
                    last_activity = std::time::Instant::now();
                }

                match msg {
                    Message::Text(t) => {
                        let client_msg: RealtimeClientMessage = match serde_json::from_str(&t) {
                            Ok(m) => m,
                            Err(e) => {
                                let _ = send_msg(&mut socket, &RealtimeServerMessage::Error {
                                    message: format!("Invalid message: {}", e),
                                }).await;
                                continue;
                            },
                        };
                        match client_msg {
                            RealtimeClientMessage::SessionCreate { model, voice, stt_provider, tts_provider } => {
                                break (model, voice, stt_provider, tts_provider);
                            },
                            _ => {
                                let _ = send_msg(&mut socket, &RealtimeServerMessage::Error {
                                    message: "Send session.create first".into(),
                                }).await;
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
    };

    // ── Phase 2: 解析提供商并校验语音能力 ───────────────────────

    // 2a. LLM 主提供商
    let (llm_provider_config, llm_provider_key, _resolved_for_node) = match state
        .adapter
        .providers()
        .resolve_model_for_node(Some(&resolved_model), None, None, None)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(session_id = %session_id, model = %resolved_model, error = %e, "Failed to resolve model");
            let _ = send_msg(
                &mut socket,
                &RealtimeServerMessage::Error {
                    message: format!("Failed to resolve model '{}': {}", resolved_model, e),
                },
            )
            .await;
            return;
        },
    };

    let llm_adapter =
        match state.provider_registry.get(provider_type_to_str(&llm_provider_config.provider_type)) {
            Some(a) => a,
            None => {
                let _ = send_msg(
                    &mut socket,
                    &RealtimeServerMessage::Error {
                        message: format!(
                            "Provider type '{}' not found in registry",
                            provider_type_to_str(&llm_provider_config.provider_type)
                        ),
                    },
                )
                .await;
                return;
            },
        };

    // 2b. 辅助函数：按 provider_id 查找适配器和解密 key
    async fn resolve_speech_adapter<'a>(
        provider_id: &str,
        state: &'a GatewayAppState,
    ) -> Result<(Arc<dyn ProviderAdapter>, ProviderRequestContext), String> {
        let providers = state.adapter.providers();
        let configs = providers.list_providers().await.map_err(|e| e.to_string())?;
        let cfg = configs.iter().find(|p| p.id == provider_id)
            .ok_or_else(|| format!("STT/TTS provider '{}' not found", provider_id))?;
        let adapter = state.provider_registry
            .get(provider_type_to_str(&cfg.provider_type))
            .ok_or_else(|| format!("Provider type '{}' not in registry", provider_type_to_str(&cfg.provider_type)))?;
        let caps = adapter.supports_speech();
        if !caps.stt && !caps.tts {
            return Err(format!("Provider '{}' (type '{}') does not support speech", provider_id, provider_type_to_str(&cfg.provider_type)));
        }
        let key = providers.get_active_key(&cfg.id).await.map_err(|e| e.to_string())?;
        let decrypted = state.adapter.crypto().decrypt_key(&key.key_encrypted)
            .map_err(|e| format!("Failed to decrypt key: {}", e))?;
        let ctx = ProviderRequestContext {
            api_key: decrypted,
            key_id: key.id,
            provider_id: cfg.id.clone(),
            base_url: Some(cfg.api_host.clone()),
            api_path: cfg.api_path.clone(),
            proxy_config: cfg.proxy_config.clone(),
            custom_headers: cfg.custom_headers.as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        };
        Ok((adapter, ctx))
    }

    // 2c. 解析 LLM key + ctx（沿用主提供商）
    let llm_decrypted_key = match state.adapter.crypto().decrypt_key(&llm_provider_key.key_encrypted) {
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
    let llm_ctx = ProviderRequestContext {
        api_key: llm_decrypted_key,
        key_id: llm_provider_key.id.clone(),
        provider_id: llm_provider_config.id.clone(),
        base_url: Some(llm_provider_config.api_host.clone()),
        api_path: llm_provider_config.api_path.clone(),
        proxy_config: llm_provider_config.proxy_config.clone(),
        custom_headers: llm_provider_config
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    // 2d. 解析 STT / TTS 适配器（可选用不同提供商）
    let (stt_adapter, stt_ctx) = match stt_provider_opt.as_ref() {
        Some(pid) => match resolve_speech_adapter(pid, &state).await {
            Ok(r) => r,
            Err(e) => {
                let _ = send_msg(&mut socket, &RealtimeServerMessage::Error { message: e }).await;
                return;
            },
        },
        None => (llm_adapter.clone(), llm_ctx.clone()),
    };
    let (tts_adapter, tts_ctx) = match tts_provider_opt.as_ref() {
        Some(pid) => match resolve_speech_adapter(pid, &state).await {
            Ok(r) => r,
            Err(e) => {
                let _ = send_msg(&mut socket, &RealtimeServerMessage::Error { message: e }).await;
                return;
            },
        },
        None => (llm_adapter.clone(), llm_ctx.clone()),
    };

    // 2e. 校验语音能力
    let stt_caps = stt_adapter.supports_speech();
    let tts_caps = tts_adapter.supports_speech();
    if !stt_caps.stt || !tts_caps.tts {
        let _ = send_msg(
            &mut socket,
            &RealtimeServerMessage::Error {
                message:
                    "当前模型提供商不支持语音（STT/TTS 提供商需分别支持语音能力）"
                        .to_string(),
            },
        )
        .await;
        return;
    }

    tracing::info!(
        session_id = %session_id,
        llm_provider = %provider_type_to_str(&llm_provider_config.provider_type),
        model = %resolved_model,
        "Voice session (local STT→LLM→TTS pipeline) started"
    );

    // 通知前端会话已建立
    if send_msg(
        &mut socket,
        &RealtimeServerMessage::SessionCreated {
            session_id: session_id.clone(),
        },
    )
    .await
    .is_err()
    {
        return;
    }

    // ── Phase 3: 本地语音闭环编排 ────────────────────────────────

    let audio_format = AudioFormat {
        sample_rate: 24_000,
        channels: 1,
        encoding: AudioEncoding::Pcm16,
    };

    let (tts_tx, mut tts_rx) = mpsc::channel::<String>(LLM_CHANNEL_SIZE);
    let (transcript_tx, mut transcript_rx) = mpsc::channel::<String>(LLM_CHANNEL_SIZE);
    let (done_tx, mut done_rx) = mpsc::channel::<()>(8);

    let history: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
    let generation = Arc::new(AtomicU64::new(0));

    let mut audio_buffer: Vec<u8> = Vec::new();
    let mut is_responding = false;
    let mut pipeline_task: Option<JoinHandle<()>> = None;
    let mut current_cancel: Option<Arc<AtomicBool>> = None;
    let mut current_turn_history_len: Option<usize> = None;

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
                if let Some(t) = pipeline_task.take() { t.abort(); }
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

            // 后端 → 前端：AI 音频增量
            tts_msg = tts_rx.recv() => {
                match tts_msg {
                    Some(b64) => {
                        is_responding = true;
                        if socket.send(Message::Text(
                            serde_json::to_string(&RealtimeServerMessage::AudioDelta { delta: b64 })
                                .unwrap_or_default()
                                .into(),
                        )).await.is_err() {
                            break;
                        }
                    },
                    None => break, // 通道关闭
                }
            }

            // 后端 → 前端：用户侧识别文本（字幕）
            transcript_msg = transcript_rx.recv() => {
                match transcript_msg {
                    Some(t) => {
                        if socket.send(Message::Text(
                            serde_json::to_string(&RealtimeServerMessage::InputTranscript { transcript: t })
                                .unwrap_or_default()
                                .into(),
                        )).await.is_err() {
                            break;
                        }
                    },
                    None => break,
                }
            }

            // 后端 → 前端：本回合结束
            done_msg = done_rx.recv() => {
                match done_msg {
                    Some(()) => {
                        is_responding = false;
                        current_turn_history_len = None;
                        current_cancel = None;
                        pipeline_task = None;
                        if socket.send(Message::Text(
                            serde_json::to_string(&RealtimeServerMessage::AudioDone).unwrap_or_default().into(),
                        )).await.is_err() { break; }
                        if socket.send(Message::Text(
                            serde_json::to_string(&RealtimeServerMessage::ResponseDone).unwrap_or_default().into(),
                        )).await.is_err() { break; }
                    },
                    None => break,
                }
            }

            // 前端 → 后端
            msg_result = socket.recv() => {
                let msg = match msg_result {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        tracing::debug!(error = %e, "WebSocket recv error");
                        break;
                    },
                    None => break, // 前端断开
                };

                if !matches!(msg, Message::Ping(_) | Message::Pong(_)) {
                    last_activity = std::time::Instant::now();
                }

                match msg {
                    Message::Text(t) => {
                        let client_msg: RealtimeClientMessage = match serde_json::from_str(&t) {
                            Ok(m) => m,
                            Err(e) => {
                                let _ = send_msg(&mut socket, &RealtimeServerMessage::Error {
                                    message: format!("Invalid message: {}", e),
                                }).await;
                                continue;
                            },
                        };
                        match client_msg {
                            RealtimeClientMessage::SessionCreate { .. } => {
                                // 已建立，忽略重复
                            },
                            RealtimeClientMessage::AudioCommit => {
                                if audio_buffer.is_empty() {
                                    continue;
                                }
                                // 打断并接管：中止上一轮（若有），开启新一轮
                                abort_current_turn(
                                    &mut pipeline_task,
                                    &mut current_cancel,
                                    &mut current_turn_history_len,
                                    &history,
                                ).await;
                                let audio = std::mem::take(&mut audio_buffer);
                                let hist_len = history.lock().await.len();
                                spawn_turn(
                                    &stt_adapter,
                                    &llm_adapter,
                                    &tts_adapter,
                                    &stt_ctx,
                                    &llm_ctx,
                                    &tts_ctx,
                                    &resolved_model,
                                    &voice_opt,
                                    &audio_format,
                                    &history,
                                    &generation,
                                    &tts_tx,
                                    &transcript_tx,
                                    &done_tx,
                                    audio,
                                    hist_len,
                                    &mut current_turn_history_len,
                                    &mut current_cancel,
                                    &mut pipeline_task,
                                );
                                is_responding = true;
                            },
                            RealtimeClientMessage::ResponseCancel => {
                                abort_current_turn(
                                    &mut pipeline_task,
                                    &mut current_cancel,
                                    &mut current_turn_history_len,
                                    &history,
                                ).await;
                                audio_buffer.clear();
                                is_responding = false;
                            },
                            RealtimeClientMessage::SessionClose => {
                                let _ = socket.send(Message::Close(None)).await;
                                break;
                            },
                        }
                    }
                    Message::Binary(b) => {
                        // AI 正在说话时收到音频 = 用户打断（隐式 cancel）
                        if is_responding {
                            abort_current_turn(
                                &mut pipeline_task,
                                &mut current_cancel,
                                &mut current_turn_history_len,
                                &history,
                            ).await;
                            audio_buffer.clear();
                            is_responding = false;
                        }
                        if audio_buffer.len() + b.len() <= REALTIME_MAX_AUDIO_BYTES {
                            audio_buffer.extend_from_slice(&b);
                        } else {
                            tracing::warn!(session_id = %session_id, "audio buffer overflow, dropping");
                        }
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

    if let Some(t) = pipeline_task.take() {
        t.abort();
    }

    tracing::debug!(session_id = %session_id, "Realtime session closed");
}

/// 中止当前正在进行的语音回合（用户打断 / 新音频到达时调用）。
async fn abort_current_turn(
    pipeline_task: &mut Option<JoinHandle<()>>,
    current_cancel: &mut Option<Arc<AtomicBool>>,
    current_turn_history_len: &mut Option<usize>,
    history: &Arc<Mutex<Vec<ChatMessage>>>,
) {
    if let Some(c) = current_cancel.take() {
        c.store(true, Ordering::SeqCst);
    }
    if let Some(t) = pipeline_task.take() {
        t.abort();
    }
    // 回滚本回合已写入历史的部分（如已落库的用户消息），避免上下文污染
    if let Some(len) = current_turn_history_len.take() {
        let mut h = history.lock().await;
        h.truncate(len);
    }
}

/// 开启新一轮语音回合（STT→LLM→TTS）。
#[allow(clippy::too_many_arguments)]
fn spawn_turn(
    stt_adapter: &Arc<dyn ProviderAdapter>,
    llm_adapter: &Arc<dyn ProviderAdapter>,
    tts_adapter: &Arc<dyn ProviderAdapter>,
    stt_ctx: &ProviderRequestContext,
    llm_ctx: &ProviderRequestContext,
    tts_ctx: &ProviderRequestContext,
    resolved_model: &str,
    voice_opt: &Option<String>,
    audio_format: &AudioFormat,
    history: &Arc<Mutex<Vec<ChatMessage>>>,
    generation: &Arc<AtomicU64>,
    tts_tx: &mpsc::Sender<String>,
    transcript_tx: &mpsc::Sender<String>,
    done_tx: &mpsc::Sender<()>,
    audio: Vec<u8>,
    hist_len: usize,
    current_turn_history_len: &mut Option<usize>,
    current_cancel: &mut Option<Arc<AtomicBool>>,
    pipeline_task: &mut Option<JoinHandle<()>>,
) {
    let my_generation = generation.fetch_add(1, Ordering::SeqCst) + 1;
    *current_turn_history_len = Some(hist_len);
    let cancel = Arc::new(AtomicBool::new(false));
    *current_cancel = Some(cancel.clone());

    let deps = VoicePipelineDeps {
        stt: stt_adapter.clone(),
        llm: llm_adapter.clone(),
        tts: tts_adapter.clone(),
        stt_ctx: stt_ctx.clone(),
        llm_ctx: llm_ctx.clone(),
        tts_ctx: tts_ctx.clone(),
        chat_model: resolved_model.to_string(),
        voice: voice_opt.clone(),
        audio_format: audio_format.clone(),
        history: history.clone(),
        generation: generation.clone(),
        tts_tx: tts_tx.clone(),
        transcript_tx: transcript_tx.clone(),
        done_tx: done_tx.clone(),
    };

    *pipeline_task = Some(tokio::spawn(run_voice_turn(deps, audio, cancel, my_generation)));
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
        let json = r#"{"type":"session.create","model":"gpt-4o"}"#;
        let msg: RealtimeClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            RealtimeClientMessage::SessionCreate { model, voice, .. } => {
                assert_eq!(model, "gpt-4o");
                assert_eq!(voice, None);
            },
            _ => panic!("expected SessionCreate"),
        }
    }

    #[test]
    fn deserialize_session_create_with_voice() {
        let json = r#"{"type":"session.create","model":"gpt-4o","voice":"nova"}"#;
        let msg: RealtimeClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            RealtimeClientMessage::SessionCreate { model, voice, .. } => {
                assert_eq!(model, "gpt-4o");
                assert_eq!(voice.as_deref(), Some("nova"));
            },
            _ => panic!("expected SessionCreate"),
        }
    }

    #[test]
    fn deserialize_audio_commit() {
        let json = r#"{"type":"input_audio_buffer.commit"}"#;
        let msg: RealtimeClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, RealtimeClientMessage::AudioCommit));
    }

    #[test]
    fn deserialize_response_cancel() {
        let json = r#"{"type":"response.cancel"}"#;
        let msg: RealtimeClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, RealtimeClientMessage::ResponseCancel));
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
    fn serialize_input_transcript() {
        let msg = RealtimeServerMessage::InputTranscript { transcript: "你好".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "conversation.item.input_audio_transcription.completed");
        assert_eq!(v["transcript"], "你好");
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
        assert!(REALTIME_MAX_AUDIO_BYTES >= 1024 * 1024);
        assert_eq!(TICKET_TTL_SECS, 30);
        assert_eq!(REALTIME_HEARTBEAT_INTERVAL.as_secs(), 30);
        assert_eq!(LLM_CHANNEL_SIZE, 64);
    }
}
