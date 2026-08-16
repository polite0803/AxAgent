// SPDX-License-Identifier: AGPL-3.0-only

//! 进化产物执行统计 repository —— 持久化真实执行反馈（阶段四后置闭环 · D3）。
//!
//! `EvolutionFeedbackSinkImpl::record` 在更新内存统计后异步调用
//! [`upsert_execution_feedback`] 落库；应用启动时 [`load_all_execution_stats`]
//! 一次性读回内存，保证重启后真实执行证据不丢失。
//!
//! 表 `evolution_execution_stats` 的 DDL 在 `axagent_dao::migrations::v122`
//! 中创建，复合主键 `(conversation_id, tool_id)`。

use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::workflow_evolution::ToolExecutionStats;
use sea_orm::ConnectionTrait;
use sea_orm::{DatabaseConnection, DbBackend, Statement};
use std::collections::HashMap;

/// UPSERT 一次执行反馈：`usage_count + 1`，并按成败累计 successes / failures。
///
/// 用增量更新的 `ON CONFLICT ... DO UPDATE`（而非 `INSERT OR REPLACE`，
/// 后者会整行覆盖、丢失既有计数）。
/// SQLite 用 `?N` 占位符；PostgreSQL 用 `$N` 占位符（否则 PG 会把 `?1`
/// 解析成 `?` 操作符 + integer，触发 "operator does not exist: ? integer"）。
pub async fn upsert_execution_feedback(
    db: &DatabaseConnection,
    conversation_id: &str,
    tool_id: &str,
    success: bool,
) -> Result<()> {
    let success_inc: i64 = if success { 1 } else { 0 };
    let failure_inc: i64 = if success { 0 } else { 1 };
    let values = vec![
        conversation_id.to_string().into(),
        tool_id.to_string().into(),
        success_inc.into(),
        failure_inc.into(),
    ];
    let stmt = if db.get_database_backend() == DbBackend::Postgres {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO evolution_execution_stats \
             (conversation_id, tool_id, usage_count, successes, failures) \
             VALUES ($1, $2, 1, $3, $4) \
             ON CONFLICT (conversation_id, tool_id) DO UPDATE SET \
             usage_count = evolution_execution_stats.usage_count + 1, \
             successes = evolution_execution_stats.successes + EXCLUDED.successes, \
             failures = evolution_execution_stats.failures + EXCLUDED.failures",
            values,
        )
    } else {
        Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO evolution_execution_stats \
             (conversation_id, tool_id, usage_count, successes, failures) \
             VALUES (?1, ?2, 1, ?3, ?4) \
             ON CONFLICT (conversation_id, tool_id) DO UPDATE SET \
             usage_count = usage_count + 1, \
             successes = successes + excluded.successes, \
             failures = failures + excluded.failures",
            values,
        )
    };
    db.execute_raw(stmt).await.map_err(AxAgentError::Database)?;
    Ok(())
}

/// 一次性读取全部持久化的执行统计，按 `conversation_id → tool_id → stats` 组装。
///
/// 启动时调用，把上次会话的真实执行证据加载回 `AppState.evolution_execution_stats`。
/// 空表返回空 HashMap（正常，非错误）。
pub async fn load_all_execution_stats(
    db: &DatabaseConnection,
) -> Result<HashMap<String, HashMap<String, ToolExecutionStats>>> {
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT conversation_id, tool_id, usage_count, successes, failures \
             FROM evolution_execution_stats",
        ))
        .await?;

    let mut result: HashMap<String, HashMap<String, ToolExecutionStats>> = HashMap::new();
    for row in rows {
        let conv: String = row.try_get_by("conversation_id").unwrap_or_default();
        let tool: String = row.try_get_by("tool_id").unwrap_or_default();
        let usage_count: u32 = row.try_get_by::<i64, _>("usage_count").unwrap_or(0).max(0) as u32;
        let successes: u32 = row.try_get_by::<i64, _>("successes").unwrap_or(0).max(0) as u32;
        let failures: u32 = row.try_get_by::<i64, _>("failures").unwrap_or(0).max(0) as u32;
        result
            .entry(conv)
            .or_default()
            .insert(tool, ToolExecutionStats { usage_count, successes, failures });
    }
    Ok(result)
}
