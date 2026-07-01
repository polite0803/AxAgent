// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS dynamic_ui_schemas (\
         id TEXT NOT NULL PRIMARY KEY, \
         title TEXT NOT NULL, \
         description TEXT NOT NULL DEFAULT '', \
         schema_json TEXT NOT NULL, \
         category TEXT NOT NULL DEFAULT 'custom', \
         tags TEXT NOT NULL DEFAULT '[]', \
         is_builtin INTEGER NOT NULL DEFAULT 0, \
         created_at TEXT NOT NULL, \
         updated_at TEXT NOT NULL)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS dynamic_ui_form_data (\
         id TEXT NOT NULL PRIMARY KEY, \
         schema_id TEXT NOT NULL, \
         form_data_json TEXT NOT NULL, \
         instance_key TEXT NOT NULL DEFAULT 'default', \
         updated_at TEXT NOT NULL)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_dynamic_ui_schemas_category \
         ON dynamic_ui_schemas (category)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_dynamic_ui_schemas_updated \
         ON dynamic_ui_schemas (updated_at DESC)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_dynamic_ui_form_data_schema \
         ON dynamic_ui_form_data (schema_id)",
    )
    .await?;

    Ok(())
}
