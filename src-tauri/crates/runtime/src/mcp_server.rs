// SPDX-License-Identifier: AGPL-3.0-only

//! Minimal Model Context Protocol (MCP) server.
//!
//! Implements a newline-safe, LSP-framed JSON-RPC server over stdio that
//! answers `initialize`, `tools/list`, and `tools/call` requests. The framing
//! matches the client transport implemented in [`crate::mcp_stdio`] so this
//! server can be driven by either an external MCP client (e.g. Claude
//! Desktop) or `claw`'s own [`McpServerManager`](crate::McpServerManager).
//!
//! 除工具三大方法外，本模块还实现了 MCP 2025-03-26 规范的以下能力：
//! - `resources/list`、`resources/read`、`resources/subscribe`、`resources/unsubscribe`
//! - `prompts/list`、`prompts/get`
//! - `logging/setLevel`
//! - `completion/complete`
//! - `sampling/createMessage`（需注入 [`LlmSampler`] 桥接器）
//!
//! The server is intentionally small: it exposes a list of pre-built
//! [`McpTool`] descriptors and delegates `tools/call` to a caller-supplied
//! handler. Tool execution itself lives in the `tools` crate; this module is
//! purely the transport + dispatch loop.
//!
//! [`McpTool`]: crate::mcp_stdio::McpTool

use std::collections::HashSet;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use tokio::io::{
    AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Stdin, Stdout, stdin, stdout,
};
use tokio::sync::Mutex;

use axagent_harness::{McpPrompt, McpPromptResult};

use crate::mcp_stdio::{
    JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, McpInitializeResult,
    McpInitializeServerInfo, McpListResourcesResult, McpListToolsResult, McpReadResourceParams,
    McpReadResourceResult, McpResource, McpResourceContents, McpTool, McpToolCallContent,
    McpToolCallParams, McpToolCallResult,
};

/// Protocol version the server advertises during `initialize`.
///
/// Matches the version used by the built-in client in
/// [`crate::mcp_stdio`], so the two stay in lockstep.
pub const MCP_SERVER_PROTOCOL_VERSION: &str = "2025-03-26";

/// Synchronous handler invoked for every `tools/call` request.
///
/// Returning `Ok(text)` yields a single `text` content block and
/// `isError: false`. Returning `Err(message)` yields a `text` block with the
/// error and `isError: true`, mirroring the error-surfacing convention used
/// elsewhere in claw.
pub type ToolCallHandler =
    Box<dyn Fn(&str, &JsonValue) -> Result<String, String> + Send + Sync + 'static>;

// ──────────────────────────────────────────────────────────────────────
// 日志级别（AtomicU8 存储，默认 info）
// ──────────────────────────────────────────────────────────────────────

/// MCP `logging/setLevel` 支持的日志级别。
///
/// 用 `u8` 原子存储，避免在同步 dispatch 中引入异步锁。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub(crate) enum LogLevel {
    Debug = 0,
    #[default]
    Info = 1,
    Notice = 2,
    Warning = 3,
    Error = 4,
    Critical = 5,
    Alert = 6,
    Emergency = 7,
}

impl LogLevel {
    /// 从 MCP 规范字符串解析日志级别。
    pub(crate) fn from_level_str(s: &str) -> Option<Self> {
        match s {
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "notice" => Some(Self::Notice),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            "critical" => Some(Self::Critical),
            "alert" => Some(Self::Alert),
            "emergency" => Some(Self::Emergency),
            _ => None,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────
// 新增 MCP 协议类型（harness 未定义的部分）
// ──────────────────────────────────────────────────────────────────────

/// `prompts/list` 返回结果。
///
/// 不 derive `PartialEq`：`McpPrompt` 来自 harness 且未实现该 trait。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpListPromptsResult {
    pub prompts: Vec<McpPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// `prompts/get` 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct McpGetPromptParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonValue>,
}

/// `resources/subscribe` / `resources/unsubscribe` 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct McpSubscribeParams {
    pub uri: String,
}

/// `logging/setLevel` 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct McpLoggingSetLevelParams {
    pub level: String,
}

/// `completion/complete` 请求参数中的 `argument` 字段。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct McpCompletionArgument {
    pub name: String,
    pub value: String,
}

/// `completion/complete` 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct McpCompletionCompleteParams {
    /// MCP 规范中字段名为 `ref`，是 Rust 关键字，故重命名。
    #[serde(rename = "ref")]
    pub reference: JsonValue,
    pub argument: McpCompletionArgument,
}

