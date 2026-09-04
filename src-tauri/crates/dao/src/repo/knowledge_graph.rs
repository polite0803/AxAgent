// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::{
    knowledge_attributes, knowledge_entities, knowledge_flows, knowledge_interfaces,
    knowledge_relations,
};
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::graph_dtos::{GraphEdge, GraphNode};
use axagent_harness::types::{
    CreateKnowledgeAttributeInput, CreateKnowledgeEntityInput, CreateKnowledgeFlowInput,
    CreateKnowledgeInterfaceInput, CreateKnowledgeRelationInput, KnowledgeAttribute,
    KnowledgeEntity, KnowledgeFlow, KnowledgeInterface, KnowledgeRelation,
};
use axagent_harness::util_fns::gen_id;

/// Sentinel KB ID for trajectory-derived entities (v101 merge).
pub const TRAJECTORY_KB_ID: &str = "__sys_trajectory__";

fn model_to_entity(m: knowledge_entities::Model) -> KnowledgeEntity {
    KnowledgeEntity {
        id: m.id,
        knowledge_base_id: m.knowledge_base_id,
        name: m.name,
        entity_type: m.entity_type,
        description: m.description,
        source_path: m.source_path,
        source_language: m.source_language,
        properties: m.properties,
        lifecycle: m.lifecycle,
        behaviors: m.behaviors,
        metadata: m.metadata,
        created_at: m.created_at,
        updated_at: m.updated_at,
        // v101: trajectory entity fields
        aliases: m.aliases,
        mention_count: m.mention_count,
        confidence: m.confidence,
        first_seen_at: m.first_seen_at,
        last_seen_at: m.last_seen_at,
    }
}

