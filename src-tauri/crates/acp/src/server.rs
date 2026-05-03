//! ACP HTTP/WebSocket 服务端
//! Feature flag: ACP_PROTOCOL

use std::sync::Arc;
use axum::{
    Router,
    extract::{Json, Path, State, ws::{WebSocket, WebSocketUpgrade, Message}},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};

use crate::protocol::{
    CreateSessionParams, CreateSessionResult, SendPromptParams, SendPromptResult,
    RegisterHookParams, StatusResult,
};
use crate::session::AcpSessionManager;
use crate::types::{AcpRequest, AcpResponse, AcpNotification};

/// ACP 服务端共享状态
pub struct AcpServerState {
    pub session_manager: AcpSessionManager,
    pub ws_clients: Arc<tokio::sync::RwLock<Vec<tokio::sync::mpsc::Sender<String>>>>,
    /// Prompt 处理器 — 由宿主注入，负责实际执行 agent 循环
    pub prompt_handler: Option<Arc<dyn PromptHandler>>,
}

/// Prompt 处理器 trait — 由实现了 AgentRuntime 的宿主实现
#[async_trait::async_trait]
pub trait PromptHandler: Send + Sync {
    /// 执行 prompt 并返回结果
    async fn handle_prompt(
        &self,
        session_id: &str,
        prompt: &str,
        work_dir: &str,
        max_turns: Option<u32>,
    ) -> SendPromptResult;
}

impl AcpServerState {
    pub fn new() -> Self {
        Self {
            session_manager: AcpSessionManager::new(),
            ws_clients: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            prompt_handler: None,
        }
    }

    /// 设置 prompt 处理器
    pub fn with_prompt_handler(mut self, handler: Arc<dyn PromptHandler>) -> Self {
        self.prompt_handler = Some(handler);
        self
    }

    /// 广播通知到所有 WebSocket 客户端
    pub async fn broadcast(&self, notification: &AcpNotification) {
        let json = serde_json::to_string(notification).unwrap_or_default();
        let clients = self.ws_clients.read().await;
        for sender in clients.iter() {
            let _ = sender.send(json.clone()).await;
        }
    }
}

/// 启动 ACP HTTP 服务端
pub async fn start_acp_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    if !AcpResponse::is_enabled() {
        tracing::info!("[ACP] ACP 协议未启用，跳过服务端启动");
        return Ok(());
    }

    let state = Arc::new(AcpServerState::new());

    let app = Router::new()
        // 会话管理
        .route("/acp/v1/sessions", post(create_session))
        .route("/acp/v1/sessions/{id}", get(get_session))
        .route("/acp/v1/sessions/{id}/close", post(close_session))
        .route("/acp/v1/sessions", get(list_sessions))
        // Prompt 交互
        .route("/acp/v1/sessions/{id}/prompts", post(send_prompt))
        .route("/acp/v1/sessions/{id}/interrupt", post(interrupt))
        // Hook 管理
        .route("/acp/v1/hooks", post(register_hook))
        // WebSocket
        .route("/acp/v1/ws", get(ws_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("[ACP] ACP 服务端启动在 {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 创建会话
async fn create_session(
    State(state): State<Arc<AcpServerState>>,
    Json(params): Json<CreateSessionParams>,
) -> impl IntoResponse {
    let result = state.session_manager.create_session(&params).await;
    (StatusCode::CREATED, Json(result))
}

/// 获取会话信息
async fn get_session(
    State(state): State<Arc<AcpServerState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.session_manager.get_status(&session_id).await {
        Some(status) => (StatusCode::OK, Json(status)).into_response(),
        None => (StatusCode::NOT_FOUND, Json(AcpResponse::error(
            None,
            crate::protocol::error_codes::SESSION_NOT_FOUND,
            "会话不存在",
        ))).into_response(),
    }
}

/// 关闭会话
async fn close_session(
    State(state): State<Arc<AcpServerState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if state.session_manager.close_session(&session_id).await {
        (StatusCode::OK, Json(serde_json::json!({"closed": true})))
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "会话不存在"})))
    }
}

