// SPDX-License-Identifier: AGPL-3.0-only

//! 项目管理领域 — DTO 定义与 trait 接口

use serde::{Deserialize, Serialize};

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

impl std::str::FromStr for ProjectStatus {
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

use crate::OpcResult;

#[async_trait::async_trait]
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

#[async_trait::async_trait]
impl ProjectService for NoopProjectService {
    async fn create_project(&self, _input: CreateProjectInput) -> OpcResult<Project> {
        Err(crate::OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn get_project(&self, _id: &str) -> OpcResult<Project> {
        Err(crate::OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn list_projects(&self, _filter: ProjectFilter) -> OpcResult<Vec<Project>> {
        Ok(Vec::new())
    }
    async fn update_project(&self, _id: &str, _input: UpdateProjectInput) -> OpcResult<Project> {
        Err(crate::OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn delete_project(&self, _id: &str) -> OpcResult<()> {
        Err(crate::OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn add_milestone(&self, _project_id: &str, _milestone: Milestone) -> OpcResult<Project> {
        Err(crate::OpcError::NotFound("ProjectService not implemented".into()))
    }
    async fn complete_milestone(
        &self,
        _project_id: &str,
        _milestone_id: &str,
    ) -> OpcResult<Project> {
        Err(crate::OpcError::NotFound("ProjectService not implemented".into()))
    }
}
