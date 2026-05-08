use axagent_trajectory::{
    MessageRole, ToolCall as TrajectoryToolCall, ToolResult as TrajectoryToolResult, Trajectory,
    TrajectoryOutcome, TrajectoryQuality, TrajectoryStep,
};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
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
        let tool_call_count = t
            .steps
            .iter()
            .filter_map(|s| s.tool_calls.as_ref())
            .map(|c| c.len())
            .sum();
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStep {
    pub step_index: usize,
    pub timestamp_ms: u64,
    pub action: String,
    pub result_summary: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub trajectory: Trajectory,
    pub replay_log: Vec<ReplayStep>,
    pub insights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayComparison {
    pub trajectory_a: TrajectorySummary,
    pub trajectory_b: TrajectorySummary,
    pub quality_diff: f64,
    pub duration_diff_ms: i64,
    pub tool_count_diff: i32,
    pub outcome_match: bool,
}

pub struct TrajectoryStore {
    db: Arc<DatabaseConnection>,
}

impl std::fmt::Debug for TrajectoryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrajectoryStore").finish()
    }
}

impl TrajectoryStore {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn save(&self, trajectory: &Trajectory) -> Result<(), String> {
        use axagent_core::entity::settings as settings_model;
        use axagent_core::entity::settings::Entity as SettingsEntity;

        let key = format!("trajectory:{}", trajectory.id);
        let json = serde_json::to_string(trajectory).map_err(|e| e.to_string())?;

        let existing = SettingsEntity::find()
            .filter(settings_model::Column::Key.eq(&key))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        match existing {
            Some(record) => {
                let mut active: settings_model::ActiveModel = record.into();
                active.value = Set(json);
                active
                    .update(self.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())?;
            },
            None => {
                let active = settings_model::ActiveModel {
                    key: Set(key),
                    value: Set(json),
                };
                active
                    .insert(self.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())?;
            },
        }

        let index_key = "trajectory_index".to_string();
        let mut index: Vec<String> = self
            .load_index(&index_key)
            .await
            .unwrap_or_default()
            .unwrap_or_default();

        if !index.contains(&trajectory.id) {
            index.push(trajectory.id.clone());
            self.save_index(&index_key, &index).await?;
        }

        Ok(())
    }

    pub async fn load(&self, id: &str) -> Result<Option<Trajectory>, String> {
        use axagent_core::entity::settings as settings_model;
        use axagent_core::entity::settings::Entity as SettingsEntity;

        let key = format!("trajectory:{}", id);
        let result = SettingsEntity::find()
            .filter(settings_model::Column::Key.eq(&key))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        match result {
            Some(record) => {
                let trajectory: Trajectory =
                    serde_json::from_str(&record.value).map_err(|e| e.to_string())?;
                Ok(Some(trajectory))
            },
            None => Ok(None),
        }
    }

    pub async fn list(&self, limit: usize) -> Result<Vec<TrajectorySummary>, String> {
        let index = self
            .load_index("trajectory_index")
            .await
            .unwrap_or_default()
            .unwrap_or_default();
        let mut summaries = Vec::new();

        for id in index.iter().rev() {
            if summaries.len() >= limit {
                break;
            }
            if let Some(trajectory) = self.load(id).await? {
                summaries.push(TrajectorySummary::from(&trajectory));
            }
        }

        Ok(summaries)
    }

    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<TrajectorySummary>, String> {
        let all = self.list(1000).await?;
        let query_lower = query.to_lowercase();
        let results: Vec<TrajectorySummary> = all
            .into_iter()
            .filter(|s| {
                s.topic.to_lowercase().contains(&query_lower)
                    || s.session_id.to_lowercase().contains(&query_lower)
                    || s.id.to_lowercase().contains(&query_lower)
            })
            .take(limit)
            .collect();
        Ok(results)
    }

