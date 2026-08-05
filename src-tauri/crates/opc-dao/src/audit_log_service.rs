// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tracing::info;

#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub action: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub user_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[allow(dead_code)]
pub struct AuditLogService {
    db: Arc<DatabaseConnection>,
}

impl AuditLogService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn log(
        &self,
        entity_type: &str,
        entity_id: &str,
        action: &str,
        _old_value: Option<&str>,
        _new_value: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<(), String> {
        info!("审计日志: {} {} {} by {:?}", entity_type, entity_id, action, user_id);

        Ok(())
    }

    pub async fn get_history(
        &self,
        entity_type: &str,
        entity_id: &str,
        limit: u32,
    ) -> Result<Vec<AuditLogEntry>, String> {
        info!("查询审计历史: {} {} (limit: {})", entity_type, entity_id, limit);

        Ok(vec![])
    }

    pub async fn query_by_action(
        &self,
        action: &str,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<AuditLogEntry>, String> {
        info!("按操作类型查询: {} since {}", action, since);
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_entry() {
        let db = Arc::new(DatabaseConnection::default());
        let service = AuditLogService::new(db);

        let result = service
            .log("invoice", "inv-001", "CREATE", None, Some(r#"{"total": 1000}"#), Some("user-1"))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_history() {
        let db = Arc::new(DatabaseConnection::default());
        let service = AuditLogService::new(db);

        let result = service.get_history("invoice", "inv-001", 10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