/// `completion/complete` 返回的补全值集合。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct McpCompletionValues {
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "hasMore")]
    pub has_more: Option<bool>,
}

/// `completion/complete` 返回结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct McpCompletionResult {
    pub completion: McpCompletionValues,
}

/// `sampling/createMessage` 请求中的消息条目。
///
/// 标记为 `pub`：作为 [`McpSamplingCreateMessageParams`] 公有字段的元素类型，
/// 且 [`LlmSampler::sample`] 签名间接暴露此类型，必须与 trait 同级可见。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpSamplingMessage {
    pub role: String,
    pub content: JsonValue,
}

/// `sampling/createMessage` 请求参数。
///
/// 标记为 `pub`：作为 [`LlmSampler::sample`] 的入参类型被外部实现者引用。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpSamplingCreateMessageParams {
    pub messages: Vec<McpSamplingMessage>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "modelPreferences")]
    pub model_preferences: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemPrompt")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "includeContext")]
    pub include_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(rename = "maxTokens")]
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none", rename = "stopSequences")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonValue>,
}

/// `sampling/createMessage` 返回结果。
///
/// 标记为 `pub`：作为 [`LlmSampler::sample`] 的返回类型被外部实现者引用。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpSamplingCreateMessageResult {
    pub role: String,
    pub content: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "stopReason")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "stopSequence")]
    pub stop_sequence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<JsonValue>,
}

/// LLM 采样桥接器 trait。
///
/// 当 MCP 客户端通过 `sampling/createMessage` 请求 LLM 推理时，
/// 服务端通过此 trait 将请求转发给宿主 LLM（如 AxAgent 的 provider 层）。
///
/// 该 trait 故意定义在 `runtime` crate 内，不污染 `harness` 基座层：
/// sampling 是服务端能力，而非跨 crate 共享的 DTO 契约。
/// 未注入实现时，`sampling/createMessage` 返回 `MethodNotFound`。
pub trait LlmSampler: Send + Sync + 'static {
    /// 执行采样请求，返回生成的消息。
    fn sample(
        &self,
        params: McpSamplingCreateMessageParams,
    ) -> Result<McpSamplingCreateMessageResult, String>;
}

// ──────────────────────────────────────────────────────────────────────
// McpServerSpec / McpServer
// ──────────────────────────────────────────────────────────────────────

/// Configuration for an [`McpServer`] instance.
///
/// Named `McpServerSpec` rather than `McpServerConfig` to avoid colliding
/// with the existing client-side [`axagent_runtime_core::config::McpServerConfig`] that
/// describes *remote* MCP servers the runtime connects to.
pub struct McpServerSpec {
    /// Name advertised in the `serverInfo` field of the `initialize` response.
    pub server_name: String,
    /// Version advertised in the `serverInfo` field of the `initialize`
    /// response.
    pub server_version: String,
    /// Tool descriptors returned for `tools/list`.
    pub tools: Vec<McpTool>,
    /// Handler invoked for `tools/call`.
    pub tool_handler: ToolCallHandler,
    /// `resources/list` 返回的资源描述符列表（默认空）。
    pub resources: Vec<McpResource>,
    /// `prompts/list` 返回的提示词模板列表（默认空）。
    pub prompts: Vec<McpPrompt>,
    /// LLM 采样桥接器。注入后 `sampling/createMessage` 才可用，
    /// 未注入时该方法返回 `MethodNotFound`。
    pub llm_sampler: Option<Arc<dyn LlmSampler>>,
}

/// Minimal MCP stdio server.
///
/// The server runs a blocking read/dispatch/write loop over the current
/// process's stdin/stdout, terminating cleanly when the peer closes the
/// stream.
pub struct McpServer {
    spec: McpServerSpec,
    stdin: BufReader<Stdin>,
    stdout: Stdout,
    /// `resources/subscribe` 记录的 URI 集合。
    /// 用 `tokio::sync::Mutex` 存储，遵循 AGENTS.md 异步锁约束。
    subscribed_uris: Arc<Mutex<HashSet<String>>>,
    /// `logging/setLevel` 设置的日志级别（AtomicU8，默认 `info`）。
    log_level: Arc<AtomicU8>,
}

