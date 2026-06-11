// SPDX-License-Identifier: AGPL-3.0-only

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub episode_id: String,
    pub step: u32,
    pub state: ExperienceState,
    pub action: ExperienceAction,
    pub reward: f32,
    pub cumulative_reward: f32,
    pub next_state: ExperienceState,
    pub done: bool,
    pub metadata: ExperienceMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceState {
    pub task_id: String,
    pub task_type: TaskType,
    pub context: StateContext,
    pub available_actions: Vec<String>,
    pub completed_actions: Vec<String>,
    pub error_count: u32,
    pub elapsed_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    CodeGeneration,
    InformationRetrieval,
    DataAnalysis,
    FileOperation,
    WebInteraction,
    ProblemSolving,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateContext {
    pub entities: HashMap<String, String>,
    pub constraints: Vec<String>,
    pub preferences: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceAction {
    pub action_id: String,
    pub action_type: ActionType,
    pub tool_id: Option<String>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    ToolCall,
    TaskDecomposition,
    ErrorRecovery,
    Reflection,
    UserConfirmation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceMetadata {
    pub environment: String,
    pub model_id: String,
    pub session_id: String,
    pub user_id: Option<String>,
}

impl Experience {
    pub fn new(
        episode_id: String,
        step: u32,
        state: ExperienceState,
        action: ExperienceAction,
        reward: f32,
        next_state: ExperienceState,
        done: bool,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            episode_id,
            step,
            state,
            action,
            reward,
            cumulative_reward: 0.0,
            next_state,
            done,
            metadata: ExperienceMetadata {
                environment: "axagent".to_string(),
                model_id: "unknown".to_string(),
                session_id: "unknown".to_string(),
                user_id: None,
            },
        }
    }

    pub fn state_action_key(&self) -> String {
        format!("{}:{}", self.state.task_id, self.action.action_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub task_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub experiences: Vec<Experience>,
    pub total_reward: f32,
    pub status: EpisodeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EpisodeStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Episode {
    pub fn new(task_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id,
            start_time: Utc::now(),
            end_time: None,
            experiences: Vec::new(),
            total_reward: 0.0,
            status: EpisodeStatus::Running,
        }
    }

    pub fn add_experience(&mut self, experience: Experience) {
        self.total_reward += experience.reward;
        self.experiences.push(experience);
    }

    pub fn complete(&mut self) {
        self.end_time = Some(Utc::now());
        self.status = EpisodeStatus::Completed;
    }

    pub fn fail(&mut self) {
        self.end_time = Some(Utc::now());
        self.status = EpisodeStatus::Failed;
    }
}

pub struct ExperienceBuffer {
    capacity: usize,
    buffer: Vec<Experience>,
}

impl ExperienceBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, experience: Experience) {
        if self.buffer.len() >= self.capacity {
            self.buffer.remove(0);
        }
        self.buffer.push(experience);
    }

    pub fn sample(&self, batch_size: usize) -> Vec<&Experience> {
        let len = self.buffer.len();
        if len == 0 {
            return vec![];
        }
        let batch_size = batch_size.min(len);
        let mut indices: Vec<usize> = (0..len).collect();
        for i in 0..batch_size {
            let j = i + (fastrand::usize(..(len - i)));
            indices.swap(i, j);
        }
        indices
            .into_iter()
            .take(batch_size)
            .map(|i| &self.buffer[i])
            .collect()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(task_id: &str) -> ExperienceState {
        ExperienceState {
            task_id: task_id.to_string(),
            task_type: TaskType::CodeGeneration,
            context: StateContext {
                entities: HashMap::new(),
                constraints: vec![],
                preferences: HashMap::new(),
            },
            available_actions: vec!["action_a".to_string()],
            completed_actions: vec![],
            error_count: 0,
            elapsed_ms: 0,
            timestamp: Utc::now(),
        }
    }

    fn make_action(action_id: &str) -> ExperienceAction {
        ExperienceAction {
            action_id: action_id.to_string(),
            action_type: ActionType::ToolCall,
            tool_id: Some("tool_1".to_string()),
            parameters: HashMap::new(),
            reasoning: "test".to_string(),
        }
    }

    fn make_experience(episode_id: &str, step: u32, reward: f32) -> Experience {
        Experience::new(
            episode_id.to_string(),
            step,
            make_state("task_1"),
            make_action("action_1"),
            reward,
            make_state("task_1"),
            false,
        )
    }

    #[test]
    fn test_experience_new() {
        let exp = make_experience("ep1", 1, 1.0);
        assert!(!exp.id.is_empty());
        assert_eq!(exp.episode_id, "ep1");
        assert_eq!(exp.step, 1);
        assert!((exp.reward - 1.0).abs() < 0.001);
        assert!((exp.cumulative_reward - 0.0).abs() < 0.001);
        assert!(!exp.done);
    }

    #[test]
    fn test_experience_state_action_key() {
        let exp = make_experience("ep1", 1, 1.0);
        let key = exp.state_action_key();
        assert!(key.contains("task_1"));
        assert!(key.contains("action_1"));
    }

    #[test]
    fn test_experience_metadata_default() {
        let exp = make_experience("ep1", 1, 0.5);
        assert_eq!(exp.metadata.environment, "axagent");
        assert_eq!(exp.metadata.model_id, "unknown");
        assert!(exp.metadata.user_id.is_none());
    }

    #[test]
    fn test_episode_new() {
        let episode = Episode::new("task_1".to_string());
        assert!(!episode.id.is_empty());
        assert_eq!(episode.task_id, "task_1");
        assert!(episode.experiences.is_empty());
        assert!((episode.total_reward - 0.0).abs() < 0.001);
        assert_eq!(episode.status, EpisodeStatus::Running);
        assert!(episode.end_time.is_none());
    }

    #[test]
    fn test_episode_add_experience() {
        let mut episode = Episode::new("task_1".to_string());
        let exp = make_experience("ep1", 1, 0.5);
        episode.add_experience(exp);
        assert_eq!(episode.experiences.len(), 1);
        assert!((episode.total_reward - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_episode_complete() {
        let mut episode = Episode::new("task_1".to_string());
        episode.complete();
        assert_eq!(episode.status, EpisodeStatus::Completed);
        assert!(episode.end_time.is_some());
    }

    #[test]
    fn test_episode_fail() {
        let mut episode = Episode::new("task_1".to_string());
        episode.fail();
        assert_eq!(episode.status, EpisodeStatus::Failed);
        assert!(episode.end_time.is_some());
    }

    #[test]
    fn test_experience_buffer_new() {
        let buffer = ExperienceBuffer::new(100);
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_experience_buffer_push() {
        let mut buffer = ExperienceBuffer::new(100);
        let exp = make_experience("ep1", 1, 1.0);
        buffer.push(exp);
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_experience_buffer_capacity() {
        let mut buffer = ExperienceBuffer::new(3);
        for i in 0..5 {
            buffer.push(make_experience("ep1", i, i as f32));
        }
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_experience_buffer_sample_empty() {
        let buffer = ExperienceBuffer::new(100);
        let sample = buffer.sample(10);
        assert!(sample.is_empty());
    }

    #[test]
    fn test_experience_buffer_sample() {
        let mut buffer = ExperienceBuffer::new(100);
        for i in 0..20 {
            buffer.push(make_experience("ep1", i, i as f32));
        }
        let sample = buffer.sample(5);
        assert_eq!(sample.len(), 5);
    }

    #[test]
    fn test_experience_buffer_sample_more_than_available() {
        let mut buffer = ExperienceBuffer::new(100);
        for i in 0..3 {
            buffer.push(make_experience("ep1", i, i as f32));
        }
        let sample = buffer.sample(10);
        assert_eq!(sample.len(), 3);
    }

    #[test]
    fn test_experience_buffer_clear() {
        let mut buffer = ExperienceBuffer::new(100);
        buffer.push(make_experience("ep1", 1, 1.0));
        buffer.push(make_experience("ep1", 2, 2.0));
        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_task_type_variants() {
        let types = vec![
            TaskType::CodeGeneration,
            TaskType::InformationRetrieval,
            TaskType::DataAnalysis,
            TaskType::FileOperation,
            TaskType::WebInteraction,
            TaskType::ProblemSolving,
            TaskType::General,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let de: TaskType = serde_json::from_str(&json).unwrap();
            assert_eq!(de, t);
        }
    }

    #[test]
    fn test_action_type_variants() {
        let types = vec![
            ActionType::ToolCall,
            ActionType::TaskDecomposition,
            ActionType::ErrorRecovery,
            ActionType::Reflection,
            ActionType::UserConfirmation,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let de: ActionType = serde_json::from_str(&json).unwrap();
            assert_eq!(de, t);
        }
    }

    #[test]
    fn test_episode_status_variants() {
        let statuses = vec![
            EpisodeStatus::Running,
            EpisodeStatus::Completed,
            EpisodeStatus::Failed,
            EpisodeStatus::Cancelled,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let de: EpisodeStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(de, s);
        }
    }

    #[test]
    fn test_experience_serialization() {
        let exp = make_experience("ep1", 1, 0.8);
        let json = serde_json::to_string(&exp).unwrap();
        let de: Experience = serde_json::from_str(&json).unwrap();
        assert_eq!(de.episode_id, "ep1");
        assert!((de.reward - 0.8).abs() < 0.001);
    }
}
