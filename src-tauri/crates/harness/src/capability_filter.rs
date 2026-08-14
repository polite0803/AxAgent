// SPDX-License-Identifier: AGPL-3.0-only
//! 8维过滤闸门 — 硬性剔除不合规能力
//!
//! 每个维度实现为独立方法，任何一个维度返回 Reject 则直接出局。

use crate::capability::{
    CapabilityPassportDto, InputModality, PlanningComplexity, SecurityLevel, SessionBudget,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 过滤上下文 ────────────────────────────────────

/// 能力过滤上下文（包含所有用于过滤的运行时信息）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilterContext {
    /// 用户输入模态（从 L1 多模态预处理器提取）
    #[serde(default)]
    pub input_modalities: Vec<InputModality>,
    /// 检测到的 PII 类型列表
    #[serde(default)]
    pub detected_pii_types: Vec<PiiType>,
    /// 当前会话预算
    #[serde(default)]
    pub session_budget: SessionBudget,
    /// 设备类型
    #[serde(default)]
    pub device_type: OutputDeviceType,
    /// 任务复杂度标记（None = 自动推断）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_planning_level: Option<TaskPlanningLevel>,
    /// 当前用户 ID（用于个性化过滤）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// 用户历史使用过的能力 ID 列表（用于维度二：记忆/状态）
    #[serde(default)]
    pub user_history_ids: Vec<String>,
    /// 实验分组（None = 不区分组）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment_group: Option<String>,
}

// ── 辅助类型 ──────────────────────────────────────

/// 检测到的 PII 类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiType {
    IdCard,
    PhoneNumber,
    Email,
    BankCard,
    Address,
    Other,
}

/// 输出设备类型（用于维度六：交互策略过滤）
///
/// 注意：与 device_sync::DeviceType 命名不同，
/// 本类型专注于输出能力判定（屏幕/无屏）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputDeviceType {
    #[default]
    Desktop,
    Laptop,
    Tablet,
    Phone,
    /// 智能音箱（无屏）
    SmartSpeaker,
    /// 车载系统
    Car,
    Other,
}

impl OutputDeviceType {
    /// 是否支持富文本输出（表格/图表等）
    pub fn supports_rich_output(&self) -> bool {
        matches!(
            self,
            OutputDeviceType::Desktop | OutputDeviceType::Laptop | OutputDeviceType::Tablet
        )
    }

    /// 是否支持可视化输出
    pub fn supports_visualization(&self) -> bool {
        matches!(
            self,
            OutputDeviceType::Desktop
                | OutputDeviceType::Laptop
                | OutputDeviceType::Tablet
                | OutputDeviceType::Phone
        )
    }
}

/// 任务规划级别（用于维度七：规划复杂度过滤）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPlanningLevel {
    /// 简单任务：单步即可完成
    Simple,
    /// 普通任务：需要几步但无需循环
    Moderate,
    /// 复杂任务：需要多步循环调用
    Complex,
}

// ── 过滤结果 ──────────────────────────────────────

/// 单个维度的过滤决策
#[derive(Debug, Clone, PartialEq)]
pub enum FilterDecision {
    /// 通过此维度
    Pass,
    /// 被此维度拒绝
    Reject { reason: String, dimension: FilterDimension },
    /// 需要更多信息（不硬性拒绝，但降低优先级）
    NeedsMoreInfo { reason: String },
}

/// 过滤维度标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterDimension {
    /// 维度零：可见性（元能力隔离第一闸门，SystemOnly/Hidden 直接剔除）
    Visibility,
    /// 维度一：置信度（Top1 vs Top2 分差）
    Confidence,
    /// 维度二：记忆/状态（历史使用提权）
    MemoryState,
    /// 维度三：模态（多模态支持）
    Modality,
    /// 维度四：安全/合规（PII 检测）
    Security,
    /// 维度五：资源/成本（预算检查）
    ResourceCost,
    /// 维度六：交互策略（设备能力）
    Interaction,
    /// 维度七：规划复杂度（任务匹配）
    PlanningComplexity,
    /// 维度八：实验/灰度（分组过滤）
    ExperimentGroup,
}

