// SPDX-License-Identifier: AGPL-3.0-only

//! v217: OPC 发布计划表（opc_publish_schedules）

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let create_table = r#"
CREATE TABLE IF NOT EXISTS opc_publish_schedules (
    id TEXT NOT NULL PRIMARY KEY,
    content_ref_type TEXT NOT NULL,
    content_ref_id TEXT NOT NULL,
    scheduled_at INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    published_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let idx_status = "CREATE INDEX IF NOT EXISTS idx_opc_publish_schedules_status ON opc_publish_schedules(status)";
    let idx_scheduled_at = "CREATE INDEX IF NOT EXISTS idx_opc_publish_schedules_scheduled_at ON opc_publish_schedules(scheduled_at)";
    let idx_content = "CREATE INDEX IF NOT EXISTS idx_opc_publish_schedules_content ON opc_publish_schedules(content_ref_type, content_ref_id)";

    db.execute_unprepared(create_table).await?;
    db.execute_unprepared(idx_status).await?;
    db.execute_unprepared(idx_scheduled_at).await?;
    db.execute_unprepared(idx_content).await?;

    Ok(())
}
