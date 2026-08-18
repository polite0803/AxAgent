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

mod domain_agents;
mod domain_experts;
mod industry_agents;
mod industry_experts;
mod roles;
pub mod seed_opc_cron;
mod seed_opc_workflow_template;

pub use roles::{APPROVAL_ROLES, OPC_OPERATIONAL_ROLES, OPC_ROLES};

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
        "cmo-literary-creator",
        "文学创作者",
        include_str!("../../../agency_experts/opc/cmo-literary-creator.md"),
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
    ("cmo-literary-creator", "cmo"),
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
            "OpcCreateBlogPost",
            "OpcCreateLandingPage",
            "OpcListLandingPages",
            "OpcGetDashboard",
            "OpcSendNotification",
            "OpcSearchWiki",
        ],
    ),
    (
        "cmo-literary-creator",
        &[
            "OpcListBlogPosts",
            "OpcCreateBlogPost",
            "FileWrite",
            "FileRead",
            "WebSearch",
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

/// 主入口：种子化所有 OPC 专家/角色/Profile/工作流模板
pub async fn ensure_opc_company_seeded(db: &DatabaseConnection) -> Result<(), String> {
    // 0. 安全网：确保 category CHECK 约束包含所有业务值（修复旧版 v200 约束遗漏 opc-company 的问题）
    if let Err(e) = axagent_dao::migrations::ensure_category_check_constraints(db).await {
        tracing::warn!("[opc-company] CHECK 约束修复失败（将继续尝试种子化）: {}", e);
    }

    // 1. 种子化 6 个核心公司专家 + 角色 + Profile
    seed_opc_experts(db).await?;
    seed_opc_roles(db).await?;
    seed_opc_operational_roles(db).await?;
    seed_opc_approval_roles(db).await?; // 审批类岗位
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

    // 4. 行业专属 agent（ai-research + 12 个新行业）
    industry_agents::seed_ai_research_agents(db).await?;
    industry_agents::seed_all_industry_agents(db).await?;

    // 5. 领域专属 agent（17 个领域，72 个专家）
    domain_agents::seed_all_domain_agents(db).await?;

    // 6. 种子化需求发现工作流模板（持久化到 workflow_template 表）
    seed_opc_workflow_template::seed_opc_workflow_template(db).await?;

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

/// 种子化 4 个 OPC 运营角色到 agent_roles 表
///
/// 这些角色被 preset_templates.rs 的 PresetStep.role 引用，
/// 工作流执行时 agent_executor 会通过 agent_role 反查此表获取 system_prompt。
async fn seed_opc_operational_roles(db: &DatabaseConnection) -> Result<(), String> {
    let mut count = 0u32;
    for role in OPC_OPERATIONAL_ROLES {
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

/// 种子化 3 个审批类岗位到 agent_roles 表
///
/// 审批类岗位用于岗位驱动型工作流（如总经理审批、财务审批人）
async fn seed_opc_approval_roles(db: &DatabaseConnection) -> Result<(), String> {
    let mut count = 0u32;
    for role in APPROVAL_ROLES {
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
            "opc-approval",
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
    tracing::info!("[opc-company] 种子化 {count} 个审批类 agent_roles");
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

/// 专家源目录（source_dir）→ agent_roles 真实角色映射。
///
/// AgentProfile 是「专家 × 角色」组合：expert_id 指向 agency_experts（人才），
/// agent_role 指向 agent_roles（岗位/执行器）。历史实现把 agent_role 直接填
/// source_dir（如 "engineering"），但 agent_roles 表中不存在该 id → executor
/// 反查失败 → 角色层提示词（怎么干活）静默失效，只剩专家层。
/// 此处把工程相关域映射到真实角色，使 exp-* profile 成为完整组合。
fn role_for_source_dir(dir: &str) -> Option<String> {
    let role = match dir {
        // 技术域 → CTO/技术负责人
        "engineering" | "testing" | "security" | "specialized" | "gamedev" | "gis" | "spatial" => {
            "cto"
        },
        // 项目管理 → 项目经理
        "pm" | "project-management" => "opc_project_manager",
        // 运营/支持 → 运营经理
        "operations" | "support" | "hr" => "opc_operations_manager",
        // 产品/设计 → CPO
        "product" => "cpo",
        "design" => "opc_product_designer",
        // 财务 → CFO
        "finance" | "accounting" | "tax" => "cfo",
        // 增长/销售 → CMO
        "marketing" | "paidmedia" | "sales" => "cmo",
        // 内容创作 → 内容创作者
        "content" | "writing" | "media" => "opc_content_creator",
        // 客户服务 → 客户成功
        "customer-success" | "client-management" => "opc_customer_success",
        // 数据分析 → 数据分析师
        "data" | "analytics" | "bi" => "opc_data_analyst",
        // 战略 → CEO
        "strategy" | "executive" => "ceo",
        // 研究 → AI 研究员
        "academic" | "research" => "ai_researcher",
        // 法律 → 法务
        "legal" | "compliance" => "legal_advisor",
        // 审计 → 审计师
        "audit" | "compliance-audit" => "financial_auditor",
        // 未映射域：保持 source_dir（与历史行为一致，仅影响日志）
        _ => return None,
    };
    Some(role.to_string())
}

/// 为所有已导入的 agency_experts 批量创建 agent_profiles（专家 × 角色组合）
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

        let existing =
            agent_profiles::Entity::find_by_id(&profile_id).one(db).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;

        let now = chrono::Utc::now().timestamp_millis();
        let am = agent_profiles::ActiveModel {
            id: Set(profile_id),
            name: Set(expert.name.clone()),
            description: Set(expert.description.clone()),
            category: Set("opc-experts".into()),
            icon: Set("👤".into()),
            // 专家 × 角色组合：agent_role 指向 agent_roles 真实角色（而非 source_dir）
            agent_role: Set(
                role_for_source_dir(&expert.source_dir).or_else(|| Some(expert.source_dir.clone()))
            ),
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
            created_at: Set(now),
            updated_at: Set(now),
        };
        if existing.is_some() {
            // 历史 profile 已存在：更新组合角色（此前 agent_role=source_dir 反查失败）
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
    tracing::info!("[opc-company] 批量创建 {count} 个专家 agent_profiles");
    Ok(())
}
