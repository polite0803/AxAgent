// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 公司架构种子化 — 按 AxInvest 模式规范化
//!
//! 将专家、角色、Profile 种子化到对应数据库表。
//! 参考 AxInvest 的 stock_analysis_setup 模式。
//!
//! 包含 6 个公司角色 × 20+ 专家 Profile 的组合。
//! 另外启动时自动 import 227 个 agency-agents-src 专家。

use axagent_dao::repo::agent_role;
use axagent_entities::{agency_experts, agent_profiles};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

mod roles;

pub use roles::{OPC_BUSINESS_ROLES, OPC_ROLES};

// ── 编译期嵌入的专家提示词 ──────────────────────────────────────

const EMBEDDED_PROMPTS: &[(&str, &str, &str)] = &[
    (
        "ceo-business-strategist",
        "CEO/创始人",
        include_str!("../../../agency_experts/opc/ceo-business-strategist.md"),
    ),
    (
        "cto-ai-engineer",
        "CTO/技术负责人",
        include_str!("../../../agency_experts/opc/cto-ai-engineer.md"),
    ),
    (
        "cfo-financial-analyst",
        "CFO/财务负责人",
        include_str!("../../../agency_experts/opc/cfo-financial-analyst.md"),
    ),
    (
        "coo-operations-manager",
        "COO/运营负责人",
        include_str!("../../../agency_experts/opc/coo-operations-manager.md"),
    ),
    (
        "cmo-content-strategist",
        "CMO/增长负责人",
        include_str!("../../../agency_experts/opc/cmo-content-strategist.md"),
    ),
    (
        "cpo-product-manager",
        "CPO/产品负责人",
        include_str!("../../../agency_experts/opc/cpo-product-manager.md"),
    ),
];

/// 专家 → 角色 映射
const EXPERT_ROLE_MAP: &[(&str, &str)] = &[
    ("ceo-business-strategist", "ceo"),
    ("cto-ai-engineer", "cto"),
    ("cfo-financial-analyst", "cfo"),
    ("coo-operations-manager", "coo"),
    ("cmo-content-strategist", "cmo"),
    ("cpo-product-manager", "cpo"),
];

/// Profile → 工具白名单
const PROFILE_TOOLS: &[(&str, &[&str])] = &[
    (
        "ceo-business-strategist",
        &[
            "OpcGetDashboard",
            "OpcGetFinancialReport",
            "OpcListKpis",
            "OpcListInvoices",
            "OpcListCustomers",
            "OpcListProjects",
            "OpcSearchWiki",
        ],
    ),
    (
        "cto-ai-engineer",
        &[
            "OpcListProjects",
            "OpcCreateProject",
            "OpcAddMilestone",
            "OpcListKpis",
            "OpcRecordKpi",
            "OpcSearchWiki",
            "OpcSendNotification",
        ],
    ),
    (
        "cfo-financial-analyst",
        &[
            "OpcListInvoices",
            "OpcCreateInvoice",
            "OpcTransitionInvoice",
            "OpcListCustomers",
            "OpcGetDashboard",
            "OpcGetFinancialReport",
            "OpcRecordKpi",
            "OpcListKpis",
            "OpcSendNotification",
            "OpcSearchWiki",
        ],
    ),
    (
        "coo-operations-manager",
        &[
            "OpcListProjects",
            "OpcCreateProject",
            "OpcAddMilestone",
            "OpcListCustomers",
            "OpcCreateCustomer",
            "OpcListInvoices",
            "OpcGetDashboard",
            "OpcSendNotification",
            "OpcSearchWiki",
        ],
    ),
    (
        "cmo-content-strategist",
        &[
            "OpcListCustomers",
            "OpcCreateCustomer",
            "OpcListBlogPosts",
            "OpcCreateLandingPage",
            "OpcListLandingPages",
            "OpcGetDashboard",
            "OpcSendNotification",
            "OpcSearchWiki",
        ],
    ),
    (
        "cpo-product-manager",
        &[
            "OpcListProjects",
            "OpcCreateProject",
            "OpcAddMilestone",
            "OpcListLandingPages",
            "OpcCreateLandingPage",
            "OpcListCustomers",
            "OpcSearchWiki",
        ],
    ),
];

/// 主入口：种子化所有 OPC 专家/角色/Profile
pub async fn ensure_opc_company_seeded(db: &DatabaseConnection) -> Result<(), String> {
    // 1. 种子化 6 个核心公司专家 + 角色 + Profile
    seed_opc_experts(db).await?;
    seed_opc_roles(db).await?;
    seed_opc_business_roles(db).await?;
    seed_opc_profiles(db).await?;

    // 2. 自动导入 agency-agents-src 下 227 个专家文件
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let experts_path = project_root.join("agency-agents-src");
    match crate::commands::agency_expert::import_agency_experts_from_dir(
        db,
        &experts_path.to_string_lossy(),
    )
    .await
    {
        Ok(result) => tracing::info!("[opc-company] 已导入 {} 个专家", result.count),
        Err(e) => tracing::warn!("[opc-company] 专家导入跳过: {}", e),
    }

    // 3. 为所有导入的专家批量创建 agent_profiles
    seed_bulk_expert_profiles(db).await?;

    tracing::info!("[opc-company] 公司架构种子化完成");
    Ok(())
}

