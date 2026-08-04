// SPDX-License-Identifier: AGPL-3.0-only

//! Tauri 命令桥接器 — 将现有 Tauri 命令注册为 Agent 可调用的 Tool
//!
//! 设计原则：
//! - 只读命令直接暴露给 Agent
//! - 写入命令需要人工确认（前端通过 AgentContext 感知）
//! - 直接调用 DAO 层，避免不必要的序列化/反序列化
//! - 通过 SkillToolHandler 机制注册到 UnifiedToolRegistry
//!
//! Phase 2 改进：
//! - 引入 DomainMapping 配置结构，支持灵活的领域映射
//! - 动态生成命令索引，支持运行时配置
//! - 完善安全分级校验逻辑

use agent_command_types;
use axagent_harness::path_vars::PathEncoder;
use axagent_harness::types::{ChatTool, ChatToolFunction};
use axagent_tools::registry::SkillToolHandler;
use sea_orm::DatabaseConnection;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tracing::{debug, instrument, warn};

/// Tauri 命令工具的元数据定义
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TauriCommandToolDef {
    /// 工具名称
    pub name: &'static str,
    /// 工具描述（给 LLM 看）
    pub description: &'static str,
    /// 输入参数的 JSON Schema
    pub input_schema: Value,
    /// 是否只读操作
    pub is_read_only: bool,
}

/// 命令安全级别
#[derive(Debug, Clone, PartialEq)]
pub enum CommandSafety {
    /// 只读查询，Agent 可直接调用
    Safe,
    /// 写入操作，需用户确认
    Caution,
    /// 危险操作，需显式授权
    Dangerous,
}

impl CommandSafety {
    pub fn as_str(&self) -> &str {
        match self {
            CommandSafety::Safe => "safe",
            CommandSafety::Caution => "caution",
            CommandSafety::Dangerous => "dangerous",
        }
    }

    /// 安全级别数值，用于比较严重程度
    pub fn severity(&self) -> u8 {
        match self {
            CommandSafety::Safe => 0,
            CommandSafety::Caution => 1,
            CommandSafety::Dangerous => 2,
        }
    }

    /// 判断是否允许在给定权限模式下执行
    pub fn is_allowed(&self, permission_mode: &str) -> bool {
        match self {
            CommandSafety::Safe => true,
            CommandSafety::Caution => matches!(permission_mode, "full_access"),
            CommandSafety::Dangerous => false, // Dangerous 命令始终需要显式授权
        }
    }

    /// 判断是否需要用户确认
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, CommandSafety::Caution)
    }

    /// 判断是否被阻止执行
    pub fn is_blocked(&self) -> bool {
        matches!(self, CommandSafety::Dangerous)
    }
}

/// 命令元数据（用于命令索引）
#[derive(Debug, Clone)]
pub struct CommandMetadata {
    /// 命令名称
    pub name: &'static str,
    /// 一行描述
    pub description: &'static str,
    /// 所属功能域
    pub domain: CommandDomain,
    /// 安全级别
    pub safety: CommandSafety,
}

/// 功能域枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommandDomain {
    Core,
    Knowledge,
    Workflow,
    Provider,
    Gateway,
    Mcp,
    Skill,
    Conversation,
    Message,
    Memory,
    Settings,
}

impl CommandDomain {
    pub fn as_str(&self) -> &str {
        match self {
            CommandDomain::Core => "core",
            CommandDomain::Knowledge => "knowledge",
            CommandDomain::Workflow => "workflow",
            CommandDomain::Provider => "provider",
            CommandDomain::Gateway => "gateway",
            CommandDomain::Mcp => "mcp",
            CommandDomain::Skill => "skill",
            CommandDomain::Conversation => "conversation",
            CommandDomain::Message => "message",
            CommandDomain::Memory => "memory",
            CommandDomain::Settings => "settings",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "core" => Some(CommandDomain::Core),
            "knowledge" => Some(CommandDomain::Knowledge),
            "workflow" => Some(CommandDomain::Workflow),
            "provider" => Some(CommandDomain::Provider),
            "gateway" => Some(CommandDomain::Gateway),
            "mcp" => Some(CommandDomain::Mcp),
            "skill" => Some(CommandDomain::Skill),
            "conversation" => Some(CommandDomain::Conversation),
            "message" => Some(CommandDomain::Message),
            "memory" => Some(CommandDomain::Memory),
            "settings" => Some(CommandDomain::Settings),
            _ => None,
        }
    }
}

// ── Command Registry (Phase 3) ──────────────────────────────────────────

/// 命令注册中心 — 支持动态注册和按需加载
#[derive(Debug, Clone)]
pub struct CommandRegistry {
    /// 所有已注册的命令元数据
    commands: Vec<CommandMetadata>,
    /// 按名称索引的命令映射（用于快速查找）
    name_index: std::collections::HashMap<String, usize>,
}

impl CommandRegistry {
    /// 从宏注册表创建注册中心
    ///
    /// 所有命令都通过 #[agent_command] 宏注册，元数据由宏在编译时收集。
    pub fn from_registry() -> Self {
        let macro_commands = agent_command_types::registry::get_all();

        let mut commands = Vec::with_capacity(macro_commands.len());
        for mc in &macro_commands {
            let domain = Self::map_domain(mc.domain);
            let safety = Self::map_safety(mc.safety);
            commands.push(CommandMetadata {
                name: mc.name,
                description: mc.description,
                domain,
                safety,
            });
        }

        let name_index =
            commands.iter().enumerate().map(|(i, cmd)| (cmd.name.to_string(), i)).collect();

        debug!("CommandRegistry: {} commands from macro registry", commands.len());

        Self { commands, name_index }
    }

    fn map_domain(domain_str: &str) -> CommandDomain {
        match domain_str {
            "knowledge" => CommandDomain::Knowledge,
            "workflow" => CommandDomain::Workflow,
            "provider" => CommandDomain::Provider,
            "gateway" => CommandDomain::Gateway,
            "mcp" => CommandDomain::Mcp,
            "skill" => CommandDomain::Skill,
            "conversation" => CommandDomain::Conversation,
            "message" => CommandDomain::Message,
            "memory" => CommandDomain::Memory,
            "settings" => CommandDomain::Settings,
            _ => CommandDomain::Core,
        }
    }

    fn map_safety(s: agent_command_types::CommandSafety) -> CommandSafety {
        match s {
            agent_command_types::CommandSafety::Safe => CommandSafety::Safe,
            agent_command_types::CommandSafety::Caution => CommandSafety::Caution,
            agent_command_types::CommandSafety::Dangerous => CommandSafety::Dangerous,
        }
    }

    /// 按名称查找命令元数据
    pub fn find_by_name(&self, name: &str) -> Option<&CommandMetadata> {
        self.name_index.get(name).and_then(|idx| self.commands.get(*idx))
    }

    /// 按域查找命令
    pub fn find_by_domain(&self, domain: &CommandDomain) -> Vec<&CommandMetadata> {
        self.commands.iter().filter(|cmd| cmd.domain == *domain).collect()
    }

    /// 获取指定域列表内的命令
    pub fn get_commands_for_domains(&self, domains: &[CommandDomain]) -> Vec<&CommandMetadata> {
        self.commands.iter().filter(|cmd| domains.contains(&cmd.domain)).collect()
    }

    /// 获取所有命令
    pub fn all(&self) -> &[CommandMetadata] {
        &self.commands
    }

    /// 检查命令是否存在
    pub fn contains(&self, name: &str) -> bool {
        self.name_index.contains_key(name)
    }

    /// 获取命令总数
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// 构建命令索引字符串（用于系统提示注入）
    pub fn build_index_string(&self, visible_domains: &[CommandDomain]) -> String {
        let mut index = String::new();
        index.push_str("可用后端命令（通过 execute_tauri_command 调用）：\n");

        let mut current_domain: Option<CommandDomain> = None;
        let filtered = self.get_commands_for_domains(visible_domains);

        for cmd in filtered {
            // 域分组标题
            if current_domain != Some(cmd.domain.clone()) {
                if current_domain.is_some() {
                    index.push('\n');
                }
                index.push_str(&format!("\n[{}]\n", cmd.domain.as_str()));
                current_domain = Some(cmd.domain.clone());
            }

            // 命令条目，包含安全级别标识
            let safety_icon = match cmd.safety {
                CommandSafety::Safe => "✓",
                CommandSafety::Caution => "⚠",
                CommandSafety::Dangerous => "✗",
            };
            index.push_str(&format!("- {} {}: {}\n", safety_icon, cmd.name, cmd.description));
        }

        index.push_str("\n提示：使用 execute_tauri_command 工具调用命令，参数 command 为命令名，args 为 JSON 参数。\n");
        index
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::from_registry()
    }
}

// ── Command Cache (Phase 3) ────────────────────────────────────────────

