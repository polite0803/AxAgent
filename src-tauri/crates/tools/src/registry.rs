//! 统一工具注册表
//!
//! 管理所有已注册工具的生命周期：注册、查找、列举、启用/禁用。
//! 集成 MCP 执行、DB 审计记录、缓存、使用统计。

use crate::audit::{AuditEntry, ToolAuditor, shared_auditor};
use crate::hooks::executors::execute_hook;
use crate::hooks::registry::HookRegistry;
use crate::hooks::{HookAction, HookConfig, HookEventType};
use crate::orchestration::{Orchestrator, ToolCallRequest};
use crate::permissions::{PermissionMode, PermissionPolicy};
use crate::recorder::ToolExecutionRecorder;
use crate::stats::ToolUsageStats;
use crate::{Tool, ToolCategory, ToolError, ToolErrorKind, ToolInfo, ToolResult};
use axagent_runtime_core::ToolExecutor as RuntimeToolExecutor;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub type SkillToolHandler = Box<dyn FnMut(&str) -> Result<String, crate::ToolError> + Send>;

/// 工具组摘要信息（替代 agent::LocalToolGroup）
#[derive(Debug, Clone)]
pub struct ToolGroupInfo {
    pub group_id: String,
    pub group_name: String,
    pub enabled: bool,
    pub tools: Vec<ToolInfo>,
}

/// 统一工具注册表
///
/// 支持按名称、别名查找工具，按类别筛选，启用/禁用管理。
#[derive(Clone)]
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

const CACHE_TTL_SECS: u64 = 300;
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

// McpRegistry 已删除 — MCP 配置直接存储在 UnifiedToolRegistry.mcp_tools/.mcp_servers 中

/// 完整的统一工具注册表
#[derive(Clone)]
pub struct UnifiedToolRegistry {
    /// Tool trait 实现的工具（原生 + 已迁移旧工具）
    pub tools: ToolRegistry,
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
    /// 工具调用审计器
    pub auditor: Arc<ToolAuditor>,
    /// 结果缓存（待集成）
    result_cache: HashMap<(String, u64), (String, Instant)>,
    /// 权限控制
    allowed_tools: HashSet<String>,
    blocked_tools: HashSet<String>,
    strict_mode: bool,
    /// 会话上下文
    conversation_id: Option<String>,
    message_id: Option<String>,
    /// 当前工作目录（来自 agent session 的 workspace cwd）
    pub working_dir: String,
    /// 工具组启用状态（从 DB 加载）
    pub group_enabled: HashMap<String, bool>,
    /// 单个工具禁用列表（从 DB 加载，空=全部启用）
    pub disabled_tools: HashSet<String>,
    /// 工具组显示名称
    pub group_names: HashMap<String, String>,
    /// 搜索/网络配置（通过 ToolContext.extra 传递给工具）
    pub tool_extra: HashMap<String, String>,
}

impl UnifiedToolRegistry {
    /// 创建并初始化：自动注册全部本地工具（数量见 tools/mod.rs register_all()）
    pub fn new() -> Self {
        let mut reg = Self {
            tools: ToolRegistry::new(),
            mcp_tools: BTreeMap::new(),
            mcp_servers: BTreeMap::new(),
            recorder: None,
            usage_stats: ToolUsageStats::new(),
            permission_policy: PermissionPolicy::new(PermissionMode::WorkspaceWrite),
            hook_registry: HookRegistry::new(),
            auditor: shared_auditor(),
            result_cache: HashMap::new(),
            allowed_tools: HashSet::new(),
            blocked_tools: HashSet::new(),
            strict_mode: false,
            conversation_id: None,
            message_id: None,
            working_dir: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            group_enabled: HashMap::new(),
            disabled_tools: HashSet::new(),
            group_names: HashMap::new(),
            tool_extra: HashMap::new(),
        };
        reg.init_all();
        reg
    }

