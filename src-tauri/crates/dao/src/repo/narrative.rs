// SPDX-License-Identifier: AGPL-3.0-only

//! 叙事结构（v126）DAO —— 弧线/交汇点/伏笔持久化。
//!
//! 契约对齐前端 `src/lib/narrativeStructure.ts`：
//! list（按模板/体裁过滤）→ get → create → update（version+1）→ delete。

use sea_orm::*;

use axagent_entities::narrative_structures;
use axagent_harness::core_error::{AxAgentError, Result};

pub async fn list_narrative_structures(
    db: &DatabaseConnection,
    is_template: Option<bool>,
    genre: Option<&str>,
) -> Result<Vec<narrative_structures::Model>> {
    let mut select = narrative_structures::Entity::find();
    if let Some(t) = is_template {
        select = select.filter(narrative_structures::Column::IsTemplate.eq(t));
    }
    if let Some(g) = genre {
        select = select.filter(narrative_structures::Column::Genre.eq(g));
    }
    let rows = select.order_by_desc(narrative_structures::Column::UpdatedAt).all(db).await?;
    Ok(rows)
}

pub async fn get_narrative_structure(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<narrative_structures::Model>> {
    let row = narrative_structures::Entity::find_by_id(id).one(db).await?;
    Ok(row)
}

pub async fn insert_narrative_structure(
    db: &DatabaseConnection,
    active: narrative_structures::ActiveModel,
) -> Result<narrative_structures::Model> {
    let row = active.insert(db).await?;
    Ok(row)
}

/// 更新叙事结构：`version` 递增、`updated_at` 刷新；不存在返回 NotFound。
pub async fn update_narrative_structure(
    db: &DatabaseConnection,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    genre: Option<String>,
    structure: Option<String>,
) -> Result<narrative_structures::Model> {
    let existing = narrative_structures::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("NarrativeStructure {id}")))?;

    let mut active: narrative_structures::ActiveModel = existing.into();
    if let Some(n) = name {
        active.name = Set(n);
    }
    // 注意：description 允许显式置空（Some(None) 语义由命令层转换，这里
    // 只处理「提供了新值」），避免 Option<Option<T>> 泛滥。
    if description.is_some() {
        active.description = Set(description);
    }
    if let Some(g) = genre {
        active.genre = Set(g);
    }
    if let Some(s) = structure {
        active.structure = Set(s);
    }
    active.version = Set(active.version.unwrap() + 1);
    active.updated_at = Set(chrono::Utc::now().timestamp_millis());

    let row = active.update(db).await?;
    Ok(row)
}

pub async fn delete_narrative_structure(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = narrative_structures::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("NarrativeStructure {id}")));
    }
    Ok(())
}
