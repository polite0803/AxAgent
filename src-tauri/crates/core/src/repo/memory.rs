use sea_orm::sea_query::Expr;
use sea_orm::*;

use crate::entity::{memory_items, memory_namespaces};
use crate::error::{AxAgentError, Result};
use crate::types::{
    CreateMemoryItemInput, CreateMemoryNamespaceInput, MemoryItem, MemoryNamespace,
    UpdateMemoryItemInput, UpdateMemoryNamespaceInput,
};
use crate::utils::gen_id;

fn model_to_namespace(m: memory_namespaces::Model) -> MemoryNamespace {
    MemoryNamespace {
        id: m.id,
        name: m.name,
        scope: m.scope,
        embedding_provider: m.embedding_provider,
        embedding_dimensions: m.embedding_dimensions,
        retrieval_threshold: m.retrieval_threshold,
        retrieval_top_k: m.retrieval_top_k,
        icon_type: m.icon_type,
        icon_value: m.icon_value,
        sort_order: m.sort_order,
    }
}

fn model_to_item(m: memory_items::Model) -> MemoryItem {
    MemoryItem {
        id: m.id,
        namespace_id: m.namespace_id,
        title: m.title,
        content: m.content,
        source: m.source,
        index_status: m.index_status,
        index_error: m.index_error,
        updated_at: m.updated_at,
    }
}

pub async fn list_namespaces(db: &DatabaseConnection) -> Result<Vec<MemoryNamespace>> {
    let models = memory_namespaces::Entity::find()
        .order_by_asc(memory_namespaces::Column::SortOrder)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_namespace).collect())
}

pub async fn get_namespace(db: &DatabaseConnection, id: &str) -> Result<MemoryNamespace> {
    let model = memory_namespaces::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryNamespace {}", id)))?;

    Ok(model_to_namespace(model))
}

pub async fn create_namespace(
    db: &DatabaseConnection,
    input: CreateMemoryNamespaceInput,
) -> Result<MemoryNamespace> {
    let id = gen_id();

    let am = memory_namespaces::ActiveModel {
        id: Set(id.clone()),
        name: Set(input.name),
        scope: Set(input.scope),
        embedding_provider: Set(input.embedding_provider),
        embedding_dimensions: Set(input.embedding_dimensions),
        retrieval_threshold: Set(input.retrieval_threshold),
        retrieval_top_k: Set(input.retrieval_top_k),
        icon_type: Set(input.icon_type),
        icon_value: Set(input.icon_value),
        sort_order: Set(0),
    };

    am.insert(db).await?;

    get_namespace(db, &id).await
}

pub async fn delete_namespace(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = memory_namespaces::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("MemoryNamespace {}", id)));
    }
    Ok(())
}

pub async fn update_namespace(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateMemoryNamespaceInput,
) -> Result<MemoryNamespace> {
    let model = memory_namespaces::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryNamespace {}", id)))?;

    let mut am: memory_namespaces::ActiveModel = model.clone().into();
    if let Some(name) = input.name {
        am.name = Set(name);
    }
    if input.update_embedding_provider {
        am.embedding_provider = Set(input.embedding_provider);
    }
    if input.update_embedding_dimensions {
        am.embedding_dimensions = Set(input.embedding_dimensions);
    }
    if input.update_retrieval_threshold {
        am.retrieval_threshold = Set(input.retrieval_threshold);
    }
    if input.update_retrieval_top_k {
        am.retrieval_top_k = Set(input.retrieval_top_k);
    }
    if input.update_icon {
        am.icon_type = Set(input.icon_type);
        am.icon_value = Set(input.icon_value);
    }
    if let Some(sort_order) = input.sort_order {
        am.sort_order = Set(sort_order);
    }
    am.update(db).await?;

    get_namespace(db, id).await
}

pub async fn reorder_namespaces(db: &DatabaseConnection, namespace_ids: &[String]) -> Result<()> {
    for (i, id) in namespace_ids.iter().enumerate() {
        memory_namespaces::Entity::update_many()
            .col_expr(memory_namespaces::Column::SortOrder, Expr::value(i as i32))
            .filter(memory_namespaces::Column::Id.eq(id))
            .exec(db)
            .await?;
    }
    Ok(())
}

pub async fn list_items(db: &DatabaseConnection, namespace_id: &str) -> Result<Vec<MemoryItem>> {
    let models = memory_items::Entity::find()
        .filter(memory_items::Column::NamespaceId.eq(namespace_id))
        .order_by_desc(memory_items::Column::UpdatedAt)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_item).collect())
}

pub async fn add_item(db: &DatabaseConnection, input: CreateMemoryItemInput) -> Result<MemoryItem> {
    let id = gen_id();
    let source = input.source.unwrap_or_else(|| "manual".to_string());

    let am = memory_items::ActiveModel {
        id: Set(id.clone()),
        namespace_id: Set(input.namespace_id),
        title: Set(input.title),
        content: Set(input.content),
        source: Set(source),
        ..Default::default()
    };

    am.insert(db).await?;

    let model = memory_items::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    Ok(model_to_item(model))
}

pub async fn delete_item(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = memory_items::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("MemoryItem {}", id)));
    }
    Ok(())
}

pub async fn update_item(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateMemoryItemInput,
) -> Result<MemoryItem> {
    let model = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    let mut am: memory_items::ActiveModel = model.into();
    if let Some(title) = input.title {
        am.title = Set(title);
    }
    if let Some(content) = input.content {
        am.content = Set(content);
        // Content changed — reset index status to pending
        am.index_status = Set("pending".to_string());
    }
    am.updated_at = Set(chrono::Utc::now().to_rfc3339());
    am.update(db).await?;

    let updated = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    Ok(model_to_item(updated))
}

pub async fn update_item_index_status(
    db: &DatabaseConnection,
    id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<()> {
    let model = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    let mut am: memory_items::ActiveModel = model.into();
    am.index_status = Set(status.to_string());
    am.index_error = Set(error.map(|e| e.to_string()));
    am.update(db).await?;

    Ok(())
}
