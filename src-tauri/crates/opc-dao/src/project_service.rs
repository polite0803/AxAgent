// SPDX-License-Identifier: AGPL-3.0-only

//! 项目服务实现 — SeaORM CRUD + 里程碑管理

use async_trait::async_trait;
use sea_orm::QuerySelect;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::str::FromStr;

use axagent_harness::util_fns::{gen_id, now_ts};
use axagent_opc_entities::opc_projects;
use axagent_opc_types::{
    CreateProjectInput, Milestone, MilestoneStatus, OpcError, OpcResult, Project, ProjectFilter,
    ProjectService, ProjectStatus, UpdateProjectInput,
};
// use tracing;

/// 默认项目服务实现
pub struct DefaultProjectService {
    pub db: DatabaseConnection,
}

impl DefaultProjectService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Entity ↔ DTO 转换 ─────────────────────────────────────────────

fn entity_to_dto(e: opc_projects::Model) -> OpcResult<Project> {
    let milestones: Vec<Milestone> = serde_json::from_str(&e.milestones_json).unwrap_or_default();

    let status = ProjectStatus::from_str(&e.status).unwrap_or(ProjectStatus::Planning);

    Ok(Project {
        id: e.id,
        customer_id: e.customer_id,
        title: e.title,
        description: e.description,
        status,
        milestones,
        budget: e.budget,
        currency: e.currency,
        started_at: e.started_at,
        deadline: e.deadline,
        completed_at: e.completed_at,
        notes: e.notes,
        created_at: e.created_at,
        updated_at: e.updated_at,
    })
}

// ── Service 实现 ───────────────────────────────────────────────────

#[async_trait]
impl ProjectService for DefaultProjectService {
    async fn create_project(&self, input: CreateProjectInput) -> OpcResult<Project> {
        let id = gen_id();
        let now = now_ts();

        opc_projects::ActiveModel {
            id: Set(id.clone()),
            customer_id: Set(input.customer_id),
            title: Set(input.title),
            description: Set(input.description),
            status: Set(ProjectStatus::Planning.as_str().to_string()),
            milestones_json: Set("[]".to_string()),
            budget: Set(input.budget),
            currency: Set(input.currency),
            started_at: Set(None),
            deadline: Set(input.deadline),
            completed_at: Set(None),
            notes: Set(input.notes),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await
        .map_err(|e| OpcError::Database(e.to_string()))?;

        self.get_project(&id).await
    }

    async fn get_project(&self, id: &str) -> OpcResult<Project> {
        let entity = opc_projects::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("project {id}")))?;

        entity_to_dto(entity)
    }

    async fn list_projects(&self, filter: ProjectFilter) -> OpcResult<Vec<Project>> {
        let mut query = opc_projects::Entity::find().order_by_desc(opc_projects::Column::CreatedAt);

        if let Some(status) = &filter.status {
            query = query.filter(opc_projects::Column::Status.eq(status.as_str()));
        }
        if let Some(cid) = &filter.customer_id {
            query = query.filter(opc_projects::Column::CustomerId.eq(cid));
        }
        if let Some(search) = &filter.search {
            query = query.filter(opc_projects::Column::Title.contains(search));
        }
        if let Some(limit) = filter.limit {
            query = query.limit(limit as u64);
        }
        if let Some(offset) = filter.offset {
            query = query.offset(offset as u64);
        }

        let entities = query.all(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        entities.into_iter().map(entity_to_dto).collect()
    }

    async fn update_project(&self, id: &str, input: UpdateProjectInput) -> OpcResult<Project> {
        let entity = opc_projects::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("project {id}")))?;

        let mut am: opc_projects::ActiveModel = entity.into();
        am.updated_at = Set(now_ts());

        if let Some(title) = input.title {
            am.title = Set(title);
        }
        if let Some(desc) = input.description {
            am.description = Set(desc);
        }
        if let Some(budget) = input.budget {
            am.budget = Set(budget);
        }
        if let Some(deadline) = input.deadline {
            am.deadline = Set(deadline);
        }
        if let Some(notes) = input.notes {
            am.notes = Set(notes);
        }

        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        self.get_project(id).await
    }

    async fn delete_project(&self, id: &str) -> OpcResult<()> {
        let result = opc_projects::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(OpcError::NotFound(format!("project {id}")));
        }
        Ok(())
    }

    async fn add_milestone(&self, project_id: &str, milestone: Milestone) -> OpcResult<Project> {
        let entity = opc_projects::Entity::find_by_id(project_id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("project {project_id}")))?;

        let mut milestones: Vec<Milestone> = serde_json::from_str(&entity.milestones_json)
            .map_err(|e| OpcError::Database(e.to_string()))?;

        milestones.push(milestone);

        let mut am: opc_projects::ActiveModel = entity.into();
        am.milestones_json =
            Set(serde_json::to_string(&milestones)
                .map_err(|e| OpcError::Database(e.to_string()))?);
        am.updated_at = Set(now_ts());

        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        self.get_project(project_id).await
    }

    async fn complete_milestone(&self, project_id: &str, milestone_id: &str) -> OpcResult<Project> {
        let entity = opc_projects::Entity::find_by_id(project_id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("project {project_id}")))?;

        let mut milestones: Vec<Milestone> = serde_json::from_str(&entity.milestones_json)
            .map_err(|e| OpcError::Database(e.to_string()))?;

        let now = now_ts();
        let mut found = false;
        for ms in &mut milestones {
            if ms.id == milestone_id {
                ms.status = MilestoneStatus::Completed;
                ms.completed_at = Some(now);
                found = true;
                break;
            }
        }

        if !found {
            return Err(OpcError::NotFound(format!("milestone {milestone_id}")));
        }

        let mut am: opc_projects::ActiveModel = entity.into();
        am.milestones_json =
            Set(serde_json::to_string(&milestones)
                .map_err(|e| OpcError::Database(e.to_string()))?);
        am.updated_at = Set(now_ts());

        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        self.get_project(project_id).await
    }
}
