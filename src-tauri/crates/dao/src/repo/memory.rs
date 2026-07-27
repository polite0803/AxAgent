// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::sea_query::Expr;
use sea_orm::*;

use axagent_entities::{memory_items, memory_namespaces};
use axagent_harness::constants;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::{
    CreateMemoryItemInput, CreateMemoryNamespaceInput, MemoryItem, MemoryNamespace,
    UpdateMemoryItemInput, UpdateMemoryNamespaceInput,
};
use axagent_harness::util_fns::current_rfc3339;
use axagent_harness::util_fns::gen_id;

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
    // tags 存储为 JSON 数组字符串，反序列化为 Vec<String>；失败时降级为空数组
    let tags = serde_json::from_str::<Vec<String>>(&m.tags).unwrap_or_default();
    MemoryItem {
        id: m.id,
        namespace_id: m.namespace_id,
        title: m.title,
        content: m.content,
        source: m.source,
        index_status: m.index_status,
        index_error: m.index_error,
        updated_at: m.updated_at,
        tier: m.tier,
        importance: m.importance,
        access_count: m.access_count,
        last_accessed: m.last_accessed,
        decay_rate: m.decay_rate,
        expires_at: m.expires_at,
        memory_nature: m.memory_nature,
        tags,
        source_conversation_id: m.source_conversation_id,
        source_message_id: m.source_message_id,
    }
}

pub async fn list_namespaces(db: &DatabaseConnection) -> Result<Vec<MemoryNamespace>> {
    let models = memory_namespaces::Entity::find()
        .filter(memory_namespaces::Column::Scope.ne("system"))
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
    // 物理删除该命名空间下的所有索引任务，容器已不存在，保留 CANCELLED job 无意义
    if let Err(e) = crate::repo::index_jobs::delete_jobs_by_container(db, "memory", id).await {
        tracing::warn!(
            ns_id = id,
            error = %e,
            "[dao::memory] 删除相关索引任务失败，继续级联删除"
        );
    }

    // 先删除所有关联的 memory_items
    let _ = memory_items::Entity::delete_many()
        .filter(memory_items::Column::NamespaceId.eq(id))
        .exec(db)
        .await?;

    // 再删除 namespace
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

pub async fn get_item(db: &DatabaseConnection, id: &str) -> Result<MemoryItem> {
    let model = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;
    Ok(model_to_item(model))
}

pub async fn add_item(db: &DatabaseConnection, input: CreateMemoryItemInput) -> Result<MemoryItem> {
    let id = gen_id();
    let source = input.source.unwrap_or_else(|| "manual".to_string());

    // 三层记忆系统：用户传 tier/importance/nature/tags 优先，否则用默认值
    let tier = input.tier.unwrap_or_else(|| "working".to_string());
    let importance = input.importance.unwrap_or(0.5);
    let memory_nature = input.memory_nature.unwrap_or_else(|| "semantic".to_string());
    let tags_json =
        serde_json::to_string(&input.tags.unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());
    let decay_rate = input.decay_rate.unwrap_or_else(|| default_decay_rate_for_tier(&tier));
    let expires_at = input.expires_at;

    let am = memory_items::ActiveModel {
        id: Set(id.clone()),
        namespace_id: Set(input.namespace_id),
        title: Set(input.title),
        content: Set(input.content),
        source: Set(source),
        index_status: Set(constants::status::PENDING.to_string()),
        index_error: Set(None),
        updated_at: Set(current_rfc3339()),
        tier: Set(tier),
        importance: Set(importance),
        access_count: Set(0),
        last_accessed: Set(None),
        decay_rate: Set(decay_rate),
        expires_at: Set(expires_at),
        source_conversation_id: Set(None),
        source_message_id: Set(None),
        memory_nature: Set(memory_nature),
        tags: Set(tags_json),
    };

    am.insert(db).await?;

    let model = memory_items::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    Ok(model_to_item(model))
}

/// 三层记忆系统：根据 tier 返回默认衰减率（与 trajectory crate 的 MemoryTier 默认值对齐）
fn default_decay_rate_for_tier(tier: &str) -> f64 {
    match tier {
        "short_term" => 0.1,
        "working" => 0.02,
        "long_term" => 0.005,
        "core" => 0.001,
        _ => 0.01,
    }
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
        am.index_status = Set(constants::status::PENDING.to_string());
    }
    // 三层记忆系统：支持 tier/importance/nature/tags 更新
    if let Some(tier) = input.tier {
        am.tier = Set(tier.clone());
        // tier 变化时同步更新 decay_rate 为新 tier 的默认值
        am.decay_rate = Set(default_decay_rate_for_tier(&tier));
    }
    if let Some(importance) = input.importance {
        am.importance = Set(importance);
    }
    if let Some(nature) = input.memory_nature {
        am.memory_nature = Set(nature);
    }
    if let Some(tags) = input.tags {
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        am.tags = Set(tags_json);
    }
    am.updated_at = Set(current_rfc3339());
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

// ── 三层记忆系统：晋升 / 降级 / 衰减 / 容量管理 ───────────────────────────
//
// 算法与 trajectory crate 的 MemoryService 对齐（service.rs:155-712），
// 但直接操作 memory_items 表，覆盖所有 namespace（包括用户自建）。
// 定时器在 init/services.rs 调用 apply_decay_tick。

/// tier 晋升链：short_term → working → long_term → core
fn next_tier(tier: &str) -> Option<&'static str> {
    match tier {
        "short_term" => Some("working"),
        "working" => Some("long_term"),
        "long_term" => Some("core"),
        _ => None,
    }
}

