// SPDX-License-Identifier: AGPL-3.0-only
//! 能力数字护照 — 统一的能力元数据契约
//!
//! 所有可被发现的能力（工具/工作流/知识库/Agent/Skill）通过实现 `CapabilityPassport` trait，
//! 向能力发现系统暴露统一的元数据接口。
//!
//! # 设计原则
//! - 只暴露元数据，不暴露执行逻辑
//! - 所有方法提供默认实现，便于渐进式集成
//! - 负面场景是关键创新点：防止误匹配

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── 能力类型枚举 ──────────────────────────────────

/// 能力承载载体
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    /// 工具（Tool / MCP Tool / Built-in Tool）
    #[default]
    Tool,
    /// 工作流（WorkflowTemplate）
    Workflow,
    /// 知识库（RAG Knowledge Base）
    KnowledgeBase,
    /// Agent（智能体）
    Agent,
    /// 技能（Skill）
    Skill,
    /// 工具链（固定顺序的工具组合，线性串接、无业务分支逻辑）
    Toolchain,
    /// 模板（含占位符参数，命中后不直接执行，仅提示"可实例化为具体 Skill"）
    Template,
}

impl CapabilityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityKind::Tool => "tool",
            CapabilityKind::Workflow => "workflow",
            CapabilityKind::KnowledgeBase => "knowledge_base",
            CapabilityKind::Agent => "agent",
            CapabilityKind::Skill => "skill",
            CapabilityKind::Toolchain => "toolchain",
            CapabilityKind::Template => "template",
        }
    }
}

/// 能力来源（追溯护照注册来源，用于能力发现与进化的边界判断）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySource {
    /// 内置能力（应用自身注册的护照）
    #[default]
    Builtin,
    /// 插件提供的能力（配合 plugin_id 溯源；禁用/卸载插件时回滚）
    Plugin,
}

/// 能力可进化性（决定进化引擎的分发边界）
///
/// 外部插件声明的能力默认不可进化；仅当载体本地可写时才允许就地进化
/// （Local），否则以衍生产物（Derived）方式产出独立副本、原护照不变。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvolvability {
    /// 不可进化（外部插件只读能力）
    #[default]
    None,
    /// 本地进化：能力载体本地可写，直接提升等级/参数
    Local,
    /// 衍生产物：进化产生新的独立能力副本，原护照保持不变
    Derived,
}

// ── 能力域（唯一权威分类轴） ───────────────────────

/// 能力所属功能域
///
/// **标准化约束**（消除历史歧义，2026-08 重构）：
/// - 唯一权威分类轴，全部按业务常识的功能语义划分，禁止引入自定义/产品线概念。
/// - 业务线（AxInvest/AxOPC）通过护照 `tags`（`axinvest`/`axopc`）表达，不占域轴。
/// - `General` 是唯一兜底域：任何不专属于下述功能域的能力归入此处。
///   （历史 `Core` 域已合并进 `General`，避免“核心/通用”双兜底边界模糊。）
/// - 各功能域互斥，以“该域的专业操作”为判据。
/// - `System` 为内部域，仅配合 `Visibility::SystemOnly` 使用，永不进入检索结果。
///
/// # 历史字符串兼容
/// 反序列化与 `FromStr` 接受旧值别名：`core`→General、`invest`→Finance、`opc`→Automation，
/// 保证存量数据库（如 `active_domains`、路由路径）不受损；`as_str()`/`Display` 只输出新值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityDomain {
    /// 通用：文件/Shell/文本/网络/搜索/文档/配置等兜底通用能力
    #[serde(alias = "core")]
    General,
    /// 运维：CI/CD、部署、监控告警、安全审计、容器编排
    Devops,
    /// AI 媒体：图像/视频/音频的生成与处理
    AiMedia,
    /// 数据分析：SQL 查询、数据可视化、ETL/数据清洗
    DataAnalysis,
    /// 内容创作：写作、设计、排版
    ContentCreation,
    /// 通信：IM、邮件、推送通知
    Communication,
    /// 金融：行情、交易、风控、组合管理（业务标签 axinvest）
    #[serde(alias = "invest")]
    Finance,
    /// 自动化：RPA、定时任务、工作流编排（业务标签 axopc）
    #[serde(alias = "opc")]
    Automation,
    /// 系统域（编排器、降级控制器等内部能力，仅配合 SystemOnly，不可被用户发现）
    System,
}

impl CapabilityDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityDomain::General => "general",
            CapabilityDomain::Devops => "devops",
            CapabilityDomain::AiMedia => "ai_media",
            CapabilityDomain::DataAnalysis => "data_analysis",
            CapabilityDomain::ContentCreation => "content_creation",
            CapabilityDomain::Communication => "communication",
            CapabilityDomain::Finance => "finance",
            CapabilityDomain::Automation => "automation",
            CapabilityDomain::System => "system",
        }
    }

    /// 是否为系统域（不可被用户发现）
    pub fn is_system(&self) -> bool {
        matches!(self, CapabilityDomain::System)
    }
}

impl std::fmt::Display for CapabilityDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for CapabilityDomain {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            // 新值
            "general" => CapabilityDomain::General,
            "devops" => CapabilityDomain::Devops,
            "ai_media" => CapabilityDomain::AiMedia,
            "data_analysis" => CapabilityDomain::DataAnalysis,
            "content_creation" => CapabilityDomain::ContentCreation,
            "communication" => CapabilityDomain::Communication,
            "finance" => CapabilityDomain::Finance,
            "automation" => CapabilityDomain::Automation,
            "system" => CapabilityDomain::System,
            // 历史别名（兼容存量数据，单向收敛到标准域，不构成独立域定义）
            "core" => CapabilityDomain::General,
            "invest" => CapabilityDomain::Finance,
            "opc" => CapabilityDomain::Automation,
            // 历史命令域值别名（命令桥统一前的存量标注，收敛到标准域）
            "quant" => CapabilityDomain::Finance,
            "portfolio" => CapabilityDomain::Finance,
            "stock_analysis" => CapabilityDomain::Finance,
            "device" => CapabilityDomain::General,
            "dynamic_ui" => CapabilityDomain::General,
            "fine_tune" => CapabilityDomain::General,
            "cloud" => CapabilityDomain::General,
            "pty" => CapabilityDomain::Devops,
            "rl_training" => CapabilityDomain::General,
            "context" => CapabilityDomain::General,
            "db_config" => CapabilityDomain::Devops,
            "conversation" => CapabilityDomain::General,
            "orchestrator" => CapabilityDomain::System,
            "agent" => CapabilityDomain::System,
            "workflow" => CapabilityDomain::Automation,
            _ => return Err(()),
        })
    }
}

