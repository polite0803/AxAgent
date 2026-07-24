// SPDX-License-Identifier: AGPL-3.0-only
//! v201_lesson_application_tracking: P2-F15 切入点 3 —— lesson 应用追踪闭环
//!
//! ## 背景
//!
//! 现有 `reflection_lessons` 表已有 `times_applied` / `success_count` 字段，
//! 但 `run_lesson_validation` 只能用 `stock_reflections.lesson_summary.contains(...)`
//! 模糊匹配估算应用次数，准确度低：
//!
//! - `contains` 模糊匹配会漏匹配（lesson_summary 文本被改写后无法匹配）
//!   也会误匹配（子串撞车）
//! - 统计的是"反思时提到该 lesson 的次数"，**不是"决策时应用了该 lesson 的次数"**
//!   —— 两个概念完全不同
//! - `success_count` 用反思 verdict 推断，而不是用决策实际盈亏
//!   （`stock_analyses.outcome`）推断
//!
//! ## 本迁移创建的表
//!
//! `lesson_applications`：记录每次决策分析时引用了哪些 lesson，以及后续
//! T+N 验证完成后的实际 outcome。`run_lesson_validation` 据此精确计算
//! `times_applied` 和 `success_count`，反哺 `reflection_lessons.confidence`。
//!
//! ## 字段语义
//!
//! - `lesson_id` → `reflection_lessons.id`（被引用的规则）
//! - `analysis_id` → `stock_analyses.id`（引用该规则的决策分析）
//! - `applied_at`：注入时间（ISO 8601）
//! - `outcome_at_validation`：T+N 验证完成后该 analysis 的 outcome
//!   （`win` / `loss` / NULL=未验证）
//! - `validation_source`：`t_plus_5` / `t_plus_20` / `t_plus_60` / `manual`
//!
//! ## DDL 风格
//!
//! 与 v200 保持一致：直接写 PG 语法，SQLite 侧由
//! [`sqlite_ddl`](super::pg_ddl::sqlite_ddl) 自动转换。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 创建 lesson_applications 关联表
    // ========================================================================

    for sql in &["CREATE TABLE IF NOT EXISTS lesson_applications (\
            id TEXT NOT NULL PRIMARY KEY, \
            lesson_id TEXT NOT NULL, \
            analysis_id TEXT NOT NULL, \
            stock_code TEXT NOT NULL, \
            applied_at TEXT NOT NULL, \
            outcome_at_validation TEXT, \
            validation_source TEXT, \
            created_at BIGINT NOT NULL)"]
    {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 2: 索引
    //   - 按 lesson_id 查：统计某规则被引用次数（times_applied）
    //   - 按 analysis_id 查：T+N 验证反推 outcome 时批量更新
    //   - 按 (lesson_id, outcome_at_validation) 查：统计成功次数（success_count）
    // ========================================================================

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_lesson_applications_lesson_id \
         ON lesson_applications(lesson_id)",
        "CREATE INDEX IF NOT EXISTS idx_lesson_applications_analysis_id \
         ON lesson_applications(analysis_id)",
        "CREATE INDEX IF NOT EXISTS idx_lesson_applications_lesson_outcome \
         ON lesson_applications(lesson_id, outcome_at_validation)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v201_is_self_idempotent() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        // v201 依赖 v200 已建好的 reflection_lessons / stock_analyses 表
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        super::super::v200_axinvest_stock_tables::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：所有 CREATE 都是 IF NOT EXISTS，应幂等
        up(db).await.expect("v201 must be re-runnable in isolation");
    }

    #[tokio::test]
    async fn v201_creates_lesson_applications_table() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        super::super::v200_axinvest_stock_tables::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' AND name='lesson_applications'",
            ))
            .await
            .unwrap();
        assert!(row.is_some(), "lesson_applications 表应存在");

        // 验证索引存在
        for idx in &[
            "idx_lesson_applications_lesson_id",
            "idx_lesson_applications_analysis_id",
            "idx_lesson_applications_lesson_outcome",
        ] {
            let row = db
                .query_one_raw(sea_orm::Statement::from_sql_and_values(
                    sea_orm::DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='index' AND name=?",
                    [(*idx).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "索引 {} 应存在", idx);
        }
    }
}
