// SPDX-License-Identifier: AGPL-3.0-only
//! v111_retrieval_hits_feedback: 为 retrieval_hits 表添加反馈闭环字段。
//!
//! ## 背景
//!
//! retrieval_hits 表此前只有写入路径（streaming.rs:930 的 record_hits），
//! 无任何读取方与反馈字段，形成"只写不读"的数据沼泽，无法做 RL 检索 /
//! embedder 微调 / RAG 自适应优化。
//!
//! 本迁移添加以下字段，构建完整的反馈闭环：
//! - `feedback`: 用户反馈（'positive' / 'negative' / 'irrelevant' / NULL）
//! - `feedback_at`: 反馈时间戳（Unix 秒）
//! - `used_in_response`: 是否在最终回复中被引用（0/1）
//! - `score_after_rerank`: 重排后分数（可选，用于对比原始 score）
//! - `created_at`: 创建时间戳（用于按时间聚合反馈统计）
//!
//! ## 幂等性
//!
//! - PostgreSQL: `ALTER TABLE ... ADD COLUMN IF NOT EXISTS ...`
//! - SQLite: 先查 `pragma table_info` 判断列是否存在，再 ADD COLUMN
//!   （SQLite 不支持 IF NOT EXISTS on ADD COLUMN，重复添加会报 duplicate column）

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

/// 待添加的列清单：(列名, 列定义)
/// 列定义同时兼容 PostgreSQL 与 SQLite（用通用类型：TEXT/BIGINT/INTEGER/DOUBLE PRECISION）。
const NEW_COLUMNS: &[(&str, &str)] = &[
    ("feedback", "TEXT"),
    ("feedback_at", "BIGINT"),
    ("used_in_response", "INTEGER NOT NULL DEFAULT 0"),
    ("score_after_rerank", "DOUBLE PRECISION"),
    ("created_at", "BIGINT NOT NULL DEFAULT 0"),
];

/// 检查表是否存在。
async fn table_exists(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    is_pg: bool,
) -> Result<bool, DbErr> {
    let sql = if is_pg {
        format!(
            "SELECT COUNT(*) AS cnt FROM information_schema.tables WHERE table_name = '{}'",
            table
        )
    } else {
        format!("SELECT COUNT(*) AS cnt FROM sqlite_master WHERE type='table' AND name='{}'", table)
    };
    let row = db.query_one_raw(Statement::from_string(db.get_database_backend(), sql)).await?;
    match row {
        Some(r) => {
            let cnt: i64 = r.try_get_by("cnt").unwrap_or(0);
            Ok(cnt > 0)
        },
        None => Ok(false),
    }
}

