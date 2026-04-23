//! Tool Registry for AxAgent Agent

use axagent_core::repo::tool_execution;
use axagent_runtime::{
    PermissionPolicy, PermissionOutcome, PermissionMode,
    ToolExecutor, ToolError as RuntimeToolError,
};
use sea_orm::DatabaseConnection;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::local_tool_registry::LocalToolRegistry;

/// Tool error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError {
    message: String,
}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolError {}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub server_id: String,
    pub tool_name: String,
    pub input: String,
}

/// Tool result
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: String,
    pub execution_id: String,
    pub duration_ms: Option<i64>,
}

/// Tool handler type
type ToolHandler = Box<dyn FnMut(&str) -> Result<String, ToolError> + Send>;

/// Tool execution recorder for database logging
#[derive(Clone)]
pub struct ToolExecutionRecorder {
    db: Arc<DatabaseConnection>,
}

impl ToolExecutionRecorder {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn record_start(&self, ctx: &ToolContext) -> Result<String, ToolError> {
        let execution = tool_execution::create_tool_execution(
            &self.db,
            &ctx.conversation_id,
            ctx.message_id.as_deref(),
            &ctx.server_id,
            &ctx.tool_name,
            Some(&ctx.input),
            None,
        )
        .await
        .map_err(|e| ToolError::new(e.to_string()))?;

        Ok(execution.id)
    }

    pub async fn record_success(
        &self,
        execution_id: &str,
        output: &str,
        _duration_ms: Option<i64>,
    ) -> Result<(), ToolError> {
        tool_execution::update_tool_execution_status(
            &self.db,
            execution_id,
            "success",
            Some(output),
            None,
        )
        .await
        .map_err(|e| ToolError::new(e.to_string()))?;

        Ok(())
    }

    pub async fn record_error(
        &self,
        execution_id: &str,
        error: &str,
        _duration_ms: Option<i64>,
    ) -> Result<(), ToolError> {
        tool_execution::update_tool_execution_status(
            &self.db,
            execution_id,
            "failed",
            None,
            Some(error),
        )
        .await
        .map_err(|e| ToolError::new(e.to_string()))?;

        Ok(())
    }
}

/// MCP server configuration for tool dispatch
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub server_id: String,
    pub server_name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args_json: Option<String>,
    pub env_json: Option<String>,
    pub endpoint: Option<String>,
    pub execute_timeout_secs: Option<i32>,
}

/// Tool Registry
pub struct ToolRegistry {
    handlers: BTreeMap<String, ToolHandler>,
    /// Local (builtin) tool registry — executes directly without MCP
    local_tools: LocalToolRegistry,
    mcp_tools: BTreeMap<String, McpToolConfig>,
    /// MCP server configs indexed by server_id, used for tool dispatch
    mcp_servers: BTreeMap<String, McpServerConfig>,
    recorder: Option<ToolExecutionRecorder>,
    permission_policy: PermissionPolicy,
    /// Current conversation ID for tool execution recording
    conversation_id: Option<String>,
    /// Current assistant message ID for tool execution recording
    message_id: Option<String>,
    /// Cache for read-only tool results. Key: (tool_name, input_hash), Value: (result, timestamp).
    /// Only read-only tools are cached; write/execute tools are never cached.
    result_cache: std::collections::HashMap<(String, u64), (String, std::time::Instant)>,
}

/// Cache configuration
const CACHE_TTL_SECS: u64 = 300; // 5 minutes TTL for cached results
const CACHE_MAX_ENTRIES: usize = 200; // Maximum cached entries