    /// 初始化：注册全部本地工具（约 138 个，来自 tools/ 下 43 个模块），配置默认权限
    pub fn init_all(&mut self) {
        // 第一层：注册全部本地 Rust Tool trait 实现
        crate::tools::register_all(&mut self.tools);

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

    /// 已启用工具总数（排除禁用的）
    pub fn count_enabled_tools(&self) -> u32 {
        self.tools
            .tools
            .iter()
            .filter(|(name, tool)| tool.is_enabled() && !self.disabled_tools.contains(*name))
            .count() as u32
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

    /// 设置工具执行时的工作目录（来自 agent session 的 workspace cwd）
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = dir.into();
        self
    }

    /// 设置工具执行时的额外上下文参数（如搜索提供商配置）
    /// 这些参数会通过 ToolContext.extra 传递给工具的 call() 方法
    pub fn with_tool_extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tool_extra.insert(key.into(), value.into());
        self
    }

    /// 批量设置工具执行时的额外上下文参数
    pub fn with_tool_extras(
        mut self,
        extras: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (k, v) in extras {
            self.tool_extra.insert(k.into(), v.into());
        }
        self
    }

    /// 从 DB 加载工具组启用状态及单工具禁用列表
    pub async fn load_enabled_state(&mut self, db: &sea_orm::DatabaseConnection) {
        use axagent_core::entity::settings;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        // 加载分类启用状态
        let key = "tool_groups_enabled";
        let result = settings::Entity::find()
            .filter(settings::Column::Key.eq(key))
            .one(db)
            .await;

        if let Ok(Some(record)) = result
            && let Ok(map) = serde_json::from_str::<HashMap<String, bool>>(&record.value)
        {
            self.group_enabled = map;
        }

        // 加载单工具禁用列表
        let dt_key = "disabled_tools";
        let dt_result = settings::Entity::find()
            .filter(settings::Column::Key.eq(dt_key))
            .one(db)
            .await;

        if let Ok(Some(record)) = dt_result
            && let Ok(list) = serde_json::from_str::<Vec<String>>(&record.value)
        {
            self.disabled_tools = list.into_iter().collect();
        }

        // 初始化默认组名
        let default_groups: Vec<(&str, &str)> = vec![
            ("builtin-file-read", "文件读取"),
            ("builtin-file-write", "文件写入"),
            ("builtin-shell", "Shell 命令"),
            ("builtin-network", "网络请求"),
            ("builtin-system-tools", "系统工具"),
            ("builtin-agent", "Agent 工具"),
            ("builtin-vcs", "版本控制"),
            ("builtin-automation", "自动化"),
            ("builtin-communication", "通信"),
            ("builtin-ai-media", "AI 媒体"),
            ("builtin-integration", "外部集成"),
            ("builtin-storage", "存储管理"),
            ("builtin-knowledge", "知识库"),
            ("builtin-browser", "浏览器"),
            ("builtin-desktop", "桌面控制"),
        ];
        for (gid, gname) in &default_groups {
            self.group_names
                .entry(gid.to_string())
                .or_insert_with(|| gname.to_string());
        }
    }

    /// 获取工具组列表
    pub fn get_tool_groups(&self) -> Vec<ToolGroupInfo> {
        let mut groups_map: HashMap<String, (String, bool, Vec<ToolInfo>)> = HashMap::new();
        for info in self.tools.list_all() {
            let gid = match info.category {
                ToolCategory::FileRead => "builtin-file-read",
                ToolCategory::FileWrite => "builtin-file-write",
                ToolCategory::Shell => "builtin-shell",
                ToolCategory::Network => "builtin-network",
                ToolCategory::System => "builtin-system-tools",
                ToolCategory::Agent => "builtin-agent",
                ToolCategory::Vcs => "builtin-vcs",
                ToolCategory::Automation => "builtin-automation",
                ToolCategory::Communication => "builtin-communication",
                ToolCategory::AiMedia => "builtin-ai-media",
                ToolCategory::Integration => "builtin-integration",
                ToolCategory::Storage => "builtin-storage",
                ToolCategory::Knowledge => "builtin-knowledge",
                ToolCategory::Browser => "builtin-browser",
                ToolCategory::Desktop => "builtin-desktop",
            };
            let entry = groups_map.entry(gid.to_string()).or_insert_with(|| {
                let name = self
                    .group_names
                    .get(gid)
                    .cloned()
                    .unwrap_or_else(|| gid.to_string());
                let enabled = self.group_enabled.get(gid).copied().unwrap_or(true);
                (name, enabled, Vec::new())
            });
            entry.2.push(info);
        }
        let mut groups: Vec<ToolGroupInfo> = groups_map
            .into_iter()
            .map(|(gid, (name, enabled, tools))| ToolGroupInfo {
                group_id: gid,
                group_name: name,
                enabled,
                tools,
            })
            .collect();
        groups.sort_by_key(|g| g.group_id.clone());
        groups
    }

