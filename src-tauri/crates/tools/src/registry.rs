// SPDX-License-Identifier: AGPL-3.0-only

//! 统一工具注册表
//!
//! 管理所有已注册工具的生命周期：注册、查找、列举、启用/禁用。
//! 集成 MCP 执行、DB 审计记录、使用统计、安全沙箱、权限检查、Hook。

#![allow(clippy::disallowed_types)]

use crate::audit::{AuditEntry, ToolAuditor};
use crate::group_manager::ToolGroupManager;
use crate::hooks::executors::execute_hook;
use crate::hooks::registry::HookRegistry;
use crate::hooks::{HookAction, HookConfig, HookEventType};
pub use crate::mcp_manager::{McpManager, McpServerConfig, McpToolConfig};
use crate::permissions::{PermissionMode, ToolPermissionPolicy};
use crate::recorder::ToolExecutionRecorder;
use crate::stats::ToolUsageStats;
use crate::{Tool, ToolCategory, ToolDomain, ToolError, ToolErrorKind, ToolInfo, ToolResult};
use async_trait::async_trait;
use axagent_harness::ToolExecutionAudit;
use axagent_harness::runtime_types::conversation::ToolExecutor as RuntimeToolExecutor;
// serde_json::Value used for JSON Schema in MCP tool configs
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

pub type SkillToolHandler = Box<dyn Fn(&str) -> Result<String, crate::ToolError> + Send + Sync>;

// ── 全局沙箱策略（PLAN-codex-parity P0-1c） ──
//
// Settings 的 `sandbox_mode` 在启动初始化 / `save_settings` 时写入这里；
// 所有 `UnifiedToolRegistry`（含每次请求临时 `new()` 的实例）构建 ToolContext
// 时自动回退读取，无需逐站点注入。registry 实例上显式设置的
// `sandbox_policy`（测试 / 特殊会话）优先于全局值。
//
// 锁说明：parking_lot RwLock，临界区只有 Arc 克隆，无 await，同步安全。
static GLOBAL_SANDBOX_POLICY: parking_lot::RwLock<Option<Arc<axagent_harness::SandboxPolicy>>> =
    parking_lot::RwLock::new(None);

/// 设置全局沙箱策略（启动初始化 / Settings 变更时调用）。
pub fn set_global_sandbox_policy(policy: axagent_harness::SandboxPolicy) {
    *GLOBAL_SANDBOX_POLICY.write() = Some(Arc::new(policy));
}

/// 读取全局沙箱策略快照（未设置时为 `None`）。
#[must_use]
pub fn global_sandbox_policy() -> Option<Arc<axagent_harness::SandboxPolicy>> {
    GLOBAL_SANDBOX_POLICY.read().clone()
}

// ── 全局审批策略（PLAN-codex-parity P0-2） ──
//
// 与沙箱策略同款模式：Settings 的 `approval_policy` 在启动初始化 /
// `save_settings` 时写入；ToolContext 构建时回退读取。实例显式设置优先。
static GLOBAL_APPROVAL_POLICY: parking_lot::RwLock<Option<Arc<axagent_harness::ApprovalPolicy>>> =
    parking_lot::RwLock::new(None);

/// 设置全局审批策略（启动初始化 / Settings 变更时调用）。
pub fn set_global_approval_policy(policy: axagent_harness::ApprovalPolicy) {
    *GLOBAL_APPROVAL_POLICY.write() = Some(Arc::new(policy));
}

/// 读取全局审批策略快照（未设置时为 `None`，消费方回退默认 `OnRequest`）。
#[must_use]
pub fn global_approval_policy() -> Option<Arc<axagent_harness::ApprovalPolicy>> {
    GLOBAL_APPROVAL_POLICY.read().clone()
}

/// 工具组摘要信息（替代 agent::LocalToolGroup）
#[derive(Debug, Clone)]
pub struct ToolGroupInfo {
    pub group_id: String,
    pub group_name: String,
    pub enabled: bool,
    pub tools: Vec<ToolInfo>,
}

/// 可逆副作用记录器 — 记录一次动态注册/修改操作，卸载时自动回滚。
///
/// # 设计
/// - `target`: 关联的资源名（工具名），用于匹配卸载目标；
/// - `description`: 人类可读的操作描述（审计/日志用）；
/// - `dispose`: 回滚闭包，卸载时调用。
///
/// # 回滚语义
/// 核心回滚（`runtime_tool_sources` 登记移除 + 工具卸载）由
/// `UnifiedToolRegistry::unregister_runtime_tool` 完成；`dispose` 闭包用于
/// 扩展副作用（如能力护照移除 / 审计清理 / 持久化标记失效），由注册方按需提供，
/// 默认 no-op。
///
/// 副作用栈（`UnifiedToolRegistry::effects`）按后进先出（LIFO）清理，
/// 与 Rust 作用域释放语义一致：后注册的先回滚。
pub struct Disposer {
    /// 关联的资源名（工具名），用于匹配卸载目标
    pub target: String,
    /// 操作描述（审计用）
    pub description: String,
    /// 回滚闭包
    dispose: Box<dyn Fn() + Send + Sync + 'static>,
}

impl Disposer {
    /// 创建副作用记录器。
    ///
    /// # 参数
    /// - `target`: 关联资源名（工具名）
    /// - `description`: 操作描述
    /// - `dispose`: 回滚闭包（默认 no-op 可传 `|| {}`）
    pub fn new(
        target: impl Into<String>,
        description: impl Into<String>,
        dispose: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self { target: target.into(), description: description.into(), dispose: Box::new(dispose) }
    }

    /// 执行回滚
    pub fn dispose(&self) {
        (self.dispose)();
    }
}

