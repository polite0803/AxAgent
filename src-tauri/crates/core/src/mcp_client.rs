use crate::error::{AxAgentError, Result};
#[cfg(not(target_os = "android"))]
use rmcp::transport::TokioChildProcess;
use rmcp::{
    model::{CallToolRequestParams, CallToolResult, Tool},
    transport::streamable_http_client::StreamableHttpClientWorker,
    RoleClient, ServiceExt,
};

/// Type alias for a connected MCP client peer.
/// Using Peer<RoleClient> (which is Clone + Send + Sync) instead of the
/// ClientHandler trait allows storing connections in the pool and cloning
/// them for reuse across multiple tool calls.
type McpPeer = rmcp::service::Peer<RoleClient>;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tracing::info;

static SSE_JSON_RPC_ID: AtomicU64 = AtomicU64::new(1);

fn next_rpc_id() -> u64 {
    SSE_JSON_RPC_ID.fetch_add(1, Ordering::Relaxed)
}

/// Progress update during a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolProgress {
    pub phase: String,
    pub message: String,
    pub percent: Option<u8>,
}

/// Progress callback for streaming updates during execution.
pub type ToolProgressCallback = Arc<dyn Fn(&McpToolProgress) + Send + Sync + 'static>;

/// Result of a tool call via MCP.
#[derive(Debug, Clone)]
pub struct McpToolResult {
    pub content: String,
    pub is_error: bool,
    pub progress: Vec<McpToolProgress>,
}

/// A tool discovered from an MCP server via tools/list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

/// Resolve the user's login shell PATH so that GUI-launched apps can find
/// tools like `npx`, `node`, `python`, etc. that are installed via version
/// managers (nvm, fnm, volta, pyenv, …).
///
/// On macOS/Linux GUI apps inherit a minimal PATH (`/usr/bin:/bin:…`).
/// This function runs the user's login shell once and caches the full PATH.
#[cfg(not(target_os = "android"))]
fn get_shell_path() -> &'static str {
    static SHELL_PATH: OnceLock<String> = OnceLock::new();
    SHELL_PATH.get_or_init(|| resolve_login_shell_path().unwrap_or_default())
}

#[cfg(all(unix, not(target_os = "android")))]
fn resolve_login_shell_path() -> Option<String> {
    let current_path = std::env::var("PATH").ok();
    let mut best_path: Option<String> = None;

    for shell in shell_candidates() {
        if let Some(candidate_path) = read_path_from_shell(&shell) {
            let merged = merge_paths(&candidate_path, current_path.as_deref());
            if path_score(&merged) > best_path.as_ref().map(|path| path_score(path)).unwrap_or(0) {
                best_path = Some(merged);
            }
        }
    }

    best_path.or(current_path)
}

#[cfg(all(unix, not(target_os = "android")))]
fn shell_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for candidate in [
        std::env::var("SHELL").ok(),
        Some("zsh".to_string()),
        Some("/bin/zsh".to_string()),
        Some("bash".to_string()),
        Some("/bin/bash".to_string()),
        Some("sh".to_string()),
        Some("/bin/sh".to_string()),
    ]
    .into_iter()
    .flatten()
    {
        if !candidate.is_empty() && seen.insert(candidate.clone()) {
            candidates.push(candidate);
        }
    }

    candidates
}

#[cfg(all(unix, not(target_os = "android")))]
fn read_path_from_shell(shell: &str) -> Option<String> {
    const START: &str = "__AxAgent_PATH_START__";
    const END: &str = "__AxAgent_PATH_END__";

    let output = std::process::Command::new(shell)
        .args([
            "-i",
            "-l",
            "-c",
            &format!("printf '{}'; printenv PATH; printf '{}'", START, END),
        ])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    extract_marked_path(&output.stdout, START, END)
}

