// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of ConversationRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_harness::repositories::{ConversationRepository, CreateConversationInput};
use axagent_harness::types::Conversation;

/// DAO implementation of ConversationRepository.
pub struct DaoConversationRepository {
    db: DatabaseConnection,
}

impl DaoConversationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ConversationRepository for DaoConversationRepository {
    async fn get_conversation(&self, id: &str) -> Result<Conversation, String> {
        crate::repo::conversation::get_conversation(&self.db, id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn create_conversation(
        &self,
        input: CreateConversationInput,
    ) -> Result<Conversation, String> {
        crate::repo::conversation::create_conversation(
            &self.db,
            &input.title,
            &input.model_id,
            &input.provider_id,
            input.system_prompt.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn increment_message_count(&self, conversation_id: &str) -> Result<(), String> {
        crate::repo::conversation::increment_message_count(&self.db, conversation_id)
            .await
            .map_err(|e| e.to_string())
    }
}
