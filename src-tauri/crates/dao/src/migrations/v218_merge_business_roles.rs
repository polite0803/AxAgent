// SPDX-License-Identifier: AGPL-3.0-only

//! v218: 业务岗位合并进角色表（business_roles → agent_roles）
//!
//! 背景：business_roles 与 agent_roles 语义重叠（业务岗位 = 组织角色）。
//! 此前两份体系并存：agent_roles（CEO/CTO 等）+ business_roles（证券投资负责人等），
//! agent_profiles.business_role_id 外键引用 business_roles。审计结论：业务岗位就是角色，
//! 独立表是重复设计。本迁移：
//!   1. agent_roles 扩展 business_roles 独有字段
//!   2. 搬 business_roles 数据进 agent_roles（幂等，id 冲突跳过）
//!   3. 删 agent_profiles.business_role_id 列
//!   4. 删 business_roles 表
//!
//! 幂等：列存在性/表存在性先检查再操作；重复执行安全。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ── 1. agent_roles 扩展列（幂等：information_schema / pragma_table_info 查缺） ──
    let new_cols: &[(&str, &str)] = &[
        ("responsibilities", "TEXT"),
        ("decision_authority", "TEXT"),
        ("reports_to", "TEXT"),
        ("managed_expert_ids", "TEXT"),
        ("required_certifications", "TEXT"),
        ("icon", "TEXT"),
        ("color", "TEXT"),
        ("is_enabled", "INTEGER NOT NULL DEFAULT 1"),
    ];
    for (col, ty) in new_cols {
        let exists = column_exists(&db, is_pg, "agent_roles", col).await?;
        if !exists {
            db.execute_unprepared(&format!("ALTER TABLE agent_roles ADD COLUMN {col} {ty}"))
                .await?;
        }
    }

    // ── 2. 搬 business_roles 数据进 agent_roles（表存在且有数据才搬） ──
    let br_exists = table_exists(&db, is_pg, "business_roles").await?;
    if br_exists {
        let select_sql = r#"
SELECT
    br.id, br.name, br.description, br.system_prompt, NULL AS default_tools,
    br.active_domains, 3 AS max_concurrent, 600 AS timeout_seconds,
    br.source, br.sort_order, br.created_at, br.updated_at,
    br.responsibilities, br.decision_authority, br.reports_to, br.managed_expert_ids,
    br.required_certifications, br.icon, br.color, br.is_enabled
FROM business_roles br"#;
        // 2a. 插入新角色（幂等；与 agent_roles 同 id 的 v100 内置岗位 ceo/cto/cfo/cpo 跳过）
        if is_pg {
            db.execute_unprepared(&format!(
                "INSERT INTO agent_roles (\
                    id, name, description, system_prompt, default_tools, active_domains, \
                    max_concurrent, timeout_seconds, source, sort_order, created_at, updated_at, \
                    responsibilities, decision_authority, reports_to, managed_expert_ids, \
                    required_certifications, icon, color, is_enabled) \
                {select_sql} \
                ON CONFLICT (id) DO NOTHING"
            ))
            .await?;
        } else {
            db.execute_unprepared(&format!(
                "INSERT OR IGNORE INTO agent_roles (\
                    id, name, description, system_prompt, default_tools, active_domains, \
                    max_concurrent, timeout_seconds, source, sort_order, created_at, updated_at, \
                    responsibilities, decision_authority, reports_to, managed_expert_ids, \
                    required_certifications, icon, color, is_enabled) \
                {select_sql}"
            ))
            .await?;
        }
        // 2b. 补扩展字段：同 id 冲突行（v100 内置 ceo/cto/cfo/cpo 已在 agent_roles）不回插，
        //     用子查询把 business_roles 的岗位扩展字段（职责/权限/图标/颜色等）补齐到既有行。
        //     子查询形式 PG / SQLite 均兼容。
        db.execute_unprepared(
            "UPDATE agent_roles SET \
                responsibilities = (SELECT br.responsibilities FROM business_roles br WHERE br.id = agent_roles.id), \
                decision_authority = (SELECT br.decision_authority FROM business_roles br WHERE br.id = agent_roles.id), \
                reports_to = (SELECT br.reports_to FROM business_roles br WHERE br.id = agent_roles.id), \
                managed_expert_ids = (SELECT br.managed_expert_ids FROM business_roles br WHERE br.id = agent_roles.id), \
                required_certifications = (SELECT br.required_certifications FROM business_roles br WHERE br.id = agent_roles.id), \
                icon = (SELECT br.icon FROM business_roles br WHERE br.id = agent_roles.id), \
                color = (SELECT br.color FROM business_roles br WHERE br.id = agent_roles.id), \
                is_enabled = (SELECT br.is_enabled FROM business_roles br WHERE br.id = agent_roles.id), \
                updated_at = (SELECT br.updated_at FROM business_roles br WHERE br.id = agent_roles.id) \
             WHERE id IN (SELECT id FROM business_roles)",
        )
        .await?;
    }

    // ── 3. 删 agent_profiles.business_role_id 列（幂等：列存在才删） ──
    let br_id_exists = column_exists(&db, is_pg, "agent_profiles", "business_role_id").await?;
    if br_id_exists {
        db.execute_unprepared("ALTER TABLE agent_profiles DROP COLUMN business_role_id")
            .await?;
    }

    // ── 4. 删 business_roles 表 ──
    if br_exists {
        db.execute_unprepared("DROP TABLE IF EXISTS business_roles").await?;
    }

    Ok(())
}

async fn column_exists(
    db: &sea_orm::DatabaseConnection,
    is_pg: bool,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    if is_pg {
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                format!(
                    "SELECT 1 AS f FROM information_schema.columns \
                     WHERE table_schema = current_schema() \
                       AND table_name = '{table}' AND column_name = '{column}'"
                ),
            ))
            .await?;
        Ok(row.is_some())
    } else {
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                [table.into()],
            ))
            .await?;
        Ok(rows.iter().any(|r| {
            r.try_get_by::<String, _>("name")
                .map(|n| n == column)
                .unwrap_or(false)
        }))
    }
}

async fn table_exists(
    db: &sea_orm::DatabaseConnection,
    is_pg: bool,
    table: &str,
) -> Result<bool, DbErr> {
    if is_pg {
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                format!(
                    "SELECT 1 AS f FROM information_schema.tables \
                     WHERE table_schema = current_schema() AND table_name = '{table}'"
                ),
            ))
            .await?;
        Ok(row.is_some())
    } else {
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                [table.into()],
            ))
            .await?;
        Ok(!rows.is_empty())
    }
}
