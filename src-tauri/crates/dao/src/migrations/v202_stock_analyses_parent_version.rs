// SPDX-License-Identifier: AGPL-3.0-only
//! v202_stock_analyses_parent_version: stock_analyses 表新增 parent_analysis_id 字段
//!
//! ## 背景
//!
//! 原"重跑分析"采用覆盖语义（临时行 + 事后改 ID），存在时序竞态：
//! 后端把临时行 ID 改回旧 ID 时，前端 store 仍持有临时 ID，导致
//! `extract_evidence_citations` 等后续查询用临时 ID 找不到记录。
//!
//! 改为版本化语义：重跑分析时直接 INSERT 新行，通过 `parent_analysis_id`
//! 指向原始分析记录，保留同一股票的多个时间版本用于决策演变复盘。
//!
//! ## 本迁移做的事
//!
//! - `ALTER TABLE stock_analyses ADD COLUMN parent_analysis_id TEXT`
//! - 新增索引 `idx_stock_analyses_parent`（按父分析 ID 查询版本链）
//!
//! ## DDL 风格
//!
//! 与 v200/v201 保持一致：直接写 PG 语法，SQLite 侧由
//! [`sqlite_ddl`](super::pg_ddl::sqlite_ddl) 自动转换。
//! ALTER TABLE ADD COLUMN 在 SQLite/PG 语法相同，无需适配。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // PHASE 1: 新增 parent_analysis_id 列
    // SQLite/PG 都不支持 ADD COLUMN IF NOT EXISTS，直接执行 ALTER TABLE。
    // 列已存在时会报错（SQLite: "duplicate column name"），此时忽略即可。
    match db
        .execute_unprepared("ALTER TABLE stock_analyses ADD COLUMN parent_analysis_id TEXT")
        .await
    {
        Ok(_) => {},
        Err(e) => {
            // 列已存在是预期情况，不阻塞迁移
            // 兼容中英文错误消息：PostgreSQL 中文本地化返回 "已经存在"
            let msg = e.to_string();
            if !msg.contains("duplicate column")
                && !msg.contains("already exists")
                && !msg.contains("已经存在")
            {
                return Err(e);
            }
        },
    }

    // PHASE 2: 索引（按父分析 ID 查版本链）
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_stock_analyses_parent \
         ON stock_analyses(parent_analysis_id)",
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v202_is_self_idempotent() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        // 依赖 v100 + v200 已建好 stock_analyses 表
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        super::super::v200_axinvest_stock_tables::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：列已存在错误被忽略，索引 IF NOT EXISTS 幂等
        up(db).await.expect("v202 must be re-runnable in isolation");
    }
}
