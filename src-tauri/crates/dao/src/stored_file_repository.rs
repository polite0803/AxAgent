// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of StoredFileRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use axagent_entities::stored_files;
use axagent_harness::repo_dtos::{CreateStoredFileInput, StoredFile};
use axagent_harness::repositories::StoredFileRepository;
use axagent_harness::util_fns::now_datetime_str;

fn model_to_dto(m: stored_files::Model) -> StoredFile {
    StoredFile {
        id: m.id,
        hash: m.hash,
        original_name: m.original_name,
        mime_type: m.mime_type,
        size_bytes: m.size_bytes,
        storage_path: m.storage_path,
        conversation_id: m.conversation_id,
        created_at: m.created_at,
    }
}

pub struct DaoStoredFileRepository {
    db: DatabaseConnection,
}

impl DaoStoredFileRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl StoredFileRepository for DaoStoredFileRepository {
    async fn create_stored_file(&self, input: CreateStoredFileInput) -> Result<StoredFile, String> {
        let now = now_datetime_str();

        let am = stored_files::ActiveModel {
            id: Set(input.id),
            hash: Set(input.hash),
            original_name: Set(input.original_name),
            mime_type: Set(input.mime_type),
            size_bytes: Set(input.size_bytes),
            storage_path: Set(input.storage_path),
            conversation_id: Set(input.conversation_id),
            created_at: Set(now),
        };

        let model = am.insert(&self.db).await.map_err(|e| format!("create_stored_file: {}", e))?;

        Ok(model_to_dto(model))
    }

    async fn get_stored_file(&self, id: &str) -> Result<StoredFile, String> {
        let model = stored_files::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| format!("get_stored_file: {}", e))?;

        model.map(model_to_dto).ok_or_else(|| format!("stored file not found: {}", id))
    }

    async fn list_all_stored_files(&self) -> Result<Vec<StoredFile>, String> {
        let models = stored_files::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| format!("list_all_stored_files: {}", e))?;

        Ok(models.into_iter().map(model_to_dto).collect())
    }

    async fn update_storage_path(&self, id: &str, storage_path: &str) -> Result<(), String> {
        let model = stored_files::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| format!("update_storage_path find: {}", e))?;

        let Some(model) = model else {
            return Err(format!("stored file not found: {}", id));
        };

        let mut am: stored_files::ActiveModel = model.into();
        am.storage_path = Set(storage_path.to_string());
        am.update(&self.db).await.map_err(|e| format!("update_storage_path update: {}", e))?;

        Ok(())
    }
}
