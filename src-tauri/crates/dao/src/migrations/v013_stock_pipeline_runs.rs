//! v013 — stock_pipeline_runs 表
//!
//! 记录每次股票管道执行的历史（发现→分析→持仓再评估）。
//!
//! ## 字段说明
//!
//! - `id`: UUID 主键
//! - `run_date`: 管道执行日期（YYYY-MM-DD）
//! - `as_of_date`: 时间旅行模式截止日（可选）
//! - `status`: running / completed / failed
//! - `candidates_json`: 候选股列表 JSON
//! - `new_analyses_json`: 新候选股分析摘要 JSON
//! - `reassessed_json`: 持仓再评估摘要 JSON
//! - `summary_json`: 汇总报告 JSON
//! - `error_message`: 失败原因（status=failed 时）
//! - `started_at`: 开始时间戳（ms）
//! - `completed_at`: 完成时间戳（ms，可空）
//! - `created_at`: 创建时间戳（ms）
//!
//! ## 幂等保护
//!
//! 所有 CREATE 语句均使用 `IF NOT EXISTS`，重复执行不报错。
//!
//! ## 可移植性
//!
//! - 时间戳列在 SQLite 用 `INTEGER`（动态 64 位），在 PostgreSQL 必须用
//!   `BIGINT`，否则 int4 范围（±21 亿）无法容纳毫秒时间戳（~1.7e12）。
//! - `created_at` 默认值：SQLite 用 `strftime('%s','now')*1000`，
//!   PostgreSQL 用 `EXTRACT(EPOCH FROM now())*1000`。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr};

/// 创建 stock_pipeline_runs 表
pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    let create_sql = if backend == DatabaseBackend::Postgres {
        "CREATE TABLE IF NOT EXISTS stock_pipeline_runs (\
            id TEXT NOT NULL PRIMARY KEY, \
            run_date TEXT NOT NULL, \
            as_of_date TEXT, \
            status TEXT NOT NULL, \
            candidates_json TEXT, \
            new_analyses_json TEXT, \
            reassessed_json TEXT, \
            summary_json TEXT, \
            error_message TEXT, \
            started_at BIGINT NOT NULL, \
            completed_at BIGINT, \
            created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM now()) * 1000)::bigint)"
    } else {
        "CREATE TABLE IF NOT EXISTS stock_pipeline_runs (\
            id TEXT NOT NULL PRIMARY KEY, \
            run_date TEXT NOT NULL, \
            as_of_date TEXT, \
            status TEXT NOT NULL, \
            candidates_json TEXT, \
            new_analyses_json TEXT, \
            reassessed_json TEXT, \
            summary_json TEXT, \
            error_message TEXT, \
            started_at INTEGER NOT NULL, \
            completed_at INTEGER, \
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now') * 1000))"
    };

    db.execute_unprepared(create_sql).await?;

    // 创建索引（幂等）
    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_pipeline_runs_date ON stock_pipeline_runs(run_date)",
        "CREATE INDEX IF NOT EXISTS idx_pipeline_runs_status ON stock_pipeline_runs(status)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}
