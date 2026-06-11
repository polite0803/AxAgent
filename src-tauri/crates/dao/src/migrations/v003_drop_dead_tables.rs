//! v003 — 清理死表
//!
//! ddl.rs 注释明确标记以下三张表为死表（无代码引用，现存代码用
//! `conversation_categories` 等实体替代）：
//!
//! - `categories`
//! - `apps`
//! - `context_packs`
//!
//! 这些表在 v001 阶段只是占位定义、从未被实际读写。DROP 它们让
//! sqlite_master 清单更清晰，避免 schema 漂移分析误判。
//!
//! `scheduled_tasks` 也是死表（ddl.rs 注释说明 CronJobStore 走内存），
//! 但因为它的清理涉及到 CronJobStore 重启的潜在依赖（虽然 v001 没
//! 真的建），放到后续 v004 再处理。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    for sql in &[
        "DROP TABLE IF EXISTS categories",
        "DROP TABLE IF EXISTS apps",
        "DROP TABLE IF EXISTS context_packs",
    ] {
        db.execute_unprepared(sql).await?;
    }
    Ok(())
}
