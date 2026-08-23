// SPDX-License-Identifier: AGPL-3.0-only

use axagent_agent_macro::agent_command;

use axagent_entities::dynamic_ui_form_data::Column as FormDataColumn;
use axagent_entities::dynamic_ui_form_data::{
    ActiveModel as FormDataActiveModel, Entity as FormDataEntity, Model as FormDataModel,
};
use axagent_entities::dynamic_ui_pins::{
    ActiveModel as PinActiveModel, Column as PinColumn, Entity as PinEntity, Model as PinModel,
};
use axagent_entities::dynamic_ui_schema_versions::{
    ActiveModel as VersionActiveModel, Entity as VersionEntity, Model as VersionModel,
};
use axagent_entities::dynamic_ui_schemas::{
    ActiveModel as SchemaActiveModel, Column as SchemaColumn, Entity as SchemaEntity,
    Model as SchemaModel,
};
use axagent_runtime::llm_bridge::build_llm_bridge_from_db;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing;
use uuid::Uuid;

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::dynamic_ui as dynamic_ui_err;

// ── DTOs ──

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct DynamicUIFormDataDTO {
    pub id: String,
    pub schema_id: String,
    pub form_data_json: String,
    pub instance_key: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateSchemaRequest {
    pub title: String,
    pub description: String,
    pub schema_json: String,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct SaveFormDataRequest {
    pub schema_id: String,
    pub form_data_json: String,
    pub instance_key: Option<String>,
}

// ── 版本管理 DTO ──

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
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

#[agent_command(domain = "dynamic_ui", safety = Safe, call_mode = StateInput, description = "列出动态UI Schema")]
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

#[agent_command(domain = "dynamic_ui", safety = Safe, call_mode = StateInput, description = "获取动态UI Schema详情")]
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

#[agent_command(domain = "dynamic_ui", safety = Caution, call_mode = StateInput, description = "创建动态UI Schema")]
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

#[agent_command(domain = "dynamic_ui", safety = Caution, call_mode = StateInput, description = "更新动态UI Schema")]
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
        return Err(ErrorResponse::err(dynamic_ui_err::BUILTIN_NOT_MODIFIABLE));
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

#[agent_command(domain = "dynamic_ui", safety = Dangerous, call_mode = StateInput, description = "删除动态UI Schema")]
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
        return Err(ErrorResponse::err(dynamic_ui_err::BUILTIN_NOT_DELETABLE));
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
        return Err(ErrorResponse::err(dynamic_ui_err::SCHEMA_NOT_FOUND));
    }
    tracing::info!(id = %id, "删除 DynamicUI Schema 及其版本历史");
    Ok(())
}

// ── 版本管理命令 ──

/// 查询指定 schema 的所有版本历史
#[agent_command(domain = "dynamic_ui", safety = Safe, call_mode = StateInput, description = "列出Schema版本历史")]
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
#[agent_command(domain = "dynamic_ui", safety = Safe, call_mode = StateInput, description = "获取Schema版本详情")]
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
#[agent_command(domain = "dynamic_ui", safety = Caution, call_mode = StateInput, description = "回滚Schema到指定版本")]
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
        return Err(ErrorResponse::err(dynamic_ui_err::BUILTIN_NOT_ROLLBACK));
    }

    let version = VersionEntity::find_by_id(version_id)
        .one(db)
        .await
        .map_err(|e| format!("查询版本失败: {e}"))?
        .ok_or_else(|| format!("版本 {} 不存在", version_id))?;

    // 校验版本归属：防止把 Schema A 的历史版本回滚到 Schema B（跨 Schema 数据污染）
    if version.schema_id != schema_id {
        return Err(format!("版本 {} 不属于 Schema {}，拒绝回滚", version_id, schema_id));
    }

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

    // 回滚后 live 版本直接沿用目标版本号（不 bump），使版本线清晰可追溯（D-08）
    let restore_version = version.version.clone();

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

#[agent_command(domain = "dynamic_ui", safety = Caution, call_mode = StateInput, description = "保存动态UI表单数据")]
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

#[agent_command(domain = "dynamic_ui", safety = Safe, call_mode = StateInput, description = "获取动态UI表单数据")]
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

#[agent_command(domain = "dynamic_ui", safety = Dangerous, call_mode = StateInput, description = "删除动态UI表单数据")]
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

// ── NL2UI 自然语言编辑 ──

/// NL2UI 编辑结果：修改后的完整 Schema + AI 简述
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EditSchemaNlResult {
    /// 修改后的完整 Schema JSON 字符串
    pub schema: String,
    /// 编辑说明（AI 对本次修改的简述）
    pub description: String,
}

