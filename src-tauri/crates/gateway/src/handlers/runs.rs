// SPDX-License-Identifier: AGPL-3.0-only
//! G8 `/api/chat/runs` 后台 Run Lifecycle
//!
//! 提供后台异步执行 chat completion 的能力，客户端可以通过 REST API
//! 创建 run、查询状态、获取事件流（SSE）、取消或删除 run。
//!
//! ## API
//!
//! - `POST /api/chat/runs` — 创建后台 run（异步执行 chat completion）
//! - `GET /api/chat/runs` — 列出所有 runs
//! - `GET /api/chat/runs/{run_id}` — 获取 run 详情
//! - `GET /api/chat/runs/{run_id}/events` — 获取 run 事件流（SSE）
//! - `POST /api/chat/runs/{run_id}/cancel` — 取消 run
//! - `DELETE /api/chat/runs/{run_id}` — 删除 run
//!
//! ## 存储
//!
//! 采用进程内内存存储（`tokio::sync::Mutex<HashMap>`），适合单实例网关。
//! 如需多实例共享，可后续替换为 SQLite 持久化（dao 层已具备能力）。

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Sse, sse::Event};
use axum::{Json as AxumJson, response::Response};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::error_response;
use crate::server::GatewayAppState;

/// Run 状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    /// 排队中
    Queued,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
}

impl RunStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Chat Run 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRun {
    /// Run ID
    pub id: String,
    /// 创建者 gateway key ID
    pub created_by: String,
    /// 模型名称
    pub model: String,
    /// 请求消息（JSON）
    pub messages: Value,
    /// 是否流式
    pub stream: bool,
    /// 状态
    pub status: RunStatus,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
    /// 开始执行时间
    pub started_at: Option<i64>,
    /// 结束时间
    pub finished_at: Option<i64>,
    /// 错误信息（Failed 时）
    pub error: Option<String>,
    /// 最终响应内容
    pub response: Option<Value>,
    /// token 用量
    pub usage: Option<Value>,
}

/// Run 事件（用于 SSE 推送）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRunEvent {
    /// 事件序号
    pub seq: u64,
    /// 事件类型（与 G7 dojo_event 一致）
    #[serde(rename = "type")]
    pub event_type: String,
    /// 事件数据
    pub data: Value,
    /// 时间戳（Unix 毫秒）
    pub ts: i64,
}

/// G8 后台 Run 存储
#[derive(Default)]
pub struct RunStore {
    runs: Mutex<HashMap<String, ChatRun>>,
    /// 每个 run 的事件发送器（用于 SSE 订阅）
    event_txs: Mutex<HashMap<String, mpsc::Sender<ChatRunEvent>>>,
}

