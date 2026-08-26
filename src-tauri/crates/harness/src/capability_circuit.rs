// SPDX-License-Identifier: AGPL-3.0-only
//! 主动式能力管理：熔断 / 补全 / 热更新
//!
//! # 三大主动玩法
//! 1. CapabilityCircuitBreaker — 能力级熔断降级
//! 2. CapabilityCompleter — 主动式关联能力补全
//! 3. CapabilityHotSwapper — 热更新后台刷新

use crate::capability::CapabilityPassportDto;
use crate::circuit_breaker::CircuitState;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 能力熔断器 ────────────────────────────────────

/// 能力级熔断器（扩展基础 CircuitBreaker，增加能力降级逻辑）
#[async_trait]
pub trait CapabilityCircuitBreaker: Send + Sync {
    /// 获取指定能力的熔断状态
    async fn get_state(&self, capability_id: &str) -> CircuitState;

    /// 记录成功执行
    async fn record_success(&self, capability_id: &str);

    /// 记录失败执行
    async fn record_failure(&self, capability_id: &str);

    /// 检查能力是否可用（含熔断降级）
    ///
    /// # 返回
    /// - `Some(capability_id)`: 可用的能力（可能是原能力或降级能力）
    /// - `None`: 无可用能力
    async fn resolve_available(
        &self,
        primary_capability_id: &str,
        candidates: &[CapabilityPassportDto],
    ) -> Option<String>;

    /// 强制降级：即便原能力语义匹配 Top1，也返回替代能力
    async fn force_downgrade(&self, from_capability_id: &str, to_capability_id: &str);

    /// 获取熔断统计快照
    async fn snapshot(&self) -> Vec<CapabilityCircuitSnapshot>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityCircuitSnapshot {
    pub capability_id: String,
    pub state: CircuitState,
    pub failure_rate: f64,
    pub last_failure_ms_ago: Option<u64>,
}

// ── 能力补全器 ────────────────────────────────────

/// 能力补全器 — 主动式推荐关联能力
///
/// 场景：用户问"怎么退货"，虽然匹配了"退货工作流"，
/// 但系统发现订单状态是"已签收"，于是额外推荐"上门取件"能力。
#[async_trait]
pub trait CapabilityCompleter: Send + Sync {
    /// 基于主命中能力 + 用户上下文，生成补全建议
    async fn suggest_completions(
        &self,
        primary_match: &CapabilityPassportDto,
        user_context: &UserContextSnapshot,
    ) -> Vec<CapabilitySuggestion>;
}

/// 用户上下文快照（用于补全逻辑）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserContextSnapshot {
    /// 用户 ID
    pub user_id: Option<String>,
    /// 会话 ID
    pub conversation_id: Option<String>,
    /// 相关实体（订单号、产品名等）
    #[serde(default)]
    pub entities: Vec<ContextEntity>,
    /// 用户历史行为
    #[serde(default)]
    pub recent_actions: Vec<String>,
    /// 自定义上下文扩展
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEntity {
    pub entity_type: String,
    pub value: String,
}

/// 能力补全建议
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitySuggestion {
    pub capability_id: String,
    pub name: String,
    pub reason: String,
}

// ── 能力热更新器 ──────────────────────────────────

/// 能力热更新器 — 无需重启服务，定期刷新向量索引
#[async_trait]
pub trait CapabilityHotSwapper: Send + Sync {
    /// 启动后台刷新任务
    async fn start_background_refresh(&self, interval_secs: u64) -> Result<(), String>;

    /// 立即触发索引刷新
    async fn trigger_refresh(&self) -> Result<RefreshReport, String>;

    /// 注册新能力到索引
    async fn register_capability(&self, passport: &CapabilityPassportDto) -> Result<(), String>;

    /// 注销能力
    async fn deregister_capability(&self, capability_id: &str) -> Result<(), String>;

    /// 获取刷新状态
    async fn refresh_status(&self) -> RefreshReport;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshReport {
    pub last_refresh_ms: Option<u64>,
    pub capabilities_indexed: u64,
    pub capabilities_removed: u64,
    pub next_refresh_ms: Option<u64>,
    pub is_refreshing: bool,
}

// ── 自指熔断保护器（Layer 3） ──────────────────────────

/// 自指熔断保护器 — 防止"编排器编排编排器"的无限递归
///
/// # 设计原理
/// 当认知编排器（CognitiveRouter）作为一个系统能力时，
/// 能力发现机制可能会检索到编排器自身，导致：
/// 1. 编排器调用编排器 → 无限递归
/// 2. 编排器修改自身规则 → 自毁风险
/// 3. 编排器的 Prompt 被用户 query 影响 → 提示词注入
///
/// # 保护机制
/// ```text
/// 用户 Query → [能力发现] → [自指熔断检查] → [路由决策]
///                                    │
///                                    ├── 命中受保护能力？→ 直接剔除，不参与后续
///                                    └── 无命中？→ 放行
/// ```
///
/// # 代码级熔断
/// 不同于 FilterDimension::Visibility 的向量索引层过滤，
/// SelfReferenceCircuitBreaker 在路由决策层进行最终检查，
/// 确保即便绕过了注册/检索层，系统能力也不会被路由到。
#[async_trait]
pub trait SelfReferenceCircuitBreaker: Send + Sync {
    /// 注册受保护的系统能力 ID
    ///
    /// 调用此方法后，指定的能力 ID 将被标记为"受保护"，
    /// 后续在路由决策时会被自动剔除。
    async fn register_protected(
        &self,
        capability_id: &str,
        reason: ProtectionReason,
    ) -> Result<(), String>;

