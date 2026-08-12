// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业专家种子化（专家 → Profile）
//!
//! 注意：角色对应岗位，在 agent 节点中可以为空。
//! 专家（Expert）是核心，Profile 可以只绑定专家，不绑定角色。

use axagent_entities::{agency_experts, agent_profiles};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use super::industry_experts;

// ── AI Research 行业 ──

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

const AI_RESEARCH_PROFILE_TOOLS: &[(&str, &[&str])] = &[
    ("ai-research-director", &["OpcListProjects", "OpcCreateProject", "OpcSearchWiki"]),
    ("ai-literature-analyst", &["WebSearch", "FileRead", "OpcSearchWiki"]),
    ("ai-benchmark-analyst", &["Bash", "FileRead", "FileWrite"]),
    ("ai-report-analyst", &["FileWrite", "OpcListKpis", "OpcRecordKpi", "OpcSendNotification"]),
];

// ── 通用种子化函数 ──

/// 种子化专家到 agency_experts 表
async fn seed_experts(
    db: &DatabaseConnection,
    experts: &[(&str, &str, &str)],
    category: &str,
    color: &str,
) -> Result<(), String> {
    let mut count = 0u32;
    for (id, name, content) in experts {
        let expert_id = format!("opc-{id}");
        let domain = content
            .lines()
            .find(|l| l.starts_with("domain:"))
            .and_then(|l| l.strip_prefix("domain:").map(|s| s.trim().to_string()))
            .unwrap_or_default();

        let am = agency_experts::ActiveModel {
            id: Set(expert_id.clone()),
            name: Set(name.to_string()),
            description: Set(Some(format!("OPC {category} — {name}"))),
            category: Set(category.into()),
            system_prompt: Set(content.to_string()),
            color: Set(Some(color.to_string())),
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
    tracing::info!("[opc-industry] 种子化 {count} 个 {category} agency_experts");
    Ok(())
}

/// 种子化 AgentProfile（仅绑定专家，角色可选）
async fn seed_profiles(
    db: &DatabaseConnection,
    experts: &[(&str, &str, &str)],
    profile_tools: &[(&str, &[&str])],
    category: &str,
    icon: &str,
) -> Result<(), String> {
    let mut count = 0u32;
    for (expert_key, name, _) in experts {
        let profile_id = format!("opc-{expert_key}");
        let expert_id = format!("opc-{expert_key}");

        let display_name = name.to_string();

        let tools_json = profile_tools
            .iter()
            .find(|(k, _)| k == expert_key)
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
            name: Set(format!("{icon} {display_name}")),
            description: Set(Some(format!("OPC {category} — 专家 {expert_key}"))),
            category: Set(category.into()),
            icon: Set(icon.into()),
            agent_role: Set(None), // 角色可选，专家独立存在
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
    tracing::info!("[opc-industry] 种子化 {count} 个 {category} agent_profiles (专家驱动)");
    Ok(())
}

#[allow(dead_code)]
/// 种子化岗位驱动型 AgentProfile（只有角色，没有专家）
///
/// 岗位驱动型工作流：必须有角色，专家可选
/// 示例：总经理审批、财务审批人
pub async fn seed_role_only_profile(
    db: &DatabaseConnection,
    profile_id: &str,
    role_id: &str,
    name: &str,
    description: Option<&str>,
    category: &str,
    icon: &str,
    tools: Option<&[&str]>,
) -> Result<(), String> {
    let display_name = name.to_string();
    let tools_json = tools.map(|t| serde_json::to_string(t).unwrap_or_default());

    let now = chrono::Utc::now().timestamp_millis();
    let existing = agent_profiles::Entity::find_by_id(profile_id).one(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let am = agent_profiles::ActiveModel {
        id: Set(profile_id.to_string()),
        name: Set(format!("{icon} {display_name}")),
        description: Set(description.map(|s| s.to_string())),
        category: Set(category.into()),
        icon: Set(icon.into()),
        agent_role: Set(Some(role_id.to_string())), // 岗位驱动：必须有角色
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
        expert_id: Set(None), // 岗位驱动：专家可选
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
    tracing::info!("[opc-industry] 种子化岗位驱动 Profile: {profile_id} (role={role_id})");
    Ok(())
}

#[allow(dead_code)]
/// 种子化混合型 AgentProfile（角色 + 专家）
///
/// 混合型工作流：角色定义身份，专家定义方法论
/// 示例：CEO + 市场分析师
pub async fn seed_hybrid_profile(
    db: &DatabaseConnection,
    profile_id: &str,
    role_id: &str,
    expert_id: &str,
    name: &str,
    description: Option<&str>,
    category: &str,
    icon: &str,
    tools: Option<&[&str]>,
) -> Result<(), String> {
    let display_name = name.to_string();
    let tools_json = tools.map(|t| serde_json::to_string(t).unwrap_or_default());

    let now = chrono::Utc::now().timestamp_millis();
    let existing = agent_profiles::Entity::find_by_id(profile_id).one(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let am = agent_profiles::ActiveModel {
        id: Set(profile_id.to_string()),
        name: Set(format!("{icon} {display_name}")),
        description: Set(description.map(|s| s.to_string())),
        category: Set(category.into()),
        icon: Set(icon.into()),
        agent_role: Set(Some(role_id.to_string())), // 混合型：角色
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
        expert_id: Set(Some(expert_id.to_string())), // 混合型：专家
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
    tracing::info!(
        "[opc-industry] 种子化混合型 Profile: {profile_id} (role={role_id}, expert={expert_id})"
    );
    Ok(())
}

// ── AI Research 行业 ──

/// 主入口：种子化 ai-research 行业专家和 Profile
pub async fn seed_ai_research_agents(db: &DatabaseConnection) -> Result<(), String> {
    seed_experts(db, AI_RESEARCH_EXPERTS, "opc-industry", "#722ed1").await?;
    seed_profiles(db, AI_RESEARCH_EXPERTS, AI_RESEARCH_PROFILE_TOOLS, "opc-industry", "🤖").await?;
    tracing::info!("[opc-industry] ai-research 行业专家种子化完成");
    Ok(())
}

// ── 12 个新行业 ──

/// 种子化所有 12 个新行业的专家和 Profile
pub async fn seed_all_industry_agents(db: &DatabaseConnection) -> Result<(), String> {
    // 1. 会计与财务管理
    seed_industry_experts_inner(
        db,
        "accounting",
        industry_experts::ACCOUNTING_EXPERTS,
        industry_experts::ACCOUNTING_PROFILE_TOOLS,
        "opc-industry",
        "#1890ff",
        "📊",
    )
    .await?;

    // 2. 金融投资
    seed_industry_experts_inner(
        db,
        "finance-invest",
        industry_experts::FINANCE_INVEST_EXPERTS,
        industry_experts::FINANCE_INVEST_PROFILE_TOOLS,
        "opc-industry",
        "#faad14",
        "💰",
    )
    .await?;

    // 3. 游戏开发
    seed_industry_experts_inner(
        db,
        "game-dev",
        industry_experts::GAME_DEV_EXPERTS,
        industry_experts::GAME_DEV_PROFILE_TOOLS,
        "opc-industry",
        "#52c41a",
        "🎮",
    )
    .await?;

    // 4. 设计
    seed_industry_experts_inner(
        db,
        "design",
        industry_experts::DESIGN_EXPERTS,
        industry_experts::DESIGN_PROFILE_TOOLS,
        "opc-industry",
        "#eb2f96",
        "🎨",
    )
    .await?;

    // 5. 电子商务
    seed_industry_experts_inner(
        db,
        "ecommerce",
        industry_experts::ECOMMERCE_EXPERTS,
        industry_experts::ECOMMERCE_PROFILE_TOOLS,
        "opc-industry",
        "#13c2c2",
        "🛒",
    )
    .await?;

    // 6. 教育培训
    seed_industry_experts_inner(
        db,
        "education",
        industry_experts::EDUCATION_EXPERTS,
        industry_experts::EDUCATION_PROFILE_TOOLS,
        "opc-industry",
        "#2f54eb",
        "📚",
    )
    .await?;

    // 7. 地理信息
    seed_industry_experts_inner(
        db,
        "geospatial",
        industry_experts::GEOSPATIAL_EXPERTS,
        industry_experts::GEOSPATIAL_PROFILE_TOOLS,
        "opc-industry",
        "#389e0d",
        "🗺️",
    )
    .await?;

    // 8. 行业咨询
    seed_industry_experts_inner(
        db,
        "industry-consulting",
        industry_experts::INDUSTRY_CONSULTING_EXPERTS,
        industry_experts::INDUSTRY_CONSULTING_PROFILE_TOOLS,
        "opc-industry",
        "#722ed1",
        "💼",
    )
    .await?;

    // 9. 项目管理
    seed_industry_experts_inner(
        db,
        "project-management",
        industry_experts::PROJECT_MANAGEMENT_EXPERTS,
        industry_experts::PROJECT_MANAGEMENT_PROFILE_TOOLS,
        "opc-industry",
        "#fa8c16",
        "📋",
    )
    .await?;

    // 10. 销售增长
    seed_industry_experts_inner(
        db,
        "sales-growth",
        industry_experts::SALES_GROWTH_EXPERTS,
        industry_experts::SALES_GROWTH_PROFILE_TOOLS,
        "opc-industry",
        "#f5222d",
        "📈",
    )
    .await?;

    // 11. 安全合规
    seed_industry_experts_inner(
        db,
        "security",
        industry_experts::SECURITY_EXPERTS,
        industry_experts::SECURITY_PROFILE_TOOLS,
        "opc-industry",
        "#000000",
        "🔒",
    )
    .await?;

    // 12. 软件开发
    seed_industry_experts_inner(
        db,
        "software-dev",
        industry_experts::SOFTWARE_DEV_EXPERTS,
        industry_experts::SOFTWARE_DEV_PROFILE_TOOLS,
        "opc-industry",
        "#2f54eb",
        "💻",
    )
    .await?;

    tracing::info!("[opc-industry] 12 个新行业专家种子化完成");
    Ok(())
}

/// 内部通用函数：种子化单个行业的专家和 Profile
async fn seed_industry_experts_inner(
    db: &DatabaseConnection,
    industry_name: &str,
    experts: &[(&str, &str, &str)],
    profile_tools: &[(&str, &[&str])],
    category: &str,
    color: &str,
    icon: &str,
) -> Result<(), String> {
    seed_experts(db, experts, category, color).await?;
    seed_profiles(db, experts, profile_tools, category, icon).await?;
    tracing::info!("[opc-industry] {industry_name} 行业专家种子化完成");
    Ok(())
}
