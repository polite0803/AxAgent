// SPDX-License-Identifier: AGPL-3.0-only
//! v109_repair_memory_items_columns: 修复 memory_items 表缺失的列。
//!
//! ## 背景
//!
//! v101 迁移在 SQLite 路径上用 `let _ = db.execute_unprepared(...)` 静默
//! 吞掉了 ALTER TABLE 错误，导致部分旧数据库（v100 建表后 v101 ALTER 失败）
//! 缺少 tier/importance/access_count 等 trajectory memory 字段，运行时
//! 报 "字段 memory_items.tier 不存在" 错误。
//!
//! 本迁移作为防御性修复：检查 memory_items 表的全部必需列，缺哪个补哪个，
//! 不依赖 v101/v108 是否成功执行过。
//!
//! ## 需要补全的列（与 entities/src/memory_items.rs 对齐）
//!
//! | 列名                    | 类型           | 默认值                  | 来源   |
//! |-------------------------|----------------|-------------------------|--------|
//! | tier                    | TEXT           | 'working'               | v101   |
//! | importance              | REAL           | 0.5                     | v101   |
//! | access_count            | INTEGER        | 0                       | v101   |
//! | last_accessed           | BIGINT         | NULL                    | v101   |
//! | decay_rate              | REAL           | 0.01                    | v101   |
//! | expires_at              | BIGINT         | NULL                    | v101   |
//! | source_conversation_id  | TEXT           | NULL                    | v101   |
//! | source_message_id       | TEXT           | NULL                    | v101   |
//! | memory_nature           | TEXT           | 'semantic'              | v101   |
//! | tags                    | TEXT           | '[]'                    | v101   |
//! | applicability_tags      | TEXT           | '[]'                    | v108   |
//! | confirmed               | INTEGER        | 0                       | v108   |

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // (列名, SQLite ALTER 语句, PG ALTER 语句)
    let columns: &[(&str, &str, &str)] = &[
        (
            "tier",
            "ALTER TABLE memory_items ADD COLUMN tier TEXT NOT NULL DEFAULT 'working'",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS tier TEXT NOT NULL DEFAULT 'working'",
        ),
        (
            "importance",
            "ALTER TABLE memory_items ADD COLUMN importance REAL NOT NULL DEFAULT 0.5",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS importance DOUBLE PRECISION NOT NULL DEFAULT 0.5",
        ),
        (
            "access_count",
            "ALTER TABLE memory_items ADD COLUMN access_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS access_count INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "last_accessed",
            "ALTER TABLE memory_items ADD COLUMN last_accessed BIGINT",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS last_accessed BIGINT",
        ),
        (
            "decay_rate",
            "ALTER TABLE memory_items ADD COLUMN decay_rate REAL NOT NULL DEFAULT 0.01",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS decay_rate DOUBLE PRECISION NOT NULL DEFAULT 0.01",
        ),
        (
            "expires_at",
            "ALTER TABLE memory_items ADD COLUMN expires_at BIGINT",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS expires_at BIGINT",
        ),
        (
            "source_conversation_id",
            "ALTER TABLE memory_items ADD COLUMN source_conversation_id TEXT",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS source_conversation_id TEXT",
        ),
        (
            "source_message_id",
            "ALTER TABLE memory_items ADD COLUMN source_message_id TEXT",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS source_message_id TEXT",
        ),
        (
            "memory_nature",
            "ALTER TABLE memory_items ADD COLUMN memory_nature TEXT NOT NULL DEFAULT 'semantic'",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS memory_nature TEXT NOT NULL DEFAULT 'semantic'",
        ),
        (
            "tags",
            "ALTER TABLE memory_items ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS tags TEXT NOT NULL DEFAULT '[]'",
        ),
        (
            "applicability_tags",
            "ALTER TABLE memory_items ADD COLUMN applicability_tags TEXT DEFAULT '[]'",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS applicability_tags TEXT DEFAULT '[]'",
        ),
        (
            "confirmed",
            "ALTER TABLE memory_items ADD COLUMN confirmed INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS confirmed INTEGER NOT NULL DEFAULT 0",
        ),
    ];

    if is_pg {
        for (_, _, pg_sql) in columns {
            db.execute_unprepared(pg_sql).await?;
        }
    } else {
        let existing_cols = existing_columns(&db, "memory_items").await?;
        for (col_name, sqlite_sql, _) in columns {
            if !existing_cols.iter().any(|c| c == col_name) {
                db.execute_unprepared(sqlite_sql).await?;
            }
        }
    }

    // 确保索引存在（v101 可能漏建）
    for idx_sql in &[
        "CREATE INDEX IF NOT EXISTS idx_memory_items_tier ON memory_items(tier)",
        "CREATE INDEX IF NOT EXISTS idx_memory_items_importance ON memory_items(importance)",
        "CREATE INDEX IF NOT EXISTS idx_memory_items_namespace ON memory_items(namespace_id)",
    ] {
        let _ = db.execute_unprepared(idx_sql).await;
    }

    Ok(())
}

async fn existing_columns(
    db: &sea_orm::DatabaseConnection,
    table: &str,
) -> Result<Vec<String>, DbErr> {
    let backend = db.get_database_backend();
    let rows = match backend {
        DbBackend::Sqlite => {
            let stmt = Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                [table.into()],
            );
            db.query_all_raw(stmt).await?
        },
        DbBackend::Postgres => {
            let stmt = Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT column_name AS name FROM information_schema.columns WHERE table_name = $1",
                [table.into()],
            );
            db.query_all_raw(stmt).await?
        },
        _ => return Ok(vec![]),
    };

    let mut cols = Vec::with_capacity(rows.len());
    for row in rows {
        if let Ok(name) = row.try_get_by::<String, _>("name") {
            cols.push(name);
        }
    }
    Ok(cols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn v109_repairs_missing_columns_on_bare_v100_table() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        let cols = existing_columns(&db, "memory_items").await.unwrap();
        for required in &[
            "tier",
            "importance",
            "access_count",
            "last_accessed",
            "decay_rate",
            "expires_at",
            "source_conversation_id",
            "source_message_id",
            "memory_nature",
            "tags",
            "applicability_tags",
            "confirmed",
        ] {
            assert!(
                cols.iter().any(|c| c == required),
                "column {} should exist after v109",
                required
            );
        }
    }

    #[tokio::test]
    async fn v109_is_idempotent_on_fully_migrated_db() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::run_migrations(&db).await.unwrap();
        up(db.clone()).await.expect("v109 must be re-runnable without error");
    }
}
