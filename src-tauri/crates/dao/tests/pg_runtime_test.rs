// SPDX-License-Identifier: AGPL-3.0-only
//! PostgreSQL 迁移可移植性回归测试（可选）
//!
//! 默认 `cargo test` 不运行：需设置 `AXINVEST_PG_TEST=1` 才执行，避免 CI
//! 无 PG 时失败。本测试针对上游"设置切换数据库连接 + 重启生效"后的双后端
//! 改造做真实验证：
//!
//! 在本地 PostgreSQL（`axinvest_pg_test` 库）跑全量迁移 v001..v013，确认
//! 我们改过的 v010/v011/v012/v013 在 PG 下不再有 SQLite 专属语法报错，且
//! 类型/约束正确：
//! - v010：时间戳列应为 BIGINT（防 PG int4 溢出）
//! - v011：stock_analyses.node_results_snapshot 列应已删除
//! - v012：news_archive.article_code 应为 NOT NULL
//! - v013：stock_pipeline_runs 应已建表且 created_at 为 BIGINT（含默认值）

use axagent_dao::migrations;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};

const PG_URL: &str =
    "postgres://postgres:Hjdssyqsyl410@127.0.0.1:5432/axinvest_pg_test?sslmode=disable";

async fn col_type(db: &sea_orm::DatabaseConnection, table: &str, column: &str) -> Option<String> {
    let sql = format!(
        "SELECT data_type FROM information_schema.columns \
         WHERE table_schema='public' AND table_name='{table}' AND column_name='{column}'"
    );
    let row =
        db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql)).await.unwrap();
    row.and_then(|r| r.try_get_by::<String, _>("data_type").ok())
}

async fn col_exists(db: &sea_orm::DatabaseConnection, table: &str, column: &str) -> bool {
    col_type(db, table, column).await.is_some()
}

async fn col_is_not_null(db: &sea_orm::DatabaseConnection, table: &str, column: &str) -> bool {
    let sql = format!(
        "SELECT is_nullable FROM information_schema.columns \
         WHERE table_schema='public' AND table_name='{table}' AND column_name='{column}'"
    );
    let row =
        db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql)).await.unwrap();
    matches!(row.and_then(|r| r.try_get_by::<String, _>("is_nullable").ok()).as_deref(), Some("NO"))
}

async fn table_exists(db: &sea_orm::DatabaseConnection, table: &str) -> bool {
    let sql = format!(
        "SELECT 1 FROM information_schema.tables \
         WHERE table_schema='public' AND table_name='{table}'"
    );
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .unwrap()
        .is_some()
}

#[tokio::test]
async fn pg_runtime_migration_portability() {
    if std::env::var("AXINVEST_PG_TEST").is_err() {
        eprintln!("AXINVEST_PG_TEST not set; skipping PG runtime test");
        return;
    }

    let db = Database::connect(PG_URL).await.expect("connect to local PostgreSQL test DB");

    // 干净起点：重建 public schema（仅测试库，不影响其他库）
    db.execute_unprepared("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .await
        .expect("reset public schema for clean migration run");

    // 1) 跑全量迁移 v001..v013 —— 验证双后端改造在 PG 下无 SQLite 专属语法报错
    migrations::run_migrations(&db).await.expect("run_migrations must succeed on PostgreSQL");

    // 2) v010：时间戳列应为 BIGINT（防 PG int4 溢出）
    assert_eq!(
        col_type(&db, "stock_analyses", "created_at").await.as_deref(),
        Some("bigint"),
        "stock_analyses.created_at must be BIGINT on PG"
    );
    assert_eq!(
        col_type(&db, "news_archive", "publish_time").await.as_deref(),
        Some("bigint"),
        "news_archive.publish_time must be BIGINT on PG"
    );
    assert!(table_exists(&db, "stock_pipeline_runs").await, "stock_pipeline_runs must exist");

    // 3) v011：node_results_snapshot 列应已删除
    assert!(
        !col_exists(&db, "stock_analyses", "node_results_snapshot").await,
        "v011 should have dropped node_results_snapshot column"
    );

    // 4) v012：news_archive.article_code 应为 NOT NULL
    assert!(
        col_is_not_null(&db, "news_archive", "article_code").await,
        "v012 should have made article_code NOT NULL"
    );

    // 5) v013：created_at 应为 BIGINT 且有默认值
    assert_eq!(
        col_type(&db, "stock_pipeline_runs", "created_at").await.as_deref(),
        Some("bigint"),
        "stock_pipeline_runs.created_at must be BIGINT on PG"
    );

    println!(
        "PG runtime migration portability test PASSED: v001..v013 applied cleanly on PostgreSQL"
    );
}
