// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of PlatformConfigRepository using SeaORM.

use std::collections::HashMap;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_harness::repositories::PlatformConfigRepository;

/// DAO implementation of PlatformConfigRepository.
pub struct DaoPlatformConfigRepository {
    db: DatabaseConnection,
}

impl DaoPlatformConfigRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PlatformConfigRepository for DaoPlatformConfigRepository {
    async fn load_session_routes(&self) -> HashMap<String, String> {
        crate::repo::platform_config::load_session_routes(&self.db).await
    }

    async fn save_session_routes(
        &self,
        routes: &HashMap<String, String>,
    ) -> Result<(), String> {
        crate::repo::platform_config::save_session_routes(&self.db, routes)
            .await
            .map_err(|e| e.to_string())
    }

    async fn save_platform_cursor(&self, platform: &str, cursor: i64) -> Result<(), String> {
        crate::repo::platform_config::save_platform_cursor(&self.db, platform, cursor)
            .await
            .map_err(|e| e.to_string())
    }
}