impl std::fmt::Debug for Disposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Disposer")
            .field("target", &self.target)
            .field("description", &self.description)
            .finish()
    }
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
    disabled: HashSet<String>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools_count", &self.tools.len())
            .field("aliases_count", &self.aliases.len())
            .field("disabled_count", &self.disabled.len())
            .finish()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new(), aliases: HashMap::new(), disabled: HashSet::new() }
    }

    /// 注册一个工具
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();

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

    /// 检查工具是否被禁用（先查主名，再查别名映射后的主名）
    fn is_name_disabled(&self, name: &str) -> bool {
        if self.disabled.contains(name) {
            return true;
        }
        if let Some(primary) = self.aliases.get(name) {
            return self.disabled.contains(primary);
        }
        false
    }

    /// 查找工具（支持别名匹配，自动排除禁用的工具）
    pub fn find(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        if self.is_name_disabled(name) {
            return None;
        }
        if let Some(tool) = self.tools.get(name) {
            return Some(tool);
        }
        if let Some(primary) = self.aliases.get(name) {
            if self.disabled.contains(primary) {
                return None;
            }
            return self.tools.get(primary);
        }
        None
    }

    /// 按类别筛选工具（排除禁用的）
    pub fn by_category(&self, category: ToolCategory) -> Vec<&Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|t| {
                t.category() == category && t.is_enabled() && !self.disabled.contains(t.name())
            })
            .collect()
    }

    /// 列出所有已启用且未禁用工具的信息
    pub fn list_all(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .filter(|t| t.is_enabled() && !self.disabled.contains(t.name()))
            .map(|t| ToolInfo::from_tool(t.as_ref()))
            .collect()
    }

    /// 列出所有工具（含禁用）
    pub fn list_all_with_disabled(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|t| {
                let mut info = ToolInfo::from_tool(t.as_ref());
                if self.disabled.contains(t.name()) {
                    info.enabled = false;
                }
                info
            })
            .collect()
    }

    /// 获取只读工具列表（排除禁用的）
    pub fn read_only_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .filter(|t| t.is_read_only() && t.is_enabled() && !self.disabled.contains(t.name()))
            .map(|t| ToolInfo::from_tool(t.as_ref()))
            .collect()
    }

    /// 获取可并发工具列表（排除禁用的）
    pub fn concurrency_safe_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .filter(|t| {
                t.is_concurrency_safe() && t.is_enabled() && !self.disabled.contains(t.name())
            })
            .map(|t| ToolInfo::from_tool(t.as_ref()))
            .collect()
    }

    /// 禁用工具
    pub fn disable(&mut self, name: &str) {
        if let Some(primary) = self.aliases.get(name) {
            self.disabled.insert(primary.clone());
        } else {
            self.disabled.insert(name.to_string());
        }
    }

    /// 启用工具
    pub fn enable(&mut self, name: &str) {
        self.disabled.remove(name);
        if let Some(primary) = self.aliases.get(name) {
            self.disabled.remove(primary);
        }
    }

    /// 批量按类别禁用
    pub fn disable_category(&mut self, category: ToolCategory) {
        for tool in self.tools.values() {
            if tool.category() == category {
                self.disabled.insert(tool.name().to_string());
            }
        }
    }

    /// 是否已注册且未被禁用
    pub fn contains(&self, name: &str) -> bool {
        if self.is_name_disabled(name) {
            return false;
        }
        if self.tools.contains_key(name) {
            return true;
        }
        if let Some(primary) = self.aliases.get(name) {
            return !self.disabled.contains(primary) && self.tools.contains_key(primary);
        }
        false
    }

    /// 工具总数（不含禁用）
    pub fn len(&self) -> usize {
        self.tools.values().filter(|t| !self.disabled.contains(t.name())).count()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 已注册总数（含禁用）
    pub fn total_registered(&self) -> usize {
        self.tools.len()
    }

    /// 移除工具
    pub fn unregister(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        let primary = self.aliases.get(name).cloned().unwrap_or_else(|| name.to_string());
        self.aliases.retain(|_, v| v != &primary);
        self.disabled.remove(&primary);
        self.tools.remove(&primary)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Harness ToolRegistry trait 实现
// ============================================================

#[async_trait]
impl axagent_harness::ToolRegistry for ToolRegistry {
    fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        if self.is_name_disabled(name) {
            return None;
        }
        if let Some(primary) = self.aliases.get(name) {
            if self.disabled.contains(primary) {
                return None;
            }
            return self.tools.get(primary).cloned();
        }
        self.tools.get(name).cloned()
    }

    fn find(&self, name: &str) -> Option<Arc<dyn Tool>> {
        if self.is_name_disabled(name) {
            return None;
        }
        if let Some(tool) = self.tools.get(name) {
            return Some(tool.clone());
        }
        if let Some(primary) = self.aliases.get(name) {
            if self.disabled.contains(primary) {
                return None;
            }
            return self.tools.get(primary).cloned();
        }
        None
    }

    fn list(&self) -> Vec<ToolInfo> {
        self.list_all()
    }

    fn list_by_category(&self, category: ToolCategory) -> Vec<ToolInfo> {
        self.by_category(category).into_iter().map(|t| ToolInfo::from_tool(t.as_ref())).collect()
    }

    fn is_disabled(&self, name: &str) -> bool {
        self.is_name_disabled(name)
    }
}

/// 工具注册表构建器，方便链式注册
pub struct ToolRegistryBuilder {
    registry: ToolRegistry,
}

impl ToolRegistryBuilder {
    pub fn new() -> Self {
        Self { registry: ToolRegistry::new() }
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
// 统一 ToolRegistry（含 MCP + 审计 + 统计）
// ============================================================

// McpServerConfig / McpToolConfig 已迁移至 mcp_manager 模块

/// 渐进式披露工具白名单（L0 索引层 + L1 定义层）。
///
/// 认知编排执行阶段（`execution_mode=Some`）会跳过全量 built-in tools 收集，
/// 但 LLM 仍需自主完成「查看目录 → 展开定义」的按需披露，故按名字点名放行这几个。
///
/// 之所以必须按名放行而非按域放行：这组工具的 `domain` 是 `General`，而 `General`
/// 域下含 `Bash` / `FileWrite` / `FileEdit` / `DeleteFile` 等写操作工具，
/// 加域进 `active_domains` 等于把危险操作一并暴露给编排执行阶段。
pub const DISCLOSURE_TOOLS: [&str; 7] = [
    "SkillsList",
    "SkillView",
    "SkillReference",
    "DiscoverSkills",
    "CapabilityView",
    "CapabilityLoad",
    "CapabilityBrowse",
];

/// 屏幕感知能力的承载工具名（`settings.screen_perception_enabled` 的落点）。
///
/// **两个消费点必须共用本常量**（禁区 12：禁止重复字面量）：
/// - 可见性侧：`commands/agent/mod.rs` 工具策略块把它并入 blocked，使 schema 不下发 LLM
/// - 执行期侧：同文件 `tool_registry.tools.disable(SCREEN_PERCEPTION_TOOL)`
///
/// 两侧缺一不可 —— 只做执行期会让 LLM「看得到却调不动」（每次调用都注定失败），
/// 只做可见性则挡不住手工构造的调用。
///
/// 常量与真实注册名的一致性由测试 `screen_perception_tool_matches_registered_name` 锁定：
/// 若有人改了 `ComputerUseTool::name()` 而漏改此常量，该测试会红。
pub const SCREEN_PERCEPTION_TOOL: &str = "ComputerUse";

/// 判断工具名是否对 **profile 黑名单**（`AgentProfile.disallowed_tools`）免疫。
///
/// **为什么需要豁免**：`DISCLOSURE_TOOLS` 是编排器「发现能力 / 展开定义」的唯一入口。
/// 它们被某个 profile 静默禁用后，外在表现是「编排器发现不了任何能力」，且极难归因到
/// 「原来是某个 profile 的配置」——故障现象与根因相隔两层。故让它们对 profile 黑名单
/// 免疫，把这类元工具的可见性从普通配置中摘出来。
///
/// **免疫不等于无约束**：① registry 层的 `disable()`（用户在设置里关掉）照常生效，
/// 不在此豁免范围；② 执行期安全由 registry 的 `blocked_tools`（`tools.rs` 中拦截）
/// 与各工具自身的权限模型兜底，可见性过滤从来不是安全边界。
///
/// **两侧必须共用本函数**（禁区 12：禁止语义漂移）：
/// - `commands/agent/mod.rs` 的 `apply_tool_policy` —— 构造真正发给 LLM 的工具列表
/// - `commands/local_tool.rs` 的 `get_tool_count` —— UI 展示的可用工具数
///
/// 若只在前者豁免，会出现「工具在列表里但计数不计」；只在后者豁免则相反。
pub fn is_disclosure_immune(name: &str) -> bool {
    DISCLOSURE_TOOLS.contains(&name)
}

/// 完整的统一工具注册表
pub struct UnifiedToolRegistry {
    /// Tool trait 实现的工具（原生 + 已迁移旧工具）
    pub tools: ToolRegistry,
    /// MCP 工具管理器（M-02 拆分）
    pub mcp: McpManager,
    /// 工具组管理器（M-02 拆分）
    pub groups: ToolGroupManager,
    /// 执行记录器
    pub recorder: Option<ToolExecutionRecorder>,
    /// 使用统计
    pub usage_stats: ToolUsageStats,
    /// 权限策略（集成到执行路径）
    ///
    /// 用 `Arc<Mutex<..>>` 包裹以支持 `&self` 执行路径：trait `ToolRegistry::execute_tool`
    /// 仅持有 `&self`，而 `PermissionPolicy::authorize` 需可变访问内部 `DenialTracker`。
    pub permission_policy: Arc<Mutex<ToolPermissionPolicy>>,
    /// Hook 注册表（集成到执行路径）
    pub hook_registry: HookRegistry,
    /// 工具调用审计器
    pub auditor: Arc<ToolAuditor>,
    /// 安全沙箱配置（路径/命令/网络控制）
    sandbox: Arc<crate::AccessPolicyValidator>,
    /// 权限控制
    allowed_tools: HashSet<String>,
    blocked_tools: HashSet<String>,
    strict_mode: bool,
    /// 会话上下文
    conversation_id: Option<String>,
    message_id: Option<String>,
    /// 当前工作目录（来自 agent session 的 workspace cwd）
    pub working_dir: String,
    /// 搜索/网络配置（通过 ToolContext.extra 传递给工具）
    pub tool_extra: HashMap<String, String>,
    /// 注册的 Skill 工具：name → handler（register_skill_tool 填充）
    #[allow(clippy::disallowed_types)]
    pub skill_handlers: HashMap<String, SkillToolHandler>,
    /// 用户提问桥接器（AskUserQuestion 工具阻塞等待用户输入）
    pub ask_user_bridge: Option<Arc<dyn axagent_harness::AskUserBridge>>,
    /// Agent 作用域标识 —— 透传进 `ToolContext.agent_id`，供会话状态的
    /// 多 Agent 隔离使用（能力加载状态按 agent 分键）。
    pub agent_id: Option<String>,
    /// 运行时动态工具集 —— 透传进 `ToolContext.dynamic_tools`，是
    /// `CapabilityLoad` 把工具定义追加进下一轮 LLM 请求的唯一出口。
    pub dynamic_tools: Option<axagent_harness::DynamicToolSet>,

    /// RL 策略工具排名器（可选），在 `get_chat_tools()` 返回前重排工具列表。
    /// 高权重工具排前面，间接影响 LLM 的工具选择偏好。
    pub tool_ranker: Option<Arc<dyn axagent_harness::ToolRanker>>,
    /// 运行时动态注册的工具来源跟踪：工具名 → 来源标识（如 "runtime_evolution"）。
    /// 仅存在于 `runtime_tool_sources` 中的工具才允许被 `unregister_runtime_tool` 卸载，
    /// 原生内置工具与 MCP 工具不受影响。
    pub runtime_tool_sources: HashMap<String, String>,
    /// OS 级沙箱策略（PLAN-codex-parity P0-1）—— 透传进 `ToolContext.sandbox`，
    /// Shell 类工具据此决定是否在受限子进程中执行。`None` 保持旧行为。
    pub sandbox_policy: Option<Arc<axagent_harness::SandboxPolicy>>,
    /// 审批策略（PLAN-codex-parity P0-2）—— 透传进 `ToolContext.approval_policy`，
    /// Shell 类工具据此决定敏感操作是跑、问用户还是拒绝。`None` 走全局/默认 `on-request`。
    pub approval_policy: Option<Arc<axagent_harness::ApprovalPolicy>>,
    /// 副作用栈：记录所有动态注册/修改操作，卸载时自动回滚（后进先出）。
    ///
    /// 每个 `register_runtime_tool` 会登记一个 `Disposer`；`unregister_runtime_tool`
    /// 按目标名匹配并执行回滚；`cleanup_runtime_effects` 全量清理（应用退出前）。
    ///
    /// 注意：`Disposer` 的回滚闭包不可 Clone，故 `Clone` 时重置为空
    /// （与 `skill_handlers` 的处理方式一致，克隆体不携带副作用）。
    pub effects: Vec<Disposer>,
}

impl Clone for UnifiedToolRegistry {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            mcp: self.mcp.clone(),
            groups: self.groups.clone(),
            recorder: self.recorder.clone(),
            usage_stats: self.usage_stats.clone(),
            permission_policy: self.permission_policy.clone(),
            hook_registry: self.hook_registry.clone(),
            auditor: self.auditor.clone(),
            sandbox: self.sandbox.clone(),
            allowed_tools: self.allowed_tools.clone(),
            blocked_tools: self.blocked_tools.clone(),
            strict_mode: self.strict_mode,
            conversation_id: self.conversation_id.clone(),
            message_id: self.message_id.clone(),
            working_dir: self.working_dir.clone(),
            tool_extra: self.tool_extra.clone(),
            skill_handlers: HashMap::new(), // handlers 不可 Clone，clone 时重置为空
            ask_user_bridge: self.ask_user_bridge.clone(),
            agent_id: self.agent_id.clone(),
            dynamic_tools: self.dynamic_tools.clone(),
            tool_ranker: self.tool_ranker.clone(),
            runtime_tool_sources: self.runtime_tool_sources.clone(),
            sandbox_policy: self.sandbox_policy.clone(),
            approval_policy: self.approval_policy.clone(),
            effects: Vec::new(), // Disposer 不可 Clone，克隆体不携带副作用
        }
    }
}

impl UnifiedToolRegistry {
    /// 创建默认安全沙箱配置（向后兼容宽松模式：允许网络、不限制路径/命令，可通过 configure_sandbox 收紧）
    fn default_sandbox(working_dir: &str) -> Arc<crate::AccessPolicyValidator> {
        let config = crate::SandboxConfig {
            network_enabled: true,
            allowed_paths: vec![std::path::PathBuf::from(working_dir)],
            ..Default::default()
        };
        Arc::new(crate::AccessPolicyValidator::new(config))
    }

    /// 配置安全沙箱
    pub fn configure_sandbox(&mut self, config: crate::SandboxConfig) {
        self.sandbox = Arc::new(crate::AccessPolicyValidator::new(config));
    }

    /// 设置 OS 级沙箱策略（PLAN-codex-parity P0-1）。
    ///
    /// 与 [`Self::configure_sandbox`]（路径/命令白名单校验器）互补：
    /// 本策略由 Shell 类工具消费，决定子进程是否在受限 token 下执行。
    pub fn set_sandbox_policy(&mut self, policy: axagent_harness::SandboxPolicy) {
        self.sandbox_policy = Some(Arc::new(policy));
    }

    /// 设置审批策略（PLAN-codex-parity P0-2），透传进 `ToolContext.approval_policy`。
    pub fn set_approval_policy(&mut self, policy: axagent_harness::ApprovalPolicy) {
        self.approval_policy = Some(Arc::new(policy));
    }

    /// 临时禁用单个工具（仅内存，不持久化到 DB）。
    ///
    /// 与 `toggle_tool`（UI 设置持久化）不同，本方法用于子代理等临时场景：
    /// 工具名单隔离、防递归等，注册表实例销毁后状态即消失。
    pub fn disable_tool(&mut self, name: &str) {
        self.groups.disabled_tools.insert(name.to_string());
        self.tools.disable(name);
    }

    /// 将 UnifiedToolRegistry 的 disabled_tools 同步到内层 ToolRegistry
    fn sync_disabled_to_inner(&mut self) {
        for tool_name in &self.groups.disabled_tools {
            self.tools.disable(tool_name);
        }
    }

    /// 检查工具是否允许执行（包含组启用、单工具禁用、黑白名单检查）
    fn check_tool_enabled(&self, tool_name: &str) -> Result<(), crate::ToolError> {
        if self.blocked_tools.contains(tool_name) {
            return Err(ToolError::permission_denied(tool_name, "工具在黑名单中"));
        }
        if self.strict_mode
            && !self.allowed_tools.is_empty()
            && !self.allowed_tools.contains(tool_name)
        {
            return Err(ToolError::permission_denied(tool_name, "严格模式下工具不在白名单中"));
        }

        if let Some(tool) = self.tools.find(tool_name) {
            let info = ToolInfo::from_tool(tool.as_ref());
            if !self.groups.is_tool_enabled(&info) {
                return Err(ToolError::permission_denied(
                    tool_name,
                    "工具所属组已被禁用或工具已被单独禁用",
                ));
            }
        } else if let Some((mcp_key, _config)) = self.resolve_mcp_tool(tool_name)
            && self.groups.disabled_tools.contains(&mcp_key)
        {
            return Err(ToolError::permission_denied(tool_name, "MCP 工具已被禁用"));
        }
        Ok(())
    }

    /// 创建并初始化：自动注册全部本地工具（数量见 tools/mod.rs register_all()）
    pub fn new() -> Self {
        let working_dir = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let mut reg = Self {
            tools: ToolRegistry::new(),
            mcp: McpManager::new(),
            groups: ToolGroupManager::new(),
            recorder: None,
            usage_stats: ToolUsageStats::new(),
            permission_policy: Arc::new(Mutex::new(ToolPermissionPolicy::new(
                PermissionMode::WorkspaceWrite,
            ))),
            hook_registry: HookRegistry::new(),
            auditor: Arc::new(ToolAuditor::default()),
            sandbox: Self::default_sandbox(&working_dir),
            sandbox_policy: None,
            approval_policy: None,
            allowed_tools: HashSet::new(),
            blocked_tools: HashSet::new(),
            strict_mode: false,
            conversation_id: None,
            message_id: None,
            working_dir,
            tool_extra: HashMap::new(),
            skill_handlers: HashMap::new(),
            ask_user_bridge: None,
            agent_id: None,
            dynamic_tools: None,
            tool_ranker: None,
            runtime_tool_sources: HashMap::new(),
            effects: Vec::new(),
        };
        reg.init_all();
        reg
    }

    /// 初始化：注册全部本地工具（约 138 个，来自 tools/ 下 43 个模块），配置默认权限
    pub fn init_all(&mut self) {
        // 第一层：注册全部本地 Rust Tool trait 实现
        crate::tools::register_all(&mut self.tools);

        // 配置默认工具级权限要求
        self.permission_policy = Arc::new(Mutex::new(
            ToolPermissionPolicy::new(PermissionMode::WorkspaceWrite)
                .with_tool_requirement("FileRead", PermissionMode::ReadOnly)
                .with_tool_requirement("Glob", PermissionMode::ReadOnly)
                .with_tool_requirement("Grep", PermissionMode::ReadOnly)
                .with_tool_requirement("WebFetch", PermissionMode::ReadOnly)
                .with_tool_requirement("WebSearch", PermissionMode::ReadOnly)
                .with_tool_requirement("FileWrite", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("FileEdit", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("Bash", PermissionMode::DangerFullAccess)
                .with_tool_requirement("NotebookEdit", PermissionMode::WorkspaceWrite)
                .with_tool_requirement("ComputerUse", PermissionMode::DangerFullAccess),
        ));
    }

    /// 已启用工具总数（排除禁用的）
    pub fn count_enabled_tools(&self) -> u32 {
        self.tools
            .tools
            .iter()
            .filter(|(name, tool)| tool.is_enabled() && !self.groups.disabled_tools.contains(*name))
            .count() as u32
    }

    pub fn with_recorder(mut self, recorder: ToolExecutionRecorder) -> Self {
        self.recorder = Some(recorder);
        self
    }

    /// 便利方法：从 `&DatabaseConnection` 直接构建 `ToolExecutionRecorder`。
    ///
    /// 之前在 `commands/agent.rs`、`commands/plan.rs` 都有
    /// `with_recorder(ToolExecutionRecorder::new(Arc::new(db.clone())))` 的 2 步链重复。
    /// 收敛为单步。
    pub fn with_recorder_from_db(mut self, _db: &sea_orm::DatabaseConnection) -> Self {
        self.recorder = Some(ToolExecutionRecorder::new());
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

    /// 将所有已注册工具转为 ChatTool 格式（供 LLM 使用）
    /// 尊重 group_enabled 和 disabled_tools 设置。
    pub fn get_chat_tools(&self) -> Vec<axagent_harness::types::ChatTool> {
        let mut out = Vec::new();
        for info in self.tools.list_all() {
            if !self.groups.is_tool_enabled(&info) {
                continue;
            }
            out.push(axagent_harness::types::ChatTool {
                r#type: "function".into(),
                function: axagent_harness::types::ChatToolFunction {
                    name: info.name.clone(),
                    description: Some(info.description.clone()),
                    parameters: Some(info.input_schema.clone()),
                },
            });
        }
        for (key, config) in &self.mcp.mcp_tools {
            if self.tools.disabled.contains(key) {
                continue;
            }
            out.push(axagent_harness::types::ChatTool {
                r#type: "function".into(),
                function: axagent_harness::types::ChatToolFunction {
                    name: key.clone(),
                    description: Some(config.description.clone().unwrap_or_default()),
                    parameters: config.input_schema.clone(),
                },
            });
        }
        if let Some(ref ranker) = self.tool_ranker {
            out = ranker.rank_tools(out);
        }
        out
    }

    /// 获取类别筛选后的 ChatTool 列表（用于根据 permission mode 限制工具）
    /// 也尊重 group_enabled 和 disabled_tools 设置。
    pub fn get_chat_tools_filtered(
        &self,
        mode: &crate::permissions::PermissionMode,
    ) -> Vec<axagent_harness::types::ChatTool> {
        let mut out = Vec::new();
        for info in self.tools.list_all() {
            if !self.groups.is_tool_enabled(&info) {
                continue;
            }
            let allowed = match mode {
                crate::permissions::PermissionMode::ReadOnly => info.is_read_only,
                crate::permissions::PermissionMode::Allow => true,
                crate::permissions::PermissionMode::DangerFullAccess => true,
                crate::permissions::PermissionMode::WorkspaceWrite => true,
                crate::permissions::PermissionMode::Prompt => true,
            };
            if allowed {
                out.push(axagent_harness::types::ChatTool {
                    r#type: "function".into(),
                    function: axagent_harness::types::ChatToolFunction {
                        name: info.name.clone(),
                        description: Some(info.description.clone()),
                        parameters: Some(info.input_schema.clone()),
                    },
                });
            }
        }
        let mcp_allowed = !matches!(mode, crate::permissions::PermissionMode::ReadOnly);
        if mcp_allowed {
            for (key, config) in &self.mcp.mcp_tools {
                if self.tools.disabled.contains(key) {
                    continue;
                }
                out.push(axagent_harness::types::ChatTool {
                    r#type: "function".into(),
                    function: axagent_harness::types::ChatToolFunction {
                        name: key.clone(),
                        description: Some(config.description.clone().unwrap_or_default()),
                        parameters: config.input_schema.clone(),
                    },
                });
            }
        }
        out
    }

    pub fn get_chat_tools_for_domains(
        &self,
        domains: &std::collections::HashSet<ToolDomain>,
        mode: Option<&crate::permissions::PermissionMode>,
    ) -> Vec<axagent_harness::types::ChatTool> {
        let mut out = Vec::new();
        for info in self.tools.list_all() {
            if !self.groups.is_tool_enabled(&info) {
                continue;
            }
            // ★ 领域过滤
            if !domains.contains(&info.domain) {
                continue;
            }
            // ★ 权限模式过滤（可选）
            if let Some(mode) = mode {
                let allowed = match mode {
                    crate::permissions::PermissionMode::ReadOnly => info.is_read_only,
                    crate::permissions::PermissionMode::Allow => true,
                    crate::permissions::PermissionMode::DangerFullAccess => true,
                    crate::permissions::PermissionMode::WorkspaceWrite => true,
                    crate::permissions::PermissionMode::Prompt => true,
                };
                if !allowed {
                    continue;
                }
            }
            out.push(axagent_harness::types::ChatTool {
                r#type: "function".into(),
                function: axagent_harness::types::ChatToolFunction {
                    name: info.name.clone(),
                    description: Some(info.description.clone()),
                    parameters: Some(info.input_schema.clone()),
                },
            });
        }
        // MCP 工具始终按原逻辑添加（领域无关）
        let mcp_allowed =
            mode.is_none_or(|m| !matches!(m, crate::permissions::PermissionMode::ReadOnly));
        if mcp_allowed {
            for (key, config) in &self.mcp.mcp_tools {
                if self.tools.disabled.contains(key) {
                    continue;
                }
                out.push(axagent_harness::types::ChatTool {
                    r#type: "function".into(),
                    function: axagent_harness::types::ChatToolFunction {
                        name: key.clone(),
                        description: Some(config.description.clone().unwrap_or_default()),
                        parameters: config.input_schema.clone(),
                    },
                });
            }
        }
        out
    }

    /// 按名字白名单点名放行工具，返回完整 schema。
    ///
    /// 与 `get_chat_tools_for_domains` 的区别：不做领域过滤，只认名字。
    /// 认知编排执行阶段需要披露工具，但绝不能按 `General` 域放行 ——
    /// 那会把 `Bash` / `FileWrite` 等写操作一并暴露给编排阶段。
    ///
    /// 输出按名字排序，避免 `HashSet` / `HashMap` 迭代顺序导致 system prompt 抖动。
    pub fn get_chat_tools_by_names<'a, I>(&self, names: I) -> Vec<axagent_harness::types::ChatTool>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let wanted: HashSet<&str> = names.into_iter().collect();
        let mut out: Vec<axagent_harness::types::ChatTool> = self
            .tools
            .list_all()
            .into_iter()
            .filter(|info| wanted.contains(info.name.as_str()))
            .filter(|info| self.groups.is_tool_enabled(info))
            .map(|info| axagent_harness::types::ChatTool {
                r#type: "function".into(),
                function: axagent_harness::types::ChatToolFunction {
                    name: info.name.clone(),
                    description: Some(info.description.clone()),
                    parameters: Some(info.input_schema.clone()),
                },
            })
            .collect();
        out.sort_unstable_by(|a, b| a.function.name.cmp(&b.function.name));
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

    /// 设置 Agent 作用域（多 Agent 隔离），透传进 `ToolContext.agent_id`。
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// 设置运行时动态工具集，透传进 `ToolContext.dynamic_tools`。
    ///
    /// `CapabilityLoad` 依赖它把按需加载的能力变成 LLM 可调用的 function；
    /// 不注入时加载只能落状态，执行闭环不生效。
    pub fn with_dynamic_tools(mut self, set: axagent_harness::DynamicToolSet) -> Self {
        self.dynamic_tools = Some(set);
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
    pub async fn load_enabled_state(&mut self, _db: &sea_orm::DatabaseConnection) {
        // 加载分类启用状态
        let key = "tool_groups_enabled";
        let result = axagent_harness::repositories::settings_repository().get_setting(key).await;

        if let Ok(Some(value)) = result
            && let Ok(map) = serde_json::from_str::<HashMap<String, bool>>(&value)
        {
            self.groups.group_enabled = map;
        }

        // 加载单工具禁用列表
        let dt_key = "disabled_tools";
        let dt_result =
            axagent_harness::repositories::settings_repository().get_setting(dt_key).await;

        if let Ok(Some(value)) = dt_result
            && let Ok(list) = serde_json::from_str::<Vec<String>>(&value)
        {
            self.groups.disabled_tools = list.into_iter().collect();
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
            // AxInvest 专属：金融计算工具组（ToolCategory::Finance 通过 default_group() 返回 "builtin-finance"）。
            // 历史遗漏：harness/tool.rs 已定义 Finance 分组，但未在此处注册显示名，
            // 导致前端工具组管理页面看不到该组的显示名映射，用户无法在 UI 上单独禁用金融计算工具组。
            ("builtin-finance", "金融计算"),
        ];
        for (gid, gname) in &default_groups {
            self.groups.group_names.entry(gid.to_string()).or_insert_with(|| gname.to_string());
        }

        self.sync_disabled_to_inner();
    }

    /// 获取工具组列表
    pub fn get_tool_groups(&self) -> Vec<ToolGroupInfo> {
        let mut groups_map: HashMap<String, (String, bool, Vec<ToolInfo>)> = HashMap::new();
        for info in self.tools.list_all() {
            let gid = info.category.default_group();
            let entry = groups_map.entry(gid.to_string()).or_insert_with(|| {
                let name =
                    self.groups.group_names.get(gid).cloned().unwrap_or_else(|| gid.to_string());
                let enabled = self.groups.group_enabled.get(gid).copied().unwrap_or(true);
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
        _db: &sea_orm::DatabaseConnection,
        gid: &str,
    ) -> Result<bool, String> {
        let current = self.groups.group_enabled.get(gid).copied().unwrap_or(true);
        let new_state = !current;
        self.groups.group_enabled.insert(gid.to_string(), new_state);

        let key = "tool_groups_enabled";
        let serialized =
            serde_json::to_string(&self.groups.group_enabled).map_err(|e| e.to_string())?;
        axagent_harness::repositories::settings_repository()
            .set_setting(key, &serialized)
            .await
            .map_err(|e| e.to_string())?;

        Ok(new_state)
    }

    /// 切换单个工具启用状态并持久化到 DB
    pub async fn toggle_tool(
        &mut self,
        _db: &sea_orm::DatabaseConnection,
        tool_name: &str,
    ) -> Result<bool, String> {
        let currently_disabled = self.groups.disabled_tools.contains(tool_name);
        if currently_disabled {
            self.groups.disabled_tools.remove(tool_name);
            self.tools.enable(tool_name);
        } else {
            self.groups.disabled_tools.insert(tool_name.to_string());
            self.tools.disable(tool_name);
        }

        let key = "disabled_tools";
        let serialized =
            serde_json::to_string(&self.groups.disabled_tools.iter().collect::<Vec<_>>())
                .map_err(|e| e.to_string())?;
        axagent_harness::repositories::settings_repository()
            .set_setting(key, &serialized)
            .await
            .map_err(|e| e.to_string())?;

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
                    ToolCategory::Finance => "builtin-finance",
                };
                self.groups.group_enabled.get(gid).copied().unwrap_or(true)
            })
            .map(|info| info.name)
            .collect()
    }

    pub fn register_skill_tool(&mut self, name: impl Into<String>, handler: SkillToolHandler) {
        self.skill_handlers.insert(name.into(), handler);
    }

    /// 运行时动态注册一个工具（如进化引擎自动生成的工具）。
    ///
    /// - 若同名工具已存在（无论原生 / MCP / 已注册的运行时工具），返回
    ///   `ToolError::new` 且 `error_code = "tool.{name}.duplicateRegistration"`，不会覆盖既有工具。
    /// - 注册成功后记录来源标识，供 `unregister_runtime_tool` 与重启自动加载判断。
    pub fn register_runtime_tool(
        &mut self,
        tool: Arc<dyn Tool>,
        source: impl Into<String>,
    ) -> Result<(), crate::ToolError> {
        let name = tool.name().to_string();
        if self.tools.find(&name).is_some()
            || self.resolve_mcp_tool(&name).is_some()
            || self.runtime_tool_sources.contains_key(&name)
        {
            return Err(ToolError {
                message: format!("工具 '{name}' 已存在，无法重复注册"),
                kind: crate::ToolErrorKind::ExecutionFailed,
                error_code: axagent_harness::error_codes::tool::REGISTRATION_DUPLICATE.to_string(),
            });
        }
        self.tools.register(tool);
        self.runtime_tool_sources.insert(name.clone(), source.into());
        // 登记副作用（卸载时回滚）。核心回滚（来源移除 + 工具卸载）由
        // `unregister_runtime_tool` 完成；此处闭包用于扩展副作用，默认 no-op。
        self.effects.push(Disposer::new(name.clone(), format!("运行时注册工具 '{name}'"), || {}));
        Ok(())
    }

    /// 运行时卸载一个动态注册的工具。
    ///
    /// 仅允许卸载此前经 `register_runtime_tool` 注册的工具（在 `runtime_tool_sources`
    /// 中有来源记录）。原生内置工具与 MCP 工具无法通过此入口卸载。
    ///
    /// 卸载时按后进先出（LIFO）执行与该工具名匹配的副作用回滚。
    ///
    /// 一个工具可能登记多个 Disposer（`register_runtime_tool` 内置登记 + wiring
    /// 层 `push_effect` 附加的护照移除等），卸载时**全部**匹配项都要回滚，
    /// 保证副作用栈不残留。
    pub fn unregister_runtime_tool(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        self.runtime_tool_sources.remove(name)?;
        let tool = self.tools.unregister(name);
        // 回滚该工具的全部副作用：从后往前匹配 target（后进先出）
        let mut i = self.effects.len();
        while i > 0 {
            i -= 1;
            if self.effects[i].target == name {
                let disposer = self.effects.remove(i);
                disposer.dispose();
            }
        }
        tool
    }

    /// 运行时动态注册工具的来源集合（工具名 → 来源标识）
    pub fn runtime_tool_sources(&self) -> &HashMap<String, String> {
        &self.runtime_tool_sources
    }

    /// 按来源前缀批量卸载运行时工具（如删除工作流时清理 `workflow:<id>` 来源）。
    ///
    /// 仅卸载经 `register_runtime_tool` 注册的工具（在 `runtime_tool_sources`
    /// 有来源记录）；原生内置与 MCP 工具不受影响。返回卸载数量。
    /// 逐个走 `unregister_runtime_tool`，保证副作用栈（Disposer）完整回滚。
    pub fn unregister_runtime_tools_by_source(&mut self, source_prefix: &str) -> usize {
        let names: Vec<String> = self
            .runtime_tool_sources
            .iter()
            .filter(|(_, src)| src.starts_with(source_prefix))
            .map(|(name, _)| name.clone())
            .collect();
        let mut count = 0;
        for name in names {
            if self.unregister_runtime_tool(&name).is_some() {
                count += 1;
            }
        }
        count
    }

    /// 登记一个扩展副作用（供 wiring 层附加回滚逻辑，如护照移除 / 审计清理）。
    pub fn push_effect(&mut self, disposer: Disposer) {
        self.effects.push(disposer);
    }

    /// 当前副作用栈长度（调试/审计用）
    pub fn effects_len(&self) -> usize {
        self.effects.len()
    }

    /// 全量清理副作用栈（后进先出），并返回已清理数量。
    ///
    /// 用于应用退出前或用户显式回滚全部进化产物时，保证动态注册不留残余：
    /// 同步卸载全部运行时工具并清空来源记录，再依次回滚副作用。
    pub fn cleanup_runtime_effects(&mut self) -> usize {
        // 1) 卸载全部运行时工具 + 清空来源记录
        let names: Vec<String> = self.runtime_tool_sources.keys().cloned().collect();
        for name in &names {
            self.tools.unregister(name);
        }
        self.runtime_tool_sources.clear();
        // 2) 回滚副作用栈（后进先出）
        let count = self.effects.len();
        while let Some(disposer) = self.effects.pop() {
            disposer.dispose();
        }
        count
    }

    /// 从 skill_handlers 执行注册的 Skill 工具
    fn execute_skill_tool(
        &self,
        tool_name: &str,
        input: &str,
    ) -> Option<Result<ToolResult, crate::ToolError>> {
        let handler = self.skill_handlers.get(tool_name)?;
        match handler(input) {
            Ok(content) => Some(Ok(ToolResult {
                content,
                is_error: false,
                truncated: false,
                metadata: Some(serde_json::json!({
                    "source": "registered_skill",
                    "tool_name": tool_name,
                })),
                duration_ms: None,
                progress: Vec::new(),
            })),
            Err(_e) => Some(Err(ToolError::execution_failed(tool_name))),
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
        self.mcp.mcp_tools.insert(
            format!("{}/{}", server_id, tool_name),
            McpToolConfig {
                server_id: server_id.clone(),
                server_name,
                tool_name,
                description,
                input_schema,
            },
        );
        self.mcp.mcp_servers.insert(server_id, server_config);
        self
    }

    /// 列出所有已注册工具名（MCP 工具使用 server_id/tool_name 格式）
    pub fn list_all_tool_names(&self) -> Vec<String> {
        let mut names: Vec<String> =
            self.tools.list_all().into_iter().map(|t| t.name.clone()).collect();
        names.extend(self.mcp.mcp_tools.keys().cloned());
        names
    }

    /// 解析工具名，判断是否是 MCP 工具并返回 (server_id, tool_name)
    fn resolve_mcp_tool(&self, name: &str) -> Option<(String, &McpToolConfig)> {
        if let Some(config) = self.mcp.mcp_tools.get(name) {
            return Some((name.to_string(), config));
        }
        if let Some((server_id, tool_name)) = name.split_once('/') {
            if let Some(config) = self.mcp.mcp_tools.get(name) {
                return Some((server_id.to_string(), config));
            }
            for (key, config) in &self.mcp.mcp_tools {
                if config.server_id == server_id && config.tool_name == tool_name {
                    return Some((key.clone(), config));
                }
            }
        }
        for (key, config) in &self.mcp.mcp_tools {
            if config.tool_name == name {
                return Some((key.clone(), config));
            }
        }
        None
    }

    /// 执行工具（统一入口，集成权限 + Hook）
    pub async fn execute(
        &self,
        tool_name: &str,
        input: &str,
    ) -> Result<ToolResult, crate::ToolError> {
        // ── 工具启用状态检查（组开关 + 单工具禁用 + 黑白名单，位置前移避免重复） ──
        self.check_tool_enabled(tool_name)?;

        // ── 频率限制检查（审计器） ──
        if let Err(rate_limit_msg) = self.auditor.check_rate_limit(tool_name).await {
            return Err(ToolError::permission_denied(tool_name, &rate_limit_msg));
        }

        // ── 输入脱敏 ──
        let sanitized_input = self.auditor.sanitize_input(input);

        // ── 权限检查（集成 PermissionPolicy） ──
        // 锁中毒时恢复内部数据：即使前一个线程 panic，我们仍能继续执行权限检查
        let decision = self.permission_policy.lock().authorize(tool_name, &sanitized_input);
        if decision.is_denied() {
            return Err(ToolError::permission_denied(tool_name, &decision.reason));
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
                    &result.reason.unwrap_or_else(|| "PreToolUse Hook 拒绝执行".into()),
                ));
            }
            if let Some(ref modified) = result.modified_input {
                effective_input = modified.to_string();
            }
        }

        let start = Instant::now();

        let result = if let Some(tool) = self.tools.find(tool_name) {
            let input_val: Value = serde_json::from_str(&effective_input).unwrap_or(Value::Null);
            let category = tool.category();

            if category == ToolCategory::Network && !self.sandbox.config().network_enabled {
                return Err(ToolError::permission_denied(tool_name, "沙箱已禁止网络访问"));
            }
            if category == ToolCategory::Shell {
                let cmd_check = if let Some(cmd) = input_val["command"].as_str() {
                    self.sandbox.check_command(cmd)
                } else {
                    self.sandbox.check_command("")
                };
                if !cmd_check.allowed {
                    let msg = cmd_check
                        .violations
                        .first()
                        .map(|v| v.message.as_str())
                        .unwrap_or("命令被沙箱策略拒绝");
                    return Err(ToolError::permission_denied(tool_name, msg));
                }
            }

            // ── 文件类工具路径校验（FileRead / FileWrite） ──
            // 闭合 P0 安全缺口：原仅 Shell/Network 走沙箱，文件工具可绕过
            // allowed_paths / denied_paths 限制。此处从工具参数提取路径并统一校验。
            // 若参数中无路径字段，跳过校验，避免误伤（如 ListDirectory 默认工作目录）。
            if matches!(category, ToolCategory::FileRead | ToolCategory::FileWrite) {
                let paths = extract_paths_from_args(&input_val);
                if !paths.is_empty()
                    && let Err(msg) =
                        validate_file_paths(&paths, self.sandbox.as_ref(), &self.working_dir)
                {
                    return Err(ToolError::permission_denied(tool_name, &msg));
                }
            }

            let ctx = crate::ToolContext {
                working_dir: self.working_dir.clone(),
                conversation_id: self.conversation_id.clone(),
                message_id: self.message_id.clone(),
                allow_write: true,
                allow_execute: true,
                allow_network: self.sandbox.config().network_enabled,
                abort_signal: None,
                extra: self.tool_extra.clone(),
                permissions: None,
                output_sanitizer: None,
                ask_user_bridge: self.ask_user_bridge.clone(),
                rollback_stack: None,
                agent_id: self.agent_id.clone(),
                dynamic_tools: self.dynamic_tools.clone(),
                // P0-1c/P0-2：实例级显式策略优先，否则回退全局 Settings 策略。
                sandbox: self.sandbox_policy.clone().or_else(global_sandbox_policy),
                approval_policy: self.approval_policy.clone().or_else(global_approval_policy),
            };

            // ── 运行时 Schema 校验（M-05） ──
            tool.validate(&input_val, &ctx).await?;

            match tool.call(input_val, &ctx).await {
                Ok(mut r) => {
                    r.duration_ms = Some(start.elapsed().as_millis() as u64);
                    Ok(r)
                },
                Err(e) => Err(e),
            }
        } else if let Some(result) = self.execute_skill_tool(tool_name, &effective_input) {
            result
        } else if let Some((mcp_key, _config)) = self.resolve_mcp_tool(tool_name) {
            if !self.sandbox.config().network_enabled {
                return Err(ToolError::permission_denied(
                    &mcp_key,
                    "沙箱已禁止网络访问，MCP 工具无法调用",
                ));
            }
            self.execute_mcp(tool_name, &effective_input).await
        } else {
            Err(ToolError::not_found(tool_name))
        };

        // ── 审计日志记录 ──
        let duration_ms = start.elapsed().as_millis() as u64;
        let success = result.is_ok();
        let output_content = result.as_ref().map(|r| &r.content).map(|c| {
            if c.len() > 200 {
                format!("{}...", c.chars().take(200).collect::<String>())
            } else {
                c.clone()
            }
        });
        let has_sensitive_output =
            output_content.as_ref().map(|c| self.auditor.scan_output(c)).unwrap_or(false);
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
        let post_hooks: Vec<HookConfig> =
            self.hook_registry.get_matching(event_type, tool_name).into_iter().cloned().collect();
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
        let (mcp_key, config) =
            self.resolve_mcp_tool(tool_name).ok_or_else(|| ToolError::not_found(tool_name))?;

        let server = self.mcp.mcp_servers.get(&config.server_id).ok_or_else(|| {
            ToolError::execution_failed(format!("MCP server '{}' 未找到", config.server_id))
        })?;

        let arguments: Value = serde_json::from_str(input).unwrap_or(Value::Null);
        let timeout = server.get_timeout();
        let started = std::time::Instant::now();

        // 准备传输参数
        let transport = server.transport.as_str();
        let command = server.command.as_deref();
        let args: Option<Vec<String>> =
            server.args_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
        let env: Option<HashMap<String, String>> =
            server.env_json.as_ref().and_then(|s| serde_json::from_str(s).ok());
        let endpoint = server.endpoint.as_deref();

        // 使用统一的 MCP 客户端入口（使用原始 MCP 工具名，不带前缀）
        let result = tokio::time::timeout(
            timeout,
            axagent_mcp::mcp_client::call_tool_unified(
                transport,
                command,
                args.as_deref(),
                env.as_ref(),
                endpoint,
                &config.tool_name,
                arguments,
                Some(&config.server_id),
            ),
        )
        .await;

        let duration_ms: u64 = started.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(mcp_result)) => {
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

                if let Some(ref recorder) = self.recorder {
                    let input_preview = truncate_str(input, 200);
                    let _ = recorder
                        .record_start(
                            self.conversation_id.as_deref().unwrap_or(""),
                            self.message_id.as_deref(),
                            &config.server_id,
                            &mcp_key,
                            Some(&input_preview),
                        )
                        .await;
                }

                Ok(tool_result)
            },
            Ok(Err(e)) => {
                let err_msg = format!("MCP 工具调用失败: {e}");
                Err(ToolError::execution_failed_for(&mcp_key, err_msg))
            },
            Err(_) => Err(ToolError {
                error_code: format!("tool.{}.timeout", mcp_key),
                message: format!("MCP 工具 '{}' 执行超时（{} 秒）", mcp_key, timeout.as_secs()),
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

// ============================================================
// 文件类工具沙箱路径校验
//
// 闭合 P0 安全缺口：原 registry 仅对 Shell/Network 应用沙箱策略，
// FileRead/FileWrite/FileEdit/ListDirectory/MoveFile 等文件工具可绕过
// allowed_paths / denied_paths 限制。此处统一从工具参数提取路径并校验。
// ============================================================

/// 从工具参数中提取文件路径
///
/// 覆盖所有内置文件工具的参数字段：
/// - 单值：`file_path` / `path` / `target_path` / `source_path` / `source` /
///   `destination` / `notebook_path` / `vault_path`
/// - 数组：`paths`
///
/// 仅提取非空字符串；空字符串与缺失字段会被忽略（跳过校验，避免误伤）。
fn extract_paths_from_args(args: &Value) -> Vec<PathBuf> {
    const SINGLE_KEYS: &[&str] = &[
        "file_path",
        "path",
        "target_path",
        "source_path",
        "source",
        "destination",
        "notebook_path",
        "vault_path",
    ];
    let mut out = Vec::new();
    for key in SINGLE_KEYS {
        if let Some(s) = args.get(key).and_then(|v| v.as_str())
            && !s.is_empty()
        {
            out.push(PathBuf::from(s));
        }
    }
    if let Some(arr) = args.get("paths").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str()
                && !s.is_empty()
            {
                out.push(PathBuf::from(s));
            }
        }
    }
    out
}

/// 规范化路径：先 `canonicalize`（解析符号链接、绝对化），失败时退化为词法规范化
///
/// 用于统一比较基准，防止 `../` 跨越与符号链接逃逸。
fn normalize_path(path: &Path) -> PathBuf {
    match std::fs::canonicalize(path) {
        Ok(canon) => simplify_unc(canon),
        Err(_) => lexical_normalize(path),
    }
}

/// 去掉 Windows `canonicalize` 产生的 `\\?\` 前缀
///
/// Windows 上 `std::fs::canonicalize` 返回 UNC 形式 `\\?\D:\foo`，
/// 与配置中的普通路径 `D:\foo` 按组件比较会因 Prefix 不同而失败。
/// 统一去掉前缀，保证两侧比较基准一致。
#[cfg(windows)]
fn simplify_unc(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped.to_string())
    } else {
        path
    }
}

#[cfg(not(windows))]
fn simplify_unc(path: PathBuf) -> PathBuf {
    path
}

/// 词法规范化路径（处理 `.` 和 `..`，不解析符号链接）
///
/// 用于 `canonicalize` 失败（文件不存在等）时的回退，至少消除 `.`/`..`
/// 段，避免明显的路径穿越逃逸。
fn lexical_normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out: Vec<Component> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {},
            Component::ParentDir => {
                // 回退一级：保留 RootDir / Prefix，避免 `..` 越过根
                if let Some(last) = out.last()
                    && !matches!(last, Component::RootDir | Component::Prefix(_))
                {
                    out.pop();
                }
            },
            c => out.push(c),
        }
    }
    out.iter().collect()
}

