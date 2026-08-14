// SPDX-License-Identifier: AGPL-3.0-only

//! v221: OPC 需求发现相关表
//!
//! - `opc_demand_lead`：需求线索（平台来源 / 内容 / 预算 / 状态流转）
//! - `opc_delivery`：交付记录（关联 project + 产出路径 + 通知状态）
//! - `opc_market_platform`：市场平台连接器配置（闲鱼/猪八戒等，控制启停与抓取参数）
//! - `opc_capability_gap`：能力缺口记录（需求匹配时的热门/高价值需求若能力集不满足则落档）
//!
//! 注：`opc_capability` 能力清单快照表已移除。能力基座复用上游能力发现索引
//! （`capability_indexer` 的能力护照），不再本地冗余落库。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let create_demand_lead = r#"
CREATE TABLE IF NOT EXISTS opc_demand_lead (
    id TEXT NOT NULL PRIMARY KEY,
    platform TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    budget_min REAL,
    budget_max REAL,
    budget_currency TEXT NOT NULL DEFAULT 'CNY',
    contact_name TEXT,
    contact_email TEXT,
    contact_phone TEXT,
    source_url TEXT,
    raw_snapshot_json TEXT NOT NULL DEFAULT '{}',
    matched_capabilities_json TEXT NOT NULL DEFAULT '[]',
    ai_analysis_json TEXT NOT NULL DEFAULT '{}',
    recommended_workflow_id TEXT,
    status TEXT NOT NULL DEFAULT 'new',
    priority INTEGER NOT NULL DEFAULT 3,
    confidence REAL NOT NULL DEFAULT 0.0,
    notes TEXT NOT NULL DEFAULT '',
    project_id TEXT,
    customer_id TEXT,
    expires_at BIGINT,
    claimed_by TEXT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
)"#;

    let create_delivery = r#"
CREATE TABLE IF NOT EXISTS opc_delivery (
    id TEXT NOT NULL PRIMARY KEY,
    lead_id TEXT,
    project_id TEXT,
    customer_id TEXT,
    title TEXT NOT NULL,
    workflow_template_id TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    progress REAL NOT NULL DEFAULT 0.0,
    started_at BIGINT,
    completed_at BIGINT,
    result_summary TEXT,
    deliverables_json TEXT NOT NULL DEFAULT '[]',
    errors_json TEXT NOT NULL DEFAULT '[]',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
)"#;

    let create_market_platform = r#"
CREATE TABLE IF NOT EXISTS opc_market_platform (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    platform_type TEXT NOT NULL DEFAULT 'manual',
    enabled INTEGER NOT NULL DEFAULT 1,
    base_url TEXT,
    config_json TEXT NOT NULL DEFAULT '{}',
    last_sync_at BIGINT,
    status TEXT NOT NULL DEFAULT 'idle',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
)"#;

    let create_capability_gap = r#"
CREATE TABLE IF NOT EXISTS opc_capability_gap (
    id TEXT NOT NULL PRIMARY KEY,
    lead_id TEXT,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    missing_capability TEXT NOT NULL DEFAULT '',
    gap_type TEXT NOT NULL DEFAULT 'capability',
    suggested_action TEXT NOT NULL DEFAULT '',
    priority INTEGER NOT NULL DEFAULT 3,
    status TEXT NOT NULL DEFAULT 'open',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    closed_at BIGINT
)"#;

    let indices = [
        "CREATE INDEX IF NOT EXISTS idx_opc_demand_lead_platform ON opc_demand_lead(platform)",
        "CREATE INDEX IF NOT EXISTS idx_opc_demand_lead_status ON opc_demand_lead(status)",
        "CREATE INDEX IF NOT EXISTS idx_opc_demand_lead_created ON opc_demand_lead(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_opc_demand_lead_project ON opc_demand_lead(project_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_delivery_lead ON opc_delivery(lead_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_delivery_status ON opc_delivery(status)",
        "CREATE INDEX IF NOT EXISTS idx_opc_platform_enabled ON opc_market_platform(enabled)",
        "CREATE INDEX IF NOT EXISTS idx_opc_gap_lead ON opc_capability_gap(lead_id)",
        "CREATE INDEX IF NOT EXISTS idx_opc_gap_status ON opc_capability_gap(status)",
    ];

    db.execute_unprepared(create_demand_lead).await?;
    db.execute_unprepared(create_delivery).await?;
    db.execute_unprepared(create_market_platform).await?;
    db.execute_unprepared(create_capability_gap).await?;

    for stmt in &indices {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
