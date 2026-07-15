// SPDX-License-Identifier-Identifier: AGPL-3.0-only
//! v101_route_history: Smart Router 路由历史持久化表
//!
//! 为 `CostAwareRouter` 增加 `route_history` 表，记录每次路由决策及其后续
//! 反馈结果。程序启动时由 `CostAwareRouter::load_from_db` 读取全量历史并
//! 重建内存中的 `history` / `global_stats` / `bucket_stats` / 原子计数器，
//! 让 ML 路由优化能力在重启后得以延续。
//!
//! 幂等：所有 CREATE 均带 IF NOT EXISTS，重复执行安全。

use sea_orm::{DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

/// 创建 route_history 表 + prompt_hash 索引。
pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // 主表：所有列类型显式声明，避免 pg_ddl 自动转换的不确定性。
    // - TEXT / Option<String> → TEXT
    // - bool / Option<bool>   → INTEGER（SQLite）/ BOOLEAN（PG，由 pg_ddl 不改）
    //   SQLite 下 0/1，SeaORM 自动转换；PG 下用 INTEGER 也兼容。
    // - i64 / Option<i64>     → BIGINT
    // - f64 / Option<f64>     → REAL（pg_ddl 转 DOUBLE PRECISION）
    let create_table = "\
        CREATE TABLE IF NOT EXISTS route_history (\
            id TEXT NOT NULL PRIMARY KEY, \
            prompt_hash TEXT NOT NULL, \
            prompt_preview TEXT NOT NULL, \
            heuristic_tier TEXT NOT NULL, \
            selected_tier TEXT NOT NULL, \
            outcome_success INTEGER, \
            outcome_quality_score REAL, \
            outcome_user_override INTEGER, \
            outcome_user_tier TEXT, \
            outcome_latency_ms BIGINT, \
            outcome_tokens_used BIGINT, \
            outcome_cost_usd REAL, \
            timestamp BIGINT NOT NULL, \
            features_json TEXT)";
    exec_ddl(&db, is_pg, create_table).await?;

    // 反馈回写按 prompt_hash 定位，建索引加速 UPDATE。
    let create_index =
        "CREATE INDEX IF NOT EXISTS idx_route_history_prompt_hash ON route_history(prompt_hash)";
    exec_ddl(&db, is_pg, create_index).await?;

    // 启动时按时间倒序加载，建索引加速 ORDER BY。
    let create_ts_index =
        "CREATE INDEX IF NOT EXISTS idx_route_history_timestamp ON route_history(timestamp)";
    exec_ddl(&db, is_pg, create_ts_index).await?;

    Ok(())
}
