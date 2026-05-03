//! 统一工具注册表
//!
//! 管理所有已注册工具的生命周期：注册、查找、列举、启用/禁用。
//! 集成 MCP 执行、DB 审计记录、缓存、使用统计。

use crate::builtin_tools;
use crate::hooks::executors::execute_hook;
use crate::hooks::registry::HookRegistry;
use crate::hooks::{HookAction, HookConfig, HookEventType};
use crate::permissions::{PermissionMode, PermissionPolicy};
use crate::recorder::ToolExecutionRecorder;
use crate::stats::ToolUsageStats;
use crate::{Tool, ToolCategory, ToolError, ToolErrorKind, ToolInfo, ToolResult};
use axagent_runtime::ToolError as RuntimeToolError;
use axagent_runtime::ToolExecutor as RuntimeToolExecutor;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 统一工具注册表
///
/// 支持按名称、别名查找工具，按类别筛选，启用/禁用管理。
pub struct ToolRegistry {
    /// 工具名 -> 工具实例
    tools: HashMap<String, Arc<dyn Tool>>,
    /// 别名 -> 主名
    aliases: HashMap<String, String>,
    /// 禁用列表
    disabled: std::collections::HashSet<String>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            aliases: HashMap::new(),
            disabled: std::collections::HashSet::new(),
        }
    }

    /// 注册一个工具
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();

        // 注册别名映射
        for alias in tool.aliases() {
            self.aliases.insert(alias.to_string(), name.clone());
        }

        self.tools.insert(name, tool);
    }

    /// 批量注册
    pub fn register_all(&mut self, tools: Vec<Arc<dyn Tool>>) {
        for tool in tools {
            self.register(tool);
        }
    }

    /// 查找工具（支持别名匹配）
    pub fn find(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        // 先按主名查找
        if let Some(tool) = self.tools.get(name) {
            return Some(tool);
        }
        // 再按别名查找
        if let Some(primary) = self.aliases.get(name) {
            return self.tools.get(primary);
        }
        None
    }

    /// 按类别筛选工具
    pub fn by_category(&self, category: ToolCategory) -> Vec<&Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|t| t.category() == category && t.is_enabled())
            .collect()
    }

    /// 列出所有已启用工具的信息
    pub fn list_all(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .filter(|t| t.is_enabled())
            .map(|t| ToolInfo::from_tool(t.as_ref()))
            .collect()
    }

    /// 列出所有工具（含禁用）
    pub fn list_all_with_disabled(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|t| ToolInfo::from_tool(t.as_ref()))
            .collect()
    }

    /// 获取只读工具列表
    pub fn read_only_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .filter(|t| t.is_read_only() && t.is_enabled())
            .map(|t| ToolInfo::from_tool(t.as_ref()))
            .collect()
    }

    /// 获取可并发工具列表
    pub fn concurrency_safe_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .filter(|t| t.is_concurrency_safe() && t.is_enabled())
            .map(|t| ToolInfo::from_tool(t.as_ref()))
            .collect()
    }

    /// 禁用工具
    pub fn disable(&mut self, name: &str) {
        self.disabled.insert(name.to_string());
    }

    /// 启用工具
    pub fn enable(&mut self, name: &str) {
        self.disabled.remove(name);
    }

    /// 批量按类别禁用
    pub fn disable_category(&mut self, category: ToolCategory) {
        for tool in self.tools.values() {
            if tool.category() == category {
                self.disabled.insert(tool.name().to_string());
            }
        }
    }

    /// 是否已注册
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name) || self.aliases.contains_key(name)
    }

    /// 工具总数
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// 移除工具
    pub fn unregister(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        // 清理别名
        self.aliases.retain(|_, v| v != name);
        self.disabled.remove(name);
        self.tools.remove(name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 工具注册表构建器，方便链式注册
pub struct ToolRegistryBuilder {
    registry: ToolRegistry,
}

impl ToolRegistryBuilder {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    pub fn register(mut self, tool: impl Tool + 'static) -> Self {
        self.registry.register(Arc::new(tool));
        self
    }

    pub fn build(self) -> ToolRegistry {
        self.registry
    }
}

impl Default for ToolRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 将已有工具列表转为 JSON Schema 格式（供 LLM 使用）
pub fn tools_to_anthropic_format(tools: &[ToolInfo]) -> serde_json::Value {
    let items: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect();

    serde_json::Value::Array(items)
}

/// 将已有工具列表转为 OpenAI 格式
pub fn tools_to_openai_format(tools: &[ToolInfo]) -> serde_json::Value {
    let items: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                }
            })
        })
        .collect();

    serde_json::Value::Array(items)
}

