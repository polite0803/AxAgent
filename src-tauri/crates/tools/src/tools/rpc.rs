use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};

static RPC_REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);
static RPC_WINDOW_START: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
const RATE_LIMIT_PER_MINUTE: u64 = 100;

fn axagent_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".axagent")
}

#[cfg(unix)]
fn rpc_socket_path() -> PathBuf {
    axagent_dir().join("rpc.sock")
}

#[cfg(windows)]
fn rpc_pipe_name() -> String {
    r"\\.\pipe\axagent-rpc".to_string()
}

fn audit_log_path() -> PathBuf {
    axagent_dir().join("rpc-audit.log")
}

fn check_rate_limit() -> bool {
    let mut guard = match RPC_WINDOW_START.lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    let now = Instant::now();
    match *guard {
        None => {
            *guard = Some(now);
            RPC_REQUEST_COUNT.store(1, Ordering::Relaxed);
            true
        },
        Some(start) => {
            let elapsed = now.duration_since(start);
            if elapsed.as_secs() >= 60 {
                *guard = Some(now);
                RPC_REQUEST_COUNT.store(1, Ordering::Relaxed);
                true
            } else {
                let count = RPC_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                count <= RATE_LIMIT_PER_MINUTE
            }
        },
    }
}

fn append_audit_log(entry: &str) -> Result<(), ToolError> {
    let dir = axagent_dir();
    std::fs::create_dir_all(&dir).map_err(|e| {
        ToolError::execution_failed(format!("Failed to create .axagent directory: {}", e))
    })?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let line = format!("[{}] {}\n", timestamp, entry);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_log_path())
        .map_err(|e| ToolError::execution_failed(format!("Failed to open rpc-audit.log: {}", e)))?;
    file.write_all(line.as_bytes())
        .map_err(|e| ToolError::execution_failed(format!("Failed to write rpc-audit.log: {}", e)))
}

// ── JSON-RPC 2.0 types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Option<Value>,
    id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcError {
    fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: None,
        }
    }

    fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    fn invalid_params(msg: &str) -> Self {
        Self {
            code: -32602,
            message: format!("Invalid params: {}", msg),
            data: None,
        }
    }

    fn internal_error(msg: &str) -> Self {
        Self {
            code: -32603,
            message: format!("Internal error: {}", msg),
            data: None,
        }
    }

    fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        }
    }
}

fn validate_request(req: &JsonRpcRequest) -> Result<(), JsonRpcError> {
    if req.jsonrpc != "2.0" {
        return Err(JsonRpcError::invalid_request());
    }
    Ok(())
}

fn make_success_response(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(result),
        error: None,
        id,
    }
}

fn make_error_response(id: Option<Value>, error: JsonRpcError) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(error),
        id,
    }
}

// ── RpcServer ──

pub struct RpcServer {
    running: Arc<AtomicBool>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    local_addr: Arc<parking_lot::Mutex<Option<String>>>,
}

impl RpcServer {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
            local_addr: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn address(&self) -> Option<String> {
        self.local_addr.lock().clone()
    }
}

#[cfg(windows)]
async fn accept_pipe_connection(
    pipe_name: &str,
) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    let server = tokio::net::windows::named_pipe::ServerOptions::new()
        .first_pipe_instance(false)
        .create(pipe_name)?;
    server.connect().await?;
    Ok(server)
}

static GLOBAL_RPC_SERVER: std::sync::LazyLock<parking_lot::Mutex<RpcServer>> =
    std::sync::LazyLock::new(|| parking_lot::Mutex::new(RpcServer::new()));

