// SPDX-License-Identifier: AGPL-3.0-only
//! v122: 创建 evolution_execution_stats 表 —— 进化产物真实执行反馈的持久化。
//!
//! ## Background
//!
//! 阶段四后置闭环（D3 持久化）：`EvolutionFeedbackSinkImpl::record` 把
//! 进化产物（计算型 / 编排型）的真实执行成败累计到内存统计后，异步 upsert
//! 到本表；应用重启时由启动流程 `load_all_execution_stats` 读回内存，
//! 保证真实执行证据不丢失、贝叶斯决策器的后验在重启后依然可信。
//!
//! ## Schema
//!
//! 复合主键 `(conversation_id, tool_id)`：一个会话内同一产物只有一行，
//! upsert 时做增量累加。`conversation_id` 用空串 `""` 表示"无会话上下文"
//! （纯 tools 层 / 无会话的执行）。
//!
//! ## Strategy
//!
//! `CREATE TABLE IF NOT EXISTS` —— 幂等，可重复执行；SQLite 与 PostgreSQL
//! 均支持该 ANSI DDL 与复合主键语法。

use sea_orm::ConnectionTrait;
use sea_orm::DbErr;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS evolution_execution_stats (\
         conversation_id TEXT NOT NULL, \
         tool_id TEXT NOT NULL, \
         usage_count INTEGER NOT NULL DEFAULT 0, \
         successes INTEGER NOT NULL DEFAULT 0, \
         failures INTEGER NOT NULL DEFAULT 0, \
         PRIMARY KEY (conversation_id, tool_id))",
    )
    .await?;

    tracing::info!("[v122] Created evolution_execution_stats table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use sea_orm::DbBackend;

    /// v122 单独幂等：重复跑不报错。
    #[tokio::test]
    async fn v122_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v122 must be re-runnable in isolation");
    }

    /// 防回归：v122 之后 evolution_execution_stats 表与复合主键必须存在。
    #[tokio::test]
    async fn v122_creates_table_with_composite_pk() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='evolution_execution_stats'",
            ))
            .await
            .expect("测试应成功")
            .expect("evolution_execution_stats 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();
        assert!(
            ddl.contains("PRIMARY KEY")
                && ddl.contains("conversation_id")
                && ddl.contains("tool_id"),
            "evolution_execution_stats 应含 (conversation_id, tool_id) 复合主键，实际: {}",
            ddl
        );
    }
}
