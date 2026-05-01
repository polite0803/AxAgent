use crate::adaptation::{ContentFormat, TechnicalLevel, Verbosity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub coding_style: CodingStyleProfile,
    pub communication: CommunicationProfile,
    pub work_habits: WorkHabitProfile,
    pub domain_knowledge: DomainKnowledgeProfile,
    pub learning_state: LearningState,
    #[serde(default)]
    pub preferences: Vec<String>,
    #[serde(default)]
    pub expertise: Vec<String>,
}

impl UserProfile {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: "default".to_string(),
            created_at: now,
            updated_at: now,
            coding_style: CodingStyleProfile::default(),
            communication: CommunicationProfile::default(),
            work_habits: WorkHabitProfile::default(),
            domain_knowledge: DomainKnowledgeProfile::default(),
            learning_state: LearningState::default(),
            preferences: Vec::new(),
            expertise: Vec::new(),
        }
    }

    pub fn with_user_id(user_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            created_at: now,
            updated_at: now,
            coding_style: CodingStyleProfile::default(),
            communication: CommunicationProfile::default(),
            work_habits: WorkHabitProfile::default(),
            domain_knowledge: DomainKnowledgeProfile::default(),
            learning_state: LearningState::default(),
            preferences: Vec::new(),
            expertise: Vec::new(),
        }
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = Utc::now();
    }

    pub fn to_user_md(&self) -> String {
        let mut md = String::new();
        md.push_str("# User Profile\n\n");
        md.push_str("## Basic Info\n");
        md.push_str(&format!("- ID: {}\n", self.id));
        md.push_str(&format!("- User ID: {}\n", self.user_id));
        md.push_str(&format!("- Created: {}\n", self.created_at));
        md.push_str(&format!("- Updated: {}\n\n", self.updated_at));

        md.push_str("## Coding Style\n");
        md.push_str(&format!(
            "- Naming Convention: {:?}\n",
            self.coding_style.naming_convention
        ));
        md.push_str(&format!(
            "- Indentation: {:?}\n",
            self.coding_style.indentation_style
        ));
        md.push_str(&format!(
            "- Comment Style: {:?}\n",
            self.coding_style.comment_style
        ));
        md.push_str(&format!(
            "- Module Org: {:?}\n",
            self.coding_style.module_organization
        ));
        md.push_str(&format!(
            "- Confidence: {:.2}\n\n",
            self.coding_style.confidence
        ));

        md.push_str("## Communication\n");
        md.push_str(&format!(
            "- Detail Level: {:?}\n",
            self.communication.detail_level
        ));
        md.push_str(&format!("- Tone: {:?}\n", self.communication.tone));
        md.push_str(&format!("- Language: {}\n", self.communication.language));
        md.push_str(&format!(
            "- Confidence: {:.2}\n\n",
            self.communication.confidence
        ));

        md.push_str("## Preferences\n");
        for pref in &self.preferences {
            md.push_str(&format!("- {}\n", pref));
        }
        md.push_str("\n## Expertise\n");
        for exp in &self.expertise {
            md.push_str(&format!("- {}\n", exp));
        }
        md
    }

    pub fn from_user_md(content: &str) -> Option<Self> {
        let mut profile = UserProfile::new();
        let mut in_coding_style = false;
        let mut in_communication = false;
        let mut in_preferences = false;
        let mut in_expertise = false;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("## Coding Style") {
                in_coding_style = true;
                in_communication = false;
                in_preferences = false;
                in_expertise = false;
                continue;
            } else if line.starts_with("## Communication") {
                in_coding_style = false;
                in_communication = true;
                in_preferences = false;
                in_expertise = false;
                continue;
            } else if line.starts_with("## Preferences") {
                in_coding_style = false;
                in_communication = false;
                in_preferences = true;
                in_expertise = false;
                continue;
            } else if line.starts_with("## Expertise") {
                in_coding_style = false;
                in_communication = false;
                in_preferences = false;
                in_expertise = true;
                continue;
            }

            if line.starts_with("- ") {
                let value = line.trim_start_matches("- ");
                if in_coding_style {
                    if value.starts_with("Naming Convention:") {
                        let _v = value.trim_start_matches("Naming Convention:");
                    } else if value.starts_with("Confidence:") {
                    }
                } else if in_communication {
                    if value.starts_with("Language:") {
                        let lang = value.trim_start_matches("Language:").trim();
                        profile.communication.language = lang.to_string();
                    }
                } else if in_preferences {
                    profile.preferences.push(value.to_string());
                } else if in_expertise {
                    profile.expertise.push(value.to_string());
                }
            }
        }
        Some(profile)
    }

    pub fn format_for_prompt(&self) -> String {
        let mut md = String::new();
        md.push_str("## User Profile\n\n");

        md.push_str("### Coding Style:\n");
        md.push_str(&format!(
            "- Naming: {:?}\n",
            self.coding_style.naming_convention
        ));
        md.push_str(&format!(
            "- Indentation: {:?}\n",
            self.coding_style.indentation_style
        ));
        md.push_str(&format!(
            "- Comments: {:?}\n",
            self.coding_style.comment_style
        ));
        md.push('\n');

        md.push_str("### Communication:\n");
        md.push_str(&format!(
            "- Detail Level: {:?}\n",
            self.communication.detail_level
        ));
        md.push_str(&format!("- Tone: {:?}\n", self.communication.tone));
        md.push_str(&format!("- Language: {}\n", self.communication.language));
        md.push('\n');

        md.push_str("### Preferences:\n");
        for pref in &self.preferences {
            md.push_str(&format!("- {}\n", pref));
        }
        md.push('\n');

        md.push_str("### Expertise:\n");
        for exp in &self.expertise {
            md.push_str(&format!("- {}\n", exp));
        }

        md
    }

    pub fn update_style(
        &mut self,
        verbosity: Verbosity,
        technical_level: TechnicalLevel,
        format: ContentFormat,
    ) {
        // Update communication profile based on the new style settings
        match verbosity {
            Verbosity::Shorter => {
                self.communication.detail_level = DetailLevel::Minimal;
                self.communication.response_length_pref = ResponseLength::Short;
            },
            Verbosity::Longer => {
                self.communication.detail_level = DetailLevel::Comprehensive;
                self.communication.response_length_pref = ResponseLength::Long;
            },
            Verbosity::Unchanged => {},
        }

        match technical_level {
            TechnicalLevel::Simpler => {
                self.communication.explanation_depth = ExplanationDepth::Brief;
            },
            TechnicalLevel::MoreDetailed => {
                self.communication.explanation_depth = ExplanationDepth::Detailed;
            },
            TechnicalLevel::Unchanged => {},
        }

        match format {
            ContentFormat::List => {
                self.communication.format_preference.use_bullets = true;
            },
            ContentFormat::Paragraph => {
                self.communication.format_preference.use_bullets = false;
            },
            ContentFormat::Code => {
                self.communication.format_preference.include_code_blocks = true;
            },
            ContentFormat::Unchanged => {},
        }

        self.update_timestamp();
    }

    pub fn set_preference(&mut self, key: String, value: String) {
        self.preferences.push(format!("{}: {}", key, value));
        self.update_timestamp();
    }

    pub fn set_expertise(&mut self, area: String, level: ExpertiseLevel) {
        self.expertise.push(format!("{}: {:?}", area, level));
        self.update_timestamp();
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CodingStyleProfile {
    pub naming_convention: NamingConvention,
    pub code_patterns: Vec<CodePattern>,
    pub framework_preferences: Vec<String>,
    pub indentation_style: IndentationStyle,
    pub comment_style: CommentStyle,
    pub module_organization: ModuleOrgStyle,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NamingConvention {
    CamelCase,
    #[default]
    SnakeCase,
    PascalCase,
    KebabCase,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodePattern {
    pub pattern_type: String,
    pub pattern: String,
    pub usage_count: u32,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum IndentationStyle {
    TwoSpaces,
    #[default]
    FourSpaces,
    Tabs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommentStyle {
    Minimal,
    #[default]
    Moderate,
    Extensive,
    DocBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModuleOrgStyle {
    Monolithic,
    #[default]
    Modular,
    Layered,
    FeatureBased,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CommunicationProfile {
    pub detail_level: DetailLevel,
    pub tone: Tone,
    pub format_preference: FormatPreference,
    pub language: String,
    pub response_length_pref: ResponseLength,
    pub explanation_depth: ExplanationDepth,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    Minimal,
    #[default]
    Moderate,
    Comprehensive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Formal,
    #[default]
    Neutral,
    Casual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FormatPreference {
    pub use_markdown: bool,
    pub use_bullets: bool,
    pub use_headings: bool,
    pub include_code_blocks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ResponseLength {
    Short,
    #[default]
    Medium,
    Long,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExplanationDepth {
    Brief,
    #[default]
    Standard,
    Detailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WorkHabitProfile {
    pub active_hours: TimeRange,
    pub task_preferences: Vec<LearningTaskType>,
    pub tool_usage_patterns: Vec<ToolUsagePattern>,
    pub workflow_preference: WorkflowPreference,
    pub context_switch_tolerance: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: String,
}

impl Default for TimeRange {
    fn default() -> Self {
        Self {
            start_hour: 9,
            end_hour: 17,
            timezone: "UTC".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LearningTaskType {
    Coding,
    Debugging,
    Documentation,
    Research,
    Design,
    Review,
    Testing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUsagePattern {
    pub tool_name: String,
    pub usage_count: u32,
    pub avg_duration_ms: u64,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowPreference {
    #[default]
    Sequential,
    Parallel,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainKnowledgeProfile {
    pub expertise_areas: Vec<ExpertiseArea>,
    pub interest_topics: Vec<String>,
    pub skill_levels: HashMap<String, SkillLevel>,
    pub recent_topics: Vec<RecentTopic>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseArea {
    pub name: String,
    pub level: SkillLevel,
    pub years_experience: u32,
    pub last_applied: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SkillLevel {
    Beginner,
    #[default]
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExpertiseLevel {
    Beginner,
    Novice,
    #[default]
    Intermediate,
    Advanced,
    Expert,
    Master,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentTopic {
    pub topic: String,
    pub frequency: u32,
    pub last_discussed: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LearningState {
    pub total_interactions: u64,
    pub last_updated: DateTime<Utc>,
    pub learning_version: u32,
    pub stability_score: f32,
    pub freshness_score: f32,
    pub explicitly_set: Vec<String>,
}

impl LearningState {
    pub fn new() -> Self {
        Self {
            total_interactions: 0,
            last_updated: Utc::now(),
            learning_version: 1,
            stability_score: 0.0,
            freshness_score: 1.0,
            explicitly_set: Vec::new(),
        }
    }

    pub fn increment_interactions(&mut self) {
        self.total_interactions += 1;
        self.last_updated = Utc::now();
    }

    pub fn add_explicit_setting(&mut self, key: String) {
        if !self.explicitly_set.contains(&key) {
            self.explicitly_set.push(key);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileUpdate {
    pub field_changed: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub confidence_change: f32,
    pub source: UpdateSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateSource {
    Explicit,
    Inferred,
    UserFeedback,
}

pub fn calculate_confidence(sample_count: u32, time_span_hours: u64) -> f32 {
    let count_factor = (sample_count as f32).min(100.0) / 100.0;
    let time_factor = if time_span_hours > 0 {
        ((time_span_hours as f32).clamp(1.0, 168.0) / 168.0)
    } else {
        1.0
    };
    (count_factor * 0.6 + time_factor * 0.4).min(1.0)
}
