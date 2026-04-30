use axagent_trajectory::{
    MessageRole, ToolCall as TrajectoryToolCall, ToolResult as TrajectoryToolResult, Trajectory,
    TrajectoryOutcome, TrajectoryQuality, TrajectoryStep,
};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct TrajectoryRecorder {
    state: Arc<RwLock<TrajectoryRecorderState>>,
}

#[derive(Debug)]
struct TrajectoryRecorderState {
    session_id: String,
    user_id: String,
    topic: String,
    start_time: chrono::DateTime<Utc>,
    steps: Vec<TrajectoryStep>,
    tool_calls: Vec<TrajectoryToolCall>,
    tool_results: Vec<TrajectoryToolResult>,
    input: String,
    is_recording: bool,
}

impl TrajectoryRecorder {
    pub fn new(session_id: String, user_id: String, topic: String) -> Self {
        Self {
            state: Arc::new(RwLock::new(TrajectoryRecorderState {
                session_id,
                user_id,
                topic,
                start_time: Utc::now(),
                steps: Vec::new(),
                tool_calls: Vec::new(),
                tool_results: Vec::new(),
                input: String::new(),
                is_recording: false,
            })),
        }
    }

    pub async fn start_recording(&self, input: &str) {
        let mut state = self.state.write().await;
        state.input = input.to_string();
        state.start_time = Utc::now();
        state.steps.clear();
        state.tool_calls.clear();
        state.tool_results.clear();
        state.is_recording = true;
    }

    pub async fn record_tool_call(&self, tool_name: &str, tool_use_id: &str, arguments: &str) {
        let mut state = self.state.write().await;
        if !state.is_recording {
            return;
        }
        state.tool_calls.push(TrajectoryToolCall {
            id: tool_use_id.to_string(),
            name: tool_name.to_string(),
            arguments: arguments.to_string(),
        });
    }

    pub async fn record_tool_result(
        &self,
        tool_use_id: &str,
        tool_name: &str,
        output: &str,
        is_error: bool,
    ) {
        let mut state = self.state.write().await;
        if !state.is_recording {
            return;
        }
        state.tool_results.push(TrajectoryToolResult {
            tool_use_id: tool_use_id.to_string(),
            tool_name: tool_name.to_string(),
            output: output.to_string(),
            is_error,
        });
    }

    pub async fn record_llm_response(&self, content: &str, reasoning: Option<&str>) {
        let mut state = self.state.write().await;
        if !state.is_recording {
            return;
        }

        let tool_calls_for_step = if !state.tool_calls.is_empty() {
            let calls: Vec<TrajectoryToolCall> = state.tool_calls.clone();
            state.tool_calls.clear();
            Some(calls)
        } else {
            None
        };

        let tool_results_for_step = if !state.tool_results.is_empty() {
            let results: Vec<TrajectoryToolResult> = state.tool_results.clone();
            state.tool_results.clear();
            Some(results)
        } else {
            None
        };

        let step = TrajectoryStep {
            timestamp_ms: (Utc::now() - state.start_time).num_milliseconds() as u64,
            role: MessageRole::Assistant,
            content: content.to_string(),
            reasoning: reasoning.map(|s| s.to_string()),
            tool_calls: tool_calls_for_step,
            tool_results: tool_results_for_step,
        };

        state.steps.push(step);
    }

    pub async fn stop_recording(&self) -> Trajectory {
        let mut state = self.state.write().await;
        state.is_recording = false;

        let end_time = Utc::now();
        let duration_ms = (end_time - state.start_time).num_milliseconds() as u64;

        let outcome = self.determine_outcome(&state);
        let quality = self.compute_quality(&state.steps, outcome);
        let value_score = Self::compute_value_score(quality.overall, outcome, &state.steps);

        Trajectory {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: state.session_id.clone(),
            user_id: state.user_id.clone(),
            topic: state.topic.clone(),
            summary: self.generate_summary(&state.steps),
            outcome,
            duration_ms,
            quality,
            value_score,
            patterns: Vec::new(),
            steps: state.steps.clone(),
            rewards: Vec::new(),
            created_at: state.start_time,
            replay_count: 0,
            last_replay_at: None,
        }
    }

    fn determine_outcome(&self, state: &TrajectoryRecorderState) -> TrajectoryOutcome {
        let has_errors = state.tool_results.iter().any(|r| r.is_error);

        if has_errors || state.steps.is_empty() {
            TrajectoryOutcome::Failure
        } else {
            TrajectoryOutcome::Success
        }
    }

    fn compute_quality(
        &self,
        steps: &[TrajectoryStep],
        outcome: TrajectoryOutcome,
    ) -> TrajectoryQuality {
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

        let overall = (task_completion * 0.4
            + tool_efficiency * 0.2
            + reasoning_quality * 0.2
            + user_satisfaction * 0.2)
            .clamp(0.0, 1.0);

        TrajectoryQuality {
            overall,
            task_completion,
            tool_efficiency,
            reasoning_quality,
            user_satisfaction,
        }
    }

    fn compute_value_score(
        overall: f64,
        outcome: TrajectoryOutcome,
        steps: &[TrajectoryStep],
    ) -> f64 {
        let outcome_bonus = match outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure => 0.0,
            TrajectoryOutcome::Abandoned => -0.5,
        };

        let efficiency = if !steps.is_empty() {
            1.0 / steps.len() as f64
        } else {
            0.0
        };

        (overall + outcome_bonus + efficiency).clamp(-1.0, 2.0)
    }

    fn generate_summary(&self, steps: &[TrajectoryStep]) -> String {
        if steps.is_empty() {
            return "No steps recorded".to_string();
        }

        let tool_count = steps.iter().filter(|s| s.tool_calls.is_some()).count();
        let total_steps = steps.len();

        format!(
            "Executed {} steps with {} tool calls",
            total_steps, tool_count
        )
    }
}

impl Default for TrajectoryRecorder {
    fn default() -> Self {
        Self::new(
            uuid::Uuid::new_v4().to_string(),
            "default".to_string(),
            "unknown".to_string(),
        )
    }
}