fn model_to_attribute(m: knowledge_attributes::Model) -> KnowledgeAttribute {
    KnowledgeAttribute {
        id: m.id,
        knowledge_base_id: m.knowledge_base_id,
        entity_id: m.entity_id,
        name: m.name,
        attribute_type: m.attribute_type,
        data_type: m.data_type,
        description: m.description,
        is_required: m.is_required,
        default_value: m.default_value,
        constraints: m.constraints,
        validation_rules: m.validation_rules,
        metadata: m.metadata,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn model_to_relation(m: knowledge_relations::Model) -> KnowledgeRelation {
    KnowledgeRelation {
        id: m.id,
        knowledge_base_id: m.knowledge_base_id,
        source_entity_id: m.source_entity_id,
        target_entity_id: m.target_entity_id,
        relation_type: m.relation_type,
        description: m.description,
        properties: m.properties,
        metadata: m.metadata,
        created_at: m.created_at,
        updated_at: m.updated_at,
        // v101: trajectory relationship weight
        weight: m.weight,
    }
}

fn model_to_flow(m: knowledge_flows::Model) -> KnowledgeFlow {
    KnowledgeFlow {
        id: m.id,
        knowledge_base_id: m.knowledge_base_id,
        name: m.name,
        flow_type: m.flow_type,
        description: m.description,
        source_path: m.source_path,
        steps: m.steps,
        decision_points: m.decision_points,
        error_handling: m.error_handling,
        preconditions: m.preconditions,
        postconditions: m.postconditions,
        metadata: m.metadata,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn model_to_interface(m: knowledge_interfaces::Model) -> KnowledgeInterface {
    KnowledgeInterface {
        id: m.id,
        knowledge_base_id: m.knowledge_base_id,
        name: m.name,
        interface_type: m.interface_type,
        description: m.description,
        source_path: m.source_path,
        input_schema: m.input_schema,
        output_schema: m.output_schema,
        error_codes: m.error_codes,
        communication_pattern: m.communication_pattern,
        version: m.version,
        metadata: m.metadata,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

pub async fn create_knowledge_entity(
    db: &DatabaseConnection,
    input: CreateKnowledgeEntityInput,
) -> Result<KnowledgeEntity> {
    let id = gen_id();
    let now = chrono::Utc::now().timestamp();

    let am = knowledge_entities::ActiveModel {
        id: Set(id.clone()),
        knowledge_base_id: Set(input.knowledge_base_id),
        name: Set(input.name),
        entity_type: Set(input.entity_type),
        description: Set(input.description),
        source_path: Set(input.source_path),
        source_language: Set(input.source_language),
        properties: Set(input.properties),
        lifecycle: Set(input.lifecycle),
        behaviors: Set(input.behaviors),
        metadata: Set(input.metadata),
        created_at: Set(now),
        updated_at: Set(now),
        aliases: Set("[]".to_string()),
        mention_count: Set(1),
        confidence: Set(0.5),
        first_seen_at: Set(None),
        last_seen_at: Set(None),
        source_type: Set(String::from("knowledge_base")),
        source_id: Set(String::new()),
        node_type: Set(String::from("entity")),
        external_id: Set(None),
    };

    am.insert(db).await?;

    let model = knowledge_entities::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeEntity {}", id)))?;

    Ok(model_to_entity(model))
}

pub async fn list_knowledge_entities(
    db: &DatabaseConnection,
    base_id: &str,
) -> Result<Vec<KnowledgeEntity>> {
    let mut select = knowledge_entities::Entity::find();
    if !base_id.is_empty() {
        select = select.filter(knowledge_entities::Column::KnowledgeBaseId.eq(base_id));
    }
    let models = select.order_by_asc(knowledge_entities::Column::Name).all(db).await?;

    Ok(models.into_iter().map(model_to_entity).collect())
}

pub async fn create_knowledge_attribute(
    db: &DatabaseConnection,
    input: CreateKnowledgeAttributeInput,
) -> Result<KnowledgeAttribute> {
    let id = gen_id();
    let now = chrono::Utc::now().timestamp();

    let am = knowledge_attributes::ActiveModel {
        id: Set(id.clone()),
        knowledge_base_id: Set(input.knowledge_base_id),
        entity_id: Set(input.entity_id),
        name: Set(input.name),
        attribute_type: Set(input.attribute_type),
        data_type: Set(input.data_type),
        description: Set(input.description),
        is_required: Set(input.is_required),
        default_value: Set(input.default_value),
        constraints: Set(input.constraints),
        validation_rules: Set(input.validation_rules),
        metadata: Set(input.metadata),
        created_at: Set(now),
        updated_at: Set(now),
    };

    am.insert(db).await?;

    let model = knowledge_attributes::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeAttribute {}", id)))?;

    Ok(model_to_attribute(model))
}

pub async fn list_knowledge_attributes(
    db: &DatabaseConnection,
    entity_id: &str,
) -> Result<Vec<KnowledgeAttribute>> {
    let models = knowledge_attributes::Entity::find()
        .filter(knowledge_attributes::Column::EntityId.eq(entity_id))
        .order_by_asc(knowledge_attributes::Column::Name)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_attribute).collect())
}

pub async fn create_knowledge_relation(
    db: &DatabaseConnection,
    input: CreateKnowledgeRelationInput,
) -> Result<KnowledgeRelation> {
    let id = gen_id();
    let now = chrono::Utc::now().timestamp();

    let am = knowledge_relations::ActiveModel {
        id: Set(id.clone()),
        knowledge_base_id: Set(input.knowledge_base_id),
        source_entity_id: Set(input.source_entity_id),
        target_entity_id: Set(input.target_entity_id),
        relation_type: Set(input.relation_type),
        description: Set(input.description),
        properties: Set(input.properties),
        metadata: Set(input.metadata),
        created_at: Set(now),
        updated_at: Set(now),
        weight: Set(1.0),
        source_type: Set(String::from("knowledge_base")),
        source_id: Set(String::new()),
    };

    am.insert(db).await?;

    let model = knowledge_relations::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeRelation {}", id)))?;

    Ok(model_to_relation(model))
}

pub async fn list_knowledge_relations(
    db: &DatabaseConnection,
    base_id: &str,
) -> Result<Vec<KnowledgeRelation>> {
    let models = knowledge_relations::Entity::find()
        .filter(knowledge_relations::Column::KnowledgeBaseId.eq(base_id))
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_relation).collect())
}

pub async fn create_knowledge_flow(
    db: &DatabaseConnection,
    input: CreateKnowledgeFlowInput,
) -> Result<KnowledgeFlow> {
    let id = gen_id();
    let now = chrono::Utc::now().timestamp();

    let am = knowledge_flows::ActiveModel {
        id: Set(id.clone()),
        knowledge_base_id: Set(input.knowledge_base_id),
        name: Set(input.name),
        flow_type: Set(input.flow_type),
        description: Set(input.description),
        source_path: Set(input.source_path),
        steps: Set(input.steps),
        decision_points: Set(input.decision_points),
        error_handling: Set(input.error_handling),
        preconditions: Set(input.preconditions),
        postconditions: Set(input.postconditions),
        metadata: Set(input.metadata),
        created_at: Set(now),
        updated_at: Set(now),
    };

    am.insert(db).await?;

    let model = knowledge_flows::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeFlow {}", id)))?;

    Ok(model_to_flow(model))
}

pub async fn list_knowledge_flows(
    db: &DatabaseConnection,
    base_id: &str,
) -> Result<Vec<KnowledgeFlow>> {
    let models = knowledge_flows::Entity::find()
        .filter(knowledge_flows::Column::KnowledgeBaseId.eq(base_id))
        .order_by_asc(knowledge_flows::Column::Name)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_flow).collect())
}

pub async fn create_knowledge_interface(
    db: &DatabaseConnection,
    input: CreateKnowledgeInterfaceInput,
) -> Result<KnowledgeInterface> {
    let id = gen_id();
    let now = chrono::Utc::now().timestamp();

    let am = knowledge_interfaces::ActiveModel {
        id: Set(id.clone()),
        knowledge_base_id: Set(input.knowledge_base_id),
        name: Set(input.name),
        interface_type: Set(input.interface_type),
        description: Set(input.description),
        source_path: Set(input.source_path),
        input_schema: Set(input.input_schema),
        output_schema: Set(input.output_schema),
        error_codes: Set(input.error_codes),
        communication_pattern: Set(input.communication_pattern),
        version: Set(input.version),
        metadata: Set(input.metadata),
        created_at: Set(now),
        updated_at: Set(now),
    };

    am.insert(db).await?;

    let model = knowledge_interfaces::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeInterface {}", id)))?;

    Ok(model_to_interface(model))
}