async fn do_start_server() -> Result<String, ToolError> {
    let running: Arc<AtomicBool>;
    let local_addr: Arc<parking_lot::Mutex<Option<String>>>;
    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();

    {
        let mut server = GLOBAL_RPC_SERVER.lock();
        if server.is_running() {
            return Err(ToolError::execution_failed("RPC server is already running"));
        }
        running = server.running.clone();
        local_addr = server.local_addr.clone();
        server.shutdown_tx = Some(tx);
        running.store(true, Ordering::Relaxed);
    }

    #[cfg(unix)]
    {
        let socket_path = rpc_socket_path();
        if socket_path.exists() {
            std::fs::remove_file(&socket_path).map_err(|e| {
                ToolError::execution_failed(format!("Failed to remove existing socket: {}", e))
            })?;
        }
        let dir = socket_path.parent().unwrap_or(&socket_path);
        std::fs::create_dir_all(dir).map_err(|e| {
            ToolError::execution_failed(format!("Failed to create socket directory: {}", e))
        })?;

        let listener = tokio::net::UnixListener::bind(&socket_path).map_err(|e| {
            ToolError::execution_failed(format!("Failed to bind Unix socket: {}", e))
        })?;

        let addr_str = format!("unix:{}", socket_path.display());
        *local_addr.lock() = Some(addr_str.clone());

        let _ = append_audit_log(&format!("RPC server started at {}", addr_str));

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, _addr)) => {
                                if !check_rate_limit() {
                                    let _ = append_audit_log(
                                        "RPC request rejected: rate limit exceeded"
                                    );
                                    continue;
                                }
                                tokio::spawn(
                                    handle_unix_connection(stream),
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "RPC accept error: {}",
                                    e
                                );
                            }
                        }
                    }
                    _ = &mut rx => {
                        let _ = append_audit_log(
                            "RPC server shutting down"
                        );
                        let _ = std::fs::remove_file(&socket_path);
                        running.store(false, Ordering::Relaxed);
                        return;
                    }
                }
            }
        });

        Ok(addr_str)
    }

    #[cfg(windows)]
    {
        let pipe_name = rpc_pipe_name();
        let addr_str = format!("pipe:{}", pipe_name);
        *local_addr.lock() = Some(addr_str.clone());

        let _ = append_audit_log(&format!("RPC server started at {}", addr_str));

        let pipe_name_clone = pipe_name.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    connect_result =
                        accept_pipe_connection(&pipe_name_clone) =>
                    {
                        match connect_result {
                            Ok(stream) => {
                                if !check_rate_limit() {
                                    let _ = append_audit_log(
                                        "RPC request rejected: rate limit exceeded"
                                    );
                                    continue;
                                }
                                tokio::spawn(
                                    handle_pipe_connection(stream),
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "RPC pipe accept error: {}",
                                    e
                                );
                            }
                        }
                    }
                    _ = &mut rx => {
                        let _ = append_audit_log(
                            "RPC server shutting down"
                        );
                        running.store(false, Ordering::Relaxed);
                        return;
                    }
                }
            }
        });

        Ok(addr_str)
    }
}

// ── Connection handlers ──

#[cfg(unix)]
async fn handle_unix_connection(stream: tokio::net::UnixStream) {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    match buf_reader.read_line(&mut line).await {
        Ok(0) => return,
        Ok(_) => {},
        Err(e) => {
            tracing::warn!("RPC read error: {}", e);
            return;
        },
    }

    let response = process_rpc_message(line.trim()).await;
    let response_bytes = match serde_json::to_string(&response) {
        Ok(s) => s,
        Err(e) => {
            let err_resp = make_error_response(
                None,
                JsonRpcError::internal_error(&format!("Serialization error: {}", e)),
            );
            serde_json::to_string(&err_resp).unwrap_or_default()
        },
    };

    let _ = writer
        .write_all(format!("{}\n", response_bytes).as_bytes())
        .await;
}

#[cfg(windows)]
async fn handle_pipe_connection(stream: tokio::net::windows::named_pipe::NamedPipeServer) {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    match buf_reader.read_line(&mut line).await {
        Ok(0) => return,
        Ok(_) => {},
        Err(e) => {
            tracing::warn!("RPC read error: {}", e);
            return;
        },
    }

    let response = process_rpc_message(line.trim()).await;
    let response_bytes = match serde_json::to_string(&response) {
        Ok(s) => s,
        Err(e) => {
            let err_resp = make_error_response(
                None,
                JsonRpcError::internal_error(&format!("Serialization error: {}", e)),
            );
            serde_json::to_string(&err_resp).unwrap_or_default()
        },
    };

    let _ = writer
        .write_all(format!("{}\n", response_bytes).as_bytes())
        .await;
}

async fn process_rpc_message(msg: &str) -> Value {
    if msg.is_empty() {
        let resp = make_error_response(None, JsonRpcError::parse_error());
        return serde_json::to_value(resp).unwrap_or(Value::Null);
    }

    let parsed: Value = match serde_json::from_str(msg) {
        Ok(v) => v,
        Err(_) => {
            let resp = make_error_response(None, JsonRpcError::parse_error());
            return serde_json::to_value(resp).unwrap_or(Value::Null);
        },
    };

    if parsed.is_array() {
        let mut batch = Vec::new();
        for item in parsed.as_array().unwrap() {
            batch.push(process_single_request(item).await);
        }
        Value::Array(batch)
    } else {
        process_single_request(&parsed).await
    }
}

