use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityType {
    ContextPrediction,
    ProactiveSuggestion,
    TaskPrefetch,
    RoutineReminder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveCapability {
    pub capability_type: CapabilityType,
    pub confidence: f32,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub action: ProactiveAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCondition {
    pub condition_type: TriggerConditionType,
    pub threshold: Option<f32>,
    pub context_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerConditionType {
    FileOpened,
    ErrorDetected,
    TimeBased,
    PatternMatch,
    UserIdle,
    LowActivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProactiveAction {
    ShowSuggestion,
    PrefetchResource,
    SendReminder,
    GenerateNudge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPrediction {
    pub predicted_intent: PredictedIntent,
    pub confidence: f32,
    pub reasoning: String,
    pub suggested_actions: Vec<SuggestedAction>,
    pub context_window: ContextWindow,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PredictedIntent {
    CodeCompletion { language: String, context: String },
    Documentation { topic: String },
    Search { query_type: String },
    Refactoring { target: String },
    Debug { error: String },
    TestGeneration { target: String },
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    pub files: Vec<String>,
    pub recent_actions: Vec<String>,
    pub current_language: Option<String>,
    pub project_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_type: String,
    pub title: String,
    pub description: String,
    pub priority: Priority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Priority {
    pub fn as_u32(&self) -> u32 {
        match self {
            Priority::Low => 0,
            Priority::Medium => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveSuggestion {
    pub id: String,
    pub suggestion_type: SuggestionType,
    pub title: String,
    pub description: String,
    pub action: SuggestionAction,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub accepted: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionType {
    Completion,
    Refactor,
    Documentation,
    Test,
    Optimization,
    Learning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SuggestionAction {
    PrefetchCompletion { language: String, context: String },
    ShowRefactorOptions { target: String },
    GenerateDocs { topic: String },
    GenerateTests { target: String },
    ShowOptimizations { target: String },
    ShowLearningResources { topic: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub description: String,
    pub scheduled_at: DateTime<Utc>,
    pub recurrence: Option<ReminderRecurrence>,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderRecurrence {
    pub frequency: RecurrenceFrequency,
    pub interval: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveConfig {
    pub enabled: bool,
    pub max_suggestions: usize,
    pub suggestion_ttl_minutes: i64,
    pub prediction_confidence_threshold: f32,
    pub prefetch_enabled: bool,
    pub reminder_enabled: bool,
}

impl Default for ProactiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_suggestions: 5,
            suggestion_ttl_minutes: 5,
            prediction_confidence_threshold: 0.6,
            prefetch_enabled: true,
            reminder_enabled: true,
        }
    }
}

pub struct ProactiveAssistant {
    config: ProactiveConfig,
    active_suggestions: HashMap<String, ProactiveSuggestion>,
    reminders: HashMap<String, Reminder>,
    recent_predictions: Vec<ContextPrediction>,
}

impl Default for ProactiveAssistant {
    fn default() -> Self {
        Self::new()
    }
}

impl ProactiveAssistant {
    pub fn new() -> Self {
        Self {
            config: ProactiveConfig::default(),
            active_suggestions: HashMap::new(),
            reminders: HashMap::new(),
            recent_predictions: Vec::new(),
        }
    }

    pub fn with_config(config: ProactiveConfig) -> Self {
        Self {
            config,
            active_suggestions: HashMap::new(),
            reminders: HashMap::new(),
            recent_predictions: Vec::new(),
        }
    }

    pub fn get_config(&self) -> &ProactiveConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: ProactiveConfig) {
        self.config = config;
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    pub fn add_suggestion(&mut self, suggestion: ProactiveSuggestion) {
        if self.active_suggestions.len() >= self.config.max_suggestions {
            return;
        }
        self.active_suggestions
            .insert(suggestion.id.clone(), suggestion);
    }

    pub fn get_active_suggestions(&self) -> Vec<&ProactiveSuggestion> {
        let now = Utc::now();
        self.active_suggestions
            .values()
            .filter(|s| s.expires_at > now)
            .collect()
    }

    pub fn dismiss_suggestion(&mut self, id: &str) -> Option<ProactiveSuggestion> {
        self.active_suggestions.remove(id)
    }

    pub fn accept_suggestion(&mut self, id: &str) -> Option<&ProactiveSuggestion> {
        if let Some(suggestion) = self.active_suggestions.get_mut(id) {
            suggestion.accepted = Some(true);
        }
        self.active_suggestions.get(id)
    }

    pub fn snooze_suggestion(
        &mut self,
        id: &str,
        duration_minutes: i64,
    ) -> Option<ProactiveSuggestion> {
        if let Some(suggestion) = self.active_suggestions.get_mut(id) {
            suggestion.expires_at = Utc::now() + Duration::minutes(duration_minutes);
        }
        self.active_suggestions.get(id).cloned()
    }

    pub fn add_reminder(&mut self, reminder: Reminder) {
        self.reminders.insert(reminder.id.clone(), reminder);
    }

    pub fn get_reminders(&self) -> Vec<&Reminder> {
        self.reminders.values().filter(|r| !r.completed).collect()
    }

    pub fn complete_reminder(&mut self, id: &str) -> Option<Reminder> {
        if let Some(reminder) = self.reminders.get_mut(id) {
            reminder.completed = true;
        }
        self.reminders.get(id).cloned()
    }

    pub fn delete_reminder(&mut self, id: &str) -> Option<Reminder> {
        self.reminders.remove(id)
    }

    pub fn get_due_reminders(&self) -> Vec<&Reminder> {
        let now = Utc::now();
        self.reminders
            .values()
            .filter(|r| !r.completed && r.scheduled_at <= now)
            .collect()
    }

    pub fn record_prediction(&mut self, prediction: ContextPrediction) {
        self.recent_predictions.push(prediction);
        if self.recent_predictions.len() > 100 {
            self.recent_predictions.remove(0);
        }
    }

    pub fn get_recent_predictions(&self) -> &[ContextPrediction] {
        &self.recent_predictions
    }

    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();
        self.active_suggestions.retain(|_, s| s.expires_at > now);
    }

    pub fn generate_suggestion_id() -> String {
        format!("suggestion_{}", Uuid::new_v4())
    }

    pub fn generate_reminder_id() -> String {
        format!("reminder_{}", Uuid::new_v4())
    }
}

impl ProactiveSuggestion {
    pub fn new(
        suggestion_type: SuggestionType,
        title: String,
        description: String,
        action: SuggestionAction,
        priority: Priority,
        ttl_minutes: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ProactiveAssistant::generate_suggestion_id(),
            suggestion_type,
            title,
            description,
            action,
            priority,
            created_at: now,
            expires_at: now + Duration::minutes(ttl_minutes),
            accepted: None,
        }
    }
}

impl Reminder {
    pub fn new(title: String, description: String, scheduled_at: DateTime<Utc>) -> Self {
        Self {
            id: ProactiveAssistant::generate_reminder_id(),
            title,
            description,
            scheduled_at,
            recurrence: None,
            completed: false,
            created_at: Utc::now(),
        }
    }

    pub fn with_recurrence(mut self, frequency: RecurrenceFrequency, interval: u32) -> Self {
        self.recurrence = Some(ReminderRecurrence {
            frequency,
            interval,
        });
        self
    }
}
