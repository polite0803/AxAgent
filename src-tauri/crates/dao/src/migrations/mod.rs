// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned schema migration framework.
//!
//! Phase 2 引入的迁移系统：取代旧 `ddl.rs::run_initialization` 的 "drop
//! seaql_migrations + 全量重建" 模式。`run_migrations` 启动时读
//! `axagent_schema_version.MAX(version)`，按顺序补跑未应用迁移，每条
//! 完成后写入版本号 → 幂等 + 可重启 + 任何后续 ALTER 都可以版本化追踪。
//!
//! ## 约定
//!
//! - 版本号单调递增；每条 migration 写一个文件 `vNNN_xxx.rs`。
//! - 每个 `up()` 必须自带 `CREATE ... IF NOT EXISTS` 等幂等保护，
//!   或显式检查 schema_version 防重复跑。
//! - 新加 migration 时：1) 创建文件，2) 在 `MIGRATIONS` 数组中注册，
//!   3) 在 `CURRENT_VERSION` 累加。
//!
//! ## 向后兼容
//!
//! `ddl.rs::run_initialization` 现在是 thin shim，直接转发到
//! `migrations::run_migrations`，确保所有已有 call sites 继续工作。
//! 旧 DROP seaql_migrations 行为已删除（无 seaql 依赖）。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

// ============================================================================
// v100 是合并后的唯一迁移。历史 v001–v011 已合并至此文件，旧文件已删除。
// 所有未来变更直接加在 v100 上或创建新版本。
//
// ============================================================================
// AxInvest 版本号策略：本地迁移从 v200 起编号
// ============================================================================
// - v100：上游 AxAgent 合并迁移（来自 ecba666de `refactor(dao): consolidate
//   v001–v011 → v100 merged migration`），上游 master 当前最新版本号。
// - v101–v199：预留段。留给上游 AxAgent 未来新增迁移使用，避免合并上游时
//   版本号冲突。AxInvest 新增迁移一律跳过本段。
// - v200+：AxInvest 本地迁移段。所有 AxInvest 独有的 schema 变更从这里
//   起单调递增。新增 v201/v202/... 时仅累加 CURRENT_VERSION 即可。
// ============================================================================

pub mod pg_ddl;
pub mod v100_consolidated;
pub mod v200_pg_int4_to_int8_axinvest;

/// 当前 schema 版本号。每次新增 migration 时必须累加此常量。
pub const CURRENT_VERSION: i32 = 200;

/// 迁移函数签名：所有 `up()` 都遵循这个接口。
///
/// `DatabaseConnection` 是 `Arc<DbConnection>` 的 newtype，clone
/// 是引用计数 +1，零拷贝。所以 `up` 接收 owned 是 trivial 的：
/// 调用方在每次 invoke 时 clone 一份即可。
///
/// 用 owned 而非 `&DatabaseConnection` 是为了让 boxed future 不带
/// 借用——`Pin<Box<dyn Future + 'static>>` 可以装进 `const MIGRATIONS`
/// 数组（fn pointer 自身要求 'static）。
///
/// `Send` 是为了让 `run_migrations` 能在 multi-threaded runtime 中
/// 被调用（生产环境 `tokio::main` 默认是 multi_thread）。不需要
/// `Sync`：future 只在 await 期间被一个 task 持有，不存在共享。
pub type MigrationFn =
    fn(
        sea_orm::DatabaseConnection,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), DbErr>> + Send>>;

struct Migration {
    version: i32,
    description: &'static str,
    up: MigrationFn,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 100,
        description: "v100_consolidated: 合并 v001–v011 + v101–v104 的全部 DDL（表/索引/触发器/种子数据），统一用正确类型建表；不再保留旧库类型修复 ALTER 通道",
        up: |db| Box::pin(v100_consolidated::up(db)),
    },
    Migration {
        version: 200,
        description: "v200_pg_int4_to_int8_axinvest: AxInvest-targeted INT4→INT8 fix for timestamps + max_tokens/thinking_budget",
        up: |db| Box::pin(v200_pg_int4_to_int8_axinvest::up(db)),
    },
];

