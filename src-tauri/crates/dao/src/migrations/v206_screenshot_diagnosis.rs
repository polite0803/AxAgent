// SPDX-License-Identifier: AGPL-3.0-only
//! v206_screenshot_diagnosis: G6 截图持仓诊断完整闭环
//!
//! ## 背景
//!
//! DojoAgents 宣传场景 3「截图持仓诊断」依赖：
//! - 用户上传券商 App / 同花顺 / 东方财富 / 雪球截图
//! - LLM 视觉管线（OCR + 结构化解析）提取持仓列表
//! - 风险诊断 schema 计算（concentration_risk / overlap_positions /
//!   defense_ratio / us_exposure / weak_exposure / repeated_positions /
//!   core_concentration）
//! - 输出观察列表（可一键转为 paper_portfolio）
//! - 持久化到 screenshot_diagnoses 表，便于后续回看 / 复盘
//!
//! AxInvest 已有 `kit::screen_vision` / `providers::screen_vision` /
//! `agent::VisionPipeline`（OCR + ChartAnalysis 等任务），但缺少：
//! - 截图持仓诊断专属表
//! - 风险诊断 schema 标准化输出
//! - 与 paper_portfolio 联动入口（已在 v204 预留
//!   `source_screenshot_diagnosis_id` 外键字段）
//!
//! ## 本迁移创建的表
//!
//! - `screenshot_diagnoses`：截图诊断记录（每次截图诊断 = 一行记录，
//!   含截图元信息 + 提取的持仓 JSON + 风险诊断 JSON + 状态）
//!
//! ## 字段语义
//!
//! - `id`：UUID
//! - `image_hash`：截图 SHA256（用于去重，避免同一截图重复诊断）
//! - `image_path`：截图本地存储路径（可选，若存了原图）
//! - `image_thumbnail_base64`：缩略图 base64（可选，前端列表预览用）
//! - `image_width` / `image_height`：原图尺寸
//! - `source_app`：截图来源 App（同花顺 / 东方财富 / 雪球 / 通达信 / 其他）
//! - `ocr_text`：LLM 视觉管线 OCR 提取的完整文本（debug 用）
//! - `positions_json`：结构化持仓 JSON 数组（每项含 code/name/qty/cost_price/market_value/weight）
//! - `total_market_value`：截图时刻总市值（用于权重计算）
//! - `diagnosis_json`：风险诊断 JSON（concentration_risk / overlap_positions /
//!   defense_ratio / us_exposure / weak_exposure / repeated_positions /
//!   core_concentration 七项）
//! - `narrative`：LLM 自然语言诊断说明（1-3 段中文）
//! - `recommended_actions`：建议动作 JSON 数组（如 "减持 X" / "分散 Y" / "关注 Z 行业"）
//! - `source_workflow_execution_id`：来源工作流执行 ID（可空，手动上传则为 null）
//! - `provider_id` / `model_id`：使用的 LLM provider / model（溯源 + 复算用）
//! - `status`：active / archived / failed
//! - `error_message`：若失败，错误原因
//! - `created_at` / `updated_at`：时间戳（ms）
//!
//! ## DDL 风格
//!
//! 与 v204/v205 保持一致：直接写 PG 语法，SQLite 侧由
//! [`sqlite_ddl`](super::pg_ddl::sqlite_ddl) 自动转换。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 创建 screenshot_diagnoses 主表
    // ========================================================================

    for sql in &["CREATE TABLE IF NOT EXISTS screenshot_diagnoses (\
            id TEXT NOT NULL PRIMARY KEY, \
            image_hash TEXT, \
            image_path TEXT, \
            image_thumbnail_base64 TEXT, \
            image_width INTEGER, \
            image_height INTEGER, \
            source_app TEXT, \
            ocr_text TEXT, \
            positions_json TEXT NOT NULL DEFAULT '[]', \
            total_market_value REAL NOT NULL DEFAULT 0.0, \
            diagnosis_json TEXT NOT NULL DEFAULT '{}', \
            narrative TEXT NOT NULL DEFAULT '', \
            recommended_actions TEXT NOT NULL DEFAULT '[]', \
            source_workflow_execution_id TEXT, \
            provider_id TEXT, \
            model_id TEXT, \
            status TEXT NOT NULL DEFAULT 'active', \
            error_message TEXT, \
            created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL)"]
    {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 2: 索引
    //   - 按 image_hash 查：去重判断（同一截图不重复诊断）
    //   - 按 created_at 查：按时间倒序列出
    //   - 按 status 查：active / archived / failed
    //   - 按 source_app 查：按 App 过滤
    //   - 按 source_workflow_execution_id 查：工作流执行溯源
    // ========================================================================

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_screenshot_diagnoses_hash \
         ON screenshot_diagnoses(image_hash)",
        "CREATE INDEX IF NOT EXISTS idx_screenshot_diagnoses_created \
         ON screenshot_diagnoses(created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_screenshot_diagnoses_status \
         ON screenshot_diagnoses(status)",
        "CREATE INDEX IF NOT EXISTS idx_screenshot_diagnoses_source_app \
         ON screenshot_diagnoses(source_app)",
        "CREATE INDEX IF NOT EXISTS idx_screenshot_diagnoses_workflow \
         ON screenshot_diagnoses(source_workflow_execution_id)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v206_is_self_idempotent() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        up(db).await.expect("v206 必须可单独重复执行");
    }

    #[tokio::test]
    async fn v206_creates_screenshot_diagnoses_table() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' AND name='screenshot_diagnoses'",
            ))
            .await
            .unwrap();
        assert!(row.is_some(), "screenshot_diagnoses 表应存在");

        for idx in &[
            "idx_screenshot_diagnoses_hash",
            "idx_screenshot_diagnoses_created",
            "idx_screenshot_diagnoses_status",
            "idx_screenshot_diagnoses_source_app",
            "idx_screenshot_diagnoses_workflow",
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
