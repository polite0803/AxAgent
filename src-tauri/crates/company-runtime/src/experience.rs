// SPDX-License-Identifier: AGPL-3.0-only

//! 经验闭环（Self-Grown）：evaluate_round → opc_experience_records →
//! 晋升 opc_playbooks → 新人继承。
//!
//! **归因铁律**（文档 §3.4）：用户反馈解析为逐员工评估，只更新拥有相关
//! 工作项的角色档案（防互相污染）。`evaluate_round` 的输入必须带
//! `work_item_id` + `role_id`，本模块按角色隔离写入。

use crate::error::{CompanyError, CompanyResult};
use axagent_opc_entities::{opc_experience_records, opc_playbooks};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

/// 经验信号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Success,
    Failure,
    Feedback,
}

impl Signal {
    pub fn as_str(&self) -> &'static str {
        match self {
            Signal::Success => "success",
            Signal::Failure => "failure",
            Signal::Feedback => "feedback",
        }
    }
}

/// 经验闭环服务。
pub struct ExperienceService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> ExperienceService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 记录一条经验（按 role_id + work_item_id 归因）。
    pub async fn record(
        &self,
        id: &str,
        role_id: &str,
        work_item_id: &str,
        signal: Signal,
        content: &str,
    ) -> CompanyResult<opc_experience_records::Model> {
        let now = chrono::Utc::now().timestamp();
        let am = opc_experience_records::ActiveModel {
            id: Set(id.to_string()),
            role_id: Set(role_id.to_string()),
            work_item_id: Set(work_item_id.to_string()),
            signal: Set(signal.as_str().to_string()),
            content: Set(content.to_string()),
            created_at: Set(now),
        };
        Ok(am.insert(self.db).await?)
    }

    /// 读取角色全部经验（新员工入职继承用）。
    pub async fn list_for_role(
        &self,
        role_id: &str,
    ) -> CompanyResult<Vec<opc_experience_records::Model>> {
        Ok(opc_experience_records::Entity::find()
            .filter(opc_experience_records::Column::RoleId.eq(role_id))
            .order_by_asc(opc_experience_records::Column::CreatedAt)
            .all(self.db)
            .await?)
    }

    /// 晋升机制：单角色经验档案达到阈值（成功条数）→ 晋升到共享 Playbook。
    /// 返回晋升的 playbook id（None = 未达阈值）。
    pub async fn promote_to_playbook(
        &self,
        role_id: &str,
        success_threshold: u64,
    ) -> CompanyResult<Option<String>> {
        let success_count = opc_experience_records::Entity::find()
            .filter(opc_experience_records::Column::RoleId.eq(role_id))
            .filter(opc_experience_records::Column::Signal.eq("success"))
            .count(self.db)
            .await?;

        if success_count < success_threshold {
            return Ok(None);
        }

        // 已有同角色 playbook → 版本升级
        let existing = opc_playbooks::Entity::find()
            .filter(opc_playbooks::Column::RoleId.eq(role_id))
            .one(self.db)
            .await?;

        // 聚合经验内容为 playbook
        let records = self.list_for_role(role_id).await?;
        let mut lines = Vec::new();
        for r in &records {
            lines.push(format!("[{}] {}", r.signal, r.content));
        }
        let content = if lines.is_empty() {
            format!("角色 {role_id} 的经验档案（已晋升）")
        } else {
            lines.join("\n")
        };
        let now = chrono::Utc::now().timestamp();

        if let Some(pb) = existing {
            let mut am: opc_playbooks::ActiveModel = pb.clone().into();
            am.content = Set(content);
            am.version = Set(pb.version + 1);
            am.updated_at = Set(now);
            am.update(self.db).await?;
            Ok(Some(pb.id))
        } else {
            let id = format!("playbook-{role_id}");
            let am = opc_playbooks::ActiveModel {
                id: Set(id.clone()),
                role_id: Set(role_id.to_string()),
                title: Set(format!("{} 经验 Playbook", role_id)),
                content: Set(content),
                promoted_from: Set(Some(format!(
                    "role-{role_id}@{}",
                    chrono::Utc::now().timestamp()
                ))),
                version: Set(1),
                created_at: Set(now),
                updated_at: Set(now),
            };
            am.insert(self.db).await?;
            Ok(Some(id))
        }
    }

    /// 读取角色 playbook（新人继承）。
    pub async fn playbook_for_role(
        &self,
        role_id: &str,
    ) -> CompanyResult<Option<opc_playbooks::Model>> {
        Ok(opc_playbooks::Entity::find()
            .filter(opc_playbooks::Column::RoleId.eq(role_id))
            .one(self.db)
            .await?)
    }

    /// 对接 trajectory ExperiencePool 的本地接口（防污染校验）。
    /// 仅当 role_id 与 work_item 的 owner_role_id 一致时允许写入。
    pub async fn validate_attribution(
        &self,
        role_id: &str,
        work_item_id: &str,
        owner_role_id: &str,
    ) -> CompanyResult<bool> {
        if role_id == owner_role_id {
            Ok(true)
        } else {
            Err(CompanyError::Other(format!(
                "归因污染拒绝: role {role_id} 不拥有 work item {work_item_id}（owner={owner_role_id}）"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn experience_attribution_and_promotion() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let svc = ExperienceService::new(db);

        // 归因铁律：非 owner 写入被拒
        assert!(svc.validate_attribution("role-cto", "wi-1", "role-cfo").await.is_err());
        assert!(svc.validate_attribution("role-cfo", "wi-1", "role-cfo").await.unwrap());

        // 记录经验
        svc.record(
            "exp-1",
            "role-cfo",
            "wi-1",
            Signal::Success,
            "发票流转优化：自动对账减少 30% 错误",
        )
        .await
        .unwrap();
        svc.record("exp-2", "role-cfo", "wi-2", Signal::Success, "现金流预测：滚动 90 天窗口更稳")
            .await
            .unwrap();

        // 未达阈值不晋升
        assert!(svc.promote_to_playbook("role-cfo", 5).await.unwrap().is_none());

        // 达阈值晋升
        for i in 0..4 {
            svc.record(
                &format!("exp-x{i}"),
                "role-cfo",
                &format!("wi-x{i}"),
                Signal::Success,
                "模板经验",
            )
            .await
            .unwrap();
        }
        let pb = svc.promote_to_playbook("role-cfo", 3).await.unwrap();
        assert!(pb.is_some(), "3 条成功应晋升");
        assert_eq!(pb.as_deref(), Some("playbook-role-cfo"));

        // 再次晋升 → 版本 +1
        let pb2 = svc.promote_to_playbook("role-cfo", 3).await.unwrap();
        assert!(pb2.is_some());
        let model = svc.playbook_for_role("role-cfo").await.unwrap().unwrap();
        assert!(model.version >= 2, "重复晋升应版本递增，实际 {}", model.version);
    }

    #[tokio::test]
    async fn experience_role_isolation() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let svc = ExperienceService::new(db);

        svc.record("e-1", "role-cfo", "wi-1", Signal::Success, "CFO 经验").await.unwrap();
        svc.record("e-2", "role-cto", "wi-2", Signal::Failure, "CTO 经验").await.unwrap();

        let cfo = svc.list_for_role("role-cfo").await.unwrap();
        assert_eq!(cfo.len(), 1, "角色档案互相隔离");
        assert_eq!(cfo[0].signal, "success");
    }
}

// ── SelfImprovingRound trait（本地定义，不进 harness）────────────

/// AI Agent 自改进循环抽象（生成 → 评估 → 改进 → 再生成）。
///
/// 方案文档 §3.4：补 Loop Engineering 缺口——Reflector 输出桥接回
/// 工作循环；Synthesizing→Finished 前有质量门。trait 定义在本地 crate
/// （company-runtime），不进 harness；基座后续若有 ExperiencePool
/// 由 trait 实现方对接。
/// 本地 crate trait，仅自用，允许 async fn in trait。
#[allow(async_fn_in_trait)]
pub trait SelfImprovingRound {
    /// 输入：一次 round 的原始输出（生成物）。
    type Output;

    /// 评估：返回质量信号 + 反思内容。
    /// `role_id` 用于归因隔离（只评估拥有该 work item 的角色）。
    async fn evaluate(
        &self,
        role_id: &str,
        work_item_id: &str,
        output: &Self::Output,
    ) -> crate::CompanyResult<RoundEvaluation>;

    /// 改进：基于评估生成改进后的输出（None = 质量达标，无需改进）。
    async fn improve(
        &self,
        output: &Self::Output,
        evaluation: &RoundEvaluation,
    ) -> crate::CompanyResult<Option<Self::Output>>;
}

/// Round 评估结果。
#[derive(Debug, Clone)]
pub struct RoundEvaluation {
    /// 质量分 0.0-1.0。
    pub quality: f64,
    /// 是否达标（≥ threshold 即通过质量门）。
    pub passed: bool,
    /// 反思内容（写入经验记录）。
    pub reflection: String,
    /// 经验信号。
    pub signal: Signal,
}

impl RoundEvaluation {
    pub fn new(quality: f64, threshold: f64, reflection: impl Into<String>) -> Self {
        let passed = quality >= threshold;
        Self {
            quality,
            passed,
            reflection: reflection.into(),
            signal: if passed {
                Signal::Success
            } else {
                Signal::Feedback
            },
        }
    }
}

/// 质量门服务：将 RoundEvaluation 落库为经验记录（防污染归因）。
pub struct QualityGateService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> QualityGateService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 质量门：评估结果 → 经验记录（仅当 role_id 拥有 work_item 时写入）。
    /// 返回是否通过质量门。
    pub async fn apply(
        &self,
        role_id: &str,
        work_item_id: &str,
        owner_role_id: &str,
        evaluation: &RoundEvaluation,
    ) -> crate::CompanyResult<bool> {
        let exp = ExperienceService::new(self.db);
        // 归因铁律校验
        exp.validate_attribution(role_id, work_item_id, owner_role_id).await?;
        // 落库经验
        exp.record(
            &format!("gate-{work_item_id}-{}", chrono::Utc::now().timestamp()),
            role_id,
            work_item_id,
            evaluation.signal,
            &evaluation.reflection,
        )
        .await?;
        Ok(evaluation.passed)
    }
}

#[cfg(test)]
mod quality_gate_tests {
    use super::*;

    #[tokio::test]
    async fn quality_gate_attribution_and_pass() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let gate = QualityGateService::new(db);

        // 归因正确 + 达标 → 通过 + 落库经验
        let ev = RoundEvaluation::new(0.9, 0.7, "分析结构清晰，结论可执行");
        let passed = gate.apply("role-cfo", "wi-1", "role-cfo", &ev).await.unwrap();
        assert!(passed);

        // 归因污染 → 拒绝
        let ev2 = RoundEvaluation::new(0.5, 0.7, "CTO 尝试写入 CFO 的经验");
        assert!(gate.apply("role-cto", "wi-1", "role-cfo", &ev2).await.is_err());

        // 未达标 → 不通过但记录 feedback
        let ev3 = RoundEvaluation::new(0.4, 0.7, "缺数据支撑");
        let passed3 = gate.apply("role-cfo", "wi-2", "role-cfo", &ev3).await.unwrap();
        assert!(!passed3);

        // 经验记录已写入（晋升阈值用）
        let exp = ExperienceService::new(db);
        let records = exp.list_for_role("role-cfo").await.unwrap();
        assert_eq!(records.len(), 2);
    }

    #[tokio::test]
    async fn quality_gate_signal_mapping() {
        let ok = RoundEvaluation::new(0.9, 0.7, "达标");
        assert_eq!(ok.signal, Signal::Success);
        assert!(ok.passed);
        let bad = RoundEvaluation::new(0.3, 0.7, "不达标");
        assert_eq!(bad.signal, Signal::Feedback);
        assert!(!bad.passed);
    }
}
