// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;
use serde_json;

use axagent_entities::agency_experts;
use axagent_entities::agent_profiles;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::AgentProfile;
use axagent_harness::util_fns::now_ts;

/// agent_profiles.category 合法枚举值。
/// 与前端 `src/types/expert.ts` 的 `EXPERT_CATEGORY_KEYS` 保持一致。
const ALLOWED_CATEGORIES: &[&str] = &[
    "general",
    "development",
    "security",
    "data",
    "finance",
    "devops",
    "design",
    "writing",
    "business",
];

/// 校验 category 是否在合法枚举值内。空字符串视为 "general"。
/// 返回归一化后的 category。
fn validate_category(category: &str) -> Result<String> {
    let normalized = if category.is_empty() {
        "general"
    } else {
        category
    };
    if ALLOWED_CATEGORIES.contains(&normalized) {
        Ok(normalized.to_string())
    } else {
        Err(AxAgentError::Validation(format!(
            "Invalid category '{}', allowed: {:?}",
            category, ALLOWED_CATEGORIES
        )))
    }
}

/// 校验 expert_id（如果不为 None）在 agency_experts 表中存在。
async fn validate_expert_id(
    db: &DatabaseConnection,
    expert_id: Option<&str>,
) -> Result<Option<String>> {
    match expert_id {
        None => Ok(None),
        Some("") => Ok(None),
        Some(eid) => {
            let exists = agency_experts::Entity::find_by_id(eid).one(db).await?.is_some();
            if exists {
                Ok(Some(eid.to_string()))
            } else {
                Err(AxAgentError::Validation(format!(
                    "expert_id '{}' does not exist in agency_experts",
                    eid
                )))
            }
        },
    }
}

/// 校验 business_role_id（如果不为 None）在 business_roles 表中存在。
async fn validate_business_role_id(
    db: &DatabaseConnection,
    business_role_id: Option<&str>,
) -> Result<Option<String>> {
    use axagent_entities::business_roles;
    match business_role_id {
        None => Ok(None),
        Some("") => Ok(None),
        Some(rid) => {
            let exists = business_roles::Entity::find_by_id(rid).one(db).await?.is_some();
            if exists {
                Ok(Some(rid.to_string()))
            } else {
                Err(AxAgentError::Validation(format!(
                    "business_role_id '{}' does not exist in business_roles",
                    rid
                )))
            }
        },
    }
}

fn profile_from_entity(m: agent_profiles::Model) -> AgentProfile {
    let parse_json_arr = |raw: &Option<String>| -> Vec<String> {
        raw.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default()
    };

    AgentProfile {
        id: m.id,
        name: m.name,
        description: m.description,
        category: m.category,
        icon: m.icon,
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
        expert_id: m.expert_id,
        business_role_id: m.business_role_id,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn stringify_json_arr(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|e| {
        tracing::warn!("JSON 数组序列化失败: {e}");
        "[]".to_string()
    })
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

#[allow(clippy::too_many_arguments)]
pub async fn create_agent_profile(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
    category: &str,
    icon: &str,
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
        agent_role: Set(agent_role.map(|s| s.to_string())),
        source: Set(source.to_string()),
        tags: Set(if tags.is_empty() {
            None
        } else {
            Some(stringify_json_arr(tags))
        }),
        sort_order: Set(0),
        is_enabled: Set(1),
        expert_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    get_agent_profile(db, id).await
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_agent_profile(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
    category: &str,
    icon: &str,
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
    expert_id: Option<&str>,
    business_role_id: Option<&str>,
) -> Result<AgentProfile> {
    let now = now_ts();

    // P2-8: category 白名单校验（存量库兜底，新部署由 v100 DDL CHECK 约束保证）
    let category = validate_category(category)?;
    // P1-5: expert_id 存在性校验（存量库兜底，新部署由 v100 DDL FK 约束保证）
    let expert_id = validate_expert_id(db, expert_id).await?;
    // 业务岗位 ID 存在性校验
    let business_role_id = validate_business_role_id(db, business_role_id).await?;

    let am = agent_profiles::ActiveModel {
        id: Set(id.to_string()),
        name: Set(name.to_string()),
        description: Set(description.map(|s| s.to_string())),
        category: Set(category),
        icon: Set(icon.to_string()),
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
        expert_id: Set(expert_id),
        business_role_id: Set(business_role_id),
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
                .update_column(agent_profiles::Column::ExpertId)
                .update_column(agent_profiles::Column::BusinessRoleId)
                .update_column(agent_profiles::Column::UpdatedAt)
                .to_owned(),
        )
        .exec(db)
        .await?;

    get_agent_profile(db, id).await
}

#[allow(clippy::too_many_arguments)]
pub async fn update_agent_profile(
    db: &DatabaseConnection,
    id: &str,
    name: Option<&str>,
    description: Option<Option<&str>>,
    category: Option<&str>,
    icon: Option<&str>,
    agent_role: Option<Option<&str>>,
    tags: Option<&[String]>,
    is_enabled: Option<bool>,
    business_role_id: Option<Option<&str>>,
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
        // P2-8: category 白名单校验
        am.category = Set(validate_category(v)?);
    }
    if let Some(v) = icon {
        am.icon = Set(v.to_string());
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
    if let Some(v) = business_role_id {
        // 校验目标 business_role_id 存在性（None 表示解除关联）
        let validated = validate_business_role_id(db, v).await?;
        am.business_role_id = Set(validated);
    }

    am.update(db).await?;
    get_agent_profile(db, id).await
}

pub async fn delete_agent_profile(db: &DatabaseConnection, id: &str) -> Result<()> {
    let row = agent_profiles::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("AgentProfile {}", id)))?;

    agent_profiles::Entity::delete_by_id(row.id).exec(db).await?;
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
    Ok((profile.agent_role, profile.recommended_tools, profile.disallowed_tools))
}