async fn process_single_request(value: &Value) -> Value {
    let request: JsonRpcRequest = match serde_json::from_value(value.clone()) {
        Ok(r) => r,
        Err(_) => {
            let resp = make_error_response(None, JsonRpcError::invalid_request());
            return serde_json::to_value(resp).unwrap_or(Value::Null);
        },
    };

    if let Err(e) = validate_request(&request) {
        let resp = make_error_response(request.id.clone(), e);
        return serde_json::to_value(resp).unwrap_or(Value::Null);
    }

    if request.id.is_none() {
        return Value::Null;
    }

    let _ = append_audit_log(&format!(
        "method={} params={}",
        request.method,
        request
            .params
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_default()
    ));

    let result = dispatch_rpc_method(&request.method, request.params.as_ref()).await;

    let resp = match result {
        Ok(val) => make_success_response(request.id, val),
        Err(e) => make_error_response(request.id, e),
    };

    serde_json::to_value(resp).unwrap_or(Value::Null)
}

async fn dispatch_rpc_method(method: &str, params: Option<&Value>) -> Result<Value, JsonRpcError> {
    match method {
        "tools.list" => {
            let registry = get_global_registry();
            let tools = registry.list_all();
            let tool_names: Vec<Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.input_schema,
                        "category": format!("{:?}", t.category),
                    })
                })
                .collect();
            Ok(Value::Array(tool_names))
        },
        "tools.call" => {
            let params = params
                .ok_or_else(|| JsonRpcError::invalid_params("params required for tools.call"))?;
            let tool_name = params["tool"]
                .as_str()
                .ok_or_else(|| JsonRpcError::invalid_params("tool name required"))?;
            let tool_input = params
                .get("input")
                .cloned()
                .unwrap_or(Value::Object(serde_json::Map::new()));

            let registry = get_global_registry();
            let tool = registry
                .find(tool_name)
                .ok_or_else(|| JsonRpcError::method_not_found(tool_name))?;

            let ctx = ToolContext::new(
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
            );

            let result = tool.call(tool_input, &ctx).await;

            match result {
                Ok(r) => {
                    if r.is_error {
                        Err(JsonRpcError::internal_error(&r.content))
                    } else {
                        Ok(serde_json::json!({
                            "content": r.content,
                            "metadata": r.metadata,
                        }))
                    }
                },
                Err(e) => Err(JsonRpcError::internal_error(&e.message)),
            }
        },
        "ping" => Ok(serde_json::json!({"pong": true})),
        _ => Err(JsonRpcError::method_not_found(method)),
    }
}

fn get_global_registry() -> crate::registry::ToolRegistry {
    let mut reg = crate::registry::ToolRegistry::new();
    crate::tools::register_all(&mut reg);
    reg
}

// ── RpcTool ──

pub struct RpcTool;

#[async_trait]
impl Tool for RpcTool {
    fn name(&self) -> &str {
        "Rpc"
    }

    fn description(&self) -> &str {
        "管理 JSON-RPC 2.0 服务器，允许外部脚本调用 agent 工具。\
         支持操作：start（启动 RPC 服务器）、stop（停止服务器）、\
         status（查询运行状态和地址）、call（通过 RPC 调用工具）。\
         服务器监听 Unix socket (Linux/Mac) 或 Windows named pipe。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "stop", "status", "call"],
                    "description": "操作类型：start/stop/status/call"
                },
                "method": {
                    "type": "string",
                    "description": "RPC 方法名（call 操作时使用，如 tools.list, tools.call, ping）"
                },
                "params": {
                    "type": "object",
                    "description": "RPC 调用参数（call 操作时使用）"
                }
            },
            "required": ["action"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    fn is_destructive(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = input["action"].as_str().unwrap_or("");

        match action {
            "start" => self.handle_start().await,
            "stop" => self.handle_stop().await,
            "status" => self.handle_status().await,
            "call" => self.handle_call(&input).await,
            _ => Err(ToolError::invalid_input(format!(
                "Unknown action '{}'. Supported: start, stop, status, call",
                action
            ))),
        }
    }
}

