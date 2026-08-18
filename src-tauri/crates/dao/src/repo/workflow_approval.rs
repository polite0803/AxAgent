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
    /// 审批超时自动裁决动作：auto_reject(默认) / auto_approve
    pub timeout_action: String,
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
        timeout_action: row
            .try_get::<String>("", "timeout_action")
            .ok()
            .unwrap_or_else(|| "auto_reject".to_string()),
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
                payload, timeout_action, timeout_secs, expires_at, created_at) \
               VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)";
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
            // timeout_action 升级兜底：旧结构体未提供时缺省 auto_reject
            if record.timeout_action.is_empty() {
                "auto_reject".to_string()
            } else {
                record.timeout_action.clone()
            }
            .into(),
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

/// 超时自动裁决结果，交由命令层联动引擎（拒→cancel / 放→resume）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeoutResolution {
    /// 超时默认拒绝：应取消该工作流
    Rejected { approval_id: String, execution_id: String },
    /// 配置了 auto_approve：应恢复该工作流
    Approved { approval_id: String, execution_id: String },
}

/// 对已超时的 pending 记录执行自动裁决（按 timeout_action 策略）。
///
/// 策略：
/// - `timeout_action = auto_reject`（默认，安全取向）：状态置 `rejected`，
///   `decision=rejected`，comment 标注"审批超时，默认拒绝"，返回 `Rejected`。
/// - `timeout_action = auto_approve`：状态置 `approved`，`decision=approved`，
///   返回 `Approved`。
///
/// 幂等：仅在 `status = 'pending'` 时生效，已被人工/其他入口解决过的记录
/// 不会重复处理（rows_affected = 0 即视为已处理，跳过）。
pub async fn auto_resolve_timeouts(
    db: &DatabaseConnection,
    now_ms: i64,
) -> Result<Vec<TimeoutResolution>, String> {
    // 查出所有超时的 pending 记录（含其 timeout_action 策略）
    let sql = "SELECT * FROM workflow_approvals \
               WHERE status = 'pending' AND expires_at > 0 AND expires_at <= ?1";
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(DbBackend::Sqlite, sql, vec![now_ms.into()]))
        .await
        .map_err(|e| e.to_string())?;
    let expired: Vec<WorkflowApprovalRecord> = rows.iter().filter_map(row_to_record).collect();

    let mut resolved = Vec::with_capacity(expired.len());
    for r in &expired {
        let is_auto_approve = {
            let ta = r.timeout_action.trim().to_ascii_lowercase();
            ta == "auto_approve" || ta == "approve"
        };
        let (status, decision, comment) = if is_auto_approve {
            ("approved", "approved", "审批超时，按策略自动批准")
        } else {
            ("rejected", "rejected", "审批超时，默认拒绝")
        };

        // 幂等更新：仅 pending 才生效
        let sql = "UPDATE workflow_approvals \
                   SET status = ?1, decision = ?2, approver_actual = 'system', \
                       comment = ?3, resolved_at = ?4 \
                   WHERE id = ?5 AND status = 'pending'";
        let affected = db
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                sql,
                vec![
                    status.to_string().into(),
                    decision.to_string().into(),
                    comment.to_string().into(),
                    now_ms.into(),
                    r.id.clone().into(),
                ],
            ))
            .await
            .map_err(|e| e.to_string())?;

        if affected.rows_affected() == 0 {
            // 已被人工或其他入口处理，跳过，不产生联动
            continue;
        }

        resolved.push(if is_auto_approve {
            TimeoutResolution::Approved {
                approval_id: r.id.clone(),
                execution_id: r.execution_id.clone(),
            }
        } else {
            TimeoutResolution::Rejected {
                approval_id: r.id.clone(),
                execution_id: r.execution_id.clone(),
            }
        });
    }

    Ok(resolved)
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