/// 校验文件路径是否符合沙箱策略
///
/// 规则（与现有 bash sandbox 保持一致）：
/// 1. `denied_paths` 命中即拒绝（优先级最高）
/// 2. `allowed_paths` 非空时，必须在任一 `allowed_paths` 内（任一匹配即通过）
/// 3. `allowed_paths` 为空时，默认只允许 `workspace_root` 内的路径
/// 4. 路径规范化（`canonicalize` + 词法回退）后再比较，防止 `../` 穿越
///
/// 相对路径会先 join `working_dir` 再规范化，确保比较基准正确。
fn validate_file_paths(
    paths: &[PathBuf],
    sandbox: &crate::AccessPolicyValidator,
    working_dir: &str,
) -> Result<(), String> {
    let config = sandbox.config();
    let working_dir_norm = normalize_path(Path::new(working_dir));
    let allowed_empty = config.allowed_paths.is_empty();

    for path in paths {
        // 相对路径先 join working_dir，确保规范化后能正确比较
        let abs_path = if path.is_absolute() {
            path.clone()
        } else {
            Path::new(working_dir).join(path)
        };
        let norm = normalize_path(&abs_path);

        // 1. denied_paths 检查（优先级最高）
        for denied in &config.denied_paths {
            let denied_norm = normalize_path(denied);
            if norm.starts_with(&denied_norm) {
                return Err(format!("路径 '{}' 在沙箱禁止列表中", path.display()));
            }
        }

        // 2. allowed_paths 处理
        if allowed_empty {
            // 未配置 allowed_paths：默认只允许 workspace_root 内
            if !norm.starts_with(&working_dir_norm) {
                return Err(format!(
                    "路径 '{}' 不在工作区 '{}' 内（allowed_paths 未配置）",
                    path.display(),
                    working_dir
                ));
            }
        } else {
            // 配置了 allowed_paths：必须在任一允许路径内
            let is_allowed = config.allowed_paths.iter().any(|allowed| {
                let allowed_norm = normalize_path(allowed);
                norm.starts_with(&allowed_norm)
            });
            if !is_allowed {
                return Err(format!("路径 '{}' 不在沙箱允许列表中", path.display()));
            }
        }
    }
    Ok(())
}

