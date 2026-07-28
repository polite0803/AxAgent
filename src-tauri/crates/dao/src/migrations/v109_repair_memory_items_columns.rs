// SPDX-License-Identifier: AGPL-3.0-only
//! v109_repair_memory_items_columns: 修复 memory_items 表缺失的列（现由 v100 PHASE 3.9 保障）。
//!
//! ## 背景
//!
//! 原先 v101 迁移在 SQLite 路径上用 `let _ = db.execute_unprepared(...)` 静默
//! 吞掉了 ALTER TABLE 错误，导致部分旧数据库（v100 建表后 v101 ALTER 失败）
//! 缺少 tier/importance/access_count 等 trajectory memory 字段，运行时
//! 报 "字段 memory_items.tier 不存在" 错误。
//!
//! v100 PHASE 3.9 全表合规检查现已统一处理所有后加字段的兜底补列，
//! 本迁移保留为无操作（兼容历史数据库标记此版本已应用）。

use sea_orm::DbErr;

pub async fn up(_db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    tracing::info!("[v109] 列定义已由 v100 PHASE 3.9 统一保障，无需修复");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

    #[tokio::test]
    async fn v109_does_not_break_on_bare_v100_table() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // v100 PHASE 3.9 已保证所有列存在
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        // v109 现在为 no-op，不报错即可
        up(db.clone()).await.unwrap();

        // 验证关键列存在：SELECT 0 行不报错说明列存在
        for col in &[
            "tier",
            "importance",
            "access_count",
            "last_accessed",
            "decay_rate",
            "expires_at",
            "source_conversation_id",
            "source_message_id",
            "memory_nature",
            "tags",
            "applicability_tags",
            "confirmed",
        ] {
            let sql = format!("SELECT {} FROM memory_items LIMIT 0", col);
            let result = db.query_one_raw(Statement::from_string(DbBackend::Sqlite, sql)).await;
            assert!(result.is_ok(), "column {} should exist in memory_items", col);
        }
    }

    #[tokio::test]
    async fn v109_is_idempotent_on_fully_migrated_db() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::run_migrations(&db).await.unwrap();
        up(db.clone()).await.expect("v109 must be re-runnable without error");
    }
}
