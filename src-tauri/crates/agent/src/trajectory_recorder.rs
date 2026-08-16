// SPDX-License-Identifier: AGPL-3.0-only

use axagent_harness::trajectory_types::{
    MessageRole, ToolCall as TrajectoryToolCall, Trajectory, TrajectoryOutcome, TrajectoryQuality,
    TrajectoryStep, TrajectoryToolResult,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectorySummary {
    pub id: String,
    pub session_id: String,
    pub topic: String,
    pub outcome: TrajectoryOutcome,
    pub quality_score: f64,
    pub duration_ms: u64,
    pub step_count: usize,
    pub tool_call_count: usize,
    pub created_at: DateTime<Utc>,
}

impl From<&Trajectory> for TrajectorySummary {
    fn from(t: &Trajectory) -> Self {
        let tool_call_count =
            t.steps.iter().filter_map(|s| s.tool_calls.as_ref()).map(|c| c.len()).sum();
        Self {
            id: t.id.clone(),
            session_id: t.session_id.clone(),
            topic: t.topic.clone(),
            outcome: t.outcome,
            quality_score: t.quality.overall,
            duration_ms: t.duration_ms,
            step_count: t.steps.len(),
            tool_call_count,
            created_at: t.created_at,
        }
    }
}

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
            agent_name: None,
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
        let has_errors = state.tool_results.iter().any(|r| r.is_error)
            || state.steps.iter().any(|s| {
                s.tool_results.as_ref().map(|r| r.iter().any(|tr| tr.is_error)).unwrap_or(false)
            });

        if has_errors {
            TrajectoryOutcome::Failure
        } else if state.steps.is_empty() {
            // 无步骤、无错误：标记为 Abandoned（用户取消/任务被遗弃）
            TrajectoryOutcome::Abandoned
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
                s.tool_results.as_ref().map(|r| !r.iter().any(|tr| tr.is_error)).unwrap_or(false)
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

        format!("Executed {} steps with {} tool calls", total_steps, tool_count)
    }
}

impl Default for TrajectoryRecorder {
    fn default() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string(), "default".to_string(), "unknown".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::trajectory_types::TrajectoryToolResult;
    use axagent_harness::trajectory_types::{
        MessageRole, ToolCall as TrajectoryToolCall, Trajectory, TrajectoryOutcome,
        TrajectoryQuality, TrajectoryStep,
    };

    fn make_step(
        role: MessageRole,
        content: &str,
        tool_calls: Option<Vec<TrajectoryToolCall>>,
        tool_results: Option<Vec<TrajectoryToolResult>>,
        reasoning: Option<&str>,
    ) -> TrajectoryStep {
        TrajectoryStep {
            timestamp_ms: 100,
            role,
            content: content.to_string(),
            reasoning: reasoning.map(|s| s.to_string()),
            tool_calls,
            tool_results,
        }
    }