// ── 安全等级 ──────────────────────────────────────

/// 安全等级（用于硬性闸门过滤）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityLevel {
    /// 公开可执行（无敏感数据）
    Public,
    /// 内部使用（含非公开但非敏感数据）
    Internal,
    /// 敏感（含 PII、凭证、商业机密）
    Sensitive,
    /// 受限（需审批、审计、加密传输）
    Restricted,
}

impl SecurityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityLevel::Public => "public",
            SecurityLevel::Internal => "internal",
            SecurityLevel::Sensitive => "sensitive",
            SecurityLevel::Restricted => "restricted",
        }
    }

    /// 是否已加密传输
    pub fn requires_encrypted_transmission(&self) -> bool {
        matches!(self, SecurityLevel::Sensitive | SecurityLevel::Restricted)
    }

    /// 是否需要完整审计日志
    pub fn requires_audit_log(&self) -> bool {
        matches!(self, SecurityLevel::Restricted)
    }
}

// ── 能力可见性（元能力隔离核心） ──────────────────

/// 能力可见性枚举 — 用于硬性过滤，防止系统能力被用户发现
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, TS)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// 公开可发现（业务能力，用户可检索）
    #[default]
    Public,
    /// 系统内部专用（编排器、降级控制器等，不可被用户发现）
    SystemOnly,
    /// 仅特权用户可见（受 dual_registry 特权管道保护，普通检索不可发现）
    PrivilegedOnly,
    /// 已废弃/隐藏（不参与任何检索）
    Hidden,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::SystemOnly => "system_only",
            Visibility::PrivilegedOnly => "privileged_only",
            Visibility::Hidden => "hidden",
        }
    }

    /// 是否可被用户发现
    pub fn is_discoverable(&self) -> bool {
        matches!(self, Visibility::Public)
    }

    /// 是否为系统专用
    pub fn is_system_only(&self) -> bool {
        matches!(self, Visibility::SystemOnly)
    }
}

// ── 调用权限控制 ──────────────────────────────────

/// 能力调用权限 — 控制哪些角色可以调用该能力
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallerPermissions {
    /// 允许调用的角色列表（空 = 所有人可调用）
    #[serde(default)]
    pub allowed_roles: Vec<String>,
    /// 是否允许终端用户调用
    #[serde(default = "default_true")]
    pub allow_end_user: bool,
}

fn default_true() -> bool {
    true
}

impl CallerPermissions {
    pub fn new() -> Self {
        Self { allowed_roles: Vec::new(), allow_end_user: true }
    }

    /// 系统专用权限（仅编排器/管理员可调用）
    pub fn system_only() -> Self {
        Self {
            allowed_roles: vec!["orchestrator".to_string(), "admin".to_string()],
            allow_end_user: false,
        }
    }

    /// 检查指定角色是否有权调用
    pub fn can_be_called_by(&self, role: &str) -> bool {
        if self.allowed_roles.is_empty() {
            return true;
        }
        self.allowed_roles.iter().any(|r| r == role)
    }

    /// 是否允许终端用户调用
    pub fn allows_end_user(&self) -> bool {
        self.allow_end_user
    }
}

// ── 模态支持 ──────────────────────────────────────

/// 模态支持声明
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalitySupport {
    #[serde(default)]
    pub supports_text: bool,
    #[serde(default)]
    pub supports_image: bool,
    #[serde(default)]
    pub supports_audio: bool,
    #[serde(default)]
    pub supports_video: bool,
    #[serde(default)]
    pub supports_file: bool,
}

impl ModalitySupport {
    /// 检查是否支持所有输入模态
    pub fn supports_all(&self) -> bool {
        self.supports_text && self.supports_image
    }

    /// 检查是否支持指定模态
    pub fn supports(&self, modality: &InputModality) -> bool {
        match modality {
            InputModality::Text => self.supports_text,
            InputModality::Image => self.supports_image,
            InputModality::Audio => self.supports_audio,
            InputModality::Video => self.supports_video,
            InputModality::File => self.supports_file,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputModality {
    Text,
    Image,
    Audio,
    Video,
    File,
}

// ── 规划复杂度 ────────────────────────────────────

/// 规划复杂度（用于区分简单任务和复杂工作流）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningComplexity {
    /// 单步直接执行（如 read_file）
    Simple,
    /// 条件分支 + 少量步骤（如 web_search + summarize）
    Moderate,
    /// 多步循环 / DAG / 并行（如完整数据流水线）
    Complex,
}

impl PlanningComplexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanningComplexity::Simple => "simple",
            PlanningComplexity::Moderate => "moderate",
            PlanningComplexity::Complex => "complex",
        }
    }
}

// ── 执行模式 ──────────────────────────────────────

/// 执行模式 — 编排器据此决策执行路径（直发 / 异步 / 流式）
///
/// 统一能力模型 `ExecutableCapability.execution.mode` 的落地。
/// 认知编排在 `clamp_mode_for_kind` 之外获得护照级声明，避免仅靠 kind 猜测。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// 同步执行：调用方阻塞等待结果
    #[default]
    Sync,
    /// 异步执行：立即返回任务句柄，结果稍后查询
    Async,
    /// 流式执行：结果分片推送（SSE/WebSocket）
    Streaming,
}

impl ExecutionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionMode::Sync => "sync",
            ExecutionMode::Async => "async",
            ExecutionMode::Streaming => "streaming",
        }
    }
}

/// 执行分派解析（任务①）：返回能力护照声明的执行模式，对不支持该模式的 kind 做安全降级。
///
/// 触发条件（任务①）：认知编排需按护照声明（而非仅靠 kind）决定
/// Sync/Async/Streaming 分派。本函数是该决策的**单一来源**。
///
/// 降级规则：仅 `Tool`/`Workflow`/`Skill`/`Toolchain`/`Agent` 等可执行 kind 可声明
/// 非 Sync 模式；`KnowledgeBase`/`Template` 等非执行 kind 强制 `Sync`，
/// 避免把"声明"误当"可异步/流式执行"（Template 命中后仅提示实例化、不直接执行）。
///
/// 当前所有护照 `execution_mode` 默认 `Sync`，故不改变既有行为；
/// 仅当护照显式声明 `Async`/`Streaming` 时生效 —— 符合 Phase 0 验收"字段存在"范围，
/// 且为后续 dispatcher 接线提供唯一判定入口。
pub fn resolve_execution_mode(passport_mode: ExecutionMode, kind: CapabilityKind) -> ExecutionMode {
    match kind {
        CapabilityKind::KnowledgeBase | CapabilityKind::Template => ExecutionMode::Sync,
        _ => passport_mode,
    }
}