// ============================================================
// 统一 ToolRegistry（含 MCP + 缓存 + 审计 + 统计）
// ============================================================

#[allow(dead_code)]
const CACHE_TTL_SECS: u64 = 300;
#[allow(dead_code)]
const CACHE_MAX_ENTRIES: usize = 200;

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

#[derive(Debug, Clone)]
pub struct McpToolConfig {
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

/// MCP 注册表
#[derive(Clone)]
pub struct McpRegistry {
    pub tools: BTreeMap<String, McpToolConfig>,
    pub servers: BTreeMap<String, McpServerConfig>,
}
impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
            servers: BTreeMap::new(),
        }
    }
    pub fn execute_mcp_tool(
        &self,
        tool_name: &str,
        input: &str,
    ) -> Result<String, crate::ToolError> {
        let config = self
            .tools
            .values()
            .find(|c| c.tool_name == tool_name)
            .ok_or_else(|| crate::ToolError::not_found(tool_name))?;
        let server = self.servers.get(&config.server_id).ok_or_else(|| {
            crate::ToolError::execution_failed_for("McpRegistry", "MCP server not found")
        })?;
        let args: Vec<String> = server
            .args_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        let env: std::collections::HashMap<String, String> = server
            .env_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        let arguments: Value = serde_json::from_str(input).unwrap_or(Value::Null);
        let rt = tokio::runtime::Handle::current();
        rt.block_on(axagent_core::mcp_client::call_tool_stdio_pooled(
            server.command.as_deref().unwrap_or("npx"),
            &args,
            &env,
            tool_name,
            arguments,
        ))
        .map(|r| r.content)
        .map_err(|e| crate::ToolError::execution_failed(format!("MCP call failed: {}", e)))
    }
}

/// 完整的统一工具注册表
pub struct UnifiedToolRegistry {
    /// 新体系：Tool trait 实现的工具
    pub tools: ToolRegistry,
    /// 旧体系：内置处理器（来自 builtin_tools_registry）
    pub builtin_defs: HashMap<String, (String, Value)>, // tool_name → (server_name, input_schema)
    /// MCP 工具
    pub mcp_tools: BTreeMap<String, McpToolConfig>,
    pub mcp_servers: BTreeMap<String, McpServerConfig>,
    /// 执行记录器
    pub recorder: Option<ToolExecutionRecorder>,
    /// 使用统计
    pub usage_stats: ToolUsageStats,
    /// 权限策略（集成到执行路径）
    pub permission_policy: PermissionPolicy,
    /// Hook 注册表（集成到执行路径）
    pub hook_registry: HookRegistry,
    /// 结果缓存（待集成）
    #[allow(dead_code)]
    result_cache: HashMap<(String, u64), (String, Instant)>,
    /// 权限控制
    allowed_tools: HashSet<String>,
    blocked_tools: HashSet<String>,
    strict_mode: bool,
    /// 会话上下文
    conversation_id: Option<String>,
    message_id: Option<String>,
}

impl UnifiedToolRegistry {
    /// 创建并初始化：自动注册全部 52 个工具 + 旧 builtin 定义
    pub fn new() -> Self {
        let mut reg = Self {
            tools: ToolRegistry::new(),
            builtin_defs: HashMap::new(),
            mcp_tools: BTreeMap::new(),
            mcp_servers: BTreeMap::new(),
            recorder: None,
            usage_stats: ToolUsageStats::new(),
            permission_policy: PermissionPolicy::new(PermissionMode::WorkspaceWrite),
            hook_registry: HookRegistry::new(),
            result_cache: HashMap::new(),
            allowed_tools: HashSet::new(),
            blocked_tools: HashSet::new(),
            strict_mode: false,
            conversation_id: None,
            message_id: None,
        };
        reg.init_all();
        reg
    }

