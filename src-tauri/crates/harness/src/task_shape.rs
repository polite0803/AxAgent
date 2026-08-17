// SPDX-License-Identifier: AGPL-3.0-only

//! 任务形态分类器 — 原则三核心标尺。
//!
//! 在能力路由之前，先显式做「任务形态」分类，以"上下文保留成本 × 安全隔离需求"
//! 两条标尺决定合并/拆分策略。后续所有编排策略都消费 [`TaskShapeDecision`]，
//! 而非再靠隐式启发式。
//!
//! ## 设计动机
//!
//! 当前能力路由是"内容→能力命中→分发"的单轴逻辑，缺少原则三要求的两轴标尺。
//! 本模块在路由前先显式分类任务形态，作为 P1/P2 所有编排策略的唯一输入。

use crate::runtime_types::permissions::PermissionMode;
use crate::runtime_types::session::Session;
use serde::{Deserialize, Serialize};

// ── 两条标尺枚举 ────────────────────────────────────────────

/// 上下文保留成本（高连贯性 → 合并优先）。
///
/// 衡量任务对上下文连贯性的依赖程度：
/// - 低成本 → 可安全拆分（专家独立处理后回流结果）
/// - 高成本 → 优先合并（避免转述损耗）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextRetentionCost {
    /// 单次问答 / 无状态任务 → 成本极低
    Ephemeral = 0,
    /// 有回溯需求但可断点续传 → 中
    Resumable = 1,
    /// 长周期项目管理 / 对话流不可拆 → 高
    LongRunningProject = 2,
    /// 跨天持续演进 / 强记忆依赖 → 极高
    PersistentJourney = 3,
}

/// 安全隔离需求（高风险 → 拆分 + 物理权限墙优先）。
///
/// 衡量任务对权限隔离的要求：
/// - 低需求 → 可在主控上下文内直接处理
/// - 高需求 → 必须拆分到独立沙箱 + 强制权限墙
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityIsolationNeed {
    /// 纯读取 / 纯思考 → 不需要隔离
    ReadOnlyThought = 0,
    /// 工作区写入 / 常规代码 → 可合并
    WorkspaceWrite = 1,
    /// 专业领域（法务/财务审计/密钥）→ 强制沙箱
    SandboxRequired = 2,
    /// 高危操作（转账/发布/删除）→ 强制审批 + 气隔
    ApprovalAndAirGap = 3,
}

// ── 执行策略 ───────────────────────────────────────────────

/// 分类器输出的执行策略建议。
///
/// 下游编排器（CognitiveRouter / OrchestratorExecutor）根据此枚举
/// 决定是自行处理、委派单个专家、并行调度多个专家，还是走动态 DAG。
///
/// ## serde 注解说明
///
/// - 顶层 `rename_all = "camelCase"` 将变体名（HandleLocally / DelegateSingleExpert 等）
///   转为 camelCase
/// - 变体内部字段 **不继承** enum 级 rename_all，必须用 `#[serde(rename)]` 手动转
///   camelCase，保持与前端 TS 类型 `expertId` / `expertIds` 对齐
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionStrategy {
    /// 主控 Agent 自行处理（高上下文 + 低风险）
    HandleLocally,
    /// 委派单个专家 Agent 处理（强专业壁垒 + 中风险）
    DelegateSingleExpert {
        /// 专家 ID（对应 agency_experts 表的主键）
        #[serde(rename = "expertId")]
        expert_id: String,
    },
    /// 并行调度多个专家（专业壁垒正交 + 可独立上下文）
    ParallelExperts {
        /// 专家 ID 列表
        #[serde(rename = "expertIds")]
        expert_ids: Vec<String>,
    },
    /// 走 Orchestrator 动态拆 DAG（中等上下文 + 中等风险，可断点）
    OrchestratorDag,
    /// 强制人类审批后再执行（ApprovalAndAirGap）
    ApprovalGate,
}

// ── 分类器输出 ─────────────────────────────────────────────

