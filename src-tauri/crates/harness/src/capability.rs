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
}

impl CapabilityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityKind::Tool => "tool",
            CapabilityKind::Workflow => "workflow",
            CapabilityKind::KnowledgeBase => "knowledge_base",
            CapabilityKind::Agent => "agent",
            CapabilityKind::Skill => "skill",
        }
    }
}

// ── 能力域（复用 ToolDomain + 拓展） ────────────────

/// 能力所属业务域
///
/// 扩展自 `ToolDomain`，新增数据分析/内容创作/通信等业务域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityDomain {
    Core,
    General,
    Devops,
    AiMedia,
    Invest,
    Opc,
    /// 数据分析域
    DataAnalysis,
    /// 内容创作域
    ContentCreation,
    /// 通信域
    Communication,
    /// 系统域（编排器、降级控制器等内部能力，不可被用户发现）
    System,
}

impl CapabilityDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityDomain::Core => "core",
            CapabilityDomain::General => "general",
            CapabilityDomain::Devops => "devops",
            CapabilityDomain::AiMedia => "ai_media",
            CapabilityDomain::Invest => "invest",
            CapabilityDomain::Opc => "opc",
            CapabilityDomain::DataAnalysis => "data_analysis",
            CapabilityDomain::ContentCreation => "content_creation",
            CapabilityDomain::Communication => "communication",
            CapabilityDomain::System => "system",
        }
    }

    /// 是否为系统域（不可被用户发现）
    pub fn is_system(&self) -> bool {
        matches!(self, CapabilityDomain::System)
    }
}

impl From<super::tool::ToolDomain> for CapabilityDomain {
    fn from(d: super::tool::ToolDomain) -> Self {
        match d {
            super::tool::ToolDomain::Core => CapabilityDomain::Core,
            super::tool::ToolDomain::General => CapabilityDomain::General,
            super::tool::ToolDomain::Devops => CapabilityDomain::Devops,
            super::tool::ToolDomain::AiMedia => CapabilityDomain::AiMedia,
            super::tool::ToolDomain::Invest => CapabilityDomain::Invest,
            super::tool::ToolDomain::Opc => CapabilityDomain::Opc,
        }
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

// ── 输出能力 ──────────────────────────────────────

/// 输出格式能力声明（用于设备兼容性过滤）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

    /// 能力类型
    fn kind(&self) -> CapabilityKind {
        CapabilityKind::Tool
    }

    /// 所属业务域
    fn domain(&self) -> CapabilityDomain {
        CapabilityDomain::Core
    }

    /// 子分类（L2 集群标识，用于三层路由的第二层）
    ///
    /// 返回 `CapabilityCluster::cluster_id`，如 `"core_file_ops"`。
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
        CapabilityPassportDto {
            capability_id: self.capability_id(),
            name: self.name().to_string(),
            description: self.description().to_string(),
            kind: self.kind(),
            domain: self.domain(),
            sub_category: self.sub_category(),
            visibility: self.visibility(),
            caller_permissions: self.caller_permissions(),
            input_schema: self.input_schema(),
            tags: self.tags(),
            negative_scenarios: self.negative_scenarios(),
            security_level: self.security_level(),
            modality_support: self.modality_support(),
            output_capabilities: self.output_capabilities(),
            estimated_cost_usd: self.estimated_cost_usd(),
            avg_duration_seconds: self.avg_duration_seconds(),
            planning_complexity: self.planning_complexity(),
            model_iq_requirement: self.model_iq_requirement(),
            experiment_group: self.experiment_group(),
            stats: self.stats(),
            enabled: self.is_enabled(),
        }
    }
}

// ── CapabilityPassportDto（序列化 DTO） ─────────────

/// 能力护照的序列化 DTO（用于索引存储和前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityPassportDto {
    pub capability_id: String,
    pub name: String,
    pub description: String,
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
    pub planning_complexity: PlanningComplexity,
    #[serde(default)]
    pub model_iq_requirement: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment_group: Option<String>,
    #[serde(default)]
    pub stats: CapabilityStats,
    pub enabled: bool,
}

impl Default for CapabilityPassportDto {
    fn default() -> Self {
        Self {
            capability_id: String::new(),
            name: String::new(),
            description: String::new(),
            kind: CapabilityKind::Tool,
            domain: CapabilityDomain::Core,
            sub_category: String::new(),
            visibility: Visibility::Public,
            caller_permissions: CallerPermissions::new(),
            input_schema: None,
            tags: Vec::new(),
            negative_scenarios: Vec::new(),
            security_level: SecurityLevel::Public,
            modality_support: ModalitySupport::default(),
            output_capabilities: OutputCapabilities::default(),
            estimated_cost_usd: None,
            avg_duration_seconds: None,
            planning_complexity: PlanningComplexity::Simple,
            model_iq_requirement: 0,
            experiment_group: None,
            stats: CapabilityStats::default(),
            enabled: true,
        }
    }
}

impl CapabilityPassportDto {
    /// 是否为系统专用能力（不可被用户发现）
    pub fn is_system_only(&self) -> bool {
        self.visibility.is_system_only() || self.domain.is_system()
    }

    /// 是否可被指定角色调用
    pub fn can_be_called_by(&self, role: &str) -> bool {
        self.caller_permissions.can_be_called_by(role)
    }
}

// ── 用户偏好（用于动态调整 α/β/γ/δ 权重） ────────────

/// 能力发现的用户偏好权重
#[derive(Debug, Clone, Serialize, Deserialize)]
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
