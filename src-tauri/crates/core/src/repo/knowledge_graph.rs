use sea_orm::*;

use crate::entity::{
    knowledge_attributes, knowledge_entities, knowledge_flows, knowledge_interfaces,
    knowledge_relations,
};
use crate::error::{AxAgentError, Result};
use crate::types::{
    CreateKnowledgeAttributeInput, CreateKnowledgeEntityInput, CreateKnowledgeFlowInput,
    CreateKnowledgeInterfaceInput, CreateKnowledgeRelationInput, KnowledgeAttribute,
    KnowledgeEntity, KnowledgeFlow, KnowledgeInterface, KnowledgeRelation,
};
use crate::utils::gen_id;

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
    let models = knowledge_entities::Entity::find()
        .filter(knowledge_entities::Column::KnowledgeBaseId.eq(base_id))
        .order_by_asc(knowledge_entities::Column::Name)
        .all(db)
        .await?;

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
