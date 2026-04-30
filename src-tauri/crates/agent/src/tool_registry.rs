//! Tool Registry for AxAgent Agent

use axagent_core::repo::tool_execution;
use axagent_runtime::{
    PermissionMode, PermissionOutcome, PermissionPolicy, ToolError as RuntimeToolError,
    ToolExecutor,
};
use chrono::{DateTime, Utc};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::local_tool_registry::LocalToolRegistry;
use std::collections::HashSet;

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

    pub fn permission_denied(tool_name: &str) -> Self {
        Self {
            message: format!("Tool '{}' is not allowed to execute", tool_name),
        }
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolError {}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub server_id: String,
    pub tool_name: String,
    pub input: String,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: String,
    pub execution_id: String,
    pub duration_ms: Option<i64>,
}

type ToolHandler = Box<dyn FnMut(&str) -> Result<String, ToolError> + Send>;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub server_id: String,
    pub server_name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args_json: Option<String>,
    pub env_json: Option<String>,
    pub endpoint: Option<String>,
    pub execute_timeout_secs: Option<i32>,
    pub connection_pool_size: Option<usize>,
    pub retry_attempts: Option<u32>,
    pub retry_delay_ms: Option<u64>,
}

impl McpServerConfig {
    pub fn get_timeout(&self) -> Duration {
        Duration::from_secs(self.execute_timeout_secs.unwrap_or(30) as u64)
    }

    pub fn get_pool_size(&self) -> usize {
        self.connection_pool_size.unwrap_or(4)
    }

    pub fn get_retry_attempts(&self) -> u32 {
        self.retry_attempts.unwrap_or(3)
    }

    pub fn get_retry_delay(&self) -> Duration {
        Duration::from_millis(self.retry_delay_ms.unwrap_or(100))
    }
}

#[derive(Clone)]
pub struct McpRegistry {
    mcp_tools: BTreeMap<String, McpToolConfig>,
    mcp_servers: BTreeMap<String, McpServerConfig>,
}

unsafe impl Send for McpRegistry {}
unsafe impl Sync for McpRegistry {}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            mcp_tools: BTreeMap::new(),
            mcp_servers: BTreeMap::new(),
        }
    }

    pub fn with_tools_and_servers(
        mcp_tools: BTreeMap<String, McpToolConfig>,
        mcp_servers: BTreeMap<String, McpServerConfig>,
    ) -> Self {
        Self {
            mcp_tools,
            mcp_servers,
        }
    }

    pub fn execute_mcp_tool(&self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        let mcp_config = self
            .mcp_tools
            .values()
            .find(|c| c.tool_name == tool_name)
            .ok_or_else(|| ToolError::new(format!("MCP tool '{}' not found", tool_name)))?;

        let server_config = self.mcp_servers.get(&mcp_config.server_id).ok_or_else(|| {
            ToolError::new(format!(
                "MCP server '{}' not found for tool '{}'",
                mcp_config.server_id, tool_name
            ))
        })?;

        let command = server_config.command.as_deref().unwrap_or("npx");
        let args: Vec<String> = if let Some(ref args_json) = server_config.args_json {
            serde_json::from_str(args_json).unwrap_or_default()
        } else {
            Vec::new()
        };
        let env: std::collections::HashMap<String, String> =
            if let Some(ref env_json) = server_config.env_json {
                serde_json::from_str(env_json).unwrap_or_default()
            } else {
                std::collections::HashMap::new()
            };

        let arguments: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| ToolError::new(format!("Failed to parse tool arguments: {}", e)))?;

        let rt = tokio::runtime::Handle::current();
        rt.block_on(axagent_core::mcp_client::call_tool_stdio_pooled(
            command,
            &args,
            &env,
            &mcp_config.tool_name,
            arguments,
        ))
        .map_err(|e| ToolError::new(format!("MCP call failed: {}", e)))
        .map(|r| r.content)
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    ReadOnly,
    Write,
    Execute,
    Network,
    System,
}

impl ToolCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolCategory::ReadOnly => "read_only",
            ToolCategory::Write => "write",
            ToolCategory::Execute => "execute",
            ToolCategory::Network => "network",
            ToolCategory::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub category: ToolCategory,
    pub is_cacheable: bool,
    pub avg_execution_time_ms: Option<f64>,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_used: Option<DateTime<Utc>>,
}