    /// 切换工具组启用状态并持久化到 DB
    pub async fn toggle_group(
        &mut self,
        db: &sea_orm::DatabaseConnection,
        gid: &str,
    ) -> Result<bool, String> {
        use axagent_core::entity::settings;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        let current = self.group_enabled.get(gid).copied().unwrap_or(true);
        let new_state = !current;
        self.group_enabled.insert(gid.to_string(), new_state);

        let key = "tool_groups_enabled";
        let existing = settings::Entity::find()
            .filter(settings::Column::Key.eq(key))
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        let serialized = serde_json::to_string(&self.group_enabled).map_err(|e| e.to_string())?;

        match existing {
            Some(record) => {
                let mut active: settings::ActiveModel = record.into();
                active.value = Set(serialized);
                active.update(db).await.map_err(|e| e.to_string())?;
            },
            None => {
                let active = settings::ActiveModel {
                    key: Set(key.to_string()),
                    value: Set(serialized),
                };
                active.insert(db).await.map_err(|e| e.to_string())?;
            },
        }

        Ok(new_state)
    }

    /// 切换单个工具启用状态并持久化到 DB
    pub async fn toggle_tool(
        &mut self,
        db: &sea_orm::DatabaseConnection,
        tool_name: &str,
    ) -> Result<bool, String> {
        use axagent_core::entity::settings;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        let currently_disabled = self.disabled_tools.contains(tool_name);
        if currently_disabled {
            self.disabled_tools.remove(tool_name);
        } else {
            self.disabled_tools.insert(tool_name.to_string());
        }

        let key = "disabled_tools";
        let existing = settings::Entity::find()
            .filter(settings::Column::Key.eq(key))
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        let serialized = serde_json::to_string(&self.disabled_tools.iter().collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;

        match existing {
            Some(record) => {
                let mut active: settings::ActiveModel = record.into();
                active.value = Set(serialized);
                active.update(db).await.map_err(|e| e.to_string())?;
            },
            None => {
                let active = settings::ActiveModel {
                    key: Set(key.to_string()),
                    value: Set(serialized),
                };
                active.insert(db).await.map_err(|e| e.to_string())?;
            },
        }

        Ok(!currently_disabled)
    }

    /// 获取所有已启用工具名称
    pub fn enabled_tool_names(&self) -> Vec<String> {
        self.tools
            .list_all()
            .into_iter()
            .filter(|info| {
                let gid = match info.category {
                    ToolCategory::FileRead => "builtin-file-read",
                    ToolCategory::FileWrite => "builtin-file-write",
                    ToolCategory::Shell => "builtin-shell",
                    ToolCategory::Network => "builtin-network",
                    ToolCategory::System => "builtin-system-tools",
                    ToolCategory::Agent => "builtin-agent",
                    ToolCategory::Vcs => "builtin-vcs",
                    ToolCategory::Automation => "builtin-automation",
                    ToolCategory::Communication => "builtin-communication",
                    ToolCategory::AiMedia => "builtin-ai-media",
                    ToolCategory::Integration => "builtin-integration",
                    ToolCategory::Storage => "builtin-storage",
                    ToolCategory::Knowledge => "builtin-knowledge",
                    ToolCategory::Browser => "builtin-browser",
                    ToolCategory::Desktop => "builtin-desktop",
                };
                self.group_enabled.get(gid).copied().unwrap_or(true)
            })
            .map(|info| info.name)
            .collect()
    }

    pub fn register_skill_tool(self, _name: impl Into<String>, _handler: SkillToolHandler) -> Self {
        self
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
        names.extend(self.mcp_tools.values().map(|c| c.tool_name.clone()));
        names
    }

    /// 执行工具（统一入口，集成权限 + Hook）
    pub async fn execute(
        &mut self,
        tool_name: &str,
        input: &str,
    ) -> Result<ToolResult, crate::ToolError> {
        // ── 频率限制检查（审计器） ──
        if let Err(rate_limit_msg) = self.auditor.check_rate_limit(tool_name).await {
            return Err(ToolError::permission_denied(tool_name, &rate_limit_msg));
        }

        // ── 输入脱敏 ──
        let sanitized_input = self.auditor.sanitize_input(input);

        // ── 权限检查（集成 PermissionPolicy） ──
        let decision = self
            .permission_policy
            .authorize(tool_name, &sanitized_input);
        if decision.is_denied() {
            return Err(ToolError::permission_denied(tool_name, &decision.reason));
        }

        // 简单黑白名单检查（兼容旧逻辑）
        if !self.is_allowed(tool_name) {
            return Err(ToolError::permission_denied(tool_name, "工具被黑白名单策略阻止"));
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

        self.result_cache
            .retain(|_, (_, inserted)| inserted.elapsed().as_secs() < CACHE_TTL_SECS);
        if self.result_cache.len() > CACHE_MAX_ENTRIES {
            let mut entries: Vec<_> = self.result_cache.iter().collect();
            entries.sort_by_key(|(_, (_, t))| *t);
            let keys_to_remove: Vec<_> = entries
                .into_iter()
                .take(self.result_cache.len() - CACHE_MAX_ENTRIES)
                .map(|(k, _)| k.clone())
                .collect();
            for k in keys_to_remove {
                self.result_cache.remove(&k);
            }
        }

        let result = {
            // 1. 尝试新体系工具
            if let Some(tool) = self.tools.find(tool_name) {
                let input_val: Value =
                    serde_json::from_str(&effective_input).unwrap_or(Value::Null);
                let ctx = crate::ToolContext {
                    working_dir: self.working_dir.clone(),
                    conversation_id: self.conversation_id.clone(),
                    message_id: self.message_id.clone(),
                    allow_write: true,
                    allow_execute: true,
                    allow_network: true,
                    abort_signal: None,
                    extra: self.tool_extra.clone(),
                };

                match tool.call(input_val, &ctx).await {
                    Ok(mut r) => {
                        r.duration_ms = Some(start.elapsed().as_millis() as u64);
                        Ok(r)
                    },
                    Err(e) => Err(e),
                }
            }
            // 2. 尝试 MCP 工具
            else if self.mcp_tools.values().any(|c| c.tool_name == tool_name) {
                self.execute_mcp(tool_name, input).await
            } else {
                Err(ToolError::not_found(tool_name))
            }
        };

        // ── 审计日志记录 ──
        let duration_ms = start.elapsed().as_millis() as u64;
        let success = result.is_ok();
        let output_content = result.as_ref().map(|r| &r.content).map(|c| {
            if c.len() > 200 {
                format!("{}...", &c[..200])
            } else {
                c.clone()
            }
        });
        let has_sensitive_output = output_content
            .as_ref()
            .map(|c| self.auditor.scan_output(c))
            .unwrap_or(false);
        let has_sensitive_input = input != sanitized_input;

        self.auditor
            .log(AuditEntry {
                timestamp: chrono::Utc::now().timestamp_millis(),
                tool_name: tool_name.to_string(),
                conversation_id: self.conversation_id.clone(),
                success,
                duration_ms,
                output_preview: output_content.unwrap_or_default(),
                has_sensitive_input,
                has_sensitive_output,
            })
            .await;

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
                format!("tool_name={}, input={}, output={}", tool_name, effective_input, out)
            } else {
                format!("tool_name={}, input={}", tool_name, effective_input)
            };
            execute_hook(hook, tool_name, &exec_input).await;
        }

        result
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
        let started = std::time::Instant::now();

        // 准备传输参数
        let transport = server.transport.as_str();
        let command = server.command.as_deref();
        let args: Option<Vec<String>> = server
            .args_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        let env: Option<HashMap<String, String>> = server
            .env_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        let endpoint = server.endpoint.as_deref();

        // 使用统一的 MCP 客户端入口
        let result = tokio::time::timeout(
            timeout,
            axagent_core::mcp_client::call_tool_unified(
                transport,
                command,
                args.as_deref(),
                env.as_ref(),
                endpoint,
                tool_name,
                arguments,
            ),
        )
        .await;

        let duration_ms: u64 = started.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(mcp_result)) => {
                // 将 MCP 进度条目转换为 ToolResult 进度
                let progress: Vec<crate::ProgressEntry> = mcp_result
                    .progress
                    .iter()
                    .map(|p| crate::ProgressEntry {
                        phase: p.phase.clone(),
                        message: p.message.clone(),
                        percent: p.percent,
                        timestamp_ms: 0,
                    })
                    .collect();

                let tool_result = ToolResult {
                    content: mcp_result.content.clone(),
                    truncated: false,
                    is_error: mcp_result.is_error,
                    metadata: None,
                    duration_ms: Some(duration_ms),
                    progress,
                };

                // 写入执行记录
                if let Some(ref recorder) = self.recorder {
                    let input_preview = truncate_str(input, 200);
                    let _ = recorder
                        .record_start(
                            "", // conversation_id 由调用方设置
                            None,
                            &config.server_id,
                            tool_name,
                            Some(&input_preview),
                        )
                        .await;
                }

                Ok(tool_result)
            },
            Ok(Err(e)) => {
                let err_msg = format!("MCP 工具调用失败: {e}");
                Err(ToolError::execution_failed_for(tool_name, err_msg))
            },
            Err(_) => Err(ToolError {
                error_code: format!("tool.{}.timeout", tool_name),
                message: format!("MCP 工具 '{}' 执行超时（{} 秒）", tool_name, timeout.as_secs()),
                kind: ToolErrorKind::Timeout,
            }),
        }
    }
}

