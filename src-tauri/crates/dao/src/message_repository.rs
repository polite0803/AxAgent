// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of MessageRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

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
}