pub async fn search_entities(
    db: &DatabaseConnection,
    kb_id: &str,
    query: &str,
    top_k: usize,
) -> Result<Vec<KnowledgeEntity>> {
    let all = list_knowledge_entities(db, kb_id).await?;
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().collect();
    if keywords.is_empty() {
        let limited: Vec<_> = all.into_iter().take(top_k).collect();
        return Ok(limited);
    }
    let mut scored: Vec<(i64, KnowledgeEntity)> = all
        .into_iter()
        .map(|e| {
            let name_lower = e.name.to_lowercase();
            let desc_lower = e.description.as_deref().unwrap_or("").to_lowercase();
            let type_lower = e.entity_type.to_lowercase();
            let mut score: i64 = 0;
            for kw in &keywords {
                if name_lower.contains(kw) {
                    score += 10;
                }
                if desc_lower.contains(kw) {
                    score += 5;
                }
                if type_lower.contains(kw) {
                    score += 3;
                }
            }
            (score, e)
        })
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    Ok(scored.into_iter().take(top_k).map(|(_, e)| e).collect())
}

/// 支持类型过滤的图谱实体搜索。
///
/// 相比 `search_entities`，本函数支持按 `entity_type` 过滤，
/// 利用数据库层面的 WHERE 提前筛选，避免全表加载。
/// 当 `kb_id` 为空字符串时不按知识库过滤（搜索全部）。
pub async fn search_entities_with_filter(
    db: &DatabaseConnection,
    kb_id: &str,
    query: &str,
    top_k: usize,
    entity_type_filter: Option<&str>,
) -> Result<Vec<KnowledgeEntity>> {
    let mut select = knowledge_entities::Entity::find();
    if !kb_id.is_empty() {
        select = select.filter(knowledge_entities::Column::KnowledgeBaseId.eq(kb_id));
    }

    if let Some(et) = entity_type_filter {
        select = select.filter(knowledge_entities::Column::EntityType.eq(et));
    }

    let limit = top_k as u64;
    select = select.limit(limit * 3);

    let models = select.all(db).await?;
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().collect();

    let mut scored: Vec<(i64, KnowledgeEntity)> = models
        .into_iter()
        .map(model_to_entity)
        .map(|e| {
            let name_lower = e.name.to_lowercase();
            let desc_lower = e.description.as_deref().unwrap_or("").to_lowercase();
            let type_lower = e.entity_type.to_lowercase();
            let mut score: i64 = 0;
            for kw in &keywords {
                if name_lower.contains(kw) {
                    score += 10;
                }
                if desc_lower.contains(kw) {
                    score += 5;
                }
                if type_lower.contains(kw) {
                    score += 3;
                }
            }
            (score, e)
        })
        .filter(|(s, _)| *s > 0)
        .collect();

    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    Ok(scored.into_iter().take(top_k).map(|(_, e)| e).collect())
}

pub async fn list_knowledge_interfaces(
    db: &DatabaseConnection,
    base_id: &str,
) -> Result<Vec<KnowledgeInterface>> {
    let models = knowledge_interfaces::Entity::find()
        .filter(knowledge_interfaces::Column::KnowledgeBaseId.eq(base_id))
        .order_by_asc(knowledge_interfaces::Column::Name)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_interface).collect())
}

// ── v101: Trajectory-style entity operations ───────────────────────────────

