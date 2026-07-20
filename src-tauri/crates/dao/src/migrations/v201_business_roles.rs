// SPDX-License-Identifier: AGPL-3.0-only

//! v201: 业务岗位表 + agency_experts / agent_profiles 字段扩展 + workflow_execution_stats 表
//!
//! ## 版本号策略
//!
//! AxInvest 本地迁移从 **v200** 起单调递增，预留 v101–v199 给上游 AxAgent 未来
//! 新增迁移使用。本文件原名 `v101_business_roles.rs`，现重命名为 v201 以落实该策略。
//!
//! ## 背景
//!
//! 目标架构要求「角色对应现实业务岗位（CEO/CTO/产品经理），专家对应现实业务人才
//! （证券分析师/律师）」。原 `agent_roles` 表是抽象执行器类型（executor/planner 等），
//! 语义错位。本 migration 新建 `business_roles` 表表达业务岗位，并扩展 `agency_experts`
//! 表增加人才属性（资历/擅长领域/归属岗位/绩效），扩展 `agent_profiles` 表增加
//! `business_role_id` 外键关联岗位。
//!
//! 同时新增 `workflow_execution_stats` 表，记录每次工作流执行的效果数据，
//! 驱动效果导向的工作流优化（区别于失败驱动的 replan）。
//!
//! ## 幂等性
//!
//! - PostgreSQL：使用 `ADD COLUMN IF NOT EXISTS` / `CREATE TABLE IF NOT EXISTS`，幂等。
//! - SQLite：`CREATE TABLE IF NOT EXISTS` 幂等；`ADD COLUMN` 重复执行会报错，
//!   用 `let _ = ...` 吞掉「重复列」错误实现幂等。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let is_pg = backend == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 创建 business_roles 表（业务岗位）
    // ========================================================================

    let create_business_roles = if is_pg {
        "CREATE TABLE IF NOT EXISTS business_roles (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            responsibilities TEXT, decision_authority TEXT, reports_to TEXT, \
            managed_expert_ids TEXT, required_certifications TEXT, active_domains TEXT, \
            system_prompt TEXT NOT NULL DEFAULT '', \
            icon TEXT, color TEXT, \
            source TEXT NOT NULL DEFAULT 'builtin', \
            sort_order INTEGER NOT NULL DEFAULT 0, is_enabled INTEGER NOT NULL DEFAULT 1, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            FOREIGN KEY (reports_to) REFERENCES business_roles(id) ON DELETE SET NULL)"
    } else {
        "CREATE TABLE IF NOT EXISTS business_roles (\
            id TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, description TEXT, \
            responsibilities TEXT, decision_authority TEXT, reports_to TEXT, \
            managed_expert_ids TEXT, required_certifications TEXT, active_domains TEXT, \
            system_prompt TEXT NOT NULL DEFAULT '', \
            icon TEXT, color TEXT, \
            source TEXT NOT NULL DEFAULT 'builtin', \
            sort_order INTEGER NOT NULL DEFAULT 0, is_enabled INTEGER NOT NULL DEFAULT 1, \
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)"
    };
    db.execute_unprepared(create_business_roles).await?;

    // ========================================================================
    // PHASE 2: 创建 workflow_execution_stats 表（工作流执行统计）
    // ========================================================================

    let create_stats = if is_pg {
        "CREATE TABLE IF NOT EXISTS workflow_execution_stats (\
            id TEXT NOT NULL PRIMARY KEY, mission_hash TEXT, template_id TEXT, \
            execution_id TEXT, status TEXT NOT NULL, \
            total_time_ms BIGINT NOT NULL DEFAULT 0, \
            input_tokens BIGINT NOT NULL DEFAULT 0, \
            output_tokens BIGINT NOT NULL DEFAULT 0, \
            error_message TEXT, user_rating DOUBLE PRECISION, \
            created_at BIGINT NOT NULL)"
    } else {
        "CREATE TABLE IF NOT EXISTS workflow_execution_stats (\
            id TEXT NOT NULL PRIMARY KEY, mission_hash TEXT, template_id TEXT, \
            execution_id TEXT, status TEXT NOT NULL, \
            total_time_ms INTEGER NOT NULL DEFAULT 0, \
            input_tokens INTEGER NOT NULL DEFAULT 0, \
            output_tokens INTEGER NOT NULL DEFAULT 0, \
            error_message TEXT, user_rating REAL, \
            created_at INTEGER NOT NULL)"
    };
    db.execute_unprepared(create_stats).await?;

    // 索引：按 mission_hash 聚合查询（PG/SQLite 语法一致）
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_workflow_exec_stats_mission \
         ON workflow_execution_stats(mission_hash)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_workflow_exec_stats_template \
         ON workflow_execution_stats(template_id)",
    )
    .await?;

    // ========================================================================
    // PHASE 3: 扩展 agency_experts 表（人才属性）
    // ========================================================================

    let agency_experts_columns: &[(&str, &str)] = &[
        ("seniority", "TEXT"),
        ("specialties", "TEXT"),
        ("parent_role_id", "TEXT"),
        ("success_rate", "REAL"),
        ("avg_latency_ms", "BIGINT"),
        ("avg_token_cost", "BIGINT"),
    ];

    for (col, ty) in agency_experts_columns {
        if is_pg {
            let sql = format!("ALTER TABLE agency_experts ADD COLUMN IF NOT EXISTS {} {}", col, ty);
            db.execute_unprepared(&sql).await?;
        } else {
            // SQLite: ADD COLUMN 不支持 IF NOT EXISTS，重复列错误吞掉实现幂等
            let sql = format!("ALTER TABLE agency_experts ADD COLUMN {} {}", col, ty);
            let _ = db.execute_raw(Statement::from_string(backend, sql)).await;
        }
    }

    // SQLite 的 agency_experts.parent_role_id 无法加 FK（SQLite 限制），靠应用层校验。
    // PostgreSQL 的 FK 也跳过（ALTER ADD CONSTRAINT IF NOT EXISTS 在 PG < 9.4 不支持，
    // 且存量库可能存在数据不一致），改由应用层 validate_parent_role_id 校验。

    // ========================================================================
    // PHASE 4: 扩展 agent_profiles 表（business_role_id 外键）
    // ========================================================================

    if is_pg {
        db.execute_unprepared(
            "ALTER TABLE agent_profiles ADD COLUMN IF NOT EXISTS business_role_id TEXT",
        )
        .await?;
    } else {
        let _ = db
            .execute_raw(Statement::from_string(
                backend,
                "ALTER TABLE agent_profiles ADD COLUMN business_role_id TEXT",
            ))
            .await;
    }

    // ========================================================================
    // PHASE 5: 内置业务岗位种子数据（仅首次创建时插入）
    // ========================================================================

    let now = axagent_harness::util_fns::now_ts();
    let builtin_roles = builtin_business_roles(now);

    for role in builtin_roles {
        let stmt = if is_pg {
            Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO business_roles \
                 (id, name, description, responsibilities, decision_authority, reports_to, \
                  managed_expert_ids, required_certifications, active_domains, system_prompt, \
                  icon, color, source, sort_order, is_enabled, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17) \
                 ON CONFLICT (id) DO NOTHING",
                [
                    role.id.into(),
                    role.name.into(),
                    role.description.into(),
                    role.responsibilities.into(),
                    role.decision_authority.into(),
                    role.reports_to.into(),
                    role.managed_expert_ids.into(),
                    role.required_certifications.into(),
                    role.active_domains.into(),
                    role.system_prompt.into(),
                    role.icon.into(),
                    role.color.into(),
                    role.source.into(),
                    role.sort_order.into(),
                    1i32.into(),
                    now.into(),
                    now.into(),
                ],
            )
        } else {
            Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT OR IGNORE INTO business_roles \
                 (id, name, description, responsibilities, decision_authority, reports_to, \
                  managed_expert_ids, required_certifications, active_domains, system_prompt, \
                  icon, color, source, sort_order, is_enabled, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    role.id.into(),
                    role.name.into(),
                    role.description.into(),
                    role.responsibilities.into(),
                    role.decision_authority.into(),
                    role.reports_to.into(),
                    role.managed_expert_ids.into(),
                    role.required_certifications.into(),
                    role.active_domains.into(),
                    role.system_prompt.into(),
                    role.icon.into(),
                    role.color.into(),
                    role.source.into(),
                    role.sort_order.into(),
                    1i32.into(),
                    now.into(),
                    now.into(),
                ],
            )
        };
        db.execute_raw(stmt).await?;
    }

    Ok(())
}

