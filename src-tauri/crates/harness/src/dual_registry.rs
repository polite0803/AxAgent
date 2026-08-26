// SPDX-License-Identifier: AGPL-3.0-only
//! 双注册表管理器 — 元能力隔离核心 Layer 1
//!
//! ## 设计原则
//! - 业务能力（Business Registry）：面向用户，可被发现
//! - 系统能力（System Registry）：面向运维，不可被用户发现
//! - 物理隔离：两套独立的索引/存储，从源头杜绝误召回
//!
//! # 架构
//! ```text
//! ┌─────────────────────┐    ┌─────────────────────┐
//! │ Business Registry   │    │ System Registry    │
//! │ (面向用户)          │    │ (面向运维/架构师)    │
//! │                     │    │                     │
//! │ • 查订单、退货款    │    │ • 路由编排器        │
//! │ • 发优惠券、查物流  │    │ • Model-Fallback    │
//! │ • 所有业务工作流    │    │ • Token预算切分器   │
//! │                     │    │ • 能力索引更新器    │
//! │ ✅ 挂载到能力发现   │    │ ✅ 不挂载到发现机制  │
//! └─────────────────────┘    └─────────────────────┘
//! ```

use crate::capability::CapabilityPassportDto;
use crate::capability_filter::FilterContext;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 注册表类型 ──────────────────────────────────────

/// 注册表类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryType {
    /// 业务能力注册表：面向用户，能力发现的搜索范围
    Business,
    /// 系统能力注册表：面向运维/架构师，不暴露给用户
    System,
}

impl RegistryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegistryType::Business => "business",
            RegistryType::System => "system",
        }
    }
}

// ── 注册表错误 ──────────────────────────────────────

/// 注册表操作错误
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryError {
    /// 系统能力禁止注册到业务注册表
    SystemCapabilityBlocked { capability_id: String },
    /// 业务能力禁止注册到系统注册表
    BusinessCapabilityBlocked { capability_id: String },
    /// 能力已存在
    AlreadyExists { capability_id: String },
    /// 能力不存在
    NotFound { capability_id: String },
    /// 权限不足
    PermissionDenied { capability_id: String, reason: String },
    /// 索引操作失败
    IndexFailed { capability_id: String, error: String },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::SystemCapabilityBlocked { capability_id } => {
                write!(f, "系统能力 {} 禁止注册到业务注册表", capability_id)
            },
            RegistryError::BusinessCapabilityBlocked { capability_id } => {
                write!(f, "业务能力 {} 禁止注册到系统注册表", capability_id)
            },
            RegistryError::AlreadyExists { capability_id } => {
                write!(f, "能力 {} 已存在", capability_id)
            },
            RegistryError::NotFound { capability_id } => {
                write!(f, "能力 {} 不存在", capability_id)
            },
            RegistryError::PermissionDenied { capability_id, reason } => {
                write!(f, "权限不足：{} ({})", capability_id, reason)
            },
            RegistryError::IndexFailed { capability_id, error } => {
                write!(f, "能力 {} 索引失败: {}", capability_id, error)
            },
        }
    }
}

impl std::error::Error for RegistryError {}

// ── 系统配置存储接口 ──────────────────────────────

/// 系统配置存储 trait — 用于存储系统能力元数据（不建向量索引）
#[async_trait]
pub trait SystemConfigStore: Send + Sync {
    /// 存储系统能力配置
    async fn store(&self, passport: &CapabilityPassportDto) -> Result<(), String>;

    /// 获取系统能力配置
    async fn get(&self, capability_id: &str) -> Option<CapabilityPassportDto>;

    /// 列出所有系统能力
    async fn list_all(&self) -> Vec<CapabilityPassportDto>;

    /// 删除系统能力
    async fn remove(&self, capability_id: &str) -> Result<(), String>;

    /// 更新系统配置（如路由表、Prompt模板等）
    async fn update_config(&self, key: &str, value: serde_json::Value) -> Result<(), String>;

    /// 获取系统配置
    async fn get_config(&self, key: &str) -> Option<serde_json::Value>;
}

// ── 双注册表管理器 ──────────────────────────────────

/// 双注册表管理器 — 元能力隔离核心
///
/// # 职责
/// - 维护业务注册表（可被用户发现）和系统注册表（不可被用户发现）
/// - 强制校验注册类型，防止系统能力泄漏到业务注册表
/// - 提供统一的能力发现接口（仅查询业务注册表）
#[async_trait]
pub trait DualRegistry: Send + Sync {
    /// 注册业务能力
    ///
    /// # 强制校验
    /// - visibility 为 SystemOnly 的能力禁止注册到业务注册表
    /// - domain 为 System 的能力禁止注册到业务注册表
    async fn register_business(
        &self,
        passport: &CapabilityPassportDto,
    ) -> Result<(), RegistryError>;

