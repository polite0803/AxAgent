// SPDX-License-Identifier: AGPL-3.0-only

//! Core types for the Orchestrator system.
//!
//! Defines task decomposition, worker assignment, structured handover,
//! and orchestration strategy types used across the orchestrator module.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use axagent_harness::workflow_types::AgentRole;
use axagent_harness::workflow_types::ToolDef;

// ── Orchestration Strategy ──────────────────────────────────────────

/// Strategy for how the orchestrator decomposes and schedules tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationStrategy {
    /// Serial execution, one after another. For tasks with strict ordering.
    #[default]
    Ordered,
    /// Parallel fan-out. All independent subtasks run concurrently.
    FanOut,
    /// Intra-stage parallel, inter-stage serial. E.g., dev pipeline.
    Pipeline,
    /// Race — take the first completed result. For solution exploration.
    Race,
    /// Debate — multiple agents argue then adjudicate. For architectural decisions.
    Debate,
    /// Dynamic — LLM determines topology at runtime. For unstructured tasks.
    Dynamic,
}

impl OrchestrationStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ordered => "ordered",
            Self::FanOut => "fan_out",
            Self::Pipeline => "pipeline",
            Self::Race => "race",
            Self::Debate => "debate",
            Self::Dynamic => "dynamic",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "ordered" => Some(Self::Ordered),
            "fan_out" => Some(Self::FanOut),
            "pipeline" => Some(Self::Pipeline),
            "race" => Some(Self::Race),
            "debate" => Some(Self::Debate),
            "dynamic" => Some(Self::Dynamic),
            _ => None,
        }
    }
}

// ── SubTask ───────────────────────────────────────────────────────────

/// Status of a single sub-task within an orchestration plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubTaskStatus {
    /// Waiting for dependencies to complete.
    Pending,
    /// Ready to be dispatched.
    Ready,
    /// Currently executing (assigned to a Worker).
    Running,
    /// Completed successfully.
    Completed,
    /// Failed — may trigger replanning.
    Failed,
    /// Skipped due to upstream failure or conditional exclusion.
    Skipped,
}

impl std::fmt::Display for SubTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Ready => write!(f, "Ready"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Skipped => write!(f, "Skipped"),
        }
    }
}

/// A single sub-task decomposed from the high-level mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// Unique identifier within this orchestration run.
    pub id: String,
    /// Human-readable task name.
    pub name: String,
    /// Detailed task description for the worker Agent.
    pub description: String,
    /// The Agent role best suited for this task.
    pub role: AgentRole,
    /// IDs of sub-tasks that must complete before this one starts.
    pub dependencies: Vec<String>,
    /// Current execution status.
    pub status: SubTaskStatus,
    /// System prompt override for the worker Agent.
    pub system_prompt: Option<String>,
    /// Expected output variable name in the execution context.
    pub output_var: String,
    /// Error message if status is Failed.
    pub error: Option<String>,
    /// Number of retry attempts.
    pub attempts: u32,
    /// Maximum allowed retries before marking as Failed.
    pub max_retries: u32,
    /// Tools available to this sub-task's Agent node.
    /// Populated by the decomposer or caller. Empty = no tool access.
    #[serde(default)]
    pub tools: Vec<ToolDef>,
}

impl SubTask {
    pub fn new(id: String, name: String, description: String, role: AgentRole) -> Self {
        Self {
            id,
            name,
            description,
            role,
            dependencies: Vec::new(),
            status: SubTaskStatus::Pending,
            system_prompt: None,
            output_var: format!(
                "subtask_output_{}",
                uuid::Uuid::new_v4().to_string().replace('-', "_")
            ),
            error: None,
            attempts: 0,
            max_retries: 3,
            tools: Vec::new(),
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }
}

// ── WorkerAssignment ───────────────────────────────────────────────────

/// Assignment of a sub-task to a specific worker Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerAssignment {
    /// The sub-task being assigned.
    pub sub_task_id: String,
    /// The worker node ID in the generated subgraph.
    pub worker_node_id: String,
    /// The Agent role assigned.
    pub role: AgentRole,
    /// The generated system prompt for this worker.
    pub system_prompt: String,
}

// ── StructuredHandover ─────────────────────────────────────────────────

/// Structured handover protocol between orchestrated Agents.
///
/// Six required fields as defined in the multi-agent design. A
/// Scorecard check verifies all fields are present after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredHandover {
    /// Summary of what the Agent completed.
    pub completed_work: String,
    /// List of files changed with summary per file.
    pub changes: Vec<ChangeRecord>,
    /// What the next Agent should do with this output.
    pub next_steps: String,
    /// Known issues or concerns that remain unresolved.
    pub remaining_issues: String,
    /// Upstream dependencies the next Agent needs.
    pub dependencies: String,
    /// Evidence that work was validated (test results, etc.).
    pub validation_evidence: String,
}

impl StructuredHandover {
    /// Returns true if all six required fields are non-empty.
    pub fn is_complete(&self) -> bool {
        !self.completed_work.is_empty()
            && !self.changes.is_empty()
            && !self.next_steps.is_empty()
            && !self.remaining_issues.is_empty()
            && !self.dependencies.is_empty()
            && !self.validation_evidence.is_empty()
    }

