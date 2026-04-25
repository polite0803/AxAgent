use sea_orm::*;

use crate::entity::skill_references;
use crate::error::Result;
use crate::utils::now_ts;

pub async fn create_reference(
    db: &DatabaseConnection,
    id: &str,
    skill_id: &str,
    workflow_id: &str,
    node_id: &str,
) -> Result<()> {
    let model = skill_references::ActiveModel {
        id: Set(id.to_string()),
        skill_id: Set(skill_id.to_string()),
        workflow_id: Set(workflow_id.to_string()),
        node_id: Set(node_id.to_string()),
        created_at: Set(now_ts()),
    };
    model.insert(db).await?;
    Ok(())
}

pub async fn get_references_by_skill(
    db: &DatabaseConnection,
    skill_id: &str,
) -> Result<Vec<skill_references::Model>> {
    let refs = skill_references::Entity::find()
        .filter(skill_references::Column::SkillId.eq(skill_id))
        .all(db)
        .await?;
    Ok(refs)
}

pub async fn get_references_by_workflow(
    db: &DatabaseConnection,
    workflow_id: &str,
) -> Result<Vec<skill_references::Model>> {
    let refs = skill_references::Entity::find()
        .filter(skill_references::Column::WorkflowId.eq(workflow_id))
        .all(db)
        .await?;
    Ok(refs)
}

pub async fn delete_references_by_workflow(
    db: &DatabaseConnection,
    workflow_id: &str,
) -> Result<u64> {
    let result = skill_references::Entity::delete_many()
        .filter(skill_references::Column::WorkflowId.eq(workflow_id))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

pub async fn count_references(
    db: &DatabaseConnection,
    skill_id: &str,
) -> Result<i64> {
    let count = skill_references::Entity::find()
        .filter(skill_references::Column::SkillId.eq(skill_id))
        .count(db)
        .await?;
    Ok(count as i64)
}
