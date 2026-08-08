// SPDX-License-Identifier: AGPL-3.0-only

//! v216: OPC 内容资产表（opc_content_assets）

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let content_assets = r#"
CREATE TABLE IF NOT EXISTS opc_content_assets (
    id TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    content_type TEXT NOT NULL,
    body TEXT NOT NULL DEFAULT '',
    tags_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'draft',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)"#;

    let idx_status =
        "CREATE INDEX IF NOT EXISTS idx_opc_content_assets_status ON opc_content_assets(status)";
    let idx_type = "CREATE INDEX IF NOT EXISTS idx_opc_content_assets_type ON opc_content_assets(content_type)";

    db.execute_unprepared(content_assets).await?;
    db.execute_unprepared(idx_status).await?;
    db.execute_unprepared(idx_type).await?;

    Ok(())
}
