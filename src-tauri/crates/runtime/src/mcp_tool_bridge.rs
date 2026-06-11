// SPDX-License-Identifier: AGPL-3.0-only

//! Bridge between MCP tool surface (ListMcpResources, ReadMcpResource, McpAuth, MCP)
//! and the existing McpServerManager runtime.
//!
//! Provides a stateful client registry that tool handlers can use to
//! connect to MCP servers and invoke their capabilities.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::mcp::mcp_tool_name;
use crate::mcp_stdio::McpServerManager;
use crate::util::lock_or_recover;
use serde::{Deserialize, Serialize};

/// Status of a managed MCP server connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    AuthRequired,
    Error,
}

impl std::fmt::Display for McpConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "disconnected"),
            Self::Connecting => write!(f, "connecting"),
            Self::Connected => write!(f, "connected"),
            Self::AuthRequired => write!(f, "auth_required"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Metadata about an MCP resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceInfo {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// Metadata about an MCP tool exposed by a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
}

/// Tracked state of an MCP server connection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpServerState {
    pub server_name: String,
    pub status: McpConnectionStatus,
    pub tools: Vec<McpToolInfo>,
    pub resources: Vec<McpResourceInfo>,
    pub server_info: Option<String>,
    pub error_message: Option<String>,
    /// Transport type: "stdio", "http", "sse"
    #[serde(default)]
    pub transport: Option<String>,
    /// Command for stdio transport
    #[serde(default)]
    pub command: Option<String>,
    /// JSON-serialized args for stdio transport
    #[serde(default)]
    pub args_json: Option<String>,
    /// JSON-serialized env for stdio transport
    #[serde(default)]
    pub env_json: Option<String>,
    /// Endpoint URL for HTTP/SSE transport
    #[serde(default)]
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct McpToolRegistry {
    inner: Arc<Mutex<HashMap<String, McpServerState>>>,
    manager: Arc<OnceLock<Arc<tokio::sync::Mutex<McpServerManager>>>>,
}

impl McpToolRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_manager(
        &self,
        manager: Arc<tokio::sync::Mutex<McpServerManager>>,
    ) -> Result<(), Arc<tokio::sync::Mutex<McpServerManager>>> {
        self.manager.set(manager)
    }

    pub fn register_server(
        &self,
        server_name: &str,
        status: McpConnectionStatus,
        tools: Vec<McpToolInfo>,
        resources: Vec<McpResourceInfo>,
        server_info: Option<String>,
    ) {
        let mut inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        inner.insert(
            server_name.to_owned(),
            McpServerState {
                server_name: server_name.to_owned(),
                status,
                tools,
                resources,
                server_info,
                error_message: None,
                ..Default::default()
            },
        );
    }

    pub fn get_server(&self, server_name: &str) -> Option<McpServerState> {
        let inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        inner.get(server_name).cloned()
    }

    pub fn list_servers(&self) -> Vec<McpServerState> {
        let inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        inner.values().cloned().collect()
    }

    pub fn list_resources(&self, server_name: &str) -> Result<Vec<McpResourceInfo>, String> {
        let inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        match inner.get(server_name) {
            Some(state) => {
                if state.status != McpConnectionStatus::Connected {
                    return Err(format!(
                        "server '{}' is not connected (status: {})",
                        server_name, state.status
                    ));
                }
                Ok(state.resources.clone())
            },
            None => Err(format!("server '{}' not found", server_name)),
        }
    }

    pub fn read_resource(&self, server_name: &str, uri: &str) -> Result<McpResourceInfo, String> {
        let inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        let state = inner
            .get(server_name)
            .ok_or_else(|| format!("server '{}' not found", server_name))?;

        if state.status != McpConnectionStatus::Connected {
            return Err(format!(
                "server '{}' is not connected (status: {})",
                server_name, state.status
            ));
        }

        state
            .resources
            .iter()
            .find(|r| r.uri == uri)
            .cloned()
            .ok_or_else(|| format!("resource '{}' not found on server '{}'", uri, server_name))
    }

    pub fn list_tools(&self, server_name: &str) -> Result<Vec<McpToolInfo>, String> {
        let inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        match inner.get(server_name) {
            Some(state) => {
                if state.status != McpConnectionStatus::Connected {
                    return Err(format!(
                        "server '{}' is not connected (status: {})",
                        server_name, state.status
                    ));
                }
                Ok(state.tools.clone())
            },
            None => Err(format!("server '{}' not found", server_name)),
        }
    }

    #[allow(clippy::await_holding_lock)]
    async fn call_tool_via_manager(
        manager: Arc<tokio::sync::Mutex<McpServerManager>>,
        qualified_tool_name: String,
        arguments: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        // 在当前 tokio 上下文中直接调用，不再创建嵌套 runtime
        let response = {
            let mut mgr = manager.lock().await;
            mgr.discover_tools()
                .await
                .map_err(|error| error.to_string())?;
            let response = mgr
                .call_tool(&qualified_tool_name, arguments)
                .await
                .map_err(|error| error.to_string());
            let shutdown = mgr.shutdown().await.map_err(|error| error.to_string());

            match (response, shutdown) {
                (Ok(response), Ok(())) => Ok(response),
                (Err(error), Ok(())) | (Err(error), Err(_)) => Err(error),
                (Ok(_), Err(error)) => Err(error),
            }
        }?;

        if let Some(error) = response.error {
            return Err(format!(
                "MCP server returned JSON-RPC error for tools/call: {} ({})",
                error.message, error.code
            ));
        }

        let result = response
            .result
            .ok_or_else(|| "MCP server returned no result for tools/call".to_string())?;

        serde_json::to_value(result)
            .map_err(|error| format!("failed to serialize MCP tool result: {error}"))
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        let state = inner
            .get(server_name)
            .ok_or_else(|| format!("server '{}' not found", server_name))?;

        if state.status != McpConnectionStatus::Connected {
            return Err(format!(
                "server '{}' is not connected (status: {})",
                server_name, state.status
            ));
        }

        if !state.tools.iter().any(|t| t.name == tool_name) {
            return Err(format!("tool '{}' not found on server '{}'", tool_name, server_name));
        }

        drop(inner);

        let manager = self
            .manager
            .get()
            .cloned()
            .ok_or_else(|| "MCP server manager is not configured".to_string())?;

        Self::call_tool_via_manager(
            manager,
            mcp_tool_name(server_name, tool_name),
            (!arguments.is_null()).then(|| arguments.clone()),
        )
        .await
    }

    /// Call a tool using the unified MCP client (`core::mcp_client`).
    ///
    /// This is the preferred execution path — it uses connection pooling and
    /// supports all transport types (stdio/http/sse). Falls back to the legacy
    /// `McpServerManager` path if the server has no transport config stored.
    #[allow(clippy::await_holding_lock)]
    pub async fn call_tool_via_unified_client(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        let state = inner
            .get(server_name)
            .ok_or_else(|| format!("server '{}' not found", server_name))?;

        if state.status != McpConnectionStatus::Connected {
            return Err(format!(
                "server '{}' is not connected (status: {})",
                server_name, state.status
            ));
        }

        if !state.tools.iter().any(|t| t.name == tool_name) {
            return Err(format!("tool '{}' not found on server '{}'", tool_name, server_name));
        }

        let transport = state.transport.clone();
        let command = state.command.clone();
        let args: Option<Vec<String>> = state
            .args_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        let env: Option<std::collections::HashMap<String, String>> = state
            .env_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        let endpoint = state.endpoint.clone();

        drop(inner);

        // If we have transport config, use the unified client
        if let Some(ref transport) = transport
            && transport != "builtin"
        {
            let result = axagent_core::mcp_client::call_tool_unified(
                transport,
                command.as_deref(),
                args.as_deref(),
                env.as_ref(),
                endpoint.as_deref(),
                tool_name,
                arguments.clone(),
            )
            .await
            .map_err(|e| format!("MCP 工具调用失败: {e}"))?;

            if result.is_error {
                return Err(format!("MCP 工具返回错误: {}", result.content));
            }
            return Ok(serde_json::Value::String(result.content));
        }

        // Fallback: try legacy McpServerManager
        let manager = self
            .manager
            .get()
            .cloned()
            .ok_or_else(|| "MCP server manager is not configured".to_string())?;

        Self::call_tool_via_manager(
            manager,
            mcp_tool_name(server_name, tool_name),
            (!arguments.is_null()).then(|| arguments.clone()),
        )
        .await
    }

    /// Set auth status for a server.
    pub fn set_auth_status(
        &self,
        server_name: &str,
        status: McpConnectionStatus,
    ) -> Result<(), String> {
        let mut inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        let state = inner
            .get_mut(server_name)
            .ok_or_else(|| format!("server '{}' not found", server_name))?;
        state.status = status;
        Ok(())
    }

    /// Disconnect / remove a server.
    pub fn disconnect(&self, server_name: &str) -> Option<McpServerState> {
        let mut inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        inner.remove(server_name)
    }

    /// Number of registered servers.
    #[must_use]
    pub fn len(&self) -> usize {
        let inner = lock_or_recover(self.inner.lock(), "mcp_tool_bridge");
        inner.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::config::{
        ConfigSource, McpServerConfig, McpStdioServerConfig, ScopedMcpServerConfig,
    };

    /// 查找 MCP 测试服务器二进制文件路径
    fn mcp_test_server_path() -> PathBuf {
        // 测试二进制通常在 target/debug/deps/ 下
        // mcp-test-server 在 target/debug/ 下
        let test_exe = std::env::current_exe().expect("current exe");
        let target_dir = test_exe
            .parent()  // deps/
            .and_then(|p| p.parent())  // debug/ or release/
            .expect("target dir");

        let exe_name = if cfg!(windows) {
            "mcp-test-server.exe"
        } else {
            "mcp-test-server"
        };

        let path = target_dir.join(exe_name);
        assert!(
            path.exists(),
            "mcp-test-server binary not found at {}. Build with: cargo build --package axagent-runtime",
            path.display()
        );
        path
    }

    fn temp_dir() -> PathBuf {
        static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let unique_id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("runtime-mcp-tool-bridge-{nanos}-{unique_id}"))
    }

    fn cleanup_temp_dir(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    fn manager_server_config(server_name: &str, log_path: &Path) -> ScopedMcpServerConfig {
        manager_server_config_with_env(server_name, log_path, BTreeMap::new())
    }

    fn manager_server_config_with_env(
        server_name: &str,
        log_path: &Path,
        extra_env: BTreeMap<String, String>,
    ) -> ScopedMcpServerConfig {
        let mut env = BTreeMap::from([
            ("MCP_SERVER_LABEL".to_string(), server_name.to_string()),
            ("MCP_LOG_PATH".to_string(), log_path.to_string_lossy().into_owned()),
        ]);
        env.extend(extra_env);
        ScopedMcpServerConfig {
            scope: ConfigSource::Local,
            config: McpServerConfig::Stdio(McpStdioServerConfig {
                command: mcp_test_server_path().to_string_lossy().into_owned(),
                args: Vec::new(),
                env,
                tool_call_timeout_ms: Some(1_000),
            }),
        }
    }

    #[test]
    fn registers_and_retrieves_server() {
        let registry = McpToolRegistry::new();
        registry.register_server(
            "test-server",
            McpConnectionStatus::Connected,
            vec![McpToolInfo {
                name: "greet".into(),
                description: Some("Greet someone".into()),
                input_schema: None,
            }],
            vec![McpResourceInfo {
                uri: "res://data".into(),
                name: "Data".into(),
                description: None,
                mime_type: Some("application/json".into()),
            }],
            Some("TestServer v1.0".into()),
        );

        let server = registry.get_server("test-server").expect("should exist");
        assert_eq!(server.status, McpConnectionStatus::Connected);
        assert_eq!(server.tools.len(), 1);
        assert_eq!(server.resources.len(), 1);
    }

    #[test]
    fn lists_resources_from_connected_server() {
        let registry = McpToolRegistry::new();
        registry.register_server(
            "srv",
            McpConnectionStatus::Connected,
            vec![],
            vec![McpResourceInfo {
                uri: "res://alpha".into(),
                name: "Alpha".into(),
                description: None,
                mime_type: None,
            }],
            None,
        );

        let resources = registry.list_resources("srv").expect("should succeed");
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].uri, "res://alpha");
    }

    #[test]
    fn rejects_resource_listing_for_disconnected_server() {
        let registry = McpToolRegistry::new();
        registry.register_server("srv", McpConnectionStatus::Disconnected, vec![], vec![], None);
        assert!(registry.list_resources("srv").is_err());
    }

    #[test]
    fn reads_specific_resource() {
        let registry = McpToolRegistry::new();
        registry.register_server(
            "srv",
            McpConnectionStatus::Connected,
            vec![],
            vec![McpResourceInfo {
                uri: "res://data".into(),
                name: "Data".into(),
                description: Some("Test data".into()),
                mime_type: Some("text/plain".into()),
            }],
            None,
        );

        let resource = registry
            .read_resource("srv", "res://data")
            .expect("should find");
        assert_eq!(resource.name, "Data");

        assert!(registry.read_resource("srv", "res://missing").is_err());
    }

    #[test]
    fn given_connected_server_without_manager_when_calling_tool_then_it_errors() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let registry = McpToolRegistry::new();
            registry.register_server(
                "srv",
                McpConnectionStatus::Connected,
                vec![McpToolInfo {
                    name: "greet".into(),
                    description: None,
                    input_schema: None,
                }],
                vec![],
                None,
            );

            let error = registry
                .call_tool("srv", "greet", &serde_json::json!({"name": "world"}))
                .await
                .expect_err("should require a configured manager");
            assert!(error.contains("MCP server manager is not configured"));

            // Unknown tool should fail
            assert!(
                registry
                    .call_tool("srv", "missing", &serde_json::json!({}))
                    .await
                    .is_err()
            );
        });
    }

    #[test]
    fn given_connected_server_with_manager_when_calling_tool_then_it_returns_live_result() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let root = temp_dir();
            fs::create_dir_all(&root).expect("temp dir");
            let log_path = root.join("bridge.log");
            let servers =
                BTreeMap::from([("alpha".to_string(), manager_server_config("alpha", &log_path))]);
            let manager =
                Arc::new(tokio::sync::Mutex::new(McpServerManager::from_servers(&servers)));

            let registry = McpToolRegistry::new();
            registry.register_server(
                "alpha",
                McpConnectionStatus::Connected,
                vec![McpToolInfo {
                    name: "echo".into(),
                    description: Some("Echo tool for alpha".into()),
                    input_schema: Some(serde_json::json!({
                        "type": "object",
                        "properties": {"text": {"type": "string"}},
                        "required": ["text"]
                    })),
                }],
                vec![],
                Some("bridge test server".into()),
            );
            registry
                .set_manager(Arc::clone(&manager))
                .expect("manager should only be set once");

            let result = registry
                .call_tool("alpha", "echo", &serde_json::json!({"text": "hello"}))
                .await
                .expect("should return live MCP result");

            assert_eq!(result["structuredContent"]["server"], serde_json::json!("alpha"));
            assert_eq!(result["structuredContent"]["echoed"], serde_json::json!("hello"));
            assert_eq!(result["content"][0]["text"], serde_json::json!("alpha:hello"));

            let log = fs::read_to_string(&log_path).expect("read log");
            assert_eq!(
                log.lines().collect::<Vec<_>>(),
                vec!["initialize", "tools/list", "tools/call"]
            );

            cleanup_temp_dir(&root);
        });
    }

    #[test]
    fn rejects_tool_call_on_disconnected_server() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let registry = McpToolRegistry::new();
            registry.register_server(
                "srv",
                McpConnectionStatus::AuthRequired,
                vec![McpToolInfo {
                    name: "greet".into(),
                    description: None,
                    input_schema: None,
                }],
                vec![],
                None,
            );

            assert!(
                registry
                    .call_tool("srv", "greet", &serde_json::json!({}))
                    .await
                    .is_err()
            );
        });
    }

    #[test]
    fn sets_auth_and_disconnects() {
        let registry = McpToolRegistry::new();
        registry.register_server("srv", McpConnectionStatus::AuthRequired, vec![], vec![], None);

        registry
            .set_auth_status("srv", McpConnectionStatus::Connected)
            .expect("should succeed");
        let state = registry.get_server("srv").unwrap();
        assert_eq!(state.status, McpConnectionStatus::Connected);

        let removed = registry.disconnect("srv");
        assert!(removed.is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn rejects_operations_on_missing_server() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let registry = McpToolRegistry::new();
            assert!(registry.list_resources("missing").is_err());
            assert!(registry.read_resource("missing", "uri").is_err());
            assert!(registry.list_tools("missing").is_err());
            assert!(
                registry
                    .call_tool("missing", "tool", &serde_json::json!({}))
                    .await
                    .is_err()
            );
            assert!(
                registry
                    .set_auth_status("missing", McpConnectionStatus::Connected)
                    .is_err()
            );
        });
    }

    #[test]
    fn mcp_connection_status_display_all_variants() {
        // given
        let cases = [
            (McpConnectionStatus::Disconnected, "disconnected"),
            (McpConnectionStatus::Connecting, "connecting"),
            (McpConnectionStatus::Connected, "connected"),
            (McpConnectionStatus::AuthRequired, "auth_required"),
            (McpConnectionStatus::Error, "error"),
        ];

        // when
        let rendered: Vec<_> = cases
            .into_iter()
            .map(|(status, expected)| (status.to_string(), expected))
            .collect();

        // then
        assert_eq!(
            rendered,
            vec![
                ("disconnected".to_string(), "disconnected"),
                ("connecting".to_string(), "connecting"),
                ("connected".to_string(), "connected"),
                ("auth_required".to_string(), "auth_required"),
                ("error".to_string(), "error"),
            ]
        );
    }

    #[test]
    fn list_servers_returns_all_registered() {
        // given
        let registry = McpToolRegistry::new();
        registry.register_server("alpha", McpConnectionStatus::Connected, vec![], vec![], None);
        registry.register_server("beta", McpConnectionStatus::Connecting, vec![], vec![], None);

        // when
        let servers = registry.list_servers();

        // then
        assert_eq!(servers.len(), 2);
        assert!(servers.iter().any(|server| server.server_name == "alpha"));
        assert!(servers.iter().any(|server| server.server_name == "beta"));
    }

    #[test]
    fn list_tools_from_connected_server() {
        // given
        let registry = McpToolRegistry::new();
        registry.register_server(
            "srv",
            McpConnectionStatus::Connected,
            vec![McpToolInfo {
                name: "inspect".into(),
                description: Some("Inspect data".into()),
                input_schema: Some(serde_json::json!({"type": "object"})),
            }],
            vec![],
            None,
        );

        // when
        let tools = registry.list_tools("srv").expect("tools should list");

        // then
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "inspect");
    }

    #[test]
    fn list_tools_rejects_disconnected_server() {
        // given
        let registry = McpToolRegistry::new();
        registry.register_server("srv", McpConnectionStatus::AuthRequired, vec![], vec![], None);

        // when
        let result = registry.list_tools("srv");

        // then
        let error = result.expect_err("non-connected server should fail");
        assert!(error.contains("not connected"));
        assert!(error.contains("auth_required"));
    }

    #[test]
    fn list_tools_rejects_missing_server() {
        // given
        let registry = McpToolRegistry::new();

        // when
        let result = registry.list_tools("missing");

        // then
        assert_eq!(result.expect_err("missing server should fail"), "server 'missing' not found");
    }

    #[test]
    fn get_server_returns_none_for_missing() {
        // given
        let registry = McpToolRegistry::new();

        // when
        let server = registry.get_server("missing");

        // then
        assert!(server.is_none());
    }

    #[test]
    fn call_tool_payload_structure() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let root = temp_dir();
            fs::create_dir_all(&root).expect("temp dir");
            let log_path = root.join("payload.log");
            let servers =
                BTreeMap::from([("srv".to_string(), manager_server_config("srv", &log_path))]);
            let registry = McpToolRegistry::new();
            let arguments = serde_json::json!({"text": "world"});
            registry.register_server(
                "srv",
                McpConnectionStatus::Connected,
                vec![McpToolInfo {
                    name: "echo".into(),
                    description: Some("Echo tool for srv".into()),
                    input_schema: Some(serde_json::json!({
                        "type": "object",
                        "properties": {"text": {"type": "string"}},
                        "required": ["text"]
                    })),
                }],
                vec![],
                None,
            );
            registry
                .set_manager(Arc::new(tokio::sync::Mutex::new(McpServerManager::from_servers(
                    &servers,
                ))))
                .expect("manager should only be set once");

            let result = registry
                .call_tool("srv", "echo", &arguments)
                .await
                .expect("tool should return live payload");

            assert_eq!(result["structuredContent"]["server"], "srv");
            assert_eq!(result["structuredContent"]["echoed"], "world");
            assert_eq!(result["content"][0]["text"], "srv:world");

            cleanup_temp_dir(&root);
        });
    }

    #[test]
    fn upsert_overwrites_existing_server() {
        // given
        let registry = McpToolRegistry::new();
        registry.register_server("srv", McpConnectionStatus::Connecting, vec![], vec![], None);

        // when
        registry.register_server(
            "srv",
            McpConnectionStatus::Connected,
            vec![McpToolInfo {
                name: "inspect".into(),
                description: None,
                input_schema: None,
            }],
            vec![],
            Some("Inspector".into()),
        );
        let state = registry.get_server("srv").expect("server should exist");

        // then
        assert_eq!(state.status, McpConnectionStatus::Connected);
        assert_eq!(state.tools.len(), 1);
        assert_eq!(state.server_info.as_deref(), Some("Inspector"));
    }

    #[test]
    fn disconnect_missing_returns_none() {
        // given
        let registry = McpToolRegistry::new();

        // when
        let removed = registry.disconnect("missing");

        // then
        assert!(removed.is_none());
    }

    #[test]
    fn len_and_is_empty_transitions() {
        // given
        let registry = McpToolRegistry::new();

        // when
        registry.register_server("alpha", McpConnectionStatus::Connected, vec![], vec![], None);
        registry.register_server("beta", McpConnectionStatus::Connected, vec![], vec![], None);
        let after_create = registry.len();
        registry.disconnect("alpha");
        let after_first_remove = registry.len();
        registry.disconnect("beta");

        // then
        assert_eq!(after_create, 2);
        assert_eq!(after_first_remove, 1);
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }
}