    pub async fn delete(&self, id: &str) -> Result<bool, String> {
        use axagent_core::entity::settings as settings_model;
        use axagent_core::entity::settings::Entity as SettingsEntity;

        let key = format!("trajectory:{}", id);
        let existing = SettingsEntity::find()
            .filter(settings_model::Column::Key.eq(&key))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        match existing {
            Some(record) => {
                let active: settings_model::ActiveModel = record.into();
                active
                    .delete(self.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())?;

                let index_key = "trajectory_index".to_string();
                let mut index = self
                    .load_index(&index_key)
                    .await
                    .unwrap_or_default()
                    .unwrap_or_default();
                index.retain(|i| i != id);
                self.save_index(&index_key, &index).await?;

                Ok(true)
            },
            None => Ok(false),
        }
    }

    async fn load_index(&self, index_key: &str) -> Result<Option<Vec<String>>, String> {
        use axagent_core::entity::settings as settings_model;
        use axagent_core::entity::settings::Entity as SettingsEntity;

        let result = SettingsEntity::find()
            .filter(settings_model::Column::Key.eq(index_key))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        match result {
            Some(record) => {
                let index: Vec<String> =
                    serde_json::from_str(&record.value).map_err(|e| e.to_string())?;
                Ok(Some(index))
            },
            None => Ok(None),
        }
    }

    async fn save_index(&self, index_key: &str, index: &[String]) -> Result<(), String> {
        use axagent_core::entity::settings as settings_model;
        use axagent_core::entity::settings::Entity as SettingsEntity;

        let json = serde_json::to_string(index).map_err(|e| e.to_string())?;

        let existing = SettingsEntity::find()
            .filter(settings_model::Column::Key.eq(index_key))
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        match existing {
            Some(record) => {
                let mut active: settings_model::ActiveModel = record.into();
                active.value = Set(json);
                active
                    .update(self.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())?;
            },
            None => {
                let active = settings_model::ActiveModel {
                    key: Set(index_key.to_string()),
                    value: Set(json),
                };
                active
                    .insert(self.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())?;
            },
        }

        Ok(())
    }
}

pub struct TrajectoryReplayer {
    store: Arc<TrajectoryStore>,
}

impl TrajectoryReplayer {
    pub fn new(store: Arc<TrajectoryStore>) -> Self {
        Self { store }
    }

    pub async fn replay(&self, id: &str) -> Result<ReplayResult, String> {
        let trajectory = self
            .store
            .load(id)
            .await?
            .ok_or_else(|| format!("Trajectory {} not found", id))?;

        let mut replay_log = Vec::new();
        let mut prev_ts: u64 = 0;

        for (i, step) in trajectory.steps.iter().enumerate() {
            let action = if let Some(ref tool_calls) = step.tool_calls {
                let names: Vec<&str> = tool_calls.iter().map(|c| c.name.as_str()).collect();
                format!("tool_calls: {}", names.join(", "))
            } else if let Some(ref tool_results) = step.tool_results {
                let names: Vec<&str> = tool_results.iter().map(|r| r.tool_name.as_str()).collect();
                format!("tool_results: {}", names.join(", "))
            } else {
                format!("{:?}", step.role)
            };

            let result_summary = if let Some(ref tool_results) = step.tool_results {
                let errors = tool_results.iter().filter(|r| r.is_error).count();
                if errors > 0 {
                    format!("{} results, {} errors", tool_results.len(), errors)
                } else {
                    format!("{} results", tool_results.len())
                }
            } else {
                let content_preview: String = step.content.chars().take(80).collect();
                content_preview
            };

            let duration_ms = if prev_ts > 0 {
                step.timestamp_ms.saturating_sub(prev_ts)
            } else {
                step.timestamp_ms
            };

            replay_log.push(ReplayStep {
                step_index: i,
                timestamp_ms: step.timestamp_ms,
                action,
                result_summary,
                duration_ms,
            });

            prev_ts = step.timestamp_ms;
        }

        let mut insights = Vec::new();

        let tool_steps: Vec<&TrajectoryStep> = trajectory
            .steps
            .iter()
            .filter(|s| s.tool_calls.is_some())
            .collect();
        if !tool_steps.is_empty() {
            insights.push(format!(
                "Used tools in {} out of {} steps ({:.0}% tool usage rate)",
                tool_steps.len(),
                trajectory.steps.len(),
                tool_steps.len() as f64 / trajectory.steps.len() as f64 * 100.0
            ));
        }

        let error_count = trajectory
            .steps
            .iter()
            .filter_map(|s| s.tool_results.as_ref())
            .flat_map(|r| r.iter())
            .filter(|r| r.is_error)
            .count();
        if error_count > 0 {
            insights.push(format!("Encountered {} tool errors during execution", error_count));
        }

        let reasoning_steps: Vec<&TrajectoryStep> = trajectory
            .steps
            .iter()
            .filter(|s| s.reasoning.is_some())
            .collect();
        if !reasoning_steps.is_empty() {
            insights.push(format!(
                "Reasoning present in {} steps ({:.0}%)",
                reasoning_steps.len(),
                reasoning_steps.len() as f64 / trajectory.steps.len().max(1) as f64 * 100.0
            ));
        }

        if trajectory.duration_ms > 0 && !trajectory.steps.is_empty() {
            let avg_step_ms = trajectory.duration_ms / trajectory.steps.len() as u64;
            insights.push(format!("Average step duration: {}ms", avg_step_ms));
        }

        let unique_tools: std::collections::HashSet<String> = trajectory
            .steps
            .iter()
            .filter_map(|s| s.tool_calls.as_ref())
            .flat_map(|c| c.iter())
            .map(|c| c.name.clone())
            .collect();
        if !unique_tools.is_empty() {
            insights.push(format!("Unique tools used: {}", unique_tools.len()));
        }

        insights.push(format!(
            "Outcome: {:?}, Quality: {:.2}, Value: {:.2}",
            trajectory.outcome, trajectory.quality.overall, trajectory.value_score
        ));

        Ok(ReplayResult {
            trajectory,
            replay_log,
            insights,
        })
    }