/// 内置业务岗位种子数据
struct BuiltinRole {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    responsibilities: &'static str,
    decision_authority: &'static str,
    reports_to: Option<&'static str>,
    managed_expert_ids: &'static str,
    required_certifications: &'static str,
    active_domains: &'static str,
    system_prompt: &'static str,
    icon: &'static str,
    color: &'static str,
    source: &'static str,
    sort_order: i32,
}

fn builtin_business_roles(_now: i64) -> Vec<BuiltinRole> {
    vec![
        BuiltinRole {
            id: "ceo",
            name: "CEO 首席执行官",
            description: "负责公司整体战略与决策",
            responsibilities: "[\"制定公司战略\",\"重大决策审批\",\"资源分配\"]",
            decision_authority: "{\"max_budget\": 10000000, \"scopes\": [\"all\"]}",
            reports_to: None,
            managed_expert_ids: "[]",
            required_certifications: "[\"10 年管理经验\"]",
            active_domains: "[\"business\",\"strategy\"]",
            system_prompt: "你是 CEO 首席执行官。你负责公司整体战略方向，对重大决策有最终审批权。在分析问题时，从全局视角出发，平衡短期收益与长期价值。",
            icon: "👑",
            color: "#FFD700",
            source: "builtin",
            sort_order: 0,
        },
        BuiltinRole {
            id: "cto",
            name: "CTO 首席技术官",
            description: "负责技术战略与研发管理",
            responsibilities: "[\"技术战略制定\",\"技术选型决策\",\"技术团队管理\",\"技术风险评估\"]",
            decision_authority: "{\"max_budget\": 1000000, \"scopes\": [\"tech\",\"architecture\",\"security\"]}",
            reports_to: Some("ceo"),
            managed_expert_ids: "[]",
            required_certifications: "[\"8 年技术管理经验\"]",
            active_domains: "[\"development\",\"security\",\"devops\",\"data\"]",
            system_prompt: "你是 CTO 首席技术官。你负责技术战略、架构选型与团队管理。在决策时权衡技术先进性、团队能力与交付风险，优先考虑长期可维护性。",
            icon: "💻",
            color: "#4169E1",
            source: "builtin",
            sort_order: 1,
        },
        BuiltinRole {
            id: "cfo",
            name: "CFO 首席财务官",
            description: "负责财务管理与风险控制",
            responsibilities: "[\"财务规划\",\"预算审批\",\"财务风险评估\",\"投资决策\"]",
            decision_authority: "{\"max_budget\": 5000000, \"scopes\": [\"finance\",\"budget\"]}",
            reports_to: Some("ceo"),
            managed_expert_ids: "[]",
            required_certifications: "[\"CPA 或同等资质\",\"8 年财务管理经验\"]",
            active_domains: "[\"finance\",\"business\"]",
            system_prompt: "你是 CFO 首席财务官。你负责财务规划、预算控制与风险评估。在决策时严格把控财务纪律，对投入产出比与现金流敏感。",
            icon: "💰",
            color: "#2E8B57",
            source: "builtin",
            sort_order: 2,
        },
        BuiltinRole {
            id: "cpo",
            name: "CPO 首席产品官",
            description: "负责产品战略与规划",
            responsibilities: "[\"产品战略\",\"需求优先级\",\"用户体验\",\"产品路线图\"]",
            decision_authority: "{\"max_budget\": 500000, \"scopes\": [\"product\",\"design\"]}",
            reports_to: Some("ceo"),
            managed_expert_ids: "[]",
            required_certifications: "[\"8 年产品管理经验\"]",
            active_domains: "[\"business\",\"design\",\"writing\"]",
            system_prompt: "你是 CPO 首席产品官。你负责产品战略、需求优先级与用户体验。在决策时以用户价值为核心，平衡商业目标与技术成本。",
            icon: "🎯",
            color: "#FF6347",
            source: "builtin",
            sort_order: 3,
        },
        BuiltinRole {
            id: "pm",
            name: "产品经理",
            description: "负责产品需求与项目执行",
            responsibilities: "[\"需求分析\",\"产品文档\",\"项目跟进\",\"跨部门协调\"]",
            decision_authority: "{\"max_budget\": 100000, \"scopes\": [\"product\",\"project\"]}",
            reports_to: Some("cpo"),
            managed_expert_ids: "[]",
            required_certifications: "[\"3 年产品经验\"]",
            active_domains: "[\"business\",\"design\",\"writing\"]",
            system_prompt: "你是产品经理。你负责需求分析、产品文档与项目跟进。在执行时关注用户痛点与商业目标，善用数据驱动决策。",
            icon: "📋",
            color: "#9370DB",
            source: "builtin",
            sort_order: 4,
        },
        BuiltinRole {
            id: "tech_lead",
            name: "技术负责人",
            description: "负责技术架构与研发执行",
            responsibilities: "[\"架构设计\",\"技术方案评审\",\"代码审查\",\"技术难点攻坚\"]",
            decision_authority: "{\"max_budget\": 100000, \"scopes\": [\"tech\",\"architecture\"]}",
            reports_to: Some("cto"),
            managed_expert_ids: "[]",
            required_certifications: "[\"5 年研发经验\",\"架构设计能力\"]",
            active_domains: "[\"development\",\"security\",\"devops\"]",
            system_prompt: "你是技术负责人。你负责架构设计、技术评审与代码质量。在执行时关注可维护性、可扩展性与工程效率。",
            icon: "🔧",
            color: "#1E90FF",
            source: "builtin",
            sort_order: 5,
        },
    ]
}