impl ToolMetadata {
    pub fn new(name: String, category: ToolCategory) -> Self {
        Self {
            name,
            category,
            is_cacheable: matches!(category, ToolCategory::ReadOnly),
            avg_execution_time_ms: None,
            success_count: 0,
            failure_count: 0,
            last_used: None,
        }
    }

    pub fn record_success(&mut self, execution_time_ms: f64) {
        self.success_count += 1;
        self.last_used = Some(Utc::now());
        let n = self.success_count as f64;
        let current_avg = self.avg_execution_time_ms.unwrap_or(execution_time_ms);
        self.avg_execution_time_ms = Some(current_avg + (execution_time_ms - current_avg) / n);
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_used = Some(Utc::now());
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            1.0
        } else {
            self.success_count as f64 / total as f64
        }
    }
}

#[derive(Clone)]
pub struct ToolUsageStats {
    metrics: Arc<parking_lot::RwLock<std::collections::HashMap<String, ToolMetadata>>>,
}

impl ToolUsageStats {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn record_execution(
        &self,
        tool_name: &str,
        category: ToolCategory,
        execution_time_ms: f64,
        success: bool,
    ) {
        let mut metrics = self.metrics.write();
        let metadata = metrics
            .entry(tool_name.to_string())
            .or_insert_with(|| ToolMetadata::new(tool_name.to_string(), category));

        if success {
            metadata.record_success(execution_time_ms);
        } else {
            metadata.record_failure();
        }
    }

    pub fn get_stats(&self, tool_name: &str) -> Option<ToolMetadata> {
        self.metrics.read().get(tool_name).cloned()
    }

    pub fn get_all_stats(&self) -> std::collections::HashMap<String, ToolMetadata> {
        self.metrics.read().clone()
    }

    pub fn top_tools(&self, limit: usize) -> Vec<(String, ToolMetadata)> {
        let metrics = self.metrics.read();
        let mut sorted: Vec<_> = metrics
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        sorted.sort_by(|a, b| b.1.success_count.cmp(&a.1.success_count));
        sorted.into_iter().take(limit).collect()
    }

    pub fn failed_tools(&self) -> Vec<(String, ToolMetadata)> {
        let metrics = self.metrics.read();
        metrics
            .iter()
            .filter(|(_, v)| v.failure_count > 0)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl Default for ToolUsageStats {
    fn default() -> Self {
        Self::new()
    }
}

const CACHE_TTL_SECS: u64 = 300;
const CACHE_MAX_ENTRIES: usize = 200;

fn is_read_only_tool(tool_name: &str) -> bool {
    let name_lower = tool_name.to_lowercase();
    const READ_PATTERNS: &[&str] = &[
        "read", "list", "get", "grep", "glob", "head", "cat", "stat", "ls", "dir", "type", "peek",
        "view", "search", "find", "query", "fetch", "info", "show", "describe", "inspect", "check",
        "test", "validate", "health",
    ];
    const WRITE_PATTERNS: &[&str] = &[
        "write", "edit", "create", "delete", "remove", "move", "rename", "patch", "mkdir", "save",
        "put", "post", "upload", "install", "shell", "bash", "exec", "run", "command", "terminal",
        "spawn",
    ];
    if WRITE_PATTERNS.iter().any(|p| name_lower.contains(p)) {
        return false;
    }
    READ_PATTERNS.iter().any(|p| name_lower.contains(p))
}

fn classify_tool(tool_name: &str) -> ToolCategory {
    let name_lower = tool_name.to_lowercase();
    const EXECUTE_PATTERNS: &[&str] = &[
        "shell", "bash", "exec", "run", "command", "terminal", "spawn", "install", "npm", "cargo",
    ];
    const NETWORK_PATTERNS: &[&str] = &[
        "fetch", "http", "request", "download", "upload", "curl", "wget", "api",
    ];
    const SYSTEM_PATTERNS: &[&str] = &["process", "memory", "cpu", "disk", "system", "os"];

    if is_read_only_tool(tool_name) {
        ToolCategory::ReadOnly
    } else if EXECUTE_PATTERNS.iter().any(|p| name_lower.contains(p)) {
        ToolCategory::Execute
    } else if NETWORK_PATTERNS.iter().any(|p| name_lower.contains(p)) {
        ToolCategory::Network
    } else if SYSTEM_PATTERNS.iter().any(|p| name_lower.contains(p)) {
        ToolCategory::System
    } else {
        ToolCategory::Write
    }
}

#[derive(Debug, Clone)]
pub struct McpToolConfig {
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

pub struct ToolRegistry {
    handlers: BTreeMap<String, ToolHandler>,
    local_tools: LocalToolRegistry,
    mcp_tools: BTreeMap<String, McpToolConfig>,
    mcp_servers: BTreeMap<String, McpServerConfig>,
    recorder: Option<ToolExecutionRecorder>,
    permission_policy: PermissionPolicy,
    conversation_id: Option<String>,
    message_id: Option<String>,
    result_cache: std::collections::HashMap<(String, u64), (String, Instant)>,
    usage_stats: ToolUsageStats,
    allowed_tools: HashSet<String>,
    blocked_tools: HashSet<String>,
    strict_mode: bool,
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
                .with_tool_requirement("run_command", PermissionMode::DangerFullAccess),
            conversation_id: None,
            message_id: None,
            result_cache: std::collections::HashMap::new(),
            usage_stats: ToolUsageStats::new(),
            allowed_tools: HashSet::new(),
            blocked_tools: HashSet::new(),
            strict_mode: false,
        }
    }

    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools.into_iter().collect();
        self
    }