impl FilterDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterDimension::Visibility => "visibility",
            FilterDimension::Confidence => "confidence",
            FilterDimension::MemoryState => "memory_state",
            FilterDimension::Modality => "modality",
            FilterDimension::Security => "security",
            FilterDimension::ResourceCost => "resource_cost",
            FilterDimension::Interaction => "interaction",
            FilterDimension::PlanningComplexity => "planning_complexity",
            FilterDimension::ExperimentGroup => "experiment_group",
        }
    }
}

/// 过滤后的候选列表
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilteredCandidates {
    /// 通过所有闸门的候选
    pub passed: Vec<crate::capability::CapabilityPassportDto>,
    /// 被拒绝的候选及原因
    pub rejected: Vec<RejectedCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedCandidate {
    pub capability_id: String,
    pub rejected_by: FilterDimension,
    pub reason: String,
}

// ── 过滤器 trait ──────────────────────────────────

/// 能力过滤器 — 9维硬性闸门（第0维 visibility 为元能力隔离核心）
///
/// 每个维度实现为独立方法，任何一个维度返回 Reject 则直接出局。
/// 过滤器可组合（支持管道式过滤）。
#[async_trait]
pub trait CapabilityFilter: Send + Sync {
    /// 对单个能力执行全部 9 个维度的检查
    ///
    /// 默认实现：先检查 visibility（元能力隔离第一闸门），再依次检查其他维度。
    /// 实现方可覆盖以自定义检查顺序。
    async fn check_all(
        &self,
        passport: &CapabilityPassportDto,
        ctx: &FilterContext,
    ) -> FilterDecision {
        // 维度零：可见性检查（SystemOnly/Hidden 直接剔除）
        match self.check_visibility(passport, ctx).await {
            FilterDecision::Pass => {},
            decision => return decision,
        }

        // 维度三：模态检查
        match self.check_modality(passport, ctx).await {
            FilterDecision::Pass => {},
            decision => return decision,
        }

        // 维度四：安全/合规检查
        match self.check_security(passport, ctx).await {
            FilterDecision::Pass => {},
            decision => return decision,
        }

        // 维度五：资源/成本检查
        match self.check_resource_cost(passport, ctx).await {
            FilterDecision::Pass => {},
            decision => return decision,
        }

        // 维度六：交互策略检查
        match self.check_interaction(passport, ctx).await {
            FilterDecision::Pass => {},
            decision => return decision,
        }

        // 维度七：规划复杂度检查
        match self.check_planning_complexity(passport, ctx).await {
            FilterDecision::Pass => {},
            decision => return decision,
        }

        // 维度八：实验/灰度检查
        match self.check_experiment_group(passport, ctx).await {
            FilterDecision::Pass => {},
            decision => return decision,
        }

        FilterDecision::Pass
    }

    /// 对候选列表批量过滤
    async fn filter_candidates(
        &self,
        candidates: &[CapabilityPassportDto],
        ctx: &FilterContext,
    ) -> FilteredCandidates {
        let mut passed = Vec::new();
        let mut rejected = Vec::new();

        for candidate in candidates {
            match self.check_all(candidate, ctx).await {
                FilterDecision::Pass => passed.push(candidate.clone()),
                FilterDecision::Reject { reason, dimension } => {
                    rejected.push(RejectedCandidate {
                        capability_id: candidate.capability_id.clone(),
                        rejected_by: dimension,
                        reason,
                    });
                },
                FilterDecision::NeedsMoreInfo { .. } => passed.push(candidate.clone()),
            }
        }

        FilteredCandidates { passed, rejected }
    }

    // ── 各维度独立检查（可单独覆盖）────────────────

    /// 维度零：可见性检查（元能力隔离第一闸门）
    ///
    /// SystemOnly 或 Hidden 能力直接剔除，不参与任何相似度计算。
    async fn check_visibility(
        &self,
        passport: &CapabilityPassportDto,
        _ctx: &FilterContext,
    ) -> FilterDecision {
        // 🔒 元能力隔离核心：SystemOnly 直接剔除
        if passport.visibility.is_system_only() {
            return FilterDecision::Reject {
                reason: format!(
                    "能力 {} 为系统专用（SystemOnly），不可被用户发现",
                    passport.capability_id
                ),
                dimension: FilterDimension::Visibility,
            };
        }

        // 🔒 Hidden 能力直接剔除
        if matches!(passport.visibility, crate::capability::Visibility::Hidden) {
            return FilterDecision::Reject {
                reason: format!("能力 {} 已标记为隐藏（Hidden）", passport.capability_id),
                dimension: FilterDimension::Visibility,
            };
        }

        // 🔒 System 域能力直接剔除（双重保险）
        if passport.domain.is_system() {
            return FilterDecision::Reject {
                reason: format!(
                    "能力 {} 属于系统域（System），不可被用户发现",
                    passport.capability_id
                ),
                dimension: FilterDimension::Visibility,
            };
        }

        FilterDecision::Pass
    }

    /// 维度三：模态检查
    async fn check_modality(
        &self,
        passport: &CapabilityPassportDto,
        ctx: &FilterContext,
    ) -> FilterDecision {
        if ctx.input_modalities.is_empty() {
            return FilterDecision::Pass;
        }
        let unsupported: Vec<&InputModality> = ctx
            .input_modalities
            .iter()
            .filter(|m| !passport.modality_support.supports(m))
            .collect();
        if unsupported.is_empty() {
            FilterDecision::Pass
        } else {
            FilterDecision::Reject {
                reason: format!(
                    "不支持的输入模态: {:?}",
                    unsupported.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>()
                ),
                dimension: FilterDimension::Modality,
            }
        }
    }

    /// 维度四：安全/合规检查
    async fn check_security(
        &self,
        passport: &CapabilityPassportDto,
        ctx: &FilterContext,
    ) -> FilterDecision {
        if ctx.detected_pii_types.is_empty() {
            return FilterDecision::Pass;
        }
        if passport.security_level >= SecurityLevel::Sensitive
            && (!passport.security_level.requires_encrypted_transmission()
                || !passport.security_level.requires_audit_log())
        {
            return FilterDecision::Reject {
                reason: "检测到 PII，但能力未加密传输或缺少审计日志".to_string(),
                dimension: FilterDimension::Security,
            };
        }
        FilterDecision::Pass
    }

    /// 维度五：资源/成本检查
    async fn check_resource_cost(
        &self,
        passport: &CapabilityPassportDto,
        ctx: &FilterContext,
    ) -> FilterDecision {
        if let Some(cost) = passport.estimated_cost_usd
            && !ctx.session_budget.can_afford(cost)
        {
            return FilterDecision::Reject {
                reason: format!(
                    "预估成本 ${:.4} 超出单次预算上限 ${:.4}",
                    cost, ctx.session_budget.max_per_call_usd
                ),
                dimension: FilterDimension::ResourceCost,
            };
        }
        FilterDecision::Pass
    }

    /// 维度六：交互策略检查
    async fn check_interaction(
        &self,
        passport: &CapabilityPassportDto,
        ctx: &FilterContext,
    ) -> FilterDecision {
        if !ctx.device_type.supports_rich_output()
            && (passport.output_capabilities.supports_table
                || passport.output_capabilities.supports_chart)
        {
            return FilterDecision::Reject {
                reason: format!("设备 {:?} 不支持表格/图表输出", ctx.device_type),
                dimension: FilterDimension::Interaction,
            };
        }
        FilterDecision::Pass
    }

    /// 维度七：规划复杂度检查
    async fn check_planning_complexity(
        &self,
        passport: &CapabilityPassportDto,
        ctx: &FilterContext,
    ) -> FilterDecision {
        if let Some(TaskPlanningLevel::Simple) = ctx.task_planning_level
            && passport.planning_complexity == PlanningComplexity::Complex
        {
            return FilterDecision::Reject {
                reason: "简单任务不需要复杂工作流".to_string(),
                dimension: FilterDimension::PlanningComplexity,
            };
        }
        FilterDecision::Pass
    }

    /// 维度八：实验/灰度检查
    async fn check_experiment_group(
        &self,
        passport: &CapabilityPassportDto,
        ctx: &FilterContext,
    ) -> FilterDecision {
        if let Some(ref required_group) = ctx.experiment_group
            && let Some(ref capability_group) = passport.experiment_group
            && capability_group != required_group
        {
            return FilterDecision::Reject {
                reason: format!(
                    "当前用户在 {} 组，此能力仅对 {} 组可见",
                    required_group, capability_group
                ),
                dimension: FilterDimension::ExperimentGroup,
            };
        }
        FilterDecision::Pass
    }
}
