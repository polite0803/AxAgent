// SPDX-License-Identifier: AGPL-3.0-only
//! User Profile 契约
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpertiseLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
    Master,
}
impl ExpertiseLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Beginner => "beginner",
            Self::Intermediate => "intermediate",
            Self::Advanced => "advanced",
            Self::Expert => "expert",
            Self::Master => "master",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NamingConvention {
    CamelCase,
    SnakeCase,
    PascalCase,
    KebabCase,
}
impl NamingConvention {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CamelCase => "camelCase",
            Self::SnakeCase => "snake_case",
            Self::PascalCase => "PascalCase",
            Self::KebabCase => "kebab-case",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndentationStyle {
    Spaces(u8),
    Tabs,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommentStyle {
    DocComment,
    Inline,
    Minimal,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Tone {
    Formal,
    Casual,
    Technical,
    Friendly,
}
impl Tone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Formal => "formal",
            Self::Casual => "casual",
            Self::Technical => "technical",
            Self::Friendly => "friendly",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStyleProfile {
    pub naming_convention: NamingConvention,
    pub indentation: IndentationStyle,
    pub comment_style: CommentStyle,
    pub preferred_languages: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationProfile {
    pub tone: Tone,
    pub detail_level: u8,
    pub prefers_code_examples: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkHabitProfile {
    pub active_hours: Vec<u8>,
    pub preferred_tools: Vec<String>,
    pub avg_session_length_min: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainKnowledgeProfile {
    pub domains: Vec<String>,
    pub expertise_level: ExpertiseLevel,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningState {
    pub skills_learned: Vec<String>,
    pub patterns_discovered: Vec<String>,
    pub areas_for_improvement: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub coding_style: CodingStyleProfile,
    pub communication: CommunicationProfile,
    pub work_habits: WorkHabitProfile,
    pub domain_knowledge: Vec<DomainKnowledgeProfile>,
    pub learning: LearningState,
    pub last_updated: i64,
}
impl UserProfile {
    pub fn format_for_prompt(&self) -> String {
        format!(
            "User Profile ({}):\n- Coding: {}\n- Communication: {}\n- Learning: {} skills",
            self.user_id,
            self.coding_style.naming_convention.as_str(),
            self.communication.tone.as_str(),
            self.learning.skills_learned.len()
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileUpdate {
    pub field: String,
    pub value: serde_json::Value,
}

#[async_trait]
pub trait UserProfileService: Send + Sync {
    async fn get_profile(&self) -> Result<UserProfile, String>;
    async fn update_profile(&self, update: ProfileUpdate) -> Result<(), String>;
    async fn reset_profile(&self) -> Result<(), String>;
}
