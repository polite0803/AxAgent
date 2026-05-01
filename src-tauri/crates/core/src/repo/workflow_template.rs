use sea_orm::*;
use sea_query::OnConflict;

use crate::entity::workflow_template;
use crate::entity::workflow_template_version;
use crate::error::Result;
use crate::workflow_types::{
    ErrorConfig, JsonSchema, TriggerConfig, Variable, WorkflowEdge, WorkflowNode,
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

    if let Some(current) = current_version {
        if !versions.contains(&current) {
            versions.push(current);
        }
    }
    versions.sort_by(|a, b| b.cmp(a));
    Ok(versions)
}

pub async fn get_template_by_version(
    db: &DatabaseConnection,
    id: &str,
    version: i32,
) -> Result<Option<workflow_template::Model>> {
    let template = workflow_template::Entity::find_by_id(id).one(db).await?;
    match template {
        Some(t) if t.version == version => Ok(Some(t)),
        _ => Ok(None),
    }
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