    /// 注销受保护的系统能力 ID（谨慎使用）
    async fn unregister_protected(&self, capability_id: &str) -> Result<(), String>;

    /// 检查候选能力列表，剔除所有受保护的能力
    ///
    /// # 返回
    /// - 剔除后的能力列表
    /// - 被剔除的能力 ID 列表（用于审计日志）
    async fn filter_candidates(
        &self,
        candidates: &[CapabilityPassportDto],
    ) -> (Vec<CapabilityPassportDto>, Vec<String>);

    /// 检查单个能力是否受保护
    async fn is_protected(&self, capability_id: &str) -> bool;

    /// 获取当前所有受保护的能力 ID
    async fn list_protected(&self) -> Vec<ProtectedCapability>;

    /// 紧急熔断：立即将所有系统能力标记为不可路由
    ///
    /// 在检测到自指循环或安全威胁时调用。
    async fn emergency_lockdown(&self) -> Result<(), String>;

    /// 解锁紧急熔断
    async fn emergency_unlock(&self) -> Result<(), String>;

    /// 检查是否处于紧急熔断状态
    async fn is_emergency_locked(&self) -> bool;
}

/// 保护原因
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionReason {
    /// 核心编排器（如 CognitiveRouter 本身）
    CoreOrchestrator,
    /// 路由引擎（RoutingEngine）
    RoutingEngine,
    /// 模型回退逻辑（FallbackHandler）
    FallbackLogic,
    /// Token 预算管理
    TokenBudget,
    /// 能力索引更新器
    IndexUpdater,
    /// 配置中心
    ConfigCenter,
    /// 安全防护层
    SecurityLayer,
    /// 其他系统能力
    Other { reason: String },
}

impl ProtectionReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtectionReason::CoreOrchestrator => "core_orchestrator",
            ProtectionReason::RoutingEngine => "routing_engine",
            ProtectionReason::FallbackLogic => "fallback_logic",
            ProtectionReason::TokenBudget => "token_budget",
            ProtectionReason::IndexUpdater => "index_updater",
            ProtectionReason::ConfigCenter => "config_center",
            ProtectionReason::SecurityLayer => "security_layer",
            ProtectionReason::Other { .. } => "other",
        }
    }

    pub fn is_critical(&self) -> bool {
        matches!(self, ProtectionReason::CoreOrchestrator | ProtectionReason::SecurityLayer)
    }
}

impl std::fmt::Display for ProtectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtectionReason::CoreOrchestrator => write!(f, "核心编排器"),
            ProtectionReason::RoutingEngine => write!(f, "路由引擎"),
            ProtectionReason::FallbackLogic => write!(f, "模型回退逻辑"),
            ProtectionReason::TokenBudget => write!(f, "Token预算管理"),
            ProtectionReason::IndexUpdater => write!(f, "能力索引更新器"),
            ProtectionReason::ConfigCenter => write!(f, "配置中心"),
            ProtectionReason::SecurityLayer => write!(f, "安全防护层"),
            ProtectionReason::Other { reason } => write!(f, "其他系统能力: {}", reason),
        }
    }
}

/// 受保护能力记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedCapability {
    /// 能力 ID
    pub capability_id: String,
    /// 保护原因
    pub reason: ProtectionReason,
    /// 注册时间戳（毫秒）
    pub registered_at_ms: u64,
    /// 是否为关键保护（紧急熔断时不可解锁）
    pub is_critical: bool,
}

/// 自指熔断检查结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfReferenceCheckResult {
    /// 是否通过检查
    pub passed: bool,
    /// 被剔除的能力 ID 列表
    #[serde(default)]
    pub rejected_ids: Vec<String>,
    /// 是否处于紧急熔断状态
    pub is_emergency_locked: bool,
    /// 熔断时间戳（如果处于紧急状态）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lockdown_at_ms: Option<u64>,
}

impl SelfReferenceCheckResult {
    pub fn ok() -> Self {
        Self {
            passed: true,
            rejected_ids: vec![],
            is_emergency_locked: false,
            lockdown_at_ms: None,
        }
    }

    pub fn blocked(rejected_ids: Vec<String>) -> Self {
        Self {
            passed: rejected_ids.is_empty(),
            rejected_ids,
            is_emergency_locked: false,
            lockdown_at_ms: None,
        }
    }

    pub fn emergency(lockdown_at_ms: u64) -> Self {
        Self {
            passed: false,
            rejected_ids: vec![],
            is_emergency_locked: true,
            lockdown_at_ms: Some(lockdown_at_ms),
        }
    }
}
