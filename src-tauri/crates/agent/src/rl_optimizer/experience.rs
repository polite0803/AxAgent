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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
