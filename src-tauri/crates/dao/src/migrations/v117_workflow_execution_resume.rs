// SPDX-License-Identifier: AGPL-3.0-only
//! v117: Add execution_state_json column to workflow_executions
//!
//! ## Background
//!
//! 为支持工作流「崩溃后恢复」功能，需要将 ExecutionState 的关键状态
//! 持久化到数据库。当应用重启后，可以从暂停状态恢复工作流执行。
//!
//! ## Strategy
//!
//! - 为 workflow_executions 表新增 execution_state_json 列
//! - 存储序列化后的 ExecutionStateSnapshot（包含 variables, node_records,
//!   current_node_id 等可恢复状态）
//! - 该列仅在 PAUSED 状态下写入，恢复后清空
//! - 同时新增 paused_at 列记录暂停时间，便于超时判断

use sea_orm::{DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 为 workflow_executions 添加 execution_state_json 列
    // ========================================================================
    exec_ddl(&db, is_pg, "ALTER TABLE workflow_executions ADD COLUMN execution_state_json TEXT")
        .await
        .or_else(|e| {
            // SQLite: ALTER TABLE ADD COLUMN 如果列已存在会报错，忽略即可
            tracing::warn!("[v117] execution_state_json 列可能已存在，忽略错误: {}", e);
            Ok::<(), DbErr>(())
        })?;

    // ========================================================================
    // PHASE 2: 为 workflow_executions 添加 paused_at 列
    // ========================================================================
    exec_ddl(&db, is_pg, "ALTER TABLE workflow_executions ADD COLUMN paused_at BIGINT")
        .await
        .or_else(|e| {
            tracing::warn!("[v117] paused_at 列可能已存在，忽略错误: {}", e);
            Ok::<(), DbErr>(())
        })?;

    // ========================================================================
    // PHASE 3: 为 paused 状态添加索引，加速恢复查询
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_workflow_executions_paused ON workflow_executions(status) WHERE status = 'paused'",
    )
    .await
    .or_else(|e| {
        tracing::warn!(
            "[v117] idx_workflow_executions_paused 索引可能已存在，忽略错误: {}",
            e
        );
        Ok::<(), DbErr>(())
    })?;

    Ok(())
}
