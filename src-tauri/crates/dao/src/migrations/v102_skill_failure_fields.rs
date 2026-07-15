// SPDX-License-Identifier: AGPL-3.0-only
//! v102: 为 trajectory_skills 表添加连续失败追踪字段
//!
//! 新增列：
//! - `consecutive_failures` (INTEGER NOT NULL DEFAULT 0)：连续失败次数
//! - `last_failure_at` (TEXT)：最近一次失败时间，ISO8601 字符串
//!
//! 幂等性：通过 information_schema (PG) / PRAGMA (SQLite) 检查列是否已存在，
//! 已存在则跳过 ALTER，确保迁移可重复执行。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

/// 检查 PG 上某表某列是否已存在
async fn pg_column_exists(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT EXISTS (\
             SELECT 1 FROM information_schema.columns \
             WHERE table_schema = current_schema() \
             AND table_name = $1 AND column_name = $2) AS exists",
            [table.into(), column.into()],
        ))
        .await?;
    let exists: bool = row.and_then(|r| r.try_get_by("exists").ok()).unwrap_or(false);
    Ok(exists)
}

/// 检查 SQLite 上某表某列是否已存在
async fn sqlite_column_exists(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("PRAGMA table_info({})", table),
        ))
        .await?;
    Ok(rows.iter().any(|r| r.try_get_by::<String, _>("name").map(|n| n == column).unwrap_or(false)))
}

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    let migrations: &[(&str, &str, &str)] = &[
        (
            "consecutive_failures",
            "ALTER TABLE trajectory_skills ADD COLUMN consecutive_failures INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE trajectory_skills ADD COLUMN IF NOT EXISTS consecutive_failures INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "last_failure_at",
            "ALTER TABLE trajectory_skills ADD COLUMN last_failure_at TEXT",
            "ALTER TABLE trajectory_skills ADD COLUMN IF NOT EXISTS last_failure_at TEXT",
        ),
    ];

    for (column, sqlite_sql, pg_sql) in migrations {
        let already_exists = match backend {
            DbBackend::Postgres => pg_column_exists(&db, "trajectory_skills", column).await?,
            DbBackend::Sqlite => sqlite_column_exists(&db, "trajectory_skills", column).await?,
            _ => false,
        };
        if already_exists {
            continue;
        }
        let sql = match backend {
            DbBackend::Postgres => pg_sql,
            _ => sqlite_sql,
        };
        // 用 execute_unprepared 避免参数化 DDL 的兼容性问题
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}
