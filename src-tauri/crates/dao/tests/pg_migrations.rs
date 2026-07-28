// SPDX-License-Identifier: AGPL-3.0-only
//! 真机 PostgreSQL 迁移验证（env-gated）。
//!
//! 仅当设置 `AXAGENT_TEST_PG_URL` 时运行（URL 需含库名，例如
//! `postgres://postgres:PASS@localhost:5432/axagent`）。否则测试整体跳过，
//! 不影响普通 `cargo test` 与 CI。
//!
//! 测试动作：
//! 1. 连维护库（`/postgres`），幂等 DROP+CREATE 专用测试库 `axagent_pg_migtest`；
//! 2. 在其上跑完整 `run_migrations`（v100 consolidated）；
//! 3. 断言核心表、tsvector 生成列、GIN 索引、`schema_version = 100`；
//! 4. 插入会话+消息，跑 PG 全文检索 SQL，验证 tsvector 真实生效。
//!
//! 用法：
//! ```sh
//! AXAGENT_TEST_PG_URL=postgres://postgres:PASS@localhost:5432/axagent \
//!   cargo test -p axagent-dao --test pg_migrations
//! ```

use axagent_dao::migrations::{SCHEMA_VERSION_TABLE, run_migrations};
use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

const TEST_DB: &str = "axagent_pg_migtest";

/// 把 `postgres://user:pass@host:port/olddb` 的库名替换成 `db`。
fn with_db(url: &str, db: &str) -> String {
    let at = url.rfind('@').expect("pg url must contain '@'");
    let (head, tail) = url.split_at(at + 1); // head = "...@", tail = "host:port/olddb"
    let slash = tail.find('/').map(|i| i + 1).unwrap_or(tail.len());
    format!("{}{}{}", head, &tail[..slash], db)
}

#[tokio::test]
async fn pg_migrations_apply_and_search_works() {
    let base = match std::env::var("AXAGENT_TEST_PG_URL") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("AXAGENT_TEST_PG_URL not set; skipping PG migration test");
            return;
        },
    };

    let maint_url = with_db(&base, "postgres");
    let test_url = with_db(&base, TEST_DB);

    // 1) 维护库：清空并重建专用测试库（保证从零跑迁移，暴露任何非幂等 DDL）
    let maint = Database::connect(&maint_url).await.expect("connect maintenance db");
    let _ = maint.execute_unprepared(&format!("DROP DATABASE IF EXISTS {TEST_DB}")).await;
    maint
        .execute_unprepared(&format!("CREATE DATABASE {TEST_DB}"))
        .await
        .expect("create test database");

    // 2) 跑完整迁移链
    let db = Database::connect(&test_url).await.expect("connect test db");
    run_migrations(&db).await.expect("run_migrations should succeed on PostgreSQL");

    // 3a) 核心表存在
    let tables = [
        "messages",
        "conversations",
        "providers",
        "provider_keys",
        "gateway_keys",
        "gateway_usage",
        SCHEMA_VERSION_TABLE,
        "trajectory_trajectories",
        "trajectory_memories",
        "wiki_page_versions",
        "dynamic_ui_schema_versions",
        "route_history",
    ];
    for t in tables {
        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT 1 FROM information_schema.tables WHERE table_name = $1",
                [t.into()],
            ))
            .await
            .unwrap();
        assert!(row.is_some(), "table {t} should exist on PostgreSQL");
    }

    // 3b) tsvector 生成列存在（GENERATED ALWAYS AS STORED 替代 FTS5）
    let tsv_cols = [
        "messages.content_tsv",
        "trajectory_trajectories.tsv",
        "trajectory_memories.tsv",
        "trajectory_skills.tsv",
        "trajectory_messages.tsv",
    ];
    for col in tsv_cols {
        let (table, column) = col.split_once('.').unwrap();
        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT 1 FROM information_schema.columns \
                 WHERE table_name = $1 AND column_name = $2",
                [table.into(), column.into()],
            ))
            .await
            .unwrap();
        assert!(row.is_some(), "tsvector column {col} should exist on PostgreSQL");
    }

    // 3c) schema_version 应只有 1 行，MAX(version) = 100
    //     （v100 consolidated 合并了 v100-v104 全部迁移）
    let max_row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("SELECT COALESCE(MAX(version), 0)::int AS v FROM {SCHEMA_VERSION_TABLE}"),
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .unwrap()
        .expect("max version row");
    let max_v: i32 = max_row.try_get_by("v").unwrap();
    assert_eq!(max_v, 100, "schema version should be 100 (v100 consolidated), got {max_v}");

    // 4) 端到端：插入会话+消息，跑 PG 全文检索，验证 tsvector 真实生效
    db.execute_unprepared(
        "INSERT INTO conversations (id, title, model_id, provider_id, created_at, updated_at) \
         VALUES ('c1', 't', 'm', 'p', 0, 0)",
    )
    .await
    .expect("insert conversation");

    db.execute_unprepared(
        "INSERT INTO messages (id, conversation_id, role, content, created_at) \
         VALUES ('m1', 'c1', 'user', 'postgresql migration verification token', 0)",
    )
    .await
    .expect("insert message");

    let found = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT m.conversation_id \
             FROM messages m \
             WHERE m.content_tsv @@ plainto_tsquery('simple', $1) \
             LIMIT 1",
            ["postgresql".into()],
        ))
        .await
        .unwrap();
    assert!(found.is_some(), "PostgreSQL tsvector search should match the inserted message");
}
