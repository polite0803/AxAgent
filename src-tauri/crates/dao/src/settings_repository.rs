// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of SettingsRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_harness::repositories::SettingsRepository;
use axagent_harness::types::AppSettings;

/// DAO implementation of SettingsRepository.
pub struct DaoSettingsRepository {
    db: DatabaseConnection,
}

impl DaoSettingsRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SettingsRepository for DaoSettingsRepository {
    async fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        crate::repo::settings::get_setting(&self.db, key).await.map_err(|e| e.to_string())
    }

    async fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        crate::repo::settings::set_setting(&self.db, key, value).await.map_err(|e| e.to_string())
    }

    async fn get_settings(&self) -> Result<AppSettings, String> {
        crate::repo::settings::get_settings(&self.db).await.map_err(|e| e.to_string())
    }
}
