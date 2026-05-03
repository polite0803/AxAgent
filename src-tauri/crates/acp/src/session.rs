//! ACP 会话管理

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::protocol::{CreateSessionParams, CreateSessionResult, StatusResult};
use crate::types::{AcpNotification, AcpSession, AcpSessionStatus};

/// ACP 会话管理器
pub struct AcpSessionManager {
    sessions: Arc<RwLock<HashMap<String, AcpSession>>>,
}

impl AcpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建新会话
    pub async fn create_session(&self, params: &CreateSessionParams) -> CreateSessionResult {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let session = AcpSession {
            session_id: session_id.clone(),
            work_dir: params.work_dir.clone(),
            status: AcpSessionStatus::Idle,
            created_at: now,
            last_active: now,
            permission_mode: params
                .permission_mode
                .clone()
                .unwrap_or_else(|| "workspace-write".to_string()),
            active_tasks: 0,
        };

        let result = CreateSessionResult {
            session_id: session_id.clone(),
            work_dir: params.work_dir.clone(),
            status: "idle".to_string(),
        };

        self.sessions.write().await.insert(session_id, session);

        tracing::info!("[ACP] 会话已创建: {}", result.session_id);
        result
    }

    /// 获取会话
    pub async fn get_session(&self, session_id: &str) -> Option<AcpSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// 获取会话状态
    pub async fn get_status(&self, session_id: &str) -> Option<StatusResult> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| StatusResult {
            session_id: s.session_id.clone(),
            status: format!("{:?}", s.status).to_lowercase(),
            active_tasks: s.active_tasks,
            tokens_used: 0,
            permission_mode: s.permission_mode.clone(),
        })
    }

    /// 关闭会话
    pub async fn close_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = AcpSessionStatus::Closed;
            tracing::info!("[ACP] 会话已关闭: {}", session_id);
            true
        } else {
            false
        }
    }

    /// 列出所有活跃会话
    pub async fn list_sessions(&self) -> Vec<AcpSession> {
        self.sessions
            .read()
            .await
            .values()
            .filter(|s| s.status != AcpSessionStatus::Closed)
            .cloned()
            .collect()
    }

    /// 更新会话状态
    pub async fn update_status(&self, session_id: &str, status: AcpSessionStatus) {
        if let Some(session) = self.sessions.write().await.get_mut(session_id) {
            session.status = status;
            session.last_active = chrono::Utc::now();
        }
    }

    /// 清理已关闭的会话（超过 1 小时的）
    pub async fn cleanup(&self) {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);
        self.sessions
            .write()
            .await
            .retain(|_, s| s.status != AcpSessionStatus::Closed || s.last_active > cutoff);
    }
}

impl Default for AcpSessionManager {
    fn default() -> Self {
        Self::new()
    }
}