impl McpServer {
    #[must_use]
    pub fn new(spec: McpServerSpec) -> Self {
        Self {
            spec,
            stdin: BufReader::new(stdin()),
            stdout: stdout(),
            subscribed_uris: Arc::new(Mutex::new(HashSet::new())),
            log_level: Arc::new(AtomicU8::new(LogLevel::default() as u8)),
        }
    }

    /// Runs the server until the client closes stdin.
    ///
    /// Returns `Ok(())` on clean EOF; any other I/O error is propagated so
    /// callers can log and exit non-zero.
    pub async fn run(&mut self) -> io::Result<()> {
        loop {
            let Some(payload) = read_frame(&mut self.stdin).await? else {
                return Ok(());
            };

            // Requests and notifications share a wire format; the absence of
            // `id` distinguishes notifications, which must never receive a
            // response.
            let message: JsonValue = match serde_json::from_slice(&payload) {
                Ok(value) => value,
                Err(error) => {
                    // Parse error with null id per JSON-RPC 2.0 §4.2.
                    let response = JsonRpcResponse::<JsonValue> {
                        jsonrpc: "2.0".to_string(),
                        id: JsonRpcId::Null,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("parse error: {error}"),
                            data: None,
                        }),
                    };
                    write_response(&mut self.stdout, &response).await?;
                    continue;
                },
            };

            if message.get("id").is_none() {
                // Notification: dispatch for side effects only (e.g. log),
                // but send no reply.
                continue;
            }

