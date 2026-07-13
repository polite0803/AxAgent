// SPDX-License-Identifier: AGPL-3.0-only
//! v010_pg_timestamp_int4_to_int8: PostgreSQL 下把时间戳列从 INT4 改 INT8
//!
//! ## 背景
//!
//! 早期 v001-v009 的 DDL 把 `created_at` / `updated_at` 等时间戳列写为 `INTEGER`，
//! SQLite 下无类型问题。Phase A/B/C 引入 PostgreSQL 支持后，PG 的强类型校验报：
//!
//!   ```
//!   Query Error: error occurred while decoding column "created_at":
//!   mismatched types; Rust type `core::option::Option<i64>` (as SQL type `INT8`)
//!   is not compatible with SQL type `INT4`
//!   ```
//!
//! 因为 SeaORM entity 把这些字段声明为 `i64`（`INT8`），但 PG 表里是 `INT4`。
//!
//! v005/v006/v007/v008/v001 的 `pg_ddl()` 已加普通时间戳列 `INTEGER` → `BIGINT` 替换，
//! 保证**新建** PG 实例没问题。本 migration 处理**已存在** PG 实例：把所有 INT4 时间戳
//! 列 `ALTER COLUMN ... TYPE BIGINT`。
//!
//! ## 安全
//!
//! - SQLite 下 no-op（v001-v009 已用 INTEGER，SQLite 是 dynamic typing 无问题）。
//! - PG 下 ALTER COLUMN 不会丢数据（INT4 → INT8 是 widening）。
//! - 使用 `DO $$ ... $$` 块 + `information_schema` 动态判断列类型，
//!   只对**实际为 INT4** 的列执行 ALTER；非 INT4（如 v001_initial 在跑此 migration
//!   之前已重建过 → 已经是 BIGINT）跳过 → 幂等。
//!
//! ## 受影响列（来自 Phase C 实测清单）
//!
//! - providers.created_at / updated_at
//! - provider_keys.created_at
//! - conversations.created_at / updated_at
//! - messages.created_at
//! - gateway_keys.created_at / last_used_at
//! - gateway_usage.created_at
//! - gateway_request_logs.created_at
//! - conversation_summaries.created_at / updated_at
//! - conversation_categories.created_at / updated_at
//! - skill_states.updated_at
//! - wikis.created_at / updated_at
//! - wiki_sources.created_at / updated_at
//! - wiki_pages.created_at / updated_at
//! - wiki_operations.created_at / completed_at
//! - wiki_sync_queue.created_at / processed_at
//! - note_links.created_at
//! - note_backlinks.created_at
//! - plans.created_at / updated_at
//! - wiki_page_versions.created_at
//! - agency_experts.imported_at
//! - index_jobs.created_at / started_at / completed_at
//! - vec_collections.created_at / updated_at / last_indexed_at
//! - dynamic_ui_schema_versions.created_at
//! - credentials.created_at / updated_at
//! - conversations.max_tokens, conversations.thinking_budget
//! - models.max_tokens

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

