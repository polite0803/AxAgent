// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of MessageRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use axagent_entities::messages;
use axagent_harness::repositories::{CreateMessageInput, MessageRepository};
use axagent_harness::types::Message;

/// DAO implementation of MessageRepository.
pub struct DaoMessageRepository {
    db: DatabaseConnection,
}

impl DaoMessageRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MessageRepository for DaoMessageRepository {
    async fn create_message(&self, input: CreateMessageInput) -> Result<Message, String> {
        crate::repo::message::create_message(
            &self.db,
            &input.conversation_id,
            input.role,
            &input.content,
            &[],
            None,
            0,
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn list_all_message_attachments(&self) -> Result<Vec<(String, String)>, String> {
        let rows = messages::Entity::find().all(&self.db).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|m| (m.id, m.attachments)).collect())
    }

    async fn update_message_attachments(
        &self,
        message_id: &str,
        attachments_json: &str,
    ) -> Result<(), String> {
        let model = messages::Entity::find_by_id(message_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("message not found: {message_id}"))?;
        let mut am: messages::ActiveModel = model.into();
        am.attachments = Set(attachments_json.to_string());
        am.update(&self.db).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
