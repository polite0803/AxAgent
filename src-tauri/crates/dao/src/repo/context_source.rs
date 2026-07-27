// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::context_sources;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::{ContextSource, CreateContextSourceInput};
use axagent_harness::util_fns::gen_id;

/// 把 doc_ids Vec 序列化为 JSON 字符串（空 Vec → None）
fn serialize_doc_ids(doc_ids: &[String]) -> Option<String> {
    if doc_ids.is_empty() {
        None
    } else {
        serde_json::to_string(doc_ids).ok()
    }
}

/// 把数据库 doc_ids_json 字段反序列化为 Vec<String>（NULL/空/解析失败 → 空 Vec）
fn deserialize_doc_ids(raw: &Option<String>) -> Vec<String> {
    match raw {
        Some(s) if !s.is_empty() => serde_json::from_str(s).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn model_to_context_source(m: context_sources::Model) -> ContextSource {
    ContextSource {
        id: m.id,
        conversation_id: m.conversation_id,
        message_id: m.message_id,
        source_type: m.source_type,
        ref_id: m.ref_id,
        title: m.title,
        enabled: m.enabled != 0,
        summary: m.summary,
        doc_ids: deserialize_doc_ids(&m.doc_ids_json),
    }
}

pub async fn list_context_sources(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<Vec<ContextSource>> {
    let models = context_sources::Entity::find()
        .filter(context_sources::Column::ConversationId.eq(conversation_id))
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_context_source).collect())
}

pub async fn get_context_source(db: &DatabaseConnection, id: &str) -> Result<ContextSource> {
    let model = context_sources::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ContextSource {}", id)))?;

    Ok(model_to_context_source(model))
}

pub async fn add_context_source(
    db: &DatabaseConnection,
    input: &CreateContextSourceInput,
) -> Result<ContextSource> {
    let id = gen_id();

    let am = context_sources::ActiveModel {
        id: Set(id.clone()),
        conversation_id: Set(input.conversation_id.clone()),
        message_id: Set(input.message_id.clone()),
        source_type: Set(input.source_type.clone()),
        ref_id: Set(input.ref_id.clone()),
        title: Set(input.title.clone()),
        enabled: Set(1),
        summary: Set(input.summary.clone()),
        doc_ids_json: Set(serialize_doc_ids(&input.doc_ids)),
    };

    am.insert(db).await?;

    get_context_source(db, &id).await
}

pub async fn remove_context_source(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = context_sources::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("ContextSource {}", id)));
    }
    Ok(())
}

pub async fn delete_context_sources_by_conversation(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<()> {
    context_sources::Entity::delete_many()
        .filter(context_sources::Column::ConversationId.eq(conversation_id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn toggle_context_source(db: &DatabaseConnection, id: &str) -> Result<ContextSource> {
    let model = context_sources::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ContextSource {}", id)))?;

    let new_enabled = if model.enabled != 0 { 0 } else { 1 };
    let mut am: context_sources::ActiveModel = model.into();
    am.enabled = Set(new_enabled);
    am.update(db).await?;

    get_context_source(db, id).await
}

/// 多文档协同：根据 (conversation_id, source_type, ref_id) 唯一定位一条 context_source，
/// 更新其 doc_ids_json 字段。若行不存在则返回 NotFound。
///
/// 设计意图：用户在 ContextSourcePicker 中勾选/取消勾选文档时调用，
/// 让 `resolve_rag_ids` 在 RAG 检索时只在该容器（KB / memory namespace / wiki）的
/// 指定文档子集内检索，而非整个容器。
pub async fn set_doc_ids_by_ref(
    db: &DatabaseConnection,
    conversation_id: &str,
    source_type: &str,
    ref_id: &str,
    doc_ids: &[String],
) -> Result<ContextSource> {
    let model = context_sources::Entity::find()
        .filter(context_sources::Column::ConversationId.eq(conversation_id))
        .filter(context_sources::Column::SourceType.eq(source_type))
        .filter(context_sources::Column::RefId.eq(ref_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            AxAgentError::NotFound(format!(
                "ContextSource ({}/{}/{})",
                conversation_id, source_type, ref_id
            ))
        })?;

    let id = model.id.clone();
    let mut am: context_sources::ActiveModel = model.into();
    am.doc_ids_json = Set(serialize_doc_ids(doc_ids));
    am.update(db).await?;

    get_context_source(db, &id).await
}

/// 多文档协同：删除指定 conversation 下不在 `keep` 列表中的 context_source 行；
/// 保留行的 doc_ids 不变。返回被删除的行数。
///
/// 用于 `sync_context_sources` 的 diff 同步：用户切换 enabled_knowledge_base_ids 等
/// 偏好时，只删除被取消勾选的容器行，保留仍启用容器行已设置的 doc_ids。
pub async fn prune_context_sources(
    db: &DatabaseConnection,
    conversation_id: &str,
    keep: &[(String, String)], // (source_type, ref_id) 二元组
) -> Result<u64> {
    let existing = context_sources::Entity::find()
        .filter(context_sources::Column::ConversationId.eq(conversation_id))
        .all(db)
        .await?;

    let keep_set: std::collections::HashSet<(String, String)> = keep.iter().cloned().collect();
    let to_delete: Vec<String> = existing
        .into_iter()
        .filter(|m| !keep_set.contains(&(m.source_type.clone(), m.ref_id.clone())))
        .map(|m| m.id)
        .collect();

    if to_delete.is_empty() {
        return Ok(0);
    }

    let res = context_sources::Entity::delete_many()
        .filter(context_sources::Column::Id.is_in(to_delete))
        .exec(db)
        .await?;
    Ok(res.rows_affected)
}
