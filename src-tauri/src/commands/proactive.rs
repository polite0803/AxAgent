// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_trajectory::ProactiveSuggestionType;
use axagent_trajectory::{
    ContextFeatures, ContextPredictor, PredictionResult as TrajectoryPredictionResult,
    ProactiveAssistant, ProactiveConfig as TrajProactiveConfig,
    ProactiveSuggestion as TrajProactiveSuggestion, Reminder as TrajReminder, ReminderRecurrence,
    SuggestionAction, SuggestionEngine, TaskPrefetcher,
};
use axagent_trajectory::{PrefetchResult, PrefetchResults, PrefetchType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProactiveSuggestion {
    pub id: String,
    pub suggestion_type: String,
    pub title: String,
    pub description: String,
    pub action: serde_json::Value,
    pub priority: String,
    pub created_at: String,
    pub expires_at: String,
    pub accepted: Option<bool>,
}

impl From<&TrajProactiveSuggestion> for ProactiveSuggestion {
    fn from(s: &TrajProactiveSuggestion) -> Self {
        let action = match &s.action {
            SuggestionAction::PrefetchCompletion { language, context } => {
                serde_json::json!({ "type": "PrefetchCompletion", "language": language, "context": context })
            },
            SuggestionAction::ShowRefactorOptions { target } => {
                serde_json::json!({ "type": "ShowRefactorOptions", "target": target })
            },
            SuggestionAction::GenerateDocs { topic } => {
                serde_json::json!({ "type": "GenerateDocs", "topic": topic })
            },
            SuggestionAction::GenerateTests { target } => {
                serde_json::json!({ "type": "GenerateTests", "target": target })
            },
            SuggestionAction::ShowOptimizations { target } => {
                serde_json::json!({ "type": "ShowOptimizations", "target": target })
            },
            SuggestionAction::ShowLearningResources { topic } => {
                serde_json::json!({ "type": "ShowLearningResources", "topic": topic })
            },
        };

        let suggestion_type = match s.suggestion_type {
            ProactiveSuggestionType::Completion => "Completion",
            ProactiveSuggestionType::Refactor => "Refactor",
            ProactiveSuggestionType::Documentation => "Documentation",
            ProactiveSuggestionType::Test => "Test",
            ProactiveSuggestionType::Optimization => "Optimization",
            ProactiveSuggestionType::Learning => "Learning",
        };

        let priority = match s.priority {
            axagent_trajectory::Priority::Low => "low",
            axagent_trajectory::Priority::Medium => "medium",
            axagent_trajectory::Priority::High => "high",
            axagent_trajectory::Priority::Critical => "critical",
        };

        Self {
            id: s.id.clone(),
            suggestion_type: suggestion_type.to_string(),
            title: s.title.clone(),
            description: s.description.clone(),
            action,
            priority: priority.to_string(),
            created_at: s.created_at.to_rfc3339(),
            expires_at: s.expires_at.to_rfc3339(),
            accepted: s.accepted,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContextPrediction {
    pub predicted_intent: serde_json::Value,
    pub confidence: f32,
    pub reasoning: String,
    pub suggested_actions: Vec<serde_json::Value>,
    pub context_window: serde_json::Value,
    pub created_at: String,
}

impl From<&axagent_trajectory::ContextPrediction> for ContextPrediction {
    fn from(p: &axagent_trajectory::ContextPrediction) -> Self {
        let predicted_intent = match &p.predicted_intent {
            axagent_trajectory::PredictedIntent::CodeCompletion { language, context } => {
                serde_json::json!({ "type": "CodeCompletion", "language": language, "context": context })
            },
            axagent_trajectory::PredictedIntent::Documentation { topic } => {
                serde_json::json!({ "type": "Documentation", "topic": topic })
            },
            axagent_trajectory::PredictedIntent::Search { query_type } => {
                serde_json::json!({ "type": "Search", "query_type": query_type })
            },
            axagent_trajectory::PredictedIntent::Refactoring { target } => {
                serde_json::json!({ "type": "Refactoring", "target": target })
            },
            axagent_trajectory::PredictedIntent::Debug { error } => {
                serde_json::json!({ "type": "Debug", "error": error })
            },
            axagent_trajectory::PredictedIntent::TestGeneration { target } => {
                serde_json::json!({ "type": "TestGeneration", "target": target })
            },
            axagent_trajectory::PredictedIntent::Unknown => {
                serde_json::json!({ "type": "Unknown" })
            },
        };

        let suggested_actions: Vec<serde_json::Value> = p
            .suggested_actions
            .iter()
            .map(|a| {
                serde_json::json!({
                    "action_type": a.action_type,
                    "title": a.title,
                    "description": a.description,
                    "priority": match a.priority {
                        axagent_trajectory::Priority::Low => "low",
                        axagent_trajectory::Priority::Medium => "medium",
                        axagent_trajectory::Priority::High => "high",
                        axagent_trajectory::Priority::Critical => "critical",
                    }
                })
            })
            .collect();

        let context_window = serde_json::json!({
            "files": p.context_window.files,
            "recent_actions": p.context_window.recent_actions,
            "current_language": p.context_window.current_language,
            "project_type": p.context_window.project_type,
        });

        Self {
            predicted_intent,
            confidence: p.confidence,
            reasoning: p.reasoning.clone(),
            suggested_actions,
            context_window,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictionResult {
    pub predictions: Vec<ContextPrediction>,
}

impl From<TrajectoryPredictionResult> for PredictionResult {
    fn from(result: TrajectoryPredictionResult) -> Self {
        Self {
            predictions: result
                .predictions
                .iter()
                .map(ContextPrediction::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub description: String,
    pub scheduled_at: String,
    pub recurrence: Option<serde_json::Value>,
    pub completed: bool,
    pub created_at: String,
}

impl From<&TrajReminder> for Reminder {
    fn from(r: &TrajReminder) -> Self {
        let recurrence = r.recurrence.as_ref().map(|rec| {
            serde_json::json!({
                "frequency": match rec.frequency {
                    axagent_trajectory::RecurrenceFrequency::Daily => "daily",
                    axagent_trajectory::RecurrenceFrequency::Weekly => "weekly",
                    axagent_trajectory::RecurrenceFrequency::Monthly => "monthly",
                },
                "interval": rec.interval,
            })
        });

        Self {
            id: r.id.clone(),
            title: r.title.clone(),
            description: r.description.clone(),
            scheduled_at: r.scheduled_at.to_rfc3339(),
            recurrence,
            completed: r.completed,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProactiveConfig {
    pub enabled: bool,
    #[serde(rename = "max_suggestions")]
    pub max_suggestions: i32,
    #[serde(rename = "suggestion_ttl_minutes")]
    pub suggestion_ttl_minutes: i32,
    #[serde(rename = "prediction_confidence_threshold")]
    pub prediction_confidence_threshold: f32,
    #[serde(rename = "prefetch_enabled")]
    pub prefetch_enabled: bool,
    #[serde(rename = "reminder_enabled")]
    pub reminder_enabled: bool,
}

impl From<&TrajProactiveConfig> for ProactiveConfig {
    fn from(c: &TrajProactiveConfig) -> Self {
        Self {
            enabled: c.enabled,
            max_suggestions: c.max_suggestions as i32,
            suggestion_ttl_minutes: c.suggestion_ttl_minutes as i32,
            prediction_confidence_threshold: c.prediction_confidence_threshold,
            prefetch_enabled: c.prefetch_enabled,
            reminder_enabled: c.reminder_enabled,
        }
    }
}

impl From<ProactiveConfig> for TrajProactiveConfig {
    fn from(c: ProactiveConfig) -> Self {
        Self {
            enabled: c.enabled,
            max_suggestions: c.max_suggestions as usize,
            suggestion_ttl_minutes: c.suggestion_ttl_minutes as i64,
            prediction_confidence_threshold: c.prediction_confidence_threshold,
            prefetch_enabled: c.prefetch_enabled,
            reminder_enabled: c.reminder_enabled,
        }
    }
}

pub struct ProactiveService {
    assistant: ProactiveAssistant,
    predictor: ContextPredictor,
    suggestion_engine: SuggestionEngine,
    #[allow(dead_code)]
    prefetcher: TaskPrefetcher,
}

impl ProactiveService {
    pub fn new() -> Self {
        Self {
            assistant: ProactiveAssistant::new(),
            predictor: ContextPredictor::new(),
            suggestion_engine: SuggestionEngine::new(),
            prefetcher: TaskPrefetcher::new(),
        }
    }

    pub fn get_suggestions(&self) -> Vec<ProactiveSuggestion> {
        self.assistant
            .get_active_suggestions()
            .iter()
            .map(|s| ProactiveSuggestion::from(*s))
            .collect()
    }

    pub fn refresh_suggestions(&mut self, features: ContextFeatures) -> Vec<ProactiveSuggestion> {
        if !self.assistant.is_enabled() {
            return vec![];
        }

        self.assistant.cleanup_expired();

        let prediction_result = self.predictor.predict(&features);

        for prediction in &prediction_result.predictions {
            let new_suggestions = self
                .suggestion_engine
                .generate_suggestions(&features, prediction, None);
            for suggestion in new_suggestions {
                self.assistant.add_suggestion(suggestion);
            }
        }

        self.get_suggestions()
    }

    pub fn dismiss_suggestion(&mut self, id: &str) -> bool {
        self.assistant.dismiss_suggestion(id).is_some()
    }

    pub fn accept_suggestion(&mut self, id: &str) -> bool {
        self.assistant.accept_suggestion(id).is_some()
    }

    pub fn snooze_suggestion(&mut self, id: &str, duration_minutes: i64) -> bool {
        self.assistant
            .snooze_suggestion(id, duration_minutes)
            .is_some()
    }

    pub fn add_reminder(&mut self, reminder: TrajReminder) {
        self.assistant.add_reminder(reminder);
    }

    pub fn get_reminders(&self) -> Vec<Reminder> {
        self.assistant
            .get_reminders()
            .iter()
            .map(|r| Reminder::from(*r))
            .collect()
    }

    pub fn complete_reminder(&mut self, id: &str) -> bool {
        self.assistant.complete_reminder(id).is_some()
    }

    pub fn delete_reminder(&mut self, id: &str) -> bool {
        self.assistant.delete_reminder(id).is_some()
    }

    pub fn predict(&self, context: ContextFeatures) -> PredictionResult {
        self.predictor.predict(&context).into()
    }

    pub fn get_config(&self) -> ProactiveConfig {
        ProactiveConfig::from(self.assistant.get_config())
    }

    pub fn update_config(&mut self, config: TrajProactiveConfig) {
        self.assistant.update_config(config);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.assistant.set_enabled(enabled);
    }

    pub fn is_enabled(&self) -> bool {
        self.assistant.is_enabled()
    }

    pub fn prefetch_for_predictions(
        &mut self,
        predictions: &[serde_json::Value],
    ) -> PrefetchResults {
        let mut results = PrefetchResults::new();

        for pred in predictions {
            let intent_type = pred
                .get("predicted_intent")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let id = pred
                .get("resource_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let (prefetch_type, estimated_ms) = match intent_type {
                "CodeCompletion" => (PrefetchType::CodeCompletion, 200),
                "Search" => (PrefetchType::SearchResults, 200),
                "Documentation" => (PrefetchType::Documentation, 500),
                "Refactoring" => (PrefetchType::ContextAnalysis, 800),
                "TestGeneration" => (PrefetchType::ContextAnalysis, 600),
                "Debug" => (PrefetchType::ContextAnalysis, 300),
                _ => continue,
            };

            if let Some(cached) = self.prefetcher.get_cached(&id) {
                results.add(cached.clone());
                results.critical_path.push(id.clone());
                continue;
            }

            let result = PrefetchResult {
                prefetch_type,
                resource_id: id.clone(),
                data: None,
                ready: false,
                estimated_prepare_time_ms: estimated_ms,
                created_at: Utc::now(),
            };

            self.prefetcher.cache_result(result.clone());
            results.add(result);
            results.critical_path.push(id);
        }

        results
    }
}

impl Default for ProactiveService {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
pub async fn proactive_list_suggestions(
    state: State<'_, AppState>,
) -> Result<Vec<ProactiveSuggestion>, String> {
    let service = state.proactive_service.read().await;
    Ok(service.get_suggestions())
}

#[tauri::command]
pub async fn proactive_refresh_suggestions(
    state: State<'_, AppState>,
    context: serde_json::Value,
) -> Result<Vec<ProactiveSuggestion>, String> {
    let features: ContextFeatures = serde_json::from_value(context)
        .map_err(|e| format!("Failed to parse context features: {}", e))?;

    let mut service = state.proactive_service.write().await;
    Ok(service.refresh_suggestions(features))
}

#[tauri::command]
pub async fn proactive_predict(
    state: State<'_, AppState>,
    context: serde_json::Value,
) -> Result<PredictionResult, String> {
    let features: ContextFeatures = serde_json::from_value(context)
        .map_err(|e| format!("Failed to parse context features: {}", e))?;

    let service = state.proactive_service.read().await;
    Ok(service.predict(features))
}

#[tauri::command]
pub async fn proactive_list_reminders(state: State<'_, AppState>) -> Result<Vec<Reminder>, String> {
    let service = state.proactive_service.read().await;
    Ok(service.get_reminders())
}

#[tauri::command]
pub async fn proactive_dismiss_suggestion(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.dismiss_suggestion(&id))
}

#[tauri::command]
pub async fn proactive_accept_suggestion(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.accept_suggestion(&id))
}

#[tauri::command]
pub async fn proactive_snooze_suggestion(
    state: State<'_, AppState>,
    id: String,
    duration: i64,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.snooze_suggestion(&id, duration))
}

#[tauri::command]
pub async fn proactive_add_reminder(
    state: State<'_, AppState>,
    reminder: serde_json::Value,
) -> Result<Reminder, String> {
    #[derive(Deserialize)]
    struct ReminderInput {
        title: String,
        description: String,
        #[serde(rename = "scheduled_at")]
        scheduled_at: String,
        recurrence: Option<ReminderRecurrenceInput>,
    }

    #[derive(Deserialize)]
    struct ReminderRecurrenceInput {
        frequency: String,
        interval: u32,
    }

    let input: ReminderInput = serde_json::from_value(reminder)
        .map_err(|e| format!("Failed to parse reminder input: {}", e))?;

    let scheduled_at = DateTime::parse_from_rfc3339(&input.scheduled_at)
        .map_err(|e| format!("Invalid scheduled_at format: {}", e))?
        .with_timezone(&Utc);

    let recurrence = match input.recurrence {
        Some(r) => {
            let frequency = match r.frequency.as_str() {
                "daily" => axagent_trajectory::RecurrenceFrequency::Daily,
                "weekly" => axagent_trajectory::RecurrenceFrequency::Weekly,
                "monthly" => axagent_trajectory::RecurrenceFrequency::Monthly,
                _ => return Err(format!("Invalid recurrence frequency: {}", r.frequency)),
            };
            Some(ReminderRecurrence {
                frequency,
                interval: r.interval,
            })
        },
        None => None,
    };

    let traj_reminder = TrajReminder {
        id: uuid::Uuid::new_v4().to_string(),
        title: input.title,
        description: input.description,
        scheduled_at,
        recurrence,
        completed: false,
        created_at: Utc::now(),
    };

    let reminder_result = Reminder::from(&traj_reminder);

    let mut service = state.proactive_service.write().await;
    service.add_reminder(traj_reminder);

    Ok(reminder_result)
}

#[tauri::command]
pub async fn proactive_delete_reminder(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.delete_reminder(&id))
}

#[tauri::command]
pub async fn proactive_complete_reminder(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.complete_reminder(&id))
}

#[tauri::command]
pub async fn proactive_set_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    service.set_enabled(enabled);
    Ok(true)
}

#[tauri::command]
pub async fn proactive_update_config(
    state: State<'_, AppState>,
    config: ProactiveConfig,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    service.update_config(config.into());
    Ok(true)
}

#[tauri::command]
pub async fn proactive_prefetch(
    state: State<'_, AppState>,
    predictions: Vec<serde_json::Value>,
) -> Result<PrefetchResults, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.prefetch_for_predictions(&predictions))
}

#[tauri::command]
pub async fn list_insights(
    state: State<'_, AppState>,
    category: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<axagent_trajectory::LearningInsight>, String> {
    let is = state.insight_system.read().await;
    let insights = match category.as_deref() {
        Some("pattern") => {
            is.get_insights_by_category(axagent_trajectory::InsightCategory::Pattern)
        },
        Some("preference") => {
            is.get_insights_by_category(axagent_trajectory::InsightCategory::Preference)
        },
        Some("improvement") => {
            is.get_insights_by_category(axagent_trajectory::InsightCategory::Improvement)
        },
        Some("warning") => {
            is.get_insights_by_category(axagent_trajectory::InsightCategory::Warning)
        },
        _ => is.get_insights().iter().collect(),
    };
    let limit = limit.unwrap_or(50);
    let result: Vec<_> = insights.into_iter().take(limit).cloned().collect();
    Ok(result)
}
