// SPDX-License-Identifier: AGPL-3.0-only
//! v114_wiki_sources_schedule: 为 wiki_sources 表添加调度字段，支撑知识库增长更新入口。
//!
//! ## 背景
//!
//! 「知识库增长更新入口」（docs/knowledge-source-ingest-plan.md）将 wiki_sources 从
//! 「来源登记表」升级为「知识源管理实体」：
//! - `schedule_cron`：定时刷新周期（5 字段 cron，如 `0 3 * * *`）
//! - `last_fetched_at`：上次抓取时间戳（毫秒）
//! - `status`：源启用状态（active / paused）
//!
//! 内容指纹 `content_hash` / 扩展配置 `metadata_json` 已存在，直接复用。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

/// SQLite 建列（幂等：先 PRAGMA 查列，不存在才 ALTER）。
async fn add_column_sqlite(
    db: &sea_orm::DatabaseConnection,
    col: &str,
    ddl: &str,
) -> Result<(), DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT name FROM pragma_table_info('wiki_sources') WHERE name = ?",
            [col.into()],
        ))
        .await?;
    if !rows.is_empty() {
        return Ok(());
    }
    db.execute_unprepared(ddl).await?;
    tracing::info!("[v114] wiki_sources 新增列 {col}");
    Ok(())
}

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    match db.get_database_backend() {
        DbBackend::Sqlite => {
            add_column_sqlite(
                &db,
                "schedule_cron",
                "ALTER TABLE wiki_sources ADD COLUMN schedule_cron TEXT",
            )
            .await?;
            add_column_sqlite(
                &db,
                "last_fetched_at",
                "ALTER TABLE wiki_sources ADD COLUMN last_fetched_at INTEGER",
            )
            .await?;
            add_column_sqlite(
                &db,
                "status",
                "ALTER TABLE wiki_sources ADD COLUMN status TEXT NOT NULL DEFAULT 'active'",
            )
            .await?;
        },
        DbBackend::Postgres => {
            db.execute_unprepared(
                "ALTER TABLE wiki_sources ADD COLUMN IF NOT EXISTS schedule_cron TEXT",
            )
            .await?;
            db.execute_unprepared(
                "ALTER TABLE wiki_sources ADD COLUMN IF NOT EXISTS last_fetched_at BIGINT",
            )
            .await?;
            db.execute_unprepared(
                "ALTER TABLE wiki_sources ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active'",
            )
            .await?;
        },
        other => {
            tracing::warn!("[v114] 不支持的数据库后端: {other:?}，跳过");
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn v114_adds_schedule_columns() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        for col in &["schedule_cron", "last_fetched_at", "status"] {
            let result = db
                .query_one_raw(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT {col} FROM wiki_sources LIMIT 0"),
                ))
                .await;
            assert!(result.is_ok(), "column {} should exist in wiki_sources", col);
        }
    }

    #[tokio::test]
    async fn v114_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        up(db).await.expect("v114 must be re-runnable in isolation");
    }
}