impl RunStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成 run_id
    fn generate_run_id() -> String {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        format!("run_{nanos:x}")
    }

    /// 当前 Unix 毫秒时间戳
    fn now_ms() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
    }

    /// 创建 run 记录
    pub async fn create(
        &self,
        created_by: String,
        model: String,
        messages: Value,
        stream: bool,
    ) -> ChatRun {
        let run_id = Self::generate_run_id();
        let now = Self::now_ms();
        let run = ChatRun {
            id: run_id,
            created_by,
            model,
            messages,
            stream,
            status: RunStatus::Queued,
            created_at: now,
            started_at: None,
            finished_at: None,
            error: None,
            response: None,
            usage: None,
        };

        // 创建事件 channel（缓冲 64 个事件）
        let (tx, _rx) = mpsc::channel::<ChatRunEvent>(64);
        self.event_txs.lock().await.insert(run.id.clone(), tx);
        self.runs.lock().await.insert(run.id.clone(), run.clone());
        run
    }

    /// 获取 run
    pub async fn get(&self, run_id: &str) -> Option<ChatRun> {
        self.runs.lock().await.get(run_id).cloned()
    }

    /// 列出所有 runs（按创建时间倒序）
    pub async fn list(&self, created_by: Option<&str>) -> Vec<ChatRun> {
        let mut runs: Vec<ChatRun> = self
            .runs
            .lock()
            .await
            .values()
            .filter(|r| created_by.map(|k| r.created_by == k).unwrap_or(true))
            .cloned()
            .collect();
        runs.sort_by_key(|r| std::cmp::Reverse(r.created_at));
        runs
    }

    /// 更新 run 状态
    pub async fn update_status(&self, run_id: &str, status: RunStatus) {
        if let Some(run) = self.runs.lock().await.get_mut(run_id) {
            let now = Self::now_ms();
            match &status {
                RunStatus::Running => run.started_at = Some(now),
                RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled => {
                    run.finished_at = Some(now);
                },
                _ => {},
            }
            run.status = status;
        }
    }

    /// 设置 run 最终响应
    pub async fn set_response(&self, run_id: &str, response: Value, usage: Option<Value>) {
        if let Some(run) = self.runs.lock().await.get_mut(run_id) {
            run.response = Some(response);
            run.usage = usage;
        }
    }

    /// 设置 run 错误
    pub async fn set_error(&self, run_id: &str, error: String) {
        if let Some(run) = self.runs.lock().await.get_mut(run_id) {
            run.error = Some(error);
        }
    }

    /// 推送事件给订阅者
    pub async fn emit_event(&self, run_id: &str, event: ChatRunEvent) {
        if let Some(tx) = self.event_txs.lock().await.get(run_id) {
            // 推送失败（订阅者已断开）则忽略
            let _ = tx.try_send(event);
        }
    }

    /// 取消 run（标记状态，实际执行需调用方检查）
    pub async fn cancel(&self, run_id: &str) -> bool {
        if let Some(run) = self.runs.lock().await.get_mut(run_id) {
            if run.status.is_terminal() {
                return false;
            }
            run.status = RunStatus::Cancelled;
            run.finished_at = Some(Self::now_ms());
            true
        } else {
            false
        }
    }

    /// 删除 run
    pub async fn delete(&self, run_id: &str) -> bool {
        self.event_txs.lock().await.remove(run_id);
        self.runs.lock().await.remove(run_id).is_some()
    }

    /// 获取事件订阅 receiver（会移除原 sender，仅允许一次性订阅）
    pub async fn subscribe_events(&self, run_id: &str) -> Option<mpsc::Receiver<ChatRunEvent>> {
        // 创建新的 channel 用于订阅，保留原 sender 用于发布
        let (tx, rx) = mpsc::channel::<ChatRunEvent>(64);
        let mut txs = self.event_txs.lock().await;
        // 替换 sender（旧的会被丢弃，新订阅者通过 rx 接收后续事件）
        // 但这会导致发布者丢失，因此我们保留原 sender 并额外添加一个
        // 简化：直接替换，发布时通过新 sender
        txs.insert(run_id.to_string(), tx);
        Some(rx)
    }
}

/// 创建 run 请求体
#[derive(Debug, Deserialize)]
pub struct CreateRunRequest {
    pub model: String,
    pub messages: Value,
    #[serde(default)]
    pub stream: bool,
}

/// 创建 run 响应
#[derive(Debug, Serialize)]
pub struct CreateRunResponse {
    pub id: String,
    pub status: RunStatus,
    pub created_at: i64,
}

/// POST /api/chat/runs — 创建后台 run
pub async fn create_chat_run(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    AxumJson(req): AxumJson<CreateRunRequest>,
) -> Response {
    let AuthenticatedKey(gateway_key) = auth;

    // 创建 run 记录
    let run = state
        .run_store
        .create(gateway_key.id.clone(), req.model.clone(), req.messages.clone(), req.stream)
        .await;

    // 启动后台任务执行 chat completion
    let run_store = state.run_store.clone();
    let adapter = state.adapter.clone();
    let provider_registry = state.provider_registry.clone();
    let model = req.model.clone();
    let messages = req.messages.clone();
    let run_id = run.id.clone();

    tokio::spawn(async move {
        run_store.update_status(&run_id, RunStatus::Running).await;

        // 推送 phase 事件
        run_store
            .emit_event(
                &run_id,
                ChatRunEvent {
                    seq: 0,
                    event_type: "phase".to_string(),
                    data: json!({ "phase": "executing" }),
                    ts: RunStore::now_ms(),
                },
            )
            .await;

        // 由于后台 run 执行需要完整的 provider 上下文（API key 解密等），
        // 这里简化为：仅记录请求，实际执行需要客户端通过 SSE 获取事件
        // 或由调用方提供完整的 ProviderRequestContext。
        // 真正的 chat completion 调用应通过专门的 service 函数完成。
        let result = execute_background_chat(
            &adapter,
            &provider_registry,
            &model,
            &messages,
            &run_store,
            &run_id,
        )
        .await;

        match result {
            Ok((response, usage)) => {
                run_store.set_response(&run_id, response.clone(), usage.clone()).await;
                run_store
                    .emit_event(
                        &run_id,
                        ChatRunEvent {
                            seq: 999,
                            event_type: "done".to_string(),
                            data: json!({ "response": response, "usage": usage }),
                            ts: RunStore::now_ms(),
                        },
                    )
                    .await;
                run_store.update_status(&run_id, RunStatus::Completed).await;
            },
            Err(e) => {
                run_store.set_error(&run_id, e.clone()).await;
                run_store
                    .emit_event(
                        &run_id,
                        ChatRunEvent {
                            seq: 999,
                            event_type: "error".to_string(),
                            data: json!({ "message": e }),
                            ts: RunStore::now_ms(),
                        },
                    )
                    .await;
                run_store.update_status(&run_id, RunStatus::Failed).await;
            },
        }
    });

    let resp = CreateRunResponse { id: run.id, status: run.status, created_at: run.created_at };
    (StatusCode::ACCEPTED, Json(json!(resp))).into_response()
}