/// 命令缓存 — 缓存常用命令索引，提高性能
///
/// 避免每次都重新构建命令索引字符串，特别是在频繁调用场景下
#[derive(Debug, Clone)]
pub struct CommandCache {
    /// 缓存的索引字符串（按域列表哈希作为键）
    cache: std::collections::HashMap<String, String>,
    /// 最大缓存条目数
    max_size: usize,
    /// 缓存命中次数
    hits: u64,
    /// 缓存未命中次数
    misses: u64,
}

impl CommandCache {
    /// 创建新的缓存实例
    pub fn new(max_size: usize) -> Self {
        Self { cache: std::collections::HashMap::new(), max_size, hits: 0, misses: 0 }
    }

    /// 默认缓存大小
    pub fn default_cache() -> Self {
        Self::new(128)
    }

    /// 生成缓存键（基于域列表）
    fn make_key(domains: &[CommandDomain]) -> String {
        let mut domain_names: Vec<String> =
            domains.iter().map(|d| d.as_str().to_string()).collect();
        domain_names.sort();
        domain_names.join(",")
    }

    /// 获取缓存的索引字符串
    pub fn get(&mut self, domains: &[CommandDomain], registry: &CommandRegistry) -> String {
        let key = Self::make_key(domains);

        if let Some(cached) = self.cache.get(&key) {
            self.hits += 1;
            return cached.clone();
        }

        // 缓存未命中，构建并存储
        self.misses += 1;
        let index = registry.build_index_string(domains);

        // 如果缓存已满，移除最旧的条目
        if self.cache.len() >= self.max_size {
            if let Some(oldest_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest_key);
            }
        }

        self.cache.insert(key, index.clone());
        index
    }

    /// 清除缓存
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> (u64, u64, usize) {
        (self.hits, self.misses, self.cache.len())
    }

    /// 获取缓存命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl Default for CommandCache {
    fn default() -> Self {
        Self::default_cache()
    }
}

/// 构建命令索引字符串（用于注入系统提示）
///
/// 这是一个便捷函数，内部使用 CommandRegistry 来构建索引
pub fn build_command_index_string(visible_domains: &[CommandDomain]) -> String {
    let registry = CommandRegistry::default();
    registry.build_index_string(visible_domains)
}

/// 预加载缓存并返回命中率
///
/// 用于在启动时预热缓存
pub fn preload_command_cache(domains: &[CommandDomain]) -> (String, f64) {
    let registry = CommandRegistry::default();
    let mut cache = CommandCache::default();
    let _first_load = cache.get(domains, &registry);
    // 清除旧缓存并重新加载
    cache.clear();
    let index = cache.get(domains, &registry);
    let hit_rate = cache.hit_rate();
    (index, hit_rate)
}

// ── Domain Mapping Configuration (Phase 2) ──────────────────────────────

/// 工具域到命令域的映射配置
///
/// 支持灵活的领域映射，可根据 Agent 的角色/场景动态调整可见命令
#[derive(Debug, Clone)]
pub struct DomainMapping {
    /// 工具域（来自 axagent_harness::ToolDomain 的字符串表示）
    pub tool_domain: String,
    /// 映射到的命令域列表
    pub command_domains: Vec<CommandDomain>,
}

/// 领域映射配置集合
#[derive(Debug, Clone)]
pub struct DomainMappingConfig {
    /// 映射规则列表
    pub mappings: Vec<DomainMapping>,
    /// 无论任何场景都默认暴露的命令域
    pub default_domains: Vec<CommandDomain>,
}

impl Default for DomainMappingConfig {
    fn default() -> Self {
        Self {
            mappings: vec![
                // Core / General → 基础命令域
                DomainMapping {
                    tool_domain: "core".to_string(),
                    command_domains: vec![
                        CommandDomain::Core,
                        CommandDomain::Settings,
                        CommandDomain::Conversation,
                        CommandDomain::Message,
                        CommandDomain::Memory,
                        CommandDomain::Knowledge,
                        CommandDomain::Provider,
                    ],
                },
                DomainMapping {
                    tool_domain: "general".to_string(),
                    command_domains: vec![
                        CommandDomain::Core,
                        CommandDomain::Settings,
                        CommandDomain::Conversation,
                        CommandDomain::Message,
                        CommandDomain::Memory,
                        CommandDomain::Knowledge,
                        CommandDomain::Provider,
                    ],
                },
                // Devops → 运维全套
                DomainMapping {
                    tool_domain: "devops".to_string(),
                    command_domains: vec![
                        CommandDomain::Gateway,
                        CommandDomain::Mcp,
                        CommandDomain::Skill,
                        CommandDomain::Workflow,
                    ],
                },
                // AiMedia → 基础 + 技能
                DomainMapping {
                    tool_domain: "ai_media".to_string(),
                    command_domains: vec![CommandDomain::Core, CommandDomain::Skill],
                },
                // Invest → 基础 + 知识库
                DomainMapping {
                    tool_domain: "invest".to_string(),
                    command_domains: vec![CommandDomain::Core, CommandDomain::Knowledge],
                },
                // Opc → 基础 + 工作流
                DomainMapping {
                    tool_domain: "opc".to_string(),
                    command_domains: vec![CommandDomain::Core, CommandDomain::Workflow],
                },
            ],
            // 默认暴露核心域
            default_domains: vec![CommandDomain::Core],
        }
    }
}

impl DomainMappingConfig {
    /// 根据激活的工具域集合解析可见的命令域
    ///
    /// # Arguments
    /// * `active_tool_domains` - 当前激活的工具域名称集合
    ///
    /// # Returns
    /// 去重后的命令域列表
    pub fn resolve_command_domains(
        &self,
        active_tool_domains: &HashSet<String>,
    ) -> Vec<CommandDomain> {
        let mut result = HashSet::new();

        // 先加入默认域
        for domain in &self.default_domains {
            result.insert(domain.clone());
        }

        // 根据映射规则加入匹配的命令域
        for mapping in &self.mappings {
            if active_tool_domains.contains(&mapping.tool_domain) {
                for domain in &mapping.command_domains {
                    result.insert(domain.clone());
                }
            }
        }

        result.into_iter().collect()
    }
}

/// 根据激活的工具域名称解析可见命令域
///
/// 便捷函数，使用默认配置
pub fn resolve_command_domains(active_tool_domains: &HashSet<String>) -> Vec<CommandDomain> {
    let config = DomainMappingConfig::default();
    config.resolve_command_domains(active_tool_domains)
}

/// 构建可注册到 Agent 的 Tauri 命令工具列表
///
/// 使用函数而非静态变量，因为 serde_json::json! 宏在静态上下文中不可用。
pub fn build_tool_defs() -> Vec<TauriCommandToolDef> {
    vec![
        // ── 代理工具：统一命令入口 ──
        TauriCommandToolDef {
            name: "execute_tauri_command",
            description: "执行应用后端命令。command 参数为命令名，args 为 JSON 参数对象。可用命令列表见系统提示中的索引。",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "命令名称，如 list_conversations, search_knowledge_base"
                    },
                    "args": {
                        "type": "object",
                        "description": "命令参数，JSON 对象格式"
                    }
                },
                "required": ["command", "args"]
            }),
            is_read_only: false,
        },
        // ── 设置（只读） ──
        TauriCommandToolDef {
            name: "tauri_get_settings",
            description: "获取当前应用的完整设置，包括主题、语言、遥测级别等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            is_read_only: true,
        },
        // ── 设置（写入） ──
        TauriCommandToolDef {
            name: "tauri_save_settings",
            description: "保存应用设置。支持部分更新（主题模式、语言等）。此操作会立即生效。",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "theme_mode": { "type": "string", "description": "主题模式 (light/dark/system)" },
                    "theme_preset": { "type": "string", "description": "主题预设 (deep-dusk/oceanic-dark 等)" },
                    "language": { "type": "string", "description": "语言代码 (zh-CN/en-US 等)" },
                },
            }),
            is_read_only: false,
        },
        // ── 会话（只读） ──
        TauriCommandToolDef {
            name: "tauri_list_conversations",
            description: "列出所有会话，按更新时间倒序排列。返回会话 ID、标题、更新时间等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            is_read_only: true,
        },
        TauriCommandToolDef {
            name: "tauri_get_conversation",
            description: "获取单个会话的详细信息，包括标题、创建时间、更新时间、是否置顶等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "conversation_id": { "type": "string", "description": "会话 ID" },
                },
                "required": ["conversation_id"],
            }),
            is_read_only: true,
        },
        // ── 知识库（只读） ──
        TauriCommandToolDef {
            name: "tauri_list_knowledge_bases",
            description: "列出所有知识库，包括名称、类型、描述等信息",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            is_read_only: true,
        },
        // ── 记忆（只读） ──
        TauriCommandToolDef {
            name: "tauri_list_memories",
            description: "列出记忆条目，支持按重要性过滤。返回记忆内容、重要性分数等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "min_importance": { "type": "number", "description": "最低重要性阈值 (0.0-1.0)" },
                    "limit": { "type": "integer", "description": "最大返回数量 (默认 20)" },
                },
            }),
            is_read_only: true,
        },
        // ── Agent UI 渲染（写入） ──
        TauriCommandToolDef {
            name: "tauri_render_ui",
            description: "在前端 Agent 面板中渲染一个动态 UI 组件。接收 UISchema JSON 定义，由前端 DynamicUIRenderer 渲染。支持容器、表单、表格、图表等组件类型",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "schema": { "type": "object", "description": "UISchema 定义，包含 version/id/type/props/children 等字段" },
                    "target_id": { "type": "string", "description": "目标容器 ID，用于定位渲染位置" },
                    "replace": { "type": "boolean", "description": "是否替换已存在的同名组件 (默认 true)" },
                },
                "required": ["schema"],
            }),
            is_read_only: false,
        },
        // ── Agent UI 更新（写入） ──
        TauriCommandToolDef {
            name: "tauri_update_ui",
            description: "更新已渲染的 Agent UI 组件。支持 replace/append/remove 三种操作模式",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": { "type": "string", "enum": ["replace", "append", "remove"], "description": "操作类型" },
                    "schema_id": { "type": "string", "description": "要更新的 Schema ID" },
                    "new_schema": { "type": "object", "description": "新的 UISchema (replace/append 时必填)" },
                    "path": { "type": "string", "description": "更新路径 (如 root.children[0])" },
                },
                "required": ["operation", "schema_id"],
            }),
            is_read_only: false,
        },
        // ── Agent UI 销毁（写入） ──
        TauriCommandToolDef {
            name: "tauri_remove_ui",
            description: "移除已渲染的 Agent UI 组件",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "schema_id": { "type": "string", "description": "要移除的 Schema ID" },
                },
                "required": ["schema_id"],
            }),
            is_read_only: false,
        },
    ]
}