/// NL2UI 编辑系统提示词：约束 LLM 只输出合法、完整的 UI Schema JSON
const NL_EDIT_SYSTEM_PROMPT: &str = r#"你是一个 UI Schema 编辑器。你会收到一个已有的 UI Schema（JSON）和一条自然语言编辑指令。
请严格按照指令修改 Schema，并只输出修改后的【完整】Schema JSON（不要省略任何未被指令修改的部分）。

UI Schema 节点结构（每个节点都是一个 JSON 对象）：
- version: 字符串，如 "1.0"
- id: 字符串，节点唯一标识
- type: 组件类型，必须是以下之一：
  Container, Row, Column, Grid, Card, Tabs, Accordion, Form, Input, Number,
  Select, DatePicker, Switch, Checkbox, Radio, Textarea, Table, Chart, List,
  Dashboard, CodeEditor, FilePreview, Markdown, Image, Button, Text, Divider,
  Progress, Tag, Tree, Timeline
- props: 对象，组件属性（如 Input 的 {name,label,placeholder?,required?}；Text 的 {content,strong?}；Form 的 {layout,submitText?}）
- children: 可选数组，子节点
- events: 可选数组，事件处理器，如 {trigger:"onSubmit", actions:[{type:"store", config:{}}]}
- dataSource / conditionalDisplay / style: 可选

编辑规则：
1. 输出必须是合法 JSON 对象，且是完整的根 Schema（必须包含 version、id、type、props）。
2. 只修改指令要求的部分，完整保留其他结构与数据。
3. 不要输出任何解释性文字，不要用 Markdown 代码块（```）包裹。
4. 修改已有节点时复用其原有 id；新增节点请使用语义化 id。
5. 如果指令要求新增字段，请将其放入合适的 Form 或容器节点的 children 中。"#;

/// 去除 LLM 输出可能携带的 Markdown 代码块包裹（```json ... ``` 或 ``` ... ```）
fn strip_code_fence(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let rest = rest.trim_start_matches("json").trim_start();
        if let Some(end) = rest.rfind("```") {
            return rest[..end].trim().to_string();
        }
        return rest.trim().to_string();
    }
    trimmed.to_string()
}

/// 基于自然语言指令编辑已有 UI Schema（缺陷 7 补齐：后端 AI 精准编辑）
///
/// 优先调用首个启用的 LLM provider 进行精准编辑；未配置 provider 或调用失败时返回错误，
/// 由前端 `nl2ui-edit.ts` 降级为本地重新生成。
#[agent_command(domain = "dynamic_ui", safety = Caution, call_mode = StateInput, description = "AI编辑动态UI Schema")]
#[tauri::command]
pub async fn edit_dynamic_ui_schema_nl(
    state: State<'_, AppState>,
    existing_schema: String,
    prompt: String,
) -> Result<EditSchemaNlResult, String> {
    if prompt.trim().is_empty() {
        return Err(ErrorResponse::err(dynamic_ui_err::EDIT_PROMPT_EMPTY));
    }

    // 1. 解析现有 schema，确保输入合法
    let existing: serde_json::Value =
        serde_json::from_str(&existing_schema).map_err(|e| format!("现有 Schema 解析失败: {e}"))?;

    // 2. 获取 LLM bridge（首个启用的 provider）
    let master_key = state.harness.master_key_owned();
    let bridge = build_llm_bridge_from_db(&master_key)
        .await
        .ok_or_else(|| "未配置可用的 LLM provider，无法执行 AI 编辑".to_string())?;

    // 3. 构造提示词
    let existing_pretty = serde_json::to_string_pretty(&existing).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let user_prompt = format!(
        "现有 Schema:\n{existing_pretty}\n\n编辑指令: {prompt}\n\n请输出修改后的完整 Schema JSON。"
    );

    // 4. 调用 LLM
    let response = bridge
        .call_llm(NL_EDIT_SYSTEM_PROMPT, &user_prompt)
        .await
        .map_err(|e| format!("AI 编辑调用失败: {e}"))?;

    // 5. 解析 LLM 输出（兼容 {schema, description} 包装或裸 schema）
    let cleaned = strip_code_fence(&response);
    let value: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| format!("AI 返回的 Schema 不是合法 JSON: {e}"))?;

    let (schema_value, description) = if let Some(s) = value.get("schema") {
        let desc = value.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string();
        (s.clone(), desc)
    } else {
        (value.clone(), String::new())
    };

    // 基本结构校验
    if schema_value.get("type").is_none() || schema_value.get("id").is_none() {
        return Err(ErrorResponse::err(dynamic_ui_err::SCHEMA_MISSING_FIELD));
    }

    let schema_str = serde_json::to_string(&schema_value).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let description = if description.is_empty() {
        format!("根据指令\"{prompt}\"完成编辑")
    } else {
        description
    };

    tracing::info!(prompt_len = prompt.len(), "AI 编辑 DynamicUI Schema");
    Ok(EditSchemaNlResult { schema: schema_str, description })
}

