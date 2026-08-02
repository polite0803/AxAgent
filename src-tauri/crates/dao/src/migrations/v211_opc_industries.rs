// SPDX-License-Identifier: AGPL-3.0-only

//! v211: OPC 行业注册表（Industry Pack 扫描/启用/禁用/版本追踪）

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let create_industries = r#"
CREATE TABLE IF NOT EXISTS opc_industries (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    icon TEXT NOT NULL DEFAULT '🏢',
    description TEXT NOT NULL DEFAULT '',
    version INTEGER NOT NULL DEFAULT 1,
    enabled INTEGER NOT NULL DEFAULT 1,
    pack_path TEXT NOT NULL DEFAULT '',
    installed_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let indices =
        ["CREATE INDEX IF NOT EXISTS idx_opc_industries_enabled ON opc_industries(enabled)"];

    db.execute_unprepared(create_industries).await?;
    for stmt in &indices {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