// ── Tool 实现契约（统一能力模型 Tool.implementation） ──

/// Tool 底层实现载体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImplementationType {
    /// 本地函数（Rust 实现，默认）
    #[default]
    LocalFunction,
    /// REST API
    RestApi,
    /// gRPC
    Grpc,
    /// Shell 脚本
    ShellScript,
    /// MCP 工具
    Mcp,
    /// Tauri 命令桥
    TauriCommand,
}

impl ImplementationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImplementationType::LocalFunction => "local_function",
            ImplementationType::RestApi => "rest_api",
            ImplementationType::Grpc => "grpc",
            ImplementationType::ShellScript => "shell_script",
            ImplementationType::Mcp => "mcp",
            ImplementationType::TauriCommand => "tauri_command",
        }
    }
}

/// Tool 实现契约 — 描述"如何调用"，供外部接入与执行器反查。
///
/// 统一能力模型 `Tool.implementation` 的落地。护照命中后，
/// 执行器凭此契约（而非仅凭 tool_ref）定位真实调用方式。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolImplementation {
    /// 实现类型
    pub impl_type: ImplementationType,
    /// 端点（REST/gRPC：URL；本地：模块路径）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// HTTP 方法（GET/POST 等，仅 RestApi）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// 鉴权方式描述（bearer / api_key / oauth / none）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    /// 请求体模板（含占位符，如 `{"query": "{{query}}"}`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_template: Option<String>,
    /// 响应解析规则（JSONPath / 正则 / 提取表达式）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_parser: Option<String>,
}

// ── Skill 步骤（统一能力模型 Skill.steps） ──────────

/// Skill 的结构化执行步骤 — 比 Toolchain 的纯工具 ID 列表承载更多编排语义。
///
/// 统一能力模型 `Skill.steps`（step_id/capability_id/params/condition/on_error）的落地。
/// 与 `CapabilityPassportDto.steps`（Toolchain 顺序工具列表）并存：
/// - `steps`：Toolchain 专用，`Vec<String>` 线性工具链（`cognitive_execute_toolchain` 消费）
/// - `skill_steps`：Skill 专用，结构化步骤（步骤级参数/条件/错误处理）
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillStep {
    /// 步骤 ID（步骤列表内唯一；为空时按索引隐式编号）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub step_id: String,
    /// 引用的能力 ID（Tool / Skill / Workflow）
    pub capability_id: String,
    /// 步骤级参数映射（可选；不填继承上级参数）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// 可选执行条件（Rhai 表达式或自然语言）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// 错误处理策略（stop=短路 / skip=跳过继续 / fallback=回退能力ID）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<String>,
}

// ── 组合与关联模型（统一能力模型第四层 CapabilityRelationship） ──

/// 能力关系类型 — 用于关联发现与编排（图遍历）。
///
/// 统一能力模型 `CapabilityRelationship.relationship_type` 的落地。
/// 护照 `upstream`/`downstream` 为声明式一跳依赖，本类型与物化表
/// （v129 capability_relationships）承载完整关系图谱。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// 依赖关系（能力 A 的执行依赖能力 B）
    DependsOn,
    /// 使用关系（能力 A 执行中使用了能力 B）
    Uses,
    /// 替代关系（能力 A 与能力 B 等价可互换）
    AlternativeTo,
    /// 冲突关系（能力 A 与能力 B 互斥）
    ConflictsWith,
    /// 组合中的父子关系（A 包含 B）
    ParentOf,
    /// 顺序前置（A 在 B 之前执行）
    Precedes,
    /// 顺序后置（A 在 B 之后执行）
    Follows,
    /// 需要某知识片段（A 依赖知识片段 B）
    RequiresKnowledge,
    /// 版本淘汰（A 被 B 取代；A 进入 superseded 状态时写入）
    ///
    /// 统一能力模型版本治理链路（任务④）。`sync_from_passports` 物化的是护照声明来源，
    /// 本变体由运行时 `mark_superseded` 写入，语义独立于护照声明。
    SupersededBy,
}

impl RelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipType::DependsOn => "depends_on",
            RelationshipType::Uses => "uses",
            RelationshipType::AlternativeTo => "alternative_to",
            RelationshipType::ConflictsWith => "conflicts_with",
            RelationshipType::ParentOf => "parent_of",
            RelationshipType::Precedes => "precedes",
            RelationshipType::Follows => "follows",
            RelationshipType::RequiresKnowledge => "requires_knowledge",
            RelationshipType::SupersededBy => "superseded_by",
        }
    }
}

/// 能力关系 — 图边（统一能力模型 `CapabilityRelationship` 的序列化形态）。
///
/// 与护照 `upstream`/`downstream` 字段的区别：
/// - 护照字段：声明式、内联、一跳（检索扩展直接读）
/// - 本类型 + v129 表：物化镜像 + 关系元信息（type/weight/context/metadata），
///   供关系查询、审计与未来图遍历；检索多跳 BFS 仍以内存护照图为主源。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRelationship {
    /// 源能力 ID（如 `tool:read_file`）
    pub source_id: String,
    /// 目标能力 ID
    pub target_id: String,
    /// 关系类型
    pub relationship_type: RelationshipType,
    /// 关系权重（0.0-1.0，用于检索排序；默认 1.0）
    #[serde(default = "default_relation_weight")]
    pub weight: f64,
    /// 关系描述上下文
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// 扩展元信息（JSON 对象）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

fn default_relation_weight() -> f64 {
    1.0
}

// ── 能力等级 ──────────────────────────────────────

