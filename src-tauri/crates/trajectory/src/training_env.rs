use crate::trajectory::{Trajectory, TrajectoryOutcome};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub prompt: String,
    pub expected_outcome: Option<String>,
    pub difficulty: f64,
    pub category: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardComputation {
    pub task_completion: f64,
    pub tool_efficiency: f64,
    pub reasoning_quality: f64,
    pub error_recovery: f64,
    pub total: f64,
}

impl RewardComputation {
    pub fn from_trajectory(trajectory: &Trajectory) -> Self {
        let task_completion = match trajectory.outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure => 0.0,
            TrajectoryOutcome::Abandoned => 0.0,
        };
        let tool_count = trajectory
            .steps
            .iter()
            .filter(|s| {
                s.tool_calls
                    .as_ref()
                    .map(|t| !t.is_empty())
                    .unwrap_or(false)
            })
            .count();
        let tool_efficiency = if tool_count > 0 {
            (1.0 / (1.0 + tool_count as f64 * 0.1)).min(1.0)
        } else {
            0.5
        };
        let reasoning_steps = trajectory
            .steps
            .iter()
            .filter(|s| s.reasoning.is_some())
            .count();
        let reasoning_quality = (reasoning_steps as f64 * 0.2).min(1.0);
        let error_steps = trajectory
            .steps
            .iter()
            .filter(|s| {
                s.tool_results
                    .as_ref()
                    .map(|r| r.iter().any(|t| t.is_error))
                    .unwrap_or(false)
            })
            .count();
        let error_recovery = if error_steps > 0 { 0.3 } else { 1.0 };
        let total = task_completion * 0.4
            + tool_efficiency * 0.2
            + reasoning_quality * 0.15
            + error_recovery * 0.15
            + 0.1;
        Self {
            task_completion,
            tool_efficiency,
            reasoning_quality,
            error_recovery,
            total,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub task_id: String,
    pub trajectory_id: String,
    pub reward: RewardComputation,
    pub passed: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct TrainingEnv {
    tasks: Vec<TaskDefinition>,
}

impl TrainingEnv {
    pub fn new(tasks: Vec<TaskDefinition>) -> Self {
        Self { tasks }
    }

    pub fn tasks(&self) -> &[TaskDefinition] {
        &self.tasks
    }

    pub fn evaluate(&self, trajectory: &Trajectory) -> EvaluationResult {
        let reward = RewardComputation::from_trajectory(trajectory);
        EvaluationResult {
            task_id: trajectory.topic.clone(),
            trajectory_id: trajectory.id.clone(),
            reward: reward.clone(),
            passed: reward.total >= 0.6,
            metadata: HashMap::new(),
        }
    }
}