/// Determine if a tool is read-only and thus cacheable.
fn is_read_only_tool(tool_name: &str) -> bool {
    let name_lower = tool_name.to_lowercase();
    const READ_PATTERNS: &[&str] = &[
        "read", "list", "get", "grep", "glob", "head", "cat",
        "stat", "ls", "dir", "type", "peek", "view", "search",
        "find", "query", "fetch", "info", "show", "describe",
        "inspect", "check", "test", "validate", "health",
    ];
    const WRITE_PATTERNS: &[&str] = &[
        "write", "edit", "create", "delete", "remove", "move",
        "rename", "patch", "mkdir", "save", "put", "post",
        "upload", "install", "shell", "bash", "exec", "run",
        "command", "terminal", "spawn",
    ];
    // If it matches a write pattern, it's NOT read-only
    if WRITE_PATTERNS.iter().any(|p| name_lower.contains(p)) {
        return false;
    }
    // If it matches a read pattern, it IS read-only
    READ_PATTERNS.iter().any(|p| name_lower.contains(p))
}

#[derive(Debug, Clone)]
pub struct McpToolConfig {
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
            local_tools: LocalToolRegistry::init_from_registry(),
            mcp_tools: BTreeMap::new(),
            mcp_servers: BTreeMap::new(),
            recorder: None,
            permission_policy: PermissionPolicy::new(PermissionMode::WorkspaceWrite)
                // Read-only tools: only need ReadOnly permission
                .with_tool_requirement("read_file", PermissionMode::ReadOnly)
                .with_tool_requirement("list_directory", PermissionMode::ReadOnly)
                .with_tool_requirement("search_files", PermissionMode::ReadOnly)
                .with_tool_requirement("grep_content", PermissionMode::ReadOnly)
                .with_tool_requirement("file_exists", PermissionMode::ReadOnly)
                .with_tool_requirement("get_file_info", PermissionMode::ReadOnly)
                .with_tool_requirement("get_system_info", PermissionMode::ReadOnly)
                .with_tool_requirement("list_processes", PermissionMode::ReadOnly)
                .with_tool_requirement("get_storage_info", PermissionMode::ReadOnly)
                .with_tool_requirement("list_storage_files", PermissionMode::ReadOnly)
                .with_tool_requirement("fetch_url", PermissionMode::ReadOnly)
                .with_tool_requirement("fetch_markdown", PermissionMode::ReadOnly)
                .with_tool_requirement("web_search", PermissionMode::ReadOnly)
                .with_tool_requirement("session_search", PermissionMode::ReadOnly)
                .with_tool_requirement("list_knowledge_bases", PermissionMode::ReadOnly)
                .with_tool_requirement("search_knowledge", PermissionMode::ReadOnly)
                // Workspace-write tools: need WorkspaceWrite permission
                .with_tool_requirement("write_file", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("edit_file", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("create_directory", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("delete_file", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("move_file", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("upload_storage_file", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("download_storage_file", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("delete_storage_file", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("memory_flush", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("skill_manage", PermissionMode::WorkspaceWrite)
                // Dangerous tools: need DangerFullAccess (will prompt in WorkspaceWrite mode)
                .with_tool_requirement("run_command", PermissionMode::DangerFullAccess),
            conversation_id: None,
            message_id: None,
            result_cache: std::collections::HashMap::new(),
        }
    }

    pub fn with_recorder(mut self, recorder: ToolExecutionRecorder) -> Self {
        self.recorder = Some(recorder);
        self
    }

    pub fn with_permission_policy(mut self, policy: PermissionPolicy) -> Self {
        self.permission_policy = policy;
        self
    }

    /// Set the conversation and message context for tool execution recording.
    pub fn with_execution_context(mut self, conversation_id: String, message_id: Option<String>) -> Self {
        self.conversation_id = Some(conversation_id);
        self.message_id = message_id;
        self
    }

    /// Register a tool
    pub fn register(
        mut self,
        tool_name: impl Into<String>,
        handler: impl FnMut(&str) -> Result<String, ToolError> + 'static + Send,
    ) -> Self {
        self.handlers.insert(tool_name.into(), Box::new(handler));
        self
    }

    /// Register an MCP tool with its server configuration
    pub fn register_mcp_tool(
        mut self,
        server_id: impl Into<String>,
        server_name: impl Into<String>,
        tool_name: impl Into<String>,
        description: Option<String>,
        input_schema: Option<Value>,
        server_config: McpServerConfig,
    ) -> Self {
        let server_id_str = server_id.into();
        let server_name_str = server_name.into();
        let tool_name_str = tool_name.into();
        let key = format!("{}/{}", server_id_str, tool_name_str);
        self.mcp_tools.insert(
            key.clone(),
            McpToolConfig {
                server_id: server_id_str.clone(),
                server_name: server_name_str,
                tool_name: tool_name_str,
                description,
                input_schema,
            },
        );
        self.mcp_servers.insert(server_id_str, server_config);
        self
    }

    /// Register built-in tools (echo and add for testing)
    pub fn with_builtin_tools(self) -> Self {
        self.register("echo", |input| Ok(input.to_string()))
            .register("add", |input| {
                let numbers: Result<Vec<i32>, _> = input
                    .split(',')
                    .map(|s| s.trim().parse())
                    .collect();
                match numbers {
                    Ok(nums) => Ok(nums.iter().sum::<i32>().to_string()),
                    Err(e) => Err(ToolError::new(format!("Invalid input: {}", e))),
                }
            })
    }

    /// Set the local tool registry (replaces the default empty one).
    pub fn with_local_tools(mut self, local_tools: LocalToolRegistry) -> Self {
        self.local_tools = local_tools;
        self
    }

    /// Get a reference to the local tool registry.
    pub fn local_tools(&self) -> &LocalToolRegistry {
        &self.local_tools
    }

    /// Get a mutable reference to the local tool registry.
    pub fn local_tools_mut(&mut self) -> &mut LocalToolRegistry {
        &mut self.local_tools
    }

    /// Get a tool handler
    pub fn get_handler(&mut self, tool_name: &str) -> Option<&mut ToolHandler> {
        self.handlers.get_mut(tool_name)
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<String> {
        let mut tools = self.handlers.keys().cloned().collect::<Vec<_>>();
        // Add local tool names
        for group in self.local_tools.get_tool_groups() {
            for tool in group.tools {
                tools.push(tool.tool_name);
            }
        }
        tools.extend(self.mcp_tools.keys().cloned());
        tools
    }

    /// Check if tool requires permission prompt
    pub fn requires_permission(&self, tool_name: &str) -> bool {
        matches!(
            self.permission_policy.authorize(tool_name, "{}", None),
            PermissionOutcome::Deny { .. }
        )
    }

    /// Authorize tool execution
    pub fn authorize(&self, tool_name: &str, input: &str) -> Result<(), ToolError> {
        match self.permission_policy.authorize(tool_name, input, None) {
            PermissionOutcome::Allow => Ok(()),
            PermissionOutcome::Deny { reason } => Err(ToolError::new(reason)),
        }
    }

    /// Execute an MCP tool by looking up the server config and calling it.
    /// Bridges async MCP calls from sync context using the existing tokio runtime
    /// via block_in_place + Handle::current(), avoiding the "Cannot start a runtime
    /// from within a runtime" panic that occurs when creating a new runtime.
    fn execute_mcp_tool(&self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        // Find the MCP tool config that matches by tool_name (last segment of key)
        let mcp_config = self.mcp_tools.values()
            .find(|tc| tc.tool_name == tool_name)
            .ok_or_else(|| ToolError::new(format!("Unknown MCP tool: {}", tool_name)))?;

        let server_config = self.mcp_servers.get(&mcp_config.server_id)
            .ok_or_else(|| ToolError::new(format!("MCP server not found: {}", mcp_config.server_id)))?
            .clone();

        let tool_name_owned = tool_name.to_string();
        let input_owned = input.to_string();

        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| {
            handle.block_on(async move {
                let arguments: Value = serde_json::from_str(&input_owned)
                    .unwrap_or(Value::Object(serde_json::Map::new()));

                let timeout_secs = server_config.execute_timeout_secs.unwrap_or(30) as u64;
                let timeout_duration = std::time::Duration::from_secs(timeout_secs);

                let result = match server_config.transport.as_str() {
                    "stdio" => {
                        let command = server_config.command
                            .ok_or_else(|| ToolError::new("stdio server has no command configured"))?;
                        let args: Vec<String> = server_config.args_json
                            .as_ref()
                            .and_then(|s| serde_json::from_str(s).ok())
                            .unwrap_or_default();
                        let env: std::collections::HashMap<String, String> = server_config.env_json
                            .as_ref()
                            .and_then(|s| serde_json::from_str(s).ok())
                            .unwrap_or_default();
                        // Use the pooled version to reuse MCP server processes
                        // instead of spawning a new one for every call.
                        tokio::time::timeout(
                            timeout_duration,
                            axagent_core::mcp_client::call_tool_stdio_pooled(
                                &command,
                                &args,
                                &env,
                                &tool_name_owned,
                                arguments,
                            ),
                        )
                        .await
                        .map_err(|_| ToolError::new(format!("Tool timed out after {}s", timeout_secs)))?
                        .map_err(|e| ToolError::new(e.to_string()))?
                    }
                    "http" => {
                        let endpoint = server_config.endpoint
                            .ok_or_else(|| ToolError::new("HTTP server has no endpoint configured"))?;
                        tokio::time::timeout(
                            timeout_duration,
                            axagent_core::mcp_client::call_tool_http(
                                &endpoint,
                                &tool_name_owned,
                                arguments,
                            ),
                        )
                        .await
                        .map_err(|_| ToolError::new(format!("Tool timed out after {}s", timeout_secs)))?
                        .map_err(|e| ToolError::new(e.to_string()))?
                    }
                    "sse" => {
                        let endpoint = server_config.endpoint
                            .ok_or_else(|| ToolError::new("SSE server has no endpoint configured"))?;
                        tokio::time::timeout(
                            timeout_duration,
                            axagent_core::mcp_client::call_tool_sse(
                                &endpoint,
                                &tool_name_owned,
                                arguments,
                            ),
                        )
                        .await
                        .map_err(|_| ToolError::new(format!("Tool timed out after {}s", timeout_secs)))?
                        .map_err(|e| ToolError::new(e.to_string()))?
                    }
                    other => return Err(ToolError::new(format!("Unsupported transport '{}'", other))),
                };

                if result.is_error {
                    Err(ToolError::new(result.content))
                } else {
                    Ok(result.content)
                }
            })
        })
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolExecutor for ToolRegistry {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, RuntimeToolError> {
        // Check result cache for read-only tools
        if is_read_only_tool(tool_name) {
            let input_hash = {
                use std::hash::Hasher;
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                hasher.write(input.as_bytes());
                hasher.finish()
            };
            let cache_key = (tool_name.to_string(), input_hash);

            // Evict expired entries if cache is too large
            if self.result_cache.len() > CACHE_MAX_ENTRIES {
                let now = std::time::Instant::now();
                let ttl = std::time::Duration::from_secs(CACHE_TTL_SECS);
                self.result_cache.retain(|_, (_, ts)| now.duration_since(*ts) < ttl);
                // If still too large, remove oldest entries
                if self.result_cache.len() > CACHE_MAX_ENTRIES {
                    let mut entries: Vec<_> = self.result_cache.iter()
                        .map(|(k, (_, ts))| (k.clone(), *ts))
                        .collect();
                    entries.sort_by_key(|(_, ts)| *ts);
                    let to_remove = entries.len() - CACHE_MAX_ENTRIES;
                    for (key, _) in entries.into_iter().take(to_remove) {
                        self.result_cache.remove(&key);
                    }
                }
            }

            if let Some((cached_result, timestamp)) = self.result_cache.get(&cache_key) {
                let now = std::time::Instant::now();
                if now.duration_since(*timestamp) < std::time::Duration::from_secs(CACHE_TTL_SECS) {
                    tracing::debug!("[tool-cache] Cache hit for '{}' (age: {:?})", tool_name, now.duration_since(*timestamp));
                    return Ok(cached_result.clone());
                }
                // Expired — remove
                self.result_cache.remove(&cache_key);
            }
        }

        // Determine the server_id for recording
        let server_id = if self.handlers.contains_key(tool_name) {
            "builtin".to_string()
        } else if self.local_tools.contains(tool_name) {
            self.local_tools.get_group_id(tool_name)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "local".to_string())
        } else if let Some(tc) = self.mcp_tools.values().find(|tc| tc.tool_name == tool_name) {
            tc.server_id.clone()
        } else {
            "unknown".to_string()
        };

        // Record execution start (if recorder is configured)
        let execution_id = self.record_start_sync(tool_name, input, &server_id);

        let start = std::time::Instant::now();

        // Execute the tool
        let result = if let Some(handler) = self.handlers.get_mut(tool_name) {
            handler(input).map_err(|e| RuntimeToolError::new(e.to_string()))
        } else if self.local_tools.contains(tool_name) {
            // Local tool: execute directly without MCP
            let arguments: Value = serde_json::from_str(input)
                .unwrap_or(Value::Object(serde_json::Map::new()));
            let handle = tokio::runtime::Handle::current();
            let tool_name_owned = tool_name.to_string();
            tokio::task::block_in_place(|| {
                handle.block_on(self.local_tools.execute(&tool_name_owned, arguments))
            })
            .map_err(|e| RuntimeToolError::new(e))
        } else {
            let is_mcp_tool = self.mcp_tools.values()
                .any(|tc| tc.tool_name == tool_name);

            if is_mcp_tool {
                self.execute_mcp_tool(tool_name, input)
                    .map_err(|e| RuntimeToolError::new(e.to_string()))
            } else {
                Err(RuntimeToolError::new(format!("Unknown tool: {}", tool_name)))
            }
        };

        let duration_ms = start.elapsed().as_millis() as i64;

        // Record execution result (if recorder is configured)
        match &result {
            Ok(output) => {
                self.record_result_sync(&execution_id, true, output, duration_ms);

                // Cache successful results for read-only tools
                if is_read_only_tool(tool_name) {
                    let input_hash = {
                        use std::hash::Hasher;
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        hasher.write(input.as_bytes());
                        hasher.finish()
                    };
                    let cache_key = (tool_name.to_string(), input_hash);
                    self.result_cache.insert(cache_key, (output.clone(), std::time::Instant::now()));
                    tracing::debug!("[tool-cache] Cached result for '{}' ({} bytes)", tool_name, output.len());
                }
            }
            Err(e) => {
                self.record_result_sync(&execution_id, false, &e.to_string(), duration_ms);
            }
        }

        result
    }
}

impl ToolRegistry {
    /// Record execution start synchronously (bridges async recorder via existing tokio runtime)
    fn record_start_sync(&self, tool_name: &str, input: &str, server_id: &str) -> Option<String> {
        let recorder = self.recorder.as_ref()?;
        let conversation_id = self.conversation_id.as_deref().unwrap_or("unknown");
        let ctx = ToolContext {
            conversation_id: conversation_id.to_string(),
            message_id: self.message_id.clone(),
            server_id: server_id.to_string(),
            tool_name: tool_name.to_string(),
            input: input.to_string(),
        };
        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| {
            handle.block_on(recorder.record_start(&ctx)).ok()
        })
    }

    /// Record execution result synchronously (bridges async recorder via existing tokio runtime)
    fn record_result_sync(&self, execution_id: &Option<String>, is_success: bool, content: &str, duration_ms: i64) {
        let Some(recorder) = self.recorder.as_ref() else { return };
        let Some(exec_id) = execution_id.as_deref() else { return };
        let recorder = recorder.clone();
        let exec_id = exec_id.to_string();
        let content = content.to_string();
        let handle = tokio::runtime::Handle::current();
        let _ = tokio::task::block_in_place(|| {
            handle.block_on(async {
                if is_success {
                    let _ = recorder.record_success(&exec_id, &content, Some(duration_ms)).await;
                } else {
                    let _ = recorder.record_error(&exec_id, &content, Some(duration_ms)).await;
                }
            });
        });
    }
}