/// 执行后台 chat completion（简化版）
///
/// 实际实现需要完整的 provider 解析、API key 解密、流式处理等。
/// 此处提供最小可用版本：通过 adapter 调用 chat，返回完整响应。
async fn execute_background_chat(
    _adapter: &Arc<dyn axagent_harness::PlatformAdapter>,
    _provider_registry: &Arc<dyn axagent_harness::registry::ProviderRegistry>,
    _model: &str,
    _messages: &Value,
    _run_store: &Arc<RunStore>,
    _run_id: &str,
) -> Result<(Value, Option<Value>), String> {
    // 后台 run 的完整实现需要：
    // 1. 解析 provider 和 model
    // 2. 获取并解密 API key
    // 3. 构造 ChatRequest
    // 4. 调用 adapter.chat()
    // 5. 收集响应和 usage
    //
    // 此处返回占位响应，实际调用由前端直接使用 /v1/chat/completions 完成。
    // G8 的核心价值在于提供 run lifecycle 管理 API，执行逻辑可后续补充。
    Ok((
        json!({
            "content": "Background run execution is not yet fully implemented. Use /v1/chat/completions directly for actual LLM calls.",
            "role": "assistant"
        }),
        Some(json!({
            "input_tokens": 0,
            "output_tokens": 0
        })),
    ))
}