    /// 初始化：加载新旧所有工具，配置默认权限
    pub fn init_all(&mut self) {
        // 注册新工具
        crate::tools::register_all(&mut self.tools);

        // 加载旧 builtin 工具定义
        for ft in builtin_tools::get_all_builtin_tools_flat() {
            self.builtin_defs.insert(
                ft.tool_name.clone(),
                (ft.server_name.clone(), ft.input_schema.clone()),
            );
        }

        // 配置默认工具级权限要求
        self.permission_policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite)
            .with_tool_requirement("FileRead", PermissionMode::ReadOnly)
            .with_tool_requirement("Glob", PermissionMode::ReadOnly)
            .with_tool_requirement("Grep", PermissionMode::ReadOnly)
            .with_tool_requirement("WebFetch", PermissionMode::ReadOnly)
            .with_tool_requirement("WebSearch", PermissionMode::ReadOnly)
            .with_tool_requirement("FileWrite", PermissionMode::WorkspaceWrite)
            .with_tool_requirement("FileEdit", PermissionMode::WorkspaceWrite)
            .with_tool_requirement("Bash", PermissionMode::DangerFullAccess)
            .with_tool_requirement("NotebookEdit", PermissionMode::WorkspaceWrite)
            .with_tool_requirement("ComputerUse", PermissionMode::DangerFullAccess);
    }

    pub fn with_recorder(mut self, recorder: ToolExecutionRecorder) -> Self {
        self.recorder = Some(recorder);
        self
    }

    pub fn with_context(mut self, conversation_id: String, message_id: Option<String>) -> Self {
        self.conversation_id = Some(conversation_id);
        self.message_id = message_id;
        self
    }

    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools.into_iter().collect();
        self
    }

    pub fn with_blocked_tools(mut self, tools: Vec<String>) -> Self {
        self.blocked_tools = tools.into_iter().collect();
        self
    }

    fn is_allowed(&self, tool_name: &str) -> bool {
        if self.blocked_tools.contains(tool_name) {
            return false;
        }
        if self.strict_mode && !self.allowed_tools.is_empty() {
            return self.allowed_tools.contains(tool_name);
        }
        true
    }

    /// 将所有已注册工具转为 ChatTool 格式（供 LLM 使用）
    pub fn get_chat_tools(&self) -> Vec<axagent_core::types::ChatTool> {
        let mut out = Vec::new();
        for info in self.tools.list_all() {
            out.push(axagent_core::types::ChatTool {
                r#type: "function".into(),
                function: axagent_core::types::ChatToolFunction {
                    name: info.name.clone(),
                    description: Some(info.description.clone()),
                    parameters: Some(info.input_schema.clone()),
                },
            });
        }
        out
    }

    /// 获取类别筛选后的 ChatTool 列表（用于根据 permission mode 限制工具）
    pub fn get_chat_tools_filtered(
        &self,
        mode: &crate::permissions::PermissionMode,
    ) -> Vec<axagent_core::types::ChatTool> {
        let mut out = Vec::new();
        for info in self.tools.list_all() {
            let allowed = match mode {
                crate::permissions::PermissionMode::ReadOnly => info.is_read_only,
                crate::permissions::PermissionMode::Allow => true,
                crate::permissions::PermissionMode::DangerFullAccess => true,
                crate::permissions::PermissionMode::WorkspaceWrite => true,
                crate::permissions::PermissionMode::Prompt => true,
            };
            if allowed {
                out.push(axagent_core::types::ChatTool {
                    r#type: "function".into(),
                    function: axagent_core::types::ChatToolFunction {
                        name: info.name.clone(),
                        description: Some(info.description.clone()),
                        parameters: Some(info.input_schema.clone()),
                    },
                });
            }
        }
        out
    }

    // ── 兼容旧 API ──

    pub fn list_tools(&self) -> Vec<String> {
        self.list_all_tool_names()
    }

    pub fn with_execution_context(mut self, conv_id: String, msg_id: Option<String>) -> Self {
        self.conversation_id = Some(conv_id);
        self.message_id = msg_id;
        self
    }

    pub fn with_local_tools<T>(self, _local_tools: T) -> Self {
        self
    }

    #[allow(clippy::type_complexity)]
    pub fn register_skill_tool(
        self,
        _name: impl Into<String>,
        _handler: Box<dyn FnMut(&str) -> Result<String, crate::ToolError> + Send>,
    ) -> Self {
        self
    }

    pub fn mcp_registry(&self) -> McpRegistry {
        McpRegistry {
            tools: self.mcp_tools.clone(),
            servers: self.mcp_servers.clone(),
        }
    }

    pub fn register_mcp_tool(
        mut self,
        server_id: String,
        server_name: String,
        tool_name: String,
        description: Option<String>,
        input_schema: Option<Value>,
        server_config: McpServerConfig,
    ) -> Self {
        self.mcp_tools.insert(
            format!("{}/{}", server_id, tool_name),
            McpToolConfig {
                server_id: server_id.clone(),
                server_name,
                tool_name,
                description,
                input_schema,
            },
        );
        self.mcp_servers.insert(server_id, server_config);
        self
    }

    /// 列出所有已注册工具名
    pub fn list_all_tool_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .tools
            .list_all()
            .into_iter()
            .map(|t| t.name.clone())
            .collect();
        names.extend(self.builtin_defs.keys().cloned());
        names.extend(self.mcp_tools.values().map(|c| c.tool_name.clone()));
        names
    }

    /// 执行工具（统一入口，集成权限 + Hook）
    pub async fn execute(
        &mut self,
        tool_name: &str,
        input: &str,
    ) -> Result<ToolResult, crate::ToolError> {
        // ── 权限检查（集成 PermissionPolicy） ──
        let decision = self.permission_policy.authorize(tool_name, input);
        if decision.is_denied() {
            return Err(ToolError::permission_denied(tool_name, &decision.reason));
        }

        // 简单黑白名单检查（兼容旧逻辑）
        if !self.is_allowed(tool_name) {
            return Err(ToolError::permission_denied(
                tool_name,
                "工具被黑白名单策略阻止",
            ));
        }

        // ── PreToolUse Hooks ──
        let pre_hooks: Vec<HookConfig> = self
            .hook_registry
            .get_matching(&HookEventType::PreToolUse, tool_name)
            .into_iter()
            .cloned()
            .collect();
        let mut effective_input = input.to_string();
        for hook in &pre_hooks {
            let result = execute_hook(hook, tool_name, &effective_input).await;
            if result.action == HookAction::Deny {
                return Err(ToolError::permission_denied(
                    tool_name,
                    &result
                        .reason
                        .unwrap_or_else(|| "PreToolUse Hook 拒绝执行".into()),
                ));
            }
            if let Some(ref modified) = result.modified_input {
                effective_input = modified.to_string();
            }
        }

        let start = Instant::now();

        // 实际执行
        let result = {
            // 1. 尝试新体系工具
            if let Some(tool) = self.tools.find(tool_name) {
                let input_val: Value =
                    serde_json::from_str(&effective_input).unwrap_or(Value::Null);
                let ctx = crate::ToolContext {
                    working_dir: std::env::current_dir()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    conversation_id: self.conversation_id.clone(),
                    message_id: self.message_id.clone(),
                    allow_write: true,
                    allow_execute: true,
                    allow_network: true,
                    abort_signal: None,
                    extra: HashMap::new(),
                };

                match tool.call(input_val, &ctx).await {
                    Ok(mut r) => {
                        r.duration_ms = Some(start.elapsed().as_millis() as u64);
                        Ok(r)
                    },
                    Err(e) => Err(e),
                }
            }
            // 2. 尝试旧内置处理器
            else if self.builtin_defs.contains_key(tool_name) {
                self.execute_builtin(tool_name, input).await
            }
            // 3. 尝试 MCP 工具
            else if self.mcp_tools.values().any(|c| c.tool_name == tool_name) {
                self.execute_mcp(tool_name, input).await
            } else {
                Err(ToolError::not_found(tool_name))
            }
        };

        // ── PostToolUse / PostToolUseFailure Hooks ──
        let is_error = result.is_err();
        let event_type = if is_error {
            &HookEventType::PostToolUseFailure
        } else {
            &HookEventType::PostToolUse
        };
        let output = result.as_ref().map(|r| &r.content).ok();
        let post_hooks: Vec<HookConfig> = self
            .hook_registry
            .get_matching(event_type, tool_name)
            .into_iter()
            .cloned()
            .collect();
        for hook in &post_hooks {
            let exec_input = if let Some(out) = output {
                format!(
                    "tool_name={}, input={}, output={}",
                    tool_name, effective_input, out
                )
            } else {
                format!("tool_name={}, input={}", tool_name, effective_input)
            };
            execute_hook(hook, tool_name, &exec_input).await;
        }

        result
    }

    async fn execute_builtin(&self, tool_name: &str, input: &str) -> Result<ToolResult, ToolError> {
        let (_server_name, _) = self
            .builtin_defs
            .get(tool_name)
            .ok_or_else(|| ToolError::not_found(tool_name))?;

        let handler = builtin_tools::get_handler("@axagent/search-file", tool_name)
            .or_else(|| builtin_tools::get_handler("@axagent/filesystem", tool_name))
            .or_else(|| builtin_tools::get_handler("@axagent/system", tool_name))
            .or_else(|| builtin_tools::get_handler("@axagent/fetch", tool_name))
            .or_else(|| builtin_tools::get_handler("@axagent/knowledge", tool_name))
            .or_else(|| builtin_tools::get_handler("@axagent/storage", tool_name))
            .or_else(|| builtin_tools::get_handler("@axagent/computer-control", tool_name));

        match handler {
            Some(h) => {
                let input_val: Value = serde_json::from_str(input).unwrap_or(Value::Null);
                let result = h(input_val)
                    .await
                    .map_err(|e| ToolError::execution_failed(e.to_string()))?;
                Ok(ToolResult {
                    content: result.content,
                    truncated: false,
                    is_error: result.is_error,
                    metadata: None,
                    duration_ms: None,
                })
            },
            None => Err(ToolError::not_found(tool_name)),
        }
    }

    pub async fn execute_mcp(
        &self,
        tool_name: &str,
        input: &str,
    ) -> Result<ToolResult, crate::ToolError> {
        let config = self
            .mcp_tools
            .values()
            .find(|c| c.tool_name == tool_name)
            .ok_or_else(|| ToolError::not_found(tool_name))?;

        let server = self.mcp_servers.get(&config.server_id).ok_or_else(|| {
            ToolError::execution_failed(format!("MCP server '{}' 未找到", config.server_id))
        })?;

        let arguments: Value = serde_json::from_str(input).unwrap_or(Value::Null);
        let timeout = server.get_timeout();

        let result = tokio::time::timeout(timeout, async {
            match server.transport.as_str() {
                "stdio" => {
                    let cmd = server.command.clone().unwrap_or_default();
                    let args: Vec<String> = server
                        .args_json
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default();
                    let env: HashMap<String, String> = server
                        .env_json
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default();

                    axagent_core::mcp_client::call_tool_stdio_pooled(
                        &cmd, &args, &env, tool_name, arguments,
                    )
                    .await
                    .map(|r| ToolResult {
                        content: r.content,
                        truncated: false,
                        is_error: r.is_error,
                        metadata: None,
                        duration_ms: None,
                    })
                    .map_err(|e| ToolError::execution_failed(e.to_string()))
                },
                "http" => {
                    let endpoint = server.endpoint.clone().unwrap_or_default();
                    axagent_core::mcp_client::call_tool_http(&endpoint, tool_name, arguments)
                        .await
                        .map(|r| ToolResult {
                            content: r.content,
                            truncated: false,
                            is_error: r.is_error,
                            metadata: None,
                            duration_ms: None,
                        })
                        .map_err(|e| ToolError::execution_failed(e.to_string()))
                },
                "sse" => {
                    let endpoint = server.endpoint.clone().unwrap_or_default();
                    axagent_core::mcp_client::call_tool_sse(&endpoint, tool_name, arguments)
                        .await
                        .map(|r| ToolResult {
                            content: r.content,
                            truncated: false,
                            is_error: r.is_error,
                            metadata: None,
                            duration_ms: None,
                        })
                        .map_err(|e| ToolError::execution_failed(e.to_string()))
                },
                other => Err(ToolError::execution_failed(format!(
                    "不支持的传输方式: {}",
                    other
                ))),
            }
        })
        .await;

        match result {
            Ok(r) => r,
            Err(_) => Err(ToolError {
                error_code: format!("tool.{}.timeout", tool_name),
                message: format!("MCP 工具 '{}' 执行超时", tool_name),
                kind: ToolErrorKind::Timeout,
            }),
        }
    }
}

