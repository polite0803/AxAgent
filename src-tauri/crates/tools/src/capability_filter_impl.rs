// SPDX-License-Identifier: AGPL-3.0-only
//! 能力过滤器实现 — 8 维硬性闸门 + 可注册策略裁剪（Phase 3 策略对象化）
//!
//! 复用 harness 层 CapabilityFilter trait 的默认实现，
//! 提供可注入的结构体实例。所有 8 个维度的检查逻辑在 harness 层定义。
//!
//! # 策略对象化（Phase 3）
//! 注入 DB 连接后，`check_all` 前置执行可注册策略（capability_policies 表）的
//! 排除规则裁剪（exclude_domains / exclude_tags / exclude_capability_ids），
//! 与 8 维硬编码闸门不冲突（策略是环境性裁剪，闸门是能力自身硬约束）。
//! 未注入 DB / 无启用策略时行为与既有版本完全一致。

use async_trait::async_trait;
use axagent_harness::{
    CapabilityFilter, CapabilityPassportDto, FilterContext, FilterDecision, FilterDimension,
};
use sea_orm::DatabaseConnection;

/// 能力过滤器实现
///
/// 所有 8 个维度的检查方法均来自 harness 层默认实现，
/// 可按需 override 特定维度以实现自定义逻辑。
#[derive(Debug, Default, Clone)]
pub struct CapabilityFilterImpl {
    /// 是否启用维度二（记忆/状态）检查
    pub enable_memory_state: bool,
    /// 可选 DB 连接：注入后启用可注册策略裁剪（Phase 3 策略对象化）
    pub db: Option<DatabaseConnection>,
}

impl CapabilityFilterImpl {
    pub fn new() -> Self {
        Self { enable_memory_state: true, db: None }
    }

    pub fn with_memory_state(mut self, enabled: bool) -> Self {
        self.enable_memory_state = enabled;
        self
    }

    /// 注入 DB 连接，启用可注册策略裁剪（capability_policies 表）。
    pub fn with_db(mut self, db: DatabaseConnection) -> Self {
        self.db = Some(db);
        self
    }

    /// 策略排除规则裁剪（Phase 3 策略对象化，前置执行）。
    ///
    /// 遍历启用策略，护照命中任一排除规则（域 / 标签 / 能力 ID）即拒绝。
    /// 加载失败（DB 错误 / 规则 JSON 损坏）仅记日志放行，保证不因策略层故障阻断检索。
    async fn check_policies(&self, passport: &CapabilityPassportDto) -> FilterDecision {
        let Some(db) = &self.db else { return FilterDecision::Pass };
        let policies = match axagent_dao::repo::capability_policy::list_enabled(db).await {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!("[capability] 加载策略失败（放行）: {e}");
                return FilterDecision::Pass;
            },
        };
        if policies.is_empty() {
            return FilterDecision::Pass;
        }

        let domain_str = passport.domain.as_str();
        for policy in &policies {
            let rules = &policy.rules;
            if rules.exclude_domains.iter().any(|d| d.eq_ignore_ascii_case(domain_str)) {
                return FilterDecision::Reject {
                    reason: format!("策略「{}」排除域 {} ", policy.name, domain_str),
                    dimension: FilterDimension::Policy,
                };
            }
            if passport
                .tags
                .iter()
                .any(|t| rules.exclude_tags.iter().any(|rt| rt.eq_ignore_ascii_case(t)))
            {
                return FilterDecision::Reject {
                    reason: format!("策略「{}」排除标签", policy.name),
                    dimension: FilterDimension::Policy,
                };
            }
            if rules.exclude_capability_ids.iter().any(|id| id == &passport.capability_id) {
                return FilterDecision::Reject {
                    reason: format!("策略「{}」排除能力 {}", policy.name, passport.capability_id),
                    dimension: FilterDimension::Policy,
                };
            }
        }
        FilterDecision::Pass
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
        // 策略前置裁剪（Phase 3 策略对象化）：可注册排除规则，优先于 8 维硬闸门
        if let FilterDecision::Reject { reason, dimension } = self.check_policies(passport).await {
            return FilterDecision::Reject { reason, dimension };
        }

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