/// tier 降级链：core → long_term → working → short_term
fn prev_tier(tier: &str) -> Option<&'static str> {
    match tier {
        "core" => Some("long_term"),
        "long_term" => Some("working"),
        "working" => Some("short_term"),
        _ => None,
    }
}

/// tier 容量上限（与 trajectory MemoryTier::capacity 对齐）
fn tier_capacity(tier: &str) -> usize {
    match tier {
        "short_term" => 20,
        "working" => 50,
        "long_term" => 200,
        "core" => 30,
        _ => 50,
    }
}

/// 自动晋升阈值（access_count 达到此值自动晋升，与 trajectory 对齐）
fn promotion_threshold(tier: &str) -> i32 {
    match tier {
        "short_term" => 3,
        "working" => 8,
        "long_term" => 20,
        "core" => i32::MAX,
        _ => 8,
    }
}

/// 三层记忆系统：晋升 memory item 到下一 tier。已在 core 则无操作。
pub async fn promote_item(db: &DatabaseConnection, id: &str) -> Result<MemoryItem> {
    let model = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    let new_tier = next_tier(&model.tier)
        .ok_or_else(|| AxAgentError::Validation("已在最高 tier，无法晋升".to_string()))?;

    let mut am: memory_items::ActiveModel = model.into();
    am.tier = Set(new_tier.to_string());
    am.decay_rate = Set(default_decay_rate_for_tier(new_tier));
    am.updated_at = Set(current_rfc3339());
    am.update(db).await?;

    let updated = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;
    Ok(model_to_item(updated))
}

/// 三层记忆系统：降级 memory item 到下一 tier。已在 short_term 则无操作。
pub async fn demote_item(db: &DatabaseConnection, id: &str) -> Result<MemoryItem> {
    let model = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    let new_tier = prev_tier(&model.tier)
        .ok_or_else(|| AxAgentError::Validation("已在最低 tier，无法降级".to_string()))?;

    let mut am: memory_items::ActiveModel = model.into();
    am.tier = Set(new_tier.to_string());
    am.decay_rate = Set(default_decay_rate_for_tier(new_tier));
    am.updated_at = Set(current_rfc3339());
    am.update(db).await?;

    let updated = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;
    Ok(model_to_item(updated))
}