impl Default for UnifiedToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// ToolExecutor trait 实现（兼容 ConversationRuntime）
// ============================================================

/// 获取全局共享的备用 Tokio runtime，用于在无 runtime 上下文时执行异步任务。
/// 使用 OnceLock 保证只创建一次，避免重复创建嵌套 runtime。
/// 使用 new_current_thread() 而非 new_multi_thread()，减少资源占用且避免嵌套 runtime 风险。
fn fallback_runtime() -> &'static tokio::runtime::Runtime {
    use std::sync::OnceLock;
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create fallback Tokio runtime")
    })
}

impl RuntimeToolExecutor for UnifiedToolRegistry {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        self.check_tool_enabled(tool_name)?;

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| {
                handle.block_on(async {
                    match UnifiedToolRegistry::execute(self, tool_name, input).await {
                        Ok(r) => Ok(r.content),
                        Err(e) => Err(e),
                    }
                })
            }),
            Err(_) => fallback_runtime().block_on(async {
                match UnifiedToolRegistry::execute(self, tool_name, input).await {
                    Ok(r) => Ok(r.content),
                    Err(e) => Err(e),
                }
            }),
        }
    }
}

// ============================================================
// Harness ToolRegistry trait 实现（含 MCP + 禁用状态）
// ============================================================