/// 截断字符串到指定长度，用于输入预览
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
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
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        if !self.is_allowed(tool_name) {
            return Err(ToolError::new(format!("Tool '{}' denied", tool_name)));
        }

        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| {
            handle.block_on(async {
                match self.execute(tool_name, input).await {
                    Ok(r) => Ok(r.content),
                    Err(e) => Err(e),
                }
            })
        })
    }

    fn execute_batch(
        &mut self,
        requests: &[(String, String, String)],
    ) -> Vec<(String, String, Result<String, ToolError>)> {
        use std::sync::Arc;

        let handle = tokio::runtime::Handle::current();
        let tool_requests: Vec<ToolCallRequest> = requests
            .iter()
            .map(|(id, name, input)| ToolCallRequest {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            })
            .collect();

        let ctx = crate::ToolContext {
            working_dir: self.working_dir.clone(),
            conversation_id: None,
            message_id: None,
            allow_write: true,
            allow_execute: true,
            allow_network: true,
            abort_signal: None,
            extra: std::collections::HashMap::new(),
        };

        let orchestrator = Orchestrator::default();
        let registry = Arc::new(self.tools.clone());

        let results: Vec<_> = tokio::task::block_in_place(|| {
            handle.block_on(async { orchestrator.execute(tool_requests, registry, &ctx).await })
        });

        results
            .into_iter()
            .map(|r| {
                let output = match r.result {
                    Ok(tr) => Ok(tr.content),
                    Err(e) => Err(e),
                };
                (r.id, r.name, output)
            })
            .collect()
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
