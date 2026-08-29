// SPDX-License-Identifier: AGPL-3.0-only
//! v127: 创建 capability_stats 表 —— 能力护照执行统计持久化。
//!
//! ## Background
//!
//! 能力发现系统的排序器（CapabilityRanker）消费护照 `stats.recent_success_rate` /
//! `total_calls` / `avg_duration_seconds` 做 β 历史成功率加权与冷启动探索提权，
//! 但护照注册后 stats 从未被写回（state.rs 全部 `Default::default()`）：
//! - `total_calls` 恒 0 → `total_calls < 10` 恒真 → 探索提权对所有能力生效，
//!   排序器的 β/γ/δ/探索四维全部失真（Phase 1 反馈闭环修复）。
//!
//! 本表为反馈闭环的持久化载体：
//! - 写路径：接线点（rt-workflow 引擎 post_execution_reflect / skill 执行 / agent 执行）
//!   调 `repo::capability_stats::record_execution`
//! - 读路径：`CapabilityIndexerImpl` 返回护照前合并本表数据到 `passport.stats`
//!
//! ## Schema
//!
//! - `capability_id`：护照 ID（`workflow:{id}` / `skill:{name}` / `agent:{id}` / `tool:{name}`）
//! - `recent_window`：最近 N 次执行结果 JSON 数组（[0/1,...]），近 N 次成功率由窗口均值推导
//! - 其余为聚合字段（total_calls / success_count / avg_duration_ms）
//!
//! ## Strategy
//!
//! `CREATE TABLE IF NOT EXISTS` —— 幂等，可重复执行；SQLite 与 PostgreSQL 均支持。

use sea_orm::ConnectionTrait;
use sea_orm::DbErr;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS capability_stats (\
         capability_id TEXT NOT NULL PRIMARY KEY, \
         total_calls BIGINT NOT NULL DEFAULT 0, \
         success_count BIGINT NOT NULL DEFAULT 0, \
         recent_window TEXT NOT NULL DEFAULT '[]', \
         avg_duration_ms BIGINT NOT NULL DEFAULT 0, \
         last_executed_at BIGINT, \
         updated_at BIGINT NOT NULL)",
    )
    .await?;

    tracing::info!("[v127] Created capability_stats table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use sea_orm::DbBackend;

    /// v127 单独幂等：重复跑不报错。
    #[tokio::test]
    async fn v127_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v127 must be re-runnable in isolation");
    }

    /// 防回归：v127 之后 capability_stats 表必须存在且含全部列。
    #[tokio::test]
    async fn v127_creates_table() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='capability_stats'",
            ))
            .await
            .expect("测试应成功")
            .expect("capability_stats 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();
        assert!(
            ddl.contains("total_calls")
                && ddl.contains("success_count")
                && ddl.contains("recent_window")
                && ddl.contains("avg_duration_ms"),
            "capability_stats 应含 total_calls/success_count/recent_window/avg_duration_ms 列，实际: {}",
            ddl
        );
    }
}
