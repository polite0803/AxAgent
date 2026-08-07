// SPDX-License-Identifier: AGPL-3.0-only

//! 组织抽象（Self-Built）：opc_orgs / opc_org_roles / opc_org_employees /
//! opc_talent_templates 的 CRUD 与默认组织初始化。
//!
//! 映射关系：org_roles.role_id ↔ agent_roles / opc-xxx 专家 id；
//! org_employees.expert_id ↔ agency_experts / opc-xxx。

use crate::error::CompanyResult;
use axagent_entities::{opc_org_employees, opc_org_roles, opc_orgs, opc_talent_templates};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

/// 新增组织角色的输入。
#[derive(Debug, Clone)]
pub struct NewRole {
    pub id: String,
    pub org_id: String,
    pub role_id: String,
    pub name: String,
    pub responsibility: String,
    pub reports_to: Option<String>,
    pub seniority: String,
}

/// 新增组织员工的输入。
#[derive(Debug, Clone)]
pub struct NewEmployee {
    pub id: String,
    pub org_id: String,
    pub employee_id: String,
    pub role_id: String,
    pub expert_id: Option<String>,
    pub status: String,
    pub experience_ref: Option<String>,
}

/// 新增人才模板的输入。
#[derive(Debug, Clone)]
pub struct NewTalentTemplate {
    pub id: String,
    pub category: String,
    pub name: String,
    pub description: String,
    pub source_repo: String,
    pub prompt_refs: Option<Vec<String>>,
    pub skill_refs: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

/// 组织服务：CRUD + 默认一人公司组织初始化。
pub struct OrgService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> OrgService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    // ── 组织 ────────────────────────────────────────────────────

    pub async fn create_org(
        &self,
        id: &str,
        name: &str,
        profile: &str,
        topology: &str,
        final_decider_role_id: Option<&str>,
    ) -> CompanyResult<opc_orgs::Model> {
        let now = chrono::Utc::now().timestamp();
        let am = opc_orgs::ActiveModel {
            id: Set(id.to_string()),
            name: Set(name.to_string()),
            company_profile: Set(profile.to_string()),
            topology: Set(topology.to_string()),
            final_decider_role_id: Set(final_decider_role_id.map(|s| s.to_string())),
            created_at: Set(now),
            updated_at: Set(now),
        };
        Ok(am.insert(self.db).await?)
    }

    pub async fn get_org(&self, id: &str) -> CompanyResult<opc_orgs::Model> {
        opc_orgs::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .ok_or_else(|| crate::CompanyError::NotFound(format!("org {id}")))
    }

    pub async fn list_orgs(&self) -> CompanyResult<Vec<opc_orgs::Model>> {
        Ok(opc_orgs::Entity::find().all(self.db).await?)
    }

