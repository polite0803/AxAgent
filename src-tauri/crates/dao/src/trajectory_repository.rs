// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of TrajectoryRepository.

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use axagent_harness::repositories::TrajectoryRepository;

pub struct DaoTrajectoryRepository {
    db: Arc<DatabaseConnection>,
}

impl DaoTrajectoryRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TrajectoryRepository for DaoTrajectoryRepository {
    fn db_connection(&self) -> &DatabaseConnection {
        &self.db
    }
}