/// 能力等级（能力成熟度综合评级，L1 最低 → L5 最高）
///
/// 由 [`CapabilityLevel::derive`] 从护照多维数据（规划复杂度、IQ 需求、
/// 历史成功率、调用频次、耗时、成本）加权派生；低等级（L1/L2）能力
/// 可启用进化来提升等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityLevel {
    /// 未成熟 / 数据不足（建议进化）
    #[default]
    L1,
    /// 基础可用
    L2,
    /// 中等成熟
    L3,
    /// 高度成熟
    L4,
    /// 顶级能力
    L5,
}

impl CapabilityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityLevel::L1 => "l1",
            CapabilityLevel::L2 => "l2",
            CapabilityLevel::L3 => "l3",
            CapabilityLevel::L4 => "l4",
            CapabilityLevel::L5 => "l5",
        }
    }

    /// 数值化（1-5），用于比较与提升
    pub fn value(&self) -> u8 {
        match self {
            CapabilityLevel::L1 => 1,
            CapabilityLevel::L2 => 2,
            CapabilityLevel::L3 => 3,
            CapabilityLevel::L4 => 4,
            CapabilityLevel::L5 => 5,
        }
    }

    /// 是否为低等级（L1/L2），低于该档建议启用进化提升
    pub fn is_low(&self) -> bool {
        matches!(self, CapabilityLevel::L1 | CapabilityLevel::L2)
    }

    /// 提升一级（L5 封顶）
    pub fn promote(&self) -> CapabilityLevel {
        match self {
            CapabilityLevel::L1 => CapabilityLevel::L2,
            CapabilityLevel::L2 => CapabilityLevel::L3,
            CapabilityLevel::L3 => CapabilityLevel::L4,
            CapabilityLevel::L4 => CapabilityLevel::L5,
            CapabilityLevel::L5 => CapabilityLevel::L5,
        }
    }

    /// 从护照多维数据派生能力等级（maturity score 0-100 → L1-L5）
    ///
    /// # 评分维度
    /// - 规划复杂度（30%）：Simple=35 / Moderate=65 / Complex=95
    /// - IQ 需求（25%）：`model_iq_requirement`（0-100）
    /// - 可靠性（30%）：`stats.recent_success_rate` × 100（无调用数据给中性 60）
    /// - 效率（15%）：耗时与成本综合（越快越便宜越高，未知给中性 70）
    ///
    /// # 映射
    /// ≥80 → L5 ｜ ≥60 → L4 ｜ ≥40 → L3 ｜ ≥20 → L2 ｜ 其余 → L1
    pub fn derive(dto: &CapabilityPassportDto) -> CapabilityLevel {
        let complexity = match dto.planning_complexity {
            PlanningComplexity::Simple => 35.0,
            PlanningComplexity::Moderate => 65.0,
            PlanningComplexity::Complex => 95.0,
        };
        let iq = f64::from(dto.model_iq_requirement.clamp(0, 100));

        let reliability = if dto.stats.total_calls > 0 {
            dto.stats.recent_success_rate.clamp(0.0, 1.0) * 100.0
        } else {
            60.0
        };

        let speed = dto
            .avg_duration_seconds
            .map(|d| (1.0 - d / 120.0).clamp(0.0, 1.0) * 100.0)
            .unwrap_or(70.0);
        let cost = dto
            .estimated_cost_usd
            .map(|c| (1.0 - c / 0.10).clamp(0.0, 1.0) * 100.0)
            .unwrap_or(70.0);
        let efficiency = (speed + cost) / 2.0;

        let score = 0.30 * complexity + 0.25 * iq + 0.30 * reliability + 0.15 * efficiency;

        if score >= 80.0 {
            CapabilityLevel::L5
        } else if score >= 60.0 {
            CapabilityLevel::L4
        } else if score >= 40.0 {
            CapabilityLevel::L3
        } else if score >= 20.0 {
            CapabilityLevel::L2
        } else {
            CapabilityLevel::L1
        }
    }
}

// ── 输出能力 ──────────────────────────────────────

/// 输出格式能力声明（用于设备兼容性过滤）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputCapabilities {
    #[serde(default)]
    pub supports_text: bool,
    #[serde(default)]
    pub supports_table: bool,
    #[serde(default)]
    pub supports_chart: bool,
    #[serde(default)]
    pub supports_image: bool,
    #[serde(default)]
    pub supports_interactive: bool,
}

// ── 能力统计快照 ──────────────────────────────────

/// 能力运行时统计（从 ToolMetrics / UsagePattern 聚合）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityStats {
    /// 总调用次数
    pub total_calls: u64,
    /// 成功次数
    pub success_count: u64,
    /// 平均执行耗时（秒）
    pub avg_duration_seconds: f64,
    /// 近 5 次成功率（0.0-1.0）
    pub recent_success_rate: f64,
    /// 熔断状态（"closed"/"open"/"half_open"）
    pub circuit_state: String,
}

// ── 暴露模式（暴露层架构：被动自动暴露 vs 主动按需注入） ──────────

/// 能力暴露模式 —— 与 kind/domain 正交，决定能力如何暴露给 LLM。
///
/// 背景：项目此前未区分「能力发现/认知编排」与「工具/技能自动暴露」两条链路的
/// 差异 —— 被动模式（直连 agent）按功能域全量塞工具，主动模式（认知编排执行）
/// 工具列表为空（注释承诺"由能力发现路径注入"但未实现）。本枚举显式化暴露策略：
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityExposure {
    /// 被动对话全量暴露 + 主动模式命中即注入（轻量 built-in 工具、常用技能）
    #[default]
    Auto,
    /// 仅能力发现命中时按需注入（重型工具、Tauri 命令桥等上下文昂贵的能力）
    OnDemand,
    /// 永不自动暴露，仅参与路由/编排（system_* 元能力）
    Managed,
}

/// 护照到真实工具定义的引用 —— 主动模式按需注入闭环的关键。
///
/// 护照是元数据快照，命中后需凭此引用从 UnifiedToolRegistry 反查真实 ChatTool
/// 定义（schema）注入 LLM 上下文，否则"发现的能力执行不了"。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityToolRef {
    /// 注册表中的工具名（ChatTool.function.name / UnifiedToolRegistry 键名）
    pub tool_name: String,
    /// 注册表来源：builtin / mcp / skill / tauri_command
    #[serde(default)]
    pub registry: String,
}

