// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业专属 agent 组装（role × expert → profile）
//!
//! 目前包含：ai-research 行业的 4 个 agent

use axagent_dao::repo::agent_role;
use axagent_entities::{agency_experts, agent_profiles};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

/// 4 个 ai-research 专家（编译期嵌入）
const AI_RESEARCH_EXPERTS: &[(&str, &str, &str)] = &[
    (
        "ai-research-director",
        "AI 研究负责人",
        include_str!("../../../agency_experts/opc/ai-research-director.md"),
    ),
    (
        "ai-literature-analyst",
        "AI 文献分析师",
        include_str!("../../../agency_experts/opc/ai-literature-analyst.md"),
    ),
    (
        "ai-benchmark-analyst",
        "AI 模型评测专家",
        include_str!("../../../agency_experts/opc/ai-benchmark-analyst.md"),
    ),
    (
        "ai-report-analyst",
        "AI 报告分析师",
        include_str!("../../../agency_experts/opc/ai-report-analyst.md"),
    ),
];

/// 专家 → 角色映射（全部绑定 ai_researcher 角色）
const AI_RESEARCH_EXPERT_ROLE_MAP: &[(&str, &str)] = &[
    ("ai-research-director", "ai_researcher"),
    ("ai-literature-analyst", "ai_researcher"),
    ("ai-benchmark-analyst", "ai_researcher"),
    ("ai-report-analyst", "ai_researcher"),
];

/// 专家 → 工具白名单（只允许引用真实注册的工具）
const AI_RESEARCH_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("ai-research-director", &["OpcListProjects", "OpcCreateProject", "OpcSearchWiki"]),
    ("ai-literature-analyst", &["WebSearch", "FileRead", "OpcSearchWiki"]),
    ("ai-benchmark-analyst", &["Bash", "FileRead", "FileWrite"]),
    ("ai-report-analyst", &["FileWrite", "OpcListKpis", "OpcRecordKpi", "OpcSendNotification"]),
];

/// 主入口：种子化 ai-research 行业专属 agent（expert + role + profile）
pub async fn seed_ai_research_agents(db: &DatabaseConnection) -> Result<(), String> {
    // 1. 种子化 4 个专家到 agency_experts 表
    seed_ai_research_experts(db).await?;

    // 2. 种子化 ai_researcher 角色到 agent_roles 表
    seed_ai_research_role(db).await?;

    // 3. 种子化 4 个 AgentProfile（role × expert 组合）
    seed_ai_research_profiles(db).await?;

    tracing::info!("[opc-industry] ai-research 行业 agent 种子化完成");
    Ok(())
}

/// 种子化 4 个 ai-research 专家到 agency_experts 表
async fn seed_ai_research_experts(db: &DatabaseConnection) -> Result<(), String> {
    let mut count = 0u32;
    for (id, name, content) in AI_RESEARCH_EXPERTS {
        let expert_id = format!("opc-{id}");
        let domain = content
            .lines()
            .find(|l| l.starts_with("domain:"))
            .and_then(|l| l.strip_prefix("domain:").map(|s| s.trim().to_string()))
            .unwrap_or_default();

        let am = agency_experts::ActiveModel {
            id: Set(expert_id.clone()),
            name: Set(name.to_string()),
            description: Set(Some(format!("OPC ai-research — {} 领域", name))),
            category: Set("opc-industry".into()),
            system_prompt: Set(content.to_string()),
            color: Set(Some("#722ed1".to_string())),
            source_dir: Set("opc".into()),
            is_enabled: Set(1),
            imported_at: Set(chrono::Utc::now().timestamp()),
            recommended_workflows: Set(None),
            recommended_tools: Set(None),
            active_domains: Set(Some(serde_json::to_string(&vec![domain]).unwrap_or_default())),
            seniority: Set(None),
            specialties: Set(None),
            parent_role_id: Set(None),
            success_rate: Set(None),
            avg_latency_ms: Set(None),
            avg_token_cost: Set(None),
        };

        agency_experts::Entity::insert(am)
            .on_conflict(
                sea_orm::sea_query::OnConflict::columns([agency_experts::Column::Id])
                    .update_columns([
                        agency_experts::Column::Name,
                        agency_experts::Column::Description,
                        agency_experts::Column::SystemPrompt,
                        agency_experts::Column::ActiveDomains,
                        agency_experts::Column::IsEnabled,
                    ])
                    .to_owned(),
            )
            .exec(db)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        count += 1;
    }
    tracing::info!("[opc-industry] 种子化 {count} 个 ai-research agency_experts");
    Ok(())
}

/// 种子化 ai_researcher 角色到 agent_roles 表（定义来源：roles.rs INDUSTRY_ROLES）
async fn seed_ai_research_role(db: &DatabaseConnection) -> Result<(), String> {
    use super::roles::INDUSTRY_ROLES;
    let mut count = 0u32;
    for role in INDUSTRY_ROLES {
        agent_role::upsert_agent_role(
            db,
            role.id,
            role.name,
            Some(role.description),
            role.system_prompt,
            &[],
            &["Opc".into()],
            role.max_concurrent,
            role.timeout_seconds,
            "opc-industry",
        )
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        count += 1;
    }
    tracing::info!("[opc-industry] 种子化 {count} 个行业角色（ai_researcher）");
    Ok(())
}

/// 种子化 4 个 ai-research AgentProfile（role × expert 组合）
async fn seed_ai_research_profiles(db: &DatabaseConnection) -> Result<(), String> {
    let mut count = 0u32;
    for &(expert_key, role_id) in AI_RESEARCH_EXPERT_ROLE_MAP {
        let profile_id = format!("opc-{role_id}-{expert_key}");
        let expert_id = format!("opc-{expert_key}");

        let display_name = AI_RESEARCH_EXPERTS
            .iter()
            .find(|(k, _, _)| k == &expert_key)
            .map(|(_, n, _)| n.to_string())
            .unwrap_or_else(|| expert_key.to_string());

        let tools_json = AI_RESEARCH_PROFILE_TOOLS
            .iter()
            .find(|(k, _)| k == &expert_key)
            .map(|(_, tools)| serde_json::to_string(tools).unwrap_or_default());

        let now = chrono::Utc::now().timestamp_millis();
        let existing =
            agent_profiles::Entity::find_by_id(&profile_id).one(db).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

        let am = agent_profiles::ActiveModel {
            id: Set(profile_id.clone()),
            name: Set(format!("🤖 {}", display_name)),
            description: Set(Some(format!("OPC ai-research — 角色绑定 {}", role_id))),
            category: Set("opc-industry".into()),
            icon: Set("🤖".into()),
            agent_role: Set(Some(role_id.to_string())),
            source: Set("opc-industry".into()),
            tags: Set(None),
            suggested_provider_id: Set(None),
            suggested_model_id: Set(None),
            suggested_temperature: Set(None),
            suggested_max_tokens: Set(None),
            search_enabled: Set(None),
            recommend_permission_mode: Set(None),
            recommended_tools: Set(tools_json),
            disallowed_tools: Set(None),
            recommended_workflows: Set(None),
            sort_order: Set(0),
            is_enabled: Set(1),
            expert_id: Set(Some(expert_id)),
            business_role_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        if existing.is_some() {
            am.update(db).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        } else {
            am.insert(db).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        }
        count += 1;
    }
    tracing::info!("[opc-industry] 种子化 {count} 个 ai-research agent_profiles");
    Ok(())
}
