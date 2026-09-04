// SPDX-License-Identifier: AGPL-3.0-only

//! `AgentSessionRepository` 的 DAO 实现。

use std::sync::Arc;

use async_trait::async_trait;

use axagent_entities::agent_sessions;
use axagent_entities::conversations;
use axagent_harness::agent_session_repo::AgentSessionRepository;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::AgentSession;
use axagent_harness::util_fns::gen_id;
use sea_orm::*;

fn model_to_agent_session(model: agent_sessions::Model) -> AgentSession {
    AgentSession {
        id: model.id,
        conversation_id: model.conversation_id,
        cwd: model.cwd,
        workspace_locked: model.workspace_locked,
        permission_mode: model.permission_mode,
        runtime_status: model.runtime_status,
        sdk_context_json: model.sdk_context_json,
        sdk_context_backup_json: model.sdk_context_backup_json,
        total_tokens: model.total_tokens,
        total_cost_usd: model.total_cost_usd,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub struct DaoAgentSessionRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoAgentSessionRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AgentSessionRepository for DaoAgentSessionRepository {
    async fn upsert_agent_session(
        &self,
        conversation_id: &str,
        cwd: Option<&str>,
        permission_mode: Option<&str>,
    ) -> Result<AgentSession> {
        let existing = agent_sessions::Entity::find()
            .filter(agent_sessions::Column::ConversationId.eq(conversation_id))
            .one(self.db.as_ref())
            .await?;

        let now = chrono::Utc::now().timestamp();

        if let Some(model) = existing {
            let mut am: agent_sessions::ActiveModel = model.into();
            if let Some(cwd) = cwd {
                am.cwd = Set(Some(cwd.to_string()));
                am.workspace_locked = Set(1);
            }
            if let Some(pm) = permission_mode {
                am.permission_mode = Set(pm.to_string());
            }
            am.updated_at = Set(now);
            let updated = am.update(self.db.as_ref()).await?;
            Ok(model_to_agent_session(updated))
        } else {
            // 确保 conversations 行存在，否则 FOREIGN KEY 约束会失败
            let conv_exists = conversations::Entity::find_by_id(conversation_id)
                .one(self.db.as_ref())
                .await?
                .is_some();
            if !conv_exists {
                let now_ts = chrono::Utc::now().timestamp();
                let stmt = sea_orm::Statement::from_sql_and_values(
                    sea_orm::DatabaseBackend::Sqlite,
                    "INSERT OR IGNORE INTO conversations (id, title, model_id, provider_id, created_at, updated_at) VALUES ($1, '[auto]', 'unknown', 'unknown', $2, $2)",
                    [conversation_id.to_string().into(), now_ts.into()],
                );
                self.db.as_ref().execute_raw(stmt).await?;
            }

            let id = gen_id();
            let workspace_locked = if cwd.is_some() { 1 } else { 0 };
            let model = agent_sessions::ActiveModel {
                id: Set(id),
                conversation_id: Set(conversation_id.to_string()),
                cwd: Set(cwd.map(|s| s.to_string())),
                workspace_locked: Set(workspace_locked),
                permission_mode: Set(permission_mode.unwrap_or("default").to_string()),
                runtime_status: Set("idle".to_string()),
                sdk_context_json: Set(None),
                sdk_context_backup_json: Set(None),
                total_tokens: Set(0),
                total_cost_usd: Set(0.0),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let inserted = model.insert(self.db.as_ref()).await?;
            Ok(model_to_agent_session(inserted))
        }
    }

    async fn update_agent_session_status(&self, id: &str, runtime_status: &str) -> Result<()> {
        let model = agent_sessions::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AxAgentError::NotFound(format!("AgentSession {}", id)))?;

        let now = chrono::Utc::now().timestamp();
        let mut am: agent_sessions::ActiveModel = model.into();
        am.runtime_status = Set(runtime_status.to_string());
        am.updated_at = Set(now);
        am.update(self.db.as_ref()).await?;
        Ok(())
    }

    async fn update_agent_session_after_query(
        &self,
        id: &str,
        runtime_status: &str,
        sdk_context_json: Option<&str>,
        tokens_delta: i64,
        cost_delta: f64,
    ) -> Result<()> {
        let model = agent_sessions::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AxAgentError::NotFound(format!("AgentSession {}", id)))?;

        let now = chrono::Utc::now().timestamp();
        let mut am: agent_sessions::ActiveModel = model.clone().into();
        am.runtime_status = Set(runtime_status.to_string());
        if let Some(ctx) = sdk_context_json {
            am.sdk_context_json = Set(Some(ctx.to_string()));
        }
        am.total_tokens = Set(model.total_tokens + tokens_delta);
        am.total_cost_usd = Set(model.total_cost_usd + cost_delta);
        am.updated_at = Set(now);
        am.update(self.db.as_ref()).await?;
        Ok(())
    }

    async fn clear_sdk_context_by_conversation_id(&self, conversation_id: &str) -> Result<()> {
        let session = agent_sessions::Entity::find()
            .filter(agent_sessions::Column::ConversationId.eq(conversation_id))
            .one(self.db.as_ref())
            .await?;

        if let Some(model) = session {
            let mut am: agent_sessions::ActiveModel = model.into();
            am.sdk_context_json = Set(None);
            am.sdk_context_backup_json = Set(None);
            am.update(self.db.as_ref()).await?;
        }
        Ok(())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<AgentSession>> {
        let model = agent_sessions::Entity::find_by_id(id).one(self.db.as_ref()).await?;
        Ok(model.map(model_to_agent_session))
    }

    async fn get_by_conversation_id(&self, conversation_id: &str) -> Result<Option<AgentSession>> {
        let model = agent_sessions::Entity::find()
            .filter(agent_sessions::Column::ConversationId.eq(conversation_id))
            .one(self.db.as_ref())
            .await?;
        Ok(model.map(model_to_agent_session))
    }

    async fn list_all(&self) -> Result<Vec<AgentSession>> {
        let models = agent_sessions::Entity::find().all(self.db.as_ref()).await?;
        Ok(models.into_iter().map(model_to_agent_session).collect())
    }
}
