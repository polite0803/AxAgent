//! Skill matching module
//!
//! Replaces TypeScript `skillMatcher.ts` with Rust implementation.
//! Provides skill matching based on keyword patterns and text similarity.

use crate::skill::Skill;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMatch {
    pub skill: MatchedSkill,
    #[serde(rename = "matchScore")]
    pub match_score: f64,
    #[serde(rename = "matchReasons")]
    pub match_reasons: Vec<String>,
    pub source: MatchSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchSource {
    Installed,
    Marketplace,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingResult {
    pub matches: Vec<SkillMatch>,
    #[serde(rename = "bestMatch")]
    pub best_match: Option<SkillMatch>,
    #[serde(rename = "needsMarketplaceSearch")]
    pub needs_marketplace_search: bool,
    #[serde(rename = "suggestedMarketplaceSkills")]
    pub suggested_marketplace_skills: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Complexity {
    Low,
    Medium,
    High,
}

struct KeywordPatterns;

impl KeywordPatterns {
    fn get() -> HashMap<&'static str, Vec<&'static str>> {
        let mut patterns = HashMap::new();
        patterns.insert("pdf", vec!["pdf", "extract", "document", "text extraction"]);
        patterns.insert("docx", vec!["word", "docx", "document", "microsoft word"]);
        patterns.insert(
            "spreadsheet",
            vec!["excel", "spreadsheet", "xlsx", "csv", "table"],
        );
        patterns.insert("browser", vec!["browser", "navigate", "click", "web page", "website"]);
        patterns.insert("github", vec!["github", "git", "repository", "pull request", "issue"]);
        patterns.insert("terminal", vec!["terminal", "shell", "bash", "command", "cli"]);
        patterns.insert("memory", vec!["memory", "remember", "context", "persistent"]);
        patterns.insert("notion", vec!["notion", "notes", "database"]);
        patterns.insert("slack", vec!["slack", "message", "channel", "team"]);
        patterns.insert("email", vec!["email", "mail", "smtp", "imap"]);
        patterns.insert(
            "web-search",
            vec!["search", "web search", "google", "bing", "find information"],
        );
        patterns.insert(
            "code-review",
            vec!["code review", "review code", "security check"],
        );
        patterns.insert(
            "pptx",
            vec!["powerpoint", "presentation", "slides", "pptx"],
        );
        patterns.insert("weather", vec!["weather", "forecast", "temperature"]);
        patterns.insert(
            "sql",
            vec!["sql", "database", "query", "postgresql", "mysql"],
        );
        patterns.insert("image", vec!["image", "resize", "convert", "photo", "picture"]);
        patterns.insert(
            "calendar",
            vec!["calendar", "event", "schedule", "google calendar"],
        );
        patterns.insert("tavily", vec!["tavily", "deep research", "research search"]);
        patterns.insert("brave-search", vec!["brave", "privacy search"]);
        patterns.insert("youtube", vec!["youtube", "video", "transcript", "subtitle"]);
        patterns.insert("obsidian", vec!["obsidian", "markdown", "vault", "notes"]);
        patterns.insert("news", vec!["news", "headlines", "rss", "article"]);
        patterns.insert(
            "api-gateway",
            vec!["api", "rest", "http", "gateway", "test"],
        );
        patterns.insert(
            "desktop-control",
            vec!["desktop", "mouse", "keyboard", "automation"],
        );
        patterns.insert(
            "automation-workflows",
            vec!["automation", "workflow", "trigger", "action"],
        );
        patterns.insert(
            "self-improving",
            vec!["self-improving", "learning", "improve"],
        );
        patterns.insert(
            "skill-creator",
            vec!["create skill", "new skill", "skill creator"],
        );
        patterns.insert(
            "stock-analysis",
            vec!["stock", "finance", "market", "analysis"],
        );
        patterns.insert("humanizer", vec!["humanize", "ai text", "writing"]);
        patterns.insert(
            "data-analysis",
            vec!["data analysis", "visualization", "insights"],
        );
        patterns
    }
}

struct ComplexityIndicators;

#[allow(dead_code)]
impl ComplexityIndicators {
    fn get_high() -> Vec<&'static str> {
        vec![
            "analyze", "compare", "multiple", "complex", "integrate", "migrate", "build system",
        ]
    }

    fn get_medium() -> Vec<&'static str> {
        vec![
            "create", "generate", "process", "convert", "extract", "manage",
        ]
    }

    fn get_low() -> Vec<&'static str> {
        vec!["show", "tell", "what is", "how to", "find", "search"]
    }
}

