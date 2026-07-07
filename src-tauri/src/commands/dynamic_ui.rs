// SPDX-License-Identifier: AGPL-3.0-only

use axagent_entities::dynamic_ui_form_data::Column as FormDataColumn;
use axagent_entities::dynamic_ui_form_data::{
    ActiveModel as FormDataActiveModel, Entity as FormDataEntity, Model as FormDataModel,
};
use axagent_entities::dynamic_ui_schema_versions::{
    ActiveModel as VersionActiveModel, Entity as VersionEntity, Model as VersionModel,
};
use axagent_entities::dynamic_ui_schemas::{
    ActiveModel as SchemaActiveModel, Column as SchemaColumn, Entity as SchemaEntity,
    Model as SchemaModel,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing;
use uuid::Uuid;

use crate::AppState;

// ── DTOs ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DynamicUISchemaDTO {
    pub id: String,
    pub title: String,
    pub description: String,
    pub schema_json: String,
    pub category: String,
    pub tags: Vec<String>,
    pub version: String,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DynamicUISchemaVersionDTO {
    pub id: i64,
    pub schema_id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub schema_json: String,
    pub category: String,
    pub tags: Vec<String>,
    pub change_log: String,
    pub created_at: i64,
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
    /// 语义化版本号（可选）。不传则自动递增 patch 版本。
    /// 传了但低于等于当前版本 → 报错。
    pub version: Option<String>,
    /// 变更说明（可选），记录到版本快照中
    pub change_log: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveFormDataRequest {
    pub schema_id: String,
    pub form_data_json: String,
    pub instance_key: Option<String>,
}

// ── 版本管理 DTO ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListVersionsResponse {
    pub versions: Vec<DynamicUISchemaVersionDTO>,
    pub current_version: String,
}

// ── 转换函数 ──

fn model_to_dto(model: SchemaModel) -> DynamicUISchemaDTO {
    let tags: Vec<String> = serde_json::from_str(&model.tags).unwrap_or_default();
    DynamicUISchemaDTO {
        id: model.id,
        title: model.title,
        description: model.description,
        schema_json: model.schema_json,
        category: model.category,
        tags,
        version: model.version,
        is_builtin: model.is_builtin != 0,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

fn version_model_to_dto(model: VersionModel) -> DynamicUISchemaVersionDTO {
    let tags: Vec<String> = serde_json::from_str(&model.tags).unwrap_or_default();
    DynamicUISchemaVersionDTO {
        id: model.id,
        schema_id: model.schema_id,
        version: model.version,
        title: model.title,
        description: model.description,
        schema_json: model.schema_json,
        category: model.category,
        tags,
        change_log: model.change_log,
        created_at: model.created_at,
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

// ── 语义化版本比较（支持 major.minor.patch，如 "2.1.3") ──

/// 解析 "major.minor.patch" 版本号为三元组。
/// 非法格式返回 None（视为 0.0.0）。
fn parse_semver(v: &str) -> (u32, u32, u32) {
    let parts: Vec<&str> = v.split('.').collect();
    let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

/// 如果 `a > b` 返回 true
fn semver_gt(a: &str, b: &str) -> bool {
    parse_semver(a) > parse_semver(b)
}

/// patch 版本自增（如 "1.2.3" → "1.2.4"）
fn bump_patch(v: &str) -> String {
    let (major, minor, patch) = parse_semver(v);
    format!("{}.{}.{}", major, minor, patch + 1)
}

/// 取当前秒时间戳
fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 创建版本快照
async fn create_version_snapshot(
    db: &sea_orm::DatabaseConnection,
    schema_id: &str,
    version: &str,
    model: &SchemaModel,
    change_log: &str,
) -> Result<i64, String> {
    let now = now_unix();
    let tags_json = serde_json::to_string(
        &serde_json::from_str::<Vec<String>>(&model.tags).unwrap_or_default(),
    )
    .unwrap_or_else(|_| "[]".to_string());

    let am = VersionActiveModel {
        schema_id: Set(schema_id.to_string()),
        version: Set(version.to_string()),
        title: Set(model.title.clone()),
        description: Set(model.description.clone()),
        schema_json: Set(model.schema_json.clone()),
        category: Set(model.category.clone()),
        tags: Set(tags_json),
        change_log: Set(change_log.to_string()),
        created_at: Set(now),
        ..Default::default()
    };

    let result = am.insert(db).await.map_err(|e| format!("创建版本快照失败: {e}"))?;
    Ok(result.id)
}

/// 清理旧版本：每个 schema 保留最近 30 个版本
async fn cleanup_old_versions(
    db: &sea_orm::DatabaseConnection,
    schema_id: &str,
    keep: usize,
) -> Result<usize, String> {
    let all = VersionEntity::find()
        .filter(axagent_entities::dynamic_ui_schema_versions::Column::SchemaId.eq(schema_id))
        .order_by(axagent_entities::dynamic_ui_schema_versions::Column::CreatedAt, Order::Desc)
        .all(db)
        .await
        .map_err(|e| format!("查询版本列表失败: {e}"))?;

    if all.len() <= keep {
        return Ok(0);
    }

    let to_delete: Vec<i64> = all.into_iter().skip(keep).map(|m| m.id).collect();
    let count = to_delete.len();
    for id in to_delete {
        VersionEntity::delete_by_id(id)
            .exec(db)
            .await
            .map_err(|e| format!("删除旧版本失败: {e}"))?;
    }
    Ok(count)
}

// ── Schema CRUD ──

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
    let models = query.all(db).await.map_err(|e| format!("查询Schema列表失败: {e}"))?;
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
        version: Set("1.0.0".to_string()),
        is_builtin: Set(0),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    };

    let model = active.insert(db).await.map_err(|e| format!("创建Schema失败: {e}"))?;

    // 创建初始版本快照
    create_version_snapshot(db, &model.id, "1.0.0", &model, "初始版本").await?;

    tracing::info!(id = %model.id, version = %model.version, "创建 DynamicUI Schema");
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

    // ── 版本更替逻辑 ──
    let current_version = &model.version;
    let change_log = req.change_log.unwrap_or_else(|| "更新配置".to_string());

    // 确定新版本号
    let new_version = if let Some(ref v) = req.version {
        // 显式指定版本：必须高于当前版本
        if !semver_gt(v, current_version) {
            return Err(format!(
                "版本号冲突：新版本 {} 不高于当前版本 {}。请使用更高的版本号。",
                v, current_version
            ));
        }
        v.clone()
    } else if req.schema_json.is_some() {
        // 修改了 schema_json 但未指定版本 → 自动递增 patch
        bump_patch(current_version)
    } else {
        // 只改元数据（title/desc/category/tags）→ 版本不变
        current_version.clone()
    };

    // ── 创建旧版本快照（在 schema_json 变更时） ──
    if req.schema_json.is_some() {
        create_version_snapshot(db, &id, current_version, &model, &change_log).await?;
    }

    // ── 执行更新 ──
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
    active.version = Set(new_version);
    active.updated_at = Set(now_iso());

    let updated = active.update(db).await.map_err(|e| format!("更新Schema失败: {e}"))?;

    // ── 清理旧版本（保留最近 30 个） ──
    let cleaned = cleanup_old_versions(db, &id, 30).await?;
    if cleaned > 0 {
        tracing::info!(schema_id = %id, count = cleaned, "清理旧版本快照");
    }

    tracing::info!(id = %id, version = %updated.version, "更新 DynamicUI Schema");
    Ok(model_to_dto(updated))
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

    // 级联删除版本历史
    VersionEntity::delete_many()
        .filter(axagent_entities::dynamic_ui_schema_versions::Column::SchemaId.eq(&id))
        .exec(db)
        .await
        .map_err(|e| format!("删除版本历史失败: {e}"))?;

    let res = SchemaEntity::delete_by_id(id.clone())
        .exec(db)
        .await
        .map_err(|e| format!("删除Schema失败: {e}"))?;

    if res.rows_affected == 0 {
        return Err("DynamicUI Schema 不存在".to_string());
    }
    tracing::info!(id = %id, "删除 DynamicUI Schema 及其版本历史");
    Ok(())
}

// ── 版本管理命令 ──

/// 查询指定 schema 的所有版本历史
#[tauri::command]
pub async fn list_dynamic_ui_schema_versions(
    state: State<'_, AppState>,
    schema_id: String,
) -> Result<ListVersionsResponse, String> {
    let db = state.harness.db();

    // 获取当前 schema 信息
    let schema = SchemaEntity::find_by_id(&schema_id)
        .one(db)
        .await
        .map_err(|e| format!("查询Schema失败: {e}"))?
        .ok_or_else(|| "DynamicUI Schema 不存在".to_string())?;

    let current_version = schema.version;

    // 查询版本历史（按时间倒序）
    let models = VersionEntity::find()
        .filter(axagent_entities::dynamic_ui_schema_versions::Column::SchemaId.eq(&schema_id))
        .order_by(axagent_entities::dynamic_ui_schema_versions::Column::CreatedAt, Order::Desc)
        .limit(50)
        .all(db)
        .await
        .map_err(|e| format!("查询版本历史失败: {e}"))?;

    Ok(ListVersionsResponse {
        versions: models.into_iter().map(version_model_to_dto).collect(),
        current_version,
    })
}

/// 获取指定版本的详细信息
#[tauri::command]
pub async fn get_dynamic_ui_schema_version(
    state: State<'_, AppState>,
    version_id: i64,
) -> Result<DynamicUISchemaVersionDTO, String> {
    let db = state.harness.db();
    let model = VersionEntity::find_by_id(version_id)
        .one(db)
        .await
        .map_err(|e| format!("查询版本失败: {e}"))?
        .ok_or_else(|| format!("版本 {} 不存在", version_id))?;
    Ok(version_model_to_dto(model))
}

/// 回滚到指定版本（会创建当前版本的快照，然后覆盖 schema）
#[tauri::command]
pub async fn restore_dynamic_ui_schema_version(
    state: State<'_, AppState>,
    schema_id: String,
    version_id: i64,
) -> Result<DynamicUISchemaDTO, String> {
    let db = state.harness.db();

    let schema = SchemaEntity::find_by_id(&schema_id)
        .one(db)
        .await
        .map_err(|e| format!("查询Schema失败: {e}"))?
        .ok_or_else(|| "DynamicUI Schema 不存在".to_string())?;

    if schema.is_builtin != 0 {
        return Err("内置Schema不允许回滚".to_string());
    }

    let version = VersionEntity::find_by_id(version_id)
        .one(db)
        .await
        .map_err(|e| format!("查询版本失败: {e}"))?
        .ok_or_else(|| format!("版本 {} 不存在", version_id))?;

    // 保存当前版本的快照（回滚前存档）
    let old_version = schema.version.clone();
    create_version_snapshot(
        db,
        &schema_id,
        &old_version,
        &schema,
        &format!("回滚前存档（回滚到 {})", version.version),
    )
    .await?;

    // 解析版本号用于递增
    let restore_version = bump_patch(&version.version);

    // 将目标版本的 schema_json + 元数据写回主表
    let mut active: SchemaActiveModel = schema.into();
    active.schema_json = Set(version.schema_json);
    active.title = Set(version.title);
    active.description = Set(version.description);
    active.category = Set(version.category);
    active.tags = Set(version.tags);
    active.version = Set(restore_version);
    active.updated_at = Set(now_iso());

    let updated = active.update(db).await.map_err(|e| format!("回滚Schema失败: {e}"))?;

    tracing::info!(
        schema_id = %schema_id,
        from_version = %old_version,
        to_version = %version.version,
        restored_version = %updated.version,
        "回滚 DynamicUI Schema"
    );

    Ok(model_to_dto(updated))
}

// ── 表单数据命令（不变） ──

#[tauri::command]
pub async fn save_dynamic_ui_form_data(
    state: State<'_, AppState>,
    req: SaveFormDataRequest,
) -> Result<DynamicUIFormDataDTO, String> {
    let db = state.harness.db();
    let instance_key = req.instance_key.unwrap_or_else(|| "default".to_string());
    let now = now_iso();

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
        active.update(db).await.map_err(|e| format!("更新表单数据失败: {e}"))?
    } else {
        let active = FormDataActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            schema_id: Set(req.schema_id),
            form_data_json: Set(req.form_data_json),
            instance_key: Set(instance_key),
            updated_at: Set(now),
        };
        active.insert(db).await.map_err(|e| format!("保存表单数据失败: {e}"))?
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