/// 执行所有尚未应用的 schema 迁移。
///
/// 启动时调用；幂等，多次调用结果相同。
///
/// 第一步（建 version tracking 表、读 MAX(version)）使用 `&impl
/// ConnectionTrait`——这是 ConnectionTrait 的稳定接口，ddl.rs shim
/// 可以直接转发。第二步（实际跑 up()）需要 `&DatabaseConnection`，
/// 所以顶层 API 接收 `&DatabaseConnection`；ddl.rs shim 已经更新
/// 成强类型。
pub async fn run_migrations(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    // 1) 确保 version tracking 表存在（ANSI DDL，SQLite/PG 通用）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS axagent_schema_version (\
         version INTEGER NOT NULL PRIMARY KEY, \
         applied_at INTEGER NOT NULL, \
         description TEXT)",
    )
    .await?;

    // 2) 读已应用的最大版本号（首次启动 = 0）
    let applied_max: i32 = read_max_version(db).await?;

    // 3) 按顺序补跑未应用 migration
    for m in MIGRATIONS {
        if m.version <= applied_max {
            continue;
        }
        // db.clone() 是 Arc +1，up() 内部 await 时持有一个 owned 副本。
        (m.up)(db.clone()).await?;
        record_version(db, backend, m.version, m.description).await?;
    }

    Ok(())
}

async fn read_max_version(db: &sea_orm::DatabaseConnection) -> Result<i32, DbErr> {
    let row = db
        .query_one_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT COALESCE(MAX(version), 0) AS v FROM axagent_schema_version",
        ))
        .await?;
    match row {
        None => Ok(0),
        Some(r) => {
            // COALESCE 在空表返回 0，因此总能解析为 i32
            let v: i32 = r.try_get_by("v").unwrap_or(0);
            Ok(v)
        },
    }
}

async fn record_version(
    db: &sea_orm::DatabaseConnection,
    backend: DbBackend,
    version: i32,
    description: &str,
) -> Result<(), DbErr> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);

    // 参数化查询：避免 format! 拼接 SQL 带来的注入风险与转义负担。
    // SQLite 用 `INSERT OR IGNORE`；PostgreSQL 用 `ON CONFLICT DO NOTHING`
    // （二者语义等价：版本号冲突时静默跳过，保证幂等）。
    let stmt = if backend == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO axagent_schema_version (version, applied_at, description) \
             VALUES ($1, $2, $3) ON CONFLICT (version) DO NOTHING",
            [version.into(), now.into(), description.into()],
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT OR IGNORE INTO axagent_schema_version (version, applied_at, description) VALUES (?, ?, ?)",
            [version.into(), now.into(), description.into()],
        )
    };
    db.execute_raw(stmt).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn migrations_apply_cleanly_on_fresh_db() {
        let db = Database::connect("sqlite::memory:").await.expect("in-memory db");
        run_migrations(&db).await.expect("v1-v3 should apply on fresh db");

        // 验证关键表存在
        for table in &[
            "messages",
            "conversations",
            "providers",
            "provider_keys",
            "gateway_keys",
            "gateway_usage",
            "axagent_schema_version",
        ] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                    [(*table).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "table {} should exist", table);
        }

        // 死表应已被 v003 删除
        for dead in &["categories", "apps", "context_packs"] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                    [(*dead).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_none(), "dead table {} should have been dropped", dead);
        }
    }

    #[tokio::test]
    async fn migrations_are_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("in-memory db");
        run_migrations(&db).await.unwrap();
        // 第二次跑：所有 migration 都在 `applied_max >= m.version` 路径被 skip
        run_migrations(&db).await.expect("second run should be a no-op, not an error");

        let max: i32 = read_max_version(&db).await.unwrap();
        assert_eq!(max, CURRENT_VERSION, "version should be {}", CURRENT_VERSION);

        // schema_version 表应有 1 行（v100 已合并 v001–v011 + v101–v104 全部 DDL）
        let count_row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT COUNT(*) AS cnt FROM axagent_schema_version",
            ))
            .await
            .unwrap()
            .expect("count row");
        let cnt: i32 = count_row.try_get_by("cnt").unwrap();
        assert_eq!(cnt, 1, "schema_version should have exactly 1 row (v100 only)");
    }

    /// 防回归：v002 引入的索引必须真实存在。
    /// partial index (`idx_messages_branch`) 在 messages.branch_id IS NOT NULL
    /// 命中时使用。
    /// 注：v002 已被合并到 v100_consolidated，索引由 PHASE 4 创建。
    #[tokio::test]
    async fn v002_critical_indices_exist() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        for idx in &[
            "idx_messages_conv_created",
            "idx_conversations_updated",
            "idx_provider_keys_provider",
            "idx_gateway_usage_key",
            "idx_messages_branch",
        ] {
            let row = db
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='index' AND name=?",
                    [(*idx).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "index {} should exist", idx);
        }
    }

    /// v100 consolidated 的 `up` 也应单独 idempotent：单独跑
    /// 一次，重复跑不报错（所有 CREATE 都用 IF NOT EXISTS）。
    #[tokio::test]
    async fn v100_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 不走 run_migrations，直接跑 v100
        v100_consolidated::up(db.clone()).await.unwrap();
        v100_consolidated::up(db).await.expect("v100 must be re-runnable in isolation");
    }
}
