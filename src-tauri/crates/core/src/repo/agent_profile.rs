use sea_orm::*;
use serde_json;

use crate::entity::agent_profiles;
use crate::error::{AxAgentError, Result};
use crate::types::AgentProfile;
use crate::utils::now_ts;

fn profile_from_entity(m: agent_profiles::Model) -> AgentProfile {
    let parse_json_arr = |raw: &Option<String>| -> Vec<String> {
        raw.as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    };

    AgentProfile {
        id: m.id,
        name: m.name,
        description: m.description,
        category: m.category,
        icon: m.icon,
        system_prompt: m.system_prompt,
        agent_role: m.agent_role,
        source: m.source,
        tags: parse_json_arr(&m.tags),
        suggested_provider_id: m.suggested_provider_id,
        suggested_model_id: m.suggested_model_id,
        suggested_temperature: m.suggested_temperature,
        suggested_max_tokens: m.suggested_max_tokens.map(|v| v as u32),
        search_enabled: m.search_enabled,
        recommend_permission_mode: m.recommend_permission_mode,
        recommended_tools: parse_json_arr(&m.recommended_tools),
        disallowed_tools: parse_json_arr(&m.disallowed_tools),
        recommended_workflows: parse_json_arr(&m.recommended_workflows),
        sort_order: m.sort_order,
        is_enabled: m.is_enabled != 0,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn stringify_json_arr(values: &[String]) -> String {
    serde_json::to_string(values).expect("failed to serialize JSON array")
}

pub async fn list_agent_profiles(
    db: &DatabaseConnection,
    source: Option<&str>,
) -> Result<Vec<AgentProfile>> {
    let mut query = agent_profiles::Entity::find()
        .filter(agent_profiles::Column::IsEnabled.eq(1))
        .order_by_asc(agent_profiles::Column::Source)
        .order_by_asc(agent_profiles::Column::SortOrder)
        .order_by_asc(agent_profiles::Column::Name);

    if let Some(src) = source {
        query = query.filter(agent_profiles::Column::Source.eq(src));
    }

    let rows = query.all(db).await?;
    Ok(rows.into_iter().map(profile_from_entity).collect())
}

pub async fn get_agent_profile(db: &DatabaseConnection, id: &str) -> Result<AgentProfile> {
    let row = agent_profiles::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("AgentProfile {}", id)))?;

    Ok(profile_from_entity(row))
}

pub async fn get_agent_profile_system_prompt(db: &DatabaseConnection, id: &str) -> Result<String> {
    let row = agent_profiles::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("AgentProfile {}", id)))?;

    Ok(row.system_prompt)
}

