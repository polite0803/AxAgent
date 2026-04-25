//! User profile and modeling for cross-session personalization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::adaptation::{Verbosity, TechnicalLevel, ContentFormat};

/// Structured user profile built from cross-session interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    /// Explicit preferences (e.g., "language" -> "Python", "editor" -> "VSCode")
    pub preferences: HashMap<String, String>,
    /// Inferred communication style from adaptation signals
    pub communication_style: CommunicationStyle,
    /// Domain expertise levels
    pub expertise: HashMap<String, ExpertiseLevel>,
    /// Stated or inferred goals
    pub goals: Vec<String>,
    /// Behavior patterns detected across sessions
    pub behavior_patterns: Vec<UserBehaviorPattern>,
    /// Last updated timestamp (ms)
    pub updated_at: i64,
}

/// Communication style derived from RealTimeAdaptation signals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CommunicationStyle {
    pub verbosity: Verbosity,
    pub technical_level: TechnicalLevel,
    pub preferred_format: ContentFormat,
    pub preferred_language: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExpertiseLevel {
    #[default]
    Intermediate,
    Beginner,
    Advanced,
    Expert,
}

/// A detected user behavior pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehaviorPattern {
    pub pattern: String,
    pub frequency: u32,
    pub confidence: f64,
    pub last_seen: i64,
}