    pub async fn compare(&self, id_a: &str, id_b: &str) -> Result<ReplayComparison, String> {
        let traj_a = self
            .store
            .load(id_a)
            .await?
            .ok_or_else(|| format!("Trajectory {} not found", id_a))?;
        let traj_b = self
            .store
            .load(id_b)
            .await?
            .ok_or_else(|| format!("Trajectory {} not found", id_b))?;

        let summary_a = TrajectorySummary::from(&traj_a);
        let summary_b = TrajectorySummary::from(&traj_b);

        let quality_diff = summary_a.quality_score - summary_b.quality_score;
        let duration_diff_ms = summary_a.duration_ms as i64 - summary_b.duration_ms as i64;
        let tool_count_diff = summary_a.tool_call_count as i32 - summary_b.tool_call_count as i32;
        let outcome_match = summary_a.outcome == summary_b.outcome;

        Ok(ReplayComparison {
            trajectory_a: summary_a,
            trajectory_b: summary_b,
            quality_diff,
            duration_diff_ms,
            tool_count_diff,
            outcome_match,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TrajectoryRecorder {
    state: Arc<RwLock<TrajectoryRecorderState>>,
    store: Option<Arc<TrajectoryStore>>,
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
            store: None,
        }
    }

    pub fn with_store(mut self, store: Arc<TrajectoryStore>) -> Self {
        self.store = Some(store);
        self
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

        let trajectory = Trajectory {
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
        };

        if let Some(ref store) = self.store {
            let _ = store.save(&trajectory).await;
        }

        trajectory
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
    use axagent_trajectory::{
        MessageRole, ToolCall as TrajectoryToolCall, ToolResult as TrajectoryToolResult,
        Trajectory, TrajectoryOutcome, TrajectoryQuality, TrajectoryStep,
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

    #[test]
    fn test_replay_step_fields() {
        let step = ReplayStep {
            step_index: 0,
            timestamp_ms: 100,
            action: "tool_calls: read_file".into(),
            result_summary: "1 results".into(),
            duration_ms: 50,
        };
        assert_eq!(step.step_index, 0);
        assert_eq!(step.timestamp_ms, 100);
        assert_eq!(step.action, "tool_calls: read_file");
        assert_eq!(step.result_summary, "1 results");
        assert_eq!(step.duration_ms, 50);
    }

    #[test]
    fn test_replay_step_clone() {
        let step = ReplayStep {
            step_index: 1,
            timestamp_ms: 200,
            action: "action".into(),
            result_summary: "summary".into(),
            duration_ms: 100,
        };
        let cloned = step.clone();
        assert_eq!(cloned.step_index, step.step_index);
        assert_eq!(cloned.action, step.action);
    }

    #[test]
    fn test_replay_result_fields() {
        let traj = make_trajectory(TrajectoryOutcome::Success, vec![]);
        let result = ReplayResult {
            trajectory: traj.clone(),
            replay_log: vec![ReplayStep {
                step_index: 0,
                timestamp_ms: 0,
                action: "start".into(),
                result_summary: "begin".into(),
                duration_ms: 0,
            }],
            insights: vec!["insight1".into()],
        };
        assert_eq!(result.trajectory.id, traj.id);
        assert_eq!(result.replay_log.len(), 1);
        assert_eq!(result.insights.len(), 1);
    }

    #[test]
    fn test_replay_comparison_fields() {
        let comp = ReplayComparison {
            trajectory_a: TrajectorySummary {
                id: "a".into(),
                session_id: "s1".into(),
                topic: "t1".into(),
                outcome: TrajectoryOutcome::Success,
                quality_score: 0.9,
                duration_ms: 1000,
                step_count: 5,
                tool_call_count: 3,
                created_at: Utc::now(),
            },
            trajectory_b: TrajectorySummary {
                id: "b".into(),
                session_id: "s2".into(),
                topic: "t2".into(),
                outcome: TrajectoryOutcome::Failure,
                quality_score: 0.3,
                duration_ms: 2000,
                step_count: 8,
                tool_call_count: 6,
                created_at: Utc::now(),
            },
            quality_diff: 0.6,
            duration_diff_ms: -1000,
            tool_count_diff: -3,
            outcome_match: false,
        };
        assert!((comp.quality_diff - 0.6).abs() < f64::EPSILON);
        assert_eq!(comp.duration_diff_ms, -1000);
        assert_eq!(comp.tool_count_diff, -3);
        assert!(!comp.outcome_match);
    }

    #[test]
    fn test_replay_comparison_outcome_match() {
        let comp = ReplayComparison {
            trajectory_a: TrajectorySummary {
                id: "a".into(),
                session_id: "s1".into(),
                topic: "t1".into(),
                outcome: TrajectoryOutcome::Success,
                quality_score: 0.9,
                duration_ms: 1000,
                step_count: 5,
                tool_call_count: 3,
                created_at: Utc::now(),
            },
            trajectory_b: TrajectorySummary {
                id: "b".into(),
                session_id: "s2".into(),
                topic: "t2".into(),
                outcome: TrajectoryOutcome::Success,
                quality_score: 0.8,
                duration_ms: 1200,
                step_count: 4,
                tool_call_count: 2,
                created_at: Utc::now(),
            },
            quality_diff: 0.1,
            duration_diff_ms: -200,
            tool_count_diff: 1,
            outcome_match: true,
        };
        assert!(comp.outcome_match);
    }

    #[test]
    fn test_trajectory_summary_serialization() {
        let summary = TrajectorySummary {
            id: "id1".into(),
            session_id: "sess1".into(),
            topic: "topic1".into(),
            outcome: TrajectoryOutcome::Success,
            quality_score: 0.85,
            duration_ms: 3000,
            step_count: 10,
            tool_call_count: 4,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: TrajectorySummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "id1");
        assert_eq!(deserialized.session_id, "sess1");
        assert_eq!(deserialized.outcome, TrajectoryOutcome::Success);
        assert!((deserialized.quality_score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_replay_step_serialization() {
        let step = ReplayStep {
            step_index: 2,
            timestamp_ms: 500,
            action: "action".into(),
            result_summary: "summary".into(),
            duration_ms: 200,
        };
        let json = serde_json::to_string(&step).unwrap();
        let deserialized: ReplayStep = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.step_index, 2);
        assert_eq!(deserialized.timestamp_ms, 500);
    }

    #[test]
    fn test_replay_comparison_serialization() {
        let comp = ReplayComparison {
            trajectory_a: TrajectorySummary {
                id: "a".into(),
                session_id: "s1".into(),
                topic: "t1".into(),
                outcome: TrajectoryOutcome::Success,
                quality_score: 0.9,
                duration_ms: 1000,
                step_count: 5,
                tool_call_count: 3,
                created_at: Utc::now(),
            },
            trajectory_b: TrajectorySummary {
                id: "b".into(),
                session_id: "s2".into(),
                topic: "t2".into(),
                outcome: TrajectoryOutcome::Failure,
                quality_score: 0.3,
                duration_ms: 2000,
                step_count: 8,
                tool_call_count: 6,
                created_at: Utc::now(),
            },
            quality_diff: 0.6,
            duration_diff_ms: -1000,
            tool_count_diff: -3,
            outcome_match: false,
        };
        let json = serde_json::to_string(&comp).unwrap();
        let deserialized: ReplayComparison = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.outcome_match);
        assert_eq!(deserialized.tool_count_diff, -3);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_new() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        assert!(recorder.store.is_none());
    }

    #[tokio::test]
    async fn test_trajectory_recorder_default() {
        let recorder = TrajectoryRecorder::default();
        assert!(recorder.store.is_none());
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
        recorder
            .record_llm_response("thinking about it", Some("reasoning step"))
            .await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.steps.len(), 1);
        assert_eq!(traj.steps[0].content, "thinking about it");
        assert_eq!(traj.steps[0].reasoning.as_deref(), Some("reasoning step"));
    }

    #[tokio::test]
    async fn test_trajectory_recorder_record_tool_call_and_result() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder
            .record_tool_call("read_file", "tc1", r#"{"path":"/tmp"}"#)
            .await;
        recorder
            .record_tool_result("tc1", "read_file", "file contents", false)
            .await;
        recorder
            .record_llm_response("here is the result", None)
            .await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.steps.len(), 1);
        let step = &traj.steps[0];
        assert!(step.tool_calls.is_some());
        assert!(step.tool_results.is_some());
        assert_eq!(step.tool_calls.as_ref().unwrap().len(), 1);
        assert_eq!(step.tool_calls.as_ref().unwrap()[0].name, "read_file");
        assert_eq!(step.tool_results.as_ref().unwrap().len(), 1);
        assert_eq!(step.tool_results.as_ref().unwrap()[0].tool_name, "read_file");
    }

    #[tokio::test]
    async fn test_trajectory_recorder_no_record_when_not_recording() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder
            .record_tool_result("tc1", "read_file", "result", false)
            .await;
        recorder.record_llm_response("response", None).await;
        let traj = recorder.stop_recording().await;
        assert!(traj.steps.is_empty());
    }

    #[tokio::test]
    async fn test_trajectory_recorder_determine_outcome_success() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder
            .record_tool_result("tc1", "read_file", "ok", false)
            .await;
        recorder.record_llm_response("done", None).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.outcome, TrajectoryOutcome::Success);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_determine_outcome_failure_on_error() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("bad_tool", "tc1", "{}").await;
        recorder
            .record_tool_result("tc1", "bad_tool", "error!", true)
            .await;
        recorder.record_llm_response("oops", None).await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.outcome, TrajectoryOutcome::Failure);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_determine_outcome_failure_on_empty() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        let traj = recorder.stop_recording().await;
        assert_eq!(traj.outcome, TrajectoryOutcome::Failure);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_compute_quality_success() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder
            .record_tool_result("tc1", "read_file", "ok", false)
            .await;
        recorder
            .record_llm_response("done", Some("reasoning"))
            .await;
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
        recorder
            .record_tool_result("tc1", "bad_tool", "err", true)
            .await;
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
        recorder
            .record_tool_result("tc1", "read_file", "ok", false)
            .await;
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
        recorder
            .record_tool_result("tc1", "read_file", "ok", false)
            .await;
        recorder.record_llm_response("step1", None).await;
        recorder.record_tool_call("write_file", "tc2", "{}").await;
        recorder
            .record_tool_result("tc2", "write_file", "ok", false)
            .await;
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