    /// Returns the list of missing field names.
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.completed_work.is_empty() {
            missing.push("completed_work");
        }
        if self.changes.is_empty() {
            missing.push("changes");
        }
        if self.next_steps.is_empty() {
            missing.push("next_steps");
        }
        if self.remaining_issues.is_empty() {
            missing.push("remaining_issues");
        }
        if self.dependencies.is_empty() {
            missing.push("dependencies");
        }
        if self.validation_evidence.is_empty() {
            missing.push("validation_evidence");
        }
        missing
    }
}

/// Record of a file change within a handover.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    /// Absolute or workspace-relative file path.
    pub file_path: String,
    /// Nature of the change.
    pub change_type: ChangeType,
    /// Brief summary of what was changed.
    pub summary: String,
    /// Lines added (if known).
    pub lines_added: Option<u32>,
    /// Lines removed (if known).
    pub lines_removed: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Create,
    Modify,
    Delete,
    Refactor,
    Format,
    Config,
}

// ── DecompositionPlan ────────────────────────────────────────────────────

/// Full decomposition result: mission → SubTask[].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionPlan {
    /// Original mission description.
    pub mission: String,
    /// The strategy used for decomposition.
    pub strategy: OrchestrationStrategy,
    /// Ordered list of sub-tasks.
    pub sub_tasks: Vec<SubTask>,
    /// Maximum parallel workers allowed.
    pub max_parallel: u32,
    /// Maximum replanning rounds.
    pub max_replans: u32,
    /// Current replan round counter (lives inside the plan lock for consistency).
    pub replan_count: u32,
    /// Plan creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl DecompositionPlan {
    pub fn new(mission: String, strategy: OrchestrationStrategy) -> Self {
        Self {
            mission,
            strategy,
            sub_tasks: Vec::new(),
            max_parallel: 4,
            max_replans: 3,
            replan_count: 0,
            created_at: Utc::now(),
        }
    }

    /// Returns sub-tasks that have all dependencies met and are pending.
    pub fn ready_sub_tasks(&self) -> Vec<&SubTask> {
        self.sub_tasks
            .iter()
            .filter(|st| {
                st.status == SubTaskStatus::Pending
                    && st.dependencies.iter().all(|dep_id| {
                        self.sub_tasks
                            .iter()
                            .any(|s| s.id == *dep_id && s.status == SubTaskStatus::Completed)
                    })
            })
            .collect()
    }

    /// Returns true if all sub-tasks are in a terminal state (Completed / Failed / Skipped).
    pub fn is_terminal(&self) -> bool {
        self.sub_tasks.iter().all(|st| {
            matches!(
                st.status,
                SubTaskStatus::Completed | SubTaskStatus::Failed | SubTaskStatus::Skipped
            )
        })
    }

    /// Count of completed sub-tasks.
    pub fn completed_count(&self) -> usize {
        self.sub_tasks
            .iter()
            .filter(|st| st.status == SubTaskStatus::Completed)
            .count()
    }

    /// Count of failed sub-tasks.
    pub fn failed_count(&self) -> usize {
        self.sub_tasks
            .iter()
            .filter(|st| st.status == SubTaskStatus::Failed)
            .count()
    }
}

// ── OrchestrationEvent ───────────────────────────────────────────────────

/// Events emitted during orchestration for monitoring and logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum OrchestrationEvent {
    /// Mission received and decomposition started.
    DecompositionStarted { mission: String, strategy: String },
    /// Decomposition completed with N sub-tasks.
    DecompositionCompleted {
        sub_task_count: usize,
        plan: DecompositionPlan,
    },
    /// A sub-task has been dispatched to a worker.
    SubTaskDispatched {
        sub_task_id: String,
        worker_node_id: String,
    },
    /// A sub-task completed successfully.
    SubTaskCompleted {
        sub_task_id: String,
        handover: Option<StructuredHandover>,
    },
    /// A sub-task failed.
    SubTaskFailed { sub_task_id: String, error: String },
    /// Replanning triggered due to failures.
    ReplanTriggered {
        failed_sub_tasks: Vec<String>,
        replan_round: u32,
    },
    /// Orchestration fully complete (all sub-tasks terminal).
    OrchestrationCompleted {
        total_sub_tasks: usize,
        completed: usize,
        failed: usize,
    },
    /// Orchestration aborted (max replans exceeded).
    OrchestrationAborted { reason: String },
}

// ── OrchestrationError ────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum OrchestrationError {
    #[error("Decomposition failed: {0}")]
    DecompositionFailed(String),

    #[error("Subgraph generation failed: {0}")]
    SubgraphGenerationFailed(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Replan failed: {0}")]
    ReplanFailed(String),

    #[error("Max replan rounds ({0}) exceeded")]
    MaxReplansExceeded(u32),

    #[error("Subtask not found: {0}")]
    SubTaskNotFound(String),

    #[error("No ready sub-tasks to dispatch")]
    NoReadyTasks,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