#[cfg(all(unix, not(target_os = "android")))]
fn extract_marked_path(output: &[u8], start: &str, end: &str) -> Option<String> {
    let stdout = String::from_utf8(output.to_vec()).ok()?;
    let start_idx = stdout.find(start)? + start.len();
    let end_idx = stdout[start_idx..].find(end)? + start_idx;
    let path = stdout[start_idx..end_idx].trim().to_string();

    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

#[cfg(all(unix, not(target_os = "android")))]
fn merge_paths(primary: &str, fallback: Option<&str>) -> String {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();

    for path_list in [Some(primary), fallback] {
        for segment in path_list
            .unwrap_or_default()
            .split(':')
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
        {
            if seen.insert(segment.to_string()) {
                merged.push(segment.to_string());
            }
        }
    }

    merged.join(":")
}

#[cfg(all(unix, not(target_os = "android")))]
fn path_score(path: &str) -> usize {
    path.split(':')
        .filter(|segment| !segment.is_empty())
        .count()
}

#[cfg(all(not(unix), not(target_os = "android")))]
fn resolve_login_shell_path() -> Option<String> {
    // On Windows, packaged Tauri apps may not inherit the full PATH
    // from the user's shell (especially paths added by Node version
    // managers like nvm/fnm/volta). We merge the process PATH with
    // the system+user PATH from the Windows Registry to ensure tools
    // like `npx`, `node`, `python` can be found.
    let mut paths: Vec<String> = Vec::new();

    // 1. Start with the process environment PATH
    if let Ok(p) = std::env::var("PATH") {
        paths.push(p);
    }

    // 2. Read system PATH from registry (HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment)
    if let Some(sys_path) = read_registry_path(
        "HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
    ) {
        paths.push(sys_path);
    }

    // 3. Read user PATH from registry (HKCU\Environment)
    if let Some(user_path) = read_registry_path("HKEY_CURRENT_USER\\Environment") {
        paths.push(user_path);
    }

    // Merge and deduplicate while preserving order
    let combined = paths.join(";");
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<&str> = combined
        .split(';')
        .filter(|s| !s.is_empty() && seen.insert(s.to_lowercase()))
        .collect();
    Some(deduped.join(";"))
}

#[cfg(all(not(unix), not(target_os = "android")))]
fn read_registry_path(key: &str) -> Option<String> {
    use std::process::Command;
    let output = Command::new("reg")
        .args(["query", key, "/v", "Path"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // reg query output format:
    //   HKEY_...\Environment
    //       Path    REG_EXPAND_SZ    C:\...;D:\...
    for line in text.lines() {
        let trimmed = line.trim();
        // Skip the key name line and look for the value line
        if trimmed.starts_with("Path") || trimmed.starts_with("PATH") {
            // Format: "Path    REG_EXPAND_SZ    value" or "Path    REG_SZ    value"
            if let Some(idx) = trimmed.find("REG_EXPAND_SZ") {
                let val = trimmed[idx + "REG_EXPAND_SZ".len()..].trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            } else if let Some(idx) = trimmed.find("REG_SZ") {
                let val = trimmed[idx + "REG_SZ".len()..].trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

/// Inject login-shell PATH into the command unless the user already
/// provides an explicit PATH in their custom environment variables.
#[cfg(not(target_os = "android"))]
fn configure_stdio_env(cmd: &mut tokio::process::Command, env: &HashMap<String, String>) {
    let shell_path = get_shell_path();
    if !shell_path.is_empty() && !env.contains_key("PATH") {
        cmd.env("PATH", shell_path);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
}

/// On Windows, commands like `npx` are actually `npx.cmd` batch scripts.
/// Rust's `Command::new("npx")` uses `CreateProcess` which does NOT search
/// for `.cmd`/`.bat` extensions — only `cmd.exe /C` does. This helper
/// wraps the command through `cmd.exe /C` on Windows so that `.cmd` scripts
/// (npx, npm, etc.) can be found and executed correctly.
#[cfg(all(target_os = "windows", not(target_os = "android")))]
fn build_stdio_command(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("cmd.exe");
    let mut all_args: Vec<String> = vec!["/C".to_string(), command.to_string()];
    all_args.extend_from_slice(args);
    cmd.args(&all_args);
    configure_stdio_env(&mut cmd, env);
    cmd
}

#[cfg(all(not(target_os = "windows"), not(target_os = "android")))]
fn build_stdio_command(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args);
    configure_stdio_env(&mut cmd, env);
    cmd
}

/// Convert rmcp Tool to our DiscoveredTool.
fn tool_to_discovered(tool: &Tool) -> DiscoveredTool {
    DiscoveredTool {
        name: tool.name.to_string(),
        description: tool.description.as_ref().map(|d| d.to_string()),
        input_schema: serde_json::to_value(&tool.input_schema).ok(),
    }
}

/// Convert serde_json::Value to serde_json::Map for rmcp arguments.
fn value_to_map(v: Value) -> serde_json::Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => serde_json::Map::new(),
    }
}

/// Extract text content from an rmcp CallToolResult.
fn extract_call_result(result: &CallToolResult) -> (String, bool) {
    let texts: Vec<String> = result
        .content
        .iter()
        .filter_map(|c| c.as_text().map(|t| t.text.clone()))
        .collect();
    let content = if texts.is_empty() {
        serde_json::to_string_pretty(&result.content).unwrap_or_else(|_| "null".into())
    } else {
        texts.join("\n")
    };
    (content, result.is_error.unwrap_or(false))
}

// ---------------------------------------------------------------------------
// Stdio transport
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// MCP Stdio Connection Pool
// ---------------------------------------------------------------------------

/// Key for identifying a stdio MCP server configuration.
#[cfg(not(target_os = "android"))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StdioServerKey {
    pub command: String,
    pub args_json: String,
    pub env_json: String,
    pub server_id: Option<String>,
}

#[cfg(not(target_os = "android"))]
impl StdioServerKey {
    pub fn new(command: &str, args: &[String], env: &HashMap<String, String>) -> Self {
        Self {
            command: command.to_string(),
            args_json: serde_json::to_string(args).unwrap_or_default(),
            env_json: serde_json::to_string(env).unwrap_or_default(),
            server_id: None,
        }
    }

    #[must_use]
    pub fn with_server_id(mut self, server_id: impl Into<String>) -> Self {
        self.server_id = Some(server_id.into());
        self
    }
}

/// A cached MCP stdio client connection with its last-use timestamp.
#[cfg(not(target_os = "android"))]
struct PooledConnection {
    peer: McpPeer,
    cancel_token: rmcp::service::RunningServiceCancellationToken,
    last_used: std::time::Instant,
}

/// Connection pool for MCP stdio servers.
///
/// Instead of spawning a new child process for every tool call,
/// the pool keeps existing connections alive and reuses them.
/// Connections that have been idle longer than `idle_timeout` are
/// automatically evicted on the next `get_or_connect` call.
///
/// This eliminates the overhead of process spawn + MCP handshake
/// for repeated calls to the same server, which is the common
/// pattern in Agent mode (multiple tool calls per turn).
#[cfg(not(target_os = "android"))]
pub struct McpConnectionPool {
    connections: Mutex<HashMap<StdioServerKey, PooledConnection>>,
    idle_timeout: std::time::Duration,
}

#[cfg(not(target_os = "android"))]
impl McpConnectionPool {
    /// Create a new connection pool with the given idle timeout.
    pub fn new(idle_timeout: std::time::Duration) -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
            idle_timeout,
        }
    }

    /// Get an existing connection or create a new one for the given server config.
    /// Stale connections (idle > idle_timeout) are evicted before returning.
    pub async fn get_or_connect(&self, key: &StdioServerKey) -> Result<McpPeer> {
        let mut conns = self.connections.lock().await;

        // Evict stale entries
        let timeout = self.idle_timeout;
        conns.retain(|_, v| v.last_used.elapsed() < timeout);

        if let Some(pooled) = conns.get(key) {
            // Check if the underlying process is still alive by trying a ping.
            // If the client has been cancelled or the process died, we need to reconnect.
            // Since rmcp doesn't expose a simple ping, we check by attempting
            // list_tools (lightweight). If it fails, evict and reconnect.
            info!("[McpPool] Reusing cached connection for '{}'", key.command);
            return Ok(pooled.peer.clone());
        }

        // No cached connection — spawn a new one
        info!("[McpPool] No cached connection for '{}', spawning new process", key.command);
        let args: Vec<String> = serde_json::from_str(&key.args_json).unwrap_or_default();
        let env: HashMap<String, String> = serde_json::from_str(&key.env_json).unwrap_or_default();

        let (peer, cancel_token) = spawn_stdio_client(&key.command, &args, &env).await?;

        conns.insert(
            key.clone(),
            PooledConnection {
                peer: peer.clone(),
                cancel_token,
                last_used: std::time::Instant::now(),
            },
        );

        Ok(peer)
    }

    /// Mark a connection as recently used (call after successful tool invocation).
    pub async fn touch(&self, key: &StdioServerKey) {
        let mut conns = self.connections.lock().await;
        if let Some(pooled) = conns.get_mut(key) {
            pooled.last_used = std::time::Instant::now();
        }
    }

    /// Evict a specific connection (e.g. after a fatal error).
    pub async fn evict(&self, key: &StdioServerKey) {
        let mut conns = self.connections.lock().await;
        if let Some(pooled) = conns.remove(key) {
            pooled.cancel_token.cancel();
        }
    }

    /// Shut down all cached connections.
    pub async fn shutdown_all(&self) {
        let mut conns = self.connections.lock().await;
        for (_, pooled) in conns.drain() {
            pooled.cancel_token.cancel();
        }
    }

    /// Evict cached connections for a specific server_id.
    /// Used by hot-reload to force reconnection after server config changes.
    /// Only evicts connections whose `server_id` field matches exactly.
    pub fn evict_by_server_id(&self, server_id: &str) {
        if let Ok(mut conns) = self.connections.try_lock() {
            let matching_keys: Vec<StdioServerKey> = conns
                .keys()
                .filter(|key| key.server_id.as_deref() == Some(server_id))
                .cloned()
                .collect();
            for key in matching_keys {
                if let Some(pooled) = conns.remove(&key) {
                    pooled.cancel_token.cancel();
                }
            }
            let remaining = conns.len();
            info!(
                "[McpPool] Precisely evicted connections for hot-reload of server '{server_id}', {remaining} remaining in pool"
            );
        }
    }

    /// Return the number of currently cached connections.
    pub async fn len(&self) -> usize {
        self.connections.lock().await.len()
    }

    /// Return true if there are no cached connections.
    pub async fn is_empty(&self) -> bool {
        self.connections.lock().await.is_empty()
    }
}

/// Spawn a new stdio MCP client (child process + handshake).
/// Returns the peer for making calls and a cancellation token for shutdown.
#[cfg(not(target_os = "android"))]
async fn spawn_stdio_client(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<(McpPeer, rmcp::service::RunningServiceCancellationToken)> {
    let cmd = build_stdio_command(command, args, env);
    let transport = TokioChildProcess::new(cmd).map_err(|e| {
        AxAgentError::Gateway(format!("Failed to spawn MCP server '{}': {}", command, e))
    })?;

    let service = ()
        .serve(transport)
        .await
        .map_err(|e| {
            let err_str = e.to_string();
            // Provide more helpful error messages for common handshake failures
            let hint = if err_str.contains("connection closed") || err_str.contains("UnexpectedEof") {
                format!(
                    "{}\n\nThe MCP server process exited unexpectedly during initialization. \
                    Possible causes:\n\
                    - The command or package may not be installed (run `{} {}` manually to verify)\n\
                    - Node.js / Python / runtime may not be in PATH\n\
                    - The server package version may be incompatible\n\
                    - Check the server's stderr output for details",
                    err_str, command, args.join(" ")
                )
            } else {
                err_str
            };
            AxAgentError::Gateway(format!("MCP handshake failed: {}", hint))
        })?;

    let peer = service.peer().clone();
    let cancel_token = service.cancellation_token();
    Ok((peer, cancel_token))
}

/// Global MCP connection pool (lazy-initialized).
#[cfg(not(target_os = "android"))]
static MCP_POOL: OnceLock<Arc<McpConnectionPool>> = OnceLock::new();

/// Get the global MCP connection pool.
/// Idle timeout is 5 minutes — connections not used for 5 min are evicted.
#[cfg(not(target_os = "android"))]
pub fn global_mcp_pool() -> Arc<McpConnectionPool> {
    MCP_POOL
        .get_or_init(|| Arc::new(McpConnectionPool::new(std::time::Duration::from_secs(300))))
        .clone()
}

/// Execute a tool call against an MCP server via stdio transport,
/// using the connection pool to reuse existing connections.
#[cfg(not(target_os = "android"))]
pub async fn call_tool_stdio_pooled(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    tool_name: &str,
    tool_arguments: Value,
) -> Result<McpToolResult> {
    let pool = global_mcp_pool();
    let key = StdioServerKey::new(command, args, env);

    let client = pool.get_or_connect(&key).await?;

    let params = CallToolRequestParams::new(tool_name.to_string())
        .with_arguments(value_to_map(tool_arguments));

    match client.call_tool(params).await {
        Ok(result) => {
            pool.touch(&key).await;
            let (content, is_error) = extract_call_result(&result);
            Ok(McpToolResult {
                content,
                is_error,
                progress: Vec::new(),
            })
        },
        Err(e) => {
            let err_str = e.to_string();
            // If the call failed with a transport/connection error, evict the
            // cached connection so the next call will spawn a fresh process.
            let err_lower = err_str.to_lowercase();
            if err_lower.contains("broken pipe")
                || err_lower.contains("connection reset")
                || err_lower.contains("eof")
                || err_lower.contains("closed")
                || err_lower.contains("transport")
            {
                info!("[McpPool] Evicting stale connection for '{}' due to: {}", command, err_str);
                pool.evict(&key).await;
            }
            Err(AxAgentError::Gateway(format!("MCP tool call failed: {}", err_str)))
        },
    }
}

/// Execute a tool call against an MCP server via stdio transport.
/// (Legacy non-pooled version — kept for backward compatibility and tests)
#[cfg(not(target_os = "android"))]
pub async fn call_tool_stdio(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    tool_name: &str,
    tool_arguments: Value,
) -> Result<McpToolResult> {
    let cmd = build_stdio_command(command, args, env);
    let transport = TokioChildProcess::new(cmd).map_err(|e| {
        AxAgentError::Gateway(format!("Failed to spawn MCP server '{}': {}", command, e))
    })?;

    let client = ().serve(transport).await.map_err(|e| {
        let err_str = e.to_string();
        let hint = if err_str.contains("connection closed") || err_str.contains("UnexpectedEof") {
            format!(
                "{}\n\nThe MCP server process exited unexpectedly during initialization. \
                    Possible causes:\n\
                    - The command or package may not be installed\n\
                    - Node.js / Python / runtime may not be in PATH\n\
                    - The server package version may be incompatible",
                err_str
            )
        } else {
            err_str
        };
        AxAgentError::Gateway(format!("MCP handshake failed: {}", hint))
    })?;

    let params = CallToolRequestParams::new(tool_name.to_string())
        .with_arguments(value_to_map(tool_arguments));
    let result = client
        .call_tool(params)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP tool call failed: {}", e)))?;

    let _ = client.cancel().await;

    let (content, is_error) = extract_call_result(&result);
    Ok(McpToolResult {
        content,
        is_error,
        progress: Vec::new(),
    })
}

/// Discover tools from an MCP server via stdio transport.
#[cfg(not(target_os = "android"))]
pub async fn discover_tools_stdio(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<Vec<DiscoveredTool>> {
    let cmd = build_stdio_command(command, args, env);
    let transport = TokioChildProcess::new(cmd).map_err(|e| {
        AxAgentError::Gateway(format!("Failed to spawn MCP server '{}': {}", command, e))
    })?;

    let client = ()
        .serve(transport)
        .await
        .map_err(|e| {
            let err_str = e.to_string();
            let hint = if err_str.contains("connection closed") || err_str.contains("UnexpectedEof") {
                format!(
                    "{}\n\nThe MCP server process exited unexpectedly during initialization. \
                    Possible causes:\n\
                    - The command or package may not be installed (run `{} {}` manually to verify)\n\
                    - Node.js / Python / runtime may not be in PATH\n\
                    - The server package version may be incompatible",
                    err_str, command, args.join(" ")
                )
            } else {
                err_str
            };
            AxAgentError::Gateway(format!("MCP handshake failed: {}", hint))
        })?;

    let tools = client
        .list_all_tools()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP tools/list failed: {}", e)))?;

    let _ = client.cancel().await;

    Ok(tools.iter().map(tool_to_discovered).collect())
}

#[cfg(target_os = "android")]
pub async fn call_tool_stdio(
    _command: &str,
    _args: &[String],
    _env: &HashMap<String, String>,
    _tool_name: &str,
    _tool_arguments: Value,
) -> Result<McpToolResult> {
    Err(AxAgentError::Gateway("MCP stdio transport is not available on Android".into()))
}

#[cfg(target_os = "android")]
pub async fn call_tool_stdio_pooled(
    _command: &str,
    _args: &[String],
    _env: &HashMap<String, String>,
    _tool_name: &str,
    _tool_arguments: Value,
) -> Result<McpToolResult> {
    Err(AxAgentError::Gateway("MCP stdio transport is not available on Android".into()))
}

#[cfg(target_os = "android")]
pub async fn discover_tools_stdio(
    _command: &str,
    _args: &[String],
    _env: &HashMap<String, String>,
) -> Result<Vec<DiscoveredTool>> {
    Err(AxAgentError::Gateway("MCP stdio transport is not available on Android".into()))
}

// ---------------------------------------------------------------------------
// HTTP / SSE transport (Streamable HTTP — handles both)
// ---------------------------------------------------------------------------

/// Execute a tool call against an MCP server via HTTP/SSE transport.
pub async fn call_tool_http(
    endpoint: &str,
    tool_name: &str,
    tool_arguments: Value,
    auth_header: Option<&str>,
) -> Result<McpToolResult> {
    let mut transport_builder = StreamableHttpClientWorker::<reqwest::Client>::new_simple(endpoint);
    if let Some(auth) = auth_header {
        transport_builder = transport_builder.with_auth_header(auth.to_string());
    }
    let transport = transport_builder;

    let client = ()
        .serve(transport)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP HTTP connect failed: {}", e)))?;

    let params = CallToolRequestParams::new(tool_name.to_string())
        .with_arguments(value_to_map(tool_arguments));
    let result = client
        .call_tool(params)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP tool call failed: {}", e)))?;

    let _ = client.cancel().await;

    let (content, is_error) = extract_call_result(&result);
    Ok(McpToolResult {
        content,
        is_error,
        progress: Vec::new(),
    })
}

/// SSE transport uses the legacy MCP SSE protocol (GET /sse → endpoint → POST).
pub async fn call_tool_sse(
    endpoint: &str,
    tool_name: &str,
    tool_arguments: Value,
    auth_header: Option<&str>,
) -> Result<McpToolResult> {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": next_rpc_id(),
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": tool_arguments,
        }
    });
    let response = sse_send_request(endpoint, request, auth_header).await?;
    let result_obj = response.get("result").ok_or_else(|| {
        let err = response
            .get("error")
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown error".into());
        AxAgentError::Gateway(format!("MCP tool call error: {}", err))
    })?;
    let content_arr = result_obj.get("content").and_then(|c| c.as_array());
    let texts: Vec<String> = content_arr
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    if c.get("type").and_then(|t| t.as_str()) == Some("text") {
                        c.get("text").and_then(|t| t.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let content = if texts.is_empty() {
        serde_json::to_string_pretty(result_obj).unwrap_or_else(|_| "null".into())
    } else {
        texts.join("\n")
    };
    let is_error = result_obj
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    Ok(McpToolResult {
        content,
        is_error,
        progress: Vec::new(),
    })
}

/// Discover tools from an MCP server via HTTP transport.
pub async fn discover_tools_http(endpoint: &str) -> Result<Vec<DiscoveredTool>> {
    let transport = StreamableHttpClientWorker::<reqwest::Client>::new_simple(endpoint);

    let client = ()
        .serve(transport)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP HTTP connect failed: {}", e)))?;

    let tools = client
        .list_all_tools()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP tools/list failed: {}", e)))?;

    let _ = client.cancel().await;

    Ok(tools.iter().map(tool_to_discovered).collect())
}

/// Discover tools from an MCP server via legacy SSE protocol.
pub async fn discover_tools_sse(endpoint: &str) -> Result<Vec<DiscoveredTool>> {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": next_rpc_id(),
        "method": "tools/list",
        "params": {}
    });
    let response = sse_send_request(endpoint, request).await?;
    tracing::info!(
        "SSE tools/list response: {}",
        serde_json::to_string_pretty(&response).unwrap_or_default()
    );
    let result = response.get("result").ok_or_else(|| {
        let err_msg = response
            .get("error")
            .map(|e| format!("tools/list error: {}", e))
            .unwrap_or_else(|| format!("tools/list unexpected response: {}", response));
        AxAgentError::Gateway(err_msg)
    })?;
    let empty_tools = Vec::new();
    let tools = result
        .get("tools")
        .and_then(|t| t.as_array())
        .unwrap_or(&empty_tools);
    Ok(tools
        .iter()
        .filter_map(|t| {
            Some(DiscoveredTool {
                name: t.get("name")?.as_str()?.to_string(),
                description: t
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(String::from),
                input_schema: t.get("inputSchema").cloned(),
            })
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Unified MCP client entry points
// ---------------------------------------------------------------------------

/// Execute a tool call via the appropriate transport.
///
/// This is the unified entry point for all MCP tool calls — callers no longer
/// need to dispatch by transport type themselves.
///
/// When `server_id` is provided, OAuth credentials are automatically looked up
/// and injected for HTTP/SSE transports.
pub async fn call_tool_unified(
    transport: &str,
    command: Option<&str>,
    args: Option<&[String]>,
    env: Option<&HashMap<String, String>>,
    endpoint: Option<&str>,
    tool_name: &str,
    tool_arguments: Value,
) -> Result<McpToolResult> {
    call_tool_unified_with_opts(
        transport,
        command,
        args,
        env,
        endpoint,
        tool_name,
        tool_arguments,
        None,
        None,
    )
    .await
}

/// Extended unified entry point with optional `server_id` (OAuth) and progress callback.
pub async fn call_tool_unified_with_opts(
    transport: &str,
    command: Option<&str>,
    args: Option<&[String]>,
    env: Option<&HashMap<String, String>>,
    endpoint: Option<&str>,
    tool_name: &str,
    tool_arguments: Value,
    server_id: Option<&str>,
    on_progress: Option<ToolProgressCallback>,
) -> Result<McpToolResult> {
    let mut progress = Vec::new();
    let mut report = |phase: &str, msg: &str, pct: Option<u8>| {
        let p = McpToolProgress {
            phase: phase.to_string(),
            message: msg.to_string(),
            percent: pct,
        };
        if let Some(ref cb) = on_progress {
            cb(&p);
        }
        progress.push(p);
    };

    // OAuth: resolve credentials for HTTP/SSE servers
    let auth_header = server_id.and_then(|_sid| {
        std::env::var("MCP_OAUTH_TOKEN")
            .ok()
            .map(|token| format!("Bearer {token}"))
    });

    match transport {
        "stdio" => {
            #[cfg(target_os = "android")]
            {
                let _ = (command, args, env, tool_name, tool_arguments, on_progress);
                return Err(AxAgentError::Gateway(
                    "MCP stdio transport is not available on Android".into(),
                ));
            }
            #[cfg(not(target_os = "android"))]
            {
                report("connecting", "启动 MCP 进程...", Some(10));
                let command = command.ok_or_else(|| {
                    AxAgentError::Gateway("stdio transport requires command".into())
                })?;
                let args = args.unwrap_or(&[]);
                let env = env.cloned().unwrap_or_default();

                report("executing", &format!("执行工具: {tool_name}"), Some(50));
                let mut result =
                    call_tool_stdio_pooled(command, args, &env, tool_name, tool_arguments).await?;
                report("done", "完成", Some(100));
                result.progress = progress;
                Ok(result)
            }
        },
        "http" => {
            report("connecting", "连接 HTTP MCP 服务器...", Some(10));
            let endpoint = endpoint
                .ok_or_else(|| AxAgentError::Gateway("HTTP transport requires endpoint".into()))?;

            report("executing", &format!("执行工具: {tool_name}"), Some(50));
            let mut result =
                call_tool_http(endpoint, tool_name, tool_arguments, auth_header.as_deref()).await?;
            report("done", "完成", Some(100));
            result.progress = progress;
            Ok(result)
        },
        "sse" => {
            report("connecting", "连接 SSE MCP 服务器...", Some(10));
            let endpoint = endpoint
                .ok_or_else(|| AxAgentError::Gateway("SSE transport requires endpoint".into()))?;

            report("executing", &format!("执行工具: {tool_name}"), Some(50));
            let mut result =
                call_tool_sse(endpoint, tool_name, tool_arguments, auth_header.as_deref()).await?;
            report("done", "完成", Some(100));
            result.progress = progress;
            Ok(result)
        },
        other => Err(AxAgentError::Gateway(format!("不支持的 MCP 传输类型: {other}"))),
    }
}

/// Discover tools from an MCP server via the appropriate transport.
///
/// This is the unified entry point for all MCP tool discovery.
pub async fn discover_tools_unified(
    transport: &str,
    command: Option<&str>,
    args: Option<&[String]>,
    env: Option<&HashMap<String, String>>,
    endpoint: Option<&str>,
) -> Result<Vec<DiscoveredTool>> {
    match transport {
        "stdio" => {
            #[cfg(target_os = "android")]
            {
                let _ = (command, args, env);
                return Err(AxAgentError::Gateway(
                    "MCP stdio transport is not available on Android".into(),
                ));
            }
            #[cfg(not(target_os = "android"))]
            {
                let command = command.ok_or_else(|| {
                    AxAgentError::Gateway("stdio transport requires command".into())
                })?;
                let args = args.unwrap_or(&[]);
                let env = env.cloned().unwrap_or_default();
                discover_tools_stdio(command, args, &env).await
            }
        },
        "http" => {
            let endpoint = endpoint
                .ok_or_else(|| AxAgentError::Gateway("HTTP transport requires endpoint".into()))?;
            discover_tools_http(endpoint).await
        },
        "sse" => {
            let endpoint = endpoint
                .ok_or_else(|| AxAgentError::Gateway("SSE transport requires endpoint".into()))?;
            discover_tools_sse(endpoint).await
        },
        other => Err(AxAgentError::Gateway(format!("不支持的 MCP 传输类型: {other}"))),
    }
}

// ---------------------------------------------------------------------------
// Legacy SSE protocol helpers
// ---------------------------------------------------------------------------

/// Perform a full legacy MCP SSE session: connect → initialize → send request → return response.
async fn sse_send_request(
    sse_url: &str,
    request: Value,
    auth_header: Option<&str>,
) -> Result<Value> {
    use futures::StreamExt;

    let client = reqwest::Client::builder()
        .http1_only()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AxAgentError::Gateway(format!("Failed to build SSE client: {}", e)))?;

    // 1. GET the SSE endpoint to open a persistent stream
    tracing::info!("SSE: connecting to {}", sse_url);
    let mut sse_req = client.get(sse_url).header("Accept", "text/event-stream");
    if let Some(auth) = auth_header {
        sse_req = sse_req.header("Authorization", auth);
    }
    let sse_resp = sse_req
        .send()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("SSE connect failed: {}", e)))?;

    if !sse_resp.status().is_success() {
        return Err(AxAgentError::Gateway(format!("SSE connect returned {}", sse_resp.status())));
    }
    tracing::info!("SSE: connected, status={}", sse_resp.status());

    let base_url = {
        let parsed = reqwest::Url::parse(sse_url)
            .map_err(|e| AxAgentError::Gateway(format!("Invalid SSE URL: {}", e)))?;
        format!("{}://{}", parsed.scheme(), parsed.authority())
    };

    let mut byte_stream = sse_resp.bytes_stream();
    let mut buffer = String::new();

    // 2. Read SSE events until we get the `endpoint` event
    let messages_url = loop {
        let chunk = byte_stream
            .next()
            .await
            .ok_or_else(|| AxAgentError::Gateway("SSE stream ended before endpoint event".into()))?
            .map_err(|e| AxAgentError::Gateway(format!("SSE read error: {}", e)))?;
        let text = String::from_utf8_lossy(&chunk)
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        buffer.push_str(&text);

        if let Some(url) = extract_sse_endpoint(&mut buffer, &base_url) {
            break url;
        }
    };
    tracing::info!("SSE: got messages endpoint: {}", messages_url);

    // 3. POST initialize handshake
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": next_rpc_id(),
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "AxAgent", "version": "1.0.0" }
        }
    });
    let init_resp = client
        .post(&messages_url)
        .json(&init_request)
        .send()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("SSE initialize POST failed: {}", e)))?;
    if !init_resp.status().is_success() {
        return Err(AxAgentError::Gateway(format!(
            "SSE initialize returned {}",
            init_resp.status()
        )));
    }
    tracing::info!("SSE: initialize POST accepted, status={}", init_resp.status());

    // Read init response from SSE stream
    let _init_result = sse_read_response(&mut byte_stream, &mut buffer).await?;
    tracing::info!("SSE: initialize handshake complete");

    // 4. POST initialized notification (no id — it's a notification)
    let _ = client
        .post(&messages_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .send()
        .await;

    // 5. POST the actual request
    let resp = client
        .post(&messages_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("SSE request POST failed: {}", e)))?;
    if !resp.status().is_success() {
        return Err(AxAgentError::Gateway(format!("SSE request returned {}", resp.status())));
    }
    tracing::info!("SSE: request POST accepted, reading response...");

    // 6. Read the response from SSE stream
    sse_read_response(&mut byte_stream, &mut buffer).await
}