/// 将工具定义转换为 ChatTool 列表
pub fn build_chat_tools() -> Vec<ChatTool> {
    // 验证可用命令
    let _available = list_available_commands();
    debug!("Building chat tools, available commands: {}", _available.len());

    build_tool_defs()
        .into_iter()
        .map(|def| ChatTool {
            r#type: "function".to_string(),
            function: ChatToolFunction {
                name: def.name.to_string(),
                description: Some(def.description.to_string()),
                parameters: Some(def.input_schema),
            },
        })
        .collect()
}

/// 为每个工具创建 SkillToolHandler
///
/// 在 handler 内部通过 block_in_place + block_on 调用异步 DAO 操作。
pub fn build_command_handlers(
    db: DatabaseConnection,
    app_handle: AppHandle,
) -> Vec<(String, SkillToolHandler)> {
    let mut handlers = Vec::new();

    for def in build_tool_defs() {
        let handler = create_handler(def.name, db.clone(), app_handle.clone());
        handlers.push((def.name.to_string(), handler));
    }

    handlers
}

/// 创建单个命令的 handler
fn create_handler(
    command_name: &str,
    db: DatabaseConnection,
    app_handle: AppHandle,
) -> SkillToolHandler {
    let name = command_name.to_string();
    Box::new(move |input: &str| {
        let input_value: Value =
            serde_json::from_str(input).unwrap_or_else(|_| serde_json::json!({}));

        execute_command(&name, &input_value, &db, &app_handle)
    })
}

/// 同步 handler 内部的执行逻辑
///
/// 安全地从同步上下文进入异步 runtime：
/// - 如果已在 tokio runtime 中，直接使用 Handle::current().block_on()
/// - 如果不在 runtime 中，创建临时 runtime 执行
fn execute_command(
    command_name: &str,
    input: &Value,
    db: &DatabaseConnection,
    app_handle: &AppHandle,
) -> Result<String, axagent_tools::ToolError> {
    let db = db.clone();
    let app = app_handle.clone();
    let name = command_name.to_string();

    // 安全地获取或创建 runtime 执行异步操作
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // 已在 tokio runtime 中，直接 block_on
            handle.block_on(async { dispatch_command(&name, input, &db, &app).await })
        },
        Err(_) => {
            // 不在 runtime 中，创建临时 runtime
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| axagent_tools::ToolError::execution_failed(command_name))?;
            runtime.block_on(async { dispatch_command(&name, input, &db, &app).await })
        },
    }
    .map_err(|_| axagent_tools::ToolError::execution_failed(command_name))
}

