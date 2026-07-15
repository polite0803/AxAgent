// SPDX-License-Identifier: AGPL-3.0-only
//! v200_pg_int4_to_int8_axinvest: AxInvest 合并迁移，处理 PG 下 INT4 → INT8
//!
//! ## 背景
//!
//! 合并自历史 `v014_pg_timestamp_int4_to_int8` + `v015_pg_business_int4_to_int8`：
//!   - 早期 v001–v009 的 DDL 把 `created_at` / `updated_at` 等时间戳列写为 `INTEGER`，
//!     SQLite 下无类型问题。
//!   - Phase A/B/C 引入 PostgreSQL 支持后，PG 强类型校验报：
//!     `Rust type Option<i64> (as SQL type INT8) is not compatible with SQL type INT4`
//!     因为 SeaORM entity 把这些字段声明为 `i64`，但 PG 表里是 `INT4`。
//!
//! 上游 `v100_consolidated` 包含一份更大的全量 `ALTER_TARGETS` 列表，理论上
//! 已覆盖本文件所有目标。本 v200 作为 AxInvest 兜底迁移保留：若未来上游
//! 移除某些列，本迁移可独立保证 AxInvest 关键业务列（时间戳 + max_tokens /
//! thinking_budget）的类型正确性。
//!
//! ## 版本号策略
//!
//! AxInvest 本地迁移从 **v200** 起编号，预留 v101–v199 给上游 AxAgent 未来
//! 新增迁移使用，避免合并上游时版本号冲突。本文件原名 `v101_pg_int4_to_int8_axinvest.rs`，
//! 现重命名为 v200 以落实该策略。
//!
//! ## 安全
//!
//! - SQLite 下 no-op（v001–v009 已用 INTEGER，SQLite 是 dynamic typing 无问题）。
//! - PG 下 ALTER COLUMN 不会丢数据（INT4 → INT8 是 widening）。
//! - 使用 `information_schema` 动态判断列类型，只对实际为 INT4 的列执行 ALTER；
//!   已是 BIGINT 或不存在 → 跳过 → 幂等。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

/// 受影响 (table, column) 列表。合并 v014（时间戳）+ v015（业务列 max_tokens / thinking_budget）。
///
/// 仅保留 AxInvest 关键业务列；上游 v100 已覆盖更多列，本 v200 作为独立兜底。
const ALTER_TARGETS: &[(&str, &str)] = &[
    // ======== 时间戳列（来自 v014）========
    ("providers", "created_at"),
    ("providers", "updated_at"),
    ("provider_keys", "created_at"),
    ("conversations", "created_at"),
    ("conversations", "updated_at"),
    ("messages", "created_at"),
    ("gateway_keys", "created_at"),
    ("gateway_keys", "last_used_at"),
    ("gateway_usage", "created_at"),
    ("gateway_request_logs", "created_at"),
    ("conversation_summaries", "created_at"),
    ("conversation_summaries", "updated_at"),
    ("conversation_categories", "created_at"),
    ("conversation_categories", "updated_at"),
    ("skill_states", "updated_at"),
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
    ("plans", "created_at"),
    ("plans", "updated_at"),
    ("agency_experts", "imported_at"),
    ("index_jobs", "created_at"),
    ("index_jobs", "started_at"),
    ("index_jobs", "completed_at"),
    ("vec_collections", "created_at"),
    ("vec_collections", "updated_at"),
    ("vec_collections", "last_indexed_at"),
    ("dynamic_ui_schema_versions", "created_at"),
    ("credentials", "created_at"),
    ("credentials", "updated_at"),
    // ======== 业务列（来自 v015）========
    ("models", "max_tokens"),
    ("conversations", "max_tokens"),
    ("conversations", "thinking_budget"),
];

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    // SQLite 无类型问题，no-op
    if backend == DbBackend::Sqlite {
        tracing::info!("[v200] SQLite backend: INT4→INT8 no-op");
        return Ok(());
    }

    tracing::info!(
        "[v200] PostgreSQL detected: checking {} column(s) for INT4→INT8 upgrade",
        ALTER_TARGETS.len()
    );

    let mut altered = 0;
    let mut skipped = 0;
    let mut missing = 0;

    for (table, column) in ALTER_TARGETS {
        // 用 information_schema 查列类型：
        //   data_type = 'integer'  → INT4，需要 ALTER
        //   data_type = 'bigint'   → 已是 INT8，跳过
        //   不存在                  → 表/列还没建（旧 DB 不完整），跳过
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
                // 列不存在（可能上游 v100 还未跑过；或表被删），安全跳过
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
                        tracing::info!("[v200] {table}.{column}: INT4 → INT8");
                    },
                    Some("bigint") => {
                        // 已是 INT8，幂等跳过
                        skipped += 1;
                    },
                    Some(other) => {
                        // 其它类型（TEXT/TIMESTAMP 等）：不在本 migration 范围
                        tracing::debug!(
                            "[v200] {table}.{column}: unexpected type '{other}', skipping"
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
        "[v200] done: {} altered, {} skipped (already INT8 or wrong type), {} missing (table/column not present)",
        altered,
        skipped,
        missing
    );

    Ok(())
}
