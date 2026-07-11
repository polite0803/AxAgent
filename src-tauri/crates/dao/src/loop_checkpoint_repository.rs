// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_harness::repositories::LoopCheckpointRepository;
use axagent_harness::workflow_types::LoopCheckpoint;

pub struct DaoLoopCheckpointRepository {
    pub db: Arc<DatabaseConnection>,
}

#[async_trait]
impl LoopCheckpointRepository for DaoLoopCheckpointRepository {
    async fn save_loop_checkpoint(&self, cp: &LoopCheckpoint) -> Result<(), String> {
        crate::repo::loop_checkpoint::save_loop_checkpoint(&self.db, cp)
            .await
            .map_err(|e| e.to_string())
    }

    async fn load_loop_checkpoint(
        &self,
        execution_id: &str,
        node_id: &str,
    ) -> Result<Option<LoopCheckpoint>, String> {
        crate::repo::loop_checkpoint::load_loop_checkpoint(&self.db, execution_id, node_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn delete_loop_checkpoint(
        &self,
        execution_id: &str,
        node_id: &str,
    ) -> Result<(), String> {
        crate::repo::loop_checkpoint::delete_loop_checkpoint(&self.db, execution_id, node_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn delete_loop_checkpoints_for_execution(
        &self,
        execution_id: &str,
    ) -> Result<(), String> {
        crate::repo::loop_checkpoint::delete_loop_checkpoints_for_execution(&self.db, execution_id)
            .await
            .map_err(|e| e.to_string())
    }
}