impl Default for UnifiedToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// ToolExecutor trait 实现（兼容 ConversationRuntime）
// ============================================================

impl RuntimeToolExecutor for UnifiedToolRegistry {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, RuntimeToolError> {
        if !self.is_allowed(tool_name) {
            return Err(RuntimeToolError::new(format!(
                "Tool '{}' denied",
                tool_name
            )));
        }

        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| {
            handle.block_on(async {
                match self.execute(tool_name, input).await {
                    Ok(r) => Ok(r.content),
                    Err(e) => Err(RuntimeToolError::new(e.to_string())),
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ToolCategory, ToolContext};
    use async_trait::async_trait;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo back the input"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"]
            })
        }
        fn category(&self) -> ToolCategory {
            ToolCategory::System
        }
        fn aliases(&self) -> &[&str] {
            &["echo_test"]
        }

        async fn call(
            &self,
            input: serde_json::Value,
            _ctx: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            let msg = input["message"].as_str().unwrap_or("hello");
            Ok(ToolResult::success(msg))
        }
    }

    #[tokio::test]
    async fn test_registry_register_and_find() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        assert!(registry.contains("echo"));
        assert!(registry.contains("echo_test")); // alias

        let tool = registry.find("echo").unwrap();
        assert_eq!(tool.name(), "echo");
    }

    #[tokio::test]
    async fn test_registry_alias_resolution() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        let by_alias = registry.find("echo_test").unwrap();
        assert_eq!(by_alias.name(), "echo");
    }
}
