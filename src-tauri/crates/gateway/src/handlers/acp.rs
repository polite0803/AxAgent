// SPDX-License-Identifier: AGPL-3.0-only

//! ACP (Agent Communication Protocol) 处理器。
//!
//! 提供标准化的 Agent 会话管理接口 + 真实 LLM 调用能力。
//! 外部工具/IDE 可以通过 HTTP/WS 与 AxAgent 的智能体能力交互。
//!
//! ## 端点
//!
//! - `POST /acp/v1/sessions`              — 创建 Agent 会话
//! - `GET  /acp/v1/sessions`               — 列出所有会话
//! - `GET  /acp/v1/sessions/{id}`          — 会话详情
//! - `POST /acp/v1/sessions/{id}/prompts`  — 发送提示词（非流式，返回完整响应）
//! - `POST /acp/v1/sessions/{id}/interrupt`— 中断执行
//! - `POST /acp/v1/sessions/{id}/close`    — 关闭会话
//! - `WS   /acp/v1/ws`                     — WebSocket 流式通信

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use axum::{
    Json,
    extract::{Path, State, WebSocketUpgrade, ws},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest, ProviderConfig};
use axagent_harness::url_utils::resolve_base_url_for_type;
use axagent_harness::ProviderRequestContext;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::server::GatewayAppState;

// ── Session 模型 ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct AcpSessionMeta {
    id: String,
    work_dir: String,
    default_model: Option<String>,
    created_at: i64,
    closed: bool,
}

// ── 会话存储（模块级）──────────────────────────────────────────────────────