/// 模板占位符定义 —— 模板能力（`Template`）的参数占位（如 `{{target_ip}}` / `{{date_range}}`）。
///
/// 模板命中后不直接执行，认知编排仅收到"可实例化为具体 Skill"的提示；
/// 占位符类型用于匹配用户输入中的实体（如 IP、日期区间）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceholderDef {
    /// 占位符名（不含双花括号，如 `target_ip`）
    pub name: String,
    /// 期望类型：string / ip / date_range / number / enum
    #[serde(default)]
    pub placeholder_type: String,
    /// 占位符说明（供认知编排理解如何填充）
    #[serde(default)]
    pub description: String,
}

/// 随能力附带的知识片段（P2：能力与信息分离）。
///
/// 遵循"知识和能力分离"原则：知识片段作为能力描述的补充信息随护照返回，
/// 注入认知层上下文（如"漏洞扫描"Skill 附带"当前支持的 CVE 编号范围"），
/// 但不作为独立执行能力暴露。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSnippet {
    /// 片段键（如 "supported_cve_range"）
    pub key: String,
    /// 片段内容（随能力描述注入上下文）
    pub content: String,
}

// ── 能力数字护照 trait ─────────────────────────────

/// 能力护照 — 所有可被发现的能力的统一元数据接口
///
/// # 实现者
/// - `ToolInfo`（工具）
/// - `WorkflowTemplate`（工作流）
/// - 知识库/Agent/Skill 等
///
/// # 设计原则
/// - 只暴露元数据，不暴露执行逻辑
/// - 所有方法提供默认实现，便于渐进式集成
/// - 负面场景是关键创新点：防止误匹配
pub trait CapabilityPassport: Send + Sync {
    /// 唯一 ID（格式: `{kind}:{id}`，如 `tool:read_file`）
    fn capability_id(&self) -> String {
        String::new()
    }

    /// 能力名称
    fn name(&self) -> &str {
        ""
    }

    /// 能力描述（用于语义匹配）
    fn description(&self) -> &str {
        ""
    }

    /// 一句话摘要（渐进式披露 L0 索引层专用）。
    ///
    /// 与 [`Self::description`] 的分工：`description` 面向语义检索，可以写得很长；
    /// `summary` 面向「注入系统提示的能力目录」，必须一行内。二者分离才不必为了
    /// 目录而砍检索语料，也不必把长描述整段塞进系统提示。
    ///
    /// 返回 `None` = 未声明，索引层回退为截断 `description`。
    fn summary(&self) -> Option<String> {
        None
    }

    /// 能力类型
    fn kind(&self) -> CapabilityKind {
        CapabilityKind::Tool
    }

    /// 所属业务域
    fn domain(&self) -> CapabilityDomain {
        CapabilityDomain::General
    }

    /// 能力来源（默认内置）
    fn source(&self) -> CapabilitySource {
        CapabilitySource::Builtin
    }

    /// 能力可进化性（默认不可进化）
    fn evolvability(&self) -> CapabilityEvolvability {
        CapabilityEvolvability::None
    }

    /// 子分类（L2 集群标识，用于三层路由的第二层）
    ///
    /// 返回 `CapabilityCluster::cluster_id`，如 `"general_file_ops"`。
    /// 空字符串表示未分类。
    fn sub_category(&self) -> String {
        String::new()
    }

    /// 输入参数 JSON Schema（None = 无固定入参）
    fn input_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// 标签列表（用于硬匹配和索引增强）
    fn tags(&self) -> Vec<String> {
        Vec::new()
    }

    /// **负面场景** — "我不做这个"的描述
    ///
    /// # 示例
    /// - "此工具不处理 PDF 文件"
    /// - "此工作流仅支持 A 股，不支持港股"
    /// - "此知识库不包含 2020 年以前的数据"
    ///
    /// 检索时若用户 query 与负面场景语义相似度超过阈值，将直接剔除。
    fn negative_scenarios(&self) -> Vec<String> {
        Vec::new()
    }

    /// 安全等级
    fn security_level(&self) -> SecurityLevel {
        SecurityLevel::Public
    }

    /// 能力可见性（元能力隔离核心）
    ///
    /// SystemOnly 能力不可被用户发现，仅内部服务可调用。
    fn visibility(&self) -> Visibility {
        Visibility::Public
    }

    /// 调用权限控制
    fn caller_permissions(&self) -> CallerPermissions {
        CallerPermissions::new()
    }

    /// 模态支持
    fn modality_support(&self) -> ModalitySupport {
        ModalitySupport::default()
    }

    /// 输出能力
    fn output_capabilities(&self) -> OutputCapabilities {
        OutputCapabilities::default()
    }

    /// 单次调用预估成本（美元），None = 未知
    fn estimated_cost_usd(&self) -> Option<f64> {
        None
    }

    /// 预估平均耗时（秒），None = 未知
    fn avg_duration_seconds(&self) -> Option<f64> {
        None
    }

    /// 能力定义版本（语义化，如 "1.2.3"）。None = 未声明。
    fn version(&self) -> Option<String> {
        None
    }

    /// 能力所有者（团队/个人标识）。None = 未声明。
    fn owner(&self) -> Option<String> {
        None
    }