#[async_trait::async_trait]
impl axagent_harness::ToolRegistry for UnifiedToolRegistry {
    /// 重写 `execute_tool`：委托到完整 `execute`（含限流 / 输入脱敏 / 权限 /
    /// PreToolUse·PostToolUse Hook / 审计），覆盖 harness 默认薄实现。
    ///
    /// 修复 rt-workflow / agent 等资源经 `Arc<dyn ToolRegistry>` 调用时
    /// 缺失横切安全能力的问题（P4）。
    async fn execute_tool(
        &self,
        name: &str,
        input: serde_json::Value,
        _ctx: &axagent_harness::tool::ToolContext,
    ) -> Result<ToolResult, crate::ToolError> {
        let input_str = input.to_string();
        self.execute(name, &input_str).await
    }

    fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name)
    }

    fn find(&self, name: &str) -> Option<Arc<dyn Tool>> {
        if let Some(tool) = self.tools.find(name) {
            return Some(tool.clone());
        }
        if self.resolve_mcp_tool(name).is_some() {
            return None;
        }
        None
    }

    fn list(&self) -> Vec<ToolInfo> {
        let mut infos = self.tools.list_all();
        for (key, mcp) in &self.mcp.mcp_tools {
            if self.groups.disabled_tools.contains(key) {
                continue;
            }
            infos.push(ToolInfo {
                name: key.clone(),
                description: mcp.description.clone().unwrap_or_default(),
                input_schema: mcp.input_schema.clone().unwrap_or(serde_json::json!({})),
                aliases: vec![mcp.tool_name.clone()],
                category: ToolCategory::Integration,
                domain: ToolDomain::General,
                is_concurrency_safe: true,
                is_read_only: false,
                is_destructive: false,
                idempotent: false,
                estimated_cost: None,
                enabled: true,
            });
        }
        infos
    }

    fn list_by_category(&self, category: ToolCategory) -> Vec<ToolInfo> {
        if category == ToolCategory::Integration {
            return self
                .mcp
                .mcp_tools
                .iter()
                .filter(|(key, _)| !self.groups.disabled_tools.contains(*key))
                .map(|(key, mcp)| ToolInfo {
                    name: key.clone(),
                    description: mcp.description.clone().unwrap_or_default(),
                    input_schema: mcp.input_schema.clone().unwrap_or(serde_json::json!({})),
                    aliases: vec![mcp.tool_name.clone()],
                    category: ToolCategory::Integration,
                    domain: ToolDomain::General,
                    is_concurrency_safe: true,
                    is_read_only: false,
                    is_destructive: false,
                    idempotent: false,
                    estimated_cost: None,
                    enabled: true,
                })
                .collect();
        }
        self.tools
            .by_category(category)
            .into_iter()
            .map(|t| ToolInfo::from_tool(t.as_ref()))
            .collect()
    }

    fn is_disabled(&self, name: &str) -> bool {
        self.groups.disabled_tools.contains(name) || self.tools.is_name_disabled(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ToolCategory, ToolContext};
    use async_trait::async_trait;
    use std::sync::atomic::AtomicBool;

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

        let tool = registry.find("echo").expect("测试：find 应成功");
        assert_eq!(tool.name(), "echo");
    }

    #[tokio::test]
    async fn test_registry_alias_resolution() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        let by_alias = registry.find("echo_test").expect("测试：find 应成功");
        assert_eq!(by_alias.name(), "echo");
    }

    /// 构造一个进化生成的测试工具适配器
    fn make_generated_adapter(name: &str, id: &str) -> Arc<dyn Tool> {
        let generated = axagent_harness::trajectory_types::GeneratedTool {
            id: id.to_string(),
            name: name.to_string(),
            code: "return input;".to_string(),
            description: "运行时进化测试工具".to_string(),
            test_coverage: 0.0,
            created_at: 0,
            usage_count: 0,
            success_rate: 0.0,
            artifact_kind: axagent_harness::trajectory_types::EvolutionArtifactKind::RhaiScript,
        };
        Arc::new(crate::generated_tool::GeneratedToolAdapter::new(generated))
    }

    /// T1.1/T1.4：运行时注册 → get_chat_tools 可见 → 卸载后不可见
    #[tokio::test]
    async fn test_runtime_tool_register_and_discover() {
        let mut registry = UnifiedToolRegistry::new();
        let name = "runtime_echo_tool";

        let result = registry
            .register_runtime_tool(make_generated_adapter(name, "id-1"), "runtime_evolution");
        assert!(result.is_ok(), "测试：运行时工具注册应成功");
        assert_eq!(
            registry.runtime_tool_sources().get(name).map(|s| s.as_str()),
            Some("runtime_evolution")
        );

        // get_chat_tools 应包含运行时注册的工具（LLM 可发现）
        let chat_tools = registry.get_chat_tools();
        assert!(
            chat_tools.iter().any(|ct| ct.function.name == name),
            "测试：get_chat_tools 应包含运行时注册工具"
        );

        // 卸载后立即从 get_chat_tools 消失
        let removed = registry.unregister_runtime_tool(name);
        assert!(removed.is_some(), "测试：卸载应返回被卸载的工具");
        assert!(registry.runtime_tool_sources().get(name).is_none());
        let chat_tools_after = registry.get_chat_tools();
        assert!(
            !chat_tools_after.iter().any(|ct| ct.function.name == name),
            "测试：卸载后 get_chat_tools 不应包含该工具"
        );
    }

    /// T1.5：重复注册返回 TOOL_REGISTRATION_DUPLICATE 错误码
    #[tokio::test]
    async fn test_runtime_tool_duplicate_registration_error_code() {
        let mut registry = UnifiedToolRegistry::new();

        // 与内置工具同名 → 应拒绝并返回标准化错误码
        let err = registry
            .register_runtime_tool(
                make_generated_adapter("FileRead", "id-builtin"),
                "runtime_evolution",
            )
            .expect_err("测试：与内置工具同名注册应失败");
        assert_eq!(err.error_code, axagent_harness::error_codes::tool::REGISTRATION_DUPLICATE);

        // 同名运行时工具重复注册 → 同样拒绝
        let dup_name = "runtime_dup_tool";
        assert!(
            registry
                .register_runtime_tool(
                    make_generated_adapter(dup_name, "id-a"),
                    "runtime_evolution"
                )
                .is_ok()
        );
        let err2 = registry
            .register_runtime_tool(make_generated_adapter(dup_name, "id-b"), "runtime_evolution")
            .expect_err("测试：同名运行时工具重复注册应失败");
        assert_eq!(err2.error_code, axagent_harness::error_codes::tool::REGISTRATION_DUPLICATE);

        // 卸载非运行时工具应返回 None（不污染内置工具）
        assert!(registry.unregister_runtime_tool("FileRead").is_none());
    }

    /// T2.2：副作用栈 — register 登记 Disposer，unregister 执行对应回滚（LIFO 匹配）
    #[tokio::test]
    async fn test_runtime_tool_effects_stack_lifo_rollback() {
        let mut registry = UnifiedToolRegistry::new();

        // 两个工具 + 一个扩展副作用（模拟护照移除等 wiring 附加回滚）
        let name_a = "runtime_effects_a";
        let name_b = "runtime_effects_b";
        assert!(
            registry
                .register_runtime_tool(make_generated_adapter(name_a, "id-a"), "runtime_evolution")
                .is_ok()
        );
        assert!(
            registry
                .register_runtime_tool(make_generated_adapter(name_b, "id-b"), "runtime_evolution")
                .is_ok()
        );

        let rollback_called = Arc::new(AtomicBool::new(false));
        let rollback_flag = rollback_called.clone();
        registry.push_effect(Disposer::new(
            name_b.to_string(),
            "扩展副作用（测试）",
            move || {
                rollback_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            },
        ));

        assert_eq!(registry.effects_len(), 3, "测试：两个工具 + 一个扩展副作用");

        // 卸载 name_b → 应触发其 Disposer 回滚（扩展副作用闭包执行）
        let removed_b = registry.unregister_runtime_tool(name_b);
        assert!(removed_b.is_some());
        assert!(
            rollback_called.load(std::sync::atomic::Ordering::SeqCst),
            "测试：卸载时应执行匹配的 Disposer 回滚"
        );
        assert_eq!(registry.effects_len(), 1, "测试：卸载后对应 Disposer 已出栈");

        // 卸载 name_a → 剩余 Disposer 出栈
        let removed_a = registry.unregister_runtime_tool(name_a);
        assert!(removed_a.is_some());
        assert_eq!(registry.effects_len(), 0, "测试：全部卸载后副作用栈清空");
    }

    /// T2.2：副作用栈 — cleanup_runtime_effects 全量清理（后进先出）
    #[tokio::test]
    async fn test_runtime_tool_cleanup_all_effects() {
        let mut registry = UnifiedToolRegistry::new();
        let name = "runtime_cleanup_tool";
        assert!(
            registry
                .register_runtime_tool(make_generated_adapter(name, "id-c"), "runtime_evolution")
                .is_ok()
        );
        assert_eq!(registry.effects_len(), 1);

        let cleaned = registry.cleanup_runtime_effects();
        assert_eq!(cleaned, 1, "测试：全量清理应返回清理数量");
        assert_eq!(registry.effects_len(), 0);
        assert!(registry.runtime_tool_sources().get(name).is_none());
    }

    /// 渐进式披露白名单安全护栏：按名放行只得到披露工具，绝不泄露写操作。
    ///
    /// 认知编排执行阶段若改用「把 `General` 域加进 active_domains」来放行披露工具，
    /// 该域下 140+ 个工具（含下列危险写操作）会一并暴露给编排阶段。
    /// 本测试就是守住这条线的回归闸门。
    #[test]
    fn disclosure_tools_whitelist_never_leaks_write_tools() {
        let registry = UnifiedToolRegistry::new();
        let tools = registry.get_chat_tools_by_names(DISCLOSURE_TOOLS.iter().copied());
        let names: HashSet<String> = tools.into_iter().map(|t| t.function.name).collect();

        for wanted in DISCLOSURE_TOOLS {
            assert!(
                names.contains(wanted),
                "测试：白名单工具 {} 应从注册表取到完整 schema，实际取到 {:?}",
                wanted,
                names
            );
        }

        // 先证明危险工具确实挂在 General 域下 —— 这是「必须按名放行」这一设计的前提。
        // 若有人改用 get_chat_tools_for_domains 放行披露工具，下面这条按域取到的集合
        // 会原样暴露给认知编排执行阶段。
        let general_domains: HashSet<ToolDomain> = [ToolDomain::General].into_iter().collect();
        let domain_names: HashSet<String> = registry
            .get_chat_tools_for_domains(&general_domains, None)
            .into_iter()
            .map(|t| t.function.name)
            .collect();

        let dangerous = ["Bash", "FileWrite", "FileEdit", "DeleteFile", "Agent", "DelegateTask"];

        for name in dangerous {
            assert!(
                domain_names.contains(name),
                "测试：{} 应挂在 General 域下（按域放行会泄露它）；若已移出该域，请同步复核本护栏",
                name
            );
            assert!(!names.contains(name), "测试：按名放行不得泄露危险写操作工具 {}", name);
        }
    }

    /// `is_disclosure_immune` 必须与 `DISCLOSURE_TOOLS` 名单严格一致。
    ///
    /// 两个方向都要锁：名单里的每个工具都要免疫（漏一个就留下「能力发现闭环被
    /// profile 静默掐断」的隐患），名单外的工具不能免疫（否则豁免范围外溢，
    /// profile 的 `disallowed_tools` 会对普通工具失效）。
    #[test]
    fn is_disclosure_immune_matches_whitelist_exactly() {
        for name in DISCLOSURE_TOOLS {
            assert!(
                is_disclosure_immune(name),
                "测试：披露工具 {} 必须对 profile 黑名单免疫",
                name
            );
        }
        for name in ["Bash", "FileWrite", "FileEdit", "DeleteFile", "Agent", "ShellExec"] {
            assert!(
                !is_disclosure_immune(name),
                "测试：普通工具 {} 不得享受黑名单豁免，否则 profile 禁用策略失效",
                name
            );
        }
        assert!(!is_disclosure_immune(""));
        // 大小写敏感：名单一律 PascalCase，避免误豁免
        assert!(!is_disclosure_immune("capabilityview"));
    }

    /// `SCREEN_PERCEPTION_TOOL` **不得**落在 `DISCLOSURE_TOOLS` 豁免名单里。
    ///
    /// 两个名单若发生交集，屏幕感知的黑名单门控会被「披露工具免疫」逻辑吃掉，
    /// 形成「用户在设置里关掉屏幕感知、工具却照样下发给 LLM」的**静默失效**——
    /// 而且两侧代码各自看起来都正确。这条断言守住两个名单的互斥边界。
    #[test]
    fn screen_perception_tool_is_not_disclosure_immune() {
        assert!(
            !is_disclosure_immune(SCREEN_PERCEPTION_TOOL),
            "测试：{} 不得享受披露工具豁免，否则屏幕感知门控会被免疫逻辑绕过",
            SCREEN_PERCEPTION_TOOL
        );
        // 顺带锁定披露名单本身不含大小写变体（避免将来加名单时手滑）
        for name in DISCLOSURE_TOOLS {
            assert!(
                name.chars().next().is_some_and(char::is_uppercase),
                "测试：披露工具名 {} 应为 PascalCase",
                name
            );
        }
    }

    /// `SCREEN_PERCEPTION_TOOL` 常量必须与真实注册的工具名一致。
    ///
    /// 该常量在两处被消费（可见性过滤 + 执行期 disable）。若有人改了
    /// `ComputerUseTool::name()` 而漏改常量，两道门控会**静默失效** —— 工具照常暴露给
    /// LLM，而故障现象（屏幕感知关不掉）很难归因到「常量对不上」。本测试把这条
    /// 一致性从人肉约定变成可执行断言。
    #[test]
    fn screen_perception_tool_matches_registered_name() {
        let registry = UnifiedToolRegistry::new();
        let tools = registry.get_chat_tools_by_names([SCREEN_PERCEPTION_TOOL]);
        assert_eq!(
            tools.len(),
            1,
            "测试：常量 SCREEN_PERCEPTION_TOOL({}) 应取到唯一一个已注册工具，实际 {} 个。\
             若工具已改名，请同步更新该常量与其两处消费点",
            SCREEN_PERCEPTION_TOOL,
            tools.len()
        );
        assert_eq!(tools[0].function.name, SCREEN_PERCEPTION_TOOL);
    }
}

