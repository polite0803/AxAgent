// SPDX-License-Identifier: AGPL-3.0-only
//! v208_backfill_wiki_sync_queue_columns: 补全 wiki_sync_queue 表缺失的
//! created_at / processed_at 列。
//!
//! ## 背景
//!
//! v100 PHASE 3.9 的 `ADDITIONAL_COLUMNS` 中包含
//! `("wiki_sync_queue", "created_at", "BIGINT")` 和
//! `("wiki_sync_queue", "processed_at", "BIGINT")`，但若数据库在 v100
//! 迁移已应用 **之后** 才向 `ADDITIONAL_COLUMNS` 中添加这两列，
//! PHASE 3.9 不会再跑，导致存量库的 `wiki_sync_queue` 表缺少 `created_at`
//! 列。运行时 SeaORM 查询 `wiki_sync_queue::Column::CreatedAt` 报错：
//!
//!   `字段 "created_at" 不存在`
//!
//! ## 策略
//!
//! 与 PHASE 3.9 相同的防御性检查：
//!   - PG: `information_schema.columns` 查缺 → `ALTER TABLE ADD COLUMN`
//!   - SQLite: `pragma_table_info` 查缺 → `ALTER TABLE ADD COLUMN`
//!
//! 本迁移也作为后续类似表列缺失的通用修复模板。
//!
//! ## 幂等性
//!
//! `IF NOT EXISTS` 在 ALTER TABLE 中 PG 9.4+ 原生支持，
//! SQLite 3.38+ 支持。本迁移通过显式检查列存在性实现幂等，
//! 不依赖数据库版本特性。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

/// 需要补全的 (表名, 列名, SQL 类型定义)
const MISSING_COLUMNS: &[(&str, &str, &str)] = &[
    ("wiki_sync_queue", "created_at", "BIGINT NOT NULL DEFAULT 0"),
    ("wiki_sync_queue", "processed_at", "BIGINT"),
];

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    let mut added = 0usize;
    let mut already = 0usize;

    for (table, column, col_type) in MISSING_COLUMNS {
        let exists = if is_pg {
            let row = db
                .query_one_raw(Statement::from_string(
                    DbBackend::Postgres,
                    format!(
                        "SELECT 1 AS exists_flag FROM information_schema.columns \
                         WHERE table_schema = current_schema() \
                           AND table_name = '{table}' AND column_name = '{column}'"
                    ),
                ))
                .await?;
            row.is_some()
        } else {
            let rows = db
                .query_all_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM pragma_table_info(?)",
                    [(*table).into()],
                ))
                .await?;
            rows.iter()
                .any(|r| r.try_get_by::<String, _>("name").map(|n| n == *column).unwrap_or(false))
        };

        if exists {
            already += 1;
            continue;
        }

        let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {col_type}");
        db.execute_unprepared(&sql).await?;
        tracing::info!("[v208] 补列: {table}.{column} ({col_type})");
        added += 1;
    }

    tracing::info!(
        "[v208] 字段合规: {added} ADD, {already} 已存在 (共 {total})",
        added = added,
        already = already,
        total = MISSING_COLUMNS.len(),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v208_is_self_idempotent() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        // 先跑 v100 建表
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑应不报错（幂等）
        up(db).await.expect("v208 必须可重复执行");
    }

    #[tokio::test]
    async fn v208_backfills_on_bare_table() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        // 只建 wiki_sync_queue 表（不包含 created_at/processed_at，模拟存量库）
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS wiki_sync_queue (\
             id INTEGER PRIMARY KEY AUTOINCREMENT, \
             wiki_id TEXT NOT NULL, \
             event_type TEXT NOT NULL, \
             target_type TEXT NOT NULL, \
             target_id TEXT NOT NULL, \
             payload TEXT, \
             status TEXT NOT NULL DEFAULT 'pending', \
             retry_count INTEGER NOT NULL DEFAULT 0, \
             error_message TEXT)",
        )
        .await
        .unwrap();

        // 验证表已建但缺列
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                ["wiki_sync_queue".into()],
            ))
            .await
            .unwrap();
        let col_names: Vec<String> =
            rows.iter().filter_map(|r| r.try_get_by::<String, _>("name").ok()).collect();
        assert!(!col_names.contains(&"created_at".to_string()), "created_at 应缺失");
        assert!(!col_names.contains(&"processed_at".to_string()), "processed_at 应缺失");

        // 跑 v208 补列
        up(db.clone()).await.unwrap();

        // 验证列已补全
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                ["wiki_sync_queue".into()],
            ))
            .await
            .unwrap();
        let col_names: Vec<String> =
            rows.iter().filter_map(|r| r.try_get_by::<String, _>("name").ok()).collect();
        assert!(col_names.contains(&"created_at".to_string()), "created_at 应存在");
        assert!(col_names.contains(&"processed_at".to_string()), "processed_at 应存在");
    }
}
