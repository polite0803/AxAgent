use crate::proactive_assistant::{
    ContextPrediction, ContextWindow, PredictedIntent, Priority, SuggestedAction,
};
use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFeatures {
    pub current_file: Option<String>,
    pub current_language: Option<String>,
    pub recent_actions: Vec<ActionType>,
    pub time_of_day: u32,
    pub day_of_week: String,
    pub project_type: Option<String>,
    pub user_activity_level: ActivityLevel,
    pub detected_errors: Vec<String>,
    pub detected_patterns: Vec<PatternMatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    FileOpened,
    FileEdited,
    FileSaved,
    CommandExecuted,
    SearchPerformed,
    ToolUsed,
    ErrorEncountered,
    CodeGenerated,
    DocumentationViewed,
    TestRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern_type: String,
    pub matched_text: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    pub predictions: Vec<ContextPrediction>,
    pub top_prediction: Option<ContextPrediction>,
}

pub struct ContextPredictor {
    rules: Vec<PredictionRule>,
    pattern_weights: HashMap<String, f32>,
}

impl Default for ContextPredictor {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextPredictor {
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
            pattern_weights: Self::default_weights(),
        }
    }

    fn default_rules() -> Vec<PredictionRule> {
        vec![
            PredictionRule {
                name: "file_open_suggests_completion".to_string(),
                condition: PredictionCondition::RecentAction {
                    action: ActionType::FileOpened,
                    within_seconds: 30,
                },
                intent: PredictedIntent::CodeCompletion {
                    language: "unknown".to_string(),
                    context: "new_file".to_string(),
                },
                base_confidence: 0.7,
                reasoning: "User opened a new file, likely needs code completion".to_string(),
            },
            PredictionRule {
                name: "error_detected_suggests_debug".to_string(),
                condition: PredictionCondition::ErrorDetected,
                intent: PredictedIntent::Debug {
                    error: "detected".to_string(),
                },
                base_confidence: 0.85,
                reasoning: "An error was detected in the current context".to_string(),
            },
            PredictionRule {
                name: "refactor_keyword".to_string(),
                condition: PredictionCondition::KeywordInContext {
                    keywords: vec![
                        "refactor".to_string(),
                        "improve".to_string(),
                        "clean".to_string(),
                    ],
                },
                intent: PredictedIntent::Refactoring {
                    target: "current".to_string(),
                },
                base_confidence: 0.75,
                reasoning: "User mentioned refactoring in context".to_string(),
            },
            PredictionRule {
                name: "test_keyword".to_string(),
                condition: PredictionCondition::KeywordInContext {
                    keywords: vec![
                        "test".to_string(),
                        "spec".to_string(),
                        "coverage".to_string(),
                    ],
                },
                intent: PredictedIntent::TestGeneration {
                    target: "current".to_string(),
                },
                base_confidence: 0.7,
                reasoning: "User mentioned testing in context".to_string(),
            },
            PredictionRule {
                name: "doc_keyword".to_string(),
                condition: PredictionCondition::KeywordInContext {
                    keywords: vec![
                        "document".to_string(),
                        "doc".to_string(),
                        "readme".to_string(),
                    ],
                },
                intent: PredictedIntent::Documentation {
                    topic: "current".to_string(),
                },
                base_confidence: 0.65,
                reasoning: "User mentioned documentation in context".to_string(),
            },
            PredictionRule {
                name: "search_keyword".to_string(),
                condition: PredictionCondition::RecentAction {
                    action: ActionType::SearchPerformed,
                    within_seconds: 60,
                },
                intent: PredictedIntent::Search {
                    query_type: "general".to_string(),
                },
                base_confidence: 0.6,
                reasoning: "User performed a search recently".to_string(),
            },
        ]
    }

    fn default_weights() -> HashMap<String, f32> {
        let mut weights = HashMap::new();
        weights.insert("error_confidence".to_string(), 1.5);
        weights.insert("recency_boost".to_string(), 1.2);
        weights.insert("repetition_boost".to_string(), 1.1);
        weights
    }

    pub fn predict(&self, features: &ContextFeatures) -> PredictionResult {
        let mut scored_predictions: Vec<(ContextPrediction, f32)> = Vec::new();

        for rule in &self.rules {
            if let Some((intent, confidence, reasoning)) = self.evaluate_rule(rule, features) {
                let prediction = ContextPrediction {
                    predicted_intent: intent,
                    confidence,
                    reasoning,
                    suggested_actions: self.generate_suggested_actions(features, confidence),
                    context_window: self.build_context_window(features),
                    created_at: chrono::Utc::now(),
                };
                scored_predictions.push((prediction, confidence));
            }
        }

        scored_predictions
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let predictions: Vec<ContextPrediction> = scored_predictions
            .into_iter()
            .map(|(pred, _)| pred)
            .collect();

        let top_prediction = predictions.first().cloned();

        PredictionResult {
            predictions,
            top_prediction,
        }
    }

    fn evaluate_rule(
        &self,
        rule: &PredictionRule,
        features: &ContextFeatures,
    ) -> Option<(PredictedIntent, f32, String)> {
        let condition_met = match &rule.condition {
            PredictionCondition::RecentAction {
                action,
                within_seconds,
            } => self.has_recent_action(features, *action, *within_seconds),
            PredictionCondition::ErrorDetected => !features.detected_errors.is_empty(),
            PredictionCondition::KeywordInContext { keywords } => {
                self.has_keywords(features, keywords)
            },
            PredictionCondition::ActivityLevel { level } => features.user_activity_level == *level,
            PredictionCondition::TimeInRange { start, end } => {
                features.time_of_day >= *start && features.time_of_day <= *end
            },
            PredictionCondition::Always => true,
        };

        if condition_met {
            let confidence = rule.base_confidence * self.calculate_confidence_boost(features);
            Some((
                self.instantiate_intent(&rule.intent, features),
                confidence.min(0.99),
                rule.reasoning.clone(),
            ))
        } else {
            None
        }
    }

    fn has_recent_action(
        &self,
        features: &ContextFeatures,
        action: ActionType,
        _within_seconds: u32,
    ) -> bool {
        features.recent_actions.contains(&action)
    }

    fn has_keywords(&self, features: &ContextFeatures, keywords: &[String]) -> bool {
        if let Some(ref current_file) = features.current_file {
            for keyword in keywords {
                if current_file
                    .to_lowercase()
                    .contains(&keyword.to_lowercase())
                {
                    return true;
                }
            }
        }
        false
    }

    fn calculate_confidence_boost(&self, features: &ContextFeatures) -> f32 {
        let mut boost = 1.0;

        if !features.detected_errors.is_empty() {
            boost *= self
                .pattern_weights
                .get("error_confidence")
                .copied()
                .unwrap_or(1.0);
        }

        if matches!(features.user_activity_level, ActivityLevel::High) {
            boost *= self
                .pattern_weights
                .get("recency_boost")
                .copied()
                .unwrap_or(1.0);
        }

        boost
    }

    fn instantiate_intent(
        &self,
        intent: &PredictedIntent,
        features: &ContextFeatures,
    ) -> PredictedIntent {
        match intent {
            PredictedIntent::CodeCompletion {
                language: _,
                context: _,
            } => PredictedIntent::CodeCompletion {
                language: features
                    .current_language
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                context: features
                    .current_file
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            },
            PredictedIntent::Debug { error: _ } => PredictedIntent::Debug {
                error: features
                    .detected_errors
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
            },
            _ => intent.clone(),
        }
    }

    fn generate_suggested_actions(
        &self,
        features: &ContextFeatures,
        confidence: f32,
    ) -> Vec<SuggestedAction> {
        let mut actions = Vec::new();

        let priority = if confidence > 0.8 {
            Priority::High
        } else if confidence > 0.6 {
            Priority::Medium
        } else {
            Priority::Low
        };

        actions.push(SuggestedAction {
            action_type: "show_completion".to_string(),
            title: "Code Completion".to_string(),
            description: "Get intelligent code completion suggestions".to_string(),
            priority,
        });

        if !features.detected_errors.is_empty() {
            actions.push(SuggestedAction {
                action_type: "debug_assistance".to_string(),
                title: "Debug Help".to_string(),
                description: "Analyze and fix detected errors".to_string(),
                priority: Priority::High,
            });
        }

        actions
    }

    fn build_context_window(&self, features: &ContextFeatures) -> ContextWindow {
        ContextWindow {
            files: features.current_file.iter().cloned().collect(),
            recent_actions: features
                .recent_actions
                .iter()
                .map(|a| format!("{:?}", a))
                .collect(),
            current_language: features.current_language.clone(),
            project_type: features.project_type.clone(),
        }
    }

    pub fn extract_features_from_context(
        &self,
        current_file: Option<String>,
        recent_actions: Vec<String>,
        language: Option<String>,
    ) -> ContextFeatures {
        let now = Utc::now();
        let time_of_day = now.hour();

        let day_names = [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ];
        let day_of_week = day_names[now.weekday().num_days_from_sunday() as usize].to_string();

        let detected_errors: Vec<String> = recent_actions
            .iter()
            .filter(|a| a.to_lowercase().contains("error"))
            .cloned()
            .collect();

        let detected_patterns: Vec<PatternMatch> = if let Some(ref file) = current_file {
            self.detect_file_patterns(file)
        } else {
            Vec::new()
        };

        ContextFeatures {
            current_file,
            current_language: language,
            recent_actions: self.parse_action_types(&recent_actions),
            time_of_day,
            day_of_week,
            project_type: None,
            user_activity_level: self.estimate_activity_level(&recent_actions),
            detected_errors,
            detected_patterns,
        }
    }

    fn parse_action_types(&self, actions: &[String]) -> Vec<ActionType> {
        actions
            .iter()
            .filter_map(|a| match a.to_lowercase().as_str() {
                "file_opened" => Some(ActionType::FileOpened),
                "file_edited" => Some(ActionType::FileEdited),
                "file_saved" => Some(ActionType::FileSaved),
                "command_executed" => Some(ActionType::CommandExecuted),
                "search_performed" => Some(ActionType::SearchPerformed),
                "tool_used" => Some(ActionType::ToolUsed),
                "error_encountered" => Some(ActionType::ErrorEncountered),
                "code_generated" => Some(ActionType::CodeGenerated),
                "documentation_viewed" => Some(ActionType::DocumentationViewed),
                "test_run" => Some(ActionType::TestRun),
                _ => None,
            })
            .collect()
    }

    fn estimate_activity_level(&self, actions: &[String]) -> ActivityLevel {
        let action_count = actions.len();
        if action_count < 3 {
            ActivityLevel::Low
        } else if action_count < 10 {
            ActivityLevel::Medium
        } else {
            ActivityLevel::High
        }
    }

    fn detect_file_patterns(&self, file: &str) -> Vec<PatternMatch> {
        let mut patterns = Vec::new();
        let file_lower = file.to_lowercase();

        if file_lower.ends_with(".test.ts") || file_lower.ends_with(".spec.ts") {
            patterns.push(PatternMatch {
                pattern_type: "test_file".to_string(),
                matched_text: file.to_string(),
                confidence: 0.9,
            });
        }

        if file_lower.contains("component") {
            patterns.push(PatternMatch {
                pattern_type: "component".to_string(),
                matched_text: file.to_string(),
                confidence: 0.8,
            });
        }

        if file_lower.ends_with(".md") {
            patterns.push(PatternMatch {
                pattern_type: "documentation".to_string(),
                matched_text: file.to_string(),
                confidence: 0.95,
            });
        }

        patterns
    }

    pub fn add_rule(&mut self, rule: PredictionRule) {
        self.rules.push(rule);
    }

    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRule {
    pub name: String,
    pub condition: PredictionCondition,
    pub intent: PredictedIntent,
    pub base_confidence: f32,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PredictionCondition {
    RecentAction {
        action: ActionType,
        within_seconds: u32,
    },
    ErrorDetected,
    KeywordInContext {
        keywords: Vec<String>,
    },
    ActivityLevel {
        level: ActivityLevel,
    },
    TimeInRange {
        start: u32,
        end: u32,
    },
    Always,
}
