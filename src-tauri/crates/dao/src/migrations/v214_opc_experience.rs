// SPDX-License-Identifier: AGPL-3.0-only

//! v214: OPC 经验闭环表（Self-Grown，P3-5）

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let create_records = r#"
CREATE TABLE IF NOT EXISTS opc_experience_records (
    id TEXT NOT NULL PRIMARY KEY,
    role_id TEXT NOT NULL,
    work_item_id TEXT NOT NULL,
    signal TEXT NOT NULL DEFAULT 'success',
    content TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL
)"#;

    let create_playbooks = r#"
CREATE TABLE IF NOT EXISTS opc_playbooks (
    id TEXT NOT NULL PRIMARY KEY,
    role_id TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    promoted_from TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let indices = [
        "CREATE INDEX IF NOT EXISTS idx_opc_experience_role ON opc_experience_records(role_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_experience_workitem ON opc_experience_records(work_item_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_playbooks_role ON opc_playbooks(role_id)",
    ];

    for stmt in [create_records, create_playbooks] {
        db.execute_unprepared(stmt).await?;
    }
    for stmt in &indices {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