// ── NL2UI 自然语言创建 ──

/// NL2UI 创建结果：生成的完整 Schema + 标题 + 描述
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSchemaNlResult {
    /// 生成的完整 Schema JSON 字符串
    pub schema: String,
    /// 推断出的页面标题
    pub title: String,
    /// 生成说明
    pub description: String,
}

/// NL2UI 创建系统提示词：约束 LLM 直接输出一个完整的 UI Schema JSON（含标题/描述包装）
const NL_GENERATE_SYSTEM_PROMPT: &str = r#"你是一个 UI Schema 生成器。你会收到一段自然语言描述，请生成一个完整的 UI Schema（JSON）。

UI Schema 节点结构（每个节点都是一个 JSON 对象）：
- version: 字符串，如 "1.0"
- id: 字符串，节点唯一标识
- type: 组件类型，必须是以下之一：
  Container, Row, Column, Grid, Card, Tabs, Accordion, Form, Input, Number,
  Select, DatePicker, Switch, Checkbox, Radio, Textarea, Table, Chart, List,
  Dashboard, CodeEditor, FilePreview, Markdown, Image, Button, Text, Divider,
  Progress, Tag, Tree, Timeline
- props: 对象，组件属性（如 Input 的 {name,label,placeholder?,required?}；Text 的 {content,strong?}；Form 的 {layout,submitText?}）
- children: 可选数组，子节点
- events: 可选数组，事件处理器，如 {trigger:"onSubmit", actions:[{type:"store", config:{}}]}
- dataSource / conditionalDisplay / style: 可选

