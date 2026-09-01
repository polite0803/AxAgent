// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::sea_query::Expr;
use sea_orm::*;

use axagent_entities::{
    knowledge_attributes, knowledge_bases, knowledge_documents, knowledge_entities,
    knowledge_flows, knowledge_interfaces, knowledge_relations,
};
use axagent_harness::KbKind;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::{
    CreateKnowledgeBaseInput, KnowledgeBase, KnowledgeDocument, UpdateKnowledgeBaseInput,
};
use axagent_harness::util_fns::gen_id;

/// 把数据库 entity 的 `kind` 字符串解析为 `KbKind` 枚举。
/// 未识别值（包括历史遗留的 NULL/空串）回退为 `Indexed`，保持向后兼容。
fn parse_kb_kind(raw: &str) -> KbKind {
    match raw {
        "connected_vault" => KbKind::ConnectedVault,
        "connected_linked" => KbKind::ConnectedLinked,
        "connected_subagent" => KbKind::ConnectedSubagent,
        _ => KbKind::Indexed,
    }
}

/// `KbKind` 序列化为数据库列存的字符串
fn kb_kind_to_str(k: KbKind) -> &'static str {
    match k {
        KbKind::Indexed => "indexed",
        KbKind::ConnectedVault => "connected_vault",
        KbKind::ConnectedLinked => "connected_linked",
        KbKind::ConnectedSubagent => "connected_subagent",
    }
}

fn model_to_kb(m: knowledge_bases::Model) -> KnowledgeBase {
    KnowledgeBase {
        id: m.id,
        name: m.name,
        description: m.description,
        embedding_provider: m.embedding_provider,
        enabled: m.enabled != 0,
        icon_type: m.icon_type,
        icon_value: m.icon_value,
        sort_order: m.sort_order,
        embedding_dimensions: m.embedding_dimensions,
        retrieval_threshold: m.retrieval_threshold,
        retrieval_top_k: m.retrieval_top_k,
        chunk_size: m.chunk_size,
        chunk_overlap: m.chunk_overlap,
        separator: m.separator,
        kind: parse_kb_kind(&m.kind),
        vault_path: m.vault_path,
    }
}

fn model_to_doc(m: knowledge_documents::Model) -> KnowledgeDocument {
    KnowledgeDocument {
        id: m.id,
        knowledge_base_id: m.knowledge_base_id,
        title: m.title,
        source_path: m.source_path,
        mime_type: m.mime_type,
        size_bytes: m.size_bytes,
        indexing_status: m.indexing_status,
        doc_type: m.doc_type,
        index_error: m.index_error,
        source_conversation_id: m.source_conversation_id,
    }
}

pub async fn list_knowledge_bases(db: &DatabaseConnection) -> Result<Vec<KnowledgeBase>> {
    let models = knowledge_bases::Entity::find()
        .order_by_asc(knowledge_bases::Column::SortOrder)
        .order_by_asc(knowledge_bases::Column::Name)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_kb).collect())
}

pub async fn get_knowledge_base(db: &DatabaseConnection, id: &str) -> Result<KnowledgeBase> {
    let model = knowledge_bases::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeBase {}", id)))?;

    Ok(model_to_kb(model))
}

pub async fn create_knowledge_base(
    db: &DatabaseConnection,
    input: CreateKnowledgeBaseInput,
) -> Result<KnowledgeBase> {
    let id = gen_id();

    // ConnectedVault 类型必须提供 vault_path
    if matches!(input.kind, KbKind::ConnectedVault)
        && input.vault_path.as_deref().unwrap_or("").trim().is_empty()
    {
        return Err(AxAgentError::Validation(format!(
            "ConnectedVault KB '{}' 必须提供 vault_path",
            input.name
        )));
    }

    let am = knowledge_bases::ActiveModel {
        id: Set(id.clone()),
        name: Set(input.name),
        description: Set(input.description),
        embedding_provider: Set(input.embedding_provider),
        enabled: Set(if input.enabled.unwrap_or(true) { 1 } else { 0 }),
        icon_type: Set(None),
        icon_value: Set(None),
        sort_order: Set(0),
        embedding_dimensions: Set(None),
        retrieval_threshold: Set(None),
        retrieval_top_k: Set(None),
        chunk_size: Set(None),
        chunk_overlap: Set(None),
        separator: Set(None),
        kind: Set(kb_kind_to_str(input.kind).to_string()),
        vault_path: Set(input.vault_path),
    };

    am.insert(db).await?;

    get_knowledge_base(db, &id).await
}

