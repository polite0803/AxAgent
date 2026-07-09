// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_harness::repositories::MemoryRepository;
use axagent_harness::types::{CreateMemoryItemInput, MemoryItem, MemoryNamespace};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub struct DaoMemoryRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoMemoryRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MemoryRepository for DaoMemoryRepository {
    async fn list_namespaces(&self) -> Result<Vec<MemoryNamespace>, String> {
        crate::repo::memory::list_namespaces(&self.db).await.map_err(|e| e.to_string())
    }

    async fn add_item(&self, input: CreateMemoryItemInput) -> Result<MemoryItem, String> {
        crate::repo::memory::add_item(&self.db, input).await.map_err(|e| e.to_string())
    }
}
