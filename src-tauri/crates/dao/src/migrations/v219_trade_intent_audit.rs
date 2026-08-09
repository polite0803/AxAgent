// SPDX-License-Identifier: AGPL-3.0-only

//! v219 — stock_analyses 交易意图审核流转扩展
//!
//! ## 背景
//!
//! 股票业务的分析闭环已实现，但交易记录仍依赖手动。
//! 为实现"安全的交易记录自动化"（不执行真实交易，仅记录意图供人工审核），
//! 在 stock_analyses 表上扩展交易意图状态流转字段。
//!
//! ## 新增字段
//!
//! - trade_intent_status   : 交易意图审核状态 (pending/reviewed/executed/expired/rejected)
//! - trade_intent_source   : 意图来源 (analysis/conditional_order/quant_signal/portfolio_monitor)
//! - trade_intent_source_ref_id : 来源关联 ID（分析ID/条件单ID/信号ID）
//! - trade_intent_reviewed_at   : 审核时间（ms）
//! - trade_intent_reviewed_by   : 审核人
//! - trade_intent_review_notes  : 审核备注
//! - trade_intent_actual_trade_id : 关联的实际交易 ID（执行后关联到 trades 表）
//!
//! ## 设计理念
//!
//! - 系统自动记录交易"建议"，不自动执行真实交易
//! - 支持人工审核流程：pending → reviewed → （手动执行/过期/驳回）
//! - 复用 stock_analyses 现有 decision 字段，不新增表
//!
//! ## 幂等性
//!
//! 所有 ALTER TABLE 使用 IF NOT EXISTS，repair_schema 重跑安全。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = matches!(db.get_database_backend(), DbBackend::Postgres);

    // 1) 交易意图审核状态
    let sql_status = if is_pg {
        "ALTER TABLE stock_analyses ADD COLUMN IF NOT EXISTS trade_intent_status TEXT NOT NULL DEFAULT 'pending'"
    } else {
        "ALTER TABLE stock_analyses ADD COLUMN trade_intent_status TEXT NOT NULL DEFAULT 'pending'"
    };
    db.execute_unprepared(sql_status).await?;

    // 2) 意图来源
    let sql_source = if is_pg {
        "ALTER TABLE stock_analyses ADD COLUMN IF NOT EXISTS trade_intent_source TEXT"
    } else {
        "ALTER TABLE stock_analyses ADD COLUMN trade_intent_source TEXT"
    };
    db.execute_unprepared(sql_source).await?;

    // 3) 来源关联 ID
    let sql_ref = if is_pg {
        "ALTER TABLE stock_analyses ADD COLUMN IF NOT EXISTS trade_intent_source_ref_id TEXT"
    } else {
        "ALTER TABLE stock_analyses ADD COLUMN trade_intent_source_ref_id TEXT"
    };
    db.execute_unprepared(sql_ref).await?;

    // 4) 审核时间
    let sql_reviewed_at = if is_pg {
        "ALTER TABLE stock_analyses ADD COLUMN IF NOT EXISTS trade_intent_reviewed_at BIGINT"
    } else {
        "ALTER TABLE stock_analyses ADD COLUMN trade_intent_reviewed_at BIGINT"
    };
    db.execute_unprepared(sql_reviewed_at).await?;

    // 5) 审核人
    let sql_reviewed_by = if is_pg {
        "ALTER TABLE stock_analyses ADD COLUMN IF NOT EXISTS trade_intent_reviewed_by TEXT"
    } else {
        "ALTER TABLE stock_analyses ADD COLUMN trade_intent_reviewed_by TEXT"
    };
    db.execute_unprepared(sql_reviewed_by).await?;

    // 6) 审核备注
    let sql_review_notes = if is_pg {
        "ALTER TABLE stock_analyses ADD COLUMN IF NOT EXISTS trade_intent_review_notes TEXT"
    } else {
        "ALTER TABLE stock_analyses ADD COLUMN trade_intent_review_notes TEXT"
    };
    db.execute_unprepared(sql_review_notes).await?;

    // 7) 关联实际交易 ID
    let sql_actual_trade = if is_pg {
        "ALTER TABLE stock_analyses ADD COLUMN IF NOT EXISTS trade_intent_actual_trade_id TEXT"
    } else {
        "ALTER TABLE stock_analyses ADD COLUMN trade_intent_actual_trade_id TEXT"
    };
    db.execute_unprepared(sql_actual_trade).await?;

    // 8) 创建索引：按状态查询待审核列表
    let sql_idx_status = "CREATE INDEX IF NOT EXISTS idx_stock_analyses_trade_intent_status ON stock_analyses(trade_intent_status)";
    db.execute_unprepared(sql_idx_status).await?;

    // 9) 创建索引：按来源查询
    let sql_idx_source = "CREATE INDEX IF NOT EXISTS idx_stock_analyses_trade_intent_source ON stock_analyses(trade_intent_source)";
    db.execute_unprepared(sql_idx_source).await?;

    // 10) 为存量数据回填默认值（已完成的分析默认为 reviewed，因为是历史数据）
    db.execute_unprepared(
        "UPDATE stock_analyses SET trade_intent_status = 'reviewed' WHERE status = 'completed' AND trade_intent_status = 'pending'",
    )
    .await?;

    Ok(())
}
