// SPDX-License-Identifier: AGPL-3.0-only
//! 能力过滤器实现 — 8 维硬性闸门
//!
//! 复用 harness 层 CapabilityFilter trait 的默认实现，
//! 提供可注入的结构体实例。所有 8 个维度的检查逻辑在 harness 层定义。

use async_trait::async_trait;
use axagent_harness::{CapabilityFilter, CapabilityPassportDto, FilterContext, FilterDecision};

/// 能力过滤器实现
///
/// 所有 8 个维度的检查方法均来自 harness 层默认实现，
/// 可按需 override 特定维度以实现自定义逻辑。
#[derive(Debug, Default, Clone)]
pub struct CapabilityFilterImpl {
    /// 是否启用维度二（记忆/状态）检查
    pub enable_memory_state: bool,
}

impl CapabilityFilterImpl {
    pub fn new() -> Self {
        Self { enable_memory_state: true }
    }

    pub fn with_memory_state(mut self, enabled: bool) -> Self {
        self.enable_memory_state = enabled;
        self
    }
}

#[async_trait]
impl CapabilityFilter for CapabilityFilterImpl {
    /// 维度一：置信度检查
    ///
    /// 此维度在 Ranker 阶段通过分差检测实现，
    /// 过滤阶段默认通过（仅做硬闸门检查）
    async fn check_all(
        &self,
        passport: &CapabilityPassportDto,
        ctx: &FilterContext,
    ) -> FilterDecision {
        // 执行所有硬闸门维度检查
        // 维度三：模态
        if let FilterDecision::Reject { reason, dimension } =
            self.check_modality(passport, ctx).await
        {
            return FilterDecision::Reject { reason, dimension };
        }

        // 维度四：安全/合规
        if let FilterDecision::Reject { reason, dimension } =
            self.check_security(passport, ctx).await
        {
            return FilterDecision::Reject { reason, dimension };
        }

        // 维度五：资源/成本
        if let FilterDecision::Reject { reason, dimension } =
            self.check_resource_cost(passport, ctx).await
        {
            return FilterDecision::Reject { reason, dimension };
        }

        // 维度六：交互策略
        if let FilterDecision::Reject { reason, dimension } =
            self.check_interaction(passport, ctx).await
        {
            return FilterDecision::Reject { reason, dimension };
        }

        // 维度七：规划复杂度
        if let FilterDecision::Reject { reason, dimension } =
            self.check_planning_complexity(passport, ctx).await
        {
            return FilterDecision::Reject { reason, dimension };
        }

        // 维度八：实验/灰度
        if let FilterDecision::Reject { reason, dimension } =
            self.check_experiment_group(passport, ctx).await
        {
            return FilterDecision::Reject { reason, dimension };
        }

        FilterDecision::Pass
    }
}
