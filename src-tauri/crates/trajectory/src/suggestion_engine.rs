// SPDX-License-Identifier: AGPL-3.0-only

use crate::context_predictor::ContextFeatures;
use crate::proactive_assistant::{
    ContextPrediction, PredictedIntent, Priority, ProactiveSuggestion, SuggestionAction,
    SuggestionType,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferenceProfile {
    pub user_id: String,
    pub coding_style: CodingStylePreference,
    pub communication_style: CommunicationStylePreference,
    pub work_habits: WorkHabitPreference,
    pub learning_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStylePreference {
    pub preferred_language: Option<String>,
    pub documentation_level: DocumentationLevel,
    pub test_creation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentationLevel {
    Minimal,
    Standard,
    Comprehensive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunicationStylePreference {
    pub detail_level: DetailLevel,
    pub tone: CommunicationTone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    Brief,
    Moderate,
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommunicationTone {
    Formal,
    Neutral,
    Casual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkHabitPreference {
    pub peak_hours_start: u32,
    pub peak_hours_end: u32,
    pub multi_tasking_level: u32,
}

pub struct SuggestionEngine {
    config: SuggestionEngineConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionEngineConfig {
    pub max_suggestions: usize,
    pub min_confidence_threshold: f32,
    pub suggestion_ttl_minutes: i64,
    pub personalization_enabled: bool,
    pub habit_based_suggestions: bool,
}

impl Default for SuggestionEngineConfig {
    fn default() -> Self {
        Self {
            max_suggestions: 5,
            min_confidence_threshold: 0.5,
            suggestion_ttl_minutes: 5,
            personalization_enabled: true,
            habit_based_suggestions: true,
        }
    }
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SuggestionEngine {
    pub fn new() -> Self {
        Self { config: SuggestionEngineConfig::default() }
    }

    pub fn with_config(config: SuggestionEngineConfig) -> Self {
        Self { config }
    }

    pub fn generate_suggestions(
        &self,
        context: &ContextFeatures,
        prediction: &ContextPrediction,
        user_profile: Option<&UserPreferenceProfile>,
    ) -> Vec<ProactiveSuggestion> {
        let mut suggestions = Vec::new();

        let predictions_suggestions = self.generate_from_prediction(prediction, context);
        suggestions.extend(predictions_suggestions);

        if self.config.habit_based_suggestions
            && let Some(profile) = user_profile
        {
            let habit_suggestions = self.generate_habit_based_suggestions(context, profile);
            suggestions.extend(habit_suggestions);
        }

        suggestions.sort_by_key(|b| std::cmp::Reverse(b.priority.as_u32()));
        suggestions.truncate(self.config.max_suggestions);

        suggestions
    }

    fn generate_from_prediction(
        &self,
        prediction: &ContextPrediction,
        _context: &ContextFeatures,
    ) -> Vec<ProactiveSuggestion> {
        let mut suggestions = Vec::new();

        if prediction.confidence < self.config.min_confidence_threshold {
            return suggestions;
        }

        let suggestion_type = match &prediction.predicted_intent {
            PredictedIntent::CodeCompletion { language, .. } => {
                self.create_completion_suggestion(language)
            },
            PredictedIntent::Refactoring { target } => self.create_refactor_suggestion(target),
            PredictedIntent::Documentation { topic } => self.create_documentation_suggestion(topic),
            PredictedIntent::TestGeneration { target } => self.create_test_suggestion(target),
            PredictedIntent::Debug { error } => self.create_debug_suggestion(error),
            PredictedIntent::Search { .. } => self.create_search_suggestion(),
            PredictedIntent::Unknown => {
                vec![]
            },
        };

        suggestions.extend(suggestion_type);
        suggestions
    }

    fn create_completion_suggestion(&self, language: &str) -> Vec<ProactiveSuggestion> {
        vec![ProactiveSuggestion::new(
            SuggestionType::Completion,
            format!("Complete {} code", language),
            "为您准备代码补全".to_string(),
            SuggestionAction::PrefetchCompletion {
                language: language.to_string(),
                context: "current".to_string(),
            },
            Priority::High,
            self.config.suggestion_ttl_minutes,
        )]
    }

    fn create_refactor_suggestion(&self, target: &str) -> Vec<ProactiveSuggestion> {
        vec![ProactiveSuggestion::new(
            SuggestionType::Refactor,
            format!("Refactor {}", target),
            "检测到潜在的重构机会".to_string(),
            SuggestionAction::ShowRefactorOptions { target: target.to_string() },
            Priority::Medium,
            self.config.suggestion_ttl_minutes,
        )]
    }

    fn create_documentation_suggestion(&self, topic: &str) -> Vec<ProactiveSuggestion> {
        vec![ProactiveSuggestion::new(
            SuggestionType::Documentation,
            format!("Generate docs for {}", topic),
            "为您生成文档".to_string(),
            SuggestionAction::GenerateDocs { topic: topic.to_string() },
            Priority::Low,
            self.config.suggestion_ttl_minutes,
        )]
    }

    fn create_test_suggestion(&self, target: &str) -> Vec<ProactiveSuggestion> {
        vec![ProactiveSuggestion::new(
            SuggestionType::Test,
            format!("Generate tests for {}", target),
            "为您创建测试用例".to_string(),
            SuggestionAction::GenerateTests { target: target.to_string() },
            Priority::Medium,
            self.config.suggestion_ttl_minutes,
        )]
    }

    fn create_debug_suggestion(&self, error: &str) -> Vec<ProactiveSuggestion> {
        vec![ProactiveSuggestion::new(
            SuggestionType::Debug,
            format!("Debug: {}", error),
            "分析并修复错误".to_string(),
            SuggestionAction::ShowOptimizations { target: error.to_string() },
            Priority::High,
            self.config.suggestion_ttl_minutes,
        )]
    }

    fn create_search_suggestion(&self) -> Vec<ProactiveSuggestion> {
        vec![ProactiveSuggestion::new(
            SuggestionType::Learning,
            "Search assistance".to_string(),
            "为您准备搜索建议".to_string(),
            SuggestionAction::ShowLearningResources { topic: "general".to_string() },
            Priority::Low,
            self.config.suggestion_ttl_minutes,
        )]
    }

    fn generate_habit_based_suggestions(
        &self,
        context: &ContextFeatures,
        profile: &UserPreferenceProfile,
    ) -> Vec<ProactiveSuggestion> {
        let mut suggestions = Vec::new();

        let is_peak_hour = context.time_of_day >= profile.work_habits.peak_hours_start
            && context.time_of_day <= profile.work_habits.peak_hours_end;

        if is_peak_hour && profile.coding_style.test_creation && context.recent_actions.is_empty() {
            suggestions.push(ProactiveSuggestion::new(
                SuggestionType::Test,
                "Peak productivity time - Add tests".to_string(),
                "您的高效工作时间，适合添加测试".to_string(),
                SuggestionAction::GenerateTests { target: "current_file".to_string() },
                Priority::Medium,
                self.config.suggestion_ttl_minutes,
            ));
        }

        if profile.coding_style.documentation_level == DocumentationLevel::Comprehensive {
            suggestions.push(ProactiveSuggestion::new(
                SuggestionType::Documentation,
                "Add documentation".to_string(),
                "建议添加详细文档".to_string(),
                SuggestionAction::GenerateDocs { topic: "current".to_string() },
                Priority::Low,
                self.config.suggestion_ttl_minutes,
            ));
        }

        suggestions
    }

    pub fn get_config(&self) -> &SuggestionEngineConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: SuggestionEngineConfig) {
        self.config = config;
    }

    pub fn set_personalization(&mut self, enabled: bool) {
        self.config.personalization_enabled = enabled;
    }

    pub fn set_habit_based(&mut self, enabled: bool) {
        self.config.habit_based_suggestions = enabled;
    }
}