pub async fn update_knowledge_base(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateKnowledgeBaseInput,
) -> Result<KnowledgeBase> {
    let model = knowledge_bases::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeBase {}", id)))?;

    let existing = model_to_kb(model.clone());

    let mut am: knowledge_bases::ActiveModel = model.into();
    am.name = Set(input.name.unwrap_or(existing.name));
    am.description = Set(input.description.or(existing.description));
    am.embedding_provider = Set(input.embedding_provider.or(existing.embedding_provider));
    am.enabled = Set(if input.enabled.unwrap_or(existing.enabled) {
        1
    } else {
        0
    });
    if input.update_icon {
        am.icon_type = Set(input.icon_type);
        am.icon_value = Set(input.icon_value);
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
    if input.update_chunk_size {
        am.chunk_size = Set(input.chunk_size);
    }
    if input.update_chunk_overlap {
        am.chunk_overlap = Set(input.chunk_overlap);
    }
    if input.update_separator {
        am.separator = Set(input.separator);
    }
    am.update(db).await?;

    get_knowledge_base(db, id).await
}

pub async fn reorder_knowledge_bases(db: &DatabaseConnection, base_ids: &[String]) -> Result<()> {
    for (i, id) in base_ids.iter().enumerate() {
        knowledge_bases::Entity::update_many()
            .col_expr(knowledge_bases::Column::SortOrder, Expr::value(i as i32))
            .filter(knowledge_bases::Column::Id.eq(id))
            .exec(db)
            .await?;
    }
    Ok(())
}

/// 直接更新 KB 的 kind / vault_path 字段
///
/// 用途：将已有 KB 在 Indexed ↔ ConnectedVault 之间切换。
/// - 切到 `ConnectedVault` 时，`vault_path` 必须为 `Some`
/// - 切回 `Indexed` 时，`vault_path` 应为 `None`（清空绑定）
pub async fn set_vault_binding(
    db: &DatabaseConnection,
    id: &str,
    kind: KbKind,
    vault_path: Option<String>,
) -> Result<KnowledgeBase> {
    // ConnectedVault 必须有 vault_path
    if matches!(kind, KbKind::ConnectedVault)
        && vault_path.as_deref().unwrap_or("").trim().is_empty()
    {
        return Err(AxAgentError::Validation(format!(
            "ConnectedVault KB '{}' 必须提供 vault_path",
            id
        )));
    }

    let model = knowledge_bases::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeBase {}", id)))?;

    let mut am: knowledge_bases::ActiveModel = model.into();
    am.kind = Set(kb_kind_to_str(kind).to_string());
    am.vault_path = Set(vault_path);
    am.update(db).await?;

    get_knowledge_base(db, id).await
}

pub async fn delete_knowledge_base(db: &DatabaseConnection, id: &str) -> Result<()> {
    // 物理删除该知识库下的所有索引任务，容器已不存在，保留 CANCELLED job 无意义
    if let Err(e) = crate::repo::index_jobs::delete_jobs_by_container(db, "knowledge", id).await {
        tracing::warn!(
            kb_id = id,
            error = %e,
            "[dao::knowledge] 删除相关索引任务失败，继续级联删除"
        );
    }

    // 级联删除子表（SQLite 启用了 PRAGMA foreign_keys=ON，
    // 但 migration DDL 未统一声明 ON DELETE CASCADE，需手动清理避免外键约束失败）
    knowledge_documents::Entity::delete_many()
        .filter(knowledge_documents::Column::KnowledgeBaseId.eq(id))
        .exec(db)
        .await?;
    knowledge_entities::Entity::delete_many()
        .filter(knowledge_entities::Column::KnowledgeBaseId.eq(id))
        .exec(db)
        .await?;
    knowledge_attributes::Entity::delete_many()
        .filter(knowledge_attributes::Column::KnowledgeBaseId.eq(id))
        .exec(db)
        .await?;
    knowledge_interfaces::Entity::delete_many()
        .filter(knowledge_interfaces::Column::KnowledgeBaseId.eq(id))
        .exec(db)
        .await?;
    knowledge_relations::Entity::delete_many()
        .filter(knowledge_relations::Column::KnowledgeBaseId.eq(id))
        .exec(db)
        .await?;
    knowledge_flows::Entity::delete_many()
        .filter(knowledge_flows::Column::KnowledgeBaseId.eq(id))
        .exec(db)
        .await?;

    let result = knowledge_bases::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("KnowledgeBase {}", id)));
    }
    Ok(())
}

