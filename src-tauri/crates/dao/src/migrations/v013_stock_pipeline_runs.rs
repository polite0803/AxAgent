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

use sea_orm::{ConnectionTrait, DbErr};

/// 创建 stock_pipeline_runs 表
pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
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
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now') * 1000))",
    )
    .await?;

    // 创建索引（幂等）
    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_pipeline_runs_date ON stock_pipeline_runs(run_date)",
        "CREATE INDEX IF NOT EXISTS idx_pipeline_runs_status ON stock_pipeline_runs(status)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}
