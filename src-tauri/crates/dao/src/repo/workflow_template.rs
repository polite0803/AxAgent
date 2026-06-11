use sea_orm::*;
use sea_query::OnConflict;

use axagent_entities::workflow_template;
use axagent_entities::workflow_template_version;
use axagent_harness::core_error::Result;
use axagent_harness::workflow_types::{
    ErrorConfig, JsonSchema, RhaiToolDef, TriggerConfig, Variable, WorkflowEdge, WorkflowNode,
};

pub async fn list_workflow_templates(
    db: &DatabaseConnection,
    is_preset: Option<bool>,
) -> Result<Vec<workflow_template::Model>> {
    let mut query = workflow_template::Entity::find();

    if let Some(preset) = is_preset {
        query = query.filter(workflow_template::Column::IsPreset.eq(preset));
    }

    let templates = query
        .order_by(workflow_template::Column::UpdatedAt, Order::Desc)
        .all(db)
        .await?;
    Ok(templates)
}

pub async fn get_workflow_template(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<workflow_template::Model>> {
    let template = workflow_template::Entity::find_by_id(id).one(db).await?;
    Ok(template)
}

pub async fn insert_workflow_template(
    db: &DatabaseConnection,
    template: workflow_template::ActiveModel,
) -> Result<()> {
    template.clone().insert(db).await?;
    Ok(())
}

pub async fn upsert_workflow_template(
    db: &DatabaseConnection,
    template: workflow_template::ActiveModel,
) -> Result<()> {
    workflow_template::Entity::insert(template)
        .on_conflict(
            OnConflict::column(workflow_template::Column::Id)
                .update_column(workflow_template::Column::Name)
                .update_column(workflow_template::Column::Description)
                .update_column(workflow_template::Column::Icon)
                .update_column(workflow_template::Column::Tags)
                .update_column(workflow_template::Column::Version)
                .update_column(workflow_template::Column::IsPreset)
                .update_column(workflow_template::Column::IsEditable)
                .update_column(workflow_template::Column::IsPublic)
                .update_column(workflow_template::Column::TriggerConfig)
                .update_column(workflow_template::Column::Nodes)
                .update_column(workflow_template::Column::Edges)
                .update_column(workflow_template::Column::InputSchema)
                .update_column(workflow_template::Column::OutputSchema)
                .update_column(workflow_template::Column::Variables)
                .update_column(workflow_template::Column::ErrorConfig)
                .update_column(workflow_template::Column::UpdatedAt)
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_workflow_template(
    db: &DatabaseConnection,
    id: &str,
    name: String,
    description: Option<String>,
    icon: String,
    tags: Vec<String>,
    trigger_config: Option<TriggerConfig>,
    nodes: Vec<WorkflowNode>,
    edges: Vec<WorkflowEdge>,
    input_schema: Option<JsonSchema>,
    output_schema: Option<JsonSchema>,
    variables: Vec<Variable>,
    error_config: Option<ErrorConfig>,
    tool_defs: Option<Vec<RhaiToolDef>>,
) -> Result<bool> {
    let template = workflow_template::Entity::find_by_id(id).one(db).await?;

    if let Some(t) = template {
        // D9: save old version as a snapshot before updating
        let version_snapshot = workflow_template_version::ActiveModel {
            id: Set(format!("{}_v{}", t.id, t.version)),
            template_id: Set(t.id.clone()),
            name: Set(t.name.clone()),
            description: Set(t.description.clone()),
            icon: Set(t.icon.clone()),
            tags: Set(t.tags.clone()),
            version: Set(t.version),
            is_preset: Set(t.is_preset),
            is_editable: Set(t.is_editable),
            is_public: Set(t.is_public),
            trigger_config: Set(t.trigger_config.clone()),
            nodes: Set(t.nodes.clone()),
            edges: Set(t.edges.clone()),
            input_schema: Set(t.input_schema.clone()),
            output_schema: Set(t.output_schema.clone()),
            variables: Set(t.variables.clone()),
            error_config: Set(t.error_config.clone()),
            created_at: Set(chrono::Utc::now().timestamp_millis()),
        };
        version_snapshot.insert(db).await?;

        let mut active_model: workflow_template::ActiveModel = t.clone().into();
        active_model.name = Set(name);
        active_model.description = Set(description);
        active_model.icon = Set(icon);
        active_model.tags = Set(Some(serde_json::to_string(&tags).unwrap_or_default()));
        active_model.trigger_config =
            Set(trigger_config.and_then(|c| serde_json::to_string(&c).ok()));
        active_model.nodes = Set(serde_json::to_string(&nodes).unwrap_or_default());
        active_model.edges = Set(serde_json::to_string(&edges).unwrap_or_default());
        active_model.input_schema = Set(input_schema.and_then(|s| serde_json::to_string(&s).ok()));
        active_model.output_schema =
            Set(output_schema.and_then(|s| serde_json::to_string(&s).ok()));
        active_model.variables = Set(Some(serde_json::to_string(&variables).unwrap_or_default()));
        active_model.error_config = Set(error_config.and_then(|e| serde_json::to_string(&e).ok()));
        active_model.tool_defs = Set(tool_defs
            .as_ref()
            .map(|tds| serde_json::to_string(tds).unwrap_or_default()));
        active_model.version = Set(t.version + 1);
        active_model.updated_at = Set(chrono::Utc::now().timestamp_millis());
        active_model.update(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn delete_workflow_template(db: &DatabaseConnection, id: &str) -> Result<bool> {
    let template = workflow_template::Entity::find_by_id(id).one(db).await?;
    if let Some(t) = template {
        t.delete(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn count_workflow_templates(db: &DatabaseConnection) -> Result<i64> {
    let count = workflow_template::Entity::find().count(db).await?;
    Ok(count as i64)
}

pub async fn get_template_versions(db: &DatabaseConnection, id: &str) -> Result<Vec<i32>> {
    let template = workflow_template::Entity::find_by_id(id).one(db).await?;
    let current_version = template.as_ref().map(|t| t.version);

    // Query version history table for all previous versions
    let mut versions: Vec<i32> = workflow_template_version::Entity::find()
        .filter(workflow_template_version::Column::TemplateId.eq(id))
        .all(db)
        .await?
        .iter()
        .map(|v| v.version)
        .collect();

    if let Some(current) = current_version
        && !versions.contains(&current)
    {
        versions.push(current);
    }
    versions.sort_by(|a, b| b.cmp(a));
    Ok(versions)
}

pub async fn get_template_by_version(
    db: &DatabaseConnection,
    id: &str,
    version: i32,
) -> Result<Option<workflow_template::Model>> {
    // 1. 先查主表（当前版本，快速路径）
    let template = workflow_template::Entity::find_by_id(id).one(db).await?;
    if let Some(ref t) = template
        && t.version == version
    {
        return Ok(Some(t.clone()));
    }

    // 2. 版本不匹配，查历史表 workflow_template_versions
    use axagent_entities::workflow_template_version;
    let history = workflow_template_version::Entity::find()
        .filter(workflow_template_version::Column::TemplateId.eq(id))
        .filter(workflow_template_version::Column::Version.eq(version))
        .one(db)
        .await?;

    Ok(history.map(|v| workflow_template::Model {
        id: v.template_id,
        name: v.name,
        description: v.description,
        icon: v.icon,
        tags: v.tags,
        version: v.version,
        is_preset: v.is_preset,
        is_editable: v.is_editable,
        is_public: v.is_public,
        trigger_config: v.trigger_config,
        nodes: v.nodes,
        edges: v.edges,
        input_schema: v.input_schema,
        output_schema: v.output_schema,
        variables: v.variables,
        error_config: v.error_config,
        composite_source: None,
        tool_defs: None,
        created_at: v.created_at,
        updated_at: v.created_at,
    }))
}

pub async fn get_workflow_by_composite_source(
    db: &DatabaseConnection,
    composite_source: &str,
) -> Result<Option<workflow_template::Model>> {
    let template = workflow_template::Entity::find()
        .filter(workflow_template::Column::CompositeSource.eq(composite_source))
        .one(db)
        .await?;
    Ok(template)
}

/// 把 kit 侧预生成的 `WorkflowTemplateData` 列表（来自 preset）插入数据库：
/// - 已存在但 `nodes` 为空的行 → upsert 覆盖
/// - 已存在且非空 → 跳过
/// - 不存在 → insert
/// 替代 `kit::preset_templates::seed_preset_templates` 直接构造 `ActiveModel` 的位置，
/// 让 kit 不再依赖 `axagent_entities` 的 `ActiveModel` 类型。
pub async fn seed_preset_templates(
    db: &DatabaseConnection,
    items: Vec<axagent_harness::workflow_types::WorkflowTemplateData>,
) -> Result<()> {
    for item in items {
        let existing = get_workflow_template(db, &item.id).await?;

        match existing {
            None => {
                let active_model = build_active_model_from_data(&item);
                insert_workflow_template(db, active_model).await?;
            },
            Some(ref t) if t.nodes == "[]" || t.nodes.is_empty() => {
                let active_model = build_active_model_from_data(&item);
                upsert_workflow_template(db, active_model).await?;
            },
            _ => {},
        }
    }
    Ok(())
}

fn build_active_model_from_data(
    item: &axagent_harness::workflow_types::WorkflowTemplateData,
) -> workflow_template::ActiveModel {
    workflow_template::ActiveModel {
        id: Set(item.id.clone()),
        name: Set(item.name.clone()),
        description: Set(item.description.clone()),
        icon: Set(item.icon.clone()),
        tags: Set(Some(serde_json::to_string(&item.tags).unwrap_or_default())),
        version: Set(item.version),
        is_preset: Set(item.is_preset),
        is_editable: Set(item.is_editable),
        is_public: Set(item.is_public),
        trigger_config: Set(item
            .trigger_config
            .as_ref()
            .and_then(|c| serde_json::to_string(c).ok())),
        nodes: Set(serde_json::to_string(&item.nodes).unwrap_or_default()),
        edges: Set(serde_json::to_string(&item.edges).unwrap_or_default()),
        input_schema: Set(item
            .input_schema
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok())),
        output_schema: Set(item
            .output_schema
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok())),
        variables: Set(Some(serde_json::to_string(&item.variables).unwrap_or_default())),
        error_config: Set(item
            .error_config
            .as_ref()
            .and_then(|e| serde_json::to_string(e).ok())),
        composite_source: Set(None),
        tool_defs: Set(None),
        created_at: Set(item.created_at),
        updated_at: Set(item.updated_at),
    }
}

/// 把 `workflow_template::Model` 转换为 `WorkflowTemplateData` DTO。
/// 上层（kit / gateway）使用此 DTO 而不是直接拿 SeaORM Model。
pub fn template_model_to_data(
    model: &workflow_template::Model,
) -> axagent_harness::workflow_types::WorkflowTemplateData {
    use axagent_harness::workflow_types::*;
    WorkflowTemplateData {
        id: model.id.clone(),
        name: model.name.clone(),
        description: model.description.clone(),
        icon: model.icon.clone(),
        tags: model
            .tags
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        version: model.version,
        is_preset: model.is_preset,
        is_editable: model.is_editable,
        is_public: model.is_public,
        trigger_config: model
            .trigger_config
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        nodes: serde_json::from_str(&model.nodes).unwrap_or_default(),
        edges: serde_json::from_str(&model.edges).unwrap_or_default(),
        input_schema: model
            .input_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        output_schema: model
            .output_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        variables: model
            .variables
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        error_config: model
            .error_config
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        tool_defs: model
            .tool_defs
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

/// 把 `workflow_template_version::Model` 转换为 `WorkflowTemplateVersionData` DTO。
pub fn template_version_model_to_data(
    model: &workflow_template_version::Model,
) -> axagent_harness::workflow_types::WorkflowTemplateVersionData {
    use axagent_harness::workflow_types::*;
    WorkflowTemplateVersionData {
        template_id: model.template_id.clone(),
        name: model.name.clone(),
        description: model.description.clone(),
        icon: model.icon.clone(),
        tags: model
            .tags
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        version: model.version,
        is_preset: model.is_preset,
        is_editable: model.is_editable,
        is_public: model.is_public,
        trigger_config: model
            .trigger_config
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        nodes: serde_json::from_str(&model.nodes).unwrap_or_default(),
        edges: serde_json::from_str(&model.edges).unwrap_or_default(),
        input_schema: model
            .input_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        output_schema: model
            .output_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        variables: model
            .variables
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        error_config: model
            .error_config
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        created_at: model.created_at,
    }
}
