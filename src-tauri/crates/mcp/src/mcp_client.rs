// SPDX-License-Identifier: AGPL-3.0-only

use axagent_harness::core_error::{AxAgentError, Result};
#[cfg(not(target_os = "android"))]
use rmcp::transport::TokioChildProcess;
use rmcp::{
    RoleClient, ServiceExt,
    model::{CallToolRequestParams, CallToolResult, Tool},
    transport::streamable_http_client::StreamableHttpClientWorker,
};
/// Type alias for a connected MCP client peer.
/// Using Peer<RoleClient> (which is Clone + Send + Sync) instead of the
/// ClientHandler trait allows storing connections in the pool and cloning
/// them for reuse across multiple tool calls.
#[cfg(not(target_os = "android"))]
type McpPeer = rmcp::service::Peer<RoleClient>;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
#[cfg(all(unix, not(target_os = "android")))]
use std::collections::HashSet;
use std::sync::Arc;
#[cfg(not(target_os = "android"))]
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(target_os = "android"))]
use tokio::sync::Mutex;
#[cfg(not(target_os = "android"))]
use tracing::info;

use crate::mcp_oauth::McpOAuthStore;

/// 解析 MCP 服务器的 OAuth Authorization 头。
///
/// 优先级：持久化的服务器凭据（`McpOAuthStore`）→ 环境变量 `MCP_OAUTH_TOKEN`
/// （兼容手动注入场景）。无凭据时返回 `None`，调用方按未认证方式发起请求。
pub async fn resolve_oauth_header(server_id: Option<&str>) -> Option<String> {
    if let Some(sid) = server_id
        && let Some(store) = McpOAuthStore::try_global()
        && let Some(h) = store.get_auth_header(sid).await
    {
        return Some(h);
    }
    std::env::var("MCP_OAUTH_TOKEN").ok().map(|token| format!("Bearer {token}"))
}

static SSE_JSON_RPC_ID: AtomicU64 = AtomicU64::new(1);

/// 客户端声明支持的 MCP 协议版本（随 SDK 升级调整）。
const MCP_SSE_PROTOCOL_VERSION: &str = "2024-11-05";

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
pub use axagent_harness::mcp_types::DiscoveredTool;

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
        .args(["-i", "-l", "-c", &format!("printf '{}'; printenv PATH; printf '{}'", START, END)])
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

    if path.is_empty() { None } else { Some(path) }
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
    path.split(':').filter(|segment| !segment.is_empty()).count()
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
    let deduped: Vec<&str> =
        combined.split(';').filter(|s| !s.is_empty() && seen.insert(s.to_lowercase())).collect();
    Some(deduped.join(";"))
}