/// 列出所有会话
async fn list_sessions(
    State(state): State<Arc<AcpServerState>>,
) -> impl IntoResponse {
    let sessions = state.session_manager.list_sessions().await;
    (StatusCode::OK, Json(sessions))
}

/// 发送 prompt
async fn send_prompt(
    State(state): State<Arc<AcpServerState>>,
    Path(session_id): Path<String>,
    Json(params): Json<SendPromptParams>,
) -> impl IntoResponse {
    match state.session_manager.get_session(&session_id).await {
        Some(_session) => {
            let work_dir = _session.work_dir.clone();
            state.session_manager.update_status(
                &session_id,
                crate::types::AcpSessionStatus::Running,
            ).await;

            // 通过注入的 PromptHandler 执行实际 agent 循环
            let result = if let Some(ref handler) = state.prompt_handler {
                handler.handle_prompt(
                    &session_id,
                    &params.prompt,
                    &work_dir,
                    params.max_turns,
                ).await
            } else {
                // 无 handler 时返回提示
                SendPromptResult {
                    session_id: session_id.clone(),
                    content: format!(
                        "[ACP] 未配置 PromptHandler，prompt 已接收但未执行: {}",
                        params.prompt
                    ),
                    tool_calls: Vec::new(),
                    turns: 0,
                    tokens_used: params.prompt.len() as u64,
                }
            };

            state.session_manager.update_status(
                &session_id,
                crate::types::AcpSessionStatus::Idle,
            ).await;

            (StatusCode::OK, Json(result))
        }
        None => {
            let error = AcpResponse::error(
                None,
                crate::protocol::error_codes::SESSION_NOT_FOUND,
                "会话不存在",
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
    }
}

/// 中断执行
async fn interrupt(
    State(state): State<Arc<AcpServerState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.session_manager.get_session(&session_id).await {
        Some(_) => {
            state.session_manager.update_status(
                &session_id,
                crate::types::AcpSessionStatus::Idle,
            ).await;
            (StatusCode::OK, Json(serde_json::json!({"interrupted": true})))
        }
        None => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "会话不存在"})))
        }
    }
}

/// 注册 hook 回调
async fn register_hook(
    State(state): State<Arc<AcpServerState>>,
    Json(params): Json<RegisterHookParams>,
) -> impl IntoResponse {
    tracing::info!(
        "[ACP] Hook 已注册: {} -> {} (session: {})",
        params.event,
        params.callback_url,
        params.session_id,
    );

    // 通过 WebSocket 广播通知
    let notification = AcpNotification {
        event: "hook.registered".to_string(),
        session_id: params.session_id.clone(),
        data: serde_json::json!({
            "hook_event": params.event,
            "callback_url": params.callback_url,
        }),
        timestamp: chrono::Utc::now(),
    };
    state.broadcast(&notification).await;

    (StatusCode::CREATED, Json(serde_json::json!({"registered": true})))
}

/// WebSocket 处理
async fn ws_handler(
    State(state): State<Arc<AcpServerState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

/// WebSocket 连接处理
async fn handle_ws(mut socket: WebSocket, state: Arc<AcpServerState>) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(64);

    // 注册客户端
    state.ws_clients.write().await.push(tx.clone());

    let clients = state.ws_clients.clone();
    let ws_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // 接收要发送的消息
                Some(msg) = rx.recv() => {
                    if socket.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                // 接收客户端消息
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            tracing::debug!("[ACP] WS 收到: {}", text);
                            // 解析 JSON-RPC 请求并处理
                            if let Ok(request) = serde_json::from_str::<AcpRequest>(&text) {
                                let response = AcpResponse::error(
                                    request.id,
                                    -32601,
                                    "方法未实现",
                                );
                                if let Ok(json) = serde_json::to_string(&response) {
                                    let _ = tx.send(json).await;
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => break,
                        None => break,
                        _ => {}
                    }
                }
            }
        }
        // 清理：移除当前客户端
        let mut clients_guard = clients.write().await;
        clients_guard.retain(|c| !c.same_channel(&tx));
    });

    let _ = ws_task.await;
}
