// SPDX-License-Identifier: AGPL-3.0-only
//! v133: 创建 session_events 表 —— 跨进程 Resume 事件流（PLAN-codex-parity P0-3）。
//!
//! ## Background
//! run_turn 内的 ThoughtChain / ContextWindow 全程内存态（ReactEngine.run()
//! while 循环 L688），进程 kill 即丢执行态。此前已有 messages 表 seed 恢复
//! 上下文文本，但**跑到一半 kill 的场景**缺少"哪一步中断"的标记。
//! 本表即该标记的持久化载体。
//!
//! ## 语义
//! 只存**结构性事件**：TurnStarted / Message / ToolCall / ToolResult /
//! Compacted / TurnEnded / Interrupted。不存 LLM 输出文本（messages 表负责）。
//! 按 session_id + seq 递增，未配对的 ToolCall → Interrupted。
//!
//! ## 列
//! id PK, session_id, seq, event_type, payload(JSON NULL), created_at
//!
//! ## 索引
//! - idx_session_events_session_seq: 按 session_id + seq 升序查事件流
//! - idx_session_events_session_type: 按 session_id + event_type 查某类事件
//!
//! ## Strategy
//! CREATE TABLE IF NOT EXISTS —— 幂等；SQLite / PostgreSQL 均支持。

use sea_orm::ConnectionTrait;
use sea_orm::DbErr;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS session_events (\
         id INTEGER PRIMARY KEY AUTOINCREMENT, \
         session_id TEXT NOT NULL, \
         seq BIGINT NOT NULL, \
         event_type TEXT NOT NULL, \
         payload TEXT, \
         created_at TEXT NOT NULL)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_session_events_session_seq \
         ON session_events (session_id, seq)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_session_events_session_type \
         ON session_events (session_id, event_type)",
    )
    .await?;

    tracing::info!("[v133] Created session_events table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use sea_orm::DbBackend;
    use sea_orm::Statement;

    /// v133 单独幂等：重复跑不报错（唯一索引用 IF NOT EXISTS）。
    #[tokio::test]
    async fn v133_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v133 must be re-runnable in isolation");
    }

    /// 防回归：v133 之后 session_events 表必须存在且含全部列 + 两个索引。
    #[tokio::test]
    async fn v133_creates_table_and_indexes() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='session_events'",
            ))
            .await
            .expect("测试应成功")
            .expect("session_events 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();

        for col in ["id", "session_id", "seq", "event_type", "payload", "created_at"] {
            assert!(ddl.contains(col), "session_events 应含 {col} 列，实际: {}", ddl);
        }

        for idx in ["idx_session_events_session_seq", "idx_session_events_session_type"] {
            let r = db
                .query_one_raw(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT name FROM sqlite_master WHERE type='index' AND name='{idx}'"),
                ))
                .await
                .expect("测试应成功");
            assert!(r.is_some(), "索引 {idx} 应存在");
        }
    }
}