    /// 输出结构的 JSON Schema（None = 无固定输出结构）
    fn output_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// 执行模式（Sync/Async/Streaming），默认同步
    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sync
    }

    /// 单次执行最大超时（毫秒）。None = 未声明（使用引擎默认）
    fn timeout_ms(&self) -> Option<u64> {
        None
    }

    /// Tool 实现契约（仅 Tool 类能力有效：REST/gRPC/本地函数如何调用）
    fn implementation(&self) -> Option<ToolImplementation> {
        None
    }

    /// Skill 结构化执行步骤（仅 Skill 类能力有效；Toolchain 用 `steps`）
    fn skill_steps(&self) -> Vec<SkillStep> {
        Vec::new()
    }

    /// 规划复杂度
    fn planning_complexity(&self) -> PlanningComplexity {
        PlanningComplexity::Simple
    }

    /// 所需模型最低 "智商分"（0-100，预留接口）
    fn model_iq_requirement(&self) -> u8 {
        0
    }

    /// 实验分组（None = 所有用户可见；Some("B") = 仅 B 组可见）
    fn experiment_group(&self) -> Option<String> {
        None
    }

    /// 推荐执行专家（AgentProfile ID）。
    ///
    /// 认知编排在 Agent 执行路径（Ask/Act/Delegate）下据此自动选择专家；
    /// 返回 `None` 时由路由决策/应用层兜底到默认专家。
    fn default_agent_profile(&self) -> Option<String> {
        None
    }

    /// 能力统计快照（可选，运行时注入）
    fn stats(&self) -> CapabilityStats {
        CapabilityStats::default()
    }

    /// 是否已启用
    fn is_enabled(&self) -> bool {
        true
    }

    /// 转换为可序列化的 DTO（供前端展示和索引存储）
    fn to_passport_dto(&self) -> CapabilityPassportDto {
        let mut dto = CapabilityPassportDto {
            capability_id: self.capability_id(),
            name: self.name().to_string(),
            description: self.description().to_string(),
            summary: self.summary(),
            version: self.version(),
            owner: self.owner(),
            created_at: None,
            updated_at: None,
            kind: self.kind(),
            domain: self.domain(),
            source: self.source(),
            evolvable: self.evolvability(),
            sub_category: self.sub_category(),
            visibility: self.visibility(),
            caller_permissions: self.caller_permissions(),
            input_schema: self.input_schema(),
            output_schema: self.output_schema(),
            implementation: self.implementation(),
            tags: self.tags(),
            negative_scenarios: self.negative_scenarios(),
            security_level: self.security_level(),
            modality_support: self.modality_support(),
            output_capabilities: self.output_capabilities(),
            estimated_cost_usd: self.estimated_cost_usd(),
            avg_duration_seconds: self.avg_duration_seconds(),
            execution_mode: self.execution_mode(),
            timeout_ms: self.timeout_ms(),
            planning_complexity: self.planning_complexity(),
            model_iq_requirement: self.model_iq_requirement(),
            experiment_group: self.experiment_group(),
            agent_profile_id: self.default_agent_profile(),
            level: CapabilityLevel::L1,
            stats: self.stats(),
            enabled: self.is_enabled(),
            exposure: CapabilityExposure::Auto,
            tool_ref: None,
            aliases: Vec::new(),
            steps: Vec::new(),
            skill_steps: self.skill_steps(),
            placeholders: Vec::new(),
            template_body: None,
            instantiates_to: None,
            example_instance: None,
            upstream: Vec::new(),
            downstream: Vec::new(),
            preconditions: Vec::new(),
            attached_snippets: Vec::new(),
        };
        dto.level = CapabilityLevel::derive(&dto);
        dto
    }
}

// ── CapabilityPassportDto（序列化 DTO） ─────────────

/// 能力护照的序列化 DTO（用于索引存储和前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityPassportDto {
    pub capability_id: String,
    pub name: String,
    pub description: String,
    /// 一句话摘要（渐进式披露 L0 索引层）。None = 未声明，目录回退截断 `description`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// 能力定义版本（语义化，如 "1.2.3"）。None = 未声明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// 能力所有者（团队/个人标识）。None = 未声明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// 创建时间（unix ms）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    /// 最后更新时间（unix ms）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    pub kind: CapabilityKind,
    pub domain: CapabilityDomain,
    /// L2 子分类（集群 ID，用于三层路由第二层）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sub_category: String,
    /// 🔒 元能力隔离核心：可见性
    #[serde(default)]
    pub visibility: Visibility,
    /// 🔒 元能力隔离核心：调用权限
    #[serde(default)]
    pub caller_permissions: CallerPermissions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    /// 输出结构的 JSON Schema（None = 无固定输出结构）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    /// Tool 实现契约（仅 Tool 类能力有效：REST/gRPC/本地函数如何调用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<ToolImplementation>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub negative_scenarios: Vec<String>,
    pub security_level: SecurityLevel,
    #[serde(default)]
    pub modality_support: ModalitySupport,
    #[serde(default)]
    pub output_capabilities: OutputCapabilities,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_duration_seconds: Option<f64>,
    /// 执行模式（Sync/Async/Streaming），默认 Sync
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    /// 单次执行最大超时（毫秒）。None = 未声明（使用引擎默认）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    pub planning_complexity: PlanningComplexity,
    #[serde(default)]
    pub model_iq_requirement: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment_group: Option<String>,
    /// 推荐执行专家（AgentProfile ID）。认知编排 Agent 执行路径据此自动选择专家。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_profile_id: Option<String>,
    /// 能力等级（由多维数据派生；进化后可提升）
    #[serde(default)]
    pub level: CapabilityLevel,
    #[serde(default)]
    pub stats: CapabilityStats,
    pub enabled: bool,
    /// 能力来源（内置 / 插件），用于溯源与进化边界判断
    #[serde(default)]
    pub source: CapabilitySource,
    /// 能力可进化性（决定进化引擎分发边界）
    #[serde(default)]
    pub evolvable: CapabilityEvolvability,
    /// 暴露模式（Auto=被动全量+主动命中注入；OnDemand=仅命中注入；Managed=仅路由）
    #[serde(default)]
    pub exposure: CapabilityExposure,
    /// 真实工具定义引用（主动模式命中后凭此注入 chat_tools，解决"发现的能力执行不了"）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_ref: Option<CapabilityToolRef>,
    /// 别名列表（用户口语→能力 ID 的映射，检索时命中别名直接进候选；如 "发邮件"→mail_send）
    #[serde(default)]
    pub aliases: Vec<String>,
    /// 工具链步骤（仅 `Toolchain` 类型有效：按序排列的 capability_id 列表，线性串接、失败短路）
    #[serde(default)]
    pub steps: Vec<String>,
    /// Skill 结构化执行步骤（仅 `Skill` 类型有效：步骤级参数/条件/错误处理）
    #[serde(default)]
    pub skill_steps: Vec<SkillStep>,
    /// 模板占位符（仅 `Template` 类型有效：命中后提示"可实例化"，不直接执行）
    #[serde(default)]
    pub placeholders: Vec<PlaceholderDef>,
    /// 模板正文（仅 `Template` 类型有效：含占位符的模板内容，如 "扫描 {{target_ip}} 的 {{port_range}}"）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_body: Option<String>,
    /// 实例化目标类型（仅 `Template` 类型有效：实例化后生成 Skill 或 Workflow）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instantiates_to: Option<CapabilityKind>,
    /// 示例实例（仅 `Template` 类型有效：能力 ID 或内联定义，供 LLM 参考如何实例化）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example_instance: Option<String>,
    /// 上游依赖能力 ID 列表（关联扩展：检索命中后一跳向上扩展）
    #[serde(default)]
    pub upstream: Vec<String>,
    /// 下游依赖能力 ID 列表（关联扩展：检索命中后一跳向下扩展）
    #[serde(default)]
    pub downstream: Vec<String>,
    /// 前置条件（P1：Skill preconditions，如 "network_available" / "db_configured"）。
    /// 条件检查启用（FilterContext.conditions_checked）时，任一未满足即过滤掉该能力。
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// 附带知识片段（P2：能力与信息分离，随能力描述注入上下文，不单独执行）
    #[serde(default)]
    pub attached_snippets: Vec<KnowledgeSnippet>,
}

