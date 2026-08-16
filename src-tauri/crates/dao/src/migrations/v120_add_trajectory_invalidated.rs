// SPDX-License-Identifier: AGPL-3.0-only
//! v120: 为 trajectory_trajectories 表添加 is_invalidated 字段（append-only 证据存储）。
//!
//! ## Background
//!
//! 阶段三（证据驱动进化）要求轨迹作为进化证据必须 append-only：
//! 已失效的轨迹只能标记失效（软删除），不可物理删除，保证贝叶斯后验
//! 能回溯完整的历史观测。原 delete_trajectory / cleanup_* 会物理删除
//! 轨迹及其 steps/rewards，破坏证据链。
//!
//! ## Strategy
//!
//! - SQLite / PostgreSQL 统一使用 `INTEGER NOT NULL DEFAULT 0`
//!   （0=有效，1=失效），与 conversations.is_archived 约定一致，
//!   entity 侧用 `i32` 承接
//! - 与 v100 PHASE 3.9 全表合规检查一致的兼容写法：先查缺（PG 用
//!   information_schema，SQLite 用 pragma_table_info），再执行普通
//!   `ADD COLUMN`（**不**用 `ADD COLUMN IF NOT EXISTS`——较老 SQLite
//!   不支持该语法，会报 `near "EXISTS": syntax error`，且不要用
//!   `.or_else` 静默吞错掩盖真实失败）
//! - 无历史数据回填需求（默认 0 即有效）

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    let exists = if is_pg {
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT 1 AS exists_flag FROM information_schema.columns \
                 WHERE table_schema = current_schema() \
                   AND table_name = 'trajectory_trajectories' AND column_name = 'is_invalidated'",
            ))
            .await?;
        row.is_some()
    } else {
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                ["trajectory_trajectories".into()],
            ))
            .await?;
        rows.iter().any(|r| {
            r.try_get_by::<String, _>("name").map(|n| n == "is_invalidated").unwrap_or(false)
        })
    };

    if exists {
        tracing::info!("[v120] trajectory_trajectories.is_invalidated 已存在，跳过");
        return Ok(());
    }

    db.execute_unprepared(
        "ALTER TABLE trajectory_trajectories ADD COLUMN is_invalidated INTEGER NOT NULL DEFAULT 0",
    )
    .await?;

    tracing::info!("[v120] Added is_invalidated column to trajectory_trajectories");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    /// v120 单独幂等：重复跑不报错（ALTER ... IF NOT EXISTS）。
    #[tokio::test]
    async fn v120_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        // 先建表（v100）
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v120 must be re-runnable in isolation");
    }

    /// 防回归：v120 之后 trajectory_trajectories 必须存在 is_invalidated 列。
    #[tokio::test]
    async fn v120_adds_invalidated_column() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='trajectory_trajectories'",
            ))
            .await
            .expect("测试应成功")
            .expect("trajectory_trajectories 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();
        assert!(
            ddl.contains("is_invalidated"),
            "trajectory_trajectories 应包含 is_invalidated 列，实际: {}",
            ddl
        );
    }
}
