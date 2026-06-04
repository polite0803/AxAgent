//! Plan 模式的数据类型定义。
//!
//! 纯数据 DTO 层，无业务逻辑。供 `axagent-agent::hierarchical_planner`
//! 和 `axagent-rt-workflow::agent_executor` 共享。

use serde::{Deserialize, Serialize};

// ── 核心数据类型 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub goal: String,
    pub phases: Vec<Phase>,
    pub status: PlanStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tasks: Vec<PlannedTask>,
    pub dependencies: Vec<String>,
    pub status: PhaseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTask {
    pub id: String,
    pub description: String,
    pub action_type: String,
    pub parameters: serde_json::Value,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub assigned_role: Option<String>,
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
pub struct PhaseProgress {
    pub name: String,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
}
