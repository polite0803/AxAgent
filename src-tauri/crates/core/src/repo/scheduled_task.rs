use sea_orm::*;
use sea_query::OnConflict;

use crate::entity::scheduled_tasks;
use crate::error::Result;

pub async fn list_scheduled_tasks(db: &DatabaseConnection) -> Result<Vec<scheduled_tasks::Model>> {
    let tasks = scheduled_tasks::Entity::find().all(db).await?;
    Ok(tasks)
}

pub async fn get_scheduled_task(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<scheduled_tasks::Model>> {
    let task = scheduled_tasks::Entity::find_by_id(id).one(db).await?;
    Ok(task)
}

pub async fn insert_scheduled_task(
    db: &DatabaseConnection,
    task: &scheduled_tasks::ActiveModel,
) -> Result<()> {
    task.clone().insert(db).await?;
    Ok(())
}

pub async fn upsert_scheduled_task(
    db: &DatabaseConnection,
    task: scheduled_tasks::ActiveModel,
) -> Result<()> {
    scheduled_tasks::Entity::insert(task)
        .on_conflict(
            OnConflict::column(scheduled_tasks::Column::Id)
                .update_column(scheduled_tasks::Column::Name)
                .update_column(scheduled_tasks::Column::Description)
                .update_column(scheduled_tasks::Column::TaskType)
                .update_column(scheduled_tasks::Column::WorkflowId)
                .update_column(scheduled_tasks::Column::CronExpression)
                .update_column(scheduled_tasks::Column::IntervalSeconds)
                .update_column(scheduled_tasks::Column::NextRunAt)
                .update_column(scheduled_tasks::Column::LastRunAt)
                .update_column(scheduled_tasks::Column::LastResult)
                .update_column(scheduled_tasks::Column::Status)
                .update_column(scheduled_tasks::Column::Config)
                .update_column(scheduled_tasks::Column::UpdatedAt)
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

pub async fn delete_scheduled_task(db: &DatabaseConnection, id: &str) -> Result<bool> {
    let task = scheduled_tasks::Entity::find_by_id(id).one(db).await?;
    if let Some(t) = task {
        t.delete(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