    pub async fn delete_org(&self, id: &str) -> CompanyResult<()> {
        let org = self.get_org(id).await?;
        opc_orgs::Entity::delete_by_id(org.id).exec(self.db).await?;
        // 级联清理角色与员工
        opc_org_roles::Entity::delete_many()
            .filter(opc_org_roles::Column::OrgId.eq(id))
            .exec(self.db)
            .await?;
        opc_org_employees::Entity::delete_many()
            .filter(opc_org_employees::Column::OrgId.eq(id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    // ── 组织角色 ────────────────────────────────────────────────

    pub async fn add_role(&self, input: NewRole) -> CompanyResult<opc_org_roles::Model> {
        let now = chrono::Utc::now().timestamp();
        let am = opc_org_roles::ActiveModel {
            id: Set(input.id),
            org_id: Set(input.org_id),
            role_id: Set(input.role_id),
            name: Set(input.name),
            responsibility: Set(input.responsibility),
            reports_to: Set(input.reports_to),
            seniority: Set(input.seniority),
            created_at: Set(now),
            updated_at: Set(now),
        };
        Ok(am.insert(self.db).await?)
    }

    pub async fn list_roles(&self, org_id: &str) -> CompanyResult<Vec<opc_org_roles::Model>> {
        Ok(opc_org_roles::Entity::find()
            .filter(opc_org_roles::Column::OrgId.eq(org_id))
            .all(self.db)
            .await?)
    }

    /// 按 role_id 查角色（跨组织，供 WorkItem 归属解析）。
    pub async fn find_role_by_role_id(
        &self,
        role_id: &str,
    ) -> CompanyResult<Option<opc_org_roles::Model>> {
        Ok(opc_org_roles::Entity::find()
            .filter(opc_org_roles::Column::RoleId.eq(role_id))
            .one(self.db)
            .await?)
    }

    pub async fn delete_role(&self, id: &str) -> CompanyResult<()> {
        opc_org_roles::Entity::delete_by_id(id).exec(self.db).await?;
        Ok(())
    }

    // ── 员工 ────────────────────────────────────────────────────

    pub async fn add_employee(
        &self,
        input: NewEmployee,
    ) -> CompanyResult<opc_org_employees::Model> {
        let now = chrono::Utc::now().timestamp();
        let am = opc_org_employees::ActiveModel {
            id: Set(input.id),
            org_id: Set(input.org_id),
            employee_id: Set(input.employee_id),
            role_id: Set(input.role_id),
            expert_id: Set(input.expert_id),
            status: Set(input.status),
            experience_ref: Set(input.experience_ref),
            created_at: Set(now),
            updated_at: Set(now),
        };
        Ok(am.insert(self.db).await?)
    }

    pub async fn list_employees(
        &self,
        org_id: &str,
    ) -> CompanyResult<Vec<opc_org_employees::Model>> {
        Ok(opc_org_employees::Entity::find()
            .filter(opc_org_employees::Column::OrgId.eq(org_id))
            .all(self.db)
            .await?)
    }

    /// 按角色查员工（活跃）。
    pub async fn active_employees_for_role(
        &self,
        org_id: &str,
        role_id: &str,
    ) -> CompanyResult<Vec<opc_org_employees::Model>> {
        Ok(opc_org_employees::Entity::find()
            .filter(opc_org_employees::Column::OrgId.eq(org_id))
            .filter(opc_org_employees::Column::RoleId.eq(role_id))
            .filter(opc_org_employees::Column::Status.eq("active"))
            .all(self.db)
            .await?)
    }

    pub async fn update_employee_status(
        &self,
        id: &str,
        status: &str,
    ) -> CompanyResult<opc_org_employees::Model> {
        let emp = opc_org_employees::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .ok_or_else(|| crate::CompanyError::NotFound(format!("employee {id}")))?;
        let mut am: opc_org_employees::ActiveModel = emp.into();
        am.status = Set(status.to_string());
        am.updated_at = Set(chrono::Utc::now().timestamp());
        Ok(am.update(self.db).await?)
    }

    pub async fn delete_employee(&self, id: &str) -> CompanyResult<()> {
        opc_org_employees::Entity::delete_by_id(id).exec(self.db).await?;
        Ok(())
    }

    // ── 人才模板 ────────────────────────────────────────────────

    pub async fn add_talent_template(
        &self,
        input: NewTalentTemplate,
    ) -> CompanyResult<opc_talent_templates::Model> {
        let now = chrono::Utc::now().timestamp();
        let am = opc_talent_templates::ActiveModel {
            id: Set(input.id),
            category: Set(input.category),
            name: Set(input.name),
            description: Set(input.description),
            source_repo: Set(input.source_repo),
            prompt_refs: Set(input
                .prompt_refs
                .map(|v| serde_json::to_string(&v).unwrap_or_default())),
            skill_refs: Set(input
                .skill_refs
                .map(|v| serde_json::to_string(&v).unwrap_or_default())),
            tags: Set(input.tags.map(|v| serde_json::to_string(&v).unwrap_or_default())),
            created_at: Set(now),
            updated_at: Set(now),
        };
        Ok(am.insert(self.db).await?)
    }

    pub async fn list_talent_templates(
        &self,
        category: Option<&str>,
    ) -> CompanyResult<Vec<opc_talent_templates::Model>> {
        let mut q = opc_talent_templates::Entity::find();
        if let Some(c) = category {
            q = q.filter(opc_talent_templates::Column::Category.eq(c));
        }
        Ok(q.all(self.db).await?)
    }

    // ── 默认一人公司组织 ────────────────────────────────────────

    /// 创建默认「一人公司」组织：6 角色 + final decider = CEO。
    /// 幂等：org 已存在则跳过。
    pub async fn ensure_default_org(&self, org_id: &str) -> CompanyResult<opc_orgs::Model> {
        if let Ok(org) = self.get_org(org_id).await {
            return Ok(org);
        }
        let org = self
            .create_org(
                org_id,
                "一人公司",
                "AI 驱动的一人公司：CEO 决策，CTO/CFO/COO/CMO/CPO 分工执行",
                "flat",
                Some("opc-ceo-ceo-business-strategist"),
            )
            .await?;

        // 6 角色（映射 opc-xxx 专家）
        let roles: &[(&str, &str, &str, Option<&str>, &str)] = &[
            (
                "role-ceo",
                "opc-ceo-ceo-business-strategist",
                "CEO/创始人",
                Some("opc-ceo-ceo-business-strategist"),
                "lead",
            ),
            (
                "role-cto",
                "opc-cto-cto-ai-engineer",
                "CTO/技术负责人",
                Some("opc-cto-cto-ai-engineer"),
                "senior",
            ),
            (
                "role-cfo",
                "opc-cfo-cfo-financial-analyst",
                "CFO/财务负责人",
                Some("opc-cfo-cfo-financial-analyst"),
                "senior",
            ),
            (
                "role-coo",
                "opc-coo-coo-operations-manager",
                "COO/运营负责人",
                Some("opc-coo-coo-operations-manager"),
                "senior",
            ),
            (
                "role-cmo",
                "opc-cmo-cmo-content-strategist",
                "CMO/增长负责人",
                Some("opc-cmo-cmo-content-strategist"),
                "senior",
            ),
            (
                "role-cpo",
                "opc-cpo-cpo-product-manager",
                "CPO/产品负责人",
                Some("opc-cpo-cpo-product-manager"),
                "senior",
            ),
        ];
        for (rid, role_id, name, expert, seniority) in roles {
            self.add_role(NewRole {
                id: rid.to_string(),
                org_id: org_id.to_string(),
                role_id: role_id.to_string(),
                name: name.to_string(),
                responsibility: format!("{name} 职责（由 OPC 角色专家承担）"),
                reports_to: if *rid == "role-ceo" {
                    None
                } else {
                    Some("role-ceo".to_string())
                },
                seniority: seniority.to_string(),
            })
            .await?;
            // 每个角色一名占位员工（降级 fallback 空员工，招聘后可替换）
            self.add_employee(NewEmployee {
                id: format!("emp-{rid}"),
                org_id: org_id.to_string(),
                employee_id: format!("emp-{rid}"),
                role_id: role_id.to_string(),
                expert_id: expert.map(|s| s.to_string()),
                status: "active".to_string(),
                experience_ref: None,
            })
            .await?;
        }
        Ok(org)
    }

    /// 删除员工（级联清理 experience_ref 指向由 experience 模块处理）。
    pub async fn remove_employee(&self, id: &str) -> CompanyResult<()> {
        self.delete_employee(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn org_crud_and_default_org() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let svc = OrgService::new(db);

        // 默认组织：幂等
        let org = svc.ensure_default_org("org-main").await.unwrap();
        assert_eq!(org.name, "一人公司");
        let org2 = svc.ensure_default_org("org-main").await.unwrap();
        assert_eq!(org.id, org2.id);

        // 6 角色
        let roles = svc.list_roles("org-main").await.unwrap();
        assert_eq!(roles.len(), 6, "应有 6 个组织角色");

        // 6 员工（每个角色 1 名占位）
        let emps = svc.list_employees("org-main").await.unwrap();
        assert_eq!(emps.len(), 6, "应有 6 名占位员工");

        // 按角色查活跃员工
        let cfo_emps = svc
            .active_employees_for_role("org-main", "opc-cfo-cfo-financial-analyst")
            .await
            .unwrap();
        assert_eq!(cfo_emps.len(), 1);

        // 员工状态流转
        let updated = svc.update_employee_status(&emps[0].id, "on_leave").await.unwrap();
        assert_eq!(updated.status, "on_leave");
    }

    #[tokio::test]
    async fn talent_template_crud() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let svc = OrgService::new(db);

        svc.add_talent_template(NewTalentTemplate {
            id: "tt-1".to_string(),
            category: "engineering".to_string(),
            name: "AI 工程师".to_string(),
            description: "AI/LLM 应用开发".to_string(),
            source_repo: "agency-agents-src".to_string(),
            prompt_refs: Some(vec!["prompt-a".to_string()]),
            skill_refs: None,
            tags: Some(vec!["ai".to_string()]),
        })
        .await
        .unwrap();

        let all = svc.list_talent_templates(None).await.unwrap();
        assert_eq!(all.len(), 1);
        let eng = svc.list_talent_templates(Some("engineering")).await.unwrap();
        assert_eq!(eng.len(), 1);
        let other = svc.list_talent_templates(Some("finance")).await.unwrap();
        assert!(other.is_empty());
    }
}
