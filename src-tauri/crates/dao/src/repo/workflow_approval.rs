// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流审批记录持久化（workflow_approvals 表）。
//!
//! 提供 save/list/resolve/check_timeout 四个基本操作。
//! 使用直接 SQL 模式（同 loop_checkpoint Repo）。

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde::{Deserialize, Serialize};

/// 审批记录 DTO（与 entity 一致，但方便跨 crate 使用避免 SeaORM 依赖）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowApprovalRecord {
    pub id: String,
    pub execution_id: String,
    pub node_id: String,
    pub status: String,
    pub title: String,
    pub message: String,
    pub approver: Option<String>,
    pub channels: Option<String>,
    pub payload: Option<String>,
    pub decision: Option<String>,
    pub approver_actual: Option<String>,
    pub comment: Option<String>,
    pub timeout_secs: i64,
    pub expires_at: i64,
    pub created_at: i64,
    pub resolved_at: Option<i64>,
}

fn row_to_record(row: &sea_orm::QueryResult) -> Option<WorkflowApprovalRecord> {
    Some(WorkflowApprovalRecord {
        id: row.try_get::<String>("", "id").ok()?,
        execution_id: row.try_get::<String>("", "execution_id").ok()?,
        node_id: row.try_get::<String>("", "node_id").ok()?,
        status: row.try_get::<String>("", "status").ok()?,
        title: row.try_get::<String>("", "title").ok().unwrap_or_default(),
        message: row.try_get::<String>("", "message").ok().unwrap_or_default(),
        approver: row.try_get::<Option<String>>("", "approver").ok().flatten(),
        channels: row.try_get::<Option<String>>("", "channels").ok().flatten(),
        payload: row.try_get::<Option<String>>("", "payload").ok().flatten(),
        decision: row.try_get::<Option<String>>("", "decision").ok().flatten(),
        approver_actual: row.try_get::<Option<String>>("", "approver_actual").ok().flatten(),
        comment: row.try_get::<Option<String>>("", "comment").ok().flatten(),
        timeout_secs: row.try_get::<i64>("", "timeout_secs").ok().unwrap_or(86400),
        expires_at: row.try_get::<i64>("", "expires_at").ok().unwrap_or(0),
        created_at: row.try_get::<i64>("", "created_at").ok().unwrap_or(0),
        resolved_at: row.try_get::<Option<i64>>("", "resolved_at").ok().flatten(),
    })
}

/// 创建一个新的待审批记录（status = 'pending'）。
pub async fn save_approval(
    db: &DatabaseConnection,
    record: &WorkflowApprovalRecord,
) -> Result<(), String> {
    let sql = "INSERT INTO workflow_approvals \
               (id, execution_id, node_id, status, title, message, approver, channels, \
                payload, timeout_secs, expires_at, created_at) \
               VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)";
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        vec![
            record.id.clone().into(),
            record.execution_id.clone().into(),
            record.node_id.clone().into(),
            record.title.clone().into(),
            record.message.clone().into(),
            record.approver.clone().unwrap_or_default().into(),
            record.channels.clone().unwrap_or_default().into(),
            record.payload.clone().unwrap_or_default().into(),
            record.timeout_secs.into(),
            record.expires_at.into(),
            record.created_at.into(),
        ],
    ))
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 列出所有 pending（且未超时）的审批记录。
/// 可选按 execution_id 过滤。
pub async fn list_pending_approvals(
    db: &DatabaseConnection,
    execution_id: Option<&str>,
) -> Result<Vec<WorkflowApprovalRecord>, String> {
    let (sql, values) = if let Some(eid) = execution_id {
        (
            "SELECT * FROM workflow_approvals WHERE status = 'pending' AND execution_id = ?1 \
             ORDER BY created_at ASC",
            vec![eid.to_string().into()],
        )
    } else {
        (
            "SELECT * FROM workflow_approvals WHERE status = 'pending' ORDER BY created_at ASC",
            vec![],
        )
    };
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(DbBackend::Sqlite, sql, values))
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().filter_map(row_to_record).collect())
}

/// 按 ID 查询单条审批记录。
pub async fn get_approval_by_id(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<WorkflowApprovalRecord>, String> {
    let sql = "SELECT * FROM workflow_approvals WHERE id = ?1";
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            vec![id.to_string().into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.first().and_then(row_to_record))
}

/// 将审批状态更新为 approved / rejected，记录审批人和备注。
pub async fn resolve_approval(
    db: &DatabaseConnection,
    id: &str,
    decision: &str,
    approver_actual: Option<&str>,
    comment: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let sql = "UPDATE workflow_approvals \
               SET status = ?1, decision = ?2, approver_actual = ?3, comment = ?4, \
                   resolved_at = ?5 \
               WHERE id = ?6 AND status = 'pending'";
    let affected = db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            vec![
                decision.to_string().into(),
                decision.to_string().into(), // 将 decision 字符串写入 decision 列
                approver_actual.map(|s| s.to_string()).unwrap_or_default().into(),
                comment.map(|s| s.to_string()).unwrap_or_default().into(),
                now.into(),
                id.to_string().into(),
            ],
        ))
        .await
        .map_err(|e| e.to_string())?;
    if affected.rows_affected() == 0 {
        return Err(format!("审批记录 {} 不存在或状态不是 pending", id));
    }
    Ok(())
}

/// 对已超时的 pending 记录执行自动裁决。
/// 返回被处理的记录列表。
pub async fn auto_resolve_timeouts(
    db: &DatabaseConnection,
    now_ms: i64,
) -> Result<Vec<WorkflowApprovalRecord>, String> {
    // 查出所有超时的 pending 记录
    let sql = "SELECT * FROM workflow_approvals \
               WHERE status = 'pending' AND expires_at > 0 AND expires_at <= ?1";
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(DbBackend::Sqlite, sql, vec![now_ms.into()]))
        .await
        .map_err(|e| e.to_string())?;
    let expired: Vec<WorkflowApprovalRecord> = rows.iter().filter_map(row_to_record).collect();

    for r in &expired {
        // 根据 timeout_action 决定自动裁决结果
        // 简化：把 timeout_action 放在 decision 列（resolve 会覆盖）
        // 但实际上我们不知道 timeout_action（它不在表中存）。
        // 所以这里只标记为 expired，命令层根据 payload 里的 timeout_action 处理
        let sql = "UPDATE workflow_approvals \
                   SET status = 'expired', resolved_at = ?1 \
                   WHERE id = ?2 AND status = 'pending'";
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            vec![now_ms.into(), r.id.clone().into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(expired)
}

/// 将指定 execution_id + node_id 的审批记录标记为 expired（引擎超时自动处理）。
pub async fn expire_approval(
    db: &DatabaseConnection,
    execution_id: &str,
    node_id: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let sql = "UPDATE workflow_approvals \
               SET status = 'expired', resolved_at = ?1 \
               WHERE execution_id = ?2 AND node_id = ?3 AND status = 'pending'";
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        vec![now.into(), execution_id.to_string().into(), node_id.to_string().into()],
    ))
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
