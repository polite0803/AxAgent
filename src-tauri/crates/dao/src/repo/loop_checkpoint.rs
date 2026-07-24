// SPDX-License-Identifier: AGPL-3.0-only

//! Loop 节点检查点持久化。
//!
//! 提供 save/load/delete 三个基本操作。底层表 `loop_checkpoints` 的 DDL 在
//! `axagent_dao::ddl` 中创建，复合主键 `(execution_id, node_id)`。
//!
//! 调用方：LoopExecutor（写检查点 + 删除已完成检查点）、
//! WorkEngine::resume_loop_iteration（按 execution_id + node_id 读检查点）。
//!
//! SQL 按后端分支：SQLite 用 `?N` 占位符 + `INSERT OR REPLACE`；
//! PostgreSQL 用 `$N` 占位符 + `ON CONFLICT ... DO UPDATE`，
//! 否则 PG 会把 `?1` 解析成 `?` 操作符 + integer，触发
//! "operator does not exist: ? integer" 错误。

use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::util_fns::now_ts;
use axagent_harness::workflow_types::LoopCheckpoint;
use sea_orm::ConnectionTrait;
use sea_orm::{DatabaseConnection, DbBackend, Statement};

/// 写入或覆盖一个 Loop 节点的检查点。
///
/// UPSERT 语义：同一 (execution_id, node_id) 已存在则替换 payload_json。
/// SQLite 用 `INSERT OR REPLACE`；PostgreSQL 用 `ON CONFLICT ... DO UPDATE`。
pub async fn save_loop_checkpoint(
    db: &DatabaseConnection,
    checkpoint: &LoopCheckpoint,
) -> Result<()> {
    let payload = serde_json::to_string(checkpoint)
        .map_err(|e| AxAgentError::Internal(format!("serialize LoopCheckpoint failed: {e}")))?;
    let values = vec![
        checkpoint.execution_id.clone().into(),
        checkpoint.node_id.clone().into(),
        payload.into(),
        (now_ts() as i64).into(),
    ];
    let stmt = if db.get_database_backend() == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO loop_checkpoints \
             (execution_id, node_id, payload_json, updated_at) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (execution_id, node_id) DO UPDATE SET \
             payload_json = EXCLUDED.payload_json, \
             updated_at = EXCLUDED.updated_at",
            values,
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT OR REPLACE INTO loop_checkpoints \
             (execution_id, node_id, payload_json, updated_at) \
             VALUES (?1, ?2, ?3, ?4)",
            values,
        )
    };
    db.execute_raw(stmt).await?;
    Ok(())
}

/// 按 (execution_id, node_id) 读取 Loop 检查点。找不到时返回 Ok(None)。
pub async fn load_loop_checkpoint(
    db: &DatabaseConnection,
    execution_id: &str,
    node_id: &str,
) -> Result<Option<LoopCheckpoint>> {
    let values = vec![execution_id.to_string().into(), node_id.to_string().into()];
    let stmt = if db.get_database_backend() == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT payload_json FROM loop_checkpoints \
             WHERE execution_id = $1 AND node_id = $2",
            values,
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT payload_json FROM loop_checkpoints \
             WHERE execution_id = ?1 AND node_id = ?2",
            values,
        )
    };
    let row = db.query_one_raw(stmt).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let payload: String =
        row.try_get::<String>("", "payload_json").map_err(AxAgentError::Database)?;
    let cp: LoopCheckpoint = serde_json::from_str(&payload)
        .map_err(|e| AxAgentError::Internal(format!("deserialize LoopCheckpoint failed: {e}")))?;
    Ok(Some(cp))
}

/// 删除指定 (execution_id, node_id) 的 Loop 检查点。
/// 在 Loop 整体完成（或用户取消）后调用，清理磁盘。
pub async fn delete_loop_checkpoint(
    db: &DatabaseConnection,
    execution_id: &str,
    node_id: &str,
) -> Result<()> {
    let values = vec![execution_id.to_string().into(), node_id.to_string().into()];
    let stmt = if db.get_database_backend() == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM loop_checkpoints \
             WHERE execution_id = $1 AND node_id = $2",
            values,
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "DELETE FROM loop_checkpoints \
             WHERE execution_id = ?1 AND node_id = ?2",
            values,
        )
    };
    db.execute_raw(stmt).await?;
    Ok(())
}

/// 删除指定 execution 的所有 Loop 检查点。
/// 在 cancel/reset 时调用，避免脏数据遗留。
pub async fn delete_loop_checkpoints_for_execution(
    db: &DatabaseConnection,
    execution_id: &str,
) -> Result<()> {
    let values = vec![execution_id.to_string().into()];
    let stmt = if db.get_database_backend() == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM loop_checkpoints WHERE execution_id = $1",
            values,
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "DELETE FROM loop_checkpoints WHERE execution_id = ?1",
            values,
        )
    };
    db.execute_raw(stmt).await?;
    Ok(())
}