    /// 注册系统能力
    ///
    /// # 强制校验
    /// - visibility 为 Public 的业务能力建议注册到业务注册表
    async fn register_system(&self, passport: &CapabilityPassportDto) -> Result<(), RegistryError>;

    /// 注销能力
    async fn unregister(
        &self,
        capability_id: &str,
        registry_type: RegistryType,
    ) -> Result<(), RegistryError>;

    /// 能力发现（仅查询业务注册表）
    ///
    /// # 物理隔离
    /// - 仅查询业务注册表
    /// - 系统能力根本不在此集合
    async fn discover(
        &self,
        query: &str,
        filter: &FilterContext,
    ) -> Result<Vec<CapabilityPassportDto>, RegistryError>;

    /// 根据 ID 获取能力（可查业务和系统注册表）
    async fn get_passport(&self, capability_id: &str) -> Option<CapabilityPassportDto>;

    /// 列出业务注册表所有能力
    async fn list_business(&self) -> Vec<CapabilityPassportDto>;

    /// 列出系统注册表所有能力
    async fn list_system(&self) -> Vec<CapabilityPassportDto>;

    /// 检查能力是否为系统专用（双重保险）
    fn is_system_capability(passport: &CapabilityPassportDto) -> bool {
        passport.visibility.is_system_only() || passport.domain.is_system()
    }

    /// 检查能力是否可被发现
    fn is_discoverable_capability(passport: &CapabilityPassportDto) -> bool {
        passport.visibility.is_discoverable() && !passport.domain.is_system()
    }
}

// ── 路由表自更新管理器接口 ──────────────────────────

/// 路由表自更新管理器 — Layer 5 自增长处理
///
/// ## 设计原则
/// - 动态更新仅修改 System Registry 和配置中心
/// - 绝不污染业务注册表
#[async_trait]
pub trait RouterSelfUpdateManager: Send + Sync {
    /// 路由表自更新（仅修改系统配置中心）
    async fn update_routing_table(&self, new_rules: Vec<RoutingRule>) -> Result<(), RegistryError>;

    /// 更新 Prompt 模板（静默）
    async fn update_prompt_template(
        &self,
        template_key: &str,
        template_content: String,
    ) -> Result<(), RegistryError>;

    /// 记录版本
    async fn record_version(
        &self,
        entity: &str,
        version_info: serde_json::Value,
    ) -> Result<(), RegistryError>;

    /// 置信度阈值自优化
    async fn optimize_confidence_threshold(
        &self,
        new_threshold: f64,
        performance_metrics: &str,
    ) -> Result<(), RegistryError>;
}

/// 路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingRule {
    /// 规则 ID
    pub rule_id: String,
    /// 源域
    pub source_domain: String,
    /// 目标簇
    pub target_cluster: String,
    /// 条件表达式（None = 无条件）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// 优先级（数字越大优先级越高）
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_priority() -> i32 {
    0
}

fn default_enabled() -> bool {
    true
}

impl RoutingRule {
    pub fn new(
        rule_id: impl Into<String>,
        source_domain: impl Into<String>,
        target_cluster: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            source_domain: source_domain.into(),
            target_cluster: target_cluster.into(),
            condition: None,
            priority: 0,
            enabled: true,
        }
    }

    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

// ── 特权管道接口（Layer 4） ──────────────────────────

/// 系统特权管道 — 系统能力执行的物理隔离通道
///
/// # 设计原理
/// 系统能力（如路由编排器、模型回退逻辑）不能通过用户网关执行，
/// 必须通过独立的特权管道。该管道具有：
/// 1. 独立的 API 端点（`/internal/system/exec`）
/// 2. mTLS 双向认证
/// 3. 严格的权限检查
/// 4. 审计日志全覆盖
///
/// # 架构
/// ```text
/// 用户请求 → [用户网关] → [能力发现] → [路由决策] → [业务管道]
///                                                          │
///                                                          └──→ 业务能力执行
///
/// 系统请求 → [特权管道] → [mTLS认证] → [权限检查] → [系统能力执行]
/// ```
#[async_trait]
pub trait SystemPrivilegedPipeline: Send + Sync {
    /// 通过特权管道执行系统能力
    ///
    /// # 参数
    /// - `capability_id`: 系统能力 ID
    /// - `input`: 能力执行输入
    /// - `caller`: 调用者信息（用于审计）
    async fn execute_system_capability(
        &self,
        capability_id: &str,
        input: &[u8],
        caller: &PrivilegedCaller,
    ) -> Result<PrivilegedExecutionResult, RegistryError>;