            let request: JsonRpcRequest<JsonValue> = match serde_json::from_value(message) {
                Ok(request) => request,
                Err(error) => {
                    let response = JsonRpcResponse::<JsonValue> {
                        jsonrpc: "2.0".to_string(),
                        id: JsonRpcId::Null,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32600,
                            message: format!("invalid request: {error}"),
                            data: None,
                        }),
                    };
                    write_response(&mut self.stdout, &response).await?;
                    continue;
                },
            };

            let response = self.dispatch(request).await;
            write_response(&mut self.stdout, &response).await?;
        }
    }

    async fn dispatch(&self, request: JsonRpcRequest<JsonValue>) -> JsonRpcResponse<JsonValue> {
        let id = request.id.clone();
        match request.method.as_str() {
            "initialize" => self.handle_initialize(id),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, request.params),
            "resources/list" => self.handle_resources_list(id),
            "resources/read" => self.handle_resources_read(id, request.params),
            "resources/subscribe" => self.handle_resources_subscribe(id, request.params).await,
            "resources/unsubscribe" => self.handle_resources_unsubscribe(id, request.params).await,
            "prompts/list" => self.handle_prompts_list(id),
            "prompts/get" => self.handle_prompts_get(id, request.params),
            "logging/setLevel" => self.handle_logging_set_level(id, request.params),
            "completion/complete" => self.handle_completion_complete(id, request.params),
            "sampling/createMessage" => self.handle_sampling_create_message(id, request.params),
            other => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("method not found: {other}"),
                    data: None,
                }),
            },
        }
    }

    fn handle_initialize(&self, id: JsonRpcId) -> JsonRpcResponse<JsonValue> {
        let result = McpInitializeResult {
            protocol_version: MCP_SERVER_PROTOCOL_VERSION.to_string(),
            capabilities: server_capabilities(),
            server_info: McpInitializeServerInfo {
                name: self.spec.server_name.clone(),
                version: self.spec.server_version.clone(),
            },
        };
        ok_response(id, serde_json::to_value(result).ok())
    }

    fn handle_tools_list(&self, id: JsonRpcId) -> JsonRpcResponse<JsonValue> {
        let result = McpListToolsResult { tools: self.spec.tools.clone(), next_cursor: None };
        ok_response(id, serde_json::to_value(result).ok())
    }

    fn handle_tools_call(
        &self,
        id: JsonRpcId,
        params: Option<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let Some(params) = params else {
            return invalid_params_response(id, "missing params for tools/call");
        };
        let call: McpToolCallParams = match serde_json::from_value(params) {
            Ok(value) => value,
            Err(error) => {
                return invalid_params_response(id, &format!("invalid tools/call params: {error}"));
            },
        };
        let arguments = call.arguments.unwrap_or_else(|| json!({}));
        let tool_result = (self.spec.tool_handler)(&call.name, &arguments);
        let (text, is_error) = match tool_result {
            Ok(text) => (text, false),
            Err(message) => (message, true),
        };
        let mut data = std::collections::BTreeMap::new();
        data.insert("text".to_string(), JsonValue::String(text));
        let call_result = McpToolCallResult {
            content: vec![McpToolCallContent { kind: "text".to_string(), data }],
            structured_content: None,
            is_error: Some(is_error),
            meta: None,
        };
        ok_response(id, serde_json::to_value(call_result).ok())
    }

    /// `resources/list` — 返回 spec 中注册的资源描述符列表。
    fn handle_resources_list(&self, id: JsonRpcId) -> JsonRpcResponse<JsonValue> {
        let result =
            McpListResourcesResult { resources: self.spec.resources.clone(), next_cursor: None };
        ok_response(id, serde_json::to_value(result).ok())
    }

    /// `resources/read` — 读取指定 URI 的资源。
    ///
    /// 当前资源描述符不携带实际内容，已知 URI 返回空 text 占位，
    /// 未知 URI 返回 `invalid params` 错误。
    fn handle_resources_read(
        &self,
        id: JsonRpcId,
        params: Option<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let Some(params) = params else {
            return invalid_params_response(id, "missing params for resources/read");
        };
        let read: McpReadResourceParams = match serde_json::from_value(params) {
            Ok(value) => value,
            Err(error) => {
                return invalid_params_response(
                    id,
                    &format!("invalid resources/read params: {error}"),
                );
            },
        };
        let Some(resource) = self.spec.resources.iter().find(|r| r.uri == read.uri) else {
            return invalid_params_response(id, &format!("unknown resource uri: {}", read.uri));
        };
        // 资源描述符不含实际内容，返回空 text 占位，保持 MCP 响应结构完整。
        let contents = vec![McpResourceContents {
            uri: resource.uri.clone(),
            mime_type: resource.mime_type.clone(),
            text: Some(String::new()),
            blob: None,
            meta: None,
        }];
        let result = McpReadResourceResult { contents };
        ok_response(id, serde_json::to_value(result).ok())
    }

    /// `resources/subscribe` — 订阅指定 URI 的变更通知。
    ///
    /// 当前仅记录到内存 `HashSet`，不实际推送通知。
    async fn handle_resources_subscribe(
        &self,
        id: JsonRpcId,
        params: Option<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let Some(params) = params else {
            return invalid_params_response(id, "missing params for resources/subscribe");
        };
        let sub: McpSubscribeParams = match serde_json::from_value(params) {
            Ok(value) => value,
            Err(error) => {
                return invalid_params_response(
                    id,
                    &format!("invalid resources/subscribe params: {error}"),
                );
            },
        };
        self.subscribed_uris.lock().await.insert(sub.uri);
        ok_response(id, Some(JsonValue::Object(serde_json::Map::new())))
    }

    /// `resources/unsubscribe` — 取消订阅指定 URI。
    async fn handle_resources_unsubscribe(
        &self,
        id: JsonRpcId,
        params: Option<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let Some(params) = params else {
            return invalid_params_response(id, "missing params for resources/unsubscribe");
        };
        let sub: McpSubscribeParams = match serde_json::from_value(params) {
            Ok(value) => value,
            Err(error) => {
                return invalid_params_response(
                    id,
                    &format!("invalid resources/unsubscribe params: {error}"),
                );
            },
        };
        self.subscribed_uris.lock().await.remove(&sub.uri);
        ok_response(id, Some(JsonValue::Object(serde_json::Map::new())))
    }

    /// `prompts/list` — 返回 spec 中注册的提示词模板列表。
    fn handle_prompts_list(&self, id: JsonRpcId) -> JsonRpcResponse<JsonValue> {
        let result = McpListPromptsResult { prompts: self.spec.prompts.clone(), next_cursor: None };
        ok_response(id, serde_json::to_value(result).ok())
    }

    /// `prompts/get` — 获取指定名称的提示词。
    ///
    /// 当前模板不携带实际消息体，命中时返回空消息列表的 `PromptResult`，
    /// 未命中返回 `invalid params` 错误。
    fn handle_prompts_get(
        &self,
        id: JsonRpcId,
        params: Option<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let Some(params) = params else {
            return invalid_params_response(id, "missing params for prompts/get");
        };
        let get: McpGetPromptParams = match serde_json::from_value(params) {
            Ok(value) => value,
            Err(error) => {
                return invalid_params_response(
                    id,
                    &format!("invalid prompts/get params: {error}"),
                );
            },
        };
        let Some(prompt) = self.spec.prompts.iter().find(|p| p.name == get.name) else {
            return invalid_params_response(id, &format!("unknown prompt: {}", get.name));
        };
        // 模板不含实际消息体，返回空消息数组占位，保持 MCP 响应结构完整。
        let result =
            McpPromptResult { description: prompt.description.clone(), messages: json!([]) };
        ok_response(id, serde_json::to_value(result).ok())
    }

    /// `logging/setLevel` — 设置服务端日志级别。
    ///
    /// 级别存入 `AtomicU8`，供服务端内部日志过滤使用。
    fn handle_logging_set_level(
        &self,
        id: JsonRpcId,
        params: Option<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let Some(params) = params else {
            return invalid_params_response(id, "missing params for logging/setLevel");
        };
        let set_level: McpLoggingSetLevelParams = match serde_json::from_value(params) {
            Ok(value) => value,
            Err(error) => {
                return invalid_params_response(
                    id,
                    &format!("invalid logging/setLevel params: {error}"),
                );
            },
        };
        match LogLevel::from_level_str(&set_level.level) {
            Some(level) => {
                self.log_level.store(level as u8, Ordering::Relaxed);
                ok_response(id, Some(JsonValue::Object(serde_json::Map::new())))
            },
            None => invalid_params_response(id, &format!("unknown log level: {}", set_level.level)),
        }
    }

    /// `completion/complete` — 参数补全。
    ///
    /// 当前未实现具体补全逻辑，返回空补全值集合。
    fn handle_completion_complete(
        &self,
        id: JsonRpcId,
        params: Option<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let Some(params) = params else {
            return invalid_params_response(id, "missing params for completion/complete");
        };
        let _: McpCompletionCompleteParams = match serde_json::from_value(params) {
            Ok(value) => value,
            Err(error) => {
                return invalid_params_response(
                    id,
                    &format!("invalid completion/complete params: {error}"),
                );
            },
        };
        let result = McpCompletionResult {
            completion: McpCompletionValues {
                values: Vec::new(),
                total: Some(0),
                has_more: Some(false),
            },
        };
        ok_response(id, serde_json::to_value(result).ok())
    }

    /// `sampling/createMessage` — 调用宿主 LLM 进行采样。
    ///
    /// 未注入 [`LlmSampler`] 时返回 `MethodNotFound`；
    /// 注入后委托给桥接器执行，桥接器错误返回 `Internal error`。
    fn handle_sampling_create_message(
        &self,
        id: JsonRpcId,
        params: Option<JsonValue>,
    ) -> JsonRpcResponse<JsonValue> {
        let Some(sampler) = &self.spec.llm_sampler else {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "sampling not configured: no LLM sampler injected".to_string(),
                    data: None,
                }),
            };
        };
        let Some(params) = params else {
            return invalid_params_response(id, "missing params for sampling/createMessage");
        };
        let sample: McpSamplingCreateMessageParams = match serde_json::from_value(params) {
            Ok(value) => value,
            Err(error) => {
                return invalid_params_response(
                    id,
                    &format!("invalid sampling/createMessage params: {error}"),
                );
            },
        };
        match sampler.sample(sample) {
            Ok(result) => ok_response(id, serde_json::to_value(result).ok()),
            Err(message) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError { code: -32603, message, data: None }),
            },
        }
    }
}