生成规则：
1. 必须包含一个根节点（通常是 Container / Column / Card），并在合适位置内嵌一个 type 为 "Form" 的节点收纳输入字段（如需收集信息）。
2. Form 节点的 events 必须包含 {trigger:"onSubmit", actions:[{type:"store", config:{}}]}，以便提交时保存数据。
3. 根据描述推断合适的字段（姓名/邮箱/电话/标题/内容/日期/分类等），字段 name 使用英文 snake_case。
4. 顶部用 Text 节点展示页面标题（props.content）。
5. 只输出一个 JSON 对象，格式为：
{
  "title": "页面标题",
  "description": "一句话描述这个页面的用途",
  "schema": { /* 完整根 Schema JSON */ }
}
6. 不要输出任何解释性文字，不要用 Markdown 代码块（```）包裹最外层。"#;

/// 基于自然语言描述生成完整 UI Schema（缺陷 1 补齐：后端 AI 生成）
///
/// 优先调用首个启用的 LLM provider 进行生成；未配置 provider 或调用失败时返回错误，
/// 由前端 `nl2ui.ts` 降级为本地规则生成。
#[agent_command(domain = "dynamic_ui", safety = Caution, call_mode = StateInput, description = "AI生成动态UI Schema")]
#[tauri::command]
pub async fn generate_dynamic_ui_schema_nl(
    state: State<'_, AppState>,
    prompt: String,
) -> Result<GenerateSchemaNlResult, String> {
    if prompt.trim().is_empty() {
        return Err(ErrorResponse::err(dynamic_ui_err::GENERATE_PROMPT_EMPTY));
    }

    // 1. 获取 LLM bridge（首个启用的 provider）
    let master_key = state.harness.master_key_owned();
    let bridge = build_llm_bridge_from_db(&master_key)
        .await
        .ok_or_else(|| "未配置可用的 LLM provider，无法执行 AI 生成".to_string())?;

    // 2. 调用 LLM
    let response = bridge
        .call_llm(NL_GENERATE_SYSTEM_PROMPT, &prompt)
        .await
        .map_err(|e| format!("AI 生成调用失败: {e}"))?;

    // 3. 解析 LLM 输出（兼容 {schema,title,description} 包装或裸 schema）
    let cleaned = strip_code_fence(&response);
    let value: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| format!("AI 返回的 Schema 不是合法 JSON: {e}"))?;

    let (schema_value, title, description) = if let Some(s) = value.get("schema") {
        let t = value.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let d = value.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
        (s.clone(), t, d)
    } else {
        (value.clone(), String::new(), String::new())
    };

    // 基本结构校验
    if schema_value.get("type").is_none() || schema_value.get("id").is_none() {
        return Err(ErrorResponse::err(dynamic_ui_err::SCHEMA_MISSING_FIELD));
    }

    let schema_str = serde_json::to_string(&schema_value).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let title = if title.is_empty() {
        // 尝试从 schema 顶层 Text 节点推断标题
        schema_value
            .get("children")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|n| n.get("props"))
            .and_then(|p| p.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("动态UI")
            .to_string()
    } else {
        title
    };
    let description = if description.is_empty() {
        format!("由自然语言生成：{}", prompt.chars().take(50).collect::<String>())
    } else {
        description
    };

    tracing::info!(prompt_len = prompt.len(), "AI 生成 DynamicUI Schema");
    Ok(GenerateSchemaNlResult { schema: schema_str, title, description })
}

// ── 导航钉入配置（后端持久化，替代原 localStorage 方案） ──

/// 导航钉入配置 DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DynamicUIPinDTO {
    pub schema_id: String,
    pub title: String,
    pub group_name: String,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

fn pin_model_to_dto(model: PinModel) -> DynamicUIPinDTO {
    DynamicUIPinDTO {
        schema_id: model.schema_id,
        title: model.title,
        group_name: model.group_name,
        position: model.position,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

/// 列出所有导航钉入配置
#[agent_command(domain = "dynamic_ui", safety = Safe, call_mode = StateOnly, description = "列出导航钉入配置")]
#[tauri::command]
pub async fn list_dynamic_ui_pins(
    state: State<'_, AppState>,
) -> Result<Vec<DynamicUIPinDTO>, String> {
    let db = state.harness.db();
    let models = PinEntity::find()
        .order_by(PinColumn::GroupName, Order::Asc)
        .order_by(PinColumn::Position, Order::Asc)
        .all(db)
        .await
        .map_err(|e| format!("查询钉入列表失败: {e}"))?;
    Ok(models.into_iter().map(pin_model_to_dto).collect())
}

/// 钉入（upsert）一个动态页面到导航。
///
/// - `position` 未提供时，自动取该分组内当前最大排序位 + 1（追加到末尾）。
/// - 已存在同名 schema 的钉入时，覆盖其 title/group_name/position。
#[agent_command(domain = "dynamic_ui", safety = Caution, call_mode = StateInput, description = "钉入动态页面到导航")]
#[tauri::command]
pub async fn pin_dynamic_ui_schema(
    state: State<'_, AppState>,
    schema_id: String,
    title: String,
    group_name: String,
    position: Option<i32>,
) -> Result<DynamicUIPinDTO, String> {
    let db = state.harness.db();
    let now = now_iso();

    // 先查 existing，再算 position（避免 update 时 max 包含自身的 race，D-09）
    let existing = PinEntity::find_by_id(schema_id.clone())
        .one(db)
        .await
        .map_err(|e| format!("查询钉入失败: {e}"))?;

    let pos = match position {
        Some(p) => p,
        None => {
            if let Some(ref existing_pin) = existing {
                // 更新已有钉入且未传 position → 保留原有位置（D-09）
                existing_pin.position
            } else {
                let max = PinEntity::find()
                    .filter(PinColumn::GroupName.eq(&group_name))
                    .all(db)
                    .await
                    .map_err(|e| format!("查询钉入失败: {e}"))?
                    .into_iter()
                    .map(|m| m.position)
                    .max()
                    .unwrap_or(-1);
                max + 1
            }
        },
    };

    let model = if let Some(existing) = existing {
        let mut active: PinActiveModel = existing.into();
        active.title = Set(title);
        active.group_name = Set(group_name.clone());
        active.position = Set(pos);
        active.updated_at = Set(now);
        active.update(db).await.map_err(|e| format!("更新钉入失败: {e}"))?
    } else {
        let active = PinActiveModel {
            schema_id: Set(schema_id.clone()),
            title: Set(title),
            group_name: Set(group_name.clone()),
            position: Set(pos),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        };
        active.insert(db).await.map_err(|e| format!("钉入失败: {e}"))?
    };

    tracing::info!(schema_id = %schema_id, group_name = %group_name, position = pos, "钉入 DynamicUI 到导航");
    Ok(pin_model_to_dto(model))
}

/// 取消钉入（移除导航配置）
#[agent_command(domain = "dynamic_ui", safety = Dangerous, call_mode = StateInput, description = "取消导航钉入")]
#[tauri::command]
pub async fn unpin_dynamic_ui_schema(
    state: State<'_, AppState>,
    schema_id: String,
) -> Result<(), String> {
    let db = state.harness.db();
    PinEntity::delete_by_id(schema_id.clone())
        .exec(db)
        .await
        .map_err(|e| format!("取消钉入失败: {e}"))?;
    tracing::info!(schema_id = %schema_id, "取消钉入 DynamicUI");
    Ok(())
}
