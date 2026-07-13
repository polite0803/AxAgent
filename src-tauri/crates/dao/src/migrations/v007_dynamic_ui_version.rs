// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // 1) 给 dynamic_ui_schemas 表添加 version 字段（默认 "1.0.0"）
    // SQLite 不支持 ADD COLUMN IF NOT EXISTS，先用 execute_unprepared 尝试
    // 如果列已存在，ALTER TABLE ADD COLUMN 会报错，此时忽略即可
    let r = db
        .execute_unprepared(
            "ALTER TABLE dynamic_ui_schemas ADD COLUMN version TEXT NOT NULL DEFAULT '1.0.0'",
        )
        .await;
    if let Err(e) = &r {
        // 如果列已存在则忽略（SQLite 报 duplicate column name）
        tracing::info!(?e, "version 列可能已存在，忽略");
    }

    // 2) 创建版本历史表
    // `AUTOINCREMENT` 是 SQLite 专有；PostgreSQL 用 `SERIAL` 表达自增主键。
    // `created_at` 用 BIGINT 匹配 entity 里 `i64`（PG 下 `INTEGER` 变 INT4 报错）。
    let create_sql = if is_pg {
        "CREATE TABLE IF NOT EXISTS dynamic_ui_schema_versions (\
         id SERIAL PRIMARY KEY, \
         schema_id TEXT NOT NULL, \
         version TEXT NOT NULL, \
         title TEXT NOT NULL, \
         description TEXT NOT NULL DEFAULT '', \
         schema_json TEXT NOT NULL, \
         category TEXT NOT NULL DEFAULT 'custom', \
         tags TEXT NOT NULL DEFAULT '[]', \
         change_log TEXT NOT NULL DEFAULT '', \
         created_at BIGINT NOT NULL)"
    } else {
        "CREATE TABLE IF NOT EXISTS dynamic_ui_schema_versions (\
         id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, \
         schema_id TEXT NOT NULL, \
         version TEXT NOT NULL, \
         title TEXT NOT NULL, \
         description TEXT NOT NULL DEFAULT '', \
         schema_json TEXT NOT NULL, \
         category TEXT NOT NULL DEFAULT 'custom', \
         tags TEXT NOT NULL DEFAULT '[]', \
         change_log TEXT NOT NULL DEFAULT '', \
         created_at INTEGER NOT NULL)"
    };
    db.execute_unprepared(create_sql).await?;

    // 3) 索引
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_dyn_ui_schema_versions_schema \
         ON dynamic_ui_schema_versions (schema_id)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_dyn_ui_schema_versions_created \
         ON dynamic_ui_schema_versions (schema_id, created_at DESC)",
    )
    .await?;

    Ok(())
}
