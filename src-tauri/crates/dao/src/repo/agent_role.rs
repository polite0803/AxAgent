// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;
use serde_json;

use axagent_entities::agent_roles;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::AgentRoleDef;

fn role_from_entity(m: agent_roles::Model) -> AgentRoleDef {
    let tools: Vec<String> =
        m.default_tools.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
    let domains: Vec<String> =
        m.active_domains.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();

    AgentRoleDef {
        id: m.id,
        name: m.name,
        description: m.description,
        system_prompt: m.system_prompt,
        default_tools: tools,
        active_domains: domains,
        max_concurrent: m.max_concurrent as usize,
        timeout_seconds: m.timeout_seconds as u64,
        source: m.source,
        sort_order: m.sort_order,
        created_at: m.created_at,
        updated_at: m.updated_at,
        responsibilities: m.responsibilities,
        decision_authority: m.decision_authority,
        reports_to: m.reports_to,
        managed_expert_ids: m.managed_expert_ids,
        required_certifications: m.required_certifications,
        icon: m.icon,
        color: m.color,
        is_enabled: m.is_enabled != 0,
    }
}

pub async fn list_agent_roles(
    db: &DatabaseConnection,
    source: Option<&str>,
) -> Result<Vec<AgentRoleDef>> {
    let mut query = agent_roles::Entity::find()
        .order_by_asc(agent_roles::Column::Source)
        .order_by_asc(agent_roles::Column::SortOrder)
        .order_by_asc(agent_roles::Column::Name);

    if let Some(src) = source {
        query = query.filter(agent_roles::Column::Source.eq(src));
    }

    let rows = query.all(db).await?;
    Ok(rows.into_iter().map(role_from_entity).collect())
}

pub async fn get_agent_role(db: &DatabaseConnection, id: &str) -> Result<Option<AgentRoleDef>> {
    let row = agent_roles::Entity::find_by_id(id).one(db).await?;
    Ok(row.map(role_from_entity))
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_agent_role(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
    system_prompt: &str,
    default_tools: &[String],
    active_domains: &[String],
    max_concurrent: i32,
    timeout_seconds: i64,
    source: &str,
) -> Result<AgentRoleDef> {
    upsert_agent_role_ext(
        db,
        id,
        name,
        description,
        system_prompt,
        default_tools,
        active_domains,
        max_concurrent,
        timeout_seconds,
        source,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        true,
        0,
    )
    .await
}

/// 扩展版 upsert：支持 business_roles 合并后的岗位扩展字段。
/// v218 起 agent_roles 承载组织岗位语义（原 business_roles），职责/权限/汇报/图标等随表并入。
#[allow(clippy::too_many_arguments)]
pub async fn upsert_agent_role_ext(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
    system_prompt: &str,
    default_tools: &[String],
    active_domains: &[String],
    max_concurrent: i32,
    timeout_seconds: i64,
    source: &str,
    responsibilities: Option<&str>,
    decision_authority: Option<&str>,
    reports_to: Option<&str>,
    managed_expert_ids: Option<&str>,
    required_certifications: Option<&str>,
    icon: Option<&str>,
    color: Option<&str>,
    is_enabled: bool,
    sort_order: i32,
) -> Result<AgentRoleDef> {
    let now = axagent_harness::util_fns::now_ts();
    let tools_json = serde_json::to_string(default_tools).unwrap_or_default();
    let domains_json = serde_json::to_string(active_domains).unwrap_or_default();

    let am = agent_roles::ActiveModel {
        id: Set(id.to_string()),
        name: Set(name.to_string()),
        description: Set(description.map(|s| s.to_string())),
        system_prompt: Set(system_prompt.to_string()),
        default_tools: Set(if default_tools.is_empty() {
            None
        } else {
            Some(tools_json)
        }),
        active_domains: Set(if active_domains.is_empty() {
            None
        } else {
            Some(domains_json)
        }),
        max_concurrent: Set(max_concurrent),
        timeout_seconds: Set(timeout_seconds),
        source: Set(source.to_string()),
        sort_order: Set(sort_order),
        created_at: Set(now),
        updated_at: Set(now),
        responsibilities: Set(responsibilities.map(|s| s.to_string())),
        decision_authority: Set(decision_authority.map(|s| s.to_string())),
        reports_to: Set(reports_to.map(|s| s.to_string())),
        managed_expert_ids: Set(managed_expert_ids.map(|s| s.to_string())),
        required_certifications: Set(required_certifications.map(|s| s.to_string())),
        icon: Set(icon.map(|s| s.to_string())),
        color: Set(color.map(|s| s.to_string())),
        is_enabled: Set(if is_enabled { 1 } else { 0 }),
    };

    agent_roles::Entity::insert(am)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(agent_roles::Column::Id)
                .update_column(agent_roles::Column::Name)
                .update_column(agent_roles::Column::Description)
                .update_column(agent_roles::Column::SystemPrompt)
                .update_column(agent_roles::Column::DefaultTools)
                .update_column(agent_roles::Column::ActiveDomains)
                .update_column(agent_roles::Column::MaxConcurrent)
                .update_column(agent_roles::Column::TimeoutSeconds)
                .update_column(agent_roles::Column::Responsibilities)
                .update_column(agent_roles::Column::DecisionAuthority)
                .update_column(agent_roles::Column::ReportsTo)
                .update_column(agent_roles::Column::ManagedExpertIds)
                .update_column(agent_roles::Column::RequiredCertifications)
                .update_column(agent_roles::Column::Icon)
                .update_column(agent_roles::Column::Color)
                .update_column(agent_roles::Column::IsEnabled)
                .update_column(agent_roles::Column::UpdatedAt)
                .to_owned(),
        )
        .exec(db)
        .await?;

    let role = get_agent_role(db, id).await?;
    role.ok_or_else(|| AxAgentError::NotFound(format!("AgentRole {}", id)))
}

pub async fn delete_agent_role(db: &DatabaseConnection, id: &str) -> Result<()> {
    let row = agent_roles::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("AgentRole {}", id)))?;
    agent_roles::Entity::delete_by_id(row.id).exec(db).await?;
    Ok(())
}