/// 受影响 (table, column) 列表。所有表都在 v001-v009 范围内已建，
/// 但只有含 `created_at`/`updated_at`/`completed_at`/`processed_at`/
/// `started_at`/`last_used_at`/`last_indexed_at`/`imported_at` 字段
/// 且 entity 类型为 `i64` 的列才需要 ALTER。
///
/// 注意：entity 类型为 `i32`（PG 仍 INT4）的不在列表里（如 rl_policies.total_experiences），
/// 不动。
const ALTER_TARGETS: &[(&str, &str)] = &[
    // providers
    ("providers", "created_at"),
    ("providers", "updated_at"),
    // provider_keys
    ("provider_keys", "created_at"),
    // models
    ("models", "max_tokens"),
    // conversations
    ("conversations", "created_at"),
    ("conversations", "updated_at"),
    ("conversations", "max_tokens"),
    ("conversations", "thinking_budget"),
    // messages
    ("messages", "created_at"),
    // gateway
    ("gateway_keys", "created_at"),
    ("gateway_keys", "last_used_at"),
    ("gateway_usage", "created_at"),
    ("gateway_request_logs", "created_at"),
    // conversation summary / categories
    ("conversation_summaries", "created_at"),
    ("conversation_summaries", "updated_at"),
    ("conversation_categories", "created_at"),
    ("conversation_categories", "updated_at"),
    // skill_states
    ("skill_states", "updated_at"),
    // wiki
    ("wikis", "created_at"),
    ("wikis", "updated_at"),
    ("wiki_sources", "created_at"),
    ("wiki_sources", "updated_at"),
    ("wiki_pages", "created_at"),
    ("wiki_pages", "updated_at"),
    ("wiki_operations", "created_at"),
    ("wiki_operations", "completed_at"),
    ("wiki_sync_queue", "created_at"),
    ("wiki_sync_queue", "processed_at"),
    ("wiki_page_versions", "created_at"),
    ("note_links", "created_at"),
    ("note_backlinks", "created_at"),
    // plans
    ("plans", "created_at"),
    ("plans", "updated_at"),
    // agency_experts
    ("agency_experts", "imported_at"),
    // index_jobs (v005)
    ("index_jobs", "created_at"),
    ("index_jobs", "started_at"),
    ("index_jobs", "completed_at"),
    // vec_collections (v006)
    ("vec_collections", "created_at"),
    ("vec_collections", "updated_at"),
    ("vec_collections", "last_indexed_at"),
    // dynamic_ui_schema_versions (v007)
    ("dynamic_ui_schema_versions", "created_at"),
    // credentials (v008)
    ("credentials", "created_at"),
    ("credentials", "updated_at"),
];

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    // SQLite 无类型问题，no-op
    if backend == DbBackend::Sqlite {
        tracing::info!("[v010] SQLite backend: timestamp INT4→INT8 no-op");
        return Ok(());
    }

    tracing::info!(
        "[v010] PostgreSQL detected: checking {} timestamp column(s) for INT4→INT8 upgrade",
        ALTER_TARGETS.len()
    );

    let mut altered = 0;
    let mut skipped = 0;
    let mut missing = 0;

    for (table, column) in ALTER_TARGETS {
        // 用 information_schema 查列类型：
        //   data_type = 'integer'  → INT4，需要 ALTER
        //   data_type = 'bigint'   → 已是 INT8，跳过
        //   不存在                → 表/列还没建（旧 DB 不完整），跳过
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
                // 列不存在（可能 v005/v006/v007/v008 未跑过；或表被删），安全跳过
                missing += 1;
            },
            Some(r) => {
                let data_type: Option<String> = r.try_get_by("data_type").ok();
                match data_type.as_deref() {
                    Some("integer") => {
                        // INT4 → INT8。USING ::bigint 处理可能的列已有数据
                        // （i32 范围内的 Unix timestamp 秒数可在 INT8 下精确表示，安全 widening）。
                        let alter_sql = format!(
                            "ALTER TABLE {table} \
                             ALTER COLUMN {column} TYPE BIGINT USING {column}::bigint"
                        );
                        db.execute_unprepared(&alter_sql).await?;
                        altered += 1;
                        tracing::info!("[v010] {table}.{column}: INT4 → INT8");
                    },
                    Some("bigint") => {
                        // 已是 INT8，幂等跳过
                        skipped += 1;
                    },
                    Some(other) => {
                        // 其它类型（TEXT/TIMESTAMP 等）：不在本 migration 范围，
                        // 可能是 entity 改了类型，跳过不打扰
                        tracing::debug!(
                            "[v010] {table}.{column}: unexpected type '{other}', skipping"
                        );
                        skipped += 1;
                    },
                    None => {
                        skipped += 1;
                    },
                }
            },
        }
    }

    tracing::info!(
        "[v010] done: {} altered, {} skipped (already INT8 or wrong type), {} missing (table/column not present)",
        altered,
        skipped,
        missing
    );

    Ok(())
}
