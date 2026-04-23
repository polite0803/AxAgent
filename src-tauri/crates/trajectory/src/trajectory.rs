//! Core trajectory data structures and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub tool_name: String,
    pub output: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStep {
    pub timestamp_ms: u64,
    pub role: MessageRole,
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_results: Option<Vec<ToolResult>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrajectoryOutcome {
    Success,
    Failure,
    Partial,
    Abandoned,
}

impl TrajectoryOutcome {
    pub fn from_str(s: &str) -> Self {
        match s {
            "success" => TrajectoryOutcome::Success,
            "failure" => TrajectoryOutcome::Failure,
            "partial" => TrajectoryOutcome::Partial,
            "abandoned" => TrajectoryOutcome::Abandoned,
            _ => TrajectoryOutcome::Failure,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TrajectoryOutcome::Success => "success",
            TrajectoryOutcome::Failure => "failure",
            TrajectoryOutcome::Partial => "partial",
            TrajectoryOutcome::Abandoned => "abandoned",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryQuality {
    pub overall: f64,
    pub task_completion: f64,
    pub tool_efficiency: f64,
    pub reasoning_quality: f64,
    pub user_satisfaction: f64,
}

impl Default for TrajectoryQuality {
    fn default() -> Self {
        Self {
            overall: 0.5,
            task_completion: 0.5,
            tool_efficiency: 0.5,
            reasoning_quality: 0.5,
            user_satisfaction: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub topic: String,
    pub summary: String,
    pub outcome: TrajectoryOutcome,
    pub duration_ms: u64,
    pub quality: TrajectoryQuality,
    pub value_score: f64,
    pub patterns: Vec<String>,
    pub steps: Vec<TrajectoryStep>,
    pub rewards: Vec<RewardSignal>,
    pub created_at: DateTime<Utc>,
    pub replay_count: u32,
    pub last_replay_at: Option<DateTime<Utc>>,
}

impl Trajectory {
    pub fn new(
        session_id: String,
        user_id: String,
        topic: String,
        summary: String,
        outcome: TrajectoryOutcome,
        duration_ms: u64,
        steps: Vec<TrajectoryStep>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let quality = Self::compute_quality(&steps, outcome);
        let value_score = Self::compute_value_score(quality.overall, outcome, &steps);

        Self {
            id,
            session_id,
            user_id,
            topic,
            summary,
            outcome,
            duration_ms,
            quality,
            value_score,
            patterns: Vec::new(),
            rewards: Vec::new(),
            steps,
            created_at: Utc::now(),
            replay_count: 0,
            last_replay_at: None,
        }
    }

    fn compute_quality(steps: &[TrajectoryStep], outcome: TrajectoryOutcome) -> TrajectoryQuality {
        let task_completion = match outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure => 0.0,
            TrajectoryOutcome::Abandoned => 0.2,
        };

        let tool_count = steps.iter().filter(|s| s.tool_calls.is_some()).count();
        let successful_tools = steps
            .iter()
            .filter(|s| {
                s.tool_results
                    .as_ref()
                    .map(|r| !r.iter().any(|tr| tr.is_error))
                    .unwrap_or(false)
            })
            .count();
        let tool_efficiency = if tool_count > 0 {
            successful_tools as f64 / tool_count as f64
        } else {
            0.5
        };

        let reasoning_count = steps.iter().filter(|s| s.reasoning.is_some()).count();
        let reasoning_quality = if !steps.is_empty() {
            reasoning_count as f64 / steps.len() as f64 * 0.5 + 0.25
        } else {
            0.25
        };

        let user_satisfaction = match outcome {
            TrajectoryOutcome::Success => 0.9,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure => 0.1,
            TrajectoryOutcome::Abandoned => 0.3,
        };

        let overall = task_completion * 0.4
            + tool_efficiency * 0.2
            + reasoning_quality * 0.15
            + user_satisfaction * 0.25;

        TrajectoryQuality {
            overall: overall.clamp(0.0, 1.0),
            task_completion,
            tool_efficiency,
            reasoning_quality,
            user_satisfaction,
        }
    }

    fn compute_value_score(
        quality: f64,
        outcome: TrajectoryOutcome,
        steps: &[TrajectoryStep],
    ) -> f64 {
        let mut score = quality * 0.5;

        match outcome {
            TrajectoryOutcome::Success => score += 0.35,
            TrajectoryOutcome::Partial => score += 0.15,
            TrajectoryOutcome::Failure => score -= 0.2,
            TrajectoryOutcome::Abandoned => score -= 0.3,
        }

        let has_reasoning = steps.iter().any(|s| s.reasoning.is_some());
        if has_reasoning {
            score += 0.1;
        }

        let step_count = steps.len();
        if step_count >= 3 && step_count <= 30 {
            score += 0.05;
        }

        score.clamp(0.0, 1.0)
    }

    pub fn add_reward(&mut self, reward: RewardSignal) {
        self.rewards.push(reward);
    }

    pub fn increment_replay(&mut self) {
        self.replay_count += 1;
        self.last_replay_at = Some(Utc::now());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardType {
    TaskCompletion,
    ToolEfficiency,
    ReasoningQuality,
    ErrorRecovery,
    UserFeedback,
    PatternMatch,
}

impl RewardType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "task_completion" => RewardType::TaskCompletion,
            "tool_efficiency" => RewardType::ToolEfficiency,
            "reasoning_quality" => RewardType::ReasoningQuality,
            "error_recovery" => RewardType::ErrorRecovery,
            "user_feedback" => RewardType::UserFeedback,
            "pattern_match" => RewardType::PatternMatch,
            _ => RewardType::TaskCompletion,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RewardType::TaskCompletion => "task_completion",
            RewardType::ToolEfficiency => "tool_efficiency",
            RewardType::ReasoningQuality => "reasoning_quality",
            RewardType::ErrorRecovery => "error_recovery",
            RewardType::UserFeedback => "user_feedback",
            RewardType::PatternMatch => "pattern_match",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSignal {
    pub reward_type: RewardType,
    pub value: f64,
    pub step_index: usize,
    pub timestamp_ms: u64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub pattern_type: String,
    pub trajectory_ids: Vec<String>,
    pub frequency: u32,
    pub success_rate: f64,
    pub average_quality: f64,
    pub average_value_score: f64,
    pub reward_profile: Vec<(RewardType, f64)>,
    pub created_at: DateTime<Utc>,
}

impl TrajectoryPattern {
    pub fn new(name: String, description: String, pattern_type: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            pattern_type,
            trajectory_ids: Vec::new(),
            frequency: 0,
            success_rate: 0.0,
            average_quality: 0.0,
            average_value_score: 0.0,
            reward_profile: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn update_from_trajectory(&mut self, trajectory: &Trajectory) {
        if !self.trajectory_ids.contains(&trajectory.id) {
            self.trajectory_ids.push(trajectory.id.clone());
        }

        self.frequency = self.trajectory_ids.len() as u32;

        let prev_total = (self.frequency - 1) as f64;
        let success = match trajectory.outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            _ => 0.0,
        };

        self.success_rate = if prev_total > 0.0 {
            (self.success_rate * prev_total + success) / self.frequency as f64
        } else {
            success
        };

        let quality_delta = trajectory.quality.overall - self.average_quality;
        self.average_quality += quality_delta / self.frequency as f64;

        let value_delta = trajectory.value_score - self.average_value_score;
        self.average_value_score += value_delta / self.frequency as f64;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayContext {
    pub trajectory_id: String,
    pub current_step: usize,
    pub original_trajectory: Trajectory,
    pub deviations: Vec<TrajectoryStep>,
    pub evaluation: f64,
    pub next_suggested_action: Option<String>,
    pub accumulated_reward: f64,
}

impl ReplayContext {
    pub fn new(trajectory: Trajectory) -> Self {
        Self {
            trajectory_id: trajectory.id.clone(),
            current_step: 0,
            original_trajectory: trajectory,
            deviations: Vec::new(),
            evaluation: 0.5,
            next_suggested_action: None,
            accumulated_reward: 0.0,
        }
    }

    pub fn evaluate(&mut self) {
        let mut score = 0.5;

        if self.deviations.is_empty() {
            score += 0.3;
        } else {
            score -= (self.deviations.len() as f64 * 0.05).min(0.25);
        }

        let step_progress =
            self.current_step as f64 / self.original_trajectory.steps.len().max(1) as f64;

        if step_progress > 0.5 && self.original_trajectory.outcome == TrajectoryOutcome::Success {
            score += 0.2;
        }

        score += self.accumulated_reward * 0.1;

        self.evaluation = score.clamp(0.0, 1.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryQuery {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub topic: Option<String>,
    pub outcome: Option<TrajectoryOutcome>,
    pub min_quality: Option<f64>,
    pub min_value_score: Option<f64>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub limit: Option<usize>,
}

impl Default for TrajectoryQuery {
    fn default() -> Self {
        Self {
            session_id: None,
            user_id: None,
            topic: None,
            outcome: None,
            min_quality: None,
            min_value_score: None,
            time_range: None,
            limit: Some(100),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryExportOptions {
    pub format: ExportFormat,
    pub min_quality: Option<f64>,
    pub min_value_score: Option<f64>,
    pub outcome_filter: Option<TrajectoryOutcome>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Jsonl,
    RlTraining,
    Compressed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLTrainingEntry {
    pub prompt: String,
    pub completion: String,
    pub trajectory_id: String,
    pub topic: String,
    pub quality: f64,
    pub value_score: f64,
    pub rewards: Vec<RewardSignal>,
}

impl Trajectory {
    pub fn export_as_rl(&self) -> RLTrainingEntry {
        let prompt = self
            .steps
            .iter()
            .filter(|s| s.role == MessageRole::User)
            .map(|s| s.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut completion = String::new();
        for step in self
            .steps
            .iter()
            .filter(|s| s.role == MessageRole::Assistant)
        {
            completion.push_str(&step.content);
            if let Some(ref tool_calls) = step.tool_calls {
                completion.push_str("\n\n<tool_calls>\n");
                completion.push_str(&serde_json::to_string(tool_calls).unwrap_or_default());
                completion.push_str("\n</tool_calls>\n");
            }
            completion.push_str("\n\n");
        }

        RLTrainingEntry {
            prompt: prompt.chars().take(4000).collect(),
            completion: completion.chars().take(4000).collect(),
            trajectory_id: self.id.clone(),
            topic: self.topic.clone(),
            quality: self.quality.overall,
            value_score: self.value_score,
            rewards: self.rewards.clone(),
        }
    }
}

pub struct TrajectoryBuilder {
    session_id: String,
    user_id: String,
    steps: Vec<TrajectoryStep>,
}

impl TrajectoryBuilder {
    pub fn new(session_id: String, user_id: String) -> Self {
        Self {
            session_id,
            user_id,
            steps: Vec::new(),
        }
    }

    pub fn add_step(mut self, step: TrajectoryStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn build(
        self,
        topic: String,
        summary: String,
        outcome: TrajectoryOutcome,
        duration_ms: u64,
    ) -> Trajectory {
        Trajectory::new(
            self.session_id,
            self.user_id,
            topic,
            summary,
            outcome,
            duration_ms,
            self.steps,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedTrajectory {
    pub id: String,
    pub topic: String,
    pub outcome: String,
    #[serde(rename = "qualityScore")]
    pub quality_score: f64,
    #[serde(rename = "valueScore")]
    pub value_score: f64,
    #[serde(rename = "stepSummaries")]
    pub step_summaries: Vec<String>,
    #[serde(rename = "toolSequence")]
    pub tool_sequence: Vec<String>,
    #[serde(rename = "finalReward")]
    pub final_reward: f64,
}