/// 任务形态决策结果（下游所有编排的唯一入口）。
///
/// 由 [`TaskShapeClassifier::classify`] 产出，注入到：
/// - `CapabilityDiscoveryRequest.task_shape`（能力路由前置输入）
/// - `RoutingDecisionV2.task_shape`（路由决策留痕）
/// - `AgentStartRequest.task_shape`（Agent 启动参数）
/// - 前端消息 `taskShape` 字段（决策标签展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskShapeDecision {
    /// 上下文保留成本标尺
    pub context_cost: ContextRetentionCost,
    /// 安全隔离需求标尺
    pub isolation_need: SecurityIsolationNeed,
    /// 推荐执行策略
    pub recommended_strategy: ExecutionStrategy,
    /// 合并倾向：0.0 ~ 1.0，越高越倾向合并（减少转述）
    pub merge_score: f32,
    /// 拆分倾向：0.0 ~ 1.0，越高越倾向拆分（保证精度/隔离）
    pub split_score: f32,
    /// 关键证据（审计 + 前端可展示）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

impl TaskShapeDecision {
    /// 创建一个默认的"本地处理"决策（用于 flag 关闭时的回退）
    #[must_use]
    pub fn local_default() -> Self {
        Self {
            context_cost: ContextRetentionCost::Ephemeral,
            isolation_need: SecurityIsolationNeed::ReadOnlyThought,
            recommended_strategy: ExecutionStrategy::HandleLocally,
            merge_score: 1.0,
            split_score: 0.0,
            evidence: Vec::new(),
        }
    }

    /// 根据两条标尺快速判断是否应该合并
    #[must_use]
    pub fn should_merge(&self) -> bool {
        self.merge_score >= self.split_score
    }

    /// 根据安全隔离需求推导建议的权限模式
    #[must_use]
    pub fn suggested_permission(&self) -> PermissionMode {
        match self.isolation_need {
            SecurityIsolationNeed::ReadOnlyThought => PermissionMode::ReadOnly,
            SecurityIsolationNeed::WorkspaceWrite => PermissionMode::WorkspaceWrite,
            SecurityIsolationNeed::SandboxRequired => PermissionMode::WorkspaceWrite,
            SecurityIsolationNeed::ApprovalAndAirGap => PermissionMode::Prompt,
        }
    }
}

// ── 权限覆盖映射（P2 运行时权限墙） ────────────────────────────

impl SecurityIsolationNeed {
    /// 按安全隔离需求推断「权限降级覆盖」。
    ///
    /// 返回 `None` 表示不需要覆盖（沿用会话级权限）；
    /// 返回 `Some(mode)` 表示强制使用该模式。
    ///
    /// ## 设计原则
    ///
    /// - **只降级不升级**：权限覆盖是安全防御性手段，绝不把只读提升为写入。
    ///   用户在会话级设置了 `Prompt`（最严格交互模式）时，必须保持用户配置优先。
    /// - `SandboxRequired` 及以上 → 强制 `Prompt`（逐次人机确认 = 物理权限墙）。
    #[must_use]
    pub fn to_permission_override(self) -> Option<PermissionMode> {
        match self {
            Self::ReadOnlyThought | Self::WorkspaceWrite => None,
            Self::SandboxRequired | Self::ApprovalAndAirGap => Some(PermissionMode::Prompt),
        }
    }
}

/// 解析「任务级权限」与「会话级权限」的最终生效模式。
///
/// 规则（单调函数，保证用户最严格配置不可被覆盖）：
/// 1. 若 `task_isolation` 为 None → 直接返回 session_mode（flag 关闭或分类失败回退）
/// 2. 调用 `to_permission_override()` 获取任务级建议
/// 3. 取严格级别更高的那个胜出（**不使用 PermissionMode::PartialOrd，因为其枚举值序
///    与语义严格程度序不一致**：Prompt(3) 比 DangerFullAccess(2) 更严格，但数值更大）。
///
/// ## 严格级别排序（从严格到宽松）
///
/// ```text
/// ReadOnly > Prompt > WorkspaceWrite > DangerFullAccess > Allow
/// ```
#[must_use]
pub fn resolve_effective_permission(
    session_mode: PermissionMode,
    task_isolation: Option<SecurityIsolationNeed>,
) -> PermissionMode {
    let Some(isolation) = task_isolation else { return session_mode };
    let Some(suggested) = isolation.to_permission_override() else { return session_mode };
    // 严格级别值越小越严格，用 rank 进行比较
    if strictness_rank(suggested) <= strictness_rank(session_mode) {
        suggested
    } else {
        session_mode
    }
}

