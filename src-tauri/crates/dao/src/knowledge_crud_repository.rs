// SPDX-License-Identifier: AGPL-3.0-only

//! DAO implementations of Knowledge*Repository traits using SeaORM.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use uuid::Uuid;

use axagent_entities::{
    knowledge_documents, knowledge_entities, knowledge_flows, knowledge_interfaces,
};
use axagent_harness::repo_dtos::{
    CreateKnowledgeDocumentInput, CreateKnowledgeEntityInput, CreateKnowledgeFlowInput,
    CreateKnowledgeInterfaceInput, KnowledgeDocumentDto, KnowledgeEntityDto, KnowledgeFlowDto,
    KnowledgeInterfaceDto,
};
use axagent_harness::repositories::{
    KnowledgeDocumentRepository, KnowledgeEntityRepository, KnowledgeFlowRepository,
    KnowledgeInterfaceRepository,
};

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn gen_uuid() -> String {
    Uuid::new_v4().to_string()
}

// ── KnowledgeEntityRepository ───────────────────

pub struct DaoKnowledgeEntityRepository {
    db: DatabaseConnection,
}

impl DaoKnowledgeEntityRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl KnowledgeEntityRepository for DaoKnowledgeEntityRepository {
    async fn insert_entity(
        &self,
        input: CreateKnowledgeEntityInput,
    ) -> Result<KnowledgeEntityDto, String> {
        let id = gen_uuid();
        let now = now_ts();

        let am = knowledge_entities::ActiveModel {
            id: Set(id.clone()),
            knowledge_base_id: Set(input.knowledge_base_id.clone()),
            name: Set(input.name.clone()),
            entity_type: Set(input.entity_type.clone()),
            description: Set(input.description.clone()),
            source_path: Set(input.source_path.clone()),
            source_language: Set(input.source_language.clone()),
            properties: Set(input.properties.clone()),
            lifecycle: Set(input.lifecycle.clone()),
            behaviors: Set(input.behaviors.clone()),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        am.insert(&self.db).await.map_err(|e| format!("insert_entity: {}", e))?;

        Ok(KnowledgeEntityDto {
            id,
            knowledge_base_id: input.knowledge_base_id,
            name: input.name,
            entity_type: input.entity_type,
            description: input.description,
            source_path: input.source_path,
            source_language: input.source_language,
            properties: input.properties,
            lifecycle: input.lifecycle,
            behaviors: input.behaviors,
            metadata: None,
            created_at: now,
            updated_at: now,
        })
    }
}

// ── KnowledgeFlowRepository ─────────────────────

pub struct DaoKnowledgeFlowRepository {
    db: DatabaseConnection,
}

impl DaoKnowledgeFlowRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl KnowledgeFlowRepository for DaoKnowledgeFlowRepository {
    async fn insert_flow(
        &self,
        input: CreateKnowledgeFlowInput,
    ) -> Result<KnowledgeFlowDto, String> {
        let id = gen_uuid();
        let now = now_ts();

        let am = knowledge_flows::ActiveModel {
            id: Set(id.clone()),
            knowledge_base_id: Set(input.knowledge_base_id.clone()),
            name: Set(input.name.clone()),
            flow_type: Set(input.flow_type.clone()),
            description: Set(input.description.clone()),
            source_path: Set(input.source_path.clone()),
            steps: Set(input.steps.clone()),
            decision_points: Set(input.decision_points.clone()),
            error_handling: Set(input.error_handling.clone()),
            preconditions: Set(input.preconditions.clone()),
            postconditions: Set(input.postconditions.clone()),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        am.insert(&self.db).await.map_err(|e| format!("insert_flow: {}", e))?;

        Ok(KnowledgeFlowDto {
            id,
            knowledge_base_id: input.knowledge_base_id,
            name: input.name,
            flow_type: input.flow_type,
            description: input.description,
            source_path: input.source_path,
            steps: input.steps,
            decision_points: input.decision_points,
            error_handling: input.error_handling,
            preconditions: input.preconditions,
            postconditions: input.postconditions,
            metadata: None,
            created_at: now,
            updated_at: now,
        })
    }
}

// ── KnowledgeInterfaceRepository ────────────────

pub struct DaoKnowledgeInterfaceRepository {
    db: DatabaseConnection,
}

impl DaoKnowledgeInterfaceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl KnowledgeInterfaceRepository for DaoKnowledgeInterfaceRepository {
    async fn insert_interface(
        &self,
        input: CreateKnowledgeInterfaceInput,
    ) -> Result<KnowledgeInterfaceDto, String> {
        let id = gen_uuid();
        let now = now_ts();

        let am = knowledge_interfaces::ActiveModel {
            id: Set(id.clone()),
            knowledge_base_id: Set(input.knowledge_base_id.clone()),
            name: Set(input.name.clone()),
            interface_type: Set(input.interface_type.clone()),
            description: Set(input.description.clone()),
            source_path: Set(input.source_path.clone()),
            input_schema: Set(input.input_schema.clone()),
            output_schema: Set(input.output_schema.clone()),
            error_codes: Set(input.error_codes.clone()),
            communication_pattern: Set(input.communication_pattern.clone()),
            version: Set(None),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        am.insert(&self.db).await.map_err(|e| format!("insert_interface: {}", e))?;

        Ok(KnowledgeInterfaceDto {
            id,
            knowledge_base_id: input.knowledge_base_id,
            name: input.name,
            interface_type: input.interface_type,
            description: input.description,
            source_path: input.source_path,
            input_schema: input.input_schema,
            output_schema: input.output_schema,
            error_codes: input.error_codes,
            communication_pattern: input.communication_pattern,
            version: None,
            metadata: None,
            created_at: now,
            updated_at: now,
        })
    }
}

// ── KnowledgeDocumentRepository ─────────────────

pub struct DaoKnowledgeDocumentRepository {
    db: DatabaseConnection,
}

impl DaoKnowledgeDocumentRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl KnowledgeDocumentRepository for DaoKnowledgeDocumentRepository {
    async fn insert_document(
        &self,
        input: CreateKnowledgeDocumentInput,
    ) -> Result<KnowledgeDocumentDto, String> {
        let id = gen_uuid();
        let now = now_ts();

        let am = knowledge_documents::ActiveModel {
            id: Set(id.clone()),
            knowledge_base_id: Set(input.knowledge_base_id.clone()),
            title: Set(input.title.clone()),
            source_path: Set(input.source_path.clone()),
            mime_type: Set(input.mime_type.clone()),
            size_bytes: Set(input.size_bytes),
            indexing_status: Set("pending".to_string()),
            doc_type: Set(input.doc_type.clone()),
            index_error: Set(None),
            source_conversation_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        am.insert(&self.db).await.map_err(|e| format!("insert_document: {}", e))?;

        Ok(KnowledgeDocumentDto {
            id,
            knowledge_base_id: input.knowledge_base_id,
            title: input.title,
            source_path: input.source_path,
            mime_type: input.mime_type,
            size_bytes: input.size_bytes,
            indexing_status: "pending".to_string(),
            doc_type: input.doc_type,
            index_error: None,
            source_conversation_id: None,
            created_at: now,
            updated_at: now,
        })
    }
}
