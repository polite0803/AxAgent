// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::conversation_branches;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::ConversationBranch;
use axagent_harness::util_fns::gen_id;

fn model_to_branch(m: conversation_branches::Model) -> ConversationBranch {
    ConversationBranch {
        id: m.id,
        conversation_id: m.conversation_id,
        parent_message_id: m.parent_message_id,
        branch_label: m.branch_label,
        branch_index: m.branch_index,
        compared_message_ids_json: m.compared_message_ids_json,
        created_at: m.created_at,
    }
}

pub async fn list_branches(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<Vec<ConversationBranch>> {
    let models = conversation_branches::Entity::find()
        .filter(conversation_branches::Column::ConversationId.eq(conversation_id))
        .order_by_asc(conversation_branches::Column::BranchIndex)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_branch).collect())
}

pub async fn get_branch(db: &DatabaseConnection, id: &str) -> Result<ConversationBranch> {
    let model = conversation_branches::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ConversationBranch {}", id)))?;

    Ok(model_to_branch(model))
}

pub async fn create_branch(
    db: &DatabaseConnection,
    conversation_id: &str,
    parent_message_id: &str,
    label: &str,
) -> Result<ConversationBranch> {
    let id = gen_id();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let backend = db.get_database_backend();
    let (count_sql, count_values): (&str, Vec<sea_orm::Value>) = match backend {
        sea_orm::DatabaseBackend::Postgres => (
            "SELECT COALESCE(MAX(branch_index), -1) + 1 FROM conversation_branches WHERE conversation_id = $1",
            vec![conversation_id.into()],
        ),
        _ => (
            "SELECT COALESCE(MAX(branch_index), -1) + 1 FROM conversation_branches WHERE conversation_id = ?",
            vec![conversation_id.into()],
        ),
    };

    let count_result =
        db.query_one_raw(Statement::from_sql_and_values(backend, count_sql, count_values)).await?;

    let next_index: i32 =
        count_result.and_then(|row| row.try_get_by_index::<i32>(0).ok()).unwrap_or(0);

    let am = conversation_branches::ActiveModel {
        id: Set(id.clone()),
        conversation_id: Set(conversation_id.to_string()),
        parent_message_id: Set(parent_message_id.to_string()),
        branch_label: Set(label.to_string()),
        branch_index: Set(next_index),
        compared_message_ids_json: Set(None),
        created_at: Set(now),
    };

    am.insert(db).await?;

    get_branch(db, &id).await
}