impl UserProfile {
    pub fn new() -> Self {
        Self {
            id: "default".to_string(),
            preferences: HashMap::new(),
            communication_style: CommunicationStyle::default(),
            expertise: HashMap::new(),
            goals: Vec::new(),
            behavior_patterns: Vec::new(),
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Update communication style from adaptation signals.
    pub fn update_style(
        &mut self,
        verbosity: Verbosity,
        technical_level: TechnicalLevel,
        preferred_format: ContentFormat,
    ) {
        if verbosity != Verbosity::Unchanged {
            self.communication_style.verbosity = verbosity;
        }
        if technical_level != TechnicalLevel::Unchanged {
            self.communication_style.technical_level = technical_level;
        }
        if preferred_format != ContentFormat::Unchanged {
            self.communication_style.preferred_format = preferred_format;
        }
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }

    /// Set a preference.
    pub fn set_preference(&mut self, key: String, value: String) {
        self.preferences.insert(key, value);
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }

    /// Set expertise level for a domain.
    pub fn set_expertise(&mut self, domain: String, level: ExpertiseLevel) {
        self.expertise.insert(domain, level);
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }

    /// Add or update a behavior pattern.
    pub fn record_pattern(&mut self, pattern: String, confidence: f64) {
        let now = chrono::Utc::now().timestamp_millis();
        if let Some(existing) = self.behavior_patterns.iter_mut().find(|p| p.pattern == pattern) {
            existing.frequency += 1;
            existing.confidence = (existing.confidence + confidence) / 2.0;
            existing.last_seen = now;
        } else {
            self.behavior_patterns.push(UserBehaviorPattern {
                pattern,
                frequency: 1,
                confidence,
                last_seen: now,
            });
        }
        self.updated_at = now;
    }

    /// Format the profile for injection into system prompt.
    pub fn format_for_prompt(&self) -> String {
        let mut parts = Vec::new();

        // Communication style
        let style = &self.communication_style;
        let mut style_parts: Vec<String> = Vec::new();
        match style.verbosity {
            Verbosity::Shorter => style_parts.push("prefers concise answers".to_string()),
            Verbosity::Longer => style_parts.push("prefers detailed explanations".to_string()),
            Verbosity::Unchanged => {}
        }
        match style.technical_level {
            TechnicalLevel::Simpler => style_parts.push("prefers simpler explanations".to_string()),
            TechnicalLevel::MoreDetailed => style_parts.push("prefers technical depth".to_string()),
            TechnicalLevel::Unchanged => {}
        }
        match style.preferred_format {
            ContentFormat::List => style_parts.push("prefers list/bullet format".to_string()),
            ContentFormat::Paragraph => style_parts.push("prefers paragraph format".to_string()),
            ContentFormat::Code => style_parts.push("prefers code-first responses".to_string()),
            ContentFormat::Unchanged => {}
        }
        if !style.preferred_language.is_empty() {
            style_parts.push(format!("primary language: {}", style.preferred_language));
        }
        if !style_parts.is_empty() {
            parts.push(format!("Communication: {}", style_parts.join(", ")));
        }

        // Preferences
        if !self.preferences.is_empty() {
            let prefs: Vec<String> = self.preferences.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            parts.push(format!("Preferences: {}", prefs.join(", ")));
        }

        // Expertise
        if !self.expertise.is_empty() {
            let exp: Vec<String> = self.expertise.iter()
                .map(|(d, l)| format!("{} ({:?})", d, l))
                .collect();
            parts.push(format!("Expertise: {}", exp.join(", ")));
        }

        // Goals
        if !self.goals.is_empty() {
            parts.push(format!("Goals: {}", self.goals.join("; ")));
        }

        // Top behavior patterns
        let top_patterns: Vec<_> = self.behavior_patterns.iter()
            .filter(|p| p.confidence >= 0.5 && p.frequency >= 2)
            .take(3)
            .collect();
        if !top_patterns.is_empty() {
            let pats: Vec<String> = top_patterns.iter()
                .map(|p| format!("{} (x{}, {:.0}%)", p.pattern, p.frequency, p.confidence * 100.0))
                .collect();
            parts.push(format!("Patterns: {}", pats.join("; ")));
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join("\n")
        }
    }

    /// Serialize to USER.md format.
    pub fn to_user_md(&self) -> String {
        let mut md = String::new();
        md.push_str("# User Profile\n\n");

        // Communication style
        md.push_str("## Communication Style\n");
        md.push_str(&format!("- Verbosity: {:?}\n", self.communication_style.verbosity));
        md.push_str(&format!("- Technical Level: {:?}\n", self.communication_style.technical_level));
        md.push_str(&format!("- Preferred Format: {:?}\n", self.communication_style.preferred_format));
        if !self.communication_style.preferred_language.is_empty() {
            md.push_str(&format!("- Language: {}\n", self.communication_style.preferred_language));
        }
        md.push('\n');

        // Preferences
        if !self.preferences.is_empty() {
            md.push_str("## Preferences\n");
            for (k, v) in &self.preferences {
                md.push_str(&format!("- {}: {}\n", k, v));
            }
            md.push('\n');
        }

        // Expertise
        if !self.expertise.is_empty() {
            md.push_str("## Expertise\n");
            for (d, l) in &self.expertise {
                md.push_str(&format!("- {}: {:?}\n", d, l));
            }
            md.push('\n');
        }

        // Goals
        if !self.goals.is_empty() {
            md.push_str("## Goals\n");
            for g in &self.goals {
                md.push_str(&format!("- {}\n", g));
            }
            md.push('\n');
        }

        // Behavior patterns
        if !self.behavior_patterns.is_empty() {
            md.push_str("## Behavior Patterns\n");
            for p in &self.behavior_patterns {
                md.push_str(&format!("- {} (frequency={}, confidence={:.2})\n", p.pattern, p.frequency, p.confidence));
            }
        }

        md
    }

    /// Parse from USER.md format (basic parser).
    pub fn from_user_md(content: &str) -> Self {
        let mut profile = Self::new();
        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                continue; // Skip title
            } else if trimmed.starts_with("## ") {
                current_section = trimmed[3..].to_string();
            } else if let Some(value) = trimmed.strip_prefix("- ") {
                match current_section.as_str() {
                    "Preferences" => {
                        if let Some((k, v)) = value.split_once(':') {
                            profile.preferences.insert(k.trim().to_string(), v.trim().to_string());
                        }
                    }
                    "Expertise" => {
                        if let Some((d, l)) = value.split_once(':') {
                            let level = match l.trim() {
                                "Beginner" => ExpertiseLevel::Beginner,
                                "Advanced" => ExpertiseLevel::Advanced,
                                "Expert" => ExpertiseLevel::Expert,
                                _ => ExpertiseLevel::Intermediate,
                            };
                            profile.expertise.insert(d.trim().to_string(), level);
                        }
                    }
                    "Goals" => {
                        profile.goals.push(value.to_string());
                    }
                    "Communication Style" => {
                        if let Some((k, v)) = value.split_once(':') {
                            let v = v.trim();
                            match k.trim() {
                                "Verbosity" => profile.communication_style.verbosity = match v {
                                    "Shorter" => Verbosity::Shorter,
                                    "Longer" => Verbosity::Longer,
                                    _ => Verbosity::Unchanged,
                                },
                                "Technical Level" => profile.communication_style.technical_level = match v {
                                    "Simpler" => TechnicalLevel::Simpler,
                                    "MoreDetailed" => TechnicalLevel::MoreDetailed,
                                    _ => TechnicalLevel::Unchanged,
                                },
                                "Preferred Format" => profile.communication_style.preferred_format = match v {
                                    "List" => ContentFormat::List,
                                    "Paragraph" => ContentFormat::Paragraph,
                                    "Code" => ContentFormat::Code,
                                    _ => ContentFormat::Unchanged,
                                },
                                "Language" => profile.communication_style.preferred_language = v.to_string(),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        profile
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self::new()
    }
}
