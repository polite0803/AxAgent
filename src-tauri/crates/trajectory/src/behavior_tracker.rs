use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorEvent {
    pub id: String,
    pub user_id: String,
    pub event_type: BehaviorEventType,
    pub timestamp: DateTime<Utc>,
    pub context: EventContext,
    pub metadata: HashMap<String, String>,
    pub interaction_id: Option<String>,
}

impl BehaviorEvent {
    pub fn new(user_id: String, event_type: BehaviorEventType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            event_type,
            timestamp: Utc::now(),
            context: EventContext::default(),
            metadata: HashMap::new(),
            interaction_id: None,
        }
    }

    pub fn with_context(mut self, context: EventContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn with_interaction_id(mut self, interaction_id: String) -> Self {
        self.interaction_id = Some(interaction_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BehaviorEventType {
    CodeGeneration {
        language: String,
        framework: Option<String>,
        line_count: u32,
        has_tests: bool,
    },
    SearchQuery {
        query_type: String,
        result_count: u32,
        clicked_result: Option<u32>,
    },
    ArtifactCreation {
        artifact_type: String,
        complexity: f32,
    },
    ConversationStart {
        intent: Option<String>,
    },
    ToolUsage {
        tool_name: String,
        success: bool,
        duration_ms: u64,
    },
    FeedbackGiven {
        feedback_type: UserFeedbackType,
        rating: Option<i32>,
    },
    PreferenceSet {
        setting_key: String,
        old_value: Option<String>,
        new_value: String,
    },
    FileOpened {
        file_path: String,
        file_type: String,
    },
    FileEdited {
        file_path: String,
        edit_type: String,
        lines_changed: u32,
    },
    ErrorOccurred {
        error_type: String,
        severity: ErrorSeverity,
        recovered: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UserFeedbackType {
    Positive,
    Negative,
    Neutral,
    Suggestion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventContext {
    pub conversation_id: Option<String>,
    pub session_id: Option<String>,
    pub project_path: Option<String>,
    pub current_file: Option<String>,
    pub language: Option<String>,
    pub time_of_day: Option<u8>,
    pub day_of_week: Option<u8>,
}

impl EventContext {
    pub fn with_conversation_id(mut self, id: String) -> Self {
        self.conversation_id = Some(id);
        self
    }

    pub fn with_session_id(mut self, id: String) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn with_project_path(mut self, path: String) -> Self {
        self.project_path = Some(path);
        self
    }

    pub fn with_current_file(mut self, file: String) -> Self {
        self.current_file = Some(file);
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_time_context(mut self) -> Self {
        let now = Utc::now();
        self.time_of_day = Some(now.hour() as u8);
        self.day_of_week = Some(now.weekday().num_days_from_monday() as u8);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSummary {
    pub user_id: String,
    pub total_events: u64,
    pub events_by_type: HashMap<String, u64>,
    pub recent_interactions: u32,
    pub most_used_tools: Vec<ToolUsageStats>,
    pub coding_stats: CodingStats,
    pub session_count: u32,
    pub avg_session_duration_minutes: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStats {
    pub tool_name: String,
    pub usage_count: u32,
    pub success_rate: f32,
    pub avg_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStats {
    pub total_lines_generated: u64,
    pub languages_used: HashMap<String, u32>,
    pub frameworks_used: HashMap<String, u32>,
    pub avg_code_complexity: f32,
}

impl Default for BehaviorSummary {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            total_events: 0,
            events_by_type: HashMap::new(),
            recent_interactions: 0,
            most_used_tools: Vec::new(),
            coding_stats: CodingStats::default(),
            session_count: 0,
            avg_session_duration_minutes: 0.0,
        }
    }
}

impl Default for CodingStats {
    fn default() -> Self {
        Self {
            total_lines_generated: 0,
            languages_used: HashMap::new(),
            frameworks_used: HashMap::new(),
            avg_code_complexity: 0.0,
        }
    }
}

pub struct BehaviorTracker {
    user_id: String,
    event_buffer: Vec<BehaviorEvent>,
    session_start: DateTime<Utc>,
}

impl BehaviorTracker {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            event_buffer: Vec::new(),
            session_start: Utc::now(),
        }
    }

    pub fn track_event(&mut self, event: BehaviorEvent) {
        self.event_buffer.push(event);
    }

    pub fn track_code_generation(
        &mut self,
        language: String,
        framework: Option<String>,
        line_count: u32,
        has_tests: bool,
    ) {
        let event = BehaviorEvent::new(
            self.user_id.clone(),
            BehaviorEventType::CodeGeneration {
                language,
                framework,
                line_count,
                has_tests,
            },
        );
        self.track_event(event);
    }

    pub fn track_search_query(
        &mut self,
        query_type: String,
        result_count: u32,
        clicked_result: Option<u32>,
    ) {
        let event = BehaviorEvent::new(
            self.user_id.clone(),
            BehaviorEventType::SearchQuery {
                query_type,
                result_count,
                clicked_result,
            },
        );
        self.track_event(event);
    }

    pub fn track_tool_usage(&mut self, tool_name: String, success: bool, duration_ms: u64) {
        let event = BehaviorEvent::new(
            self.user_id.clone(),
            BehaviorEventType::ToolUsage {
                tool_name,
                success,
                duration_ms,
            },
        );
        self.track_event(event);
    }

    pub fn track_feedback(&mut self, feedback_type: UserFeedbackType, rating: Option<i32>) {
        let event = BehaviorEvent::new(
            self.user_id.clone(),
            BehaviorEventType::FeedbackGiven {
                feedback_type,
                rating,
            },
        );
        self.track_event(event);
    }

    pub fn track_preference_change(
        &mut self,
        setting_key: String,
        old_value: Option<String>,
        new_value: String,
    ) {
        let event = BehaviorEvent::new(
            self.user_id.clone(),
            BehaviorEventType::PreferenceSet {
                setting_key,
                old_value,
                new_value,
            },
        );
        self.track_event(event);
    }

    pub fn flush_events(&mut self) -> Vec<BehaviorEvent> {
        let events: Vec<BehaviorEvent> = self.event_buffer.drain(..).collect();
        events
    }

    pub fn get_buffered_count(&self) -> usize {
        self.event_buffer.len()
    }

    pub fn summarize(&self) -> BehaviorSummary {
        let mut summary = BehaviorSummary {
            user_id: self.user_id.clone(),
            ..Default::default()
        };

        summary.total_events = self.event_buffer.len() as u64;

        for event in &self.event_buffer {
            let type_name = match &event.event_type {
                BehaviorEventType::CodeGeneration { .. } => "code_generation",
                BehaviorEventType::SearchQuery { .. } => "search_query",
                BehaviorEventType::ArtifactCreation { .. } => "artifact_creation",
                BehaviorEventType::ConversationStart { .. } => "conversation_start",
                BehaviorEventType::ToolUsage { .. } => "tool_usage",
                BehaviorEventType::FeedbackGiven { .. } => "feedback",
                BehaviorEventType::PreferenceSet { .. } => "preference_set",
                BehaviorEventType::FileOpened { .. } => "file_opened",
                BehaviorEventType::FileEdited { .. } => "file_edited",
                BehaviorEventType::ErrorOccurred { .. } => "error",
            };

            *summary
                .events_by_type
                .entry(type_name.to_string())
                .or_insert(0) += 1;
        }

        summary
    }

    pub fn session_duration(&self) -> chrono::Duration {
        Utc::now() - self.session_start
    }

    pub fn session_start_time(&self) -> DateTime<Utc> {
        self.session_start
    }
}
