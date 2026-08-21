// SPDX-License-Identifier: AGPL-3.0-only

//! v222: OPC 需求线索表扩展 - 添加需求价值评估字段
//!
//! 为 `opc_demand_lead` 表添加以下字段：
//! - `pain_score`: 痛点分（0-100）
//! - `market_gap_score`: 市场缺口分（0-100）
//! - `commercial_value_score`: 商业价值分（0-100）
//! - `opportunity_level`: 机会等级（low/medium/high/very_high）
//! - `demand_type`: 需求类型（tool_software/design/development 等）
//! - `evaluated_at`: 评估时间
//!
//! 这些字段用于存储 Phase 2 需求价值评估引擎的输出结果。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // 添加评估分数字段
    let alter_statements = [
        // 痛点分
        "ALTER TABLE opc_demand_lead ADD COLUMN pain_score REAL NOT NULL DEFAULT 0.0",
        // 市场缺口分
        "ALTER TABLE opc_demand_lead ADD COLUMN market_gap_score REAL NOT NULL DEFAULT 0.0",
        // 商业价值分（综合分）
        "ALTER TABLE opc_demand_lead ADD COLUMN commercial_value_score REAL NOT NULL DEFAULT 0.0",
        // 机会等级
        "ALTER TABLE opc_demand_lead ADD COLUMN opportunity_level TEXT NOT NULL DEFAULT 'low'",
        // 需求类型
        "ALTER TABLE opc_demand_lead ADD COLUMN demand_type TEXT NOT NULL DEFAULT 'unknown'",
        // 评估时间
        "ALTER TABLE opc_demand_lead ADD COLUMN evaluated_at INTEGER",
    ];

    for stmt in &alter_statements {
        // SQLite/PG 的 ALTER TABLE 限制：只能 ADD COLUMN
        // 使用 try-catch 模式，字段已存在时跳过
        match db.execute_unprepared(stmt).await {
            Ok(_) => tracing::info!("[v222] 执行成功: {}", stmt),
            Err(e) => {
                let err_str = e.to_string();
                // 忽略 "duplicate column name" 错误（字段已存在）
                // 兼容中英文错误消息：PostgreSQL 中文本地化返回 "已经存在"
                if err_str.contains("duplicate column")
                    || err_str.contains("already exists")
                    || err_str.contains("已经存在")
                {
                    tracing::warn!("[v222] 字段可能已存在，跳过: {} ({})", stmt, err_str);
                } else {
                    return Err(e);
                }
            },
        }
    }

    // 添加索引（用于查询优化）
    let index_statements = [
        "CREATE INDEX IF NOT EXISTS idx_opc_demand_lead_value_score ON opc_demand_lead(commercial_value_score)",
        "CREATE INDEX IF NOT EXISTS idx_opc_demand_lead_opportunity ON opc_demand_lead(opportunity_level)",
        "CREATE INDEX IF NOT EXISTS idx_opc_demand_lead_type ON opc_demand_lead(demand_type)",
        "CREATE INDEX IF NOT EXISTS idx_opc_demand_lead_evaluated ON opc_demand_lead(evaluated_at)",
    ];

    for stmt in &index_statements {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