    /// 批量执行系统能力（用于编排链路）
    async fn execute_system_chain(
        &self,
        chain: &[PrivilegedChainStep],
        caller: &PrivilegedCaller,
    ) -> Result<Vec<PrivilegedExecutionResult>, RegistryError>;

    /// 验证调用者权限
    async fn verify_caller(
        &self,
        caller: &PrivilegedCaller,
        required_permissions: &[Privilege],
    ) -> Result<bool, RegistryError>;

    /// 获取可执行的系统能力列表
    async fn list_executable_capabilities(
        &self,
        caller: &PrivilegedCaller,
    ) -> Vec<CapabilityPassportDto>;

    /// 健康检查
    async fn health_check(&self) -> PrivilegedHealthStatus;
}

/// 特权调用者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegedCaller {
    /// 调用者 ID（系统组件标识）
    pub caller_id: String,
    /// 调用者名称
    pub caller_name: String,
    /// 权限列表
    #[serde(default)]
    pub permissions: Vec<Privilege>,
    /// 证书指纹（mTLS）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate_fingerprint: Option<String>,
    /// 是否为内部系统组件
    #[serde(default = "default_true")]
    pub is_internal: bool,
}

fn default_true() -> bool {
    true
}

impl PrivilegedCaller {
    pub fn internal(caller_id: impl Into<String>, permissions: Vec<Privilege>) -> Self {
        Self {
            caller_id: caller_id.into(),
            caller_name: String::new(),
            permissions,
            certificate_fingerprint: None,
            is_internal: true,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.caller_name = name.into();
        self
    }

    pub fn with_certificate(mut self, fingerprint: impl Into<String>) -> Self {
        self.certificate_fingerprint = Some(fingerprint.into());
        self
    }

    pub fn has_permission(&self, required: &Privilege) -> bool {
        self.permissions.contains(required)
    }

    pub fn has_any_permission(&self, required: &[Privilege]) -> bool {
        required.iter().any(|p| self.has_permission(p))
    }
}

/// 系统特权权限
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Privilege {
    /// 读取系统配置
    SystemConfigRead,
    /// 写入系统配置
    SystemConfigWrite,
    /// 执行核心编排器
    CoreOrchestratorExecute,
    /// 修改路由规则
    RoutingRuleModify,
    /// 管理能力索引
    CapabilityIndexManage,
    /// 管理模型回退
    FallbackManage,
    /// 紧急熔断操作
    EmergencyOperation,
    /// 审计日志读取
    AuditLogRead,
    /// 完全访问（最高权限）
    FullAccess,
}

impl Privilege {
    pub fn as_str(&self) -> &'static str {
        match self {
            Privilege::SystemConfigRead => "system_config_read",
            Privilege::SystemConfigWrite => "system_config_write",
            Privilege::CoreOrchestratorExecute => "core_orchestrator_execute",
            Privilege::RoutingRuleModify => "routing_rule_modify",
            Privilege::CapabilityIndexManage => "capability_index_manage",
            Privilege::FallbackManage => "fallback_manage",
            Privilege::EmergencyOperation => "emergency_operation",
            Privilege::AuditLogRead => "audit_log_read",
            Privilege::FullAccess => "full_access",
        }
    }
}

/// 特权管道执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegedExecutionResult {
    /// 执行是否成功
    pub success: bool,
    /// 系统能力 ID
    pub capability_id: String,
    /// 输出数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息（失败时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 审计追踪 ID
    pub audit_trace_id: String,
}

/// 特权管道链式执行步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegedChainStep {
    /// 步骤序号
    pub step_index: u32,
    /// 系统能力 ID
    pub capability_id: String,
    /// 输入数据
    pub input: serde_json::Value,
    /// 依赖的前序步骤输出索引
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<u32>,
}

/// 特权管道健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegedHealthStatus {
    /// 是否健康
    pub healthy: bool,
    /// 活跃的连接数
    pub active_connections: u64,
    /// 最近执行时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_execution_ms: Option<u64>,
    /// 错误率（0-1）
    pub error_rate: f64,
    /// 版本信息
    pub version: String,
}
