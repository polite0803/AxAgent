use sea_orm::*;

use crate::entity::prompt_template;
use crate::entity::prompt_template_version;
use crate::error::{AxAgentError, Result};
use crate::types::{
    CreatePromptTemplateInput, PromptTemplate, PromptTemplateVersion, UpdatePromptTemplateInput,
};
use crate::utils::gen_id;

pub async fn list_prompt_templates(db: &DatabaseConnection) -> Result<Vec<PromptTemplate>> {
    let templates = prompt_template::Entity::find()
        .order_by(prompt_template::Column::UpdatedAt, Order::Desc)
        .all(db)
        .await?;

    Ok(templates.into_iter().map(model_to_template).collect())
}

pub async fn get_prompt_template(db: &DatabaseConnection, id: &str) -> Result<PromptTemplate> {
    let template = prompt_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PromptTemplate {}", id)))?;

    Ok(model_to_template(template))
}

pub async fn create_prompt_template(
    db: &DatabaseConnection,
    input: CreatePromptTemplateInput,
) -> Result<PromptTemplate> {
    let now = chrono::Utc::now().timestamp_millis();
    let id = gen_id();

    let active_model = prompt_template::ActiveModel {
        id: Set(id),
        name: Set(input.name),
        description: Set(input.description),
        content: Set(input.content),
        variables_schema: Set(input.variables_schema),
        version: Set(1),
        is_active: Set(true),
        ab_test_enabled: Set(false),
        ab_test_variant: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let model = active_model.insert(db).await?;

    Ok(model_to_template(model))
}

pub async fn update_prompt_template(
    db: &DatabaseConnection,
    id: &str,
    input: UpdatePromptTemplateInput,
) -> Result<PromptTemplate> {
    let template = prompt_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PromptTemplate {}", id)))?;

    let old_version = template.version;
    let new_version = if input.content.is_some() || input.variables_schema.is_some() {
        old_version + 1
    } else {
        old_version
    };

    if input.content.is_some() || input.variables_schema.is_some() {
        let version_snapshot = prompt_template_version::ActiveModel {
            id: Set(format!("{}_v{}", id, old_version)),
            template_id: Set(id.to_string()),
            version: Set(old_version),
            name: Set(template.name.clone()),
            description: Set(template.description.clone()),
            content: Set(template.content.clone()),
            variables_schema: Set(template.variables_schema.clone()),
            changelog: Set(Some(format!("Updated before version {}", new_version))),
            created_at: Set(template.updated_at),
        };
        version_snapshot.insert(db).await?;
    }

    let mut active_model: prompt_template::ActiveModel = template.into();
    if let Some(name) = input.name {
        active_model.name = Set(name);
    }
    if let Some(description) = input.description {
        active_model.description = Set(Some(description));
    }
    if let Some(content) = input.content {
        active_model.content = Set(content);
    }
    if let Some(variables_schema) = input.variables_schema {
        active_model.variables_schema = Set(Some(variables_schema));
    }
    if let Some(is_active) = input.is_active {
        active_model.is_active = Set(is_active);
    }
    if let Some(ab_test_enabled) = input.ab_test_enabled {
        active_model.ab_test_enabled = Set(ab_test_enabled);
    }
    active_model.version = Set(new_version);
    active_model.updated_at = Set(chrono::Utc::now().timestamp_millis());

    let model = active_model.update(db).await?;

    Ok(model_to_template(model))
}

pub async fn delete_prompt_template(db: &DatabaseConnection, id: &str) -> Result<()> {
    let template = prompt_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("PromptTemplate {}", id)))?;

    prompt_template_version::Entity::delete_many()
        .filter(prompt_template_version::Column::TemplateId.eq(id))
        .exec(db)
        .await?;

    template.delete(db).await?;

    Ok(())
}

pub async fn get_prompt_template_versions(
    db: &DatabaseConnection,
    template_id: &str,
) -> Result<Vec<PromptTemplateVersion>> {
    let versions = prompt_template_version::Entity::find()
        .filter(prompt_template_version::Column::TemplateId.eq(template_id))
        .order_by(prompt_template_version::Column::Version, Order::Desc)
        .all(db)
        .await?;

    Ok(versions.into_iter().map(model_to_version).collect())
}

fn model_to_template(m: prompt_template::Model) -> PromptTemplate {
    PromptTemplate {
        id: m.id,
        name: m.name,
        description: m.description,
        content: m.content,
        variables_schema: m.variables_schema,
        version: m.version,
        is_active: m.is_active,
        ab_test_enabled: m.ab_test_enabled,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn model_to_version(m: prompt_template_version::Model) -> PromptTemplateVersion {
    PromptTemplateVersion {
        id: m.id,
        template_id: m.template_id,
        version: m.version,
        content: m.content,
        variables_schema: m.variables_schema,
        changelog: m.changelog,
        created_at: m.created_at,
    }
}
