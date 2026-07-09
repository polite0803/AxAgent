// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of WikiOperationRepository using SeaORM.

use std::sync::Arc;

use axagent_entities::wiki_operations;
use axagent_harness::wiki_dtos::{WikiOperation, WikiOperationRepository};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

/// DAO implementation of WikiOperationRepository.
pub struct DaoWikiOperationRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoWikiOperationRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl WikiOperationRepository for DaoWikiOperationRepository {
    async fn log(&self, operation: WikiOperation) -> Result<(), String> {
        let am = wiki_operations::ActiveModel {
            wiki_id: Set(operation.wiki_id),
            operation_type: Set(operation.operation_type),
            target_type: Set(operation.target_type),
            target_id: Set(operation.target_id),
            status: Set(operation.status),
            details_json: Set(operation.details_json),
            error_message: Set(operation.error_message),
            created_at: Set(operation.created_at),
            completed_at: Set(operation.completed_at),
            ..Default::default()
        };

        am.insert(self.db.as_ref()).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}
