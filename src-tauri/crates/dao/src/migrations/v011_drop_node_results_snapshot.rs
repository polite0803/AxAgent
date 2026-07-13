//! v011 — 删除 stock_analyses.node_results_snapshot 死字段
//!
//! ## 背景
//!
//! `node_results_snapshot` 定义于 v010（原 v005 未注册迁移合并），
//! 原预留给 `rerun_decision_only` 缓存机制：命中缓存时跳过已算节点。
//! 但该机制从未实现，全代码库零写入（所有 ActiveModel 均设为 `None`），
//! 字段始终为 NULL。现删除该列以保持 schema 与 entity 一致。
//!
//! ## 幂等保护
//!
//! 新库在 v010 建表时含此列，本迁移将其删除。
//! 若列已删除（重复执行或手动清理过），则跳过。
//!
//! ## 可移植性
//!
//! 列存在性检查需按后端分支：SQLite 用 `pragma_table_info`，
//! PostgreSQL 用 `information_schema.columns`。`DROP COLUMN` 语法两边通用。

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    // 检查列是否存在（幂等保护）
    // 注意：COUNT(*) 在 PostgreSQL 返回 bigint(i64)，SQLite 返回 INTEGER。
    // 必须按 i64 读取，否则 PG 下类型不匹配导致读取失败、误判"列不存在"而跳过 DROP。
    let exists: i64 = if backend == DatabaseBackend::Postgres {
        let row = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT COUNT(*) AS cnt FROM information_schema.columns \
                 WHERE table_name = 'stock_analyses' AND column_name = 'node_results_snapshot'",
            ))
            .await?;
        row.and_then(|r| r.try_get_by("cnt").ok()).unwrap_or(0)
    } else {
        let row = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT COUNT(*) AS cnt FROM pragma_table_info('stock_analyses') \
                 WHERE name='node_results_snapshot'",
            ))
            .await?;
        row.and_then(|r| r.try_get_by("cnt").ok()).unwrap_or(0)
    };

    if exists > 0 {
        db.execute_unprepared("ALTER TABLE stock_analyses DROP COLUMN node_results_snapshot")
            .await?;
    }
    Ok(())
}