static SESSIONS: LazyLock<RwLock<HashMap<String, AcpSessionMeta>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// ── 请求/响应模型 ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    #[serde(default)]
    pub work_dir: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PromptRequest {
    pub prompt: String,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AcpResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl AcpResponse {
    fn ok(msg: impl Into<String>) -> Self {
        Self { success: true, message: msg.into(), session_id: None, data: None }
    }
    fn ok_with_session(msg: impl Into<String>, session_id: String) -> Self {
        Self { success: true, message: msg.into(), session_id: Some(session_id), data: None }
    }
    fn ok_with_data(msg: impl Into<String>, data: serde_json::Value) -> Self {
        Self { success: true, message: msg.into(), session_id: None, data: Some(data) }
    }
    fn err(msg: impl Into<String>) -> Self {
        Self { success: false, message: msg.into(), session_id: None, data: None }
    }
}

#[derive(Debug, Serialize)]
struct SessionListItem {
    id: String,
    work_dir: String,
    default_model: Option<String>,
    created_at: i64,
    closed: bool,
}

// ── Handler ───────────────────────────────────────────────────────────────

/// POST /acp/v1/sessions
pub async fn create_session(
    State(_state): State<GatewayAppState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let start = Instant::now();
    let session_id = uuid::Uuid::new_v4().to_string();
    let work_dir = req.work_dir.unwrap_or_default();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let meta = AcpSessionMeta { id: session_id.clone(), work_dir, default_model: req.model, created_at: now, closed: false };
    SESSIONS.write().await.insert(session_id.clone(), meta);
    info!("[ACP] Session created: {} ({:?})", session_id, start.elapsed());
    (StatusCode::CREATED, Json(AcpResponse::ok_with_session("Session created", session_id)))
}

/// GET /acp/v1/sessions
pub async fn list_sessions(State(_state): State<GatewayAppState>) -> impl IntoResponse {
    let s = SESSIONS.read().await;
    let items: Vec<SessionListItem> = s.iter().map(|(id, m)| SessionListItem {
        id: id.clone(), work_dir: m.work_dir.clone(), default_model: m.default_model.clone(),
        created_at: m.created_at, closed: m.closed,
    }).collect();
    Json(AcpResponse::ok_with_data(format!("{} session(s)", items.len()), serde_json::json!({"sessions": items})))
}

/// GET /acp/v1/sessions/{id}
pub async fn get_session(State(_state): State<GatewayAppState>, Path(sid): Path<String>) -> impl IntoResponse {
    let s = SESSIONS.read().await;
    match s.get(&sid) {
        Some(m) => (StatusCode::OK, Json(AcpResponse::ok_with_data("OK", serde_json::json!({
            "session": { "id": m.id, "work_dir": m.work_dir, "default_model": m.default_model, "created_at": m.created_at, "closed": m.closed }
        })))),
        None => (StatusCode::NOT_FOUND, Json(AcpResponse::err("Session not found"))),
    }
}

// ── LLM 辅助 ──────────────────────────────────────────────────────────────

async fn resolve_provider(state: &GatewayAppState, model: &str) -> Result<(ProviderConfig, String), String> {
    let providers = state.adapter.providers().list_providers().await.map_err(|e| format!("list providers: {e}"))?;
    for p in &providers {
        if !p.enabled { continue; }
        if p.models.iter().any(|m| m.enabled && m.model_id == model) {
            let t = format!("{:?}", p.provider_type).to_lowercase();
            return Ok((p.clone(), t));
        }
    }
    Err(format!("no enabled provider for model '{model}'"))
}

async fn build_ctx(state: &GatewayAppState, provider: &ProviderConfig) -> Result<ProviderRequestContext, String> {
    let key = state.adapter.providers().get_active_key(&provider.id).await.map_err(|e| format!("key: {e}"))?;
    let api_key = state.adapter.crypto().decrypt_key(&key.key_encrypted).map_err(|e| format!("decrypt: {e}"))?;
    let base = resolve_base_url_for_type(&provider.api_host, &provider.provider_type);
    Ok(ProviderRequestContext {
        api_key, key_id: key.id, provider_id: provider.id.clone(),
        base_url: Some(base), api_path: provider.api_path.clone(),
        proxy_config: None, custom_headers: None,
        api_mode: None, conversation: None, previous_response_id: None, store_response: None,
    })
}

// ── REST Prompt（非流式）────────────────────────────────────────────────────

/// POST /acp/v1/sessions/{id}/prompts
pub async fn send_prompt(
    State(state): State<GatewayAppState>,
    Path(session_id): Path<String>,
    Json(req): Json<PromptRequest>,
) -> impl IntoResponse {
    let start = Instant::now();
    let default_model = {
        let s = SESSIONS.read().await;
        match s.get(&session_id) {
            None => return (StatusCode::NOT_FOUND, Json(AcpResponse::err("Session not found"))),
            Some(m) if m.closed => return (StatusCode::GONE, Json(AcpResponse::err("Session is closed"))),
            Some(m) => m.default_model.clone(),
        }
    };
    let model = req.model.or(default_model).unwrap_or_else(|| "gpt-4o".to_string());

    let (provider, type_str) = match resolve_provider(&state, &model).await {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(AcpResponse::err(&e))),
    };
    let ctx = match build_ctx(&state, &provider).await {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(AcpResponse::err(&e))),
    };
    let adapter = match state.provider_registry.get(&type_str) {
        Some(a) => a,
        None => return (StatusCode::BAD_GATEWAY, Json(AcpResponse::err(format!("no adapter for '{type_str}'")))),
    };

    let req_msg = ChatRequest {
        model: model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: ChatContent::Text("You are a helpful assistant.".into()), tool_calls: None, tool_call_id: None, thinking: None },
            ChatMessage { role: "user".to_string(), content: ChatContent::Text(req.prompt.clone()), tool_calls: None, tool_call_id: None, thinking: None },
        ],
        temperature: None, top_p: None, stream: false, max_tokens: Some(4096),
        ..Default::default()
    };

    match adapter.chat(&ctx, Arc::new(req_msg)).await {
        Ok(resp) => {
            let (it, ot) = (resp.usage.input_tokens as u64, resp.usage.output_tokens as u64);
            info!("[ACP] LLM OK: session={}, chars={}, tok={}+{}, elapsed={:?}", session_id, resp.content.len(), it, ot, start.elapsed());
            (StatusCode::OK, Json(AcpResponse::ok_with_data("OK", serde_json::json!({
                "content": resp.content,
                "model": resp.model,
                "usage": { "input_tokens": it, "output_tokens": ot },
                "elapsed_ms": start.elapsed().as_millis() as u64,
            }))))
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(AcpResponse::err(format!("LLM call failed: {e}")))),
    }
}

/// POST /acp/v1/sessions/{id}/interrupt
pub async fn interrupt_session(State(_state): State<GatewayAppState>, Path(sid): Path<String>) -> impl IntoResponse {
    let s = SESSIONS.read().await;
    if !s.contains_key(&sid) { return (StatusCode::NOT_FOUND, Json(AcpResponse::err("Session not found"))); }
    if s.get(&sid).is_some_and(|m| m.closed) { return (StatusCode::GONE, Json(AcpResponse::err("Session is closed"))); }
    (StatusCode::OK, Json(AcpResponse::ok("Interrupt acknowledged")))
}

