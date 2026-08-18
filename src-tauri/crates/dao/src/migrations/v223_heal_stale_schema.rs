// SPDX-License-Identifier: AGPL-3.0-only
//! v223: 修复存量库缺失的列与 CHECK 约束（自愈迁移）。
//!
//! ## 背景
//!
//! 早期 `repair_schema` 在个别迁移失败时仍会强制写入 `CURRENT_VERSION`，
//! 导致版本表显示"已追平"但实际 schema 缺失：
//! - `trajectory_trajectories.agent_name`（v121 未生效）→ 轨迹清理/查询报
//!   "字段 trajectory_trajectories.agent_name 不存在"
//! - `agency_experts` / `agent_profiles` 的 category CHECK 约束缺少
//!   `opc-industry` / `opc-domain`（v200 PHASE 3 旧版本未包含）→
//!   OPC 种子化插入 `opc-industry` / `opc-domain` 专家时违反约束
//!
//! 由于版本表已 >= 222，`run_migrations` 永远不会重跑 v121/v200。
//! 本迁移以 v223 单调递增，在版本滞后的存量库上必然执行，幂等补全。
//!
//! ## Strategy
//!
//! 1. `agent_name`：先查缺（PG 用 information_schema，SQLite 用
//!    pragma_table_info），再执行普通 `ADD COLUMN`（不用
//!    `ADD COLUMN IF NOT EXISTS`——较老 SQLite 不支持该语法）。
//!    列类型 `TEXT`，可空，与 v121 完全一致。
//! 2. category CHECK：复用 [`super::ensure_category_check_constraints`]，
//!    DROP + ADD 全量值（含 opc-industry / opc-domain），幂等。
//!
//! 全新库：v121/v200 已建好相关结构，本迁移为 no-op。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // 1) 确保 trajectory_trajectories.agent_name 存在
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

    if !exists {
        db.execute_unprepared("ALTER TABLE trajectory_trajectories ADD COLUMN agent_name TEXT")
            .await?;
        tracing::info!("[v223] trajectory_trajectories.agent_name 已补全");
    }

    // 2) 重新断言 category CHECK 约束（含 opc-industry / opc-domain）
    super::ensure_category_check_constraints(&db).await?;

    tracing::info!("[v223] 存量库 schema 自愈完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    /// 查询 SQLite 表中是否存在指定列。
    async fn column_exists(db: &sea_orm::DatabaseConnection, column: &str) -> bool {
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                ["trajectory_trajectories".into()],
            ))
            .await
            .expect("查询应成功");
        rows.iter().any(|r| r.try_get_by::<String, _>("name").unwrap_or_default() == column)
    }

    /// 防回归：仅跑了 v100 的存量库（缺 agent_name），v223 必须补上该列。
    #[tokio::test]
    async fn v223_adds_agent_name_to_stale_db() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        // 模拟存量库：只跑了 v100（无 trajectory_trajectories.agent_name）
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");

        assert!(!column_exists(&db, "agent_name").await, "存量库在 v223 前应缺少 agent_name");

        up(db.clone()).await.expect("v223 应补全 agent_name");

        assert!(column_exists(&db, "agent_name").await, "v223 后 agent_name 应存在");
    }

    /// v223 单独幂等：重复跑不报错。
    #[tokio::test]
    async fn v223_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v223 must be re-runnable in isolation");
    }
}