/// 种子化 6 个核心专家到 agency_experts 表
async fn seed_opc_experts(db: &DatabaseConnection) -> Result<(), String> {
    let mut count = 0u32;
    for (id, name, content) in EMBEDDED_PROMPTS {
        let expert_id = format!("opc-{id}");
        let domain = content
            .lines()
            .find(|l| l.starts_with("domain:"))
            .and_then(|l| l.strip_prefix("domain:").map(|s| s.trim().to_string()))
            .unwrap_or_default();

        let am = agency_experts::ActiveModel {
            id: Set(expert_id.clone()),
            name: Set(name.to_string()),
            description: Set(Some(format!("OPC {} — {} 领域", name, domain))),
            category: Set("opc-company".into()),
            system_prompt: Set(content.to_string()),
            color: Set(Some("#1890ff".to_string())),
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
    tracing::info!("[opc-company] 种子化 {count} 个 agency_experts");
    Ok(())
}

/// 种子化 6 个公司角色到 agent_roles 表
async fn seed_opc_roles(db: &DatabaseConnection) -> Result<(), String> {
    let mut count = 0u32;
    for role in OPC_ROLES {
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
            "opc-builtin",
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
    tracing::info!("[opc-company] 种子化 {count} 个 agent_roles");
    Ok(())
}

/// 种子化 4 个业务执行岗位到 agent_roles 表
///
/// 这些岗位被 preset_templates.rs 的 PresetStep.role 引用，
/// 工作流执行时 agent_executor 会通过 agent_role 反查此表获取 system_prompt。
async fn seed_opc_business_roles(db: &DatabaseConnection) -> Result<(), String> {
    let mut count = 0u32;
    for role in OPC_BUSINESS_ROLES {
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
            "opc-builtin",
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
    tracing::info!("[opc-company] 种子化 {count} 个业务岗位 agent_roles");
    Ok(())
}

/// 种子化 6 个 AgentProfile（role × expert 组合）
async fn seed_opc_profiles(db: &DatabaseConnection) -> Result<(), String> {
    let mut count = 0u32;
    for &(expert_key, role_id) in EXPERT_ROLE_MAP {
        let profile_id = format!("opc-{role_id}-{expert_key}");
        let expert_id = format!("opc-{expert_key}");

        let display_name = EMBEDDED_PROMPTS
            .iter()
            .find(|(k, _, _)| k == &expert_key)
            .map(|(_, n, _)| n.to_string())
            .unwrap_or_else(|| expert_key.to_string());

        let tools_json = PROFILE_TOOLS
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
            name: Set(format!("🏢 {}", display_name)),
            description: Set(Some(format!("OPC {} — 角色绑定 {}", display_name, role_id))),
            category: Set("opc-company".into()),
            icon: Set("🏢".into()),
            agent_role: Set(Some(role_id.to_string())),
            source: Set("opc-builtin".into()),
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
    tracing::info!("[opc-company] 种子化 {count} 个 agent_profiles");
    Ok(())
}

/// 为所有已导入的 agency_experts 批量创建 agent_profiles
async fn seed_bulk_expert_profiles(db: &DatabaseConnection) -> Result<(), String> {
    let experts = agency_experts::Entity::find()
        .filter(agency_experts::Column::IsEnabled.eq(1))
        .all(db)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let mut count = 0u32;
    for expert in &experts {
        let profile_id = format!("exp-{}-{}", expert.source_dir, expert.id);

        if agent_profiles::Entity::find_by_id(&profile_id)
            .one(db)
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?
            .is_some()
        {
            continue;
        }

        let now = chrono::Utc::now().timestamp_millis();
        let am = agent_profiles::ActiveModel {
            id: Set(profile_id),
            name: Set(expert.name.clone()),
            description: Set(expert.description.clone()),
            category: Set("opc-experts".into()),
            icon: Set("👤".into()),
            agent_role: Set(Some(expert.source_dir.clone())),
            source: Set("opc-bulk".into()),
            tags: Set(None),
            suggested_provider_id: Set(None),
            suggested_model_id: Set(None),
            suggested_temperature: Set(None),
            suggested_max_tokens: Set(None),
            search_enabled: Set(None),
            recommend_permission_mode: Set(None),
            recommended_tools: Set(expert.recommended_tools.clone()),
            disallowed_tools: Set(None),
            recommended_workflows: Set(expert.recommended_workflows.clone()),
            sort_order: Set(0),
            is_enabled: Set(1),
            expert_id: Set(Some(expert.id.clone())),
            business_role_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        am.insert(db).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        count += 1;
    }
    tracing::info!("[opc-company] 批量创建 {count} 个专家 agent_profiles");
    Ok(())
}
