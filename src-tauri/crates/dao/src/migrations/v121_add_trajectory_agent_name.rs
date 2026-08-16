// SPDX-License-Identifier: AGPL-3.0-only
//! v121: 为 trajectory_trajectories 表添加 agent_name 字段（结构化 Agent 标识）。
//!
//! ## Background
//!
//! 阶段三（证据驱动进化）要求轨迹在记录时就能结构化带上 Agent 标识：
//! 进化系统按 `agent_name` 精准聚合每个 Agent 的执行证据，不再依赖
//! topic/summary 的文本模糊匹配。旧轨迹无标识，故列可空（NULL），
//! 进化时仍以文本匹配兜底。
//!
//! ## Strategy
//!
//! 与 v120 一致的兼容写法：先查缺（PG 用 information_schema，SQLite 用
//! pragma_table_info），再执行普通 `ADD COLUMN`（**不**用
//! `ADD COLUMN IF NOT EXISTS`——较老 SQLite 不支持该语法）。
//! 列类型 `TEXT`，可空，无默认值。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    let exists = if is_pg {
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT 1 AS exists_flag FROM information_schema.columns \
                 WHERE table_schema = current_schema() \
                   AND table_name = 'trajectory_trajectories' AND column_name = 'agent_name'",
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
        rows.iter()
            .any(|r| r.try_get_by::<String, _>("name").map(|n| n == "agent_name").unwrap_or(false))
    };

    if exists {
        tracing::info!("[v121] trajectory_trajectories.agent_name 已存在，跳过");
        return Ok(());
    }

    db.execute_unprepared("ALTER TABLE trajectory_trajectories ADD COLUMN agent_name TEXT").await?;

    tracing::info!("[v121] Added agent_name column to trajectory_trajectories");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    /// v121 单独幂等：重复跑不报错。
    #[tokio::test]
    async fn v121_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        // 先建表（v100）
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v121 must be re-runnable in isolation");
    }

    /// 防回归：v121 之后 trajectory_trajectories 必须存在 agent_name 列。
    #[tokio::test]
    async fn v121_adds_agent_name_column() {
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
            ddl.contains("agent_name"),
            "trajectory_trajectories 应包含 agent_name 列，实际: {}",
            ddl
        );
    }
}
