// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::narrative_structure;
use axagent_harness::core_error::Result;
use axagent_harness::narrative::NarrativeStructure;

/// 创建叙事结构
pub async fn create_narrative_structure(
    db: &DatabaseConnection,
    id: String,
    name: String,
    description: Option<String>,
    genre: String,
    structure: &NarrativeStructure,
    is_template: bool,
) -> Result<narrative_structure::Model> {
    let now = chrono::Utc::now().timestamp_millis();

    let model = narrative_structure::ActiveModel {
        id: Set(id),
        name: Set(name),
        description: Set(description),
        genre: Set(genre),
        arcs: Set(serde_json::to_string(&structure.arcs).unwrap_or_default()),
        confluences: Set(serde_json::to_string(&structure.confluences).unwrap_or_default()),
        foreshadows: Set(serde_json::to_string(&structure.foreshadows).unwrap_or_default()),
        is_template: Set(is_template),
        version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let result = model.insert(db).await?;
    Ok(result)
}

/// 根据 ID 获取叙事结构
pub async fn get_narrative_structure(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<narrative_structure::Model>> {
    let result = narrative_structure::Entity::find_by_id(id).one(db).await?;
    Ok(result)
}

/// 列出所有叙事结构（可按模板/类型过滤）
pub async fn list_narrative_structures(
    db: &DatabaseConnection,
    is_template: Option<bool>,
    genre: Option<String>,
) -> Result<Vec<narrative_structure::Model>> {
    let mut query = narrative_structure::Entity::find();

    if let Some(template) = is_template {
        query = query.filter(narrative_structure::Column::IsTemplate.eq(template));
    }

    if let Some(g) = genre {
        query = query.filter(narrative_structure::Column::Genre.eq(g));
    }

    let results =
        query.order_by(narrative_structure::Column::UpdatedAt, Order::Desc).all(db).await?;
    Ok(results)
}

/// 更新叙事结构
pub async fn update_narrative_structure(
    db: &DatabaseConnection,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    genre: Option<String>,
    structure: Option<&NarrativeStructure>,
) -> Result<narrative_structure::Model> {
    let existing = narrative_structure::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("NarrativeStructure not found".into()))?;

    let current_version = existing.version;
    let mut active: narrative_structure::ActiveModel = existing.into();
    let now = chrono::Utc::now().timestamp_millis();

    if let Some(n) = name {
        active.name = Set(n);
    }
    if let Some(d) = description {
        active.description = Set(Some(d));
    }
    if let Some(g) = genre {
        active.genre = Set(g);
    }
    if let Some(s) = structure {
        active.arcs = Set(serde_json::to_string(&s.arcs).unwrap_or_default());
        active.confluences = Set(serde_json::to_string(&s.confluences).unwrap_or_default());
        active.foreshadows = Set(serde_json::to_string(&s.foreshadows).unwrap_or_default());
        active.version = Set(current_version + 1);
    }
    active.updated_at = Set(now);

    let result = active.update(db).await?;
    Ok(result)
}

/// 删除叙事结构
pub async fn delete_narrative_structure(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = narrative_structure::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(sea_orm::DbErr::RecordNotFound("NarrativeStructure not found".into()).into());
    }
    Ok(())
}

/// 将数据库记录转换为 NarrativeStructure DTO
pub fn model_to_dto(model: &narrative_structure::Model) -> NarrativeStructure {
    let arcs = serde_json::from_str(&model.arcs).unwrap_or_default();
    let confluences = serde_json::from_str(&model.confluences).unwrap_or_default();
    let foreshadows = serde_json::from_str(&model.foreshadows).unwrap_or_default();

    NarrativeStructure { arcs, confluences, foreshadows }
}