    fn make_tool_call(name: &str, id: &str) -> TrajectoryToolCall {
        TrajectoryToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments: "{}".to_string(),
        }
    }

    fn make_tool_result(
        tool_use_id: &str,
        tool_name: &str,
        is_error: bool,
    ) -> TrajectoryToolResult {
        TrajectoryToolResult {
            tool_use_id: tool_use_id.to_string(),
            tool_name: tool_name.to_string(),
            output: "ok".to_string(),
            is_error,
        }
    }

    fn make_trajectory(outcome: TrajectoryOutcome, steps: Vec<TrajectoryStep>) -> Trajectory {
        Trajectory {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: "sess1".into(),
            user_id: "user1".into(),
            agent_name: None,
            topic: "test topic".into(),
            summary: "test summary".into(),
            outcome,
            duration_ms: 5000,
            quality: TrajectoryQuality::default(),
            value_score: 0.5,
            patterns: Vec::new(),
            steps,
            rewards: Vec::new(),
            created_at: Utc::now(),
            replay_count: 0,
            last_replay_at: None,
        }
    }

    #[test]
    fn test_trajectory_summary_from_trajectory() {
        let steps = vec![
            make_step(
                MessageRole::Assistant,
                "hello",
                Some(vec![make_tool_call("read_file", "tc1")]),
                None,
                Some("thinking"),
            ),
            make_step(
                MessageRole::Tool,
                "result",
                None,
                Some(vec![make_tool_result("tc1", "read_file", false)]),
                None,
            ),
        ];
        let traj = make_trajectory(TrajectoryOutcome::Success, steps);
        let summary = TrajectorySummary::from(&traj);
        assert_eq!(summary.id, traj.id);
        assert_eq!(summary.session_id, "sess1");
        assert_eq!(summary.topic, "test topic");
        assert_eq!(summary.outcome, TrajectoryOutcome::Success);
        assert_eq!(summary.step_count, 2);
        assert_eq!(summary.tool_call_count, 1);
        assert_eq!(summary.duration_ms, 5000);
    }

    #[test]
    fn test_trajectory_summary_from_empty_trajectory() {
        let traj = make_trajectory(TrajectoryOutcome::Failure, vec![]);
        let summary = TrajectorySummary::from(&traj);
        assert_eq!(summary.step_count, 0);
        assert_eq!(summary.tool_call_count, 0);
    }

    #[test]
    fn test_trajectory_summary_from_multiple_tool_calls() {
        let steps = vec![
            make_step(
                MessageRole::Assistant,
                "a",
                Some(vec![make_tool_call("t1", "id1"), make_tool_call("t2", "id2")]),
                None,
                None,
            ),
            make_step(
                MessageRole::Assistant,
                "b",
                Some(vec![make_tool_call("t3", "id3")]),
                None,
                None,
            ),
        ];
        let traj = make_trajectory(TrajectoryOutcome::Success, steps);
        let summary = TrajectorySummary::from(&traj);
        assert_eq!(summary.tool_call_count, 3);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_new() {
        TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
    }

    #[tokio::test]
    async fn test_trajectory_recorder_start_and_stop() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("hello").await;
        let traj = recorder.stop_recording().await;
        assert!(!traj.id.is_empty());
        assert_eq!(traj.session_id, "sess1");
        assert_eq!(traj.user_id, "user1");
        assert_eq!(traj.topic, "test topic");
    }

    #[tokio::test]
    async fn test_trajectory_recorder_record_llm_response() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_llm_response("thinking about it", Some("reasoning step")).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.steps.len(), 1);
        assert_eq!(traj.steps[0].content, "thinking about it");
        assert_eq!(traj.steps[0].reasoning.as_deref(), Some("reasoning step"));
    }

    #[tokio::test]
    async fn test_trajectory_recorder_record_tool_call_and_result() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", r#"{"path":"/tmp"}"#).await;
        recorder.record_tool_result("tc1", "read_file", "file contents", false).await;
        recorder.record_llm_response("here is the result", None).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.steps.len(), 1);
        let step = &traj.steps[0];
        assert!(step.tool_calls.is_some());
        assert!(step.tool_results.is_some());
        assert_eq!(step.tool_calls.as_ref().expect("测试：引用应存在").len(), 1);
        assert_eq!(step.tool_calls.as_ref().expect("测试：引用应存在")[0].name, "read_file");
        assert_eq!(step.tool_results.as_ref().expect("测试：引用应存在").len(), 1);
        assert_eq!(step.tool_results.as_ref().expect("测试：引用应存在")[0].tool_name, "read_file");
    }

    #[tokio::test]
    async fn test_trajectory_recorder_no_record_when_not_recording() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder.record_tool_result("tc1", "read_file", "result", false).await;
        recorder.record_llm_response("response", None).await;
        let traj = recorder.stop_recording().await;
        assert!(traj.steps.is_empty());
    }

    #[tokio::test]
    async fn test_trajectory_recorder_determine_outcome_success() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder.record_tool_result("tc1", "read_file", "ok", false).await;
        recorder.record_llm_response("done", None).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.outcome, TrajectoryOutcome::Success);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_determine_outcome_failure_on_error() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("bad_tool", "tc1", "{}").await;
        recorder.record_tool_result("tc1", "bad_tool", "error!", true).await;
        recorder.record_llm_response("oops", None).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.outcome, TrajectoryOutcome::Failure);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_determine_outcome_abandoned_on_empty() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.outcome, TrajectoryOutcome::Abandoned);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_compute_quality_success() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder.record_tool_result("tc1", "read_file", "ok", false).await;
        recorder.record_llm_response("done", Some("reasoning")).await;
        let traj = recorder.stop_recording().await;
        assert!(traj.quality.overall > 0.0);
        assert!(traj.quality.task_completion > 0.0);
        assert!(traj.quality.tool_efficiency > 0.0);
        assert!(traj.quality.reasoning_quality > 0.0);
        assert!(traj.quality.user_satisfaction > 0.0);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_compute_quality_failure() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("bad_tool", "tc1", "{}").await;
        recorder.record_tool_result("tc1", "bad_tool", "err", true).await;
        recorder.record_llm_response("failed", None).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.quality.task_completion, 0.0);
        assert_eq!(traj.quality.user_satisfaction, 0.1);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_generate_summary_empty() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.summary, "No steps recorded");
    }

    #[tokio::test]
    async fn test_trajectory_recorder_generate_summary_with_steps() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder.record_tool_result("tc1", "read_file", "ok", false).await;
        recorder.record_llm_response("done", None).await;
        let traj = recorder.stop_recording().await;
        assert!(traj.summary.contains("1 steps"));
        assert!(traj.summary.contains("1 tool calls"));
    }

    #[tokio::test]
    async fn test_trajectory_recorder_multiple_steps() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder.record_tool_result("tc1", "read_file", "ok", false).await;
        recorder.record_llm_response("step1", None).await;
        recorder.record_tool_call("write_file", "tc2", "{}").await;
        recorder.record_tool_result("tc2", "write_file", "ok", false).await;
        recorder.record_llm_response("step2", None).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.steps.len(), 2);
        assert!(traj.summary.contains("2 steps"));
        assert!(traj.summary.contains("2 tool calls"));
    }

    #[tokio::test]
    async fn test_trajectory_recorder_clears_on_start() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input1").await;
        recorder.record_llm_response("step1", None).await;
        recorder.stop_recording().await;

        recorder.start_recording("input2").await;
        recorder.record_llm_response("step2", None).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.steps.len(), 1);
        assert_eq!(traj.steps[0].content, "step2");
    }

    #[test]
    fn test_compute_value_score_success() {
        let steps = vec![make_step(MessageRole::Assistant, "a", None, None, None)];
        let score =
            TrajectoryRecorder::compute_value_score(0.8, TrajectoryOutcome::Success, &steps);
        assert!(score > 0.0);
        assert!(score <= 2.0);
    }

    #[test]
    fn test_compute_value_score_failure() {
        let steps = vec![make_step(MessageRole::Assistant, "a", None, None, None)];
        let score =
            TrajectoryRecorder::compute_value_score(0.0, TrajectoryOutcome::Failure, &steps);
        assert!(score >= -1.0);
    }

    #[test]
    fn test_compute_value_score_abandoned() {
        let steps = vec![make_step(MessageRole::Assistant, "a", None, None, None)];
        let score =
            TrajectoryRecorder::compute_value_score(0.2, TrajectoryOutcome::Abandoned, &steps);
        assert!(score >= -1.0);
    }

    #[test]
    fn test_compute_value_score_partial() {
        let steps = vec![make_step(MessageRole::Assistant, "a", None, None, None)];
        let score =
            TrajectoryRecorder::compute_value_score(0.5, TrajectoryOutcome::Partial, &steps);
        assert!(score > 0.0);
    }

    #[test]
    fn test_compute_value_score_empty_steps() {
        let score = TrajectoryRecorder::compute_value_score(0.5, TrajectoryOutcome::Success, &[]);
        assert!(score > 0.0);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_tool_calls_cleared_after_llm_response() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder.record_tool_result("tc1", "read_file", "ok", false).await;
        recorder.record_llm_response("step1", None).await;

        recorder.record_tool_call("write_file", "tc2", "{}").await;
        recorder.record_tool_result("tc2", "write_file", "ok", false).await;
        recorder.record_llm_response("step2", None).await;

        let traj = recorder.stop_recording().await;
        assert_eq!(traj.steps.len(), 2);
        assert_eq!(
            traj.steps[0].tool_calls.as_ref().expect("测试：引用应存在")[0].name,
            "read_file"
        );
        assert_eq!(
            traj.steps[1].tool_calls.as_ref().expect("测试：引用应存在")[0].name,
            "write_file"
        );
    }

    #[tokio::test]
    async fn test_trajectory_recorder_llm_response_without_tools() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_llm_response("just thinking", Some("reasoning")).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.steps.len(), 1);
        assert!(traj.steps[0].tool_calls.is_none());
        assert!(traj.steps[0].tool_results.is_none());
        assert_eq!(traj.steps[0].content, "just thinking");
    }

    #[tokio::test]
    async fn test_trajectory_recorder_quality_clamped() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder.record_tool_result("tc1", "read_file", "ok", false).await;
        recorder.record_llm_response("done", Some("deep reasoning")).await;
        let traj = recorder.stop_recording().await;
        assert!(traj.quality.overall >= 0.0 && traj.quality.overall <= 1.0);
    }
}
