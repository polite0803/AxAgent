// SPDX-License-Identifier: AGPL-3.0-only
//! v205_market_mainline: G4 市场主线自动提炼
//!
//! ## 背景
//!
//! DojoAgents 宣传场景 4「市场发现」依赖每日自动提炼市场主线：
//! - 多源数据采集（热点股 / 龙虎榜 / 北向 / 涨停板 / 财联社快讯）
//! - LLM 分类主题（AI 算力 / 半导体 / 新能源 / 消费 / 医药 / 周期 …）
//! - 过滤信号（去除噪音 / 合并相似主题）
//! - 综合主线（每条主线给出代表性标的 + 强度评分 + 持续性判断）
//! - 持久化到 DB → 推送到 Dashboard
//!
//! AxInvest 已有 `HotStocksPanel` / `DragonTigerPanel` / `NorthBoundPanel` /
//! `LimitUpPanel` / `ClsFlashPanel` 等数据源，但缺少「主线提炼 + 持久化 +
//! 跨日跟踪」能力。
//!
//! ## 本迁移创建的表
//!
//! - `market_mainlines`：市场主线记录（每日 N 条主线，每条主线包含代表性标的 + 强度 + 状态）
//!
//! ## 字段语义
//!
//! - `id`：UUID
//! - `mainline_date`：主线日期 YYYY-MM-DD（用于按日查询）
//! - `theme`：主题名（如 "AI 算力" / "光模块" / "新能源车"）
//! - `theme_category`：主题大类（科技 / 消费 / 周期 / 金融 / 医药 / 政策）
//! - `narrative`：主线叙述（LLM 综合的 1-2 句话故事线）
//! - `representative_symbols`：代表性标的 JSON 数组（如 ["600519","000858"]）
//! - `strength_score`：强度评分 0-100（综合涨停数 / 资金流入 / 龙头表现）
//! - `persistence`：持续性判断（"1d" / "1w" / "1m" / "fading" / "emerging"）
//! - `evidence_json`：证据 JSON（涨停股 / 北向净流入 / 龙虎榜数据等原始数据快照）
//! - `source_workflow_execution_id`：来源工作流执行 ID（可空，手动创建则为 null）
//! - `status`：active / fading / archived
//! - `created_at` / `updated_at`：时间戳（ms）
//!
//! ## DDL 风格
//!
//! 与 v204 保持一致：直接写 PG 语法，SQLite 侧由
//! [`sqlite_ddl`](super::pg_ddl::sqlite_ddl) 自动转换。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 创建 market_mainlines 主表
    // ========================================================================

    for sql in &["CREATE TABLE IF NOT EXISTS market_mainlines (\
            id TEXT NOT NULL PRIMARY KEY, \
            mainline_date TEXT NOT NULL, \
            theme TEXT NOT NULL, \
            theme_category TEXT NOT NULL DEFAULT '其他', \
            narrative TEXT NOT NULL, \
            representative_symbols TEXT NOT NULL DEFAULT '[]', \
            strength_score REAL NOT NULL DEFAULT 0.0, \
            persistence TEXT NOT NULL DEFAULT '1d', \
            evidence_json TEXT NOT NULL DEFAULT '{}', \
            source_workflow_execution_id TEXT, \
            status TEXT NOT NULL DEFAULT 'active', \
            created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL)"]
    {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 2: 索引
    //   - 按 mainline_date 查：列出某日所有主线
    //   - 按 (mainline_date, theme) 查：去重判断（同日同主题只保留一条）
    //   - 按 theme_category 查：按大类过滤
    //   - 按 status 查：active / fading / archived
    //   - 按 strength_score 查：按强度排序
    //   - 按 source_workflow_execution_id 查：工作流执行溯源
    // ========================================================================

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_market_mainlines_date \
         ON market_mainlines(mainline_date)",
        "CREATE INDEX IF NOT EXISTS idx_market_mainlines_date_theme \
         ON market_mainlines(mainline_date, theme)",
        "CREATE INDEX IF NOT EXISTS idx_market_mainlines_category \
         ON market_mainlines(theme_category)",
        "CREATE INDEX IF NOT EXISTS idx_market_mainlines_status \
         ON market_mainlines(status)",
        "CREATE INDEX IF NOT EXISTS idx_market_mainlines_strength \
         ON market_mainlines(strength_score DESC)",
        "CREATE INDEX IF NOT EXISTS idx_market_mainlines_workflow \
         ON market_mainlines(source_workflow_execution_id)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v205_is_self_idempotent() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        up(db).await.expect("v205 必须可单独重复执行");
    }

    #[tokio::test]
    async fn v205_creates_market_mainlines_table() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' AND name='market_mainlines'",
            ))
            .await
            .unwrap();
        assert!(row.is_some(), "market_mainlines 表应存在");

        for idx in &[
            "idx_market_mainlines_date",
            "idx_market_mainlines_date_theme",
            "idx_market_mainlines_category",
            "idx_market_mainlines_status",
            "idx_market_mainlines_strength",
            "idx_market_mainlines_workflow",
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
