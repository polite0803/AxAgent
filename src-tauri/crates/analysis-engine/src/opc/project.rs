// SPDX-License-Identifier: AGPL-3.0-only

//! 项目管理领域 — DTO 定义、trait 接口与 SeaORM 实现

use async_trait::async_trait;
use sea_orm::QuerySelect;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use axagent_entities::opc_projects;
use axagent_harness::util_fns::{gen_id, now_ts};

use super::error::{OpcError, OpcResult};

// ── DTO 定义 ──────────────────────────────────────────────────

/// 项目状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Planning,
    Active,
    Paused,
    Completed,
    Cancelled,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for ProjectStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planning" => Ok(Self::Planning),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("Unknown ProjectStatus: {s}")),
        }
    }
}

/// 项目里程碑
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub title: String,
    pub description: String,
    pub due_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub status: MilestoneStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

/// 项目 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub customer_id: Option<String>,
    pub title: String,
    pub description: String,
    pub status: ProjectStatus,
    pub milestones: Vec<Milestone>,
    pub budget: Option<f64>,
    pub currency: String,
    pub started_at: Option<i64>,
    pub deadline: Option<i64>,
    pub completed_at: Option<i64>,
    pub notes: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 创建项目请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectInput {
    pub customer_id: Option<String>,
    pub title: String,
    pub description: String,
    pub budget: Option<f64>,
    pub currency: String,
    pub deadline: Option<i64>,
    pub notes: String,
}

/// 更新项目请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub budget: Option<Option<f64>>,
    pub deadline: Option<Option<i64>>,
    pub notes: Option<String>,
}

/// 项目查询过滤
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectFilter {
    pub status: Option<ProjectStatus>,
    pub customer_id: Option<String>,
    pub search: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// ── Project Service Trait ─────────────────────────────────────────

#[async_trait]
pub trait ProjectService: Send + Sync {
    async fn create_project(&self, input: CreateProjectInput) -> OpcResult<Project>;
    async fn get_project(&self, id: &str) -> OpcResult<Project>;
    async fn list_projects(&self, filter: ProjectFilter) -> OpcResult<Vec<Project>>;
    async fn update_project(&self, id: &str, input: UpdateProjectInput) -> OpcResult<Project>;
    async fn delete_project(&self, id: &str) -> OpcResult<()>;
    async fn add_milestone(&self, project_id: &str, milestone: Milestone) -> OpcResult<Project>;
    async fn complete_milestone(&self, project_id: &str, milestone_id: &str) -> OpcResult<Project>;
}

/// Noop 实现
#[derive(Debug)]
pub struct NoopProjectService;

#[async_trait]
impl ProjectService for NoopProjectService {
    async fn create_project(&self, _input: CreateProjectInput) -> OpcResult<Project> {
        Err(OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn get_project(&self, _id: &str) -> OpcResult<Project> {
        Err(OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn list_projects(&self, _filter: ProjectFilter) -> OpcResult<Vec<Project>> {
        Ok(Vec::new())
    }
    async fn update_project(&self, _id: &str, _input: UpdateProjectInput) -> OpcResult<Project> {
        Err(OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn delete_project(&self, _id: &str) -> OpcResult<()> {
        Err(OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn add_milestone(&self, _project_id: &str, _milestone: Milestone) -> OpcResult<Project> {
        Err(OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn complete_milestone(
        &self,
        _project_id: &str,
        _milestone_id: &str,
    ) -> OpcResult<Project> {
        Err(OpcError::NotFound("ProjectService not implemented".into()))
    }
}

// ── SeaORM 实现 ───────────────────────────────────────────────────

/// 默认项目服务实现
pub struct DefaultProjectService {
    pub db: DatabaseConnection,
}

impl DefaultProjectService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

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
