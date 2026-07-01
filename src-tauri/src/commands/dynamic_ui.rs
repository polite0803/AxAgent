// SPDX-License-Identifier: AGPL-3.0-only

use axagent_entities::dynamic_ui_form_data::Column as FormDataColumn;
use axagent_entities::dynamic_ui_form_data::{
    ActiveModel as FormDataActiveModel, Entity as FormDataEntity, Model as FormDataModel,
};
use axagent_entities::dynamic_ui_schemas::{
    ActiveModel as SchemaActiveModel, Column as SchemaColumn, Entity as SchemaEntity,
    Model as SchemaModel,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DynamicUISchemaDTO {
    pub id: String,
    pub title: String,
    pub description: String,
    pub schema_json: String,
    pub category: String,
    pub tags: Vec<String>,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DynamicUIFormDataDTO {
    pub id: String,
    pub schema_id: String,
    pub form_data_json: String,
    pub instance_key: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateSchemaRequest {
    pub title: String,
    pub description: String,
    pub schema_json: String,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateSchemaRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub schema_json: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveFormDataRequest {
    pub schema_id: String,
    pub form_data_json: String,
    pub instance_key: Option<String>,
}

fn model_to_dto(model: SchemaModel) -> DynamicUISchemaDTO {
    let tags: Vec<String> = serde_json::from_str(&model.tags).unwrap_or_default();
    DynamicUISchemaDTO {
        id: model.id,
        title: model.title,
        description: model.description,
        schema_json: model.schema_json,
        category: model.category,
        tags,
        is_builtin: model.is_builtin != 0,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

fn form_data_model_to_dto(model: FormDataModel) -> DynamicUIFormDataDTO {
    DynamicUIFormDataDTO {
        id: model.id,
        schema_id: model.schema_id,
        form_data_json: model.form_data_json,
        instance_key: model.instance_key,
        updated_at: model.updated_at,
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[tauri::command]
pub async fn list_dynamic_ui_schemas(
    state: State<'_, AppState>,
    category: Option<String>,
) -> Result<Vec<DynamicUISchemaDTO>, String> {
    let db = state.harness.db();
    let mut query = SchemaEntity::find();
    if let Some(cat) = category {
        if !cat.is_empty() {
            query = query.filter(SchemaColumn::Category.eq(cat));
        }
    }
    let models = query
        .all(db)
        .await
        .map_err(|e| format!("查询Schema列表失败: {e}"))?;
    Ok(models.into_iter().map(model_to_dto).collect())
}

#[tauri::command]
pub async fn get_dynamic_ui_schema(
    state: State<'_, AppState>,
    id: String,
) -> Result<DynamicUISchemaDTO, String> {
    let db = state.harness.db();
    let model = SchemaEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| format!("查询Schema失败: {e}"))?
        .ok_or_else(|| "DynamicUI Schema 不存在".to_string())?;
    Ok(model_to_dto(model))
}

#[tauri::command]
pub async fn create_dynamic_ui_schema(
    state: State<'_, AppState>,
    req: CreateSchemaRequest,
) -> Result<DynamicUISchemaDTO, String> {
    let db = state.harness.db();
    let id = Uuid::new_v4().to_string();
    let now = now_iso();
    let tags_json = serde_json::to_string(&req.tags).unwrap_or_else(|_| "[]".to_string());

    let active = SchemaActiveModel {
        id: Set(id.clone()),
        title: Set(req.title),
        description: Set(req.description),
        schema_json: Set(req.schema_json),
        category: Set(req.category),
        tags: Set(tags_json),
        is_builtin: Set(0),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    };

    let model = active
        .insert(db)
        .await
        .map_err(|e| format!("创建Schema失败: {e}"))?;
    tracing::info!(id = %model.id, "创建 DynamicUI Schema");
    Ok(model_to_dto(model))
}

#[tauri::command]
pub async fn update_dynamic_ui_schema(
    state: State<'_, AppState>,
    id: String,
    req: UpdateSchemaRequest,
) -> Result<DynamicUISchemaDTO, String> {
    let db = state.harness.db();
    let model = SchemaEntity::find_by_id(id.clone())
        .one(db)
        .await
        .map_err(|e| format!("查询Schema失败: {e}"))?
        .ok_or_else(|| "DynamicUI Schema 不存在".to_string())?;

    if model.is_builtin != 0 {
        return Err("内置Schema不允许修改".to_string());
    }

    let mut active: SchemaActiveModel = model.into();
    if let Some(title) = req.title {
        active.title = Set(title);
    }
    if let Some(description) = req.description {
        active.description = Set(description);
    }
    if let Some(schema_json) = req.schema_json {
        active.schema_json = Set(schema_json);
    }
    if let Some(category) = req.category {
        active.category = Set(category);
    }
    if let Some(tags) = req.tags {
        active.tags = Set(serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()));
    }
    active.updated_at = Set(now_iso());

    let model = active
        .update(db)
        .await
        .map_err(|e| format!("更新Schema失败: {e}"))?;
    tracing::info!(id = %id, "更新 DynamicUI Schema");
    Ok(model_to_dto(model))
}

#[tauri::command]
pub async fn delete_dynamic_ui_schema(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let db = state.harness.db();
    let model = SchemaEntity::find_by_id(id.clone())
        .one(db)
        .await
        .map_err(|e| format!("查询Schema失败: {e}"))?
        .ok_or_else(|| "DynamicUI Schema 不存在".to_string())?;

    if model.is_builtin != 0 {
        return Err("内置Schema不允许删除".to_string());
    }

    // 级联删除关联的表单数据
    FormDataEntity::delete_many()
        .filter(FormDataColumn::SchemaId.eq(&id))
        .exec(db)
        .await
        .map_err(|e| format!("删除关联表单数据失败: {e}"))?;

    let res = SchemaEntity::delete_by_id(id.clone())
        .exec(db)
        .await
        .map_err(|e| format!("删除Schema失败: {e}"))?;

    if res.rows_affected == 0 {
        return Err("DynamicUI Schema 不存在".to_string());
    }
    tracing::info!(id = %id, "删除 DynamicUI Schema");
    Ok(())
}

#[tauri::command]
pub async fn save_dynamic_ui_form_data(
    state: State<'_, AppState>,
    req: SaveFormDataRequest,
) -> Result<DynamicUIFormDataDTO, String> {
    let db = state.harness.db();
    let instance_key = req.instance_key.unwrap_or_else(|| "default".to_string());
    let now = now_iso();

    // 先查询是否存在
    let existing = FormDataEntity::find()
        .filter(FormDataColumn::SchemaId.eq(&req.schema_id))
        .filter(FormDataColumn::InstanceKey.eq(&instance_key))
        .one(db)
        .await
        .map_err(|e| format!("查询表单数据失败: {e}"))?;

    let model = if let Some(existing) = existing {
        let mut active: FormDataActiveModel = existing.into();
        active.form_data_json = Set(req.form_data_json);
        active.updated_at = Set(now);
        active
            .update(db)
            .await
            .map_err(|e| format!("更新表单数据失败: {e}"))?
    } else {
        let active = FormDataActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            schema_id: Set(req.schema_id),
            form_data_json: Set(req.form_data_json),
            instance_key: Set(instance_key),
            updated_at: Set(now),
        };
        active
            .insert(db)
            .await
            .map_err(|e| format!("保存表单数据失败: {e}"))?
    };

    Ok(form_data_model_to_dto(model))
}

#[tauri::command]
pub async fn get_dynamic_ui_form_data(
    state: State<'_, AppState>,
    schema_id: String,
    instance_key: Option<String>,
) -> Result<Option<DynamicUIFormDataDTO>, String> {
    let db = state.harness.db();
    let key = instance_key.unwrap_or_else(|| "default".to_string());
    let model = FormDataEntity::find()
        .filter(FormDataColumn::SchemaId.eq(&schema_id))
        .filter(FormDataColumn::InstanceKey.eq(key))
        .one(db)
        .await
        .map_err(|e| format!("查询表单数据失败: {e}"))?;
    Ok(model.map(form_data_model_to_dto))
}

#[tauri::command]
pub async fn delete_dynamic_ui_form_data(
    state: State<'_, AppState>,
    schema_id: String,
    instance_key: Option<String>,
) -> Result<(), String> {
    let db = state.harness.db();
    let key = instance_key.unwrap_or_else(|| "default".to_string());
    FormDataEntity::delete_many()
        .filter(FormDataColumn::SchemaId.eq(&schema_id))
        .filter(FormDataColumn::InstanceKey.eq(key))
        .exec(db)
        .await
        .map_err(|e| format!("删除表单数据失败: {e}"))?;
    Ok(())
}
