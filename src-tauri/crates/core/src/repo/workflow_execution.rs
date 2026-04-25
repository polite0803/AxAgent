use sea_orm::*;

use crate::entity::workflow_executions;
use crate::error::Result;
use crate::utils::now_ts;

pub async fn create_workflow_execution(
    db: &DatabaseConnection,
    id: &str,
    workflow_id: &str,
    input_params: Option<&str>,
) -> Result<()> {
    let now = now_ts();
    let model = workflow_executions::ActiveModel {
        id: Set(id.to_string()),
        workflow_id: Set(workflow_id.to_string()),
        status: Set("running".to_string()),
        input_params: Set(input_params.map(|s| s.to_string())),
        output_result: Set(None),
        node_executions: Set(None),
        total_time_ms: Set(Some(0)),
        created_at: Set(now),
        updated_at: Set(now),
    };
    model.insert(db).await?;
    Ok(())
}

pub async fn get_workflow_execution(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<workflow_executions::Model>> {
    let execution = workflow_executions::Entity::find_by_id(id).one(db).await?;
    Ok(execution)
}

pub async fn update_workflow_execution_status(
    db: &DatabaseConnection,
    id: &str,
    status: &str,
    output_result: Option<&str>,
    node_executions: Option<&str>,
    total_time_ms: Option<i32>,
) -> Result<bool> {
    let execution = workflow_executions::Entity::find_by_id(id).one(db).await?;
    if let Some(e) = execution {
        let mut active_model: workflow_executions::ActiveModel = e.into();
        active_model.status = Set(status.to_string());
        if let Some(v) = output_result {
            active_model.output_result = Set(Some(v.to_string()));
        }
        if let Some(v) = node_executions {
            active_model.node_executions = Set(Some(v.to_string()));
        }
        if let Some(v) = total_time_ms {
            active_model.total_time_ms = Set(Some(v));
        }
        active_model.updated_at = Set(now_ts());
        active_model.update(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn list_workflow_executions(
    db: &DatabaseConnection,
    workflow_id: &str,
) -> Result<Vec<workflow_executions::Model>> {
    let executions = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(workflow_id))
        .order_by(workflow_executions::Column::CreatedAt, Order::Desc)
        .all(db)
        .await?;
    Ok(executions)
}
