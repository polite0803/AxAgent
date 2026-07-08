//! v010 — `strategy_performance` 加 `agreement_score` 列
//!
//! 用途:Phase 3 记录公式 vs LLM 决策的一致性分数(0-100),
//! 用于后续分析一致性趋势和分歧阈值告警。
use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "ALTER TABLE strategy_performance ADD COLUMN agreement_score INTEGER DEFAULT NULL",
    )
    .await?;
    Ok(())
}
