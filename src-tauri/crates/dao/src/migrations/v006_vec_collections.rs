// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // 走 exec_ddl：PG 下经 pg_ddl() 把 INTEGER 时间戳列转 BIGINT
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS vec_collections (\
         collection_id TEXT NOT NULL PRIMARY KEY, \
         dimensions INTEGER NOT NULL, \
         embedding_model TEXT, \
         index_type TEXT NOT NULL DEFAULT 'flat', \
         hnsw_ef_construction INTEGER, \
         hnsw_m INTEGER, \
         hnsw_ef_search INTEGER, \
         vector_count INTEGER NOT NULL DEFAULT 0, \
         created_at INTEGER NOT NULL, \
         updated_at INTEGER NOT NULL, \
         last_indexed_at INTEGER, \
         metadata TEXT)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_vec_collections_model \
         ON vec_collections (embedding_model)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_vec_collections_updated \
         ON vec_collections (updated_at DESC)",
    )
    .await?;

    Ok(())
}