/// POST /acp/v1/sessions/{id}/close
pub async fn close_session(State(_state): State<GatewayAppState>, Path(sid): Path<String>) -> impl IntoResponse {
    let mut s = SESSIONS.write().await;
    match s.get_mut(&sid) {
        Some(m) if m.closed => (StatusCode::CONFLICT, Json(AcpResponse::err("Session already closed"))),
        Some(m) => { m.closed = true; (StatusCode::OK, Json(AcpResponse::ok("Session closed"))) },
        None => (StatusCode::NOT_FOUND, Json(AcpResponse::err("Session not found"))),
    }
}

// ── WebSocket 流式 ────────────────────────────────────────────────────────

/// WS /acp/v1/ws — WebSocket 流式 LLM 调用。
///
/// 客户端发送 JSON (含 prompt)，服务端流式返回 text_delta / done / error 事件。
pub async fn acp_websocket_handler(
    State(state): State<GatewayAppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_acp_ws_session(socket, state))
}

async fn handle_acp_ws_session(mut socket: ws::WebSocket, state: GatewayAppState) {
    info!("[ACP] WS connected");
    let start = Instant::now();

    let _ = socket.send(serde_json::json!({"type": "acp.connected", "server_info": {"name": "AxAgent ACP", "version": "0.2.0"}}).to_string().into()).await;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(ws::Message::Text(text))) => {
                        let v: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(e) => { let _ = socket.send(serde_json::json!({"type":"acp.error","message":format!("bad json: {e}")}).to_string().into()).await; continue; },
                        };
                        let prompt = match v.get("prompt").and_then(|p| p.as_str()) {
                            Some(p) => p,
                            None => { let _ = socket.send(serde_json::json!({"type":"acp.ack"}).to_string().into()).await; continue; },
                        };
                        let model = v.get("model").and_then(|m| m.as_str()).unwrap_or("gpt-4o");

                        // 流式调用 LLM 并逐 chunk 转发
                        match stream_acp_ws_inner(&state, model, prompt).await {
                            Ok(mut rx) => {
                                while let Some(event) = rx.recv().await {
                                    if socket.send(event.to_string().into()).await.is_err() { break; }
                                }
                                let _ = socket.send(serde_json::json!({"type":"acp.done"}).to_string().into()).await;
                            },
                            Err(e) => { let _ = socket.send(serde_json::json!({"type":"acp.error","message":e}).to_string().into()).await; },
                        }
                    },
                    Some(Ok(ws::Message::Close(_))) | None => break,
                    Some(Ok(ws::Message::Ping(d))) => { let _ = socket.send(ws::Message::Pong(d)).await; },
                    Some(Err(e)) => { warn!("[ACP] WS error: {e}"); break; },
                    _ => {},
                }
            },
            _ = tokio::time::sleep(std::time::Duration::from_secs(120)) => {
                let _ = socket.send(serde_json::json!({"type":"acp.idle_timeout"}).to_string().into()).await;
                break;
            },
        }
    }
    info!("[ACP] WS disconnected after {:?}", start.elapsed());
}

async fn stream_acp_ws_inner(
    state: &GatewayAppState,
    model: &str,
    prompt: &str,
) -> Result<tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>, String> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let (provider, type_str) = resolve_provider(state, model).await?;
    let ctx = build_ctx(state, &provider).await?;
    let adapter = state.provider_registry.get(&type_str).ok_or_else(|| format!("no adapter '{type_str}'"))?;

    use std::sync::atomic::AtomicBool;
    let req = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: ChatContent::Text("You are a helpful assistant.".into()), tool_calls: None, tool_call_id: None, thinking: None },
            ChatMessage { role: "user".to_string(), content: ChatContent::Text(prompt.to_string()), tool_calls: None, tool_call_id: None, thinking: None },
        ],
        temperature: None, top_p: None, stream: true, max_tokens: Some(4096),
        ..Default::default()
    };

    tokio::spawn(async move {
        let mut stream = adapter.chat_stream(&ctx, req, Some(Arc::new(AtomicBool::new(false))));
        use futures::StreamExt;
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Some(ref text) = chunk.content {
                        let _ = tx.send(serde_json::json!({"type":"acp.text_delta","delta":text}));
                    }
                    if let Some(ref thinking) = chunk.thinking {
                        let _ = tx.send(serde_json::json!({"type":"acp.thinking_delta","delta":thinking}));
                    }
                    if let Some(ref usage) = chunk.usage {
                        let _ = tx.send(serde_json::json!({"type":"acp.usage","input_tokens":usage.input_tokens,"output_tokens":usage.output_tokens}));
                    }
                    if chunk.done { break; }
                },
                Err(e) => {
                    let _ = tx.send(serde_json::json!({"type":"acp.error","message":format!("stream error: {e}")}));
                    break;
                },
            }
        }
    });

    Ok(rx)
}