/// 权限模式的「语义严格程度」排名（数字越小越严格）。
///
/// 注意：**不使用 PermissionMode enum 的 discriminant 值**，因为其派生序
/// `ReadOnly < WorkspaceWrite < DangerFullAccess < Prompt < Allow` 中
/// DangerFullAccess 和 Prompt 的相对顺序不符合「Prompt 更严格（需人机逐次确认）」
/// 的语义，导致直接用 `.min()` 会误判。
fn strictness_rank(mode: PermissionMode) -> u8 {
    match mode {
        PermissionMode::ReadOnly => 0,
        PermissionMode::Prompt => 1,
        PermissionMode::WorkspaceWrite => 2,
        PermissionMode::DangerFullAccess => 3,
        PermissionMode::Allow => 4,
    }
}

// ── 分类器 trait ────────────────────────────────────────────

/// 任务形态分类器接口。
///
/// 唯一实现方：`orchestrator::decomposer::DefaultTaskShapeClassifier`。
///
/// ## 调用时机
///
/// 在 `CognitiveRouter::route()` 的 Step 0 调用，
/// 结果注入到 `CapabilityDiscoveryRequest.task_shape`，
/// 供后续 L2-L5 管线消费。
///
/// ## Feature Flag
///
/// 当 `UNITY_P0_TASK_SHAPE = false` 时，分类器不被调用，
/// `task_shape = None`，完全不影响旧链路。
#[async_trait::async_trait]
pub trait TaskShapeClassifier: Send + Sync {
    /// 对用户输入进行任务形态分类。
    ///
    /// # 参数
    /// - `user_input`: 用户原始输入
    /// - `session`: 当前会话（提供上下文历史）
    /// - `active_permission`: 当前会话的活跃权限模式
    ///
    /// # 返回
    /// 分类决策结果，或错误字符串
    async fn classify(
        &self,
        user_input: &str,
        session: &Session,
        active_permission: PermissionMode,
    ) -> Result<TaskShapeDecision, String>;
}

// ── LLM 兜底分类器（P2 hybrid 模式） ─────────────────────────

/// LLM 任务形态分类器 — 用于规则置信度不足时的兜底判断。
///
/// 由 **wiring 层**（runtime / commands）实现，调用 providers 抽象层发起轻量 LLM 请求。
/// 保持在 harness 定义不引任何 implementor / provider 依赖（consumer-only, foundation tier）。
///
/// ## 设计约束（架构铁律）
///
/// - 此 trait 定义在 `harness`（foundation），**不可**依赖任何 `providers` / `agent` /
///   `runtime-core` crate。实际实现在 P3 的 `runtime`（wiring）crate 中创建适配器。
/// - orchestrator（hybrid consumer）仅通过 `&dyn TaskShapeLlmClassifier` trait object 调用，
///   避免 orchestrator → providers 的反向依赖（违反分层：hybrid 可按需依赖 implementor，
///   但 providers 属于 implementor，引入会让 hybrid 层变"重"，故通过 trait object 解耦）。
///
/// ## 调用契约
///
/// 输入：用户原始输入 + 规则层已产出结论（含已有证据）
/// 输出：
/// - `Ok(Some(refined))`：LLM 给出了更精确判断，下游采用 refined
/// - `Ok(None)`：LLM 无法判断或置信度仍低，回退规则结论
/// - `Err(e)`：LLM 调用失败，记录 warning，仍回退规则结论（绝不阻断主流程）
#[async_trait::async_trait]
pub trait TaskShapeLlmClassifier: Send + Sync {
    async fn refine(
        &self,
        user_input: &str,
        rule_result: &TaskShapeDecision,
    ) -> Result<Option<TaskShapeDecision>, String>;
}