/// 检查列是否存在（SQLite 走 pragma table_info，PG 走 information_schema.columns）。
async fn column_exists(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    column: &str,
    is_pg: bool,
) -> Result<bool, DbErr> {
    let sql = if is_pg {
        format!(
            "SELECT COUNT(*) AS cnt FROM information_schema.columns \
             WHERE table_name = '{}' AND column_name = '{}'",
            table, column
        )
    } else {
        format!(
            "SELECT COUNT(*) AS cnt FROM pragma_table_info('{}') WHERE name = '{}'",
            table, column
        )
    };
    let row = db.query_one_raw(Statement::from_string(db.get_database_backend(), sql)).await?;
    match row {
        Some(r) => {
            let cnt: i64 = r.try_get_by("cnt").unwrap_or(0);
            Ok(cnt > 0)
        },
        None => Ok(false),
    }
}

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;
    let backend = db.get_database_backend();
    let table = "retrieval_hits";

    tracing::info!("[v111] 开始为 retrieval_hits 添加反馈闭环字段 (is_pg={})", is_pg);

    // 表不存在时跳过（v100 会建表，理论上不会走到这里，但保持防御性）
    if !table_exists(&db, table, is_pg).await? {
        tracing::warn!("[v111] 表 {} 不存在，跳过字段添加", table);
        return Ok(());
    }

    for (col, def) in NEW_COLUMNS {
        if column_exists(&db, table, col, is_pg).await? {
            tracing::debug!("[v111] 列 {}.{} 已存在，跳过", table, col);
            continue;
        }

        // PostgreSQL 支持 ADD COLUMN IF NOT EXISTS；SQLite 不支持，需先检查再添加
        let sql = if is_pg {
            format!("ALTER TABLE {} ADD COLUMN IF NOT EXISTS {} {}", table, col, def)
        } else {
            format!("ALTER TABLE {} ADD COLUMN {} {}", table, col, def)
        };
        tracing::info!("[v111] 执行: {}", sql);
        match db.execute_unprepared(&sql).await {
            Ok(_) => {},
            Err(e) => {
                // 列可能已被并发添加，仅记录警告不中断迁移
                tracing::warn!("[v111] 添加列 {}.{} 失败（可能已存在）: {}", table, col, e);
            },
        }
    }

    // 为 feedback 字段添加索引，便于按反馈类型聚合统计
    let feedback_idx_sql = if is_pg {
        format!(
            "CREATE INDEX IF NOT EXISTS idx_retrieval_hits_feedback ON {}(feedback) WHERE feedback IS NOT NULL",
            table
        )
    } else {
        format!("CREATE INDEX IF NOT EXISTS idx_retrieval_hits_feedback ON {}(feedback)", table)
    };
    let _ = db.execute_raw(Statement::from_string(backend, feedback_idx_sql)).await;

    // 为 created_at 添加索引，便于按时间范围查询反馈统计
    let created_at_idx_sql =
        format!("CREATE INDEX IF NOT EXISTS idx_retrieval_hits_created ON {}(created_at)", table);
    let _ = db.execute_raw(Statement::from_string(backend, created_at_idx_sql)).await;

    tracing::info!("[v111] retrieval_hits 反馈闭环字段添加完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn v111_adds_feedback_columns_on_fresh_db() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 先跑 v100 建 retrieval_hits 表
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        // 再跑 v111
        up(db.clone()).await.unwrap();

        // 验证新列存在
        for col in
            &["feedback", "feedback_at", "used_in_response", "score_after_rerank", "created_at"]
        {
            let sql = format!("SELECT {} FROM retrieval_hits LIMIT 0", col);
            let result = db.query_one_raw(Statement::from_string(DbBackend::Sqlite, sql)).await;
            assert!(result.is_ok(), "column {} should exist in retrieval_hits", col);
        }
    }

    #[tokio::test]
    async fn v111_is_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：所有列已存在，应跳过不报错
        up(db.clone()).await.expect("v111 must be re-runnable without error");
    }

    #[tokio::test]
    async fn v111_skips_when_table_missing() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 不跑 v100，直接跑 v111，应跳过不报错
        up(db.clone()).await.expect("v111 should skip gracefully when table is missing");
    }

    #[tokio::test]
    async fn v111_can_insert_and_query_feedback() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        // 先插入外键父记录（retrieval_hits 引用 conversations / knowledge_bases）
        db.execute_unprepared(
            "INSERT INTO conversations (id, title, model_id, provider_id, created_at, updated_at) \
             VALUES ('conv1', 'Test', 'm1', 'p1', 1700000000, 1700000000)",
        )
        .await
        .unwrap();
        db.execute_unprepared("INSERT INTO knowledge_bases (id, name) VALUES ('kb1', 'Test KB')")
            .await
            .unwrap();

        // 插入一条带反馈的记录
        db.execute_unprepared(
            "INSERT INTO retrieval_hits (id, conversation_id, message_id, knowledge_base_id, \
             document_id, chunk_ref, score, preview, feedback, feedback_at, used_in_response, created_at) \
             VALUES ('test1', 'conv1', 'msg1', 'kb1', 'doc1', 'chunk1', 0.85, 'preview', \
             'positive', 1700000000, 1, 1700000000)"
        ).await.unwrap();

        // 查询验证
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT feedback, used_in_response, created_at FROM retrieval_hits WHERE id = 'test1'"
                    .to_string(),
            ))
            .await
            .unwrap()
            .expect("row should exist");
        let feedback: String = row.try_get_by("feedback").unwrap();
        let used: i32 = row.try_get_by("used_in_response").unwrap();
        let created_at: i64 = row.try_get_by("created_at").unwrap();
        assert_eq!(feedback, "positive");
        assert_eq!(used, 1);
        assert_eq!(created_at, 1700000000);
    }
}