#[cfg(all(not(unix), not(target_os = "android")))]
fn read_registry_path(key: &str) -> Option<String> {
    use std::process::Command;
    let mut scmd = Command::new("reg");
    scmd.args(["query", key, "/v", "Path"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        scmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = scmd.output().ok()?;
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
) -> Result<tokio::process::Command> {
    // SECURITY (M7): command 与 args 必须经过白名单校验。
    // 阻断路径遍历与形如 `--script /etc/passwd` 的危险 flag。
    validate_mcp_command(command, args)?;
    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args);
    configure_stdio_env(&mut cmd, env);
    Ok(cmd)
}

#[cfg(all(not(target_os = "windows"), not(target_os = "android")))]
fn build_stdio_command(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<tokio::process::Command> {
    validate_mcp_command(command, args)?;
    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args);
    configure_stdio_env(&mut cmd, env);
    Ok(cmd)
}

/// SECURITY (M7): 校验 MCP server 启动命令。
/// - 阻断可执行路径中的 `..` / NUL
/// - 阻断已知的危险长 flag（`--script`、`--eval`、`--allow-run`、`--danger`、`-rf`）
/// - 不阻断 `-c` / `-e`：避免误伤 `bash -c`、`node -e` 等合法调用（M1）
/// - args 中路径不能含 NUL
#[cfg(not(target_os = "android"))]
fn validate_mcp_command(command: &str, args: &[String]) -> Result<()> {
    if command.is_empty() || command.contains('\0') || command.contains("..") {
        return Err(AxAgentError::Gateway(format!("MCP command path invalid: '{}'", command)));
    }

    // 阻断会让任意代码被加载的 flag。
    // 仅阻断显式的「执行/脚本」类长 flag，不阻断 `-c` / `-e` 等单字符短 flag，
    // 否则会误伤 `bash -c "..."`、`node -e "..."` 等常见合法调用（M1）。
    const DANGEROUS_FLAGS: &[&str] = &[
        "--script",
        "--eval",
        "-rf", // rm -rf 类组合通过 args 也无法触达
        "--allow-run",
        "--danger",
    ];
    for a in args {
        if a.contains('\0') {
            return Err(AxAgentError::Gateway("MCP arg contains NUL".to_string()));
        }
        // 形如 `-c` / `-e` 单独出现或与 value 一起
        for df in DANGEROUS_FLAGS {
            if a == *df || a.starts_with(&format!("{df}=")) {
                return Err(AxAgentError::Gateway(format!(
                    "MCP arg '{}' uses disallowed flag '{}'",
                    a, df
                )));
            }
        }
    }
    Ok(())
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

/// Extract content from an rmcp `CallToolResult`.
///
/// 纯文本结果按原样拼接（保持对 Agent 友好的纯文本形态）；
/// 若结果包含图片/资源等非文本块，则保留完整 JSON 表示，避免信息丢失（H2）。
fn extract_call_result(result: &CallToolResult) -> (String, bool) {
    let all_text = result.content.iter().all(|c| c.as_text().is_some());
    let content = if all_text {
        result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.clone()))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        serde_json::to_string_pretty(&result.content)
            .unwrap_or_else(|_| format!("{:?}", result.content))
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
    /// 池中允许的最大连接数；超过时淘汰最久未使用的连接
    max_connections: usize,
}

#[cfg(not(target_os = "android"))]
impl McpConnectionPool {
    /// Create a new connection pool with the given idle timeout.
    pub fn new(idle_timeout: std::time::Duration) -> Self {
        Self::with_config(32, idle_timeout)
    }

    /// Create a pool with explicit max_connections and idle_timeout.
    pub fn with_config(max_connections: usize, idle_timeout: std::time::Duration) -> Self {
        Self { connections: Mutex::new(HashMap::new()), idle_timeout, max_connections }
    }

    /// Get an existing connection or create a new one for the given server config.
    /// Stale connections (idle > idle_timeout) are evicted before returning.
    pub async fn get_or_connect(&self, key: &StdioServerKey) -> Result<McpPeer> {
        // 1. 驱逐过期连接（短锁）
        {
            let mut conns = self.connections.lock().await;
            let timeout = self.idle_timeout;
            conns.retain(|_, v| v.last_used.elapsed() < timeout);
        }

        // 2. 命中缓存？克隆 peer 后在锁外做存活探测（H4）
        let cached = {
            let conns = self.connections.lock().await;
            conns.get(key).map(|p| p.peer.clone())
        };
        if let Some(peer) = cached {
            // rmcp Peer 无简单 ping，以 list_all_tools 作为存活探针（带超时）。
            let alive =
                tokio::time::timeout(std::time::Duration::from_secs(5), peer.list_all_tools())
                    .await
                    .map(|r| r.is_ok())
                    .unwrap_or(false);
            if alive {
                info!("[McpPool] Reusing cached connection for '{}'", key.command);
                self.touch(key).await;
                return Ok(peer);
            }
            info!(
                "[McpPool] Cached connection for '{}' failed liveness probe, reconnecting",
                key.command
            );
            self.evict(key).await;
        }

        // 3. 无可用连接 — 新建
        info!("[McpPool] No cached connection for '{}', spawning new process", key.command);
        let args: Vec<String> = serde_json::from_str(&key.args_json).unwrap_or_default();
        let env: HashMap<String, String> = serde_json::from_str(&key.env_json).unwrap_or_default();

        let (peer, cancel_token) = spawn_stdio_client(&key.command, &args, &env).await?;

        // 容量保护：若池已满，淘汰最久未使用的连接
        {
            let mut conns = self.connections.lock().await;
            if conns.len() >= self.max_connections
                && let Some(oldest_key) =
                    conns.iter().min_by_key(|(_, v)| v.last_used).map(|(k, _)| k.clone())
                && let Some(pooled) = conns.remove(&oldest_key)
            {
                pooled.cancel_token.cancel();
                info!(
                    "[McpPool] Evicted oldest connection '{}' to make room (max={})",
                    oldest_key.command, self.max_connections
                );
            }
            conns.insert(
                key.clone(),
                PooledConnection {
                    peer: peer.clone(),
                    cancel_token,
                    last_used: std::time::Instant::now(),
                },
            );
        }

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

    /// 探测所有缓存连接的存活状态，驱逐已死亡的连接。
    ///
    /// 对每个连接执行一次轻量级 `list_all_tools` 往返（带超时），失败即视为死亡并取消。
    /// 返回每个 server 键的存活结果，供健康检查上报。短锁设计：仅在取 peer 时短暂持锁，
    /// 网络往返在锁外进行，避免异步锁跨 await 长时间占用。
    pub async fn probe_and_evict(&self) -> Vec<(StdioServerKey, bool)> {
        let keys: Vec<StdioServerKey> = {
            let conns = self.connections.lock().await;
            conns.keys().cloned().collect()
        };
        let mut results = Vec::with_capacity(keys.len());
        for key in &keys {
            let peer = {
                let conns = self.connections.lock().await;
                conns.get(key).map(|p| p.peer.clone())
            };
            let alive = match peer {
                Some(p) => {
                    tokio::time::timeout(std::time::Duration::from_secs(5), p.list_all_tools())
                        .await
                        .map(|r| r.is_ok())
                        .unwrap_or(false)
                },
                None => false,
            };
            results.push((key.clone(), alive));
            if !alive {
                self.evict(key).await;
            }
        }
        results
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
    let cmd = build_stdio_command(command, args, env)?;
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
        .get_or_init(|| {
            let max_connections = std::env::var("MCP_POOL_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(32);
            let idle_secs = std::env::var("MCP_POOL_IDLE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300u64);
            Arc::new(McpConnectionPool::with_config(
                max_connections,
                std::time::Duration::from_secs(idle_secs),
            ))
        })
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

    let mut progress_meta = serde_json::Map::new();
    progress_meta.insert("progressToken".to_string(), serde_json::json!(next_rpc_id()));
    let mut params = CallToolRequestParams::new(tool_name.to_string())
        .with_arguments(value_to_map(tool_arguments));
    params.meta = Some(rmcp::model::Meta(progress_meta));

    match client.call_tool(params).await {
        Ok(result) => {
            pool.touch(&key).await;
            let (content, is_error) = extract_call_result(&result);
            Ok(McpToolResult { content, is_error, progress: Vec::new() })
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
    let cmd = build_stdio_command(command, args, env)?;
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

    // Generate a unique progress token so the server can send progress notifications
    let mut progress_meta = serde_json::Map::new();
    progress_meta.insert("progressToken".to_string(), serde_json::json!(next_rpc_id()));
    let mut params = CallToolRequestParams::new(tool_name.to_string())
        .with_arguments(value_to_map(tool_arguments));
    params.meta = Some(rmcp::model::Meta(progress_meta));
    let result = client
        .call_tool(params)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP tool call failed: {}", e)))?;

    let _ = client.cancel().await;

    let (content, is_error) = extract_call_result(&result);
    Ok(McpToolResult { content, is_error, progress: Vec::new() })
}

/// Discover tools from an MCP server via stdio transport.
#[cfg(not(target_os = "android"))]
pub async fn discover_tools_stdio(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<Vec<DiscoveredTool>> {
    let cmd = build_stdio_command(command, args, env)?;
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
    let transport = {
        use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
        let mut config = StreamableHttpClientTransportConfig::with_uri(endpoint);
        if let Some(auth) = auth_header {
            config = config.auth_header(auth.to_string());
        }
        StreamableHttpClientWorker::<reqwest::Client>::new(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            config,
        )
    };

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
    Ok(McpToolResult { content, is_error, progress: Vec::new() })
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
        let err =
            response.get("error").map(|e| e.to_string()).unwrap_or_else(|| "unknown error".into());
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
    let is_error = result_obj.get("isError").and_then(|v| v.as_bool()).unwrap_or(false);
    Ok(McpToolResult { content, is_error, progress: Vec::new() })
}

/// Discover tools from an MCP server via HTTP transport.
pub async fn discover_tools_http(
    endpoint: &str,
    auth_header: Option<&str>,
) -> Result<Vec<DiscoveredTool>> {
    let transport = {
        use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
        let mut config = StreamableHttpClientTransportConfig::with_uri(endpoint);
        if let Some(auth) = auth_header {
            config = config.auth_header(auth.to_string());
        }
        StreamableHttpClientWorker::<reqwest::Client>::new(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            config,
        )
    };

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
///
/// 支持 `tools/list` 的分页（`nextCursor`）：循环拉取直到没有下一页。
pub async fn discover_tools_sse(
    endpoint: &str,
    auth_header: Option<&str>,
) -> Result<Vec<DiscoveredTool>> {
    let mut all_tools = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut params = serde_json::Map::new();
        if let Some(c) = &cursor {
            params.insert("cursor".to_string(), serde_json::Value::String(c.clone()));
        }
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": next_rpc_id(),
            "method": "tools/list",
            "params": params
        });
        let response = sse_send_request(endpoint, request, auth_header).await?;
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
        let tools = result.get("tools").and_then(|t| t.as_array()).unwrap_or(&empty_tools);
        for t in tools {
            if let Some(name) = t.get("name").and_then(|n| n.as_str()) {
                all_tools.push(DiscoveredTool {
                    name: name.to_string(),
                    description: t.get("description").and_then(|d| d.as_str()).map(String::from),
                    input_schema: t.get("inputSchema").cloned(),
                });
            }
        }

        cursor = result.get("nextCursor").and_then(|c| c.as_str()).map(String::from);
        if cursor.is_none() {
            break;
        }
    }

    Ok(all_tools)
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
#[allow(clippy::too_many_arguments)]
pub async fn call_tool_unified(
    transport: &str,
    command: Option<&str>,
    args: Option<&[String]>,
    env: Option<&HashMap<String, String>>,
    endpoint: Option<&str>,
    tool_name: &str,
    tool_arguments: Value,
    server_id: Option<&str>,
) -> Result<McpToolResult> {
    call_tool_unified_with_opts(
        transport,
        command,
        args,
        env,
        endpoint,
        tool_name,
        tool_arguments,
        server_id,
        None,
    )
    .await
}

/// Extended unified entry point with optional `server_id` (OAuth) and progress callback.
#[allow(clippy::too_many_arguments)]
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
        let p =
            McpToolProgress { phase: phase.to_string(), message: msg.to_string(), percent: pct };
        if let Some(ref cb) = on_progress {
            cb(&p);
        }
        progress.push(p);
    };

    // OAuth: resolve credentials for HTTP/SSE servers (persisted store → env fallback)
    let auth_header = resolve_oauth_header(server_id).await;

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
    server_id: Option<&str>,
) -> Result<Vec<DiscoveredTool>> {
    let auth_header = resolve_oauth_header(server_id).await;
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
            discover_tools_http(endpoint, auth_header.as_deref()).await
        },
        "sse" => {
            let endpoint = endpoint
                .ok_or_else(|| AxAgentError::Gateway("SSE transport requires endpoint".into()))?;
            discover_tools_sse(endpoint, auth_header.as_deref()).await
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
    sse_send_request_with_progress(sse_url, request, auth_header, None).await
}

/// Full version of sse_send_request with optional progress callback.
/// Progress notifications (`event: progress` with `data: {progress_token, progress, total, message}`)
/// are forwarded to the callback when provided.
async fn sse_send_request_with_progress(
    sse_url: &str,
    request: Value,
    auth_header: Option<&str>,
    on_progress: Option<&ToolProgressCallback>,
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

    let base_url = sse_url.to_string();

    let mut byte_stream = sse_resp.bytes_stream();
    let mut buffer = String::new();

    // 2. Read SSE events until we get the `endpoint` event
    let messages_url = loop {
        let chunk = byte_stream
            .next()
            .await
            .ok_or_else(|| AxAgentError::Gateway("SSE stream ended before endpoint event".into()))?
            .map_err(|e| AxAgentError::Gateway(format!("SSE read error: {}", e)))?;
        let text = String::from_utf8_lossy(&chunk).replace("\r\n", "\n").replace('\r', "\n");
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
            "protocolVersion": MCP_SSE_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "AxAgent", "version": env!("CARGO_PKG_VERSION") }
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
    let init_result = sse_read_response(&mut byte_stream, &mut buffer).await?;
    // 读取服务端协商的协议版本（M4）：仅用于观测/兼容，不参与后续请求
    if let Some(sv) =
        init_result.get("result").and_then(|r| r.get("protocolVersion")).and_then(|v| v.as_str())
    {
        tracing::info!("SSE: initialize 完成，服务端协议版本 = {}", sv);
    } else {
        tracing::info!("SSE: initialize handshake complete");
    }

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

    // 4b. H3: Drain any progress notifications sent before/during the request.
    //      The SSE stream may have buffered progress events (e.g. server sends
    //      progress before completing). Read non-blockingly: if data is available
    //      between reading chunks, collect progress events without blocking.
    if let Some(cb) = on_progress {
        // Non-blocking peek for progress events in the stream before posting the request
        let mut drained = 0u32;
        loop {
            // Try to peek at the stream — if nothing available in 200ms, move on
            let chunk_fut = byte_stream.next();
            match tokio::time::timeout(std::time::Duration::from_millis(200), chunk_fut).await {
                Ok(Some(Ok(chunk))) => {
                    let text = String::from_utf8_lossy(chunk.as_ref())
                        .replace("\r\n", "\n")
                        .replace("\r", "\n");
                    buffer.push_str(&text);
                    while let Some(prog) = extract_sse_progress(&mut buffer) {
                        cb(&McpToolProgress {
                            phase: "progress".to_string(),
                            message: prog.message.unwrap_or_default(),
                            percent: prog.total.map(|t| ((prog.progress / t) * 100.0) as u8),
                        });
                        drained += 1;
                    }
                },
                _ => break, // timeout or stream end — stop peeking
            }
        }
        if drained > 0 {
            tracing::info!("[SSE Progress] Drained {drained} pre-response progress events");
        }
    }

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
        if event_type == Some("endpoint")
            && let Some(path) = data
        {
            let url = if path.starts_with("http://") || path.starts_with("https://") {
                path.to_string()
            } else {
                // 以完整 SSE URL 为基址做标准 URL 合并，正确处理绝对路径（/foo）
                // 与相对路径（foo）两种情形，避免丢失 SSE URL 的 path 前缀。
                match reqwest::Url::parse(base_url).and_then(|b| b.join(path)) {
                    Ok(u) => u.to_string(),
                    Err(_) => format!("{}{}", base_url, path),
                }
            };
            buffer.drain(..abs_block_end);
            return Some(url);
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
                return Err(AxAgentError::Gateway("SSE stream ended before response".into()));
            },
            Ok(Some(Err(e))) => {
                return Err(AxAgentError::Gateway(format!("SSE read error: {}", e)));
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

        let abs_block_end = search_start
            + block_end.expect("block_end is Some: we would have broken out of the loop otherwise")
            + 2;

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
            if let Ok(value) = serde_json::from_str::<Value>(&data)
                && value.get("jsonrpc").is_some()
                && value.get("id").is_some()
            {
                // Remove everything up to and including this event
                buffer.drain(..abs_block_end);
                return Some(value);
            }
        }

        search_start = abs_block_end;
    }
}

// ---------------------------------------------------------------------------
// H1: Resources support (resources/list, resources/read)
// ---------------------------------------------------------------------------

/// Map an rmcp Resource to our harness McpResource.
fn resource_to_mcp(r: &rmcp::model::Resource) -> axagent_harness::mcp_types::McpResource {
    axagent_harness::mcp_types::McpResource {
        uri: r.uri.clone(),
        name: r.name.clone(),
        description: r.description.clone(),
        mime_type: r.mime_type.clone(),
    }
}

/// Map rmcp ResourceContents to our harness McpResourceContent.
fn resource_contents_to_mcp(
    contents: &[rmcp::model::ResourceContents],
) -> Vec<axagent_harness::mcp_types::McpResourceContent> {
    let mut out = Vec::with_capacity(contents.len());
    for c in contents {
        match c {
            rmcp::model::ResourceContents::TextResourceContents {
                uri,
                mime_type,
                text,
                meta: _,
            } => {
                out.push(axagent_harness::mcp_types::McpResourceContent {
                    uri: uri.clone(),
                    mime_type: mime_type.clone(),
                    text: Some(text.clone()),
                    blob: None,
                });
            },
            rmcp::model::ResourceContents::BlobResourceContents {
                uri,
                mime_type,
                blob,
                meta: _,
            } => {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(blob);
                out.push(axagent_harness::mcp_types::McpResourceContent {
                    uri: uri.clone(),
                    mime_type: mime_type.clone(),
                    text: None,
                    blob: Some(encoded),
                });
            },
            // 兜底：rmcp 2.x 将 ResourceContents 标记为 non_exhaustive，未来新增变体时不破坏调用方
            _ => {},
        }
    }
    out
}

/// Map an rmcp Prompt to our harness McpPrompt.
fn prompt_to_mcp(p: &rmcp::model::Prompt) -> axagent_harness::mcp_types::McpPrompt {
    let arguments = p
        .arguments
        .as_ref()
        .map(|args| {
            args.iter()
                .map(|a| axagent_harness::mcp_types::McpPromptArgument {
                    name: a.name.clone(),
                    description: a.description.clone(),
                    required: a.required.unwrap_or(false),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    axagent_harness::mcp_types::McpPrompt {
        name: p.name.clone(),
        description: p.description.clone(),
        arguments,
    }
}

/// List resources from an MCP server via HTTP (rmcp).
pub async fn list_resources_http(
    endpoint: &str,
    auth_header: Option<&str>,
) -> Result<Vec<axagent_harness::mcp_types::McpResource>> {
    let transport = {
        let mut config =
            rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(
                endpoint,
            );
        if let Some(auth) = auth_header {
            config = config.auth_header(auth.to_string());
        }
        rmcp::transport::streamable_http_client::StreamableHttpClientWorker::<reqwest::Client>::new(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            config,
        )
    };

    let client = ().serve(transport).await.map_err(|e| {
        AxAgentError::Gateway(format!("MCP HTTP list_resources connect failed: {}", e))
    })?;

    let resources = client
        .list_all_resources()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP resources/list failed: {}", e)))?;

    let _ = client.cancel().await;

    Ok(resources.iter().map(resource_to_mcp).collect())
}

/// List resources from an MCP server via legacy SSE protocol.
pub async fn list_resources_sse(
    endpoint: &str,
    auth_header: Option<&str>,
) -> Result<Vec<axagent_harness::mcp_types::McpResource>> {
    let mut all_resources = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut params = serde_json::Map::new();
        if let Some(c) = &cursor {
            params.insert("cursor".to_string(), serde_json::Value::String(c.clone()));
        }
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": next_rpc_id(),
            "method": "resources/list",
            "params": params
        });
        let response = sse_send_request(endpoint, request, auth_header).await?;
        let result = response.get("result").ok_or_else(|| {
            let err_msg = response
                .get("error")
                .map(|e| format!("resources/list error: {}", e))
                .unwrap_or_else(|| format!("resources/list unexpected response: {}", response));
            AxAgentError::Gateway(err_msg)
        })?;

        let empty_arr = Vec::new();
        let arr = result.get("resources").and_then(|t| t.as_array()).unwrap_or(&empty_arr);
        for item in arr {
            if let Some(uri) = item.get("uri").and_then(|u| u.as_str()) {
                all_resources.push(axagent_harness::mcp_types::McpResource {
                    uri: uri.to_string(),
                    name: item.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                    description: item.get("description").and_then(|d| d.as_str()).map(String::from),
                    mime_type: item.get("mimeType").and_then(|m| m.as_str()).map(String::from),
                });
            }
        }

        cursor = result.get("nextCursor").and_then(|c| c.as_str()).map(String::from);
        if cursor.is_none() {
            break;
        }
    }

    Ok(all_resources)
}

/// Read a specific resource from an MCP server via HTTP (rmcp).
pub async fn read_resource_http(
    endpoint: &str,
    uri: &str,
    auth_header: Option<&str>,
) -> Result<Vec<axagent_harness::mcp_types::McpResourceContent>> {
    let transport = {
        let mut config =
            rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(
                endpoint,
            );
        if let Some(auth) = auth_header {
            config = config.auth_header(auth.to_string());
        }
        rmcp::transport::streamable_http_client::StreamableHttpClientWorker::<reqwest::Client>::new(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            config,
        )
    };

    let client = ().serve(transport).await.map_err(|e| {
        AxAgentError::Gateway(format!("MCP HTTP read_resource connect failed: {}", e))
    })?;

    let params = rmcp::model::ReadResourceRequestParams::new(uri);
    let result = client
        .read_resource(params)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP resources/read failed: {}", e)))?;

    let _ = client.cancel().await;

    Ok(resource_contents_to_mcp(&result.contents))
}

/// Read a resource via legacy SSE protocol.
pub async fn read_resource_sse(
    endpoint: &str,
    uri: &str,
    auth_header: Option<&str>,
) -> Result<Vec<axagent_harness::mcp_types::McpResourceContent>> {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": next_rpc_id(),
        "method": "resources/read",
        "params": { "uri": uri }
    });
    let response = sse_send_request(endpoint, request, auth_header).await?;
    let result_obj = response.get("result").ok_or_else(|| {
        let err =
            response.get("error").map(|e| e.to_string()).unwrap_or_else(|| "unknown error".into());
        AxAgentError::Gateway(format!("MCP resources/read error: {}", err))
    })?;

    let empty_arr = Vec::new();
    let arr = result_obj.get("contents").and_then(|c| c.as_array()).unwrap_or(&empty_arr);
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let c_uri = item.get("uri").and_then(|u| u.as_str()).unwrap_or("").to_string();
        let mime = item.get("mimeType").and_then(|m| m.as_str()).map(String::from);
        let text = item.get("text").and_then(|t| t.as_str()).map(String::from);
        let blob = item.get("blob").and_then(|b| b.as_str()).map(String::from);
        out.push(axagent_harness::mcp_types::McpResourceContent {
            uri: c_uri,
            mime_type: mime,
            text,
            blob,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// H1: Prompts support (prompts/list, prompts/get)
// ---------------------------------------------------------------------------

/// List prompts from an MCP server via HTTP (rmcp).
pub async fn list_prompts_http(
    endpoint: &str,
    auth_header: Option<&str>,
) -> Result<Vec<axagent_harness::mcp_types::McpPrompt>> {
    let transport = {
        let mut config =
            rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(
                endpoint,
            );
        if let Some(auth) = auth_header {
            config = config.auth_header(auth.to_string());
        }
        rmcp::transport::streamable_http_client::StreamableHttpClientWorker::<reqwest::Client>::new(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            config,
        )
    };

    let client = ().serve(transport).await.map_err(|e| {
        AxAgentError::Gateway(format!("MCP HTTP list_prompts connect failed: {}", e))
    })?;

    let prompts = client
        .list_all_prompts()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP prompts/list failed: {}", e)))?;

    let _ = client.cancel().await;

    Ok(prompts.iter().map(prompt_to_mcp).collect())
}

/// List prompts via legacy SSE protocol.
pub async fn list_prompts_sse(
    endpoint: &str,
    auth_header: Option<&str>,
) -> Result<Vec<axagent_harness::mcp_types::McpPrompt>> {
    let mut all_prompts = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut params = serde_json::Map::new();
        if let Some(c) = &cursor {
            params.insert("cursor".to_string(), serde_json::Value::String(c.clone()));
        }
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": next_rpc_id(),
            "method": "prompts/list",
            "params": params
        });
        let response = sse_send_request(endpoint, request, auth_header).await?;
        let result = response.get("result").ok_or_else(|| {
            let err_msg = response
                .get("error")
                .map(|e| format!("prompts/list error: {}", e))
                .unwrap_or_else(|| format!("prompts/list unexpected response: {}", response));
            AxAgentError::Gateway(err_msg)
        })?;

        let empty_arr = Vec::new();
        let arr = result.get("prompts").and_then(|t| t.as_array()).unwrap_or(&empty_arr);
        for item in arr {
            if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                let args = item
                    .get("arguments")
                    .and_then(|a| a.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|arg| {
                                arg.get("name").and_then(|n| n.as_str()).map(|aname| {
                                    axagent_harness::mcp_types::McpPromptArgument {
                                        name: aname.to_string(),
                                        description: arg
                                            .get("description")
                                            .and_then(|d| d.as_str())
                                            .map(String::from),
                                        required: arg
                                            .get("required")
                                            .and_then(|r| r.as_bool())
                                            .unwrap_or(false),
                                    }
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                all_prompts.push(axagent_harness::mcp_types::McpPrompt {
                    name: name.to_string(),
                    description: item.get("description").and_then(|d| d.as_str()).map(String::from),
                    arguments: args,
                });
            }
        }

        cursor = result.get("nextCursor").and_then(|c| c.as_str()).map(String::from);
        if cursor.is_none() {
            break;
        }
    }

    Ok(all_prompts)
}

/// Get a prompt from an MCP server via HTTP (rmcp).
pub async fn get_prompt_http(
    endpoint: &str,
    name: &str,
    args: serde_json::Value,
    auth_header: Option<&str>,
) -> Result<axagent_harness::mcp_types::McpPromptResult> {
    let transport = {
        let mut config =
            rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(
                endpoint,
            );
        if let Some(auth) = auth_header {
            config = config.auth_header(auth.to_string());
        }
        rmcp::transport::streamable_http_client::StreamableHttpClientWorker::<reqwest::Client>::new(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            config,
        )
    };

    let client = ()
        .serve(transport)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP HTTP get_prompt connect failed: {}", e)))?;

    let args_map = if args.is_object() {
        let map = args.as_object().expect("MCP 客户端：is_object 检查后应为 object");
        Some(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<serde_json::Map<_, _>>())
    } else {
        None
    };
    let params = rmcp::model::GetPromptRequestParams::new(name);
    let params = if let Some(m) = args_map {
        params.with_arguments(m)
    } else {
        params
    };
    let result = client
        .get_prompt(params)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP prompts/get failed: {}", e)))?;

    let _ = client.cancel().await;

    // Serialize messages as JSON value for flexibility
    let messages_val = serde_json::to_value(&result.messages).unwrap_or(serde_json::Value::Null);
    Ok(axagent_harness::mcp_types::McpPromptResult {
        description: result.description,
        messages: messages_val,
    })
}

/// Get a prompt via legacy SSE protocol.
pub async fn get_prompt_sse(
    endpoint: &str,
    name: &str,
    args: serde_json::Value,
    auth_header: Option<&str>,
) -> Result<axagent_harness::mcp_types::McpPromptResult> {
    let params_obj = if args.is_object() {
        args.as_object().cloned()
    } else {
        None
    };
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": next_rpc_id(),
        "method": "prompts/get",
        "params": {
            "name": name,
            "arguments": params_obj,
        }
    });
    let response = sse_send_request(endpoint, request, auth_header).await?;
    let result_obj = response.get("result").ok_or_else(|| {
        let err =
            response.get("error").map(|e| e.to_string()).unwrap_or_else(|| "unknown error".into());
        AxAgentError::Gateway(format!("MCP prompts/get error: {}", err))
    })?;

    let description = result_obj.get("description").and_then(|d| d.as_str()).map(String::from);
    let messages = result_obj.get("messages").cloned().unwrap_or(serde_json::Value::Null);
    Ok(axagent_harness::mcp_types::McpPromptResult { description, messages })
}

// ---------------------------------------------------------------------------
// H1: Stdio transport helpers for resources/prompts (via connection pool)
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "android"))]
async fn stdio_list_resources_pooled(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<Vec<axagent_harness::mcp_types::McpResource>> {
    let pool = global_mcp_pool();
    let key = StdioServerKey::new(command, args, env);
    let client = pool.get_or_connect(&key).await?;
    let resources = client
        .list_all_resources()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP stdio resources/list failed: {}", e)))?;
    pool.touch(&key).await;
    Ok(resources.iter().map(resource_to_mcp).collect())
}

#[cfg(not(target_os = "android"))]
async fn stdio_read_resource_pooled(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    uri: &str,
) -> Result<Vec<axagent_harness::mcp_types::McpResourceContent>> {
    let pool = global_mcp_pool();
    let key = StdioServerKey::new(command, args, env);
    let client = pool.get_or_connect(&key).await?;
    let params = rmcp::model::ReadResourceRequestParams::new(uri);
    let result = client
        .read_resource(params)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP stdio resources/read failed: {}", e)))?;
    pool.touch(&key).await;
    Ok(resource_contents_to_mcp(&result.contents))
}

#[cfg(not(target_os = "android"))]
async fn stdio_list_prompts_pooled(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<Vec<axagent_harness::mcp_types::McpPrompt>> {
    let pool = global_mcp_pool();
    let key = StdioServerKey::new(command, args, env);
    let client = pool.get_or_connect(&key).await?;
    let prompts = client
        .list_all_prompts()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP stdio prompts/list failed: {}", e)))?;
    pool.touch(&key).await;
    Ok(prompts.iter().map(prompt_to_mcp).collect())
}

#[cfg(not(target_os = "android"))]
async fn stdio_get_prompt_pooled(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    name: &str,
    args_val: serde_json::Value,
) -> Result<axagent_harness::mcp_types::McpPromptResult> {
    let pool = global_mcp_pool();
    let key = StdioServerKey::new(command, args, env);
    let client = pool.get_or_connect(&key).await?;
    let args_map = if args_val.is_object() {
        args_val.as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    } else {
        None
    };
    let params = rmcp::model::GetPromptRequestParams::new(name);
    let params = if let Some(m) = args_map {
        params.with_arguments(m)
    } else {
        params
    };
    let result = client
        .get_prompt(params)
        .await
        .map_err(|e| AxAgentError::Gateway(format!("MCP stdio prompts/get failed: {}", e)))?;
    pool.touch(&key).await;
    let messages_val = serde_json::to_value(&result.messages).unwrap_or(serde_json::Value::Null);
    Ok(axagent_harness::mcp_types::McpPromptResult {
        description: result.description,
        messages: messages_val,
    })
}

// ---------------------------------------------------------------------------
// H1: Unified entry points for resources/prompts
// ---------------------------------------------------------------------------

/// List resources via the appropriate transport.
#[allow(clippy::too_many_arguments)]
pub async fn list_resources_unified(
    transport: &str,
    command: Option<&str>,
    args: Option<&[String]>,
    env: Option<&HashMap<String, String>>,
    endpoint: Option<&str>,
    server_id: Option<&str>,
) -> Result<Vec<axagent_harness::mcp_types::McpResource>> {
    let auth_header = resolve_oauth_header(server_id).await;
    match transport {
        "stdio" => {
            #[cfg(target_os = "android")]
            {
                let _ = (command, args, env);
                return Err(AxAgentError::Gateway("MCP stdio not available on Android".into()));
            }
            #[cfg(not(target_os = "android"))]
            {
                let cmd = command.ok_or_else(|| {
                    AxAgentError::Gateway("stdio transport requires command".into())
                })?;
                let a = args.unwrap_or(&[]);
                let e = env.cloned().unwrap_or_default();
                stdio_list_resources_pooled(cmd, a, &e).await
            }
        },
        "http" => {
            let ep = endpoint
                .ok_or_else(|| AxAgentError::Gateway("HTTP transport requires endpoint".into()))?;
            list_resources_http(ep, auth_header.as_deref()).await
        },
        "sse" => {
            let ep = endpoint
                .ok_or_else(|| AxAgentError::Gateway("SSE transport requires endpoint".into()))?;
            list_resources_sse(ep, auth_header.as_deref()).await
        },
        other => Err(AxAgentError::Gateway(format!("unsupported transport: {other}"))),
    }
}

/// Read a resource via the appropriate transport.
#[allow(clippy::too_many_arguments)]
pub async fn read_resource_unified(
    transport: &str,
    command: Option<&str>,
    args: Option<&[String]>,
    env: Option<&HashMap<String, String>>,
    endpoint: Option<&str>,
    resource_uri: &str,
    server_id: Option<&str>,
) -> Result<Vec<axagent_harness::mcp_types::McpResourceContent>> {
    let auth_header = resolve_oauth_header(server_id).await;
    match transport {
        "stdio" => {
            #[cfg(target_os = "android")]
            {
                let _ = (command, args, env, resource_uri);
                return Err(AxAgentError::Gateway("MCP stdio not available on Android".into()));
            }
            #[cfg(not(target_os = "android"))]
            {
                let cmd = command.ok_or_else(|| {
                    AxAgentError::Gateway("stdio transport requires command".into())
                })?;
                let a = args.unwrap_or(&[]);
                let e = env.cloned().unwrap_or_default();
                stdio_read_resource_pooled(cmd, a, &e, resource_uri).await
            }
        },
        "http" => {
            let ep = endpoint
                .ok_or_else(|| AxAgentError::Gateway("HTTP transport requires endpoint".into()))?;
            read_resource_http(ep, resource_uri, auth_header.as_deref()).await
        },
        "sse" => {
            let ep = endpoint
                .ok_or_else(|| AxAgentError::Gateway("SSE transport requires endpoint".into()))?;
            read_resource_sse(ep, resource_uri, auth_header.as_deref()).await
        },
        other => Err(AxAgentError::Gateway(format!("unsupported transport: {other}"))),
    }
}

/// List prompts via the appropriate transport.
#[allow(clippy::too_many_arguments)]
pub async fn list_prompts_unified(
    transport: &str,
    command: Option<&str>,
    args: Option<&[String]>,
    env: Option<&HashMap<String, String>>,
    endpoint: Option<&str>,
    server_id: Option<&str>,
) -> Result<Vec<axagent_harness::mcp_types::McpPrompt>> {
    let auth_header = resolve_oauth_header(server_id).await;
    match transport {
        "stdio" => {
            #[cfg(target_os = "android")]
            {
                let _ = (command, args, env);
                return Err(AxAgentError::Gateway("MCP stdio not available on Android".into()));
            }
            #[cfg(not(target_os = "android"))]
            {
                let cmd = command.ok_or_else(|| {
                    AxAgentError::Gateway("stdio transport requires command".into())
                })?;
                let a = args.unwrap_or(&[]);
                let e = env.cloned().unwrap_or_default();
                stdio_list_prompts_pooled(cmd, a, &e).await
            }
        },
        "http" => {
            let ep = endpoint
                .ok_or_else(|| AxAgentError::Gateway("HTTP transport requires endpoint".into()))?;
            list_prompts_http(ep, auth_header.as_deref()).await
        },
        "sse" => {
            let ep = endpoint
                .ok_or_else(|| AxAgentError::Gateway("SSE transport requires endpoint".into()))?;
            list_prompts_sse(ep, auth_header.as_deref()).await
        },
        other => Err(AxAgentError::Gateway(format!("unsupported transport: {other}"))),
    }
}

/// Get a prompt via the appropriate transport.
#[allow(clippy::too_many_arguments)]
pub async fn get_prompt_unified(
    transport: &str,
    command: Option<&str>,
    args: Option<&[String]>,
    env: Option<&HashMap<String, String>>,
    endpoint: Option<&str>,
    prompt_name: &str,
    prompt_args: serde_json::Value,
    server_id: Option<&str>,
) -> Result<axagent_harness::mcp_types::McpPromptResult> {
    let auth_header = resolve_oauth_header(server_id).await;
    match transport {
        "stdio" => {
            #[cfg(target_os = "android")]
            {
                let _ = (command, args, env, prompt_name, prompt_args);
                return Err(AxAgentError::Gateway("MCP stdio not available on Android".into()));
            }
            #[cfg(not(target_os = "android"))]
            {
                let cmd = command.ok_or_else(|| {
                    AxAgentError::Gateway("stdio transport requires command".into())
                })?;
                let a = args.unwrap_or(&[]);
                let e = env.cloned().unwrap_or_default();
                stdio_get_prompt_pooled(cmd, a, &e, prompt_name, prompt_args).await
            }
        },
        "http" => {
            let ep = endpoint
                .ok_or_else(|| AxAgentError::Gateway("HTTP transport requires endpoint".into()))?;
            get_prompt_http(ep, prompt_name, prompt_args, auth_header.as_deref()).await
        },
        "sse" => {
            let ep = endpoint
                .ok_or_else(|| AxAgentError::Gateway("SSE transport requires endpoint".into()))?;
            get_prompt_sse(ep, prompt_name, prompt_args, auth_header.as_deref()).await
        },
        other => Err(AxAgentError::Gateway(format!("unsupported transport: {other}"))),
    }
}

// ---------------------------------------------------------------------------
// M2: HTTP / SSE session cache (connection pool for HTTP/SSE transports)
// ---------------------------------------------------------------------------

/// A cached MCP HTTP/SSE client connection with its last-use timestamp.
#[cfg(not(target_os = "android"))]
struct HttpPooledConnection {
    peer: McpPeer,
    cancel_token: rmcp::service::RunningServiceCancellationToken,
    last_used: std::time::Instant,
    endpoint: String,
}

/// Connection pool for MCP HTTP and SSE servers.
///
/// Unlike stdio (keyed by command+args), HTTP/SSE sessions are keyed
/// by endpoint URL (+ auth header for caching purposes). This caches
/// rmcp peers so that multiple tool/resource/prompt calls to the same
/// server reuse the same initialized session (critical for servers
/// that require session state).
#[cfg(not(target_os = "android"))]
pub struct HttpSessionPool {
    connections: tokio::sync::Mutex<HashMap<String, HttpPooledConnection>>,
    idle_timeout: std::time::Duration,
    max_connections: usize,
}

#[cfg(not(target_os = "android"))]
impl HttpSessionPool {
    pub fn new() -> Self {
        Self {
            connections: tokio::sync::Mutex::new(HashMap::new()),
            idle_timeout: std::time::Duration::from_secs(300),
            max_connections: 32,
        }
    }

    /// Get or create an HTTP session for the given endpoint + auth combo.
    pub async fn get_or_connect(
        &self,
        endpoint: &str,
        auth_header: Option<&str>,
    ) -> Result<McpPeer> {
        // Build cache key from endpoint + auth
        let key = format!("{}::auth={}", endpoint, auth_header.unwrap_or(""));

        // 1. Evict stale connections
        {
            let mut conns = self.connections.lock().await;
            let timeout = self.idle_timeout;
            conns.retain(|_, v| v.last_used.elapsed() < timeout);
        }

        // 2. Check cache with liveness probe
        let cached = {
            let conns = self.connections.lock().await;
            conns.get(&key).map(|p| (p.peer.clone(), p.endpoint.clone()))
        };
        if let Some((peer, _)) = cached {
            let alive =
                tokio::time::timeout(std::time::Duration::from_secs(3), peer.list_all_tools())
                    .await
                    .map(|r| r.is_ok())
                    .unwrap_or(false);
            if alive {
                // Mark as used and return
                let mut conns = self.connections.lock().await;
                if let Some(pooled) = conns.get_mut(&key) {
                    pooled.last_used = std::time::Instant::now();
                }
                return Ok(peer);
            }
            // Dead — evict
            let mut conns = self.connections.lock().await;
            conns.remove(&key);
        }

        // 3. Create new connection
        let transport = {
            let mut config = rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(endpoint);
            if let Some(auth) = auth_header {
                config = config.auth_header(auth.to_string());
            }
            rmcp::transport::streamable_http_client::StreamableHttpClientWorker::<reqwest::Client>::new(
                reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(60))
                    .connect_timeout(std::time::Duration::from_secs(10))
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new()),
                config,
            )
        };

        let service = ().serve(transport).await.map_err(|e| {
            AxAgentError::Gateway(format!("MCP HTTP session connect failed: {}", e))
        })?;
        let peer = service.peer().clone();
        let cancel_token = service.cancellation_token();

        // Capacity management
        {
            let mut conns = self.connections.lock().await;
            if conns.len() >= self.max_connections
                && let Some(oldest_key) =
                    conns.iter().min_by_key(|(_, v)| v.last_used).map(|(k, _)| k.clone())
                && let Some(pooled) = conns.remove(&oldest_key)
            {
                pooled.cancel_token.cancel();
            }
            conns.insert(
                key,
                HttpPooledConnection {
                    peer: peer.clone(),
                    cancel_token,
                    last_used: std::time::Instant::now(),
                    endpoint: endpoint.to_string(),
                },
            );
        }

        Ok(peer)
    }

    /// Evict a specific endpoint session.
    pub async fn evict(&self, endpoint: &str) {
        let mut conns = self.connections.lock().await;
        let keys: Vec<String> =
            conns.iter().filter(|(_, v)| v.endpoint == endpoint).map(|(k, _)| k.clone()).collect();
        for key in keys {
            if let Some(pooled) = conns.remove(&key) {
                pooled.cancel_token.cancel();
            }
        }
    }

    /// Shut down all sessions.
    pub async fn shutdown_all(&self) {
        let mut conns = self.connections.lock().await;
        for (_, pooled) in conns.drain() {
            pooled.cancel_token.cancel();
        }
    }

    pub async fn len(&self) -> usize {
        self.connections.lock().await.len()
    }

    /// 池中是否没有任何缓存连接。
    pub async fn is_empty(&self) -> bool {
        self.connections.lock().await.is_empty()
    }

    /// Probe all cached connections and return liveness results.
    pub async fn probe_and_evict(&self) -> Vec<(String, bool)> {
        let keys: Vec<(String, String)> = {
            let conns = self.connections.lock().await;
            conns.iter().map(|(k, v)| (k.clone(), v.endpoint.clone())).collect()
        };
        let mut results = Vec::with_capacity(keys.len());
        for (key, ep) in &keys {
            let alive = {
                let conns = self.connections.lock().await;
                conns.get(key).map(|p| p.peer.clone())
            };
            let is_alive = if let Some(peer) = alive {
                tokio::time::timeout(std::time::Duration::from_secs(5), peer.list_all_tools())
                    .await
                    .map(|r| r.is_ok())
                    .unwrap_or(false)
            } else {
                false
            };
            if !is_alive {
                let mut conns = self.connections.lock().await;
                if let Some(pooled) = conns.remove(key) {
                    pooled.cancel_token.cancel();
                }
            }
            results.push((ep.clone(), is_alive));
        }
        results
    }
}

#[cfg(not(target_os = "android"))]
impl Default for HttpSessionPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Global HTTP session pool.
#[cfg(not(target_os = "android"))]
static HTTP_POOL: OnceLock<Arc<HttpSessionPool>> = OnceLock::new();

#[cfg(not(target_os = "android"))]
pub fn global_http_pool() -> Arc<HttpSessionPool> {
    HTTP_POOL.get_or_init(|| Arc::new(HttpSessionPool::new())).clone()
}

/// Try to extract an MCP progress notification from SSE event data in the buffer.
fn extract_sse_progress(buffer: &mut String) -> Option<rmcp::model::ProgressNotificationParam> {
    let mut search_start = 0;
    loop {
        let remaining = &buffer[search_start..];
        let block_end = remaining.find("\n\n")?;
        let block = &remaining[..block_end];
        let abs_block_end = search_start + block_end + 2;

        let mut event_type = None;
        let mut data_lines = Vec::new();
        for line in block.lines() {
            if let Some(val) = line.strip_prefix("event:") {
                event_type = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("data:") {
                data_lines.push(val.trim().to_string());
            }
        }

        let is_progress = event_type.as_deref() == Some("progress")
            || (event_type.as_deref() == Some("message")
                && data_lines.iter().any(|d| d.contains("\"method\":\"notifications/progress\"")));

        if is_progress {
            let data = data_lines.join("");
            if let Ok(notification) =
                serde_json::from_str::<rmcp::model::ProgressNotificationParam>(&data)
            {
                buffer.drain(..abs_block_end);
                return Some(notification);
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
                (key.to_string_lossy().to_string(), value.map(|v| v.to_string_lossy().to_string()))
            })
            .collect();

        assert_eq!(env_map.get("TAVILY_API_KEY"), Some(&Some("secret-key".to_string())));
        assert_eq!(env_map.get("PATH"), Some(&Some("/custom/bin".to_string())));
    }

    #[cfg(not(target_os = "android"))]
    #[tokio::test]
    async fn call_tool_stdio_does_not_hang_when_initialize_stdout_is_non_json_then_eof() {
        // Windows 上没有独立的 echo.exe（echo 是 cmd 内置），且 MSYS echo.exe
        // 通过 tokio piped IO 运行时管道 EOF 信号时序不可靠，导致 rmcp handshake
        // 卡住等待更多数据。使用 cmd /c echo 确保管道正确关闭。
        let (cmd, args) = if cfg!(target_os = "windows") {
            (
                "cmd".to_string(),
                vec!["/c".to_string(), "echo".to_string(), "npm notice".to_string()],
            )
        } else {
            ("echo".to_string(), vec!["npm notice".to_string()])
        };

        // 预热登录 shell PATH 解析（OnceLock 仅首次耗时）：该过程会同步拉起多个
        // 交互式登录 shell（`-i -l`），在全量并行测试（数千用例）环境下可能拖慢数秒，
        // 把它挪到计时窗口之外，避免将启动开销误判为“挂起”。
        let _ = get_shell_path();

        // 全量并行测试下子进程 stdout 管道的 EOF 调度可能被显著延迟，
        // 放宽计时窗口避免 flaky，同时仍能捕获真正的无限挂起。
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            call_tool_stdio(&cmd, &args, &HashMap::new(), "fetch_url", serde_json::json!({})),
        )
        .await;

        assert!(result.is_ok(), "call_tool_stdio hung after non-JSON initialize output");

        let err = result.expect("测试应成功").unwrap_err().to_string();
        assert!(
            err.contains("MCP")
                || err.contains("handshake")
                || err.contains("spawn")
                || err.contains("EOF"),
            "unexpected error: {err}"
        );
    }
}