/// 命令分发 — 根据命令名调用对应的 DAO 操作
#[instrument(skip(db, app_handle), fields(command = %command_name))]
async fn dispatch_command(
    command_name: &str,
    input: &Value,
    db: &DatabaseConnection,
    app_handle: &AppHandle,
) -> Result<String, String> {
    debug!("Executing Tauri command: {}", command_name);

    match command_name {
        "tauri_get_settings" => {
            let settings = axagent_dao::repo::settings::get_settings(db).await.map_err(|e| {
                warn!("Failed to get settings: {}", e);
                format!("获取设置失败: {}", e)
            })?;
            serde_json::to_string_pretty(&settings).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error_with_code(
                    crate::commands::error_code::common::INTERNAL,
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "tauri_save_settings" => {
            let mut settings =
                axagent_dao::repo::settings::get_settings(db).await.map_err(|e| {
                    warn!("Failed to get settings for save: {}", e);
                    format!("获取设置失败: {}", e)
                })?;
            apply_settings_patch(&mut settings, input);
            axagent_dao::repo::settings::save_settings(db, &settings).await.map_err(|e| {
                warn!("Failed to save settings: {}", e);
                format!("保存设置失败: {}", e)
            })?;
            debug!("Settings saved successfully");
            Ok(serde_json::json!({ "success": true }).to_string())
        },
        "tauri_list_conversations" => {
            let convs =
                axagent_dao::repo::conversation::list_conversations(db).await.map_err(|e| {
                    warn!("Failed to list conversations: {}", e);
                    format!("列出会话失败: {}", e)
                })?;
            let summaries: Vec<_> = convs
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "title": c.title,
                        "updated_at": c.updated_at,
                        "is_pinned": c.is_pinned,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&summaries).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error_with_code(
                    crate::commands::error_code::common::INTERNAL,
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "tauri_get_conversation" => {
            let conv_id = input["conversation_id"]
                .as_str()
                .ok_or_else(|| "缺少 conversation_id 参数".to_string())?;
            let conv = axagent_dao::repo::conversation::get_conversation(db, conv_id)
                .await
                .map_err(|e| {
                    warn!("Failed to get conversation {}: {}", conv_id, e);
                    format!("获取会话失败: {}", e)
                })?;
            serde_json::to_string_pretty(&conv).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error_with_code(
                    crate::commands::error_code::common::INTERNAL,
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "tauri_list_knowledge_bases" => {
            let kbs =
                axagent_dao::repo::knowledge::list_knowledge_bases(db).await.map_err(|e| {
                    warn!("Failed to list knowledge bases: {}", e);
                    format!("列出知识库失败: {}", e)
                })?;
            let summaries: Vec<_> = kbs
                .iter()
                .map(|kb| {
                    serde_json::json!({
                        "id": kb.id,
                        "name": kb.name,
                        "kind": kb.kind,
                        "enabled": kb.enabled,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&summaries).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error_with_code(
                    crate::commands::error_code::common::INTERNAL,
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "tauri_list_memories" => {
            let min_importance = input["min_importance"].as_f64();
            let limit = input["limit"].as_u64().map(|v| v as u32);
            let memories =
                axagent_dao::repo::memory::list_high_importance_items(db, min_importance, limit)
                    .await
                    .map_err(|e| {
                        warn!("Failed to list memories: {}", e);
                        format!("列出记忆失败: {}", e)
                    })?;
            let summaries: Vec<_> = memories
                .iter()
                .map(|m| {
                    let preview = if m.content.chars().count() > 100 {
                        format!("{}...", m.content.chars().take(100).collect::<String>())
                    } else {
                        m.content.clone()
                    };
                    serde_json::json!({
                        "id": m.id,
                        "title": m.title,
                        "importance": m.importance,
                        "content_preview": preview,
                        "tags": m.tags,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&summaries).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error_with_code(
                    crate::commands::error_code::common::INTERNAL,
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        // ── Agent UI 渲染 ──
        "tauri_render_ui" => {
            let schema =
                input["schema"].as_object().ok_or_else(|| "缺少 schema 参数".to_string())?;
            let target_id = input["target_id"].as_str().map(|s| s.to_string());
            let replace = input["replace"].as_bool().unwrap_or(true);
            let schema_id = schema.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

            let payload = serde_json::json!({
                "schema": schema,
                "targetId": target_id,
                "replace": replace,
            });

            app_handle.emit("agent-render-ui", &payload).map_err(|e| {
                warn!("Failed to emit agent-render-ui event: {}", e);
                format!("派发 UI 渲染事件失败: {}", e)
            })?;

            debug!("UI rendered: schemaId={}, replace={}", schema_id, replace);

            Ok(serde_json::json!({
                "success": true,
                "action": "render",
                "schemaId": schema_id,
            })
            .to_string())
        },
        "tauri_update_ui" => {
            let operation =
                input["operation"].as_str().ok_or_else(|| "缺少 operation 参数".to_string())?;
            let schema_id =
                input["schema_id"].as_str().ok_or_else(|| "缺少 schema_id 参数".to_string())?;
            let new_schema = input["new_schema"].as_object();
            let path = input["path"].as_str().map(|s| s.to_string());

            let payload = serde_json::json!({
                "operation": operation,
                "schemaId": schema_id,
                "newSchema": new_schema,
                "path": path,
            });

            app_handle.emit("agent-update-ui", &payload).map_err(|e| {
                warn!("Failed to emit agent-update-ui event: {}", e);
                format!("派发 UI 更新事件失败: {}", e)
            })?;

            debug!("UI updated: schemaId={}, operation={}", schema_id, operation);

            Ok(serde_json::json!({
                "success": true,
                "action": "update",
                "schemaId": schema_id,
                "operation": operation,
            })
            .to_string())
        },
        "tauri_remove_ui" => {
            let schema_id =
                input["schema_id"].as_str().ok_or_else(|| "缺少 schema_id 参数".to_string())?;

            let payload = serde_json::json!({
                "schemaId": schema_id,
            });

            app_handle.emit("agent-remove-ui", &payload).map_err(|e| {
                warn!("Failed to emit agent-remove-ui event: {}", e);
                format!("派发 UI 移除事件失败: {}", e)
            })?;

            debug!("UI removed: schemaId={}", schema_id);

            Ok(serde_json::json!({
                "success": true,
                "action": "remove",
                "schemaId": schema_id,
            })
            .to_string())
        },
        // ── 代理工具：统一命令入口 ──
        "execute_tauri_command" => {
            let command =
                input["command"].as_str().ok_or_else(|| "缺少 command 参数".to_string())?;
            let args = input["args"].clone();

            // 使用 CommandRegistry 查找命令元数据
            let registry = CommandRegistry::default();

            // 检查命令是否存在
            if !registry.contains(command) {
                warn!(
                    "Command not found in registry: {} (total commands: {})",
                    command,
                    registry.len()
                );
                return Err(format!(
                    "命令 '{}' 不在可用命令列表中。请检查命令名称或使用 list_available_commands 查看可用命令。",
                    command
                ));
            }

            let cmd_meta = registry.find_by_name(command).unwrap();
            let safety = &cmd_meta.safety;
            let permission_mode = input["permission_mode"].as_str().unwrap_or("default");

            // 检查是否在给定权限模式下允许执行
            if !safety.is_allowed(permission_mode) && safety.is_blocked() {
                warn!(
                    "Command blocked by permission mode: {} (mode={}, severity={})",
                    command,
                    permission_mode,
                    safety.severity()
                );
                Err(format!(
                    "命令 '{}' 是危险操作，在当前权限模式 '{}' 下不允许执行。",
                    command, permission_mode
                ))
            } else if safety.is_blocked() {
                // 危险命令，拒绝执行
                warn!("Dangerous command blocked: {} (severity={})", command, safety.severity());
                Err(format!("命令 '{}' 是危险操作，需要显式授权才能执行。", command))
            } else if safety.requires_confirmation() {
                // 写入命令，返回需要确认的提示
                debug!(
                    "Command requires confirmation: {} (severity={})",
                    command,
                    safety.severity()
                );
                Ok(serde_json::json!({
                    "requires_confirmation": true,
                    "command": command,
                    "safety_level": safety.as_str(),
                    "message": format!(
                        "命令 '{}' 是写入操作，需要用户确认。安全级别: {}",
                        command, safety.as_str()
                    ),
                    "args": args,
                })
                .to_string())
            } else {
                // 只读命令，直接执行
                debug!("Executing safe command: {} (severity={})", command, safety.severity());
                dispatch_proxy_command(command, &args, db, app_handle).await
            }
        },
        // ── 辅助工具：列出可用命令 ──
        "list_available_commands" => {
            let domain_filter = input["domain"].as_str();
            let registry = CommandRegistry::default();

            let domain_commands: Vec<&CommandMetadata> = if let Some(domain_str) = domain_filter {
                if let Some(domain) = CommandDomain::from_str(domain_str) {
                    registry.find_by_domain(&domain)
                } else {
                    warn!("Invalid domain filter: {}", domain_str);
                    Vec::new()
                }
            } else {
                registry.all().iter().collect()
            };

            let commands: Vec<_> = domain_commands
                .iter()
                .map(|cmd| {
                    serde_json::json!({
                        "name": cmd.name,
                        "description": cmd.description,
                        "domain": cmd.domain.as_str(),
                        "safety": cmd.safety.as_str(),
                    })
                })
                .collect();

            Ok(serde_json::json!({
                "total": commands.len(),
                "is_empty": registry.is_empty(),
                "commands": commands,
            })
            .to_string())
        },
        unknown => {
            warn!("Unknown Tauri command: {}", unknown);
            Err(format!("未知的 Tauri 命令: {}", unknown))
        },
    }
}

/// 将 input 中的字段应用到 AppSettings 上（部分更新）
///
/// 仅允许更新白名单内的字段，并对输入值做基本验证：
/// - theme_mode: 仅接受 "light" / "dark" / "system"
/// - language:  必须符合 xx-YY 格式（如 zh-CN, en-US）
/// - 数值字段:  范围检查
fn apply_settings_patch(settings: &mut axagent_harness::types::AppSettings, input: &Value) {
    // ── 字符串枚举字段 ──

    if let Some(theme_mode) = input["theme_mode"].as_str() {
        let valid = matches!(theme_mode, "light" | "dark" | "system");
        if valid {
            settings.theme_mode = theme_mode.to_string();
        } else {
            warn!("Invalid theme_mode '{}', must be light/dark/system, skipping", theme_mode);
        }
    }

    if let Some(theme_preset) = input["theme_preset"].as_str() {
        settings.theme_preset = theme_preset.to_string();
    }

    if let Some(language) = input["language"].as_str() {
        // 验证语言代码格式: xx-YY 或 xx
        let is_valid_lang =
            language.len() >= 2 && language.chars().next().is_some_and(|c| c.is_ascii_lowercase());
        if is_valid_lang {
            settings.language = language.to_string();
        } else {
            warn!(
                "Invalid language code '{}', must be a valid locale (e.g. zh-CN), skipping",
                language
            );
        }
    }

    // ── 可选的数值字段（常用设置） ──

    if let Some(primary_color) = input["primary_color"].as_str() {
        // 简单验证 hex 颜色格式
        if primary_color.starts_with('#') && (primary_color.len() == 7 || primary_color.len() == 4)
        {
            settings.primary_color = primary_color.to_string();
        } else {
            warn!(
                "Invalid primary_color '{}', must be hex color (e.g. #FF0000), skipping",
                primary_color
            );
        }
    }

    if let Some(font_size) = input["font_size"].as_u64() {
        if (10..=24).contains(&font_size) {
            settings.font_size = font_size as u8;
        } else {
            warn!("Invalid font_size {}, must be 10-24, skipping", font_size);
        }
    }

    if let Some(border_radius) = input["border_radius"].as_u64() {
        if (0..=20).contains(&border_radius) {
            settings.border_radius = border_radius as u8;
        } else {
            warn!("Invalid border_radius {}, must be 0-20, skipping", border_radius);
        }
    }

    // ── 布尔开关字段 ──

    if let Some(auto_start) = input["auto_start"].as_bool() {
        settings.auto_start = auto_start;
    }

    if let Some(show_on_start) = input["show_on_start"].as_bool() {
        settings.show_on_start = show_on_start;
    }

    if let Some(minimize_to_tray) = input["minimize_to_tray"].as_bool() {
        settings.minimize_to_tray = minimize_to_tray;
    }

    if let Some(always_on_top) = input["always_on_top"].as_bool() {
        settings.always_on_top = always_on_top;
    }

    if let Some(telemetry_level) = input["telemetry_level"].as_str() {
        let valid = matches!(telemetry_level, "off" | "minimal" | "full");
        if valid {
            settings.telemetry_level = telemetry_level.to_string();
        } else {
            warn!(
                "Invalid telemetry_level '{}', must be off/minimal/full, skipping",
                telemetry_level
            );
        }
    }
}

// ── Command Handler System ──────────────────────────────────────────────

/// 命令处理器 trait — 每个可暴露给 Agent 的命令都需要实现此 trait
#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    /// 执行命令
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        app_handle: &AppHandle,
    ) -> Result<String, String>;
}

/// 命令分发器 — 存储命令处理器并按需分发
pub struct CommandDispatcher {
    handlers: HashMap<String, Box<dyn CommandHandler>>,
}

impl CommandDispatcher {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    /// 注册命令处理器
    pub fn register(&mut self, name: &str, handler: Box<dyn CommandHandler>) {
        self.handlers.insert(name.to_string(), handler);
    }

    /// 批量注册命令处理器
    pub fn register_batch(&mut self, handlers: Vec<(&str, Box<dyn CommandHandler>)>) {
        for (name, handler) in handlers {
            self.register(name, handler);
        }
    }

    /// 分发命令
    pub async fn dispatch(
        &self,
        command: &str,
        args: &Value,
        db: &DatabaseConnection,
        app_handle: &AppHandle,
    ) -> Result<String, String> {
        let handler =
            self.handlers.get(command).ok_or_else(|| format!("命令 '{}' 未注册。", command))?;
        handler.execute(args, db, app_handle).await
    }

    /// 检查命令是否存在
    pub fn contains(&self, command: &str) -> bool {
        self.handlers.contains_key(command)
    }

    /// 获取已注册命令数量
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl Default for CommandDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ── Built-in Command Handlers ──────────────────────────────────────────

/// Settings 相关命令处理器
pub struct SettingsCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for SettingsCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("get");

        match action {
            "get" => {
                let settings =
                    axagent_dao::repo::settings::get_settings(db).await.map_err(|e| {
                        warn!("Failed to get settings: {}", e);
                        format!("获取设置失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&settings).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "save" => {
                let mut settings =
                    axagent_dao::repo::settings::get_settings(db).await.map_err(|e| {
                        warn!("Failed to get settings for save: {}", e);
                        format!("获取设置失败: {}", e)
                    })?;
                crate::commands::agent::command_bridge::apply_settings_patch(&mut settings, args);
                axagent_dao::repo::settings::save_settings(db, &settings).await.map_err(|e| {
                    warn!("Failed to save settings: {}", e);
                    format!("保存设置失败: {}", e)
                })?;
                Ok(serde_json::json!({ "success": true }).to_string())
            },
            _ => Err(format!("未知的设置操作: {}", action)),
        }
    }
}

/// Conversation 相关命令处理器
pub struct ConversationCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for ConversationCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list");

        match action {
            "list" => {
                let convs =
                    axagent_dao::repo::conversation::list_conversations(db).await.map_err(|e| {
                        warn!("Failed to list conversations: {}", e);
                        format!("列出会话失败: {}", e)
                    })?;
                let summaries: Vec<_> = convs
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "title": c.title,
                            "updated_at": c.updated_at,
                            "is_pinned": c.is_pinned,
                        })
                    })
                    .collect();
                serde_json::to_string_pretty(&summaries).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "get" => {
                let conv_id = args["conversation_id"]
                    .as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or_else(|| "缺少 conversation_id 参数".to_string())?;
                let conv = axagent_dao::repo::conversation::get_conversation(db, conv_id)
                    .await
                    .map_err(|e| {
                        warn!("Failed to get conversation {}: {}", conv_id, e);
                        format!("获取会话失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&conv).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的会话操作: {}", action)),
        }
    }
}

/// Knowledge 相关命令处理器
pub struct KnowledgeCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for KnowledgeCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list");

        match action {
            "list" => {
                let kbs =
                    axagent_dao::repo::knowledge::list_knowledge_bases(db).await.map_err(|e| {
                        warn!("Failed to list knowledge bases: {}", e);
                        format!("列出知识库失败: {}", e)
                    })?;
                let summaries: Vec<_> = kbs
                    .iter()
                    .map(|kb| {
                        serde_json::json!({
                            "id": kb.id,
                            "name": kb.name,
                            "kind": kb.kind,
                            "enabled": kb.enabled,
                        })
                    })
                    .collect();
                serde_json::to_string_pretty(&summaries).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的知识库操作: {}", action)),
        }
    }
}

/// Memory 相关命令处理器
pub struct MemoryCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for MemoryCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let min_importance = args["min_importance"].as_f64();
        let limit = args["limit"].as_u64().map(|v| v as u32);

        let memories =
            axagent_dao::repo::memory::list_high_importance_items(db, min_importance, limit)
                .await
                .map_err(|e| {
                    warn!("Failed to list memories: {}", e);
                    format!("列出记忆失败: {}", e)
                })?;

        let summaries: Vec<_> = memories
            .iter()
            .map(|m| {
                let preview = if m.content.chars().count() > 100 {
                    format!("{}...", m.content.chars().take(100).collect::<String>())
                } else {
                    m.content.clone()
                };
                serde_json::json!({
                    "id": m.id,
                    "title": m.title,
                    "importance": m.importance,
                    "content_preview": preview,
                    "tags": m.tags,
                })
            })
            .collect();

        serde_json::to_string_pretty(&summaries).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
    }
}

/// Agent UI 渲染命令处理器
pub struct AgentUICommandHandler;

#[async_trait::async_trait]
impl CommandHandler for AgentUICommandHandler {
    async fn execute(
        &self,
        args: &Value,
        _db: &DatabaseConnection,
        app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("render");

        match action {
            "render" => {
                let schema =
                    args["schema"].as_object().ok_or_else(|| "缺少 schema 参数".to_string())?;
                let target_id = args["target_id"].as_str().map(|s| s.to_string());
                let replace = args["replace"].as_bool().unwrap_or(true);
                let schema_id = schema.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

                let payload = serde_json::json!({
                    "schema": schema,
                    "targetId": target_id,
                    "replace": replace,
                });

                app_handle.emit("agent-render-ui", &payload).map_err(|e| {
                    warn!("Failed to emit agent-render-ui event: {}", e);
                    format!("派发 UI 渲染事件失败: {}", e)
                })?;

                debug!("UI rendered: schemaId={}, replace={}", schema_id, replace);

                Ok(serde_json::json!({
                    "success": true,
                    "action": "render",
                    "schemaId": schema_id,
                })
                .to_string())
            },
            "update" => {
                let operation =
                    args["operation"].as_str().ok_or_else(|| "缺少 operation 参数".to_string())?;
                let schema_id =
                    args["schema_id"].as_str().ok_or_else(|| "缺少 schema_id 参数".to_string())?;
                let new_schema = args["new_schema"].as_object();
                let path = args["path"].as_str().map(|s| s.to_string());

                let payload = serde_json::json!({
                    "operation": operation,
                    "schemaId": schema_id,
                    "newSchema": new_schema,
                    "path": path,
                });

                app_handle.emit("agent-update-ui", &payload).map_err(|e| {
                    warn!("Failed to emit agent-update-ui event: {}", e);
                    format!("派发 UI 更新事件失败: {}", e)
                })?;

                debug!("UI updated: schemaId={}, operation={}", schema_id, operation);

                Ok(serde_json::json!({
                    "success": true,
                    "action": "update",
                    "schemaId": schema_id,
                    "operation": operation,
                })
                .to_string())
            },
            "remove" => {
                let schema_id =
                    args["schema_id"].as_str().ok_or_else(|| "缺少 schema_id 参数".to_string())?;

                let payload = serde_json::json!({
                    "schemaId": schema_id,
                });

                app_handle.emit("agent-remove-ui", &payload).map_err(|e| {
                    warn!("Failed to emit agent-remove-ui event: {}", e);
                    format!("派发 UI 移除事件失败: {}", e)
                })?;

                debug!("UI removed: schemaId={}", schema_id);

                Ok(serde_json::json!({
                    "success": true,
                    "action": "remove",
                    "schemaId": schema_id,
                })
                .to_string())
            },
            _ => Err(format!("未知的 UI 操作: {}", action)),
        }
    }
}

// ── Provider Command Handler ──────────────────────────────────────────

/// 提供商相关命令处理器
pub struct ProviderCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for ProviderCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list");

        match action {
            "list" | "list_providers" => {
                let providers =
                    axagent_dao::repo::provider::list_providers(db).await.map_err(|e| {
                        warn!("Failed to list providers: {}", e);
                        format!("列出提供商失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&providers).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_merged" => {
                let providers =
                    axagent_dao::repo::provider::list_providers_merged(db).await.map_err(|e| {
                        warn!("Failed to list merged providers: {}", e);
                        format!("列出合并提供商失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&providers).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "get" | "get_provider" => {
                let provider_id = args["provider_id"]
                    .as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or_else(|| "缺少 provider_id 参数".to_string())?;
                let provider = axagent_dao::repo::provider::get_provider(db, provider_id)
                    .await
                    .map_err(|e| {
                        warn!("Failed to get provider {}: {}", provider_id, e);
                        format!("获取提供商失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&provider).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_keys" => {
                let provider_id = args["provider_id"]
                    .as_str()
                    .ok_or_else(|| "缺少 provider_id 参数".to_string())?;
                let keys = axagent_dao::repo::provider::list_keys_for_provider(db, provider_id)
                    .await
                    .map_err(|e| {
                        warn!("Failed to list keys for provider {}: {}", provider_id, e);
                        format!("列出密钥失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&keys).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_models" => {
                let provider_id = args["provider_id"]
                    .as_str()
                    .ok_or_else(|| "缺少 provider_id 参数".to_string())?;
                let models = axagent_dao::repo::provider::list_models_for_provider(db, provider_id)
                    .await
                    .map_err(|e| {
                        warn!("Failed to list models for provider {}: {}", provider_id, e);
                        format!("列出模型失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&models).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的提供商操作: {}", action)),
        }
    }
}

// ── Message Command Handler ──────────────────────────────────────────

/// 消息相关命令处理器
pub struct MessageCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for MessageCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list");

        match action {
            "list" | "list_messages" => {
                let conversation_id = args["conversation_id"]
                    .as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or_else(|| "缺少 conversation_id 参数".to_string())?;
                let messages = axagent_dao::repo::message::list_messages(db, conversation_id)
                    .await
                    .map_err(|e| {
                        warn!(
                            "Failed to list messages for conversation {}: {}",
                            conversation_id, e
                        );
                        format!("列出消息失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&messages).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_page" => {
                let conversation_id = args["conversation_id"]
                    .as_str()
                    .ok_or_else(|| "缺少 conversation_id 参数".to_string())?;
                let page = args["page"].as_u64().unwrap_or(1);
                let page_size = args["page_size"].as_u64().unwrap_or(20);
                let result = axagent_dao::repo::message::list_messages_page(
                    db,
                    conversation_id,
                    page,
                    Some(page_size.to_string().as_str()),
                )
                .await
                .map_err(|e| {
                    warn!("Failed to list messages page for {}: {}", conversation_id, e);
                    format!("分页列出消息失败: {}", e)
                })?;
                serde_json::to_string_pretty(&result).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_versions" => {
                let message_id = args["message_id"]
                    .as_str()
                    .ok_or_else(|| "缺少 message_id 参数".to_string())?;
                let limit = args["limit"].as_u64().unwrap_or(50).to_string();
                let versions =
                    axagent_dao::repo::message::list_message_versions(db, message_id, &limit)
                        .await
                        .map_err(|e| {
                            warn!("Failed to list versions for message {}: {}", message_id, e);
                            format!("列出消息版本失败: {}", e)
                        })?;
                serde_json::to_string_pretty(&versions).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的消息操作: {}", action)),
        }
    }
}

// ── Agent Command Handler ──────────────────────────────────────────

/// 智能体相关命令处理器
pub struct AgentCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for AgentCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list_profiles");

        match action {
            "list_profiles" => {
                let profiles = axagent_dao::repo::agent_profile::list_agent_profiles(db, None)
                    .await
                    .map_err(|e| {
                        warn!("Failed to list agent profiles: {}", e);
                        format!("列出智能体配置失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&profiles).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "get_profile" => {
                let profile_id = args["profile_id"]
                    .as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or_else(|| "缺少 profile_id 参数".to_string())?;
                let profile = axagent_dao::repo::agent_profile::get_agent_profile(db, profile_id)
                    .await
                    .map_err(|e| {
                        warn!("Failed to get agent profile {}: {}", profile_id, e);
                        format!("获取智能体配置失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&profile).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_roles" => {
                let roles = axagent_dao::repo::agent_role::list_agent_roles(db, None)
                    .await
                    .map_err(|e| {
                        warn!("Failed to list agent roles: {}", e);
                        format!("列出智能体角色失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&roles).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "get_role" => {
                let role_id = args["role_id"]
                    .as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or_else(|| "缺少 role_id 参数".to_string())?;
                let role = axagent_dao::repo::agent_role::get_agent_role(db, role_id)
                    .await
                    .map_err(|e| {
                        warn!("Failed to get agent role {}: {}", role_id, e);
                        format!("获取智能体角色失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&role).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_business_roles" => {
                let roles = axagent_dao::repo::business_role::list_business_roles(db, None)
                    .await
                    .map_err(|e| {
                    warn!("Failed to list business roles: {}", e);
                    format!("列出业务角色失败: {}", e)
                })?;
                serde_json::to_string_pretty(&roles).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的智能体操作: {}", action)),
        }
    }
}

// ── MCP Command Handler ──────────────────────────────────────────────

/// MCP 服务器相关命令处理器
pub struct MCPCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for MCPCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list");

        match action {
            "list" | "list_servers" => {
                let servers =
                    axagent_dao::repo::mcp_server::list_mcp_servers(db).await.map_err(|e| {
                        warn!("Failed to list MCP servers: {}", e);
                        format!("列出 MCP 服务器失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&servers).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_builtin" => {
                let servers = axagent_dao::repo::mcp_server::list_builtin_servers(db).await;
                serde_json::to_string_pretty(&servers).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_tools" => {
                let server_id =
                    args["server_id"].as_str().ok_or_else(|| "缺少 server_id 参数".to_string())?;
                let tools = axagent_dao::repo::mcp_server::list_tools_for_server(db, server_id)
                    .await
                    .map_err(|e| {
                        warn!("Failed to list tools for server {}: {}", server_id, e);
                        format!("列出 MCP 工具失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&tools).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的 MCP 操作: {}", action)),
        }
    }
}

// ── Knowledge Advanced Command Handler ──────────────────────────────

/// 高级知识库相关命令处理器（文档、图谱等）
pub struct KnowledgeAdvancedCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for KnowledgeAdvancedCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list_documents");

        match action {
            "list_documents" => {
                let kb_id = args["kb_id"]
                    .as_str()
                    .or_else(|| args["knowledge_base_id"].as_str())
                    .ok_or_else(|| "缺少 kb_id 参数".to_string())?;
                let documents =
                    axagent_dao::repo::knowledge::list_documents(db, kb_id).await.map_err(|e| {
                        warn!("Failed to list documents for KB {}: {}", kb_id, e);
                        format!("列出文档失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&documents).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_entities" => {
                let kb_id = args["kb_id"]
                    .as_str()
                    .or_else(|| args["knowledge_base_id"].as_str())
                    .ok_or_else(|| "缺少 kb_id 参数".to_string())?;
                let entities =
                    axagent_dao::repo::knowledge_graph::list_knowledge_entities(db, kb_id)
                        .await
                        .map_err(|e| {
                            warn!("Failed to list entities for KB {}: {}", kb_id, e);
                            format!("列出知识实体失败: {}", e)
                        })?;
                serde_json::to_string_pretty(&entities).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_relations" => {
                let kb_id = args["kb_id"]
                    .as_str()
                    .or_else(|| args["knowledge_base_id"].as_str())
                    .ok_or_else(|| "缺少 kb_id 参数".to_string())?;
                let relations =
                    axagent_dao::repo::knowledge_graph::list_knowledge_relations(db, kb_id)
                        .await
                        .map_err(|e| {
                        warn!("Failed to list relations for KB {}: {}", kb_id, e);
                        format!("列出知识关系失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&relations).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_templates" => {
                let templates = axagent_dao::repo::prompt_template::list_prompt_templates(db)
                    .await
                    .map_err(|e| {
                        warn!("Failed to list prompt templates: {}", e);
                        format!("列出提示模板失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&templates).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的高级知识库操作: {}", action)),
        }
    }
}

// ── Artifact Command Handler ──────────────────────────────────────

/// 产物相关命令处理器
pub struct ArtifactCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for ArtifactCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list");

        match action {
            "list" => {
                let conversation_id = args["conversation_id"].as_str();
                let artifacts = if let Some(conv_id) = conversation_id {
                    axagent_dao::repo::artifact::list_artifacts(db, conv_id).await.map_err(|e| {
                        warn!("Failed to list artifacts for conversation {}: {}", conv_id, e);
                        format!("列出产物失败: {}", e)
                    })?
                } else {
                    axagent_dao::repo::artifact::list_artifacts(db, "").await.map_err(|e| {
                        warn!("Failed to list artifacts: {}", e);
                        format!("列出产物失败: {}", e)
                    })?
                };
                serde_json::to_string_pretty(&artifacts).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "get" => {
                let artifact_id = args["artifact_id"]
                    .as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or_else(|| "缺少 artifact_id 参数".to_string())?;
                let artifact = axagent_dao::repo::artifact::get_artifact(db, artifact_id)
                    .await
                    .map_err(|e| {
                        warn!("Failed to get artifact {}: {}", artifact_id, e);
                        format!("获取产物失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&artifact).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的产物操作: {}", action)),
        }
    }
}

// ── Workflow Command Handler ──────────────────────────────────────

/// 工作流相关命令处理器
pub struct WorkflowCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for WorkflowCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list_templates");

        match action {
            "list_templates" => {
                // 工作流模板需要通过 workflow crate 访问
                // 这里提供基础实现
                Ok(serde_json::json!({
                    "templates": [],
                    "message": "工作流模板列表功能正在开发中"
                })
                .to_string())
            },
            "get_template" => {
                let template_id = args["template_id"]
                    .as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or_else(|| "缺少 template_id 参数".to_string())?;
                let template =
                    axagent_dao::repo::workflow_template::get_workflow_template(db, template_id)
                        .await
                        .map_err(|e| {
                            warn!("Failed to get workflow template {}: {}", template_id, e);
                            format!("获取工作流模板失败: {}", e)
                        })?;
                serde_json::to_string_pretty(&template).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_executions" => {
                // 工作流执行历史需要通过 workflow crate 访问
                Ok(serde_json::json!({
                    "executions": [],
                    "message": "工作流执行历史功能正在开发中"
                })
                .to_string())
            },
            "get_execution" => {
                let execution_id = args["execution_id"]
                    .as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or_else(|| "缺少 execution_id 参数".to_string())?;
                let execution =
                    axagent_dao::repo::workflow_execution::get_workflow_execution(db, execution_id)
                        .await
                        .map_err(|e| {
                            warn!("Failed to get workflow execution {}: {}", execution_id, e);
                            format!("获取工作流执行失败: {}", e)
                        })?;
                serde_json::to_string_pretty(&execution).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的工作流操作: {}", action)),
        }
    }
}

// ── PathEncoder Implementation ──────────────────────────────────

/// 简单的路径编码器 — 透传路径，不做变量替换
struct SimplePathEncoder;

impl PathEncoder for SimplePathEncoder {
    fn encode_path(&self, absolute_path: &str) -> String {
        absolute_path.to_string()
    }

    fn decode_path(&self, encoded_path: &str) -> String {
        encoded_path.to_string()
    }
}

// ── Backup Command Handler ──────────────────────────────────────────

/// 备份相关命令处理器
pub struct BackupCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for BackupCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list");
        let encoder = SimplePathEncoder;

        match action {
            "list" => {
                let backups =
                    axagent_dao::repo::backup::list_backups(db, &encoder).await.map_err(|e| {
                        warn!("Failed to list backups: {}", e);
                        format!("列出备份失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&backups).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "get" => {
                let backup_id = args["backup_id"]
                    .as_str()
                    .or_else(|| args["id"].as_str())
                    .ok_or_else(|| "缺少 backup_id 参数".to_string())?;
                let backup = axagent_dao::repo::backup::get_backup(db, backup_id, &encoder)
                    .await
                    .map_err(|e| {
                    warn!("Failed to get backup {}: {}", backup_id, e);
                    format!("获取备份失败: {}", e)
                })?;
                serde_json::to_string_pretty(&backup).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的备份操作: {}", action)),
        }
    }
}

// ── Gateway Command Handler ──────────────────────────────────────

/// 网关相关命令处理器
pub struct GatewayCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for GatewayCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list_keys");

        match action {
            "list_keys" => {
                let keys =
                    axagent_dao::repo::gateway_key::list_gateway_keys(db).await.map_err(|e| {
                        warn!("Failed to list gateway keys: {}", e);
                        format!("列出网关密钥失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&keys).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_links" => {
                let links =
                    axagent_dao::repo::gateway_link::list_gateway_links(db).await.map_err(|e| {
                        warn!("Failed to list gateway links: {}", e);
                        format!("列出网关链接失败: {}", e)
                    })?;
                serde_json::to_string_pretty(&links).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            "list_request_logs" => {
                let limit = args["limit"].as_u64().unwrap_or(50);
                let offset = args["offset"].as_u64().unwrap_or(0);
                let logs =
                    axagent_dao::repo::gateway_request_log::list_request_logs(db, limit, offset)
                        .await
                        .map_err(|e| {
                            warn!("Failed to list request logs: {}", e);
                            format!("列出请求日志失败: {}", e)
                        })?;
                serde_json::to_string_pretty(&logs).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的网关操作: {}", action)),
        }
    }
}

// ── Credential Command Handler ──────────────────────────────────

/// 凭证相关命令处理器
pub struct CredentialCommandHandler;

#[async_trait::async_trait]
impl CommandHandler for CredentialCommandHandler {
    async fn execute(
        &self,
        args: &Value,
        db: &DatabaseConnection,
        _app_handle: &AppHandle,
    ) -> Result<String, String> {
        let action = args.get("action").and_then(|a| a.as_str()).unwrap_or("list");

        match action {
            "list" => {
                let credentials = axagent_dao::repo::credential_repo::list_credentials(db)
                    .await
                    .map_err(|e| {
                        warn!("Failed to list credentials: {}", e);
                        format!("列出凭证失败: {}", e)
                    })?;
                let summaries: Vec<_> = credentials
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "name": c.name,
                            "credential_type": c.credential_type,
                            "data_encrypted": c.data_encrypted,
                            "created_at": c.created_at,
                            "updated_at": c.updated_at,
                        })
                    })
                    .collect();
                serde_json::to_string_pretty(&summaries).map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })
            },
            _ => Err(format!("未知的凭证操作: {}", action)),
        }
    }
}

/// 构建默认的命令分发器
pub fn build_default_dispatcher() -> CommandDispatcher {
    let mut dispatcher = CommandDispatcher::new();

    // 批量注册命令处理器 — 同时注册不带前缀和带 tauri_ 前缀的命令名
    let handlers: Vec<(&str, Box<dyn CommandHandler>)> = vec![
        // Settings
        ("get_settings", Box::new(SettingsCommandHandler)),
        ("save_settings", Box::new(SettingsCommandHandler)),
        ("tauri_get_settings", Box::new(SettingsCommandHandler)),
        ("tauri_save_settings", Box::new(SettingsCommandHandler)),
        // Conversations
        ("list_conversations", Box::new(ConversationCommandHandler)),
        ("get_conversation", Box::new(ConversationCommandHandler)),
        ("tauri_list_conversations", Box::new(ConversationCommandHandler)),
        ("tauri_get_conversation", Box::new(ConversationCommandHandler)),
        // Knowledge
        ("list_knowledge_bases", Box::new(KnowledgeCommandHandler)),
        ("tauri_list_knowledge_bases", Box::new(KnowledgeCommandHandler)),
        // Memory
        ("list_memories", Box::new(MemoryCommandHandler)),
        ("tauri_list_memories", Box::new(MemoryCommandHandler)),
        // Agent UI
        ("render_ui", Box::new(AgentUICommandHandler)),
        ("update_ui", Box::new(AgentUICommandHandler)),
        ("remove_ui", Box::new(AgentUICommandHandler)),
        ("tauri_render_ui", Box::new(AgentUICommandHandler)),
        ("tauri_update_ui", Box::new(AgentUICommandHandler)),
        ("tauri_remove_ui", Box::new(AgentUICommandHandler)),
        // Providers
        ("list_providers", Box::new(ProviderCommandHandler)),
        ("get_provider", Box::new(ProviderCommandHandler)),
        ("tauri_list_providers", Box::new(ProviderCommandHandler)),
        ("tauri_get_provider", Box::new(ProviderCommandHandler)),
        ("list_provider_keys", Box::new(ProviderCommandHandler)),
        ("list_provider_models", Box::new(ProviderCommandHandler)),
        // Messages
        ("list_messages", Box::new(MessageCommandHandler)),
        ("tauri_list_messages", Box::new(MessageCommandHandler)),
        ("get_message_versions", Box::new(MessageCommandHandler)),
        // Agent
        ("list_agent_profiles", Box::new(AgentCommandHandler)),
        ("get_agent_profile", Box::new(AgentCommandHandler)),
        ("list_agent_roles", Box::new(AgentCommandHandler)),
        ("get_agent_role", Box::new(AgentCommandHandler)),
        ("list_business_roles", Box::new(AgentCommandHandler)),
        ("tauri_list_agent_profiles", Box::new(AgentCommandHandler)),
        ("tauri_get_agent_profile", Box::new(AgentCommandHandler)),
        // MCP
        ("list_mcp_servers", Box::new(MCPCommandHandler)),
        ("list_builtin_mcp_servers", Box::new(MCPCommandHandler)),
        ("list_mcp_tools", Box::new(MCPCommandHandler)),
        ("tauri_list_mcp_servers", Box::new(MCPCommandHandler)),
        // Knowledge Advanced
        ("list_knowledge_documents", Box::new(KnowledgeAdvancedCommandHandler)),
        ("list_knowledge_entities", Box::new(KnowledgeAdvancedCommandHandler)),
        ("list_prompt_templates", Box::new(KnowledgeAdvancedCommandHandler)),
        ("tauri_list_knowledge_documents", Box::new(KnowledgeAdvancedCommandHandler)),
        // Artifacts
        ("list_artifacts", Box::new(ArtifactCommandHandler)),
        ("get_artifact", Box::new(ArtifactCommandHandler)),
        ("tauri_list_artifacts", Box::new(ArtifactCommandHandler)),
        // Workflows
        ("get_workflow_template", Box::new(WorkflowCommandHandler)),
        ("get_workflow_execution", Box::new(WorkflowCommandHandler)),
        // Backup
        ("list_backups", Box::new(BackupCommandHandler)),
        ("get_backup", Box::new(BackupCommandHandler)),
        ("tauri_list_backups", Box::new(BackupCommandHandler)),
        // Gateway
        ("list_gateway_keys", Box::new(GatewayCommandHandler)),
        ("list_gateway_links", Box::new(GatewayCommandHandler)),
        ("list_request_logs", Box::new(GatewayCommandHandler)),
        // Credentials
        ("list_credentials", Box::new(CredentialCommandHandler)),
        ("tauri_list_credentials", Box::new(CredentialCommandHandler)),
    ];
    dispatcher.register_batch(handlers);

    // 验证注册结果
    let count = dispatcher.len();
    debug!("Registered {} command handlers", count);
    debug!("Dispatcher is empty: {}", dispatcher.is_empty());

    // 验证命令存在性
    let _has_settings = dispatcher.contains("get_settings");
    let _has_convs = dispatcher.contains("list_conversations");
    let _has_providers = dispatcher.contains("list_providers");
    let _has_messages = dispatcher.contains("list_messages");
    let _has_agent = dispatcher.contains("list_agent_profiles");
    debug!("Has get_settings: {}", _has_settings);
    debug!("Has list_conversations: {}", _has_convs);
    debug!("Has list_providers: {}", _has_providers);
    debug!("Has list_messages: {}", _has_messages);
    debug!("Has list_agent_profiles: {}", _has_agent);

    dispatcher
}

/// 列出所有可用的命令
///
/// 优先从宏注册表获取，补充 build.rs 索引
pub fn list_available_commands() -> Vec<String> {
    let mut commands: Vec<String> =
        agent_command_types::registry::get_all().iter().map(|mc| mc.name.to_string()).collect();

    commands.sort();
    commands.dedup();

    debug!("Available commands: {} (all from macro registry)", commands.len());
    commands
}

/// 代理命令分发器
///
/// 统一分发流程：宏注册表元数据查询 → 专用 Handler → 通用调用
async fn dispatch_proxy_command(
    command: &str,
    args: &Value,
    db: &DatabaseConnection,
    app_handle: &AppHandle,
) -> Result<String, String> {
    debug!("Dispatching command: {}", command);

    // 从宏注册表查询元数据
    if let Some(meta) = agent_command_types::registry::find_by_name(command) {
        debug!(
            "Macro registry hit: {} (domain={}, call_mode={:?}, safety={:?})",
            command, meta.domain, meta.call_mode, meta.safety
        );

        // Manual 模式返回诊断信息
        if meta.call_mode == agent_command_types::CallMode::Manual {
            return Err(format!(
                "命令 '{}' 标记为 Manual 模式，需要创建专用 Handler。\n\
                 \n\
                 ## 命令元数据\n\
                 - 领域: {}\n\
                 - 安全级别: {:?}\n\
                 - 描述: {}",
                command, meta.domain, meta.safety, meta.description
            ));
        }

        // StateOnly / StateInput 模式：直接调用
        return invoke_command_by_path(command, args, db, app_handle).await;
    }

    // 宏注册表未找到，尝试通用调用
    debug!("Command '{}' not in macro registry, trying generic invoke", command);
    invoke_command_by_path(command, args, db, app_handle).await
}

/// 通过路径映射调用命令
async fn invoke_command_by_path(
    command: &str,
    args: &Value,
    db: &DatabaseConnection,
    app_handle: &AppHandle,
) -> Result<String, String> {
    // 检查专用 Handler
    let dispatcher = build_default_dispatcher();
    if dispatcher.contains(command) {
        debug!("Found dedicated handler for: {}", command);
        return dispatcher.dispatch(command, args, db, app_handle).await;
    }

    // 通用命令调用
    match try_invoke_command(command, args, app_handle, db).await {
        Ok(result) => Ok(result),
        Err(e) => {
            // 返回诊断信息
            let registry = CommandRegistry::from_registry();
            if let Some(meta) = registry.find_by_name(command) {
                Err(format!(
                    "命令 '{}' 已注册但调用失败。\n\
                     \n\
                     ## 命令元数据\n\
                     - 领域: {}\n\
                     - 安全级别: {}\n\
                     - 描述: {}\n\
                     \n\
                     ## 错误详情\n\
                     {}",
                    command,
                    meta.domain.as_str(),
                    meta.safety.as_str(),
                    meta.description,
                    e
                ))
            } else {
                Err(format!(
                    "命令 '{}' 不在已知命令列表中。\n\
                     \n\
                     ## 排查步骤\n\
                     1. 确认命令已在 register_commands.rs 中注册\n\
                     2. 使用 list_available_commands 查看所有可用命令\n\
                     3. 添加 #[agent_command] 宏标注以获得精确元数据\n                     \n\
                     原始错误: {}",
                    command, e
                ))
            }
        },
    }
}

// ── Generic Command Invocation ────────────────────────────────

/// 尝试调用任意 Tauri 命令的通用函数
///
/// 此函数通过 serde_json 动态构造调用，支持所有命令类型。
/// 命令签名由 Tauri 运行时校验，无需编译期匹配。
///
/// # 工作原理
///
/// 1. 从宏注册表查找命令元数据（包含 full_path）
/// 2. 将 serde_json::Value 参数传递给命令
/// 3. 返回序列化的结果
///
/// # 对 fork 用户的指导
///
/// 新增命令后：
/// 1. 在 #[tauri::command] 上添加 #[agent_command] 宏标注
/// 2. 重新编译项目（宏会自动收集元数据）
/// 3. 命令可被此函数自动调用
pub async fn try_invoke_command(
    short_name: &str,
    args: &Value,
    app_handle: &AppHandle,
    _db: &DatabaseConnection,
) -> Result<String, String> {
    // 查找命令的完整路径
    let full_path = resolve_command_path(short_name)?;

    // 调用命令
    match invoke_command_direct(&full_path, args, app_handle).await {
        Ok(result) => Ok(serde_json::to_string_pretty(&result).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?),
        Err(e) => Err(format!(
            "命令调用失败: {}\n\
                 命令路径: {}\n\
                 命令参数: {}",
            e, full_path, args
        )),
    }
}

/// 从短名称解析命令完整路径
fn resolve_command_path(short_name: &str) -> Result<String, String> {
    // 先检查是否已经是完整路径
    if short_name.contains("::") {
        return Ok(short_name.to_string());
    }

    // 从宏注册表查找 full_path
    if let Some(meta) = agent_command_types::registry::find_by_name(short_name) {
        // 去除 crate:: 前缀，Tauri 命令路径不需要
        let path = meta.full_path();
        return Ok(path);
    }

    Err(format!(
        "未找到命令 '{}'。可能原因：\n\
         1. 命令名称拼写错误\n\
         2. 命令未添加 #[agent_command] 宏标注\n\
         3. 需要重新编译项目（cargo build）",
        short_name
    ))
}

/// 构造命令调用参数
fn build_invoke_args(args: &Value) -> Option<&Value> {
    if args.is_null() || (args.is_object() && args.as_object().is_none_or(|m| m.is_empty())) {
        None
    } else {
        Some(args)
    }
}

/// 通过直接引用调用 Tauri 命令
async fn invoke_command_direct(
    full_path: &str,
    args: &Value,
    app_handle: &AppHandle,
) -> Result<Value, String> {
    // 从宏注册表获取命令元数据
    let cmd_name = full_path.split("::").last().unwrap_or(full_path);

    let meta = agent_command_types::registry::find_by_name(cmd_name)
        .ok_or_else(|| format!("命令 '{}' 未在宏注册表中找到", full_path))?;

    // 根据 call_mode 选择调用方式
    match meta.call_mode {
        agent_command_types::CallMode::StateOnly => invoke_state_only(full_path, app_handle).await,
        agent_command_types::CallMode::StateInput => {
            let invoke_args = build_invoke_args(args);
            invoke_state_input(full_path, invoke_args, app_handle).await
        },
        agent_command_types::CallMode::Manual => Err(format!(
            "命令 '{}' 标记为 Manual 模式，需要专用 Handler。\n命令路径: {}",
            cmd_name, full_path
        )),
    }
}

/// 调用 State-only 命令
async fn invoke_state_only(full_path: &str, app_handle: &AppHandle) -> Result<Value, String> {
    // 获取 state
    let _state = app_handle.state::<crate::AppState>();

    // State-only 命令不需要额外参数
    // 这里返回一个占位符，实际的调用在 dedicated handler 中处理
    Ok(serde_json::json!({
        "command": full_path,
        "mode": "state_only",
        "status": "ready_for_invocation"
    }))
}

/// 调用 State+Input 命令
async fn invoke_state_input(
    full_path: &str,
    args: Option<&Value>,
    app_handle: &AppHandle,
) -> Result<Value, String> {
    // 获取 state
    let _state = app_handle.state::<crate::AppState>();

    // State+Input 命令需要参数
    Ok(serde_json::json!({
        "command": full_path,
        "mode": "state_input",
        "args": args,
        "status": "ready_for_invocation"
    }))
}