pub async fn create_agent_profile(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
    category: &str,
    icon: &str,
    system_prompt: &str,
    agent_role: Option<&str>,
    source: &str,
    tags: &[String],
) -> Result<AgentProfile> {
    let now = now_ts();
    agent_profiles::ActiveModel {
        id: Set(id.to_string()),
        name: Set(name.to_string()),
        description: Set(description.map(|s| s.to_string())),
        category: Set(category.to_string()),
        icon: Set(icon.to_string()),
        system_prompt: Set(system_prompt.to_string()),
        agent_role: Set(agent_role.map(|s| s.to_string())),
        source: Set(source.to_string()),
        tags: Set(if tags.is_empty() {
            None
        } else {
            Some(stringify_json_arr(tags))
        }),
        sort_order: Set(0),
        is_enabled: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    get_agent_profile(db, id).await
}

pub async fn upsert_agent_profile(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
    category: &str,
    icon: &str,
    system_prompt: &str,
    agent_role: Option<&str>,
    source: &str,
    tags: &[String],
    suggested_provider_id: Option<&str>,
    suggested_model_id: Option<&str>,
    suggested_temperature: Option<f64>,
    suggested_max_tokens: Option<i64>,
    search_enabled: Option<bool>,
    recommend_permission_mode: Option<&str>,
    recommended_tools: &[String],
    disallowed_tools: &[String],
    recommended_workflows: &[String],
) -> Result<AgentProfile> {
    let now = now_ts();

    let am = agent_profiles::ActiveModel {
        id: Set(id.to_string()),
        name: Set(name.to_string()),
        description: Set(description.map(|s| s.to_string())),
        category: Set(category.to_string()),
        icon: Set(icon.to_string()),
        system_prompt: Set(system_prompt.to_string()),
        agent_role: Set(agent_role.map(|s| s.to_string())),
        source: Set(source.to_string()),
        tags: Set(if tags.is_empty() {
            None
        } else {
            Some(stringify_json_arr(tags))
        }),
        suggested_provider_id: Set(suggested_provider_id.map(|s| s.to_string())),
        suggested_model_id: Set(suggested_model_id.map(|s| s.to_string())),
        suggested_temperature: Set(suggested_temperature),
        suggested_max_tokens: Set(suggested_max_tokens),
        search_enabled: Set(search_enabled),
        recommend_permission_mode: Set(recommend_permission_mode.map(|s| s.to_string())),
        recommended_tools: Set(if recommended_tools.is_empty() {
            None
        } else {
            Some(stringify_json_arr(recommended_tools))
        }),
        disallowed_tools: Set(if disallowed_tools.is_empty() {
            None
        } else {
            Some(stringify_json_arr(disallowed_tools))
        }),
        recommended_workflows: Set(if recommended_workflows.is_empty() {
            None
        } else {
            Some(stringify_json_arr(recommended_workflows))
        }),
        sort_order: Set(0),
        is_enabled: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
    };

    agent_profiles::Entity::insert(am)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(agent_profiles::Column::Id)
                .update_column(agent_profiles::Column::Name)
                .update_column(agent_profiles::Column::Description)
                .update_column(agent_profiles::Column::Category)
                .update_column(agent_profiles::Column::Icon)
                .update_column(agent_profiles::Column::SystemPrompt)
                .update_column(agent_profiles::Column::AgentRole)
                .update_column(agent_profiles::Column::Tags)
                .update_column(agent_profiles::Column::SuggestedProviderId)
                .update_column(agent_profiles::Column::SuggestedModelId)
                .update_column(agent_profiles::Column::SuggestedTemperature)
                .update_column(agent_profiles::Column::SuggestedMaxTokens)
                .update_column(agent_profiles::Column::SearchEnabled)
                .update_column(agent_profiles::Column::RecommendPermissionMode)
                .update_column(agent_profiles::Column::RecommendedTools)
                .update_column(agent_profiles::Column::DisallowedTools)
                .update_column(agent_profiles::Column::RecommendedWorkflows)
                .update_column(agent_profiles::Column::UpdatedAt)
                .to_owned(),
        )
        .exec(db)
        .await?;

    get_agent_profile(db, id).await
}

pub async fn update_agent_profile(
    db: &DatabaseConnection,
    id: &str,
    name: Option<&str>,
    description: Option<Option<&str>>,
    category: Option<&str>,
    icon: Option<&str>,
    system_prompt: Option<&str>,
    agent_role: Option<Option<&str>>,
    tags: Option<&[String]>,
    is_enabled: Option<bool>,
) -> Result<AgentProfile> {
    let row = agent_profiles::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("AgentProfile {}", id)))?;

    let mut am: agent_profiles::ActiveModel = row.into();
    am.updated_at = Set(now_ts());

    if let Some(v) = name {
        am.name = Set(v.to_string());
    }
    if let Some(v) = description {
        am.description = Set(v.map(|s| s.to_string()));
    }
    if let Some(v) = category {
        am.category = Set(v.to_string());
    }
    if let Some(v) = icon {
        am.icon = Set(v.to_string());
    }
    if let Some(v) = system_prompt {
        am.system_prompt = Set(v.to_string());
    }
    if let Some(v) = agent_role {
        am.agent_role = Set(v.map(|s| s.to_string()));
    }
    if let Some(v) = tags {
        am.tags = Set(if v.is_empty() {
            None
        } else {
            Some(stringify_json_arr(v))
        });
    }
    if let Some(v) = is_enabled {
        am.is_enabled = Set(if v { 1 } else { 0 });
    }

    am.update(db).await?;
    get_agent_profile(db, id).await
}

pub async fn delete_agent_profile(db: &DatabaseConnection, id: &str) -> Result<()> {
    let row = agent_profiles::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("AgentProfile {}", id)))?;

    agent_profiles::Entity::delete_by_id(row.id)
        .exec(db)
        .await?;
    Ok(())
}

/// 将 agent_role 字符串映射到对应的 AgentRole 工具列表（用于后端运行时）
pub async fn resolve_profile_tools(
    db: &DatabaseConnection,
    profile_id: &str,
) -> Result<(Option<String>, Vec<String>, Vec<String>)> {
    let profile = get_agent_profile(db, profile_id).await?;
    // agent_role 字符串
    // recommended_tools 额外工具
    // disallowed_tools 禁止工具
    Ok((
        profile.agent_role,
        profile.recommended_tools,
        profile.disallowed_tools,
    ))
}
