//! v009 — reflection_lessons 规则表（F1 借鉴）
//!
//! ## 背景
//!
//! 借鉴 TradingAgents 的反思 → 规则提取机制：
//! 每次反思完成后，把 lesson_summary 提取为可重用的规则
//! 存入本表，下次决策流程可以查询相关规则作为参考。
//!
//! ## 字段
//!
//! - `id`: UUID
//! - `lesson_summary`: ≤200 字符规则描述
//! - `rule_pattern`: 规则触发条件（如"分批建仓节奏 ≤3 天"）
//! - `source_reflection_id`: 来源反思行
//! - `stock_code`: 适用 ticker（None=通用规则）
//! - `applicable_scenarios`: JSON 数组（适用场景标签）
//! - `times_applied`: 已应用次数
//! - `success_count`: 应用后成功次数
//! - `confidence`: 规则置信度 0-1
//! - `status`: active / deprecated
//! - `created_at` / `updated_at`: 毫秒时间戳
//!
//! ## 与 F1 配套
//!
//! - 反思 row 写完 status=completed 时，run_reflection_workflow 触发
//!   `extract_lesson_to_rule` 异步提取 lesson_summary → 本表
//! - 决策流程（stock-analysis trader/research-mgr）调
//!   `fetch_applicable_rules(stock_code, db)` 拿到相关规则注入 stock_lessons

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
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

    // 复合索引：按 ticker + confidence 拉取高置信度活跃规则
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_reflection_lessons_ticker_status_conf \
         ON reflection_lessons(stock_code, status, confidence DESC)",
    )
    .await?;

    // 全局规则索引：stock_code IS NULL 时的通用规则查询
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_reflection_lessons_global_status_conf \
         ON reflection_lessons(confidence DESC) WHERE stock_code IS NULL",
    )
    .await?;

    Ok(())
}
