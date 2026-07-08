//! v011 — 修复 reflection_lessons 的 FK 引用
//!
//! ## 问题
//!
//! v009 创建了 `reflection_lessons` 表，其中包含：
//!
//! ```sql
//! FOREIGN KEY (stock_code) REFERENCES stock_analyses(stock_code) ON DELETE SET NULL
//! ```
//!
//! 但 `stock_analyses.stock_code` 不是 UNIQUE（同只股票可有多次分析），
//! SQLite 要求 FK 引用的列必须是 PRIMARY KEY 或 UNIQUE，否则报错：
//!
//! ```text
//! foreign key mismatch - "reflection_lessons" referencing "stock_analyses"
//! ```
//!
//! ## 修复策略
//!
//! 在已运行的数据库上：DROP 旧表 → CREATE 新表（不含错误的 FK）。
//! 由于 `reflection_lessons` 是新表（v009 刚从上游合并），不太可能有重要数据，
//! 直接重建是最安全的方式。
//!
//! 新安装从零开始跑 v009（已更新）时不受影响。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // 重建 reflection_lessons 表，移除错误的 stock_code FK
    db.execute_unprepared("DROP TABLE IF EXISTS reflection_lessons")
        .await?;
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS reflection_lessons (\
            id TEXT NOT NULL PRIMARY KEY, \
            lesson_summary TEXT NOT NULL, \
            rule_pattern TEXT, \
            source_reflection_id TEXT, \
            stock_code TEXT, \
            applicable_scenarios TEXT, \
            times_applied INTEGER NOT NULL DEFAULT 0, \
            success_count INTEGER NOT NULL DEFAULT 0, \
            confidence REAL NOT NULL DEFAULT 0.5, \
            status TEXT NOT NULL DEFAULT 'active', \
            created_at INTEGER NOT NULL, \
            updated_at INTEGER NOT NULL, \
            FOREIGN KEY (source_reflection_id) REFERENCES stock_reflections(id) ON DELETE SET NULL\
         )",
    )
    .await?;

    // 重建索引
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_reflection_lessons_ticker_status_conf \
         ON reflection_lessons(stock_code, status, confidence DESC)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_reflection_lessons_global_status_conf \
         ON reflection_lessons(confidence DESC) WHERE stock_code IS NULL",
    )
    .await?;

    Ok(())
}
