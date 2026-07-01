// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
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