/// Extract the messages endpoint URL from SSE buffer. Drains consumed events.
fn extract_sse_endpoint(buffer: &mut String, base_url: &str) -> Option<String> {
    let mut search_start = 0;
    loop {
        let remaining = &buffer[search_start..];
        let block_end = remaining.find("\n\n")?;
        let block = &remaining[..block_end];
        let abs_block_end = search_start + block_end + 2;

        let mut event_type = None;
        let mut data = None;
        for line in block.lines() {
            if let Some(val) = line.strip_prefix("event:") {
                event_type = Some(val.trim());
            } else if let Some(val) = line.strip_prefix("data:") {
                data = Some(val.trim());
            }
        }
        if event_type == Some("endpoint") {
            if let Some(path) = data {
                let url = if path.starts_with("http://") || path.starts_with("https://") {
                    path.to_string()
                } else {
                    format!("{}{}", base_url, path)
                };
                buffer.drain(..abs_block_end);
                return Some(url);
            }
        }
        search_start = abs_block_end;
    }
}

/// Read a JSON-RPC response from the SSE byte stream.
async fn sse_read_response<S, E>(stream: &mut S, buffer: &mut String) -> Result<Value>
where
    S: futures::Stream<Item = std::result::Result<E, reqwest::Error>> + Unpin,
    E: AsRef<[u8]>,
{
    use futures::StreamExt;

    let timeout = tokio::time::Duration::from_secs(30);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        if let Some(value) = extract_sse_json_response(buffer) {
            return Ok(value);
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, stream.next()).await {
            Err(_) => return Err(AxAgentError::Gateway("SSE response timed out".into())),
            Ok(None) => {
                return Err(AxAgentError::Gateway("SSE stream ended before response".into()))
            },
            Ok(Some(Err(e))) => {
                return Err(AxAgentError::Gateway(format!("SSE read error: {}", e)))
            },
            Ok(Some(Ok(chunk))) => {
                let text = String::from_utf8_lossy(chunk.as_ref())
                    .replace("\r\n", "\n")
                    .replace('\r', "\n");
                buffer.push_str(&text);
            },
        }
    }
}