// ── 测试 ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_default_favors_merge() {
        let d = TaskShapeDecision::local_default();
        assert!(d.should_merge());
        assert_eq!(d.merge_score, 1.0);
        assert_eq!(d.split_score, 0.0);
        assert_eq!(d.suggested_permission(), PermissionMode::ReadOnly);
    }

    #[test]
    fn high_isolation_suggests_prompt_mode() {
        let d = TaskShapeDecision {
            context_cost: ContextRetentionCost::Resumable,
            isolation_need: SecurityIsolationNeed::ApprovalAndAirGap,
            recommended_strategy: ExecutionStrategy::ApprovalGate,
            merge_score: 0.2,
            split_score: 0.8,
            evidence: vec!["contains: deploy".into()],
        };
        assert!(!d.should_merge());
        assert_eq!(d.suggested_permission(), PermissionMode::Prompt);
    }

    #[test]
    fn serde_roundtrip_preserves_fields() {
        let d = TaskShapeDecision {
            context_cost: ContextRetentionCost::LongRunningProject,
            isolation_need: SecurityIsolationNeed::SandboxRequired,
            recommended_strategy: ExecutionStrategy::DelegateSingleExpert {
                expert_id: "finance".into(),
            },
            merge_score: 0.3,
            split_score: 0.7,
            evidence: vec!["keyword: audit".into()],
        };
        let json = serde_json::to_string(&d).expect("序列化应成功");
        let parsed: TaskShapeDecision = serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(parsed.context_cost, ContextRetentionCost::LongRunningProject);
        assert_eq!(parsed.isolation_need, SecurityIsolationNeed::SandboxRequired);
        assert_eq!(parsed.merge_score, 0.3);
        assert_eq!(parsed.evidence, vec!["keyword: audit".to_string()]);
    }

    #[test]
    fn execution_strategy_serializes_camel_case() {
        let s = ExecutionStrategy::DelegateSingleExpert { expert_id: "legal".into() };
        let json = serde_json::to_string(&s).expect("序列化应成功");
        // enum 级 rename_all 只转变体名（externally tagged key = delegateSingleExpert）
        assert!(json.contains("delegateSingleExpert"), "json={json}");
        // variant 内部字段不继承 enum rename_all，已在字段级加 rename="expertId"
        assert!(json.contains("expertId"), "json={json}");
        // 验证输出格式：externally tagged enum 不包含 expert_id 蛇形键
        assert!(!json.contains("\"expert_id\""), "json={json}");
    }

    #[test]
    fn context_retention_cost_ord() {
        assert!(ContextRetentionCost::PersistentJourney > ContextRetentionCost::Ephemeral);
        assert!(ContextRetentionCost::LongRunningProject > ContextRetentionCost::Resumable);
    }

    #[test]
    fn security_isolation_need_ord() {
        assert!(SecurityIsolationNeed::ApprovalAndAirGap > SecurityIsolationNeed::ReadOnlyThought);
        assert!(SecurityIsolationNeed::SandboxRequired > SecurityIsolationNeed::WorkspaceWrite);
    }

    // ── P2: 权限覆盖映射测试 ──────────────────────────────────

    #[test]
    fn test_isolation_to_permission_mapping() {
        // 纯思考 → 不降级（沿用会话级）
        assert_eq!(SecurityIsolationNeed::ReadOnlyThought.to_permission_override(), None);
        // 工作区写入 → 不降级
        assert_eq!(SecurityIsolationNeed::WorkspaceWrite.to_permission_override(), None);
        // 强制沙箱 → Prompt（逐次人机确认 = 物理权限墙）
        assert_eq!(
            SecurityIsolationNeed::SandboxRequired.to_permission_override(),
            Some(PermissionMode::Prompt)
        );
        // 审批气隔 → Prompt + 后续审批门拦截（双层保险）
        assert_eq!(
            SecurityIsolationNeed::ApprovalAndAirGap.to_permission_override(),
            Some(PermissionMode::Prompt)
        );
    }

    #[test]
    fn test_resolve_effective_permission() {
        // 会话级 Allow + SandboxRequired → 降级到 Prompt（安全优先）
        assert_eq!(
            resolve_effective_permission(
                PermissionMode::Allow,
                Some(SecurityIsolationNeed::SandboxRequired)
            ),
            PermissionMode::Prompt
        );
        // 会话级 WorkspaceWrite + ReadOnlyThought → 维持 WorkspaceWrite（不升级）
        assert_eq!(
            resolve_effective_permission(
                PermissionMode::WorkspaceWrite,
                Some(SecurityIsolationNeed::ReadOnlyThought)
            ),
            PermissionMode::WorkspaceWrite
        );
        // 会话级 Prompt + 任意 → 维持 Prompt（用户主动最严格配置优先）
        assert_eq!(
            resolve_effective_permission(
                PermissionMode::Prompt,
                Some(SecurityIsolationNeed::ReadOnlyThought)
            ),
            PermissionMode::Prompt
        );
        // None（flag 关闭或分类失败）→ 维持原样
        assert_eq!(
            resolve_effective_permission(PermissionMode::WorkspaceWrite, None),
            PermissionMode::WorkspaceWrite
        );
        // DangerFullAccess + ApprovalAndAirGap → Prompt（降级 3 档）
        assert_eq!(
            resolve_effective_permission(
                PermissionMode::DangerFullAccess,
                Some(SecurityIsolationNeed::ApprovalAndAirGap)
            ),
            PermissionMode::Prompt
        );
    }
}
