// SPDX-License-Identifier: AGPL-3.0-only

//! v210: OPC 扩展表（站点、分析、自动化）

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let landing_pages = r#"
CREATE TABLE IF NOT EXISTS opc_landing_pages (
    id TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    published INTEGER NOT NULL DEFAULT 0,
    published_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let blog_posts = r#"
CREATE TABLE IF NOT EXISTS opc_blog_posts (
    id TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    excerpt TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    published INTEGER NOT NULL DEFAULT 0,
    published_at INTEGER,
    view_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let contacts = r#"
CREATE TABLE IF NOT EXISTS opc_contact_submissions (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    message TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT '',
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
)"#;

    let kpi_records = r#"
CREATE TABLE IF NOT EXISTS opc_kpi_records (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    value REAL NOT NULL,
    unit TEXT NOT NULL DEFAULT '',
    period TEXT NOT NULL DEFAULT '',
    recorded_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
)"#;

    let revenue_records = r#"
CREATE TABLE IF NOT EXISTS opc_revenue_records (
    id TEXT NOT NULL PRIMARY KEY,
    amount REAL NOT NULL,
    currency TEXT NOT NULL DEFAULT 'CNY',
    category TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    recorded_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
)"#;

    let automation_rules = r#"
CREATE TABLE IF NOT EXISTS opc_automation_rules (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    trigger_type TEXT NOT NULL,
    trigger_config TEXT NOT NULL DEFAULT '{}',
    action_type TEXT NOT NULL,
    action_config TEXT NOT NULL DEFAULT '{}',
    enabled INTEGER NOT NULL DEFAULT 1,
    last_run_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let follow_up_tasks = r#"
CREATE TABLE IF NOT EXISTS opc_follow_up_tasks (
    id TEXT NOT NULL PRIMARY KEY,
    task_type TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    priority TEXT NOT NULL DEFAULT 'medium',
    due_at INTEGER,
    completed_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let indices = [
        "CREATE INDEX IF NOT EXISTS idx_opc_landing_slug ON opc_landing_pages(slug)",
        "CREATE INDEX IF NOT EXISTS idx_opc_blog_slug ON opc_blog_posts(slug)",
        "CREATE INDEX IF NOT EXISTS idx_opc_kpi_name ON opc_kpi_records(name)",
        "CREATE INDEX IF NOT EXISTS idx_opc_kpi_period ON opc_kpi_records(period)",
        "CREATE INDEX IF NOT EXISTS idx_opc_revenue_category ON opc_revenue_records(category)",
        "CREATE INDEX IF NOT EXISTS idx_opc_followup_status ON opc_follow_up_tasks(status)",
    ];

    for stmt in &[
        landing_pages,
        blog_posts,
        contacts,
        kpi_records,
        revenue_records,
        automation_rules,
        follow_up_tasks,
    ] {
        db.execute_unprepared(stmt).await?;
    }
    for stmt in &indices {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
