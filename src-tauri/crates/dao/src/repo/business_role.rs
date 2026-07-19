// SPDX-License-Identifier: AGPL-3.0-only

//! 业务岗位 repository —— 实现 `BusinessRoleRepository` trait。
//!
//! 业务岗位（CEO/CTO/产品经理 等）与 `agent_roles`（抽象执行器类型）区别：
//! 业务岗位表达「在组织里担什么责」，agent_role 表达「怎么干活」。

use sea_orm::*;
use serde_json;

use axagent_entities::business_roles;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::repo_dtos::BusinessRoleDto;
use axagent_harness::repositories::BusinessRoleRepository;
use axagent_harness::util_fns::now_ts;

fn role_from_entity(m: business_roles::Model) -> BusinessRoleDto {
    BusinessRoleDto {
        id: m.id,
        name: m.name,
        description: m.description,
        responsibilities: m.responsibilities,
        decision_authority: m.decision_authority,
        reports_to: m.reports_to,
        managed_expert_ids: m.managed_expert_ids,
        required_certifications: m.required_certifications,
        active_domains: m.active_domains,
        system_prompt: m.system_prompt,
        icon: m.icon,
        color: m.color,
        source: m.source,
        sort_order: m.sort_order,
        is_enabled: m.is_enabled != 0,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

pub async fn list_business_roles(
    db: &DatabaseConnection,
    source: Option<&str>,
) -> Result<Vec<BusinessRoleDto>> {
    let mut query = business_roles::Entity::find()
        .order_by_asc(business_roles::Column::SortOrder)
        .order_by_asc(business_roles::Column::Name);

    if let Some(src) = source {
        query = query.filter(business_roles::Column::Source.eq(src));
    }

    let rows = query.all(db).await?;
    Ok(rows.into_iter().map(role_from_entity).collect())
}

pub async fn get_business_role_entity(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<BusinessRoleDto>> {
    let row = business_roles::Entity::find_by_id(id).one(db).await?;
    Ok(row.map(role_from_entity))
}

pub async fn get_business_role(db: &DatabaseConnection, id: &str) -> Result<BusinessRoleDto> {
    get_business_role_entity(db, id)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("BusinessRole {}", id)))
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_business_role(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
    responsibilities: Option<&[String]>,
    decision_authority: Option<&str>,
    reports_to: Option<&str>,
    managed_expert_ids: Option<&[String]>,
    required_certifications: Option<&[String]>,
    active_domains: Option<&[String]>,
    system_prompt: &str,
    icon: Option<&str>,
    color: Option<&str>,
    source: &str,
    sort_order: i32,
) -> Result<BusinessRoleDto> {
    let now = now_ts();
    let responsibilities_json =
        responsibilities.map(|v| serde_json::to_string(v).unwrap_or_default());
    let managed_experts_json =
        managed_expert_ids.map(|v| serde_json::to_string(v).unwrap_or_default());
    let certifications_json =
        required_certifications.map(|v| serde_json::to_string(v).unwrap_or_default());
    let domains_json = active_domains.map(|v| serde_json::to_string(v).unwrap_or_default());

    // 校验 reports_to（自引用层级，防止环）
    if let Some(parent_id) = reports_to {
        if parent_id == id {
            return Err(AxAgentError::Validation(
                "business_role 的 reports_to 不能指向自己".to_string(),
            ));
        }
        let parent_exists = business_roles::Entity::find_by_id(parent_id).one(db).await?.is_some();
        if !parent_exists {
            return Err(AxAgentError::Validation(format!(
                "reports_to '{}' 不存在于 business_roles 表",
                parent_id
            )));
        }
    }

    let am = business_roles::ActiveModel {
        id: Set(id.to_string()),
        name: Set(name.to_string()),
        description: Set(description.map(|s| s.to_string())),
        responsibilities: Set(responsibilities_json),
        decision_authority: Set(decision_authority.map(|s| s.to_string())),
        reports_to: Set(reports_to.map(|s| s.to_string())),
        managed_expert_ids: Set(managed_experts_json),
        required_certifications: Set(certifications_json),
        active_domains: Set(domains_json),
        system_prompt: Set(system_prompt.to_string()),
        icon: Set(icon.map(|s| s.to_string())),
        color: Set(color.map(|s| s.to_string())),
        source: Set(source.to_string()),
        sort_order: Set(sort_order),
        is_enabled: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
    };

    business_roles::Entity::insert(am)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(business_roles::Column::Id)
                .update_column(business_roles::Column::Name)
                .update_column(business_roles::Column::Description)
                .update_column(business_roles::Column::Responsibilities)
                .update_column(business_roles::Column::DecisionAuthority)
                .update_column(business_roles::Column::ReportsTo)
                .update_column(business_roles::Column::ManagedExpertIds)
                .update_column(business_roles::Column::RequiredCertifications)
                .update_column(business_roles::Column::ActiveDomains)
                .update_column(business_roles::Column::SystemPrompt)
                .update_column(business_roles::Column::Icon)
                .update_column(business_roles::Column::Color)
                .update_column(business_roles::Column::SortOrder)
                .update_column(business_roles::Column::UpdatedAt)
                .to_owned(),
        )
        .exec(db)
        .await?;

    get_business_role(db, id).await
}

pub async fn delete_business_role(db: &DatabaseConnection, id: &str) -> Result<()> {
    let row = business_roles::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("BusinessRole {}", id)))?;

    // 检查是否有下属 business_role（防止孤儿节点）
    let children_count = business_roles::Entity::find()
        .filter(business_roles::Column::ReportsTo.eq(id))
        .count(db)
        .await?;
    if children_count > 0 {
        return Err(AxAgentError::Validation(format!(
            "无法删除 business_role '{}'：仍有 {} 个下属岗位引用它",
            id, children_count
        )));
    }

    business_roles::Entity::delete_by_id(row.id).exec(db).await?;
    Ok(())
}

/// 查询业务岗位树（递归构造，从 roots 开始）
pub async fn list_business_role_tree(db: &DatabaseConnection) -> Result<Vec<BusinessRoleDto>> {
    // 一次性查所有，按 sort_order 排序，调用方在前端自行构造树
    list_business_roles(db, None).await
}

// ── BusinessRoleRepository trait 实现 ──

pub struct BusinessRoleRepositoryImpl {
    pub db: DatabaseConnection,
}

impl BusinessRoleRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl BusinessRoleRepository for BusinessRoleRepositoryImpl {
    async fn get_business_role(
        &self,
        id: &str,
    ) -> std::result::Result<Option<BusinessRoleDto>, String> {
        get_business_role_entity(&self.db, id).await.map_err(|e| e.to_string())
    }

    async fn list_business_roles(&self) -> std::result::Result<Vec<BusinessRoleDto>, String> {
        // 仅返回 is_enabled=true 的记录，按 sort_order + name 排序
        let rows = business_roles::Entity::find()
            .filter(business_roles::Column::IsEnabled.eq(1))
            .order_by_asc(business_roles::Column::SortOrder)
            .order_by_asc(business_roles::Column::Name)
            .all(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(role_from_entity).collect())
    }
}