    pub fn with_blocked_tools(mut self, tools: Vec<String>) -> Self {
        self.blocked_tools = tools.into_iter().collect();
        self
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if self.blocked_tools.contains(tool_name) {
            return false;
        }

        if self.strict_mode {
            self.allowed_tools.is_empty() || self.allowed_tools.contains(tool_name)
        } else {
            true
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

    pub fn with_execution_context(
        mut self,
        conversation_id: String,
        message_id: Option<String>,
    ) -> Self {
        self.conversation_id = Some(conversation_id);
        self.message_id = message_id;
        self
    }

    pub fn register(
        mut self,
        tool_name: impl Into<String>,
        handler: impl FnMut(&str) -> Result<String, ToolError> + 'static + Send,
    ) -> Self {
        self.handlers.insert(tool_name.into(), Box::new(handler));
        self
    }

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

    pub fn with_builtin_tools(self) -> Self {
        self.register("echo", |input| Ok(input.to_string()))
            .register("add", |input| {
                let numbers: Result<Vec<i32>, _> =
                    input.split(',').map(|s| s.trim().parse()).collect();
                match numbers {
                    Ok(nums) => Ok(nums.iter().sum::<i32>().to_string()),
                    Err(e) => Err(ToolError::new(format!("Invalid input: {}", e))),
                }
            })
    }

    pub fn with_local_tools(mut self, local_tools: LocalToolRegistry) -> Self {
        self.local_tools = local_tools;
        self
    }

    #[allow(clippy::type_complexity)]
    pub fn register_skill_tool(
        mut self,
        tool_name: impl Into<String>,
        handler: Box<dyn FnMut(&str) -> Result<String, ToolError> + Send>,
    ) -> Self {
        self.handlers.insert(tool_name.into(), handler);
        self
    }

    pub fn local_tools(&self) -> &LocalToolRegistry {
        &self.local_tools
    }

    pub fn local_tools_mut(&mut self) -> &mut LocalToolRegistry {
        &mut self.local_tools
    }

    pub fn get_handler(&mut self, tool_name: &str) -> Option<&mut ToolHandler> {
        self.handlers.get_mut(tool_name)
    }

    pub fn list_tools(&self) -> Vec<String> {
        let mut tools = self.handlers.keys().cloned().collect::<Vec<_>>();
        for group in self.local_tools.get_tool_groups() {
            for tool in group.tools {
                tools.push(tool.tool_name);
            }
        }
        tools.extend(self.mcp_tools.keys().cloned());
        tools
    }

    pub fn list_tools_by_category(&self) -> std::collections::HashMap<ToolCategory, Vec<String>> {
        let mut categories: std::collections::HashMap<ToolCategory, Vec<String>> =
            std::collections::HashMap::new();
        for tool_name in self.list_tools() {
            let category = classify_tool(&tool_name);
            categories.entry(category).or_default().push(tool_name);
        }
        categories
    }

    pub fn requires_permission(&self, tool_name: &str) -> bool {
        matches!(
            self.permission_policy.authorize(tool_name, "{}", None),
            PermissionOutcome::Deny { .. }
        )
    }

    pub fn authorize(&self, tool_name: &str, input: &str) -> Result<(), ToolError> {
        match self.permission_policy.authorize(tool_name, input, None) {
            PermissionOutcome::Allow => Ok(()),
            PermissionOutcome::Deny { reason } => Err(ToolError::new(reason)),
        }
    }

    pub fn mcp_registry(&self) -> McpRegistry {
        McpRegistry::with_tools_and_servers(self.mcp_tools.clone(), self.mcp_servers.clone())
    }

    pub fn get_usage_stats(&self) -> &ToolUsageStats {
        &self.usage_stats
    }

    pub fn get_tool_category(&self, tool_name: &str) -> ToolCategory {
        if self.handlers.contains_key(tool_name) {
            ToolCategory::Execute
        } else if self.local_tools.contains(tool_name) {
            ToolCategory::Write
        } else if self.mcp_tools.values().any(|tc| tc.tool_name == tool_name) {
            ToolCategory::Execute
        } else {
            classify_tool(tool_name)
        }
    }

    pub fn execute_mcp_tool(&self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        let mcp_config = self
            .mcp_tools
            .values()
            .find(|tc| tc.tool_name == tool_name)
            .ok_or_else(|| ToolError::new(format!("Unknown MCP tool: {}", tool_name)))?;

        let server_config = self
            .mcp_servers
            .get(&mcp_config.server_id)
            .ok_or_else(|| {
                ToolError::new(format!("MCP server not found: {}", mcp_config.server_id))
            })?
            .clone();

        let tool_name_owned = tool_name.to_string();
        let input_owned = input.to_string();

        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| {
            handle.block_on(async move {
                let arguments: Value = serde_json::from_str(&input_owned)
                    .unwrap_or(Value::Object(serde_json::Map::new()));

                let timeout_duration = server_config.get_timeout();

                let result = match server_config.transport.as_str() {
                    "stdio" => {
                        let command = server_config.command.clone().ok_or_else(|| {
                            ToolError::new("stdio server has no command configured")
                        })?;
                        let args: Vec<String> = server_config
                            .args_json
                            .as_ref()
                            .and_then(|s| serde_json::from_str(s).ok())
                            .unwrap_or_default();
                        let env: std::collections::HashMap<String, String> = server_config
                            .env_json
                            .as_ref()
                            .and_then(|s| serde_json::from_str(s).ok())
                            .unwrap_or_default();

                        let mut last_error = None;
                        for attempt in 0..server_config.get_retry_attempts() {
                            if attempt > 0 {
                                tokio::time::sleep(server_config.get_retry_delay()).await;
                            }
                            match tokio::time::timeout(
                                timeout_duration,
                                axagent_core::mcp_client::call_tool_stdio_pooled(
                                    &command,
                                    &args,
                                    &env,
                                    &tool_name_owned,
                                    arguments.clone(),
                                ),
                            )
                            .await
                            {
                                Ok(Ok(result)) => {
                                    if !result.is_error {
                                        return Ok(result.content);
                                    }
                                    last_error = Some(result.content);
                                }
                                Ok(Err(e)) => last_error = Some(e.to_string()),
                                Err(_) => {
                                    last_error = Some(format!(
                                        "Tool timed out after {}s",
                                        timeout_duration.as_secs()
                                    ))
                                }
                            }
                        }
                        Err(ToolError::new(
                            last_error.unwrap_or("All retry attempts failed".to_string()),
                        ))
                    }
                    "http" => {
                        let endpoint = server_config.endpoint.ok_or_else(|| {
                            ToolError::new("HTTP server has no endpoint configured")
                        })?;
                        tokio::time::timeout(
                            timeout_duration,
                            axagent_core::mcp_client::call_tool_http(
                                &endpoint,
                                &tool_name_owned,
                                arguments,
                            ),
                        )
                        .await
                        .map_err(|_| {
                            ToolError::new(format!(
                                "Tool timed out after {}s",
                                timeout_duration.as_secs()
                            ))
                        })?
                        .map_err(|e| ToolError::new(e.to_string()))
                        .map(|r| r.content)
                    }
                    "sse" => {
                        let endpoint = server_config.endpoint.ok_or_else(|| {
                            ToolError::new("SSE server has no endpoint configured")
                        })?;
                        tokio::time::timeout(
                            timeout_duration,
                            axagent_core::mcp_client::call_tool_sse(
                                &endpoint,
                                &tool_name_owned,
                                arguments,
                            ),
                        )
                        .await
                        .map_err(|_| {
                            ToolError::new(format!(
                                "Tool timed out after {}s",
                                timeout_duration.as_secs()
                            ))
                        })?
                        .map_err(|e| ToolError::new(e.to_string()))
                        .map(|r| r.content)
                    }
                    other => Err(ToolError::new(format!("Unsupported transport '{}'", other))),
                };

                result
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
        if !self.is_tool_allowed(tool_name) {
            tracing::warn!(
                "Tool '{}' execution denied by whitelist/blacklist policy",
                tool_name
            );
            return Err(RuntimeToolError::new(
                ToolError::permission_denied(tool_name).to_string(),
            ));
        }

        let category = self.get_tool_category(tool_name);
        let is_cacheable =
            matches!(category, ToolCategory::ReadOnly) && is_read_only_tool(tool_name);

        if is_cacheable {
            let input_hash = compute_hash(input);
            let cache_key = (tool_name.to_string(), input_hash);

            self.evict_cache_if_needed();

            if let Some((cached_result, timestamp)) = self.result_cache.get(&cache_key) {
                let now = Instant::now();
                if now.duration_since(*timestamp) < Duration::from_secs(CACHE_TTL_SECS) {
                    tracing::debug!(
                        "[tool-cache] Cache hit for '{}' (age: {:?})",
                        tool_name,
                        now.duration_since(*timestamp)
                    );
                    return Ok(cached_result.clone());
                }
                self.result_cache.remove(&cache_key);
            }
        }

        let server_id = if self.handlers.contains_key(tool_name) {
            "builtin".to_string()
        } else if self.local_tools.contains(tool_name) {
            self.local_tools
                .get_group_id(tool_name)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "local".to_string())
        } else if let Some(tc) = self.mcp_tools.values().find(|tc| tc.tool_name == tool_name) {
            tc.server_id.clone()
        } else {
            "unknown".to_string()
        };

        let execution_id = self.record_start_sync(tool_name, input, &server_id);
        let start = Instant::now();

        let result = if let Some(handler) = self.handlers.get_mut(tool_name) {
            handler(input).map_err(|e| RuntimeToolError::new(e.to_string()))
        } else if self.local_tools.contains(tool_name) {
            let arguments: Value =
                serde_json::from_str(input).unwrap_or(Value::Object(serde_json::Map::new()));
            let handle = tokio::runtime::Handle::current();
            let tool_name_owned = tool_name.to_string();
            tokio::task::block_in_place(|| {
                handle.block_on(self.local_tools.execute(&tool_name_owned, arguments))
            })
            .map_err(RuntimeToolError::new)
        } else {
            let is_mcp_tool = self.mcp_tools.values().any(|tc| tc.tool_name == tool_name);

            if is_mcp_tool {
                self.execute_mcp_tool(tool_name, input)
                    .map_err(|e| RuntimeToolError::new(e.to_string()))
            } else {
                Err(RuntimeToolError::new(format!(
                    "Unknown tool: {}",
                    tool_name
                )))
            }
        };

        let duration_ms = start.elapsed().as_millis() as i64;
        let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        match &result {
            Ok(output) => {
                self.record_result_sync(&execution_id, true, output, duration_ms);
                self.usage_stats
                    .record_execution(tool_name, category, execution_time_ms, true);

                if is_cacheable {
                    let input_hash = compute_hash(input);
                    let cache_key = (tool_name.to_string(), input_hash);
                    self.result_cache
                        .insert(cache_key, (output.clone(), Instant::now()));
                    tracing::debug!(
                        "[tool-cache] Cached result for '{}' ({} bytes)",
                        tool_name,
                        output.len()
                    );
                }
            }
            Err(e) => {
                self.record_result_sync(&execution_id, false, &e.to_string(), duration_ms);
                self.usage_stats
                    .record_execution(tool_name, category, execution_time_ms, false);
            }
        }

        result
    }
}

impl ToolRegistry {
    fn evict_cache_if_needed(&mut self) {
        if self.result_cache.len() > CACHE_MAX_ENTRIES {
            let now = Instant::now();
            let ttl = Duration::from_secs(CACHE_TTL_SECS);
            self.result_cache
                .retain(|_, (_, ts)| now.duration_since(*ts) < ttl);
            if self.result_cache.len() > CACHE_MAX_ENTRIES {
                let mut entries: Vec<((String, u64), Instant)> = self
                    .result_cache
                    .iter()
                    .map(|((k1, k2), (_v, ts))| ((k1.clone(), *k2), *ts))
                    .collect();
                entries.sort_by_key(|(_, ts)| *ts);
                let to_remove = entries.len() - CACHE_MAX_ENTRIES;
                for (key, _) in entries.into_iter().take(to_remove) {
                    self.result_cache.remove(&key);
                }
            }
        }
    }

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
        tokio::task::block_in_place(|| handle.block_on(recorder.record_start(&ctx)).ok())
    }

    fn record_result_sync(
        &self,
        execution_id: &Option<String>,
        is_success: bool,
        content: &str,
        duration_ms: i64,
    ) {
        let Some(recorder) = self.recorder.as_ref() else {
            return;
        };
        let Some(exec_id) = execution_id.as_deref() else {
            return;
        };
        let recorder = recorder.clone();
        let exec_id = exec_id.to_string();
        let content = content.to_string();
        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| {
            handle.block_on(async {
                if is_success {
                    let _ = recorder
                        .record_success(&exec_id, &content, Some(duration_ms))
                        .await;
                } else {
                    let _ = recorder
                        .record_error(&exec_id, &content, Some(duration_ms))
                        .await;
                }
            })
        });
    }
}

fn compute_hash(input: &str) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(input.as_bytes());
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_classification() {
        assert_eq!(classify_tool("read_file"), ToolCategory::ReadOnly);
        assert_eq!(classify_tool("list_directory"), ToolCategory::ReadOnly);
        assert_eq!(classify_tool("write_file"), ToolCategory::Write);
        assert_eq!(classify_tool("edit_file"), ToolCategory::Write);
        assert_eq!(classify_tool("run_command"), ToolCategory::Execute);
        assert_eq!(classify_tool("bash_shell"), ToolCategory::Execute);
        assert_eq!(classify_tool("fetch_url"), ToolCategory::Network);
        assert_eq!(classify_tool("web_search"), ToolCategory::ReadOnly);
    }

    #[test]
    fn test_tool_usage_stats() {
        let stats = ToolUsageStats::new();
        stats.record_execution("test_tool", ToolCategory::ReadOnly, 100.0, true);
        stats.record_execution("test_tool", ToolCategory::ReadOnly, 200.0, true);
        stats.record_execution("test_tool", ToolCategory::ReadOnly, 150.0, false);

        let metadata = stats.get_stats("test_tool").unwrap();
        assert_eq!(metadata.success_count, 2);
        assert_eq!(metadata.failure_count, 1);
        assert!((metadata.avg_execution_time_ms.unwrap() - 150.0).abs() < 0.1);
    }

    #[test]
    fn test_cache_eviction() {
        let mut registry = ToolRegistry::new();
        for i in 0..(CACHE_MAX_ENTRIES + 50) {
            let key = format!("tool_{}", i);
            let input_hash = compute_hash(&key);
            registry
                .result_cache
                .insert((key, input_hash), (format!("result_{}", i), Instant::now()));
        }
        registry.evict_cache_if_needed();
        assert!(registry.result_cache.len() <= CACHE_MAX_ENTRIES);
    }
}
