use sea_orm::*;
use serde_json;

use crate::entity::agent_roles;
use crate::error::{AxAgentError, Result};
use crate::types::AgentRoleDef;

fn role_from_entity(m: agent_roles::Model) -> AgentRoleDef {
    let tools: Vec<String> = m
        .default_tools
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    AgentRoleDef {
        id: m.id,
        name: m.name,
        description: m.description,
        system_prompt: m.system_prompt,
        default_tools: tools,
        max_concurrent: m.max_concurrent as usize,
        timeout_seconds: m.timeout_seconds as u64,
        source: m.source,
        sort_order: m.sort_order,
        created_at: m.created_at,
        updated_at: m.updated_at,
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

pub async fn upsert_agent_role(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
    system_prompt: &str,
    default_tools: &[String],
    max_concurrent: i32,
    timeout_seconds: i64,
    source: &str,
) -> Result<AgentRoleDef> {
    let now = crate::utils::now_ts();
    let tools_json = serde_json::to_string(default_tools).unwrap_or_default();

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
        max_concurrent: Set(max_concurrent),
        timeout_seconds: Set(timeout_seconds),
        source: Set(source.to_string()),
        sort_order: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
    };

    agent_roles::Entity::insert(am)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(agent_roles::Column::Id)
                .update_column(agent_roles::Column::Name)
                .update_column(agent_roles::Column::Description)
                .update_column(agent_roles::Column::SystemPrompt)
                .update_column(agent_roles::Column::DefaultTools)
                .update_column(agent_roles::Column::MaxConcurrent)
                .update_column(agent_roles::Column::TimeoutSeconds)
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