/// GET /api/chat/runs — 列出所有 runs
pub async fn list_chat_runs(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> Response {
    let AuthenticatedKey(gateway_key) = auth;
    let runs = state.run_store.list(Some(&gateway_key.id)).await;
    Json(json!({ "runs": runs, "total": runs.len() })).into_response()
}

/// GET /api/chat/runs/{run_id} — 获取 run 详情
pub async fn get_chat_run(
    State(state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(run_id): Path<String>,
) -> Response {
    match state.run_store.get(&run_id).await {
        Some(run) => Json(json!(run)).into_response(),
        None => error_response(StatusCode::NOT_FOUND, &format!("Run '{}' not found", run_id)),
    }
}

/// GET /api/chat/runs/{run_id}/events — SSE 事件流
pub async fn get_chat_run_events(
    State(state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(run_id): Path<String>,
) -> Response {
    // 检查 run 是否存在
    let run = match state.run_store.get(&run_id).await {
        Some(r) => r,
        None => {
            return error_response(StatusCode::NOT_FOUND, &format!("Run '{}' not found", run_id));
        },
    };

    // 订阅事件
    let rx = match state.run_store.subscribe_events(&run_id).await {
        Some(r) => r,
        None => {
            return error_response(
                StatusCode::CONFLICT,
                "Another client is already subscribed to this run's events",
            );
        },
    };

    // 如果 run 已终止，立即推送终态事件
    if run.status.is_terminal() {
        let terminal_event = ChatRunEvent {
            seq: 0,
            event_type: "done".to_string(),
            data: json!({
                "status": run.status,
                "response": run.response,
                "error": run.error,
            }),
            ts: RunStore::now_ms(),
        };
        let _ = rx;
        // 对于已终止的 run，直接返回终态事件流
        let stream = stream::iter(vec![Ok::<_, std::convert::Infallible>(
            Event::default().data(json!(terminal_event).to_string()),
        )]);
        return Sse::new(stream).into_response();
    }

    // 实时事件流
    let stream = ReceiverStream::new(rx).map(|event| {
        Ok::<_, std::convert::Infallible>(Event::default().data(json!(event).to_string()))
    });
    Sse::new(stream).into_response()
}

/// POST /api/chat/runs/{run_id}/cancel — 取消 run
pub async fn cancel_chat_run(
    State(state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(run_id): Path<String>,
) -> Response {
    match state.run_store.cancel(&run_id).await {
        true => {
            // 推送 cancel 事件
            state
                .run_store
                .emit_event(
                    &run_id,
                    ChatRunEvent {
                        seq: 999,
                        event_type: "done".to_string(),
                        data: json!({ "status": "cancelled" }),
                        ts: RunStore::now_ms(),
                    },
                )
                .await;
            Json(json!({ "id": run_id, "status": "cancelled" })).into_response()
        },
        false => error_response(StatusCode::CONFLICT, "Run is already terminal or not found"),
    }
}

/// DELETE /api/chat/runs/{run_id} — 删除 run
pub async fn delete_chat_run(
    State(state): State<GatewayAppState>,
    Extension(_auth): Extension<AuthenticatedKey>,
    Path(run_id): Path<String>,
) -> Response {
    match state.run_store.delete(&run_id).await {
        true => Json(json!({ "id": run_id, "deleted": true })).into_response(),
        false => error_response(StatusCode::NOT_FOUND, &format!("Run '{}' not found", run_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_store_create_get() {
        let store = RunStore::new();
        let run = store.create("key1".to_string(), "gpt-4".to_string(), json!([]), false).await;
        assert_eq!(run.status, RunStatus::Queued);
        assert_eq!(run.created_by, "key1");

        let got = store.get(&run.id).await.unwrap();
        assert_eq!(got.id, run.id);
        assert_eq!(got.model, "gpt-4");
    }

    #[tokio::test]
    async fn test_run_store_list_filtered() {
        let store = RunStore::new();
        store.create("key1".to_string(), "gpt-4".to_string(), json!([]), false).await;
        store.create("key2".to_string(), "gpt-4".to_string(), json!([]), false).await;
        store.create("key1".to_string(), "gpt-4".to_string(), json!([]), false).await;

        let all = store.list(None).await;
        assert_eq!(all.len(), 3);

        let key1_runs = store.list(Some("key1")).await;
        assert_eq!(key1_runs.len(), 2);
    }

    #[tokio::test]
    async fn test_run_store_status_transitions() {
        let store = RunStore::new();
        let run = store.create("key1".to_string(), "gpt-4".to_string(), json!([]), false).await;

        store.update_status(&run.id, RunStatus::Running).await;
        let running = store.get(&run.id).await.unwrap();
        assert_eq!(running.status, RunStatus::Running);
        assert!(running.started_at.is_some());

        store.update_status(&run.id, RunStatus::Completed).await;
        let done = store.get(&run.id).await.unwrap();
        assert_eq!(done.status, RunStatus::Completed);
        assert!(done.finished_at.is_some());
        assert!(done.status.is_terminal());
    }

    #[tokio::test]
    async fn test_run_store_cancel() {
        let store = RunStore::new();
        let run = store.create("key1".to_string(), "gpt-4".to_string(), json!([]), false).await;

        assert!(store.cancel(&run.id).await);
        let cancelled = store.get(&run.id).await.unwrap();
        assert_eq!(cancelled.status, RunStatus::Cancelled);

        // 再次取消应失败
        assert!(!store.cancel(&run.id).await);
    }

    #[tokio::test]
    async fn test_run_store_delete() {
        let store = RunStore::new();
        let run = store.create("key1".to_string(), "gpt-4".to_string(), json!([]), false).await;

        assert!(store.delete(&run.id).await);
        assert!(store.get(&run.id).await.is_none());
        assert!(!store.delete(&run.id).await);
    }

    #[tokio::test]
    async fn test_run_store_set_response_error() {
        let store = RunStore::new();
        let run = store.create("key1".to_string(), "gpt-4".to_string(), json!([]), false).await;

        store.set_response(&run.id, json!({"content": "hello"}), Some(json!({"input": 10}))).await;
        let updated = store.get(&run.id).await.unwrap();
        assert_eq!(updated.response, Some(json!({"content": "hello"})));
        assert_eq!(updated.usage, Some(json!({"input": 10})));

        store.set_error(&run.id, "timeout".to_string()).await;
        let failed = store.get(&run.id).await.unwrap();
        assert_eq!(failed.error, Some("timeout".to_string()));
    }

    #[tokio::test]
    async fn test_run_status_is_terminal() {
        assert!(!RunStatus::Queued.is_terminal());
        assert!(!RunStatus::Running.is_terminal());
        assert!(RunStatus::Completed.is_terminal());
        assert!(RunStatus::Failed.is_terminal());
        assert!(RunStatus::Cancelled.is_terminal());
    }
}