fn calculate_match_score(
    user_input: &str,
    skill_id: &str,
    skill_name: &str,
    skill_description: &str,
    skill_tags: &[String],
) -> (f64, Vec<String>) {
    let input_lower = user_input.to_lowercase();
    let skill_id_lower = skill_id.to_lowercase();
    let skill_name_lower = skill_name.to_lowercase();
    let reasons: Vec<String> = Vec::new();
    let mut score: f64 = 0.0;

    let patterns = KeywordPatterns::get();

    if skill_id_lower.contains("openclaw-") {
        if let Some(skill_key) = skill_id_lower
            .replace("openclaw-", "")
            .split('-')
            .next()
        {
            if let Some(keywords) = patterns.get(skill_key) {
                for keyword in keywords {
                    if input_lower.contains(keyword) {
                        score += 0.3;
                    }
                }
            }
        }
    }

    for (key, keywords) in &patterns {
        if skill_id_lower.contains(key) || skill_name_lower.contains(key) {
            for keyword in keywords.iter() {
                if input_lower.contains(keyword) {
                    score += 0.25;
                }
            }
        }
    }

    for tag in skill_tags {
        if input_lower.contains(&tag.to_lowercase()) {
            score += 0.2;
        }
    }

    let desc_words: Vec<&str> = skill_description.split_whitespace().take(20).collect();
    for word in desc_words {
        if word.len() > 4 && input_lower.contains(&word.to_lowercase()) {
            score += 0.1;
        }
    }

    score = score.min(1.0);
    (score, reasons)
}

fn estimate_complexity(user_input: &str) -> Complexity {
    let input_lower = user_input.to_lowercase();

    for keyword in ComplexityIndicators::get_high() {
        if input_lower.contains(keyword) {
            return Complexity::High;
        }
    }

    for keyword in ComplexityIndicators::get_medium() {
        if input_lower.contains(keyword) {
            return Complexity::Medium;
        }
    }

    Complexity::Low
}

/// Public wrapper for `estimate_complexity` so external crates can call it.
pub fn estimate_complexity_public(user_input: &str) -> Complexity {
    estimate_complexity(user_input)
}

pub struct SkillMatcher {
    min_score_threshold: f64,
}

impl Default for SkillMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillMatcher {
    pub fn new() -> Self {
        Self {
            min_score_threshold: 0.1,
        }
    }

    pub fn with_threshold(min_score_threshold: f64) -> Self {
        Self {
            min_score_threshold,
        }
    }

    pub fn find_matches(
        &self,
        user_input: &str,
        installed_skills: &[Skill],
    ) -> MatchingResult {
        let mut matches: Vec<SkillMatch> = Vec::new();

        for skill in installed_skills {
            let tags = skill.metadata.hermes.tags.clone();

            let (score, _reasons) = calculate_match_score(
                user_input,
                &skill.id,
                &skill.name,
                &skill.description,
                &tags,
            );

            if score > self.min_score_threshold {
                let matched_skill = MatchedSkill {
                    id: skill.id.clone(),
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                    tags,
                    category: Some(skill.metadata.hermes.category.clone()),
                };

                matches.push(SkillMatch {
                    skill: matched_skill,
                    match_score: score,
                    match_reasons: Vec::new(),
                    source: MatchSource::Installed,
                });
            }
        }

        matches.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap());

