// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_harness::repositories::ProviderRepository;
use axagent_harness::types::ProviderConfig;
use axagent_harness::types::ProviderKey;

/// DAO implementation of ProviderRepository.
///
/// Wraps the existing `crate::repo::provider` free functions.
pub struct DaoProviderRepository {
    db: DatabaseConnection,
}

impl DaoProviderRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProviderRepository for DaoProviderRepository {
    async fn list_providers(&self) -> Result<Vec<ProviderConfig>, String> {
        crate::repo::provider::list_providers(&self.db).await.map_err(|e| e.to_string())
    }

    async fn get_provider(&self, id: &str) -> Result<ProviderConfig, String> {
        crate::repo::provider::get_provider(&self.db, id).await.map_err(|e| e.to_string())
    }

    async fn get_active_key(&self, provider_id: &str) -> Result<ProviderKey, String> {
        crate::repo::provider::get_active_key(&self.db, provider_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn resolve_model_for_node(
        &self,
        node_model: Option<&str>,
        session_model: Option<&str>,
        session_provider_id: Option<&str>,
        profile_suggested_provider: Option<&str>,
    ) -> Result<(ProviderConfig, ProviderKey, String), String> {
        crate::repo::provider::resolve_model_for_node(
            &self.db,
            node_model,
            session_model,
            session_provider_id,
            profile_suggested_provider,
        )
        .await
    }
}
