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

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // 检查列是否存在（幂等保护）
    let row = db
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) AS cnt FROM pragma_table_info('stock_analyses') \
             WHERE name='node_results_snapshot'",
        ))
        .await?;
    let exists: i32 = row.and_then(|r| r.try_get_by("cnt").ok()).unwrap_or(0);
    if exists > 0 {
        db.execute_unprepared("ALTER TABLE stock_analyses DROP COLUMN node_results_snapshot")
            .await?;
    }
    Ok(())
}
