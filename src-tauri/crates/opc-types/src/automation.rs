// SPDX-License-Identifier: AGPL-3.0-only

//! 业务自动化领域 — DTO 定义与 trait 接口

use serde::{Deserialize, Serialize};

/// 自动化规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: String,
    pub name: String,
    pub trigger_type: String,
    pub trigger_config: String,
    pub action_type: String,
    pub action_config: String,
    pub enabled: bool,
    pub last_run_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAutomationRuleInput {
    pub name: String,
    pub trigger_type: String,
    pub trigger_config: String,
    pub action_type: String,
    pub action_config: String,
}

/// 跟进任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpTask {
    pub id: String,
    pub task_type: String,
    pub title: String,
    pub description: String,
    pub status: FollowUpStatus,
    pub priority: FollowUpPriority,
    pub due_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FollowUpStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl FollowUpStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for FollowUpStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("Unknown FollowUpStatus: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FollowUpPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl FollowUpPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

impl std::str::FromStr for FollowUpPriority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "urgent" => Ok(Self::Urgent),
            _ => Err(format!("Unknown FollowUpPriority: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFollowUpTaskInput {
    pub task_type: String,
    pub title: String,
    pub description: String,
    pub priority: FollowUpPriority,
    pub due_at: Option<i64>,
}

use crate::OpcResult;

#[async_trait::async_trait]
pub trait AutomationService: Send + Sync {
    // Automation rules
    async fn create_rule(&self, input: CreateAutomationRuleInput) -> OpcResult<AutomationRule>;
    async fn list_rules(&self) -> OpcResult<Vec<AutomationRule>>;
    async fn toggle_rule(&self, id: &str, enabled: bool) -> OpcResult<AutomationRule>;

    // Follow-up tasks
    async fn create_follow_up(&self, input: CreateFollowUpTaskInput) -> OpcResult<FollowUpTask>;
    async fn list_follow_ups(&self, status: Option<FollowUpStatus>)
    -> OpcResult<Vec<FollowUpTask>>;
    async fn complete_follow_up(&self, id: &str) -> OpcResult<FollowUpTask>;
}

#[derive(Debug)]
pub struct NoopAutomationService;

#[async_trait::async_trait]
impl AutomationService for NoopAutomationService {
    async fn create_rule(&self, _: CreateAutomationRuleInput) -> OpcResult<AutomationRule> {
        Err(crate::OpcError::NotFound("AutomationService not implemented".into()))
    }
    async fn list_rules(&self) -> OpcResult<Vec<AutomationRule>> {
        Ok(Vec::new())
    }
    async fn toggle_rule(&self, _: &str, _: bool) -> OpcResult<AutomationRule> {
        Err(crate::OpcError::NotFound("AutomationService not implemented".into()))
    }
    async fn create_follow_up(&self, _: CreateFollowUpTaskInput) -> OpcResult<FollowUpTask> {
        Err(crate::OpcError::NotFound("AutomationService not implemented".into()))
    }
    async fn list_follow_ups(&self, _: Option<FollowUpStatus>) -> OpcResult<Vec<FollowUpTask>> {
        Ok(Vec::new())
    }
    async fn complete_follow_up(&self, _: &str) -> OpcResult<FollowUpTask> {
        Err(crate::OpcError::NotFound("AutomationService not implemented".into()))
    }
}