/// 引擎超时分支的幂等裁决落库：`approved`（auto_approve）或 `rejected`（默认拒绝）。
///
/// 与 DAO 层 `auto_resolve_timeouts` 共用同一语义，但以 execution_id + node_id
/// 定位单条记录。仅在 `status = 'pending'` 时生效（rows_affected 可为 0，
/// 表示已被人工或另一入口先裁决，此时无需报错）。
pub async fn resolve_approval_for_timeout(
    db: &DatabaseConnection,
    execution_id: &str,
    node_id: &str,
    approved: bool,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp_millis();
    let (status, decision, comment) = if approved {
        ("approved", "approved", "审批超时，按策略自动批准")
    } else {
        ("rejected", "rejected", "审批超时，默认拒绝")
    };
    let sql = "UPDATE workflow_approvals \
               SET status = ?1, decision = ?2, approver_actual = 'system', \
                   comment = ?3, resolved_at = ?4 \
               WHERE execution_id = ?5 AND node_id = ?6 AND status = 'pending'";
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        vec![
            status.to_string().into(),
            decision.to_string().into(),
            comment.to_string().into(),
            now.into(),
            execution_id.to_string().into(),
            node_id.to_string().into(),
        ],
    ))
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    /// 打开独立内存库并确保 workflow_approvals 表存在。
    async fn test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.expect("连接内存库应成功");
        db.execute_unprepared(
            "CREATE TABLE workflow_approvals (\
             id TEXT NOT NULL PRIMARY KEY, execution_id TEXT NOT NULL, node_id TEXT NOT NULL, \
             status TEXT NOT NULL DEFAULT 'pending', title TEXT NOT NULL DEFAULT '', \
             message TEXT NOT NULL DEFAULT '', approver TEXT, channels TEXT, payload TEXT, \
             decision TEXT, approver_actual TEXT, comment TEXT, \
             timeout_action TEXT NOT NULL DEFAULT 'auto_reject', \
             timeout_secs BIGINT NOT NULL DEFAULT 86400, expires_at BIGINT NOT NULL DEFAULT 0, \
             created_at BIGINT NOT NULL, resolved_at BIGINT)",
        )
        .await
        .expect("建表应成功");
        db
    }

    async fn insert_approval(
        db: &DatabaseConnection,
        id: &str,
        timeout_action: &str,
        expires_at: i64,
    ) {
        let sql = "INSERT INTO workflow_approvals \
                   (id, execution_id, node_id, status, title, message, approver, channels, \
                    payload, timeout_action, timeout_secs, expires_at, created_at) \
                   VALUES (?1, ?2, ?3, 'pending', ?4, '', NULL, NULL, NULL, ?5, 60, ?6, 0)";
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            vec![
                id.to_string().into(),
                format!("exec-{id}").into(),
                format!("node-{id}").into(),
                format!("审批-{id}").into(),
                timeout_action.to_string().into(),
                expires_at.into(),
            ],
        ))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn rejects_expired_by_default() {
        let db = test_db().await;
        insert_approval(&db, "a1", "auto_reject", 1000).await; // 已超时

        let res = auto_resolve_timeouts(&db, 9999999).await.unwrap();
        assert_eq!(res.len(), 1, "应裁决出一条拒绝记录");
        match &res[0] {
            TimeoutResolution::Rejected { approval_id, execution_id } => {
                assert_eq!(approval_id, "a1");
                assert_eq!(execution_id, "exec-a1");
            },
            _ => panic!("auto_reject 应返回 Rejected"),
        }

        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT status, decision FROM workflow_approvals WHERE id = ?1",
                ["a1".to_string().into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "status").unwrap(), "rejected");
        assert_eq!(row.try_get::<String>("", "decision").unwrap(), "rejected");
    }

    #[tokio::test]
    async fn approves_expired_when_auto_approve() {
        let db = test_db().await;
        insert_approval(&db, "a2", "auto_approve", 1000).await; // 已超时

        let res = auto_resolve_timeouts(&db, 9999999).await.unwrap();
        assert_eq!(res.len(), 1);
        match &res[0] {
            TimeoutResolution::Approved { approval_id, .. } => assert_eq!(approval_id, "a2"),
            _ => panic!("auto_approve 应返回 Approved"),
        }

        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT status, decision FROM workflow_approvals WHERE id = ?1",
                ["a2".to_string().into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "status").unwrap(), "approved");
        assert_eq!(row.try_get::<String>("", "decision").unwrap(), "approved");
    }

    #[tokio::test]
    async fn is_idempotent_for_already_resolved() {
        let db = test_db().await;
        insert_approval(&db, "a3", "auto_reject", 1000).await; // 已超时

        let first = auto_resolve_timeouts(&db, 9999999).await.unwrap();
        assert_eq!(first.len(), 1);

        let second = auto_resolve_timeouts(&db, 9999999).await.unwrap();
        assert!(second.is_empty(), "幂等：已解决的记录不应重复裁决");
    }

    #[tokio::test]
    async fn ignores_not_yet_expired() {
        let db = test_db().await;
        insert_approval(&db, "a4", "auto_reject", 99999999).await; // 未超时

        let res = auto_resolve_timeouts(&db, 9999).await.unwrap();
        assert!(res.is_empty(), "未超时的审批不应被裁决");
    }
}
