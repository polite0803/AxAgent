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
            // 历史别名（兼容存量数据）
            "core" => CapabilityDomain::General,
            "invest" => CapabilityDomain::Finance,
            "opc" => CapabilityDomain::Automation,
            "quant" => CapabilityDomain::Finance,
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
            kind: self.kind(),
            domain: self.domain(),
            source: self.source(),
            evolvable: self.evolvability(),
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
            agent_profile_id: self.default_agent_profile(),
            level: CapabilityLevel::L1,
            stats: self.stats(),
            enabled: self.is_enabled(),
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
}

impl Default for CapabilityPassportDto {
    fn default() -> Self {
        Self {
            capability_id: String::new(),
            name: String::new(),
            description: String::new(),
            kind: CapabilityKind::Tool,
            domain: CapabilityDomain::General,
            source: CapabilitySource::Builtin,
            evolvable: CapabilityEvolvability::None,
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
            agent_profile_id: None,
            level: CapabilityLevel::L1,
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