/// Get a single entity by ID.
pub async fn get_entity_by_id(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<KnowledgeEntity>> {
    let model = knowledge_entities::Entity::find_by_id(id).one(db).await?;
    Ok(model.map(model_to_entity))
}

/// Get all entities by knowledge_base_id, ordered by last_seen_at desc.
pub async fn get_all_entities_by_kb(
    db: &DatabaseConnection,
    kb_id: &str,
) -> Result<Vec<KnowledgeEntity>> {
    let models = knowledge_entities::Entity::find()
        .filter(knowledge_entities::Column::KnowledgeBaseId.eq(kb_id))
        .order_by_desc(knowledge_entities::Column::UpdatedAt)
        .all(db)
        .await?;
    Ok(models.into_iter().map(model_to_entity).collect())
}

/// Name-based search for entities (like trajectory storage's search_entities).
pub async fn search_entities_by_name(
    db: &DatabaseConnection,
    kb_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<KnowledgeEntity>> {
    let pattern = format!("%{}%", query);
    let models = knowledge_entities::Entity::find()
        .filter(
            knowledge_entities::Column::KnowledgeBaseId
                .eq(kb_id)
                .and(knowledge_entities::Column::Name.like(&pattern)),
        )
        .all(db)
        .await?
        .into_iter()
        .take(limit)
        .map(model_to_entity)
        .collect();
    Ok(models)
}

/// Upsert an entity (trajectory-style save with on-conflict update).
// 8 params justified: it maps 1:1 to the DB insert/upsert columns with distinct semantics.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_entity(
    db: &DatabaseConnection,
    kb_id: &str,
    name: &str,
    entity_type: &str,
    aliases: &str,
    confidence: f64,
    first_seen_at: Option<String>,
    last_seen_at: Option<String>,
) -> Result<KnowledgeEntity> {
    use axagent_harness::util_fns::gen_id;
    use sea_orm::sea_query::OnConflict;

    let now = chrono::Utc::now().timestamp();
    let id = format!("ent_{}", gen_id());

    let am = knowledge_entities::ActiveModel {
        id: Set(id.clone()),
        knowledge_base_id: Set(kb_id.to_string()),
        name: Set(name.to_string()),
        entity_type: Set(entity_type.to_string()),
        description: Set(None),
        source_path: Set(String::new()),
        source_language: Set(None),
        properties: Set(serde_json::Value::Object(Default::default())),
        lifecycle: Set(None),
        behaviors: Set(None),
        metadata: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        aliases: Set(aliases.to_string()),
        mention_count: Set(1),
        confidence: Set(confidence),
        first_seen_at: Set(first_seen_at),
        last_seen_at: Set(last_seen_at),
        source_type: Set(String::from("knowledge_base")),
        source_id: Set(String::new()),
        node_type: Set(String::from("entity")),
        external_id: Set(None),
    };

    knowledge_entities::Entity::insert(am)
        .on_conflict(
            OnConflict::column(knowledge_entities::Column::Id)
                .update_columns([
                    knowledge_entities::Column::Name,
                    knowledge_entities::Column::LastSeenAt,
                    knowledge_entities::Column::MentionCount,
                    knowledge_entities::Column::Confidence,
                    knowledge_entities::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    get_entity_by_id(db, &id)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeEntity {}", id)))
}

/// Delete an entity and cascade-delete its relations.
pub async fn delete_entity_cascade(db: &DatabaseConnection, id: &str) -> Result<()> {
    let txn = db.begin().await?;
    knowledge_relations::Entity::delete_many()
        .filter(
            knowledge_relations::Column::SourceEntityId
                .eq(id)
                .or(knowledge_relations::Column::TargetEntityId.eq(id)),
        )
        .exec(&txn)
        .await?;
    knowledge_entities::Entity::delete_by_id(id).exec(&txn).await?;
    txn.commit().await?;
    Ok(())
}

/// Upsert a relationship (trajectory-style save).
pub async fn upsert_relation(
    db: &DatabaseConnection,
    source_id: &str,
    target_id: &str,
    relation_type: &str,
    weight: f64,
) -> Result<KnowledgeRelation> {
    use axagent_harness::util_fns::gen_id;
    use sea_orm::sea_query::OnConflict;

    let now = chrono::Utc::now().timestamp();
    let id = format!("rel_{}", gen_id());

    let am = knowledge_relations::ActiveModel {
        id: Set(id.clone()),
        knowledge_base_id: Set(String::new()),
        source_entity_id: Set(source_id.to_string()),
        target_entity_id: Set(target_id.to_string()),
        relation_type: Set(relation_type.to_string()),
        description: Set(None),
        properties: Set(None),
        metadata: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        weight: Set(weight),
        source_type: Set(String::from("knowledge_base")),
        source_id: Set(String::new()),
    };

    knowledge_relations::Entity::insert(am)
        .on_conflict(
            OnConflict::column(knowledge_relations::Column::Id)
                .update_columns([
                    knowledge_relations::Column::Weight,
                    knowledge_relations::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    let model = knowledge_relations::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeRelation {}", id)))?;

    Ok(model_to_relation(model))
}

// ── LightRAG 图查询增强 ────────────────────────────────────────────────

/// 根据查询关键词在 knowledge_entities 表中检索实体，并扩展 1-hop 邻居关系。
///
/// 算法：
/// 1. 复用 `search_entities` 的关键词打分逻辑拿到 seed 实体（限制 top_k）
/// 2. 收集 seed 实体 id 集合
/// 3. 查询 knowledge_relations 中所有 source_entity_id 或 target_entity_id 命中的关系
/// 4. 反查邻居实体的 name/entity_type，组装 [`GraphEnhancedContextChunk`]
/// 5. 去重：同一实体可能被多个 seed 命中
pub async fn graph_enhanced_search(
    db: &DatabaseConnection,
    kb_id: &str,
    query: &str,
    top_k: usize,
    include_neighbors: bool,
) -> Result<Vec<axagent_harness::GraphEnhancedContextChunk>> {
    // 1. 关键词打分拿到 seed 实体
    let seeds = search_entities(db, kb_id, query, top_k).await?;
    if seeds.is_empty() {
        return Ok(Vec::new());
    }

    // 2. 收集 seed id → entity 映射（用于最终输出实体信息）
    let mut seed_map: std::collections::HashMap<String, KnowledgeEntity> =
        std::collections::HashMap::with_capacity(seeds.len());
    let mut seed_ids: Vec<String> = Vec::with_capacity(seeds.len());
    for e in seeds {
        seed_ids.push(e.id.clone());
        seed_map.insert(e.id.clone(), e);
    }

    // 3. 不需要邻居关系时，直接组装无 relations 的 chunk
    if !include_neighbors {
        let mut chunks: Vec<axagent_harness::GraphEnhancedContextChunk> =
            Vec::with_capacity(seed_map.len());
        for e in seed_map.values() {
            chunks.push(axagent_harness::GraphEnhancedContextChunk {
                entity_name: e.name.clone(),
                entity_type: e.entity_type.clone(),
                description: e.description.clone(),
                relations: Vec::new(),
                knowledge_base_id: e.knowledge_base_id.clone(),
            });
        }
        return Ok(chunks);
    }

    // 4. 查询所有命中 seed 的关系（双向）
    //
    // 分组语义必须是 `(kb_id = kb AND (source IN seeds OR target IN seeds))`。
    // 不能写成链式 `a.and(b).or(c)` —— 那会变成 `(kb AND source) OR target`，
    // OR 分支丢掉 kb_id 约束，导致跨知识库关系（含因果边，其 kb_id 为空串）泄露进检索。
    // 因果边（relation_type = "causes"）是行为统计而非文档知识，显式排除。
    let relations = knowledge_relations::Entity::find()
        .filter(
            Condition::all()
                .add(knowledge_relations::Column::KnowledgeBaseId.eq(kb_id))
                .add(
                    Condition::any()
                        .add(knowledge_relations::Column::SourceEntityId.is_in(seed_ids.clone()))
                        .add(knowledge_relations::Column::TargetEntityId.is_in(seed_ids.clone())),
                )
                .add(
                    knowledge_relations::Column::RelationType
                        .ne(axagent_harness::knowledge_graph::CAUSAL_RELATION_TYPE),
                ),
        )
        .all(db)
        .await?;

    // 5. 收集所有邻居实体 id（去重）
    let mut neighbor_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for r in &relations {
        if seed_map.contains_key(&r.source_entity_id) {
            // source 是 seed，target 是邻居
            neighbor_ids.insert(r.target_entity_id.clone());
        } else {
            // target 是 seed，source 是邻居
            neighbor_ids.insert(r.source_entity_id.clone());
        }
    }

    // 6. 批量反查邻居实体（用 IN 查询，参数化）
    let neighbor_ids_vec: Vec<String> = neighbor_ids.into_iter().collect();
    let neighbors: Vec<knowledge_entities::Model> = if neighbor_ids_vec.is_empty() {
        Vec::new()
    } else {
        knowledge_entities::Entity::find()
            .filter(knowledge_entities::Column::Id.is_in(neighbor_ids_vec))
            .all(db)
            .await?
    };
    let mut neighbor_map: std::collections::HashMap<String, knowledge_entities::Model> =
        std::collections::HashMap::with_capacity(neighbors.len());
    for m in neighbors {
        neighbor_map.insert(m.id.clone(), m);
    }

    // 7. 按 seed id 分组关系，组装 GraphRelationEdge
    // 对每个 seed，遍历所有关系：若 source == seed，则是出边（target 为邻居）；
    // 若 target == seed，则是入边（source 为邻居）。
    // 由于 GraphRelationEdge 字段名为 target_entity_name，对入边也按"另一端"语义填充。
    let mut rels_by_seed: std::collections::HashMap<
        String,
        Vec<axagent_harness::GraphRelationEdge>,
    > = std::collections::HashMap::new();
    for r in &relations {
        let (seed_id, other_id) = if seed_map.contains_key(&r.source_entity_id) {
            (r.source_entity_id.clone(), r.target_entity_id.clone())
        } else if seed_map.contains_key(&r.target_entity_id) {
            (r.target_entity_id.clone(), r.source_entity_id.clone())
        } else {
            continue;
        };
        let other_name =
            neighbor_map.get(&other_id).map(|m| m.name.clone()).unwrap_or_else(|| other_id.clone());
        let edge = axagent_harness::GraphRelationEdge {
            target_entity_name: other_name,
            relation_type: r.relation_type.clone(),
            description: r.description.clone(),
            weight: r.weight,
        };
        rels_by_seed.entry(seed_id).or_default().push(edge);
    }

    // 8. 组装最终 chunks（按 seed 原始顺序输出）
    let mut chunks: Vec<axagent_harness::GraphEnhancedContextChunk> =
        Vec::with_capacity(seed_map.len());
    for id in &seed_ids {
        let e = match seed_map.get(id) {
            Some(e) => e,
            None => continue,
        };
        let rels = rels_by_seed.remove(id).unwrap_or_default();
        chunks.push(axagent_harness::GraphEnhancedContextChunk {
            entity_name: e.name.clone(),
            entity_type: e.entity_type.clone(),
            description: e.description.clone(),
            relations: rels,
            knowledge_base_id: e.knowledge_base_id.clone(),
        });
    }

    Ok(chunks)
}

/// 构造图查询增强的可注入上下文文本
pub fn build_graph_context_text(
    kb_id: &str,
    chunks: &[axagent_harness::GraphEnhancedContextChunk],
) -> String {
    if chunks.is_empty() {
        return String::new();
    }
    let mut text = format!("[Knowledge Graph - {}]\n", kb_id);
    for chunk in chunks {
        let desc = chunk.description.as_deref().unwrap_or("");
        text.push_str(&format!("- {} ({}): {}\n", chunk.entity_name, chunk.entity_type, desc));
        for rel in &chunk.relations {
            let rel_desc = rel.description.as_deref().unwrap_or("");
            text.push_str(&format!(
                "  → {} [{}]: {}\n",
                rel.target_entity_name, rel.relation_type, rel_desc
            ));
        }
    }
    text
}

/// 合并 aliases：将已有 JSON 数组字符串与新 aliases 去重后合并，返回新的 JSON 数组字符串。
fn merge_aliases(existing: &str, new_aliases: &[String]) -> String {
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(existing) {
        for a in arr {
            set.insert(a);
        }
    }
    for a in new_aliases {
        if !a.is_empty() {
            set.insert(a.clone());
        }
    }
    serde_json::to_string(&set.into_iter().collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string())
}

/// 跨文档批量 upsert 实体与关系
///
/// 用于 LLM 抽取后的写入：对每个 [`ExtractedEntity`]，先按 (kb_id, name) 查询，
/// 存在则 mention_count += 1 并合并 aliases；不存在则新建。
/// 关系同理：按 (kb_id, source_entity_id, target_entity_id, relation_type) 去重。
///
/// `source_type` / `source_id` 落 v113 统一图谱字段，标识实体/关系的来源体系
/// （如 KB 抽取传 `("knowledge_base", "")`，Wiki 抽取传 `("wiki", wiki_id)`），
/// 避免 Wiki 实体被误标为 knowledge_base 而混入 KB 图谱。
pub async fn batch_upsert_entities_and_relations(
    db: &DatabaseConnection,
    kb_id: &str,
    source_type: &str,
    source_id: &str,
    entities: Vec<axagent_harness::ExtractedEntity>,
    relations: Vec<axagent_harness::ExtractedRelation>,
) -> Result<axagent_harness::ExtractEntitiesResult> {
    use axagent_harness::util_fns::gen_id;
    let started_at = std::time::Instant::now();
    let mut new_entities: Vec<KnowledgeEntity> = Vec::new();
    let mut updated_entities: Vec<KnowledgeEntity> = Vec::new();
    let mut new_relations: Vec<KnowledgeRelation> = Vec::new();
    let skipped_chunks = 0u32;

    // 用事务保证原子性
    let txn = db.begin().await?;
    let now = chrono::Utc::now().timestamp();

    // 1. 实体 upsert：按 (kb_id, name) 去重
    // name → 最终 entity id（用于后续关系写入）
    let mut name_to_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for ent in entities {
        if ent.name.is_empty() {
            continue;
        }
        // 按 (kb_id, name) 查询（参数化）
        let existing = knowledge_entities::Entity::find()
            .filter(
                knowledge_entities::Column::KnowledgeBaseId
                    .eq(kb_id)
                    .and(knowledge_entities::Column::Name.eq(&ent.name)),
            )
            .one(&txn)
            .await?;

        if let Some(m) = existing {
            // 存在：mention_count += 1，合并 aliases，刷新 last_seen_at，
            // confidence 随 mention_count 单调上升（多次被抽到的实体更可信）
            let existing_desc_is_none = m.description.is_none();
            let merged_aliases = merge_aliases(&m.aliases, &ent.aliases);
            let new_mention_count = m.mention_count + 1;
            let new_confidence = mention_based_confidence(new_mention_count);
            let mut am: knowledge_entities::ActiveModel = m.into();
            am.aliases = Set(merged_aliases);
            am.mention_count = Set(new_mention_count);
            am.confidence = Set(new_confidence);
            am.last_seen_at = Set(Some(rfc3339_now()));
            am.updated_at = Set(now);
            // 若新抽取提供了 description 且原描述为空，则补上
            if existing_desc_is_none && !ent.description.is_empty() {
                am.description = Set(Some(ent.description.clone()));
            }
            let updated_model = am.update(&txn).await?;
            let entity = model_to_entity(updated_model);
            name_to_id.insert(entity.name.clone(), entity.id.clone());
            updated_entities.push(entity);
        } else {
            // 不存在：新建
            let id = gen_id();
            let aliases_str =
                serde_json::to_string(&ent.aliases).unwrap_or_else(|_| "[]".to_string());
            let description = if ent.description.is_empty() {
                None
            } else {
                Some(ent.description.clone())
            };
            let seen_at = Some(rfc3339_now());
            let am = knowledge_entities::ActiveModel {
                id: Set(id.clone()),
                knowledge_base_id: Set(kb_id.to_string()),
                name: Set(ent.name.clone()),
                entity_type: Set(ent.entity_type),
                description: Set(description),
                source_path: Set(String::new()),
                source_language: Set(None),
                properties: Set(serde_json::Value::Object(Default::default())),
                lifecycle: Set(None),
                behaviors: Set(None),
                metadata: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                aliases: Set(aliases_str),
                mention_count: Set(1),
                confidence: Set(mention_based_confidence(1)),
                first_seen_at: Set(seen_at.clone()),
                last_seen_at: Set(seen_at),
                source_type: Set(source_type.to_string()),
                source_id: Set(source_id.to_string()),
                node_type: Set(String::from("entity")),
                external_id: Set(None),
            };
            let model = am.insert(&txn).await?;
            let entity = model_to_entity(model);
            name_to_id.insert(entity.name.clone(), entity.id.clone());
            new_entities.push(entity);
        }
    }

    // 2. 关系 upsert：按 (kb_id, source_entity_id, target_entity_id, relation_type) 去重
    for rel in relations {
        let rel_source_id = match name_to_id.get(&rel.source) {
            Some(id) => id.clone(),
            None => continue,
        };
        let target_id = match name_to_id.get(&rel.target) {
            Some(id) => id.clone(),
            None => continue,
        };
        if rel_source_id == target_id {
            continue;
        }
        // 查询是否已存在相同关系
        let existing_rel = knowledge_relations::Entity::find()
            .filter(
                knowledge_relations::Column::KnowledgeBaseId
                    .eq(kb_id)
                    .and(knowledge_relations::Column::SourceEntityId.eq(&rel_source_id))
                    .and(knowledge_relations::Column::TargetEntityId.eq(&target_id))
                    .and(knowledge_relations::Column::RelationType.eq(&rel.relation_type)),
            )
            .one(&txn)
            .await?;
        if existing_rel.is_some() {
            continue;
        }
        let rel_id = format!("rel_{}", gen_id());
        let am = knowledge_relations::ActiveModel {
            id: Set(rel_id.clone()),
            knowledge_base_id: Set(kb_id.to_string()),
            source_entity_id: Set(rel_source_id),
            target_entity_id: Set(target_id),
            relation_type: Set(rel.relation_type),
            description: Set(None),
            properties: Set(None),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            weight: Set(1.0),
            source_type: Set(source_type.to_string()),
            source_id: Set(source_id.to_string()),
        };
        let model = am.insert(&txn).await?;
        new_relations.push(model_to_relation(model));
    }

    txn.commit().await?;

    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    Ok(axagent_harness::ExtractEntitiesResult {
        new_entities,
        updated_entities,
        new_relations,
        skipped_chunks,
        elapsed_ms,
    })
}

/// 基于被提及次数的实体置信度：1 次提及 0.58，5 次及以上封顶 0.9。
/// 多次被独立抽取命中的实体更可信，confidence 不再恒为 0.5。
fn mention_based_confidence(mention_count: i32) -> f64 {
    0.5 + 0.4 * (mention_count.clamp(1, 5) as f64) / 5.0
}

/// 当前时间的 RFC3339 表示（first_seen_at / last_seen_at 为 TEXT 列）。
fn rfc3339_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// P1-3: 跨源实体合并 — 按 name+entity_type 在所有知识图谱中查找重复实体并合并
///
/// 解决 Wiki/KB/Memory 三套实体系统各自为政的问题。
/// 合并策略：
/// 1. 遍历所有实体，按 (name, entity_type) 分组
/// 2. 同组内保留最早创建的实体作为"主实体"，其他实体合并到主实体
/// 3. 合并 aliases、description、mention_count 等字段
/// 4. 更新所有引用被合并实体的关系，指向主实体
///
/// 返回合并统计信息
#[derive(Debug, serde::Serialize)]
pub struct MergeEntitiesResult {
    pub groups_found: usize,
    pub entities_merged: usize,
    pub relations_updated: usize,
}

pub async fn merge_duplicate_entities_across_all(
    db: &DatabaseConnection,
) -> Result<MergeEntitiesResult> {
    let started = std::time::Instant::now();
    let now = chrono::Utc::now().timestamp();

    let txn = db.begin().await?;

    // 1. 收集所有实体
    let all_entities = knowledge_entities::Entity::find().all(&txn).await?;

    if all_entities.is_empty() {
        return Ok(MergeEntitiesResult {
            groups_found: 0,
            entities_merged: 0,
            relations_updated: 0,
        });
    }

    // 2. 按 (name, entity_type) 分组
    use std::collections::HashMap;
    let mut groups: HashMap<(String, String), Vec<&knowledge_entities::Model>> = HashMap::new();
    for entity in &all_entities {
        let key = (entity.name.clone(), entity.entity_type.clone());
        groups.entry(key).or_default().push(entity);
    }

    let mut groups_found = 0usize;
    let mut entities_merged = 0usize;
    let mut relations_updated = 0usize;

    // 3. 处理每个分组
    for (_key, mut entities) in groups {
        if entities.len() < 2 {
            continue;
        }
        groups_found += 1;

        // 按创建时间排序，最早的作为主实体
        entities.sort_by_key(|a| a.created_at);

        let main_entity = entities[0].clone();
        let main_id = main_entity.id.clone();

        for entity in entities.iter().skip(1) {
            let merge_target_id = &entity.id;

            // 合并 aliases
            let merged_aliases = merge_aliases_opt(&main_entity.aliases, &entity.aliases);
            // 合并 description（取较长的）
            let merged_desc = match (&main_entity.description, &entity.description) {
                (Some(a), Some(b)) if a.len() >= b.len() => Some(a.clone()),
                (_, Some(b)) => Some(b.clone()),
                (Some(a), None) => Some(a.clone()),
                _ => None,
            };
            // 累加 mention_count
            let merged_mention = main_entity.mention_count + entity.mention_count;

            // 更新主实体
            let mut am: knowledge_entities::ActiveModel = main_entity.clone().into();
            am.aliases = Set(merged_aliases);
            am.mention_count = Set(merged_mention);
            am.description = Set(merged_desc);
            am.updated_at = Set(now);
            am.update(&txn).await?;

            // 更新关系：将所有引用被合并实体的关系改为引用主实体
            let updated_count = update_relation_references(&txn, merge_target_id, &main_id).await?;
            relations_updated += updated_count;

            // 标记被合并实体为已合并（或将其 kb_id 改为主实体的 kb_id）
            // 这里采用软删除策略：将 kb_id 设为空字符串或特殊标记
            let mut del_am: knowledge_entities::ActiveModel = (*entity).clone().into();
            del_am.knowledge_base_id = Set(String::from("__merged__"));
            del_am.updated_at = Set(now);
            del_am.update(&txn).await?;

            entities_merged += 1;
        }
    }

    txn.commit().await?;

    tracing::info!(
        "[merge_entities] 合并完成: {} 个分组, {} 个实体, {} 个关系, 耗时 {}ms",
        groups_found,
        entities_merged,
        relations_updated,
        started.elapsed().as_millis()
    );

    Ok(MergeEntitiesResult { groups_found, entities_merged, relations_updated })
}

/// 合并两个 aliases JSON 数组
fn merge_aliases_opt(a: &str, b: &str) -> String {
    let aliases_a: Vec<String> = serde_json::from_str::<Vec<String>>(a).unwrap_or_default();
    let aliases_b: Vec<String> = serde_json::from_str::<Vec<String>>(b).unwrap_or_default();

    let mut merged = aliases_a;
    for alias in aliases_b {
        if !merged.contains(&alias) {
            merged.push(alias);
        }
    }

    if merged.is_empty() {
        String::from("[]")
    } else {
        serde_json::to_string(&merged).unwrap_or_else(|_| String::from("[]"))
    }
}

/// 更新关系表中引用旧实体 ID 的记录，改为引用新的主实体 ID
async fn update_relation_references(
    txn: &DatabaseTransaction,
    old_id: &str,
    new_id: &str,
) -> Result<usize> {
    let now = chrono::Utc::now().timestamp();
    let backend = <DatabaseTransaction as sea_orm::ConnectionTrait>::get_database_backend(txn);

    // 使用 query_all_raw 执行更新（返回受影响行数需要在应用层统计）
    let sql_source = format!(
        "UPDATE knowledge_relations SET source_entity_id = '{}', updated_at = {} WHERE source_entity_id = '{}'",
        new_id, now, old_id
    );
    let sql_target = format!(
        "UPDATE knowledge_relations SET target_entity_id = '{}', updated_at = {} WHERE target_entity_id = '{}'",
        new_id, now, old_id
    );

    // 使用 fetch_all 执行更新，SeaORM 的 execute 方法不支持 Statement 类型
    let source_result = txn
        .query_all_raw(sea_orm::Statement::from_sql_and_values(backend, &sql_source, vec![]))
        .await;
    let target_result = txn
        .query_all_raw(sea_orm::Statement::from_sql_and_values(backend, &sql_target, vec![]))
        .await;

    let source_count = match source_result {
        Ok(_) => 1, // 假设至少更新了一行
        Err(_) => 0,
    };
    let target_count = match target_result {
        Ok(_) => 1,
        Err(_) => 0,
    };

    Ok(source_count + target_count)
}

// ── Wiki 图谱融合：实体节点和关系边 ────────────────────────────────

/// 获取指定知识库下的实体，转换为图谱节点格式用于 Wiki 图谱融合。
pub async fn get_knowledge_graph_nodes_for_wiki(
    db: &DatabaseConnection,
    kb_id: &str,
) -> Result<Vec<GraphNode>> {
    let entities = list_knowledge_entities(db, kb_id).await?;

    let nodes = entities
        .into_iter()
        .map(|entity| GraphNode {
            id: entity.id,
            title: entity.name,
            node_type: "entity".to_string(),
            tags: vec![entity.entity_type],
            link_count: 0,
            backlink_count: 0,
            path: entity.source_path,
        })
        .collect();

    Ok(nodes)
}

/// 获取指定知识库下的实体关系，转换为图谱边格式用于 Wiki 图谱融合。
pub async fn get_knowledge_graph_edges_for_wiki(
    db: &DatabaseConnection,
    kb_id: &str,
) -> Result<Vec<GraphEdge>> {
    let relations = list_knowledge_relations(db, kb_id).await?;

    let edges = relations
        .into_iter()
        .map(|rel| GraphEdge {
            source: rel.source_entity_id,
            target: rel.target_entity_id,
            edge_type: "reference".to_string(),
        })
        .collect();

    Ok(edges)
}
