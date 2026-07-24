//! 知识图谱加载器 — 从 DB 读取概念/行业/股票关系到 ConceptIndex
//!
//! 数据由 `import_lemonhu_knowledge` 命令（commands/knowledge.rs）从
//! knowledge-sources/lemonhu/ 导入到 DB，本模块只做读取。
//!
//! ## 定位
//!
//! 启动时自动调用，找名为 "开源股票知识库(lemonhu)" 的 knowledge_base，
//! 读取其 `knowledge_entities` + `knowledge_relations` 表，填充 ConceptIndex。

use std::collections::{HashMap, HashSet};

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::concept_index::{ConceptIndex, ConceptNode};

/// 从 DB 加载 ConceptIndex
///
/// 查找名为 "开源股票知识库(lemonhu)" 的 knowledge_base，
/// 读取其中的 has_concept / in_industry 关系。
/// 若找不到该知识库，返回 Ok(0)（静默降级）。
pub async fn load_concept_index_from_db(
    index: &mut ConceptIndex,
    db: &DatabaseConnection,
) -> Result<usize, String> {
    use axagent_entities::knowledge_bases;
    use axagent_entities::knowledge_entities;
    use axagent_entities::knowledge_relations;

    // 1. 找 knowledge_base
    let kb: Option<knowledge_bases::Model> = knowledge_bases::Entity::find()
        .filter(knowledge_bases::Column::Name.eq("开源股票知识库(lemonhu)"))
        .one(db)
        .await
        .map_err(|e| format!("查 knowledge_bases 失败: {e}"))?;

    let kb_id = match kb {
        Some(k) => k.id,
        None => return Ok(0), // 没导入过，静默降级
    };

    // 2. 读取 has_concept / in_industry 关系
    let relations = knowledge_relations::Entity::find()
        .filter(knowledge_relations::Column::KnowledgeBaseId.eq(&kb_id))
        .filter(
            knowledge_relations::Column::RelationType
                .is_in(vec!["has_concept".to_string(), "in_industry".to_string()]),
        )
        .all(db)
        .await
        .map_err(|e| format!("查 knowledge_relations 失败: {e}"))?;

    if relations.is_empty() {
        return Ok(0);
    }

    // 3. 收集涉及的实体 ID
    let mut entity_ids = HashSet::new();
    for r in &relations {
        entity_ids.insert(r.source_entity_id.clone());
        entity_ids.insert(r.target_entity_id.clone());
    }

    let entity_ids_vec: Vec<String> = entity_ids.into_iter().collect();
    let entities = knowledge_entities::Entity::find()
        .filter(knowledge_entities::Column::Id.is_in(entity_ids_vec))
        .all(db)
        .await
        .map_err(|e| format!("查 knowledge_entities 失败: {e}"))?;

    // 4. 注册概念/行业节点
    for e in &entities {
        if e.entity_type == "concept" || e.entity_type == "industry" {
            let node =
                ConceptNode::new(&e.id, &e.name, &e.entity_type).with_aliases(&[e.name.as_str()]);
            index.register(node);
        }
    }

    let entity_type_map: HashMap<String, String> =
        entities.into_iter().map(|e| (e.id.clone(), e.entity_type)).collect();

    // 5. 填充成员关系
    let mut total = 0usize;
    for r in &relations {
        let source_is_stock =
            entity_type_map.get(&r.source_entity_id).map(|t| t == "company").unwrap_or(false);
        let target_is_concept = entity_type_map
            .get(&r.target_entity_id)
            .map(|t| t == "concept" || t == "industry")
            .unwrap_or(false);
        if source_is_stock && target_is_concept {
            index.add_membership(&r.target_entity_id, &r.source_entity_id);
            total += 1;
        }
    }

    tracing::info!(
        "[ConceptIndex] 从 DB(kb={kb_id}) 加载 {total} 条概念成员关系, {} 个实体",
        entity_type_map.len()
    );

    Ok(total)
}
