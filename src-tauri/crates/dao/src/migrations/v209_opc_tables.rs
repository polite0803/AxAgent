// SPDX-License-Identifier: AGPL-3.0-only

//! v209: OPC 业务领域表（发票、客户、项目）

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let create_invoices = r#"
CREATE TABLE IF NOT EXISTS opc_invoices (
    id TEXT NOT NULL PRIMARY KEY,
    customer_id TEXT NOT NULL,
    invoice_number TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    line_items_json TEXT NOT NULL DEFAULT '[]',
    subtotal REAL NOT NULL DEFAULT 0.0,
    tax_total REAL NOT NULL DEFAULT 0.0,
    total REAL NOT NULL DEFAULT 0.0,
    currency TEXT NOT NULL DEFAULT 'CNY',
    issued_at INTEGER,
    due_at INTEGER,
    paid_at INTEGER,
    notes TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let create_customers = r#"
CREATE TABLE IF NOT EXISTS opc_customers (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    phone TEXT,
    company TEXT,
    source TEXT,
    tags_json TEXT NOT NULL DEFAULT '[]',
    notes TEXT NOT NULL DEFAULT '',
    total_revenue REAL NOT NULL DEFAULT 0.0,
    invoice_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'lead',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let create_projects = r#"
CREATE TABLE IF NOT EXISTS opc_projects (
    id TEXT NOT NULL PRIMARY KEY,
    customer_id TEXT,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'planning',
    milestones_json TEXT NOT NULL DEFAULT '[]',
    budget REAL,
    currency TEXT NOT NULL DEFAULT 'CNY',
    started_at INTEGER,
    deadline INTEGER,
    completed_at INTEGER,
    notes TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let indices = [
        "CREATE INDEX IF NOT EXISTS idx_opc_invoices_customer ON opc_invoices(customer_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_invoices_status ON opc_invoices(status)",
        "CREATE INDEX IF NOT EXISTS idx_opc_invoices_created ON opc_invoices(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_opc_customers_email ON opc_customers(email)",
        "CREATE INDEX IF NOT EXISTS idx_opc_customers_status ON opc_customers(status)",
        "CREATE INDEX IF NOT EXISTS idx_opc_projects_customer ON opc_projects(customer_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_projects_status ON opc_projects(status)",
    ];

    db.execute_unprepared(create_invoices).await?;
    db.execute_unprepared(create_customers).await?;
    db.execute_unprepared(create_projects).await?;

    for stmt in &indices {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