// ── Template 实例化（统一能力模型第四层，任务③） ──

/// Template 实例化产物。
///
/// 仅承载"已填充占位符的模板正文 + 目标类型 + 结构化步骤"；
/// **不发明 Skill/Workflow 文件格式** —— 真正的产物生成留给触发该实例化的调用点
/// （统一能力模型 §六 P2-②：执行器待真实 "模板→Skill" 场景驱动，避免凭空设计）。
#[derive(Debug, Clone, PartialEq)]
pub struct InstantiatedTemplate {
    /// 占位符已填充的模板正文
    pub filled_body: String,
    /// 实例化目标类型（Skill / Workflow）
    pub instantiates_to: Option<CapabilityKind>,
    /// 结构化步骤（直接复用护照声明的 `skill_steps`）
    pub skill_steps: Vec<SkillStep>,
    /// 未被示例实例覆盖、仍保留的占位符（供调用点补全或告警）
    pub unresolved_placeholders: Vec<String>,
}

/// 将 `Template` 类护照实例化为可执行草案。
///
/// 触发条件（任务③）：出现真实 "Template → Skill/Workflow" 实例化需求时，
/// 调用点持有一个 `Template` 类护照调用本函数即可获得填充后的正文与步骤。
///
/// 占位符解析规则（`{{key}}`，key = 字母/数字/下划线/短横线，可含空白）：
/// - `example_instance` 可解析为 JSON 对象 → 用 `key → JSON 值字符串` 填充各 `{{key}}`
/// - `example_instance` 可解析为 JSON 标量/数组 → 整值作为 `{{example}}` 的填充
/// - `example_instance` 非 JSON → 原文作为 `{{example}}` 的填充
/// - 未被覆盖的占位符保留并记入 `unresolved_placeholders`
pub fn instantiate_template(
    passport: &CapabilityPassportDto,
) -> Result<InstantiatedTemplate, String> {
    let template_body = passport
        .template_body
        .as_ref()
        .ok_or_else(|| "template_body 未定义，无法实例化".to_string())?;

    let mut values: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(example) = &passport.example_instance {
        match serde_json::from_str::<serde_json::Value>(example) {
            Ok(serde_json::Value::Object(map)) => {
                for (k, v) in map {
                    values.insert(k, fill_value_string(&v));
                }
            },
            Ok(v) => {
                values.insert("example".to_string(), fill_value_string(&v));
            },
            Err(_) => {
                values.insert("example".to_string(), example.clone());
            },
        }
    }

    let mut unresolved = Vec::new();
    let filled_body = fill_template_placeholders(template_body, &values, &mut unresolved);

    Ok(InstantiatedTemplate {
        filled_body,
        instantiates_to: passport.instantiates_to,
        skill_steps: passport.skill_steps.clone(),
        unresolved_placeholders: unresolved,
    })
}

