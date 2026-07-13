// SPDX-License-Identifier: AGPL-3.0-only
//! v011_pg_business_int4_to_int8: 补修 v010 漏掉的若干业务列 INT4 → INT8
//!
//! ## 背景
//!
//! v010 只覆盖了 `created_at` / `updated_at` 等时间戳列，但 `models.max_tokens`、
//! `conversations.max_tokens`、`conversations.thinking_budget` 在 DDL 也是 `INTEGER`
//! (INT4) 而 SeaORM entity 类型是 `i64` (INT8)。PG 启动 init 时查询这些表报错：
//!
//!   ```
//!   Rust type `core::option::Option<i64>` (as SQL type `INT8`)
//!   is not compatible with SQL type `INT4`
//!   ```
//!
//! 本 migration 只处理 v010 未覆盖的业务列。时间戳列已在 v010 处理完毕。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

/// 需要在已存在 PG 实例上 ALTER 的业务列（时间戳列已由 v010 处理）。
const ALTER_TARGETS: &[(&str, &str)] = &[
    // models.max_tokens
    ("models", "max_tokens"),
    // conversations.max_tokens / thinking_budget
    ("conversations", "max_tokens"),
    ("conversations", "thinking_budget"),
];

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    if backend == DbBackend::Sqlite {
        tracing::info!("[v011] SQLite backend: business INT4→INT8 no-op");
        return Ok(());
    }

    tracing::info!("[v011] Checking {} business column(s) for INT4→INT8 upgrade", ALTER_TARGETS.len());

    let mut altered = 0;
    let mut skipped = 0;
    let mut missing = 0;

    for (table, column) in ALTER_TARGETS {
        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                backend,
                format!(
                    "SELECT data_type FROM information_schema.columns \
                     WHERE table_schema = current_schema() \
                       AND table_name = '{table}' AND column_name = '{column}'"
                ),
            ))
            .await?;

        match row {
            None => {
                missing += 1;
            },
            Some(r) => {
                let data_type: Option<String> = r.try_get_by("data_type").ok();
                match data_type.as_deref() {
                    Some("integer") => {
                        let sql = format!(
                            "ALTER TABLE {table} \
                             ALTER COLUMN {column} TYPE BIGINT USING {column}::bigint"
                        );
                        db.execute_unprepared(&sql).await?;
                        altered += 1;
                        tracing::info!("[v011] {table}.{column}: INT4 → INT8");
                    },
                    Some("bigint") => {
                        skipped += 1;
                    },
                    _ => {
                        skipped += 1;
                    },
                }
            },
        }
    }

    tracing::info!(
        "[v011] done: {} altered, {} skipped, {} missing",
        altered, skipped, missing
    );

    Ok(())
}
