// SPDX-License-Identifier: AGPL-3.0-only

//! v213: OPC 组织抽象表（Self-Built，P3-2）

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let create_orgs = r#"
CREATE TABLE IF NOT EXISTS opc_orgs (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    company_profile TEXT NOT NULL DEFAULT '',
    topology TEXT NOT NULL DEFAULT 'flat',
    final_decider_role_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let create_org_roles = r#"
CREATE TABLE IF NOT EXISTS opc_org_roles (
    id TEXT NOT NULL PRIMARY KEY,
    org_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    name TEXT NOT NULL,
    responsibility TEXT NOT NULL DEFAULT '',
    reports_to TEXT,
    seniority TEXT NOT NULL DEFAULT 'mid',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let create_org_employees = r#"
CREATE TABLE IF NOT EXISTS opc_org_employees (
    id TEXT NOT NULL PRIMARY KEY,
    org_id TEXT NOT NULL,
    employee_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    expert_id TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    experience_ref TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let create_talent_templates = r#"
CREATE TABLE IF NOT EXISTS opc_talent_templates (
    id TEXT NOT NULL PRIMARY KEY,
    category TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    source_repo TEXT NOT NULL DEFAULT '',
    prompt_refs TEXT,
    skill_refs TEXT,
    tags TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let indices = [
        "CREATE INDEX IF NOT EXISTS idx_opc_org_roles_org ON opc_org_roles(org_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_org_employees_org ON opc_org_employees(org_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_org_employees_role ON opc_org_employees(role_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_talent_templates_cat ON opc_talent_templates(category)",
    ];

    for stmt in [create_orgs, create_org_roles, create_org_employees, create_talent_templates] {
        db.execute_unprepared(stmt).await?;
    }
    for stmt in &indices {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