/// 构造 MCP 2025-03-26 完整 capabilities 声明。
fn server_capabilities() -> JsonValue {
    json!({
        "tools": {"listChanged": true},
        "resources": {"listChanged": true},
        "prompts": {"listChanged": true},
        "sampling": {},
        "logging": {}
    })
}

/// 构造成功响应（无 error）。
fn ok_response(id: JsonRpcId, result: Option<JsonValue>) -> JsonRpcResponse<JsonValue> {
    JsonRpcResponse { jsonrpc: "2.0".to_string(), id, result, error: None }
}

fn invalid_params_response(id: JsonRpcId, message: &str) -> JsonRpcResponse<JsonValue> {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError { code: -32602, message: message.to_string(), data: None }),
    }
}

/// Reads a single LSP-framed JSON-RPC payload from `reader`.
///
/// Returns `Ok(None)` on clean EOF before any header bytes have been read,
/// matching how [`crate::mcp_stdio::McpStdioProcess`] treats stream closure.
async fn read_frame(reader: &mut BufReader<Stdin>) -> io::Result<Option<Vec<u8>>> {
    let mut content_length: Option<usize> = None;
    let mut first_header = true;
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            if first_header {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "MCP stdio stream closed while reading headers",
            ));
        }
        first_header = false;
        if line == "\r\n" || line == "\n" {
            break;
        }
        let header = line.trim_end_matches(['\r', '\n']);
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("Content-Length")
        {
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            content_length = Some(parsed);
        }
    }

    let content_length = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length header")
    })?;
    let mut payload = vec![0_u8; content_length];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