pub async fn list_documents(
    db: &DatabaseConnection,
    base_id: &str,
) -> Result<Vec<KnowledgeDocument>> {
    let models = knowledge_documents::Entity::find()
        .filter(knowledge_documents::Column::KnowledgeBaseId.eq(base_id))
        .order_by_asc(knowledge_documents::Column::Title)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_doc).collect())
}

/// 批量获取 KB 内文档的 `updated_at`（add_document 写入的源文件 mtime，epoch 秒）。
///
/// DTO（`KnowledgeDocument`）不含时间戳字段，同步命令做增量比对时由此取值；
/// 旧数据该列为 0，同步侧回退到 size 比对。
pub async fn get_document_mtime_map(
    db: &DatabaseConnection,
    base_id: &str,
) -> Result<std::collections::HashMap<String, i64>> {
    let models = knowledge_documents::Entity::find()
        .filter(knowledge_documents::Column::KnowledgeBaseId.eq(base_id))
        .all(db)
        .await?;
    Ok(models.into_iter().map(|m| (m.id, m.updated_at)).collect())
}

pub async fn add_document(
    db: &DatabaseConnection,
    knowledge_base_id: &str,
    title: &str,
    source_path: &str,
    mime_type: &str,
    doc_type: Option<&str>,
) -> Result<KnowledgeDocument> {
    let id = gen_id();

    // Read actual file size from disk
    let (file_size, file_mtime_secs) = std::fs::metadata(source_path)
        .map(|m| {
            let mtime = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            (m.len() as i64, mtime)
        })
        .unwrap_or((0, 0));

    let am = knowledge_documents::ActiveModel {
        id: Set(id.clone()),
        knowledge_base_id: Set(knowledge_base_id.to_string()),
        title: Set(title.to_string()),
        source_path: Set(source_path.to_string()),
        mime_type: Set(mime_type.to_string()),
        size_bytes: Set(file_size),
        doc_type: Set(doc_type.unwrap_or("file").to_string()),
        // 记录源文件 mtime（epoch 秒），供 sync_project_knowledge_sources 做增量比对，
        // 避免每次同步对全部已存在文件删旧重加。旧数据该列为 0，同步侧回退到 size 比对。
        updated_at: Set(file_mtime_secs),
        ..Default::default()
    };

    am.insert(db).await?;

    let model = knowledge_documents::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeDocument {}", id)))?;

    Ok(model_to_doc(model))
}

pub async fn update_document_status(db: &DatabaseConnection, id: &str, status: &str) -> Result<()> {
    update_document_status_with_error(db, id, status, None).await
}

pub async fn update_document_status_with_error(
    db: &DatabaseConnection,
    id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<()> {
    let mut am: knowledge_documents::ActiveModel = knowledge_documents::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeDocument {}", id)))?
        .into();

    am.indexing_status = Set(status.to_string());
    am.index_error = Set(error.map(|e| e.to_string()));
    am.update(db).await?;
    Ok(())
}

pub async fn get_document(db: &DatabaseConnection, id: &str) -> Result<KnowledgeDocument> {
    let model = knowledge_documents::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeDocument {}", id)))?;

    Ok(model_to_doc(model))
}

pub async fn delete_document(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = knowledge_documents::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("KnowledgeDocument {}", id)));
    }
    Ok(())
}

/// Batch lookup document titles by IDs. Returns a map of document_id -> title.
pub async fn get_document_titles(
    db: &DatabaseConnection,
    doc_ids: &[String],
) -> Result<std::collections::HashMap<String, String>> {
    if doc_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let models = knowledge_documents::Entity::find()
        .filter(knowledge_documents::Column::Id.is_in(doc_ids.iter().map(|s| s.as_str())))
        .all(db)
        .await?;
    Ok(models.into_iter().map(|m| (m.id, m.title)).collect())
}
