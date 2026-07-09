// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use axagent_harness::repositories::{GeneratedToolRepository, InsertGeneratedToolInput};

/// DAO implementation of GeneratedToolRepository.
pub struct DaoGeneratedToolRepository {
    db: DatabaseConnection,
}

impl DaoGeneratedToolRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl GeneratedToolRepository for DaoGeneratedToolRepository {
    async fn insert_generated_tool(&self, input: InsertGeneratedToolInput) -> Result<(), String> {
        let id = Uuid::new_v4().to_string();
        crate::repo::generated_tool::insert_generated_tool(
            &self.db,
            &id,
            &input.tool_name,
            &input.original_name,
            &input.original_description,
            &input.input_schema,
            &input.output_schema,
            &input.implementation,
            &input.source_info,
            input.created_at,
        )
        .await
        .map_err(|e| e.to_string())
    }
}
