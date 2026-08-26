// SPDX-License-Identifier: AGPL-3.0-only

//! Plan 模式的数据类型定义。
//!
//! 纯数据 DTO 层，无业务逻辑。供 `axagent-agent::hierarchical_planner`
//! 和 `axagent-rt-workflow::agent_executor` 共享。

use crate::workflow_types::CompensationConfig;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// ── 核心数据类型 ────────────────────────────────────────────

/// 计划任务的动作类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    /// 工具调用
    Tool,
    /// LLM 推理
    Llm,
    /// Agent 执行
    Agent,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Llm => "llm",
            Self::Agent => "agent",
        }
    }
}

impl FromStr for ActionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tool" => Ok(Self::Tool),
            "llm" => Ok(Self::Llm),
            "agent" => Ok(Self::Agent),
            other => Err(format!("Unknown action type: {other}")),
        }
    }
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: String,
    pub goal: String,
    pub phases: Vec<Phase>,
    pub status: PlanStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tasks: Vec<PlannedTask>,
    pub dependencies: Vec<String>,
    pub status: PhaseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedTask {
    pub id: String,
    pub description: String,
    pub action_type: ActionType,
    pub parameters: serde_json::Value,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub assigned_role: Option<String>,
    /// 失败补偿策略（None = WorkflowNode.base().compensation）
    #[serde(default)]
    pub compensation: Option<CompensationConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Draft,
    Executing,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanVersion {
    pub version: u32,
    pub plan: Plan,
    pub created_at: i64,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplanReason {
    StepFailed { task_id: String, error: String },
    NewDependencyDiscovered { task_id: String, dependency: String },
    GoalChanged { old_goal: String, new_goal: String },
    ResourceConstraint { constraint: String },
    ManualIntervention { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplanAction {
    Retry {
        task_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modified_parameters: Option<serde_json::Value>,
    },
    Skip {
        task_id: String,
        reason: String,
    },
    Insert {
        phase_id: String,
        task: PlannedTask,
        position: usize,
    },
    Remove {
        task_id: String,
        reason: String,
    },
    Reorder {
        task_id: String,
        new_position: usize,
    },
    AddPhase {
        phase: Phase,
        position: usize,
    },
    ModifyTask {
        task_id: String,
        modifications: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanProgress {
    pub total_phases: usize,
    pub completed_phases: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub in_progress_tasks: usize,
    pub pending_tasks: usize,
    pub percentage: f64,
    pub phase_progress: Vec<PhaseProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseProgress {
    pub name: String,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
}
