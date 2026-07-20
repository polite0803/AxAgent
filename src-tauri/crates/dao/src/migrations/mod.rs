// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned schema migration framework.
//!
//! ## 当前状态
//!
//! 本项目采用「单一基线迁移」架构：所有 DDL（表/索引/触发器/种子数据）都
//! 合并在 [v100_consolidated] 一个文件中。新装库只需跑一次 v100，类型即正确。
//!
//! ## 历史
//!
//! - 旧版采用 v001–v011 + v101–v103 + v200 多版本迁移，导致：
//!   - v100 后续追加的 PHASE 在已应用 v100 的旧库上永远不会跑
//!   - v200 INT4→INT8 ALTER 通道与 v100 pg_ddl() 类型转换重复
//!   - 多次数据转换逻辑相互覆盖，难以维护
//! - 现已合并为 v100 单一基线：
//!   - v101_business_roles / v102_mission_hash / v103_workflow_reflections
//!     的字段和表全部合并到 v100 PHASE 2/8/9/10
//!   - v200_pg_int4_to_int8_axinvest 删除（pg_ddl 在 CREATE TABLE 时一次性
//!     产出正确类型，无需二次 ALTER）
//!   - 所有 ALTER TABLE 补字段通道删除（字段直接在 CREATE TABLE 中建好）
//!
//! ## 约定
//!
//! - 新增表/字段：直接修改 v100_consolidated.rs 的 PHASE 2 CREATE TABLE
//! - 新增索引：在 v100 PHASE 4 中添加
//! - 不再创建新的迁移版本（除非有破坏性 schema 变更需要独立追踪）

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub mod pg_ddl;
pub mod v100_consolidated;

/// 当前 schema 版本号。
pub const CURRENT_VERSION: i32 = 100;

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

const MIGRATIONS: &[Migration] = &[Migration {
    version: 100,
    description: "v100_consolidated: 合并 v001–v011 + v101–v103 + v200 的全部 DDL（表/索引/触发器/种子数据），统一用正确类型建表；不再保留旧库类型修复 ALTER 通道",
    up: |db| Box::pin(v100_consolidated::up(db)),
}];

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