impl RpcTool {
    async fn handle_start(&self) -> Result<ToolResult, ToolError> {
        {
            let server = GLOBAL_RPC_SERVER.lock();
            if server.is_running() {
                return Err(ToolError::execution_failed("RPC server is already running"));
            }
        }

        let addr = do_start_server().await?;

        Ok(ToolResult {
            content: format!("✅ RPC 服务器已启动，监听地址: {}", addr),
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "running": true,
                "address": addr,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }

    async fn handle_stop(&self) -> Result<ToolResult, ToolError> {
        let shutdown_tx = {
            let mut server = GLOBAL_RPC_SERVER.lock();
            if !server.is_running() {
                return Err(ToolError::execution_failed("RPC server is not running"));
            }
            server.running.store(false, Ordering::Relaxed);
            *server.local_addr.lock() = None;
            server.shutdown_tx.take()
        };

        if let Some(tx) = shutdown_tx {
            let _ = tx.send(());
        }

        let _ = append_audit_log("RPC server stopped");

        Ok(ToolResult {
            content: "✅ RPC 服务器已停止".to_string(),
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "running": false,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }

    async fn handle_status(&self) -> Result<ToolResult, ToolError> {
        let (running, addr) = {
            let server = GLOBAL_RPC_SERVER.lock();
            (server.is_running(), server.address())
        };

        let status_str = if running {
            format!("RPC 服务器运行中，地址: {}", addr.as_deref().unwrap_or("(unknown)"))
        } else {
            "RPC 服务器未运行".to_string()
        };

        Ok(ToolResult {
            content: status_str,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "running": running,
                "address": addr,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }

    async fn handle_call(&self, input: &Value) -> Result<ToolResult, ToolError> {
        let method = input["method"].as_str().unwrap_or("").to_string();
        if method.is_empty() {
            return Err(ToolError::invalid_input("method is required for call action"));
        }

        let params = input.get("params").cloned();

        let result = dispatch_rpc_method(&method, params.as_ref()).await;

        match result {
            Ok(val) => Ok(ToolResult {
                content: serde_json::to_string_pretty(&val).unwrap_or_default(),
                is_error: false,
                truncated: false,
                metadata: Some(serde_json::json!({
                    "method": method,
                    "success": true,
                })),
                duration_ms: None,
                progress: Vec::new(),
            }),
            Err(e) => Ok(ToolResult {
                content: format!("RPC call failed: [{}] {}", e.code, e.message),
                is_error: true,
                truncated: false,
                metadata: Some(serde_json::json!({
                    "method": method,
                    "success": false,
                    "error_code": e.code,
                    "error_message": e.message,
                })),
                duration_ms: None,
                progress: Vec::new(),
            }),
        }
    }
}

// ── RpcCallTool ──

pub struct RpcCallTool;

#[async_trait]
impl Tool for RpcCallTool {
    fn name(&self) -> &str {
        "RpcCall"
    }

    fn description(&self) -> &str {
        "向外部 JSON-RPC 2.0 服务发起调用。\
         通过 HTTP 或 Unix socket/Windows named pipe 连接到外部 RPC 服务，\
         发送 JSON-RPC 2.0 请求并返回响应。\
         支持指定传输方式（http/unix/pipe）、目标地址、方法名和参数。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "transport": {
                    "type": "string",
                    "enum": ["http", "unix", "pipe"],
                    "description": "传输方式：http（HTTP POST）、unix（Unix socket）、pipe（Windows named pipe）",
                    "default": "http"
                },
                "endpoint": {
                    "type": "string",
                    "description": "目标地址：HTTP URL、Unix socket 路径或 named pipe 名称"
                },
                "method": {
                    "type": "string",
                    "description": "JSON-RPC 方法名"
                },
                "params": {
                    "description": "JSON-RPC 参数（对象或数组）",
                    "type": "object"
                },
                "id": {
                    "description": "请求 ID（默认自动生成）",
                    "type": "integer"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "超时时间（毫秒，默认 30000）",
                    "default": 30000
                }
            },
            "required": ["transport", "endpoint", "method"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let transport = input["transport"].as_str().unwrap_or("http");
        let endpoint = input["endpoint"].as_str().unwrap_or("");
        let method = input["method"].as_str().unwrap_or("");
        let params = input.get("params").cloned();
        let id = input
            .get("id")
            .cloned()
            .unwrap_or(Value::Number(serde_json::Number::from(1)));
        let timeout_ms = input["timeout_ms"].as_u64().unwrap_or(30000);

        if endpoint.is_empty() {
            return Err(ToolError::invalid_input("endpoint is required"));
        }
        if method.is_empty() {
            return Err(ToolError::invalid_input("method is required"));
        }

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(id),
        };

        let request_body = serde_json::to_string(&request).map_err(|e| {
            ToolError::execution_failed(format!("Failed to serialize RPC request: {}", e))
        })?;

        let _ = append_audit_log(&format!(
            "RpcCall transport={} endpoint={} method={}",
            transport, endpoint, method
        ));

        let result = match transport {
            "http" => call_http_rpc(endpoint, &request_body, timeout_ms).await,
            #[cfg(unix)]
            "unix" => call_unix_rpc(endpoint, &request_body, timeout_ms).await,
            #[cfg(windows)]
            "pipe" => call_pipe_rpc(endpoint, &request_body, timeout_ms).await,
            _ => Err(ToolError::invalid_input(format!(
                "Unsupported transport: '{}'. Supported: http, unix, pipe",
                transport
            ))),
        };

        match result {
            Ok(response_body) => {
                let response: JsonRpcResponse =
                    serde_json::from_str(&response_body).map_err(|e| {
                        ToolError::execution_failed(format!("Failed to parse RPC response: {}", e))
                    })?;

                if let Some(error) = &response.error {
                    Ok(ToolResult {
                        content: format!("RPC error [{}]: {}", error.code, error.message),
                        is_error: true,
                        truncated: false,
                        metadata: Some(serde_json::json!({
                            "transport": transport,
                            "endpoint": endpoint,
                            "method": method,
                            "error_code": error.code,
                            "error_message": error.message,
                        })),
                        duration_ms: None,
                        progress: Vec::new(),
                    })
                } else {
                    let result_val = response.result.unwrap_or(Value::Null);
                    Ok(ToolResult {
                        content: serde_json::to_string_pretty(&result_val).unwrap_or_default(),
                        is_error: false,
                        truncated: false,
                        metadata: Some(serde_json::json!({
                            "transport": transport,
                            "endpoint": endpoint,
                            "method": method,
                        })),
                        duration_ms: None,
                        progress: Vec::new(),
                    })
                }
            },
            Err(e) => Ok(ToolResult {
                content: format!("RPC call failed: {}", e.message),
                is_error: true,
                truncated: false,
                metadata: Some(serde_json::json!({
                    "transport": transport,
                    "endpoint": endpoint,
                    "method": method,
                    "error": e.message,
                })),
                duration_ms: None,
                progress: Vec::new(),
            }),
        }
    }
}

async fn call_http_rpc(endpoint: &str, body: &str, timeout_ms: u64) -> Result<String, ToolError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| ToolError::execution_failed(format!("Failed to build HTTP client: {}", e)))?;

    let resp = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| ToolError::execution_failed(format!("HTTP RPC request failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(ToolError::execution_failed(format!(
            "HTTP RPC returned status: {}",
            resp.status()
        )));
    }

