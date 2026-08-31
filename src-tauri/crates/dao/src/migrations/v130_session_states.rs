// SPDX-License-Identifier: AGPL-3.0-only
//! v130: 创建 session_states 表 —— 会话状态存储持久化（能力按需加载闭环 P0-1）。
//!
//! ## Background
//!
//! 渐进式披露只通了 L0（目录）→ L1（展开定义），L1 → 执行断裂：
//! Agent 展开某能力定义后没有「加载」动作落地，下一轮也读不回已加载内容。
//!
//! 根因是缺少**写入与读取之间的解耦点**：工具调用（写）与系统提示注入（读）
//! 发生在不同请求轮次，必须靠会话状态传递。本表即该状态的持久化载体。
//!
//! ## Key 语义
//!
//! key 自带 scope / namespace / conversation_id / agent_id 四段，
//! 构造规则见 `axagent_harness::session_state::scoped_key`。
//! 本表额外冗余 conversation_id / agent_id 两列 —— key 是字符串无法高效
//! 范围清理，冗余列让「会话结束清状态」「按 Agent 审计」走索引而非全表前缀扫描。
//!
//! ## Strategy
//!
//! `CREATE TABLE IF NOT EXISTS` —— 幂等，可重复执行；SQLite 与 PostgreSQL 均支持。

use sea_orm::ConnectionTrait;
use sea_orm::DbErr;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS session_states (\
         state_key TEXT NOT NULL PRIMARY KEY, \
         state_value TEXT NOT NULL, \
         scope TEXT NOT NULL, \
         conversation_id TEXT, \
         agent_id TEXT, \
         updated_at_ms BIGINT NOT NULL, \
         expires_at_ms BIGINT)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_session_states_conversation \
         ON session_states (conversation_id)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_session_states_expiry \
         ON session_states (expires_at_ms)",
    )
    .await?;

    tracing::info!("[v130] Created session_states table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use sea_orm::DbBackend;
    use sea_orm::Statement;

    /// v130 单独幂等：重复跑不报错（索引亦用 IF NOT EXISTS）。
    #[tokio::test]
    async fn v130_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v130 must be re-runnable in isolation");
    }

    /// 防回归：v130 之后 session_states 表必须存在且含全部列 + 两个索引。
    #[tokio::test]
    async fn v130_creates_table_and_indexes() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='session_states'",
            ))
            .await
            .expect("测试应成功")
            .expect("session_states 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();

        for col in [
            "state_key",
            "state_value",
            "scope",
            "conversation_id",
            "agent_id",
            "updated_at_ms",
            "expires_at_ms",
        ] {
            assert!(ddl.contains(col), "session_states 应含 {col} 列，实际: {}", ddl);
        }

        for idx in ["idx_session_states_conversation", "idx_session_states_expiry"] {
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
