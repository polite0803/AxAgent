// SPDX-License-Identifier: AGPL-3.0-only
//! v204_paper_portfolio: G2 模拟观察组合（Paper Trading Portfolio）
//!
//! ## 背景
//!
//! DojoAgents 宣传场景 1/2/3 都依赖"研究观察列表 / 模拟组合"实体：
//! - 场景 1：把市场异动摘要沉淀成研究观察列表，持续跟踪后续表现
//! - 场景 2：按消息发布日价格虚拟建仓，观察后市表现
//! - 场景 3：从持仓诊断结果生成新观察列表
//!
//! AxInvest 现有 `PortfolioDashboard` 只展示真实持仓，缺模拟观察组合实体。
//!
//! ## 本迁移创建的表
//!
//! - `paper_portfolios`：模拟组合主表（一个组合 = 一次研究观察）
//! - `paper_positions`：组合内的虚拟持仓（按事件日价格建仓）
//!
//! ## 字段语义
//!
//! ### paper_portfolios
//! - `id`：UUID
//! - `name`：组合名称（用户输入或自动生成）
//! - `source_event`：来源事件描述（如 "英伟达隔夜大跌" / "Meta 卖算力"）
//! - `source_news_id`：关联 `news_archive.id`，实现新闻→组合溯源（可空）
//! - `source_screenshot_diagnosis_id`：关联 `screenshot_diagnoses.id`（G6 用，可空）
//! - `status`：active / closed / archived
//! - `created_at` / `closed_at`：时间戳（ms）
//!
//! ### paper_positions
//! - `id`：UUID
//! - `portfolio_id`：所属组合 ID
//! - `symbol`：股票代码
//! - `market`：A / US / HK / ETF
//! - `entry_price`：虚拟建仓价
//! - `entry_date`：虚拟建仓日（YYYY-MM-DD）
//! - `quantity`：虚拟数量
//! - `exit_price`：虚拟平仓价（可空）
//! - `exit_date`：虚拟平仓日（可空）
//! - `status`：open / closed
//! - `note`：备注（如 "AI 算力链" / "光模块龙头"）
//! - `created_at` / `updated_at`：时间戳（ms）
//!
//! ## DDL 风格
//!
//! 与 v201 保持一致：直接写 PG 语法，SQLite 侧由
//! [`sqlite_ddl`](super::pg_ddl::sqlite_ddl) 自动转换。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 创建 paper_portfolios 主表
    // ========================================================================

    for sql in &["CREATE TABLE IF NOT EXISTS paper_portfolios (\
            id TEXT NOT NULL PRIMARY KEY, \
            name TEXT NOT NULL, \
            source_event TEXT NOT NULL, \
            source_news_id TEXT, \
            source_screenshot_diagnosis_id TEXT, \
            status TEXT NOT NULL DEFAULT 'active', \
            created_at BIGINT NOT NULL, \
            closed_at BIGINT)"]
    {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 2: 创建 paper_positions 持仓表
    // ========================================================================

    for sql in &["CREATE TABLE IF NOT EXISTS paper_positions (\
            id TEXT NOT NULL PRIMARY KEY, \
            portfolio_id TEXT NOT NULL, \
            symbol TEXT NOT NULL, \
            market TEXT NOT NULL DEFAULT 'A', \
            entry_price REAL NOT NULL, \
            entry_date TEXT NOT NULL, \
            quantity REAL NOT NULL, \
            exit_price REAL, \
            exit_date TEXT, \
            status TEXT NOT NULL DEFAULT 'open', \
            note TEXT, \
            created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL, \
            FOREIGN KEY (portfolio_id) REFERENCES paper_portfolios(id) ON DELETE CASCADE)"]
    {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 3: 索引
    //   - 按 status 查：列出 active 组合
    //   - 按 source_news_id 查：新闻→组合溯源
    //   - 按 portfolio_id 查：组合→持仓列表
    //   - 按 (portfolio_id, status) 查：组合内未平仓持仓
    //   - 按 symbol 查：某标的在所有模拟组合中的表现
    // ========================================================================

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_paper_portfolios_status \
         ON paper_portfolios(status)",
        "CREATE INDEX IF NOT EXISTS idx_paper_portfolios_source_news \
         ON paper_portfolios(source_news_id)",
        "CREATE INDEX IF NOT EXISTS idx_paper_positions_portfolio_id \
         ON paper_positions(portfolio_id)",
        "CREATE INDEX IF NOT EXISTS idx_paper_positions_portfolio_status \
         ON paper_positions(portfolio_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_paper_positions_symbol \
         ON paper_positions(symbol)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v204_is_self_idempotent() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        // v204 不依赖其他表，但保持习惯先跑 v100
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：所有 CREATE 都是 IF NOT EXISTS，应幂等
        up(db).await.expect("v204 must be re-runnable in isolation");
    }

    #[tokio::test]
    async fn v204_creates_paper_portfolio_tables() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        // 验证表存在
        for table in &["paper_portfolios", "paper_positions"] {
            let row = db
                .query_one_raw(sea_orm::Statement::from_string(
                    sea_orm::DbBackend::Sqlite,
                    format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"),
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "表 {table} 应存在");
        }

        // 验证索引存在
        for idx in &[
            "idx_paper_portfolios_status",
            "idx_paper_portfolios_source_news",
            "idx_paper_positions_portfolio_id",
            "idx_paper_positions_portfolio_status",
            "idx_paper_positions_symbol",
        ] {
            let row = db
                .query_one_raw(sea_orm::Statement::from_sql_and_values(
                    sea_orm::DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='index' AND name=?",
                    [(*idx).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "索引 {idx} 应存在");
        }
    }
}