/// Try to extract a JSON-RPC response from SSE event data in the buffer.
/// Removes consumed events from the buffer on success.
fn extract_sse_json_response(buffer: &mut String) -> Option<Value> {
    let mut search_start = 0;
    loop {
        let remaining = &buffer[search_start..];
        let block_end = remaining.find("\n\n");
        let block = if let Some(pos) = block_end {
            &remaining[..pos]
        } else {
            break None;
        };

        let abs_block_end = search_start + block_end.unwrap() + 2; // +2 for "\n\n"

        let mut event_type = None;
        let mut data_lines = Vec::new();
        for line in block.lines() {
            if let Some(val) = line.strip_prefix("event:") {
                event_type = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("data:") {
                data_lines.push(val.trim().to_string());
            }
        }

        // Accept "message" events or events with no explicit type that contain data
        let is_message = event_type.as_deref() == Some("message")
            || (event_type.is_none() && !data_lines.is_empty());

        if is_message {
            let data = data_lines.join("");
            if let Ok(value) = serde_json::from_str::<Value>(&data) {
                if value.get("jsonrpc").is_some() && value.get("id").is_some() {
                    // Remove everything up to and including this event
                    buffer.drain(..abs_block_end);
                    return Some(value);
                }
            }
        }

        search_start = abs_block_end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[cfg(not(target_os = "android"))]
    #[test]
    fn configure_stdio_env_applies_custom_variables() {
        let mut env = HashMap::new();
        env.insert("TAVILY_API_KEY".to_string(), "secret-key".to_string());
        env.insert("PATH".to_string(), "/custom/bin".to_string());

        let mut cmd = tokio::process::Command::new("python3");
        configure_stdio_env(&mut cmd, &env);

        let env_map: HashMap<String, Option<String>> = cmd
            .as_std()
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().to_string(),
                    value.map(|v| v.to_string_lossy().to_string()),
                )
            })
            .collect();

        assert_eq!(env_map.get("TAVILY_API_KEY"), Some(&Some("secret-key".to_string())));
        assert_eq!(env_map.get("PATH"), Some(&Some("/custom/bin".to_string())));
    }

    #[cfg(not(target_os = "android"))]
    #[tokio::test]
    async fn call_tool_stdio_does_not_hang_when_initialize_stdout_is_non_json_then_eof() {
        let args = vec!["npm notice".to_string()];

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            call_tool_stdio("echo", &args, &HashMap::new(), "fetch_url", serde_json::json!({})),
        )
        .await;

        assert!(result.is_ok(), "call_tool_stdio hung after non-JSON initialize output");

        let err = result.unwrap().unwrap_err().to_string();
        assert!(
            err.contains("MCP")
                || err.contains("handshake")
                || err.contains("spawn")
                || err.contains("EOF"),
            "unexpected error: {err}"
        );
    }
}
