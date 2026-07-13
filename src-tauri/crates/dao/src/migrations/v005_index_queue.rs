// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // 走 exec_ddl：PG 下经 pg_ddl() 把 INTEGER 时间戳列转 BIGINT（见 pg_ddl.rs 注释）
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS index_jobs (\
         id TEXT NOT NULL PRIMARY KEY, \
         job_type TEXT NOT NULL, \
         container_type TEXT NOT NULL, \
         container_id TEXT NOT NULL, \
         item_id TEXT NOT NULL, \
         status TEXT NOT NULL DEFAULT 'pending', \
         current_stage TEXT, \
         progress INTEGER NOT NULL DEFAULT 0, \
         error_message TEXT, \
         retry_count INTEGER NOT NULL DEFAULT 0, \
         max_retries INTEGER NOT NULL DEFAULT 3, \
         priority INTEGER NOT NULL DEFAULT 0, \
         created_at INTEGER NOT NULL, \
         started_at INTEGER, \
         completed_at INTEGER, \
         metadata TEXT)",
    )
    .await?;

    // 索引列名也含 `created_at` 但属于 DDL 子句结构，pg_ddl 不动这些
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_index_jobs_status \
         ON index_jobs (status, priority DESC, created_at ASC)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_index_jobs_container \
         ON index_jobs (container_type, container_id)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_index_jobs_item \
         ON index_jobs (container_type, item_id)",
    )
    .await?;

    Ok(())
}