        let best_match = matches.first().cloned();
        let complexity = estimate_complexity(user_input);

        let needs_marketplace_search = complexity == Complexity::High && matches.is_empty();

        let suggested_skills = if needs_marketplace_search {
            self.suggest_marketplace_skills(user_input)
        } else {
            Vec::new()
        };

        MatchingResult {
            matches,
            best_match,
            needs_marketplace_search,
            suggested_marketplace_skills: suggested_skills,
        }
    }

    fn suggest_marketplace_skills(&self, user_input: &str) -> Vec<String> {
        let input_lower = user_input.to_lowercase();
        let mut suggestions = Vec::new();

        let patterns = KeywordPatterns::get();

        for (key, keywords) in &patterns {
            for keyword in keywords.iter() {
                if input_lower.contains(keyword) {
                    suggestions.push(key.to_string());
                    break;
                }
            }
        }

        suggestions.truncate(5);
        suggestions
    }

    pub fn get_complexity(&self, user_input: &str) -> Complexity {
        estimate_complexity(user_input)
    }
}

impl Skill {
    pub fn to_matched_skill(&self) -> MatchedSkill {
        MatchedSkill {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            tags: self.metadata.hermes.tags.clone(),
            category: Some(self.metadata.hermes.category.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{HermesMetadata, SkillMetadata};

    fn create_test_skill(id: &str, name: &str, description: &str, tags: Vec<String>) -> Skill {
        let tags_clone = tags.clone();
        Skill {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            version: "1.0.0".to_string(),
            content: "".to_string(),
            category: "general".to_string(),
            tags,
            platforms: vec!["web".to_string()],
            scenarios: vec![],
            quality_score: 0.8,
            success_rate: 0.9,
            avg_execution_time_ms: 100,
            total_usages: 10,
            successful_usages: 9,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_used_at: None,
            metadata: SkillMetadata {
                hermes: HermesMetadata {
                    tags: tags_clone,
                    category: "general".to_string(),
                    fallback_for_toolsets: vec![],
                    requires_toolsets: vec![],
                    config: vec![],
                },
                references: vec![],
            },
        }
    }

    #[test]
    fn test_skill_matching() {
        let matcher = SkillMatcher::new();

        let skills = vec![
            create_test_skill(
                "openclaw-pdf",
                "PDF Extractor",
                "Extract text from PDF documents",
                vec!["pdf".to_string(), "document".to_string()],
            ),
            create_test_skill(
                "openclaw-github",
                "GitHub Assistant",
                "Manage GitHub repositories and pull requests",
                vec!["github".to_string(), "git".to_string()],
            ),
        ];

        let result = matcher.find_matches("Extract text from my PDF file", &skills);

        assert!(!result.matches.is_empty());
        assert_eq!(result.matches[0].skill.id, "openclaw-pdf");
    }

    #[test]
    fn test_no_matches() {
        let matcher = SkillMatcher::new();

        let skills = vec![create_test_skill(
            "openclaw-pdf",
            "PDF Extractor",
            "Extract text from PDF",
            vec![],
        )];

        let result = matcher.find_matches("What is the weather today?", &skills);

        assert!(result.matches.is_empty());
        assert!(result.best_match.is_none());
    }

    #[test]
    fn test_complexity_estimation() {
        let matcher = SkillMatcher::new();

        assert_eq!(
            matcher.get_complexity("Analyze the differences between these files"),
            Complexity::High
        );
        assert_eq!(
            matcher.get_complexity("Create a new document for me"),
            Complexity::Medium
        );
        assert_eq!(
            matcher.get_complexity("What is the weather?"),
            Complexity::Low
        );
    }

    #[test]
    fn test_marketplace_suggestions() {
        let matcher = SkillMatcher::new();

        let skills = vec![];

        let result = matcher.find_matches(
            "I need to analyze multiple complex data sets and compare them",
            &skills,
        );

        assert!(result.needs_marketplace_search);
        assert!(!result.suggested_marketplace_skills.is_empty());
    }
}