// ============================================================
// 文件类工具沙箱路径校验测试
// ============================================================

#[cfg(test)]
mod file_sandbox_tests {
    use super::*;
    use crate::SandboxConfig;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// 创建唯一的临时目录用于测试（避免并行测试冲突）
    fn make_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("测试：系统时间应晚于 UNIX EPOCH")
            .subsec_nanos();
        let dir = std::env::temp_dir().join(format!(
            "axagent_test_{}_{}_{}",
            prefix,
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&dir).expect("测试：创建目录应成功");
        dir
    }

    #[test]
    fn extract_paths_single_fields() {
        let args = serde_json::json!({
            "file_path": "/a/b.txt",
            "path": "/c",
            "source": "/d/src",
            "destination": "/e/dst",
            "target_path": "/t",
            "source_path": "/sp",
            "notebook_path": "/nb",
            "vault_path": "/v"
        });
        let paths = extract_paths_from_args(&args);
        assert_eq!(paths.len(), 8);
        assert!(paths.contains(&PathBuf::from("/a/b.txt")));
        assert!(paths.contains(&PathBuf::from("/c")));
        assert!(paths.contains(&PathBuf::from("/d/src")));
        assert!(paths.contains(&PathBuf::from("/e/dst")));
    }

    #[test]
    fn extract_paths_array_field() {
        let args = serde_json::json!({ "paths": ["/x", "/y", "/z"] });
        let paths = extract_paths_from_args(&args);
        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&PathBuf::from("/x")));
    }

    #[test]
    fn extract_paths_ignores_empty_and_null() {
        let args = serde_json::json!({
            "file_path": "",
            "path": "/c",
            "target_path": null,
            "source": "/s"
        });
        let paths = extract_paths_from_args(&args);
        // file_path 空字符串跳过，target_path null 跳过
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&PathBuf::from("/c")));
        assert!(paths.contains(&PathBuf::from("/s")));
    }

    #[test]
    fn extract_paths_empty_object_returns_empty() {
        let paths = extract_paths_from_args(&serde_json::json!({}));
        assert!(paths.is_empty());
    }

    #[test]
    fn validate_allowed_inside_workspace_passes() {
        let workspace = make_temp_dir("ws");
        let config = SandboxConfig { allowed_paths: vec![workspace.clone()], ..Default::default() };
        let sandbox = crate::AccessPolicyValidator::new(config);
        let inside = workspace.join("file.txt");
        std::fs::write(&inside, "test").expect("测试：写入文件应成功");

        let result = validate_file_paths(
            &[inside],
            &sandbox,
            workspace.to_str().expect("测试：路径转字符串应成功"),
        );
        assert!(result.is_ok(), "工作区内路径应允许");

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn validate_outside_workspace_is_denied() {
        let workspace = make_temp_dir("ws");
        let outside = make_temp_dir("out");
        let config = SandboxConfig { allowed_paths: vec![workspace.clone()], ..Default::default() };
        let sandbox = crate::AccessPolicyValidator::new(config);
        let outside_file = outside.join("secret.txt");
        std::fs::write(&outside_file, "secret").expect("测试：写入文件应成功");

        let result = validate_file_paths(
            &[outside_file],
            &sandbox,
            workspace.to_str().expect("测试：路径转字符串应成功"),
        );
        assert!(result.is_err(), "工作区外路径应拒绝");

        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn validate_denied_list_takes_priority() {
        let workspace = make_temp_dir("ws");
        let secret_dir = workspace.join("secret");
        std::fs::create_dir_all(&secret_dir).expect("测试：创建目录应成功");
        let config = SandboxConfig {
            allowed_paths: vec![workspace.clone()],
            denied_paths: vec![secret_dir.clone()],
            ..Default::default()
        };
        let sandbox = crate::AccessPolicyValidator::new(config);
        let secret_file = secret_dir.join("key.txt");
        std::fs::write(&secret_file, "key").expect("测试：写入文件应成功");

        let result = validate_file_paths(
            &[secret_file],
            &sandbox,
            workspace.to_str().expect("测试：路径转字符串应成功"),
        );
        assert!(result.is_err(), "denied_paths 命中应拒绝（即使在工作区内）");

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn validate_empty_allowed_defaults_to_workspace() {
        let workspace = make_temp_dir("ws");
        let outside = make_temp_dir("out");
        // allowed_paths 为空 → 默认只允许 workspace_root 内
        let config = SandboxConfig { allowed_paths: vec![], ..Default::default() };
        let sandbox = crate::AccessPolicyValidator::new(config);

        // 工作区内允许
        let inside = workspace.join("file.txt");
        std::fs::write(&inside, "test").expect("测试：写入文件应成功");
        let result = validate_file_paths(
            &[inside],
            &sandbox,
            workspace.to_str().expect("测试：路径转字符串应成功"),
        );
        assert!(result.is_ok(), "allowed_paths 为空时工作区内路径应允许");

        // 工作区外拒绝
        let outside_file = outside.join("secret.txt");
        std::fs::write(&outside_file, "secret").expect("测试：写入文件应成功");
        let result = validate_file_paths(
            &[outside_file],
            &sandbox,
            workspace.to_str().expect("测试：路径转字符串应成功"),
        );
        assert!(result.is_err(), "allowed_paths 为空时工作区外路径应拒绝");

        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn validate_traversal_dotdot_is_blocked() {
        let workspace = make_temp_dir("ws");
        // 在 workspace 的父目录下创建一个真实逃逸目录，使 canonicalize 能成功
        let escaped =
            workspace.parent().expect("测试：路径应有父目录").join("axagent_escaped_test");
        std::fs::create_dir_all(&escaped).expect("测试：创建目录应成功");
        let escaped_file = escaped.join("secret.txt");
        std::fs::write(&escaped_file, "secret").expect("测试：写入文件应成功");

        let config = SandboxConfig { allowed_paths: vec![workspace.clone()], ..Default::default() };
        let sandbox = crate::AccessPolicyValidator::new(config);

        // 用 ../ 形式构造穿越路径，canonicalize 后会解析到 workspace 父目录
        let traversal = workspace.join("..").join("axagent_escaped_test").join("secret.txt");
        let result = validate_file_paths(
            &[traversal],
            &sandbox,
            workspace.to_str().expect("测试：路径转字符串应成功"),
        );
        assert!(result.is_err(), "../ 穿越路径应被拒绝");

        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&escaped);
    }

    #[test]
    fn validate_relative_path_resolved_against_working_dir() {
        let workspace = make_temp_dir("ws");
        let subdir = workspace.join("sub");
        std::fs::create_dir_all(&subdir).expect("测试：创建目录应成功");
        let file = subdir.join("file.txt");
        std::fs::write(&file, "test").expect("测试：写入文件应成功");

        let config = SandboxConfig { allowed_paths: vec![workspace.clone()], ..Default::default() };
        let sandbox = crate::AccessPolicyValidator::new(config);

        // 相对路径 "sub/file.txt" 应被 join working_dir 后通过校验
        let rel = PathBuf::from("sub").join("file.txt");
        let result = validate_file_paths(
            &[rel],
            &sandbox,
            workspace.to_str().expect("测试：路径转字符串应成功"),
        );
        assert!(result.is_ok(), "工作区内相对路径应允许");

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn validate_multiple_paths_all_must_pass() {
        let workspace = make_temp_dir("ws");
        let outside = make_temp_dir("out");
        let config = SandboxConfig { allowed_paths: vec![workspace.clone()], ..Default::default() };
        let sandbox = crate::AccessPolicyValidator::new(config);

        let inside = workspace.join("a.txt");
        std::fs::write(&inside, "a").expect("测试：写入文件应成功");
        let outside_file = outside.join("b.txt");
        std::fs::write(&outside_file, "b").expect("测试：写入文件应成功");

        // 一个在工作区内，一个在外 → 整体拒绝
        let result = validate_file_paths(
            &[inside, outside_file],
            &sandbox,
            workspace.to_str().expect("测试：路径转字符串应成功"),
        );
        assert!(result.is_err(), "多路径中任一越权应整体拒绝");

        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn lexical_normalize_handles_dotdot() {
        let p = Path::new("/a/b/../c");
        assert_eq!(lexical_normalize(p), PathBuf::from("/a/c"));
    }

    #[test]
    fn lexical_normalize_handles_curdir() {
        let p = Path::new("/a/./b");
        assert_eq!(lexical_normalize(p), PathBuf::from("/a/b"));
    }

    #[test]
    fn lexical_normalize_does_not_escape_root() {
        // /.. 不应越过根，保留为 /
        let p = Path::new("/../etc");
        assert_eq!(lexical_normalize(p), PathBuf::from("/etc"));
    }
}
