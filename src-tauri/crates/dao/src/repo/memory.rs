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
    // v108: applicability_tags 同样以 JSON 数组字符串存储
    let applicability_tags =
        serde_json::from_str::<Vec<String>>(&m.applicability_tags).unwrap_or_default();
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
        applicability_tags,
        confirmed: m.confirmed,
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

/// 在数据库层面执行记忆条目搜索，带 WHERE 过滤和 LIMIT。
///
/// 避免全表加载，利用数据库索引和 LIMIT 提前截断。
/// 当 `namespace_id` 为空字符串时不按命名空间过滤（搜索全部）。
pub async fn search_items(
    db: &DatabaseConnection,
    namespace_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<MemoryItem>> {
    let query_lower = format!("%{}%", query.to_lowercase());
    let limit = limit as u64;

    let mut select = memory_items::Entity::find();
    if !namespace_id.is_empty() {
        select = select.filter(memory_items::Column::NamespaceId.eq(namespace_id));
    }
    let models = select
        .filter(
            Condition::any()
                .add(memory_items::Column::Title.like(query_lower.clone()))
                .add(memory_items::Column::Content.like(query_lower)),
        )
        .order_by_desc(memory_items::Column::Importance)
        .limit(limit)
        .all(db)
        .await?;

    // 检索回写访问统计：access_count +1、刷新 last_accessed，
    // 供衰减 tick 的 hours_since_last_access 与晋升阈值消费。
    // 批量单条 UPDATE，best-effort 不影响搜索主路径。
    let ids: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
    record_access_batch(db, &ids).await;

    Ok(models.into_iter().map(model_to_item).collect())
}

/// 批量记录访问：access_count +1、last_accessed 刷新为当前时间。
/// 不做自动晋升（晋升走显式的 `record_access_and_maybe_promote`）。
async fn record_access_batch(db: &DatabaseConnection, ids: &[String]) {
    if ids.is_empty() {
        return;
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    let result = memory_items::Entity::update_many()
        .col_expr(
            memory_items::Column::AccessCount,
            Expr::col(memory_items::Column::AccessCount).add(1),
        )
        .col_expr(memory_items::Column::LastAccessed, Expr::value(now_ms))
        .filter(memory_items::Column::Id.is_in(ids.iter().map(|s| s.as_str())))
        .exec(db)
        .await;
    if let Err(e) = result {
        tracing::debug!("[memory] 批量回写访问统计失败（非致命）: {}", e);
    }
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
        source_conversation_id: Set(input.source_conversation_id),
        source_message_id: Set(input.source_message_id),
        memory_nature: Set(memory_nature),
        tags: Set(tags_json),
        // v108: 自进化闭环 — applicability_tags + confirmed
        applicability_tags: Set(serde_json::to_string(
            &input.applicability_tags.unwrap_or_default(),
        )
        .unwrap_or_else(|_| "[]".to_string())),
        confirmed: Set(input.confirmed.unwrap_or(0)),
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
    // v108: 自进化闭环 — 支持更新 applicability_tags
    if let Some(applicability_tags) = input.applicability_tags {
        let tags_json =
            serde_json::to_string(&applicability_tags).unwrap_or_else(|_| "[]".to_string());
        am.applicability_tags = Set(tags_json);
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
///
/// v108 确认门：晋升到 core 层需要 `confirmed=1`（人工确认）。
/// Reflector 自动沉淀的经验默认未确认，必须经过人工审核才能进入 core 层。
pub async fn promote_item(db: &DatabaseConnection, id: &str) -> Result<MemoryItem> {
    let model = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    let new_tier = next_tier(&model.tier)
        .ok_or_else(|| AxAgentError::Validation("已在最高 tier，无法晋升".to_string()))?;

    // v108: 确认门 — 晋升到 core 层需要人工确认
    if new_tier == "core" && model.confirmed != 1 {
        return Err(AxAgentError::Validation(
            "晋升到 core 层需要先人工确认该记忆（confirmed=1）".to_string(),
        ));
    }

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

/// v108: 自进化闭环 — 确认记忆项（设置 confirmed=1）。
///
/// Reflector 自动沉淀的经验默认未确认（confirmed=0）。
/// 用户审核后调用此函数标记为已确认，之后才能晋升到 core 层。
pub async fn confirm_item(db: &DatabaseConnection, id: &str) -> Result<MemoryItem> {
    let model = memory_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("MemoryItem {}", id)))?;

    let mut am: memory_items::ActiveModel = model.into();
    am.confirmed = Set(1);
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
    // v108: 确认门 — 自动晋升到 core 层同样需要 confirmed=1
    // 未确认的记忆即使达到阈值也不会自动晋升到 core，需人工确认后再触发
    let confirmed = model.confirmed == 1;

    let mut am: memory_items::ActiveModel = model.into();
    am.access_count = Set(new_count);
    am.last_accessed = Set(Some(now_ms));
    // 达到晋升阈值且未在最高 tier → 自动晋升
    // v108: 但晋升目标为 core 时，需额外检查 confirmed=1
    if new_count >= threshold
        && let Some(new_tier) = next_tier(&current_tier)
        && (new_tier != "core" || confirmed)
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

    // 2. 衰减 + 3. 低分淘汰（直接 DB 层删除，无需先全量载入再按 id 删）
    let low_score_deleted = memory_items::Entity::delete_many()
        .filter(memory_items::Column::Importance.lt(0.05))
        .exec(db)
        .await?
        .rows_affected;

    // 对剩余 item 应用衰减：importance *= exp(-decay_rate * hours_since_last_access)
    // last_accessed 为 NULL 时不衰减（视为新条目）。
    // 指数衰减必须逐行计算（SQLite/PG 未必启用 exp() 数学函数），
    // 但更新改为分批 CASE 语句，避免逐行 UPDATE 的 N 次往返。
    let remaining = memory_items::Entity::find()
        .filter(memory_items::Column::LastAccessed.is_not_null())
        .all(db)
        .await?;
    let updates: Vec<(String, f64)> = remaining
        .iter()
        .filter_map(|m| {
            let last = m.last_accessed?;
            let hours = ((now_ms - last) as f64 / 3_600_000.0).max(0.0);
            let factor = (-m.decay_rate * hours).exp().max(0.01);
            let new_importance = (m.importance * factor).min(1.0);
            if (new_importance - m.importance).abs() > 1e-6 {
                Some((m.id.clone(), new_importance))
            } else {
                None
            }
        })
        .collect();

    for chunk in updates.chunks(200) {
        let mut case_sql = String::from("UPDATE memory_items SET importance = CASE id");
        for (id, imp) in chunk {
            let safe_id = id.replace('\'', "''");
            case_sql.push_str(&format!(" WHEN '{safe_id}' THEN {imp:.6}"));
        }
        case_sql.push_str(" END");
        if let Err(e) = db.execute_unprepared(&case_sql).await {
            tracing::warn!("[memory_decay] 批量衰减更新失败（{} 条）: {}", chunk.len(), e);
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

/// 高重要性条目：用于 Memory → Knowledge 实体回流。
///
/// 查询 importance >= threshold 的条目，用于定时将高价值记忆
/// 转换为知识图谱实体，解决三套实体系统各自为政的问题。
pub async fn list_high_importance_items(
    db: &DatabaseConnection,
    min_importance: Option<f64>,
    limit: Option<u32>,
) -> Result<Vec<MemoryItem>> {
    let threshold = min_importance.unwrap_or(0.7);
    let lim = limit.unwrap_or(100);

    let items = memory_items::Entity::find()
        .filter(memory_items::Column::Importance.gte(threshold))
        .order_by_desc(memory_items::Column::Importance)
        .limit(lim as u64)
        .all(db)
        .await?;

    Ok(items.into_iter().map(model_to_item).collect())
}

/// v110: Agent 工具调用结果 → Memory 自动沉淀
///
/// 扫描最近的对话消息，提取工具调用结果（WebSearch、CodeInterpreter、
/// KnowledgeRetrieval 等），自动沉淀为 Memory 条目。
///
/// 这是"Agent 执行→知识沉淀"闭环的核心：
/// Agent 在执行任务时产生的高价值工具结果（搜索发现、代码运行结果、
/// 外部数据获取）不应随对话结束而消失，而应自动沉淀为可被后续
/// RAG 检索使用的记忆条目。
///
/// # 工具重要性映射
/// - web_search / web_fetch → importance 0.6，tier: short_term
/// - code_interpreter / bash → importance 0.5，tier: short_term
/// - knowledge_retrieval / file_search → importance 0.7，tier: long_term
/// - 其他工具 → importance 0.4，tier: working
///
/// # 去重策略
/// 通过 content hash 避免重复沉淀，已存在的相同内容跳过。
pub async fn deposit_tool_results_from_recent_messages(
    db: &DatabaseConnection,
    hours_lookback: Option<i64>,
) -> Result<usize> {
    let hours = hours_lookback.unwrap_or(24);
    let now_ms = chrono::Utc::now().timestamp_millis();
    let cutoff = now_ms - hours * 3600 * 1000;

    // 查询最近包含 tool_result 的消息
    let sql = r#"
        SELECT m.id, m.content, m.role, m.created_at
        FROM messages m
        WHERE m.created_at >= ?1
          AND m.role = 'tool'
          AND m.content IS NOT NULL
          AND LENGTH(m.content) > 20
        ORDER BY m.created_at DESC
        LIMIT 200
    "#;
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            vec![cutoff.into()],
        ))
        .await?;

    if rows.is_empty() {
        return Ok(0);
    }

    let mut deposited = 0usize;
    let existing_hashes = load_existing_memory_content_hashes(db).await;
    ensure_tool_results_namespace(db).await;

    for row in &rows {
        let msg_id: String = row.try_get("", "id").unwrap_or_default();
        let content: String = row.try_get("", "content").unwrap_or_default();
        if content.is_empty() {
            continue;
        }

        // 检查是否已沉积过（按内容 hash 去重）
        let content_hash = simple_hash(&content);
        if existing_hashes.contains(&content_hash) {
            continue;
        }

        // 根据内容特征判断工具类型和重要性
        let (importance, tier, decay_rate) = infer_tool_importance(&content);

        // 截取前 200 字符作为标题
        let title: String = content.chars().take(200).collect();

        // 生成唯一 ID
        let item_id =
            format!("tool_{}_{}", msg_id, content_hash.chars().take(8).collect::<String>());

        // 插入 Memory 条目（SQLite 用 INSERT OR IGNORE，PG 用 ON CONFLICT DO NOTHING）
        let insert_sql = match db.get_database_backend() {
            sea_orm::DbBackend::Postgres => {
                r#"
            INSERT INTO memory_items
                (id, namespace_id, title, content, tier, importance, decay_rate,
                 access_count, confirmed, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 0, 0, $8, $8)
            ON CONFLICT (id) DO NOTHING
        "#
            },
            _ => {
                r#"
            INSERT OR IGNORE INTO memory_items
                (id, namespace_id, title, content, tier, importance, decay_rate,
                 access_count, confirmed, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, ?8, ?8)
        "#
            },
        };
        let values: Vec<sea_orm::Value> = vec![
            item_id.as_str().into(),
            "agent_tool_results".into(),
            title.as_str().into(),
            content.as_str().into(),
            tier.into(),
            importance.into(),
            decay_rate.into(),
            now_ms.into(),
        ];

        match db
            .query_all_raw(Statement::from_sql_and_values(
                db.get_database_backend(),
                insert_sql,
                values,
            ))
            .await
        {
            Ok(_) => {
                deposited += 1;
                tracing::debug!(
                    "[tool_deposit] 沉积成功 msg_id={} importance={} tier={}",
                    msg_id,
                    importance,
                    tier
                );
            },
            Err(e) => {
                tracing::debug!("[tool_deposit] 沉积失败 msg_id={}: {}", msg_id, e);
            },
        }
    }

    tracing::info!(
        "[tool_deposit] 工具结果沉积完成：扫描 {} 条消息，沉积 {} 条",
        rows.len(),
        deposited
    );

    Ok(deposited)
}

/// 加载已有 Memory 条目的内容 hash，用于去重。
async fn load_existing_memory_content_hashes(
    db: &DatabaseConnection,
) -> std::collections::HashSet<String> {
    let sql = "SELECT content FROM memory_items WHERE namespace_id = 'agent_tool_results'";
    let rows = match db
        .query_all_raw(Statement::from_sql_and_values(db.get_database_backend(), sql, vec![]))
        .await
    {
        Ok(rows) => rows,
        Err(_) => return std::collections::HashSet::new(),
    };

    rows.into_iter()
        .filter_map(|row| row.try_get::<String>("", "content").ok())
        .map(|content| simple_hash(&content))
        .collect()
}

/// 确保 `agent_tool_results` 命名空间存在（无 FK 约束，缺失不影响插入，
/// 但缺失会导致沉淀条目在命名空间列表 UI 中不可见）。
async fn ensure_tool_results_namespace(db: &DatabaseConnection) {
    let sql = match db.get_database_backend() {
        sea_orm::DbBackend::Postgres => {
            "INSERT INTO memory_namespaces (id, name, scope) VALUES ($1, $2, $3) \
             ON CONFLICT (id) DO NOTHING"
        },
        _ => "INSERT OR IGNORE INTO memory_namespaces (id, name, scope) VALUES (?1, ?2, ?3)",
    };
    let values: Vec<sea_orm::Value> =
        vec!["agent_tool_results".into(), "Agent 工具结果".into(), "global".into()];
    if let Err(e) =
        db.execute_raw(Statement::from_sql_and_values(db.get_database_backend(), sql, values)).await
    {
        tracing::debug!("[tool_deposit] 确保命名空间存在失败（非致命）: {}", e);
    }
}

/// 简单的字符串 hash（FNV-1a 64 位），用于去重。
fn simple_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// 根据工具结果内容推断重要性和 tier。
fn infer_tool_importance(content: &str) -> (f64, &'static str, f64) {
    let lower = content.to_lowercase();

    // Web 搜索/抓取结果 → 中短期记忆
    if lower.contains("http")
        || lower.contains("url")
        || lower.contains("搜索")
        || lower.contains("search")
    {
        return (0.6, "short_term", 0.02);
    }

    // 代码执行/运行结果 → 中短期记忆
    if lower.contains("error")
        || lower.contains("output")
        || lower.contains("stderr")
        || lower.contains("stdout")
    {
        return (0.5, "short_term", 0.02);
    }

    // 知识库检索结果 → 长期记忆
    if lower.contains("similarity") || lower.contains("relevant") || lower.contains("knowledge") {
        return (0.7, "long_term", 0.005);
    }

    // 文件操作结果 → 工作记忆
    if lower.contains("file") || lower.contains("directory") || lower.contains("文件") {
        return (0.4, "working", 0.05);
    }

    // 默认：工作记忆
    (0.4, "working", 0.05)
}