/// 把 JSON 标量值转为占位符填充文本 —— **填原始值，不填 JSON 字面量**。
///
/// `template_body` 是给人/LLM 读的指令（`扫描 {{target_ip}} 的 {{port_range}}`），
/// 填 `"10.0.0.1"`（带引号）是序列化产物而非调用方意图。
/// 仅字符串需要剥引号：数字 / bool / null 的 `to_string()` 本就无引号；
/// 数组 / 对象保留 JSON 形式（没有更合理的标量表示）。
fn fill_value_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// 手写占位符替换（避免引入 regex 依赖）：把 `{{key}}` 用 `values` 填充，
/// 未覆盖的占位符原样保留并记入 `unresolved`。
fn fill_template_placeholders(
    body: &str,
    values: &std::collections::HashMap<String, String>,
    unresolved: &mut Vec<String>,
) -> String {
    let mut out = String::with_capacity(body.len());
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
            let mut close = None;
            let mut j = i + 2;
            while j + 1 < chars.len() {
                if chars[j] == '}' && chars[j + 1] == '}' {
                    close = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(close) = close {
                let key: String = chars[i + 2..close].iter().collect::<String>().trim().to_string();
                match values.get(&key) {
                    Some(v) => out.push_str(v),
                    None => {
                        unresolved.push(key);
                        out.extend(chars[i..=close + 1].iter());
                    },
                }
                i = close + 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

impl Default for CapabilityPassportDto {
    fn default() -> Self {
        Self {
            capability_id: String::new(),
            name: String::new(),
            description: String::new(),
            summary: None,
            version: None,
            owner: None,
            created_at: None,
            updated_at: None,
            kind: CapabilityKind::Tool,
            domain: CapabilityDomain::General,
            source: CapabilitySource::Builtin,
            evolvable: CapabilityEvolvability::None,
            sub_category: String::new(),
            visibility: Visibility::Public,
            caller_permissions: CallerPermissions::new(),
            input_schema: None,
            output_schema: None,
            implementation: None,
            tags: Vec::new(),
            negative_scenarios: Vec::new(),
            security_level: SecurityLevel::Public,
            modality_support: ModalitySupport::default(),
            output_capabilities: OutputCapabilities::default(),
            estimated_cost_usd: None,
            avg_duration_seconds: None,
            execution_mode: ExecutionMode::Sync,
            timeout_ms: None,
            planning_complexity: PlanningComplexity::Simple,
            model_iq_requirement: 0,
            experiment_group: None,
            agent_profile_id: None,
            level: CapabilityLevel::L1,
            stats: CapabilityStats::default(),
            enabled: true,
            exposure: CapabilityExposure::Auto,
            tool_ref: None,
            aliases: Vec::new(),
            steps: Vec::new(),
            skill_steps: Vec::new(),
            placeholders: Vec::new(),
            template_body: None,
            instantiates_to: None,
            example_instance: None,
            upstream: Vec::new(),
            downstream: Vec::new(),
            preconditions: Vec::new(),
            attached_snippets: Vec::new(),
        }
    }
}

impl CapabilityPassportDto {
    /// 是否为系统专用能力（不可被用户发现）
    pub fn is_system_only(&self) -> bool {
        self.visibility.is_system_only() || self.domain.is_system()
    }

    /// L2 集群标签 —— 能力目录两级分组与 `CapabilityBrowse` 逐层下钻共用的分组依据。
    ///
    /// 空缺回落 `"general"`，保证任何护照都能归入一个集群；权威定义在 harness，
    /// 目录渲染（wiring 层）与下钻工具（tools 层）必须共用，否则两处分组漂移。
    pub fn cluster_label(&self) -> &str {
        let sub = self.sub_category.trim();
        if sub.is_empty() { "general" } else { sub }
    }

    /// 是否可作为内容交给 LLM 上下文（能力目录与定义层展开共用同一判据）。
    ///
    /// 比 [`Self::is_system_only`] 更严：`PrivilegedOnly` / `Hidden` 同样不得进入
    /// 系统提示 —— 把能力名交给模型属比检索更宽的危险面。
    /// 索引层（wiring 渲染目录）与定义层（`CapabilityView` 展开）必须共用此方法，
    /// 否则两处过滤条件会各自漂移，隔离口径被静默放宽。
    pub fn is_user_visible(&self) -> bool {
        self.enabled && self.visibility.is_discoverable() && !self.domain.is_system()
    }

    /// 是否可被指定角色调用
    pub fn can_be_called_by(&self, role: &str) -> bool {
        self.caller_permissions.can_be_called_by(role)
    }

    /// 派生并回填能力等级（索引入口统一调用，保证所有护照的 level 始终准确）
    pub fn with_derived_level(mut self) -> Self {
        self.level = CapabilityLevel::derive(&self);
        self
    }
}

// ── 用户偏好（用于动态调整 α/β/γ/δ 权重） ────────────

/// 能力发现的用户偏好权重
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryWeights {
    /// 语义相似度权重（α）
    pub alpha: f64,
    /// 历史成功率权重（β）
    pub beta: f64,
    /// 耗时惩罚系数（γ）
    pub gamma: f64,
    /// 成本惩罚系数（δ）
    pub delta: f64,
    /// 个性化提权比例（历史使用 +15% → 0.15）
    pub personalization_boost: f64,
    /// 冷启动探索提权（新能力 +20% → 0.20）
    pub exploration_boost: f64,
}

impl Default for DiscoveryWeights {
    fn default() -> Self {
        Self {
            alpha: 0.4,
            beta: 0.3,
            gamma: 0.15,
            delta: 0.15,
            personalization_boost: 0.15,
            exploration_boost: 0.20,
        }
    }
}

// ── 会话预算 ──────────────────────────────────────

/// 单次会话预算（用于维度五：资源/成本过滤）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionBudget {
    /// 总预算上限（美元）
    pub max_total_usd: f64,
    /// 单次调用上限（美元）
    pub max_per_call_usd: f64,
    /// 已使用金额
    pub used_usd: f64,
}

impl SessionBudget {
    pub fn remaining(&self) -> f64 {
        (self.max_total_usd - self.used_usd).max(0.0)
    }

    /// 检查单次调用是否超出预算
    pub fn can_afford(&self, cost_usd: f64) -> bool {
        cost_usd <= self.max_per_call_usd && cost_usd <= self.remaining()
    }
}

impl Default for SessionBudget {
    fn default() -> Self {
        Self { max_total_usd: 0.10, max_per_call_usd: 0.05, used_usd: 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instantiate_template_fills_object_placeholders() {
        let mut p = CapabilityPassportDto {
            capability_id: "template:scan".to_string(),
            kind: CapabilityKind::Template,
            template_body: Some("扫描 {{target_ip}} 的 {{port_range}}".to_string()),
            example_instance: Some(r#"{"target_ip":"10.0.0.1","port_range":"1-1024"}"#.to_string()),
            instantiates_to: Some(CapabilityKind::Skill),
            ..Default::default()
        };
        p.skill_steps = vec![SkillStep {
            step_id: "s1".to_string(),
            capability_id: "tool:scan".to_string(),
            ..Default::default()
        }];

        let inst = instantiate_template(&p).expect("实例化应成功");
        assert_eq!(inst.filled_body, "扫描 10.0.0.1 的 1-1024");
        assert_eq!(inst.instantiates_to, Some(CapabilityKind::Skill));
        assert_eq!(inst.skill_steps.len(), 1);
        assert!(inst.unresolved_placeholders.is_empty());
    }

    #[test]
    fn instantiate_template_records_unresolved_and_scalar_example() {
        let p = CapabilityPassportDto {
            capability_id: "template:probe".to_string(),
            kind: CapabilityKind::Template,
            template_body: Some("探测 {{missing}} 与 {{example}}".to_string()),
            example_instance: Some(r#""single-sample""#.to_string()),
            ..Default::default()
        };

        let inst = instantiate_template(&p).expect("实例化应成功");
        // 标量分支与对象分支同语义：填原始值，剥掉 JSON 字面量引号
        assert_eq!(inst.filled_body, "探测 {{missing}} 与 single-sample");
        assert_eq!(inst.unresolved_placeholders, vec!["missing".to_string()]);
    }

    #[test]
    fn instantiate_template_errors_without_body() {
        let p = CapabilityPassportDto {
            capability_id: "template:empty".to_string(),
            kind: CapabilityKind::Template,
            ..Default::default()
        };
        assert!(instantiate_template(&p).is_err());
    }

    #[test]
    fn resolve_execution_mode_clamps_non_executable_kinds() {
        // 可执行 kind 保留护照声明
        assert_eq!(
            resolve_execution_mode(ExecutionMode::Streaming, CapabilityKind::Tool),
            ExecutionMode::Streaming
        );
        assert_eq!(
            resolve_execution_mode(ExecutionMode::Async, CapabilityKind::Workflow),
            ExecutionMode::Async
        );
        // 非执行 kind 强制 Sync（避免把"声明"误当"可流式执行"）
        assert_eq!(
            resolve_execution_mode(ExecutionMode::Streaming, CapabilityKind::KnowledgeBase),
            ExecutionMode::Sync
        );
        assert_eq!(
            resolve_execution_mode(ExecutionMode::Async, CapabilityKind::Template),
            ExecutionMode::Sync
        );
    }
}
