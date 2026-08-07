// SPDX-License-Identifier: AGPL-3.0-only

//! 招聘决策（Recruiter，Self-Built 机制）。
//!
//! 规则链（文档 §3.3）：
//! 1. **复用已有员工**：同角色存在活跃员工（携带历史经验）→ 直接指派；
//! 2. **人才库招聘**：`opc_talent_templates` 有匹配模板 → 新建员工（绑定 expert）；
//! 3. **降级 fallback 空员工**：都无 → 用占位空员工（不阻塞执行）。
//!
//! 决策输出供 WorkItem 认领时选择 assignee。

use crate::error::{CompanyError, CompanyResult};
use crate::org::OrgService;
use axagent_entities::{opc_org_employees, opc_talent_templates};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

/// 招聘决策结果。
#[derive(Debug, Clone)]
pub struct HireDecision {
    pub employee_id: String,
    pub role_id: String,
    pub expert_id: Option<String>,
    /// 决策来源：reuse / hire / fallback
    pub source: &'static str,
}

/// 招聘服务：为角色决策指派员工。
pub struct HiringService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> HiringService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 核心决策：为 org + role 找一个可用员工。
    ///
    /// 优先级：活跃员工（复用）→ 人才库新建（hire）→ 占位空员工（fallback）。
    pub async fn decide(&self, org_id: &str, role_id: &str) -> CompanyResult<HireDecision> {
        let org_svc = OrgService::new(self.db);

        // 1. 复用：同角色活跃员工
        let active = org_svc.active_employees_for_role(org_id, role_id).await?;
        if let Some(emp) = active.into_iter().next() {
            return Ok(HireDecision {
                employee_id: emp.employee_id,
                role_id: emp.role_id,
                expert_id: emp.expert_id,
                source: "reuse",
            });
        }

        // 2. 人才库招聘：按角色拆词匹配模板（role_id 的每个 token 都可能命中）
        let tokens: Vec<String> =
            role_id.split('-').filter(|t| !t.is_empty()).map(|t| t.to_string()).collect();
        let templates = self.find_templates_for_keywords(&tokens).await?;
        if let Some(tt) = templates.into_iter().next() {
            let emp_id = format!("hired-{}-{}", tt.id, chrono::Utc::now().timestamp());
            // 新建员工绑定模板专家（source_repo 即 expert 来源）
            org_svc
                .add_employee(crate::org::NewEmployee {
                    id: emp_id.clone(),
                    org_id: org_id.to_string(),
                    employee_id: emp_id.clone(),
                    role_id: role_id.to_string(),
                    expert_id: Some(tt.id.clone()),
                    status: "active".to_string(),
                    experience_ref: None,
                })
                .await?;
            return Ok(HireDecision {
                employee_id: emp_id,
                role_id: role_id.to_string(),
                expert_id: Some(tt.id),
                source: "hire",
            });
        }

        // 3. 降级：占位空员工（不阻塞执行）
        // P2-9：占位员工必须落库（此前只返回 id 不落库，调用方按 employee 关联必然 NotFound）
        let emp_id = format!("placeholder-{role_id}");
        org_svc
            .add_employee(crate::org::NewEmployee {
                id: emp_id.clone(),
                org_id: org_id.to_string(),
                employee_id: emp_id.clone(),
                role_id: role_id.to_string(),
                expert_id: None,
                status: "placeholder".to_string(),
                experience_ref: None,
            })
            .await?;
        Ok(HireDecision {
            employee_id: emp_id,
            role_id: role_id.to_string(),
            expert_id: None,
            source: "fallback",
        })
    }

    /// 按关键字列表匹配人才模板（name/category/tags 任一包含任一 token）。
    async fn find_templates_for_keywords(
        &self,
        keywords: &[String],
    ) -> CompanyResult<Vec<opc_talent_templates::Model>> {
        use sea_orm::Condition;
        let mut cond = Condition::any();
        for kw in keywords {
            let like = format!("%{kw}%");
            cond = cond
                .add(opc_talent_templates::Column::Name.like(&like))
                .add(opc_talent_templates::Column::Category.like(&like))
                .add(opc_talent_templates::Column::Tags.like(&like));
        }
        Ok(opc_talent_templates::Entity::find().filter(cond).all(self.db).await?)
    }

    /// 清空占位员工标记（招聘成功后调用，将占位升级为真实员工）。
    pub async fn promote_placeholder(
        &self,
        org_id: &str,
        role_id: &str,
        employee_id: &str,
    ) -> CompanyResult<()> {
        // 校验占位存在
        let placeholder = format!("placeholder-{role_id}");
        if employee_id != placeholder {
            return Err(CompanyError::Invalid(format!("{employee_id} 不是占位员工，无需 promote")));
        }
        // P2-6：真正执行升级——占位记录 status: placeholder → active
        //（此前 `let _ = emp;` 空操作，占位永远无法转正）
        let emp = opc_org_employees::Entity::find()
            .filter(opc_org_employees::Column::OrgId.eq(org_id))
            .filter(opc_org_employees::Column::EmployeeId.eq(employee_id))
            .one(self.db)
            .await?
            .ok_or_else(|| CompanyError::NotFound(format!("employee {employee_id}")))?;
        use sea_orm::{ActiveModelTrait, Set};
        let mut am: opc_org_employees::ActiveModel = emp.into();
        am.status = Set("active".to_string());
        am.update(self.db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org::OrgService;

    #[tokio::test]
    async fn hiring_fallback_and_hire() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let org_svc = OrgService::new(db);
        let hiring = HiringService::new(db);

        // 空组织：无员工、无模板 → fallback 占位
        org_svc.create_org("org-h", "测试组织", "测试", "flat", None).await.unwrap();
        let d = hiring.decide("org-h", "opc-cfo-cfo-financial-analyst").await.unwrap();
        assert_eq!(d.source, "fallback");
        assert!(d.employee_id.starts_with("placeholder-"));

        // 添加人才模板 → hire（tags 含 cfo，命中角色拆词 token）
        org_svc
            .add_talent_template(crate::org::NewTalentTemplate {
                id: "tt-fin".to_string(),
                category: "finance".to_string(),
                name: "金融分析师".to_string(),
                description: "财务报表分析".to_string(),
                source_repo: "agency-agents-src".to_string(),
                prompt_refs: None,
                skill_refs: None,
                tags: Some(vec!["cfo".to_string(), "finance".to_string()]),
            })
            .await
            .unwrap();
        let d2 = hiring.decide("org-h", "opc-cfo-cfo-financial-analyst").await.unwrap();
        assert_eq!(d2.source, "hire", "有人才模板应走招聘");
        assert_eq!(d2.expert_id.as_deref(), Some("tt-fin"));
    }

    #[tokio::test]
    async fn hiring_reuse_active_employee() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let org_svc = OrgService::new(db);
        let hiring = HiringService::new(db);

        org_svc.create_org("org-r", "复用组织", "测试", "flat", None).await.unwrap();
        // 已有活跃员工（专家绑定）→ 复用优先于 hire
        org_svc
            .add_employee(crate::org::NewEmployee {
                id: "emp-exist".to_string(),
                org_id: "org-r".to_string(),
                employee_id: "emp-exist".to_string(),
                role_id: "opc-cfo-cfo-financial-analyst".to_string(),
                expert_id: Some("opc-cfo".to_string()),
                status: "active".to_string(),
                experience_ref: None,
            })
            .await
            .unwrap();

        let d = hiring.decide("org-r", "opc-cfo-cfo-financial-analyst").await.unwrap();
        assert_eq!(d.source, "reuse");
        assert_eq!(d.employee_id, "emp-exist");
    }
}
