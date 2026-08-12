// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 领域专家种子化（专家 → Profile）
//!
//! 注意：角色对应岗位，在 agent 节点中可以为空。
//! 专家（Expert）是核心，Profile 可以只绑定专家，不绑定角色。
//! 领域专家采用专家驱动模式（无角色绑定），纯执行类工作流使用。

use axagent_entities::{agency_experts, agent_profiles};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use super::domain_experts;

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
    tracing::info!("[opc-domain] 种子化 {count} 个 {category} agency_experts");
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
            agent_role: Set(None), // 领域专家采用专家驱动模式，无角色绑定
            source: Set("opc-domain".into()),
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
    tracing::info!("[opc-domain] 种子化 {count} 个 {category} agent_profiles (专家驱动)");
    Ok(())
}

/// 内部通用函数：种子化单个领域的专家和 Profile
async fn seed_domain_agents_inner(
    db: &DatabaseConnection,
    domain_name: &str,
    experts: &[(&str, &str, &str)],
    profile_tools: &[(&str, &[&str])],
    category: &str,
    color: &str,
    icon: &str,
) -> Result<(), String> {
    seed_experts(db, experts, category, color).await?;
    seed_profiles(db, experts, profile_tools, category, icon).await?;
    tracing::info!("[opc-domain] {domain_name} 领域专家种子化完成");
    Ok(())
}

/// 种子化所有 17 个领域的专家和 Profile
pub async fn seed_all_domain_agents(db: &DatabaseConnection) -> Result<(), String> {
    // 1. 学术研究
    seed_domain_agents_inner(
        db,
        "academic",
        domain_experts::ACADEMIC_EXPERTS,
        domain_experts::ACADEMIC_PROFILE_TOOLS,
        "opc-domain",
        "#722ed1",
        "📚",
    )
    .await?;

    // 2. 设计
    seed_domain_agents_inner(
        db,
        "design",
        domain_experts::DESIGN_EXPERTS,
        domain_experts::DESIGN_PROFILE_TOOLS,
        "opc-domain",
        "#eb2f96",
        "🎨",
    )
    .await?;

    // 3. 工程开发
    seed_domain_agents_inner(
        db,
        "engineering",
        domain_experts::ENGINEERING_EXPERTS,
        domain_experts::ENGINEERING_PROFILE_TOOLS,
        "opc-domain",
        "#2f54eb",
        "💻",
    )
    .await?;

    // 4. 金融财务
    seed_domain_agents_inner(
        db,
        "finance",
        domain_experts::FINANCE_EXPERTS,
        domain_experts::FINANCE_PROFILE_TOOLS,
        "opc-domain",
        "#faad14",
        "💰",
    )
    .await?;

    // 5. 游戏开发
    seed_domain_agents_inner(
        db,
        "gamedev",
        domain_experts::GAMEDEV_EXPERTS,
        domain_experts::GAMEDEV_PROFILE_TOOLS,
        "opc-domain",
        "#52c41a",
        "🎮",
    )
    .await?;

    // 6. 地理信息
    seed_domain_agents_inner(
        db,
        "gis",
        domain_experts::GIS_EXPERTS,
        domain_experts::GIS_PROFILE_TOOLS,
        "opc-domain",
        "#389e0d",
        "🗺️",
    )
    .await?;

    // 7. 市场营销
    seed_domain_agents_inner(
        db,
        "marketing",
        domain_experts::MARKETING_EXPERTS,
        domain_experts::MARKETING_PROFILE_TOOLS,
        "opc-domain",
        "#f5222d",
        "📈",
    )
    .await?;

    // 8. 付费媒体
    seed_domain_agents_inner(
        db,
        "paidmedia",
        domain_experts::PAIDMEDIA_EXPERTS,
        domain_experts::PAIDMEDIA_PROFILE_TOOLS,
        "opc-domain",
        "#fa8c16",
        "📢",
    )
    .await?;

    // 9. 项目管理
    seed_domain_agents_inner(
        db,
        "pm",
        domain_experts::PM_EXPERTS,
        domain_experts::PM_PROFILE_TOOLS,
        "opc-domain",
        "#1890ff",
        "📋",
    )
    .await?;

    // 10. 产品管理
    seed_domain_agents_inner(
        db,
        "product",
        domain_experts::PRODUCT_EXPERTS,
        domain_experts::PRODUCT_PROFILE_TOOLS,
        "opc-domain",
        "#13c2c2",
        "📦",
    )
    .await?;

    // 11. 销售
    seed_domain_agents_inner(
        db,
        "sales",
        domain_experts::SALES_EXPERTS,
        domain_experts::SALES_PROFILE_TOOLS,
        "opc-domain",
        "#f5222d",
        "🤝",
    )
    .await?;

    // 12. 安全
    seed_domain_agents_inner(
        db,
        "security",
        domain_experts::SECURITY_EXPERTS,
        domain_experts::SECURITY_PROFILE_TOOLS,
        "opc-domain",
        "#000000",
        "🔒",
    )
    .await?;

    // 13. 空间数据
    seed_domain_agents_inner(
        db,
        "spatial",
        domain_experts::SPATIAL_EXPERTS,
        domain_experts::SPATIAL_PROFILE_TOOLS,
        "opc-domain",
        "#389e0d",
        "🌍",
    )
    .await?;

    // 14. 专业领域
    seed_domain_agents_inner(
        db,
        "specialized",
        domain_experts::SPECIALIZED_EXPERTS,
        domain_experts::SPECIALIZED_PROFILE_TOOLS,
        "opc-domain",
        "#722ed1",
        "🔬",
    )
    .await?;

    // 15. 战略规划
    seed_domain_agents_inner(
        db,
        "strategy",
        domain_experts::STRATEGY_EXPERTS,
        domain_experts::STRATEGY_PROFILE_TOOLS,
        "opc-domain",
        "#fa8c16",
        "🎯",
    )
    .await?;

    // 16. 技术支持
    seed_domain_agents_inner(
        db,
        "support",
        domain_experts::SUPPORT_EXPERTS,
        domain_experts::SUPPORT_PROFILE_TOOLS,
        "opc-domain",
        "#13c2c2",
        "🛠️",
    )
    .await?;

    // 17. 测试
    seed_domain_agents_inner(
        db,
        "testing",
        domain_experts::TESTING_EXPERTS,
        domain_experts::TESTING_PROFILE_TOOLS,
        "opc-domain",
        "#52c41a",
        "🧪",
    )
    .await?;

    tracing::info!("[opc-domain] 17 个领域专家种子化完成");
    Ok(())
}