    #[test]
    fn test_trajectory_recorder_with_store_none() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        assert!(recorder.store.is_none());
    }

    #[test]
    fn test_trajectory_summary_clone() {
        let summary = TrajectorySummary {
            id: "id1".into(),
            session_id: "sess1".into(),
            topic: "topic1".into(),
            outcome: TrajectoryOutcome::Success,
            quality_score: 0.85,
            duration_ms: 3000,
            step_count: 10,
            tool_call_count: 4,
            created_at: Utc::now(),
        };
        let cloned = summary.clone();
        assert_eq!(cloned.id, summary.id);
        assert_eq!(cloned.outcome, summary.outcome);
    }

    #[test]
    fn test_replay_result_clone() {
        let traj = make_trajectory(TrajectoryOutcome::Success, vec![]);
        let result = ReplayResult {
            trajectory: traj,
            replay_log: vec![],
            insights: vec!["insight".into()],
        };
        let cloned = result.clone();
        assert_eq!(cloned.insights.len(), 1);
    }

    #[test]
    fn test_replay_comparison_clone() {
        let comp = ReplayComparison {
            trajectory_a: TrajectorySummary {
                id: "a".into(),
                session_id: "s1".into(),
                topic: "t1".into(),
                outcome: TrajectoryOutcome::Success,
                quality_score: 0.9,
                duration_ms: 1000,
                step_count: 5,
                tool_call_count: 3,
                created_at: Utc::now(),
            },
            trajectory_b: TrajectorySummary {
                id: "b".into(),
                session_id: "s2".into(),
                topic: "t2".into(),
                outcome: TrajectoryOutcome::Failure,
                quality_score: 0.3,
                duration_ms: 2000,
                step_count: 8,
                tool_call_count: 6,
                created_at: Utc::now(),
            },
            quality_diff: 0.6,
            duration_diff_ms: -1000,
            tool_count_diff: -3,
            outcome_match: false,
        };
        let cloned = comp.clone();
        assert_eq!(cloned.quality_diff, comp.quality_diff);
    }

    #[tokio::test]
    async fn test_trajectory_recorder_tool_calls_cleared_after_llm_response() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder.record_tool_call("read_file", "tc1", "{}").await;
        recorder
            .record_tool_result("tc1", "read_file", "ok", false)
            .await;
        recorder.record_llm_response("step1", None).await;

        recorder.record_tool_call("write_file", "tc2", "{}").await;
        recorder
            .record_tool_result("tc2", "write_file", "ok", false)
            .await;
        recorder.record_llm_response("step2", None).await;

        let traj = recorder.stop_recording().await;
        assert_eq!(traj.steps.len(), 2);
        assert_eq!(traj.steps[0].tool_calls.as_ref().unwrap()[0].name, "read_file");
        assert_eq!(traj.steps[1].tool_calls.as_ref().unwrap()[0].name, "write_file");
    }

    #[tokio::test]
    async fn test_trajectory_recorder_llm_response_without_tools() {
        let recorder = TrajectoryRecorder::new("sess1".into(), "user1".into(), "test topic".into());
        recorder.start_recording("input").await;
        recorder
            .record_llm_response("just thinking", Some("reasoning"))
            .await;
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
        recorder
            .record_tool_result("tc1", "read_file", "ok", false)
            .await;
        recorder
            .record_llm_response("done", Some("deep reasoning"))
            .await;
        let traj = recorder.stop_recording().await;
        assert!(traj.quality.overall >= 0.0 && traj.quality.overall <= 1.0);
    }

    #[test]
    fn test_trajectory_store_debug() {
        let db: Arc<DatabaseConnection> = Arc::new(unsafe { std::mem::zeroed() });
        let store = TrajectoryStore::new(db);
        let debug_str = format!("{:?}", store);
        assert!(debug_str.contains("TrajectoryStore"));
    }
}
