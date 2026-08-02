// SPDX-License-Identifier: AGPL-3.0-only

//! v212: OPC 工作项表（Self-Run 状态机持久层，P3）

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let create = r#"
CREATE TABLE IF NOT EXISTS opc_work_items (
    id TEXT NOT NULL PRIMARY KEY,
    run_id TEXT,
    phase TEXT NOT NULL DEFAULT 'QUEUED',
    title TEXT NOT NULL DEFAULT '',
    owner_role_id TEXT,
    deps_json TEXT NOT NULL DEFAULT '[]',
    assignee_agent_id TEXT,
    management_mode TEXT,
    manager_role_id TEXT,
    last_error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let indices = [
        "CREATE INDEX IF NOT EXISTS idx_opc_work_items_phase ON opc_work_items(phase)",
        "CREATE INDEX IF NOT EXISTS idx_opc_work_items_owner ON opc_work_items(owner_role_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_work_items_run ON opc_work_items(run_id)",
    ];

    db.execute_unprepared(create).await?;
    for stmt in &indices {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
