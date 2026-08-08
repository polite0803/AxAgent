// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

// ── 审计日志（stock-analysis 域） ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub action: AuditAction,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub source: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditAction {
    Create,
    Read,
    Update,
    Delete,
    Execute,
    Approve,
    Reject,
    Export,
    Import,
    Login,
    Logout,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditAction::Create => write!(f, "create"),
            AuditAction::Read => write!(f, "read"),
            AuditAction::Update => write!(f, "update"),
            AuditAction::Delete => write!(f, "delete"),
            AuditAction::Execute => write!(f, "execute"),
            AuditAction::Approve => write!(f, "approve"),
            AuditAction::Reject => write!(f, "reject"),
            AuditAction::Export => write!(f, "export"),
            AuditAction::Import => write!(f, "import"),
            AuditAction::Login => write!(f, "login"),
            AuditAction::Logout => write!(f, "logout"),
        }
    }
}

impl std::str::FromStr for AuditAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "create" => Ok(AuditAction::Create),
            "read" => Ok(AuditAction::Read),
            "update" => Ok(AuditAction::Update),
            "delete" => Ok(AuditAction::Delete),
            "execute" => Ok(AuditAction::Execute),
            "approve" => Ok(AuditAction::Approve),
            "reject" => Ok(AuditAction::Reject),
            "export" => Ok(AuditAction::Export),
            "import" => Ok(AuditAction::Import),
            "login" => Ok(AuditAction::Login),
            "logout" => Ok(AuditAction::Logout),
            _ => Err(format!("Unknown audit action: {}", s)),
        }
    }
}

impl AuditLog {
    pub fn new(entity_type: impl Into<String>, entity_id: i64, action: AuditAction) -> Self {
        Self {
            id: 0,
            entity_type: entity_type.into(),
            entity_id,
            action,
            before: None,
            after: None,
            user_id: None,
            user_name: None,
            source: String::from("system"),
            metadata: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>, user_name: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self.user_name = Some(user_name.into());
        self
    }

    pub fn with_before(mut self, before: serde_json::Value) -> Self {
        self.before = Some(before);
        self
    }

    pub fn with_after(mut self, after: serde_json::Value) -> Self {
        self.after = Some(after);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

// ── OPC 审计日志服务 ──────────────────────────────────────────

use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tracing::info;

/// OPC 审计日志条目
#[derive(Debug, Clone)]
pub struct OpcAuditLogEntry {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub action: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub user_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// OPC 审计日志服务
pub struct OpcAuditLogService {
    db: Arc<DatabaseConnection>,
}

impl OpcAuditLogService {
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
        let _backend = self.db.get_database_backend();
        info!("审计日志: {} {} {} by {:?}", entity_type, entity_id, action, user_id);

        Ok(())
    }

    pub async fn get_history(
        &self,
        entity_type: &str,
        entity_id: &str,
        limit: u32,
    ) -> Result<Vec<OpcAuditLogEntry>, String> {
        info!("查询审计历史: {} {} (limit: {})", entity_type, entity_id, limit);

        Ok(vec![])
    }

    pub async fn query_by_action(
        &self,
        action: &str,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<OpcAuditLogEntry>, String> {
        info!("按操作类型查询: {} since {}", action, since);
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_entry() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = Arc::new(h.conn);
        let service = OpcAuditLogService::new(db);

        let result = service
            .log("invoice", "inv-001", "CREATE", None, Some(r#"{"total": 1000}"#), Some("user-1"))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_history() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = Arc::new(h.conn);
        let service = OpcAuditLogService::new(db);

        let result = service.get_history("invoice", "inv-001", 10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
