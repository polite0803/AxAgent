// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementation of BackgroundTaskRepository using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set};
use uuid::Uuid;

use axagent_entities::background_tasks;
use axagent_harness::repo_dtos::{BackgroundTask, CreateBackgroundTaskInput};
use axagent_harness::repositories::BackgroundTaskRepository;

fn model_to_dto(m: background_tasks::Model) -> BackgroundTask {
    BackgroundTask {
        id: m.id,
        title: m.title,
        description: m.description,
        task_type: m.task_type,
        command: m.command,
        prompt: m.prompt,
        status: m.status,
        output: m.output,
        exit_code: m.exit_code,
        conversation_id: m.conversation_id,
        created_by: m.created_by,
        created_at: m.created_at,
        updated_at: m.updated_at,
        finished_at: m.finished_at,
    }
}

pub struct DaoBackgroundTaskRepository {
    db: DatabaseConnection,
}

impl DaoBackgroundTaskRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BackgroundTaskRepository for DaoBackgroundTaskRepository {
    async fn spawn_task(&self, input: CreateBackgroundTaskInput) -> Result<BackgroundTask, String> {
        let now = chrono::Utc::now().timestamp();
        let id = Uuid::new_v4().to_string();

        let am = background_tasks::ActiveModel {
            id: Set(id.clone()),
            title: Set(input.title),
            description: Set(input.description),
            task_type: Set(input.task_type),
            command: Set(input.command),
            prompt: Set(input.prompt),
            status: Set("pending".to_string()),
            output: Set(String::new()),
            exit_code: Set(None),
            conversation_id: Set(None),
            created_by: Set(input.created_by),
            created_at: Set(now),
            updated_at: Set(now),
            finished_at: Set(None),
        };

        let model = am.insert(&self.db).await.map_err(|e| format!("spawn_task: {}", e))?;

        Ok(model_to_dto(model))
    }

    async fn get_task(&self, id: &str) -> Result<Option<BackgroundTask>, String> {
        let model = background_tasks::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| format!("get_task: {}", e))?;

        Ok(model.map(model_to_dto))
    }

    async fn list_tasks(&self) -> Result<Vec<BackgroundTask>, String> {
        let models = background_tasks::Entity::find()
            .order_by_desc(background_tasks::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| format!("list_tasks: {}", e))?;

        Ok(models.into_iter().map(model_to_dto).collect())
    }

    async fn stop_task(&self, id: &str) -> Result<(), String> {
        let model = background_tasks::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| format!("stop_task find: {}", e))?;

        let Some(model) = model else {
            return Err(format!("task not found: {}", id));
        };

        let mut am: background_tasks::ActiveModel = model.into();
        let now = chrono::Utc::now().timestamp();
        am.status = Set("stopped".to_string());
        am.finished_at = Set(Some(now));
        am.updated_at = Set(now);
        am.update(&self.db).await.map_err(|e| format!("stop_task update: {}", e))?;

        Ok(())
    }

    async fn update_status(&self, id: &str, status: &str) -> Result<(), String> {
        let model = background_tasks::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| format!("update_status find: {}", e))?;

        let Some(model) = model else {
            return Err(format!("task not found: {}", id));
        };

        let mut am: background_tasks::ActiveModel = model.into();
        let now = chrono::Utc::now().timestamp();
        am.status = Set(status.to_string());
        am.updated_at = Set(now);
        if status == "completed" || status == "failed" || status == "stopped" {
            am.finished_at = Set(Some(now));
        }
        am.update(&self.db).await.map_err(|e| format!("update_status update: {}", e))?;

        Ok(())
    }

    async fn get_output(&self, id: &str) -> Result<Option<String>, String> {
        let model = background_tasks::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| format!("get_output: {}", e))?;

        Ok(model.map(|m| m.output))
    }
}