async fn write_response(
    stdout: &mut Stdout,
    response: &JsonRpcResponse<JsonValue>,
) -> io::Result<()> {
    let body = serde_json::to_vec(response)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    stdout.write_all(header.as_bytes()).await?;
    stdout.write_all(&body).await?;
    stdout.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造最小化测试用 `McpServerSpec`。
    fn make_spec(tools: Vec<McpTool>, handler: ToolCallHandler) -> McpServerSpec {
        McpServerSpec {
            server_name: "test".to_string(),
            server_version: "0.0.0".to_string(),
            tools,
            tool_handler: handler,
            resources: Vec::new(),
            prompts: Vec::new(),
            llm_sampler: None,
        }
    }

    /// 构造测试用 `McpServer`（不触碰真实 stdin/stdout 的读写）。
    fn make_server(spec: McpServerSpec) -> McpServer {
        McpServer {
            spec,
            stdin: BufReader::new(stdin()),
            stdout: stdout(),
            subscribed_uris: Arc::new(Mutex::new(HashSet::new())),
            log_level: Arc::new(AtomicU8::new(LogLevel::default() as u8)),
        }
    }

    fn request(id: u64, method: &str, params: Option<JsonValue>) -> JsonRpcRequest<JsonValue> {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(id),
            method: method.to_string(),
            params,
        }
    }

    #[tokio::test]
    async fn dispatch_initialize_returns_server_info() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response = server.dispatch(request(1, "initialize", None)).await;
        assert_eq!(response.id, JsonRpcId::Number(1));
        assert!(response.error.is_none());
        let result = response.result.expect("initialize result");
        assert_eq!(result["protocolVersion"], MCP_SERVER_PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], "test");
        assert_eq!(result["serverInfo"]["version"], "0.0.0");
    }

    #[tokio::test]
    async fn dispatch_initialize_advertises_full_capabilities() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response = server.dispatch(request(1, "initialize", None)).await;
        let result = response.result.expect("initialize result");
        let caps = &result["capabilities"];
        assert_eq!(caps["tools"]["listChanged"], true);
        assert_eq!(caps["resources"]["listChanged"], true);
        assert_eq!(caps["prompts"]["listChanged"], true);
        assert!(caps.get("sampling").is_some());
        assert!(caps.get("logging").is_some());
    }

    #[tokio::test]
    async fn dispatch_tools_list_returns_registered_tools() {
        let tool = McpTool {
            name: "echo".to_string(),
            description: Some("Echo".to_string()),
            input_schema: Some(json!({"type": "object"})),
            annotations: None,
            meta: None,
        };
        let server = make_server(make_spec(vec![tool.clone()], Box::new(|_, _| Ok(String::new()))));
        let response = server.dispatch(request(2, "tools/list", None)).await;
        assert!(response.error.is_none());
        let result = response.result.expect("tools/list result");
        assert_eq!(result["tools"][0]["name"], "echo");
    }

    #[tokio::test]
    async fn dispatch_tools_call_wraps_handler_output() {
        let server = make_server(make_spec(
            Vec::new(),
            Box::new(|name, args| Ok(format!("called {name} with {args}"))),
        ));
        let response = server
            .dispatch(request(
                3,
                "tools/call",
                Some(json!({"name": "echo", "arguments": {"text": "hi"}})),
            ))
            .await;
        assert!(response.error.is_none());
        let result = response.result.expect("tools/call result");
        assert_eq!(result["isError"], false);
        assert_eq!(result["content"][0]["type"], "text");
        assert!(result["content"][0]["text"].as_str().unwrap().starts_with("called echo"));
    }

    #[tokio::test]
    async fn dispatch_tools_call_surfaces_handler_error() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Err("boom".to_string()))));
        let response =
            server.dispatch(request(4, "tools/call", Some(json!({"name": "broken"})))).await;
        let result = response.result.expect("tools/call result");
        assert_eq!(result["isError"], true);
        assert_eq!(result["content"][0]["text"], "boom");
    }

    #[tokio::test]
    async fn dispatch_unknown_method_returns_method_not_found() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response = server.dispatch(request(5, "nonsense", None)).await;
        let error = response.error.expect("error payload");
        assert_eq!(error.code, -32601);
    }

    // ── resources 测试 ──

    #[tokio::test]
    async fn dispatch_resources_list_returns_registered_resources() {
        let mut spec = make_spec(Vec::new(), Box::new(|_, _| Ok(String::new())));
        spec.resources.push(McpResource {
            uri: "file://guide.txt".to_string(),
            name: Some("guide".to_string()),
            description: None,
            mime_type: Some("text/plain".to_string()),
            annotations: None,
            meta: None,
        });
        let server = make_server(spec);
        let response = server.dispatch(request(10, "resources/list", None)).await;
        assert!(response.error.is_none());
        let result = response.result.expect("resources/list result");
        assert_eq!(result["resources"][0]["uri"], "file://guide.txt");
        assert_eq!(result["resources"][0]["mimeType"], "text/plain");
    }

    #[tokio::test]
    async fn dispatch_resources_read_returns_empty_text_for_known_uri() {
        let mut spec = make_spec(Vec::new(), Box::new(|_, _| Ok(String::new())));
        spec.resources.push(McpResource {
            uri: "file://guide.txt".to_string(),
            name: None,
            description: None,
            mime_type: Some("text/plain".to_string()),
            annotations: None,
            meta: None,
        });
        let server = make_server(spec);
        let response = server
            .dispatch(request(11, "resources/read", Some(json!({"uri": "file://guide.txt"}))))
            .await;
        assert!(response.error.is_none());
        let result = response.result.expect("resources/read result");
        assert_eq!(result["contents"][0]["uri"], "file://guide.txt");
        assert_eq!(result["contents"][0]["text"], "");
    }

    #[tokio::test]
    async fn dispatch_resources_read_rejects_unknown_uri() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response = server
            .dispatch(request(12, "resources/read", Some(json!({"uri": "file://missing"}))))
            .await;
        let error = response.error.expect("error payload");
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("unknown resource uri"));
    }

    #[tokio::test]
    async fn dispatch_resources_subscribe_records_uri() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response = server
            .dispatch(request(13, "resources/subscribe", Some(json!({"uri": "file://x"}))))
            .await;
        assert!(response.error.is_none());
        assert!(server.subscribed_uris.lock().await.contains("file://x"));
    }

    #[tokio::test]
    async fn dispatch_resources_unsubscribe_removes_uri() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        server.subscribed_uris.lock().await.insert("file://x".to_string());
        let response = server
            .dispatch(request(14, "resources/unsubscribe", Some(json!({"uri": "file://x"}))))
            .await;
        assert!(response.error.is_none());
        assert!(!server.subscribed_uris.lock().await.contains("file://x"));
    }

    // ── prompts 测试 ──

    #[tokio::test]
    async fn dispatch_prompts_list_returns_registered_prompts() {
        let mut spec = make_spec(Vec::new(), Box::new(|_, _| Ok(String::new())));
        spec.prompts.push(McpPrompt {
            name: "greet".to_string(),
            description: Some("Greet user".to_string()),
            arguments: Vec::new(),
        });
        let server = make_server(spec);
        let response = server.dispatch(request(20, "prompts/list", None)).await;
        assert!(response.error.is_none());
        let result = response.result.expect("prompts/list result");
        assert_eq!(result["prompts"][0]["name"], "greet");
    }

    #[tokio::test]
    async fn dispatch_prompts_get_returns_prompt_for_known_name() {
        let mut spec = make_spec(Vec::new(), Box::new(|_, _| Ok(String::new())));
        spec.prompts.push(McpPrompt {
            name: "greet".to_string(),
            description: Some("Greet user".to_string()),
            arguments: Vec::new(),
        });
        let server = make_server(spec);
        let response =
            server.dispatch(request(21, "prompts/get", Some(json!({"name": "greet"})))).await;
        assert!(response.error.is_none());
        let result = response.result.expect("prompts/get result");
        assert_eq!(result["description"], "Greet user");
        assert!(result["messages"].is_array());
    }

    #[tokio::test]
    async fn dispatch_prompts_get_rejects_unknown_name() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response =
            server.dispatch(request(22, "prompts/get", Some(json!({"name": "missing"})))).await;
        let error = response.error.expect("error payload");
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("unknown prompt"));
    }

    // ── logging 测试 ──

    #[tokio::test]
    async fn dispatch_logging_set_level_updates_state() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response =
            server.dispatch(request(30, "logging/setLevel", Some(json!({"level": "debug"})))).await;
        assert!(response.error.is_none());
        assert_eq!(server.log_level.load(Ordering::Relaxed), LogLevel::Debug as u8);
    }

    #[tokio::test]
    async fn dispatch_logging_set_level_rejects_unknown_level() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response = server
            .dispatch(request(31, "logging/setLevel", Some(json!({"level": "verbose"}))))
            .await;
        let error = response.error.expect("error payload");
        assert_eq!(error.code, -32602);
    }

    // ── completion 测试 ──

    #[tokio::test]
    async fn dispatch_completion_complete_returns_empty_values() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response = server
            .dispatch(request(
                40,
                "completion/complete",
                Some(json!({
                    "ref": {"type": "ref/prompt", "name": "greet"},
                    "argument": {"name": "name", "value": "Al"}
                })),
            ))
            .await;
        assert!(response.error.is_none());
        let result = response.result.expect("completion/complete result");
        assert_eq!(result["completion"]["values"], json!([]));
        assert_eq!(result["completion"]["total"], 0);
        assert_eq!(result["completion"]["hasMore"], false);
    }

    // ── sampling 测试 ──

    #[tokio::test]
    async fn dispatch_sampling_returns_method_not_found_without_sampler() {
        let server = make_server(make_spec(Vec::new(), Box::new(|_, _| Ok(String::new()))));
        let response = server
            .dispatch(request(
                50,
                "sampling/createMessage",
                Some(json!({"messages": [], "maxTokens": 16})),
            ))
            .await;
        let error = response.error.expect("error payload");
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("sampling not configured"));
    }

    /// 用于测试的 `LlmSampler` 桩实现：原样回显最后一条消息。
    struct EchoSampler;

    impl LlmSampler for EchoSampler {
        fn sample(
            &self,
            params: McpSamplingCreateMessageParams,
        ) -> Result<McpSamplingCreateMessageResult, String> {
            let last = params.messages.last().ok_or_else(|| "no messages".to_string())?;
            Ok(McpSamplingCreateMessageResult {
                role: "assistant".to_string(),
                content: last.content.clone(),
                model: Some("echo-model".to_string()),
                stop_reason: Some("endTurn".to_string()),
                stop_sequence: None,
                usage: Some(json!({"inputTokens": 1, "outputTokens": 1})),
            })
        }
    }

    #[tokio::test]
    async fn dispatch_sampling_invokes_sampler_when_injected() {
        let mut spec = make_spec(Vec::new(), Box::new(|_, _| Ok(String::new())));
        spec.llm_sampler = Some(Arc::new(EchoSampler));
        let server = make_server(spec);
        let response = server
            .dispatch(request(
                51,
                "sampling/createMessage",
                Some(json!({
                    "messages": [{"role": "user", "content": {"type": "text", "text": "hi"}}],
                    "maxTokens": 16
                })),
            ))
            .await;
        assert!(response.error.is_none());
        let result = response.result.expect("sampling result");
        assert_eq!(result["role"], "assistant");
        assert_eq!(result["model"], "echo-model");
        assert_eq!(result["stopReason"], "endTurn");
    }

    #[tokio::test]
    async fn dispatch_sampling_surfaces_sampler_error() {
        struct FailingSampler;
        impl LlmSampler for FailingSampler {
            fn sample(
                &self,
                _: McpSamplingCreateMessageParams,
            ) -> Result<McpSamplingCreateMessageResult, String> {
                Err("sampler boom".to_string())
            }
        }
        let mut spec = make_spec(Vec::new(), Box::new(|_, _| Ok(String::new())));
        spec.llm_sampler = Some(Arc::new(FailingSampler));
        let server = make_server(spec);
        let response = server
            .dispatch(request(
                52,
                "sampling/createMessage",
                Some(json!({"messages": [], "maxTokens": 16})),
            ))
            .await;
        let error = response.error.expect("error payload");
        assert_eq!(error.code, -32603);
        assert_eq!(error.message, "sampler boom");
    }
}