    resp.text().await.map_err(|e| {
        ToolError::execution_failed(format!("Failed to read HTTP RPC response: {}", e))
    })
}

#[cfg(unix)]
async fn call_unix_rpc(
    socket_path: &str,
    body: &str,
    timeout_ms: u64,
) -> Result<String, ToolError> {
    let stream = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        tokio::net::UnixStream::connect(socket_path),
    )
    .await
    .map_err(|_| ToolError::execution_failed("Unix socket connect timeout"))?
    .map_err(|e| ToolError::execution_failed(format!("Unix socket connect failed: {}", e)))?;

    let (mut reader, mut writer) = stream.into_split();

    tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        writer.write_all(format!("{}\n", body).as_bytes()),
    )
    .await
    .map_err(|_| ToolError::execution_failed("Unix socket write timeout"))?
    .map_err(|e| ToolError::execution_failed(format!("Unix socket write failed: {}", e)))?;

    let mut response = String::new();
    let mut buf_reader = BufReader::new(&mut reader);
    tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        buf_reader.read_line(&mut response),
    )
    .await
    .map_err(|_| ToolError::execution_failed("Unix socket read timeout"))?
    .map_err(|e| ToolError::execution_failed(format!("Unix socket read failed: {}", e)))?;

    Ok(response.trim_end().to_string())
}

#[cfg(windows)]
async fn call_pipe_rpc(pipe_name: &str, body: &str, timeout_ms: u64) -> Result<String, ToolError> {
    let client = tokio::net::windows::named_pipe::ClientOptions::new()
        .open(pipe_name)
        .map_err(|e| ToolError::execution_failed(format!("Named pipe connect failed: {}", e)))?;

    let (mut reader, mut writer) = tokio::io::split(client);

    tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        writer.write_all(format!("{}\n", body).as_bytes()),
    )
    .await
    .map_err(|_| ToolError::execution_failed("Named pipe write timeout"))?
    .map_err(|e| ToolError::execution_failed(format!("Named pipe write failed: {}", e)))?;

    let mut response = String::new();
    let mut buf_reader = BufReader::new(&mut reader);
    tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        buf_reader.read_line(&mut response),
    )
    .await
    .map_err(|_| ToolError::execution_failed("Named pipe read timeout"))?
    .map_err(|e| ToolError::execution_failed(format!("Named pipe read failed: {}", e)))?;

    Ok(response.trim_end().to_string())
}