/// 三层记忆系统：记录访问，access_count +1，更新 last_accessed，可能触发自动晋升。
pub async fn record_access_and_maybe_promote(
    db: &DatabaseConnection,
    id: &str,
) -> Result<MemoryItem> {
    let model = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    let new_count = model.access_count + 1;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let threshold = promotion_threshold(&model.tier);
    let current_tier = model.tier.clone();

    let mut am: memory_items::ActiveModel = model.into();
    am.access_count = Set(new_count);
    am.last_accessed = Set(Some(now_ms));
    // 达到晋升阈值且未在最高 tier → 自动晋升
    if new_count >= threshold
        && let Some(new_tier) = next_tier(&current_tier)
    {
        am.tier = Set(new_tier.to_string());
        am.decay_rate = Set(default_decay_rate_for_tier(new_tier));
    }
    am.updated_at = Set(current_rfc3339());
    am.update(db).await?;

    let updated = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;
    Ok(model_to_item(updated))
}

/// 三层记忆系统：应用一次衰减 tick。
///
/// 算法（与 trajectory `apply_decay_tick` 对齐）：
/// 1. 删除已过期（expires_at < now）的 item
/// 2. 对每个 item：importance *= exp(-decay_rate * hours_since_last_access).max(0.01)
/// 3. importance < 0.05（eviction_threshold）的删除
/// 4. 每个 namespace + tier 分组超过 capacity 的按 importance 升序淘汰
///
/// 返回 (过期删除数, 衰减淘汰数, 容量淘汰数)。
pub async fn apply_decay_tick(db: &DatabaseConnection) -> Result<(u64, u64, u64)> {
    let now_ms = chrono::Utc::now().timestamp_millis();

    // 1. 删除已过期 item
    let expired_deleted = memory_items::Entity::delete_many()
        .filter(memory_items::Column::ExpiresAt.is_not_null())
        .filter(memory_items::Column::ExpiresAt.lt(now_ms))
        .exec(db)
        .await?
        .rows_affected;

    // 2. 衰减 + 3. 低分淘汰
    let all_items = memory_items::Entity::find()
        .filter(memory_items::Column::Importance.lt(0.05))
        .all(db)
        .await?;
    let low_score_deleted = all_items.len() as u64;
    if !all_items.is_empty() {
        let ids: Vec<String> = all_items.into_iter().map(|m| m.id).collect();
        memory_items::Entity::delete_many()
            .filter(memory_items::Column::Id.is_in(ids))
            .exec(db)
            .await?;
    }

    // 对剩余 item 应用衰减：importance *= exp(-decay_rate * hours_since_last_access)
    // last_accessed 为 NULL 时不衰减（视为新条目）
    let remaining = memory_items::Entity::find().all(db).await?;
    for m in remaining {
        if let Some(last) = m.last_accessed {
            let hours = ((now_ms - last) as f64 / 3_600_000.0).max(0.0);
            let factor = (-m.decay_rate * hours).exp().max(0.01);
            let new_importance = (m.importance * factor).min(1.0);
            if (new_importance - m.importance).abs() > 1e-6 {
                let mut am: memory_items::ActiveModel = m.into();
                am.importance = Set(new_importance);
                am.update(db).await?;
            }
        }
    }

    // 4. 容量淘汰：每个 (namespace_id, tier) 分组超过 capacity 的按 importance 升序淘汰
    let mut capacity_evicted: u64 = 0;
    let tiers = ["short_term", "working", "long_term", "core"];
    for tier in tiers {
        let cap = tier_capacity(tier) as i64;
        // 按 namespace 分组取每组 ids
        let items_in_tier = memory_items::Entity::find()
            .filter(memory_items::Column::Tier.eq(tier))
            .order_by_asc(memory_items::Column::Importance)
            .all(db)
            .await?;
        use std::collections::HashMap;
        let mut by_ns: HashMap<String, Vec<String>> = HashMap::new();
        for m in items_in_tier {
            by_ns.entry(m.namespace_id).or_default().push(m.id);
        }
        for (_ns, mut ids) in by_ns {
            let total = ids.len() as i64;
            if total > cap {
                // importance 升序已排，淘汰前 (total - cap) 个
                let evict_count = (total - cap) as usize;
                let to_evict: Vec<String> = ids.drain(..evict_count).collect();
                capacity_evicted += to_evict.len() as u64;
                memory_items::Entity::delete_many()
                    .filter(memory_items::Column::Id.is_in(to_evict))
                    .exec(db)
                    .await?;
            }
        }
    }

    Ok((expired_deleted, low_score_deleted, capacity_evicted))
}
