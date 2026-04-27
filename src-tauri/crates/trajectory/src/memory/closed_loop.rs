//! Closed-loop learning service
//!
//! Provides proactive nudges and automated learning actions.
//! Replaces TypeScript `ClosedLoopLearning.ts` with Rust implementation.
//! Leverages existing `skill_evolution` module for genetic algorithm-based skill optimization.

use crate::skill::Skill;
use crate::TrajectoryStorage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedLoopConfig {
    pub nudge_interval_ms: u64,
    pub min_confidence_for_auto_action: f64,
    pub consolidation_threshold: f64,
    pub skill_creation_threshold: f64,
}

impl Default for ClosedLoopConfig {
    fn default() -> Self {
        Self {
            nudge_interval_ms: 5 * 60 * 1000,
            min_confidence_for_auto_action: 0.8,
            consolidation_threshold: 0.7,
            skill_creation_threshold: 0.75,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NudgeType {
    MemoryConsolidation,
    SkillCreation,
    PatternLearn,
    ReviewReminder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicNudge {
    pub id: String,
    pub nudge_type: NudgeType,
    pub title: String,
    pub description: String,
    pub suggested_action: String,
    pub urgency: String,
    pub auto_action: Option<AutoAction>,
    pub created_at: i64,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoAction {
    pub action_type: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConsolidationTask {
    pub id: String,
    pub entities: Vec<EntityRef>,
    pub theme: String,
    pub summary: String,
    pub memory_content: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCreationProposal {
    pub task_description: String,
    pub suggested_name: String,
    pub suggested_content: String,
    pub confidence: f64,
    pub trigger_event: String,
    pub scenarios: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpgradeProposal {
    pub target_skill_id: String,
    pub suggested_improvements: String,
    pub additional_scenarios: Vec<String>,
    pub confidence: f64,
    pub trigger_event: String,
}

pub struct ClosedLoopService {
    storage: Arc<TrajectoryStorage>,
    config: ClosedLoopConfig,
    nudges: RwLock<Vec<PeriodicNudge>>,
    is_running: RwLock<bool>,
    skills_dir: Option<PathBuf>,
}

impl ClosedLoopService {
    pub fn new(storage: Arc<TrajectoryStorage>) -> Self {
        Self {
            storage,
            config: ClosedLoopConfig::default(),
            nudges: RwLock::new(Vec::new()),
            is_running: RwLock::new(false),
            skills_dir: None,
        }
    }

    pub fn with_skills_dir(mut self, skills_dir: PathBuf) -> Self {
        self.skills_dir = Some(skills_dir);
        self
    }

    pub fn start(&self) {
        *self.is_running.write().unwrap() = true;
    }

    pub fn stop(&self) {
        *self.is_running.write().unwrap() = false;
    }

    pub fn is_running(&self) -> bool {
        *self.is_running.read().unwrap()
    }

    pub async fn tick(&self) -> Vec<PeriodicNudge> {
        if !self.is_running() {
            return Vec::new();
        }

        let new_nudges = self.generate_nudges().await;
        {
            let mut nudges = self.nudges.write().unwrap();
            nudges.extend(new_nudges.clone());
            if nudges.len() > 50 {
                *nudges = nudges[nudges.len() - 50..].to_vec();
            }
        }

        self.execute_auto_actions(&new_nudges).await;

        new_nudges
    }

    pub fn get_skill_by_id(&self, skill_id: &str) -> Result<Option<Skill>, anyhow::Error> {
        self.storage.get_skill(skill_id)
    }

    pub async fn execute_upgrade_action(&self, auto_action: &AutoAction) {
        if auto_action.action_type == "upgrade_skill" {
            if let Ok(proposal) = serde_json::from_str::<SkillUpgradeProposal>(&auto_action.target)
            {
                if let Ok(Some(mut existing_skill)) =
                    self.storage.get_skill(&proposal.target_skill_id)
                {
                    let now = chrono::Utc::now();

                    let mut updated_content = existing_skill.content.clone();
                    updated_content.push_str("\n\n## Improvement Suggestions\n");
                    updated_content.push_str(&proposal.suggested_improvements);

                    let mut updated_scenarios = existing_skill.scenarios.clone();
                    for scenario in &proposal.additional_scenarios {
                        if !updated_scenarios.contains(scenario) {
                            updated_scenarios.push(scenario.clone());
                        }
                    }

                    let old_version = existing_skill.version.clone();
                    let version_parts: Vec<&str> = old_version.split('.').collect();
                    let new_version = if version_parts.len() >= 2 {
                        let minor: u32 = version_parts[1].parse().unwrap_or(0);
                        format!("{}.{}.0", version_parts[0], minor + 1)
                    } else {
                        "1.1.0".to_string()
                    };

                    existing_skill.content = updated_content;
                    existing_skill.scenarios = updated_scenarios;
                    existing_skill.version = new_version.clone();
                    existing_skill.updated_at = now;

                    if let Err(e) = self.storage.save_skill(&existing_skill) {
                        tracing::warn!("Failed to upgrade skill in storage: {}", e);
                    } else {
                        tracing::info!(
                            "Upgraded skill in storage: {} from {} to {}",
                            existing_skill.name,
                            old_version,
                            new_version
                        );
                    }

                    if let Some(ref skills_dir) = self.skills_dir {
                        let skill_dir = skills_dir.join(&existing_skill.name);
                        let skill_md = format!(
                            "---\nname: {}\ndescription: {}\nversion: {}\nscenarios:\n{}\nmetadata:\n  hermes:\n    tags: [auto-created, upgraded]\n    related_skills: []\n---\n\n{}",
                            existing_skill.name,
                            existing_skill.description,
                            existing_skill.version,
                            existing_skill.scenarios.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("\n"),
                            existing_skill.content
                        );
                        match std::fs::write(skill_dir.join("SKILL.md"), &skill_md) {
                            Ok(_) => {
                                tracing::info!("Updated skill file at {}", skill_dir.display());
                            }
                            Err(e) => {
                                tracing::warn!("Failed to update skill file: {}", e);
                            }
                        }
                    }
                } else {
                    tracing::warn!(
                        "Target skill not found for upgrade: {}",
                        proposal.target_skill_id
                    );
                }
            }
        }
    }

    async fn generate_nudges(&self) -> Vec<PeriodicNudge> {
        let mut all_nudges = Vec::new();

        if let Ok(memory_nudges) = self.evaluate_memory_consolidation().await {
            all_nudges.extend(memory_nudges);
        }

        if let Ok(skill_nudges) = self.evaluate_skill_creation().await {
            all_nudges.extend(skill_nudges);
        }

        if let Ok(pattern_nudges) = self.evaluate_pattern_learning().await {
            all_nudges.extend(pattern_nudges);
        }

        all_nudges
    }

    async fn evaluate_memory_consolidation(&self) -> Result<Vec<PeriodicNudge>, anyhow::Error> {
        let mut nudges = Vec::new();

        let recent_trajectories =
            self.storage
                .query_trajectories(&crate::trajectory::TrajectoryQuery {
                    session_id: None,
                    user_id: None,
                    topic: None,
                    min_quality: Some(0.5),
                    min_value_score: None,
                    outcome: None,
                    time_range: None,
                    limit: Some(10),
                })?;

        if recent_trajectories.len() < 2 {
            return Ok(nudges);
        }

        let theme = self.detect_common_theme(&recent_trajectories);
        if theme.is_none() {
            return Ok(nudges);
        }

        let theme = theme.unwrap();
        let entities = self.collect_entities_from_trajectories(&recent_trajectories);

        if entities.len() < 2 {
            return Ok(nudges);
        }

        let consolidation = self.propose_memory_consolidation(&theme, &entities);
        if consolidation.confidence >= self.config.consolidation_threshold {
            nudges.push(PeriodicNudge {
                id: format!("nudge_mc_{}", chrono::Utc::now().timestamp_millis()),
                nudge_type: NudgeType::MemoryConsolidation,
                title: "Memory Consolidation Suggestion".to_string(),
                description: format!("Detected multiple related entities about \"{}\" that can be consolidated into long-term memory", theme),
                suggested_action: consolidation.memory_content.clone(),
                urgency: if consolidation.confidence >= 0.9 { "high".to_string() } else { "medium".to_string() },
                auto_action: if consolidation.confidence >= self.config.min_confidence_for_auto_action {
                    Some(AutoAction {
                        action_type: "save_to_memory".to_string(),
                        target: consolidation.memory_content,
                    })
                } else {
                    None
                },
                created_at: chrono::Utc::now().timestamp(),
                acknowledged: false,
            });
        }

        Ok(nudges)
    }

    async fn evaluate_skill_creation(&self) -> Result<Vec<PeriodicNudge>, anyhow::Error> {
        let mut nudges = Vec::new();

        let recent_trajectories =
            self.storage
                .query_trajectories(&crate::trajectory::TrajectoryQuery {
                    session_id: None,
                    user_id: None,
                    topic: None,
                    min_quality: None,
                    min_value_score: None,
                    outcome: None,
                    time_range: None,
                    limit: Some(20),
                })?;

        let complex_tasks: Vec<_> = recent_trajectories
            .iter()
            .filter(|t| t.steps.len() > 10)
            .collect();

        for task in complex_tasks.iter().take(5) {
            if let Ok(similar_skills) = self.find_similar_skills(&task.topic) {
                if let Some(similar) = similar_skills.first() {
                    if let Some((upgrade_proposal, _creation_proposal)) =
                        self.propose_skill_improvement(similar, task)
                    {
                        if upgrade_proposal.confidence >= self.config.skill_creation_threshold {
                            nudges.push(PeriodicNudge {
                                id: format!(
                                    "nudge_sc_{}_{}",
                                    chrono::Utc::now().timestamp_millis(),
                                    uuid::Uuid::new_v4()
                                ),
                                nudge_type: NudgeType::SkillCreation,
                                title: "Skill Upgrade Suggestion".to_string(),
                                description: format!(
                                    "Consider upgrading skill \"{}\" with new scenarios and steps",
                                    similar.name
                                ),
                                suggested_action: upgrade_proposal.suggested_improvements.clone(),
                                urgency: if upgrade_proposal.confidence >= 0.9 {
                                    "high".to_string()
                                } else {
                                    "medium".to_string()
                                },
                                auto_action: if upgrade_proposal.confidence
                                    >= self.config.min_confidence_for_auto_action
                                {
                                    Some(AutoAction {
                                        action_type: "upgrade_skill".to_string(),
                                        target: serde_json::to_string(&upgrade_proposal)
                                            .unwrap_or_default(),
                                    })
                                } else {
                                    None
                                },
                                created_at: chrono::Utc::now().timestamp(),
                                acknowledged: false,
                            });
                        }
                    }
                } else {
                    let scenarios = self.extract_scenarios_from_topic(&task.topic);
                    let new_skill_proposal = SkillCreationProposal {
                        task_description: task.topic.clone(),
                        suggested_name: self.generate_skill_name_from_topic(&task.topic),
                        suggested_content: self.generate_skill_content_from_trajectory(task),
                        confidence: 0.5,
                        trigger_event: format!("new_skill:{}", task.topic),
                        scenarios,
                    };

                    nudges.push(PeriodicNudge {
                        id: format!(
                            "nudge_ns_{}_{}",
                            chrono::Utc::now().timestamp_millis(),
                            uuid::Uuid::new_v4()
                        ),
                        nudge_type: NudgeType::SkillCreation,
                        title: "New Skill Suggestion".to_string(),
                        description: format!(
                            "Consider creating skill \"{}\" to handle similar tasks",
                            new_skill_proposal.suggested_name
                        ),
                        suggested_action: new_skill_proposal.suggested_content.clone(),
                        urgency: "medium".to_string(),
                        auto_action: None,
                        created_at: chrono::Utc::now().timestamp(),
                        acknowledged: false,
                    });
                }
            }
        }

        Ok(nudges)
    }

    async fn evaluate_pattern_learning(&self) -> Result<Vec<PeriodicNudge>, anyhow::Error> {
        let mut nudges = Vec::new();

        let patterns = self.storage.get_patterns_by_success_rate(0.0, Some(100))?;
        let high_failure_patterns: Vec<_> = patterns
            .iter()
            .filter(|p| p.success_rate < 0.3 && p.frequency > 3)
            .collect();

        for pattern in high_failure_patterns.iter().take(3) {
            nudges.push(PeriodicNudge {
                id: format!(
                    "nudge_pl_{}_{}",
                    chrono::Utc::now().timestamp_millis(),
                    uuid::Uuid::new_v4()
                ),
                nudge_type: NudgeType::PatternLearn,
                title: "模式学习建议".to_string(),
                description: format!("检测到失败模式\"{}\"，建议调整学习策略", pattern.name),
                suggested_action: format!(
                    "用户倾向于忽略以\"{}\"开头的建议，考虑换一种表达方式",
                    pattern.name
                ),
                urgency: if pattern.frequency > 5 {
                    "high".to_string()
                } else {
                    "low".to_string()
                },
                auto_action: None,
                created_at: chrono::Utc::now().timestamp(),
                acknowledged: false,
            });
        }

        Ok(nudges)
    }

    async fn execute_auto_actions(&self, nudges: &[PeriodicNudge]) {
        for nudge in nudges {
            if let Some(ref auto_action) = nudge.auto_action {
                match auto_action.action_type.as_str() {
                    "save_to_memory" => {
                        let entry = crate::memory::MemoryEntry {
                            id: format!("mem_{}", uuid::Uuid::new_v4()),
                            content: auto_action.target.clone(),
                            memory_type: "memory".to_string(),
                            updated_at: chrono::Utc::now().timestamp(),
                        };
                        if let Err(e) = self.storage.save_memory(&entry) {
                            tracing::warn!("Failed to auto-save memory: {}", e);
                        } else {
                            tracing::info!(
                                "Auto-saved consolidated memory: {:?}",
                                &auto_action.target[..auto_action.target.len().min(50)]
                            );
                        }
                    }
                    "create_skill" => {
                        if let Ok(proposal) =
                            serde_json::from_str::<SkillCreationProposal>(&auto_action.target)
                        {
                            let now = chrono::Utc::now();
                            let skill = Skill {
                                id: uuid::Uuid::new_v4().to_string(),
                                name: proposal.suggested_name.clone(),
                                description: proposal.task_description.clone(),
                                version: "1.0.0".to_string(),
                                content: proposal.suggested_content.clone(),
                                category: "auto_created".to_string(),
                                tags: vec![],
                                platforms: vec![],
                                scenarios: proposal.scenarios.clone(),
                                quality_score: 0.0,
                                success_rate: 0.0,
                                avg_execution_time_ms: 0,
                                total_usages: 0,
                                successful_usages: 0,
                                created_at: now,
                                updated_at: now,
                                last_used_at: None,
                                metadata: crate::skill::SkillMetadata {
                                    hermes: crate::skill::HermesMetadata {
                                        tags: vec![],
                                        category: "auto_created".to_string(),
                                        fallback_for_toolsets: vec![],
                                        requires_toolsets: vec![],
                                        config: vec![],
                                        ..Default::default()
                                    },
                                    references: vec![],
                                },
                            };

                            if let Err(e) = self.storage.save_skill(&skill) {
                                tracing::warn!("Failed to save skill to storage: {}", e);
                            } else {
                                tracing::info!(
                                    "Saved skill to storage: {}",
                                    proposal.suggested_name
                                );
                            }

                            if let Some(ref skills_dir) = self.skills_dir {
                                let skill_dir = skills_dir.join(&skill.name);
                                match std::fs::create_dir_all(&skill_dir) {
                                    Ok(_) => {
                                        let skill_md = format!(
                                            "---\nname: {}\ndescription: {}\nversion: {}\nscenarios:\n{}\nmetadata:\n  hermes:\n    tags: [auto-created]\n    related_skills: []\n---\n\n{}",
                                            skill.name,
                                            skill.description,
                                            skill.version,
                                            skill.scenarios.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("\n"),
                                            skill.content
                                        );
                                        match std::fs::write(skill_dir.join("SKILL.md"), &skill_md)
                                        {
                                            Ok(_) => {
                                                tracing::info!(
                                                    "Created skill file at {}",
                                                    skill_dir.display()
                                                );
                                            }
                                            Err(e) => {
                                                tracing::warn!("Failed to write skill file: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to create skill directory: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    "upgrade_skill" => {
                        if let Ok(proposal) =
                            serde_json::from_str::<SkillUpgradeProposal>(&auto_action.target)
                        {
                            if let Ok(Some(mut existing_skill)) =
                                self.storage.get_skill(&proposal.target_skill_id)
                            {
                                let now = chrono::Utc::now();

                                let mut updated_content = existing_skill.content.clone();
                                updated_content.push_str("\n\n## Improvement Suggestions\n");
                                updated_content.push_str(&proposal.suggested_improvements);

                                let mut updated_scenarios = existing_skill.scenarios.clone();
                                for scenario in &proposal.additional_scenarios {
                                    if !updated_scenarios.contains(scenario) {
                                        updated_scenarios.push(scenario.clone());
                                    }
                                }

                                let old_version = existing_skill.version.clone();
                                let version_parts: Vec<&str> = old_version.split('.').collect();
                                let new_version = if version_parts.len() >= 2 {
                                    let minor: u32 = version_parts[1].parse().unwrap_or(0);
                                    format!("{}.{}.0", version_parts[0], minor + 1)
                                } else {
                                    "1.1.0".to_string()
                                };

                                existing_skill.content = updated_content;
                                existing_skill.scenarios = updated_scenarios;
                                existing_skill.version = new_version.clone();
                                existing_skill.updated_at = now;

                                if let Err(e) = self.storage.save_skill(&existing_skill) {
                                    tracing::warn!("Failed to upgrade skill in storage: {}", e);
                                } else {
                                    tracing::info!(
                                        "Upgraded skill in storage: {} from {} to {}",
                                        existing_skill.name,
                                        old_version,
                                        new_version
                                    );
                                }

                                if let Some(ref skills_dir) = self.skills_dir {
                                    let skill_dir = skills_dir.join(&existing_skill.name);
                                    let skill_md = format!(
                                        "---\nname: {}\ndescription: {}\nversion: {}\nscenarios:\n{}\nmetadata:\n  hermes:\n    tags: [auto-created, upgraded]\n    related_skills: []\n---\n\n{}",
                                        existing_skill.name,
                                        existing_skill.description,
                                        existing_skill.version,
                                        existing_skill.scenarios.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("\n"),
                                        existing_skill.content
                                    );
                                    match std::fs::write(skill_dir.join("SKILL.md"), &skill_md) {
                                        Ok(_) => {
                                            tracing::info!(
                                                "Updated skill file at {}",
                                                skill_dir.display()
                                            );
                                        }
                                        Err(e) => {
                                            tracing::warn!("Failed to update skill file: {}", e);
                                        }
                                    }
                                }
                            } else {
                                tracing::warn!(
                                    "Target skill not found for upgrade: {}",
                                    proposal.target_skill_id
                                );
                            }
                        }
                    }
                    _ => {
                        tracing::debug!("Unknown auto action type: {}", auto_action.action_type);
                    }
                }
            }
        }
    }

    fn detect_common_theme(
        &self,
        trajectories: &[crate::trajectory::Trajectory],
    ) -> Option<String> {
        let mut topic_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for traj in trajectories {
            let words: Vec<&str> = traj.topic.split_whitespace().collect();
            for word in words.iter().take(3) {
                *topic_counts.entry(word.to_lowercase()).or_insert(0) += 1;
            }
        }

        topic_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .filter(|(_, count)| *count >= 2)
            .map(|(topic, _)| topic)
    }

    fn collect_entities_from_trajectories(
        &self,
        trajectories: &[crate::trajectory::Trajectory],
    ) -> Vec<EntityRef> {
        let mut entities: std::collections::HashSet<String> = std::collections::HashSet::new();

        for traj in trajectories {
            for step in &traj.steps {
                let words: Vec<&str> = step.content.split_whitespace().collect();
                for word in words.iter().take(5) {
                    if word.len() > 4
                        && word
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false)
                    {
                        entities.insert(word.to_string());
                    }
                }
            }
        }

        entities
            .into_iter()
            .take(10)
            .map(|name| EntityRef { name })
            .collect()
    }

    fn propose_memory_consolidation(
        &self,
        theme: &str,
        entities: &[EntityRef],
    ) -> MemoryConsolidationTask {
        // Calculate confidence based on entity count and theme specificity
        // More entities and longer themes indicate stronger patterns
        let entity_factor = (entities.len() as f64 / 5.0).min(1.0); // normalize to max 1.0 at 5+ entities
        let theme_factor = (theme.len() as f64 / 10.0).min(1.0); // normalize to max 1.0 at 10+ chars
        let confidence = 0.5 + 0.3 * entity_factor + 0.2 * theme_factor; // range: 0.5 - 1.0

        MemoryConsolidationTask {
            id: format!("mct_{}", uuid::Uuid::new_v4()),
            entities: entities.to_vec(),
            theme: theme.to_string(),
            summary: format!("关于 {} 的记忆整合", theme),
            memory_content: format!(
                "用户经常处理与 {} 相关的主题。关键实体包括: {}",
                theme,
                entities
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            confidence,
        }
    }

    pub fn find_similar_skills(&self, topic: &str) -> Result<Vec<Skill>, anyhow::Error> {
        let all_skills = self.storage.get_all_skills()?;
        let topic_lower = topic.to_lowercase();
        let topic_keywords = self.extract_keywords(&topic_lower);

        let mut scored_skills: Vec<(Skill, f64)> = all_skills
            .into_iter()
            .filter_map(|s| {
                let name_lower = s.name.to_lowercase();
                let desc_lower = s.description.to_lowercase();

                let name_contains = name_lower.contains(&topic_lower);
                let desc_contains = desc_lower.contains(&topic_lower);
                let name_similarity = self.calculate_keyword_similarity(
                    &topic_keywords,
                    &self.extract_keywords(&name_lower),
                );
                let desc_similarity = self.calculate_keyword_similarity(
                    &topic_keywords,
                    &self.extract_keywords(&desc_lower),
                );

                let max_similarity = if name_contains || desc_contains {
                    1.0
                } else {
                    name_similarity.max(desc_similarity)
                };

                if max_similarity > 0.3 {
                    Some((s, max_similarity))
                } else {
                    None
                }
            })
            .collect();

        scored_skills.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored_skills.into_iter().map(|(s, _)| s).collect())
    }

    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let stop_words = [
            "the", "a", "an", "is", "are", "was", "were", "to", "for", "of", "and", "or", "in",
            "on", "at", "with", "by",
        ];
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2 && !stop_words.contains(w))
            .map(|w| w.to_lowercase())
            .collect()
    }

    fn calculate_keyword_similarity(&self, keywords1: &[String], keywords2: &[String]) -> f64 {
        if keywords1.is_empty() || keywords2.is_empty() {
            return 0.0;
        }

        let intersection = keywords1.iter().filter(|k1| keywords2.contains(k1)).count();

        let union = keywords1.len() + keywords2.len() - intersection;

        intersection as f64 / union as f64
    }

    fn extract_scenarios_from_topic(&self, topic: &str) -> Vec<String> {
        let mut scenarios = Vec::new();
        let topic_lower = topic.to_lowercase();

        let known_scenarios = [
            ("debug", "debug"),
            ("test", "testing"),
            ("build", "build"),
            ("deploy", "deployment"),
            ("review", "review"),
            ("code", "coding"),
            ("design", "design"),
            ("document", "documentation"),
            ("api", "api"),
            ("database", "database"),
            ("frontend", "frontend"),
            ("backend", "backend"),
            ("config", "configuration"),
            ("install", "installation"),
            ("setup", "setup"),
            ("migration", "migration"),
            ("optimize", "optimization"),
            ("security", "security"),
        ];

        for (keyword, label) in &known_scenarios {
            if topic_lower.contains(keyword) {
                scenarios.push(label.to_string());
            }
        }

        if scenarios.is_empty() {
            scenarios.push("general".to_string());
        }

        scenarios
    }

    fn propose_skill_improvement(
        &self,
        existing_skill: &Skill,
        task: &crate::trajectory::Trajectory,
    ) -> Option<(SkillUpgradeProposal, SkillCreationProposal)> {
        let skill_factor = existing_skill.success_rate;
        let complexity_factor = (task.steps.len() as f64 / 15.0).min(1.0);
        let confidence = 0.4 + 0.3 * skill_factor + 0.3 * complexity_factor;

        let should_upgrade = confidence >= 0.6 && skill_factor > 0.3;

        if should_upgrade {
            let scenarios = self.extract_scenarios_from_topic(&task.topic);
            let improvements = self.generate_improvements(existing_skill, task);

            let upgrade_proposal = SkillUpgradeProposal {
                target_skill_id: existing_skill.id.clone(),
                suggested_improvements: improvements,
                additional_scenarios: scenarios.clone(),
                confidence,
                trigger_event: format!("topic:{}", task.topic),
            };

            let creation_proposal = SkillCreationProposal {
                task_description: task.topic.clone(),
                suggested_name: existing_skill.name.clone(),
                suggested_content: existing_skill.content.clone(),
                confidence,
                trigger_event: format!("upgrade:{}", task.topic),
                scenarios,
            };

            Some((upgrade_proposal, creation_proposal))
        } else {
            None
        }
    }

    fn generate_improvements(
        &self,
        existing_skill: &Skill,
        task: &crate::trajectory::Trajectory,
    ) -> String {
        let mut improvements = String::new();

        improvements.push_str(&format!(
            "# Improvement suggestions for skill {}\n\n",
            existing_skill.name
        ));

        if existing_skill.success_rate < 0.5 {
            improvements.push_str("## Success Rate Issues\n");
            improvements.push_str("- Consider simplifying skill steps\n");
            improvements.push_str("- Add more error handling guidance\n\n");
        }

        let tool_steps: Vec<_> = task
            .steps
            .iter()
            .filter(|s| s.tool_calls.is_some())
            .collect();

        if !tool_steps.is_empty() {
            improvements.push_str("## New Step Suggestions\n");
            for (i, step) in tool_steps.iter().take(5).enumerate() {
                if let Some(ref calls) = step.tool_calls {
                    for call in calls {
                        improvements.push_str(&format!(
                            "{}. Use `{}` with args: {}\n",
                            i + 1,
                            call.name,
                            call.arguments
                        ));
                    }
                }
            }
            improvements.push('\n');
        }

        if task.outcome == crate::trajectory::TrajectoryOutcome::Failure {
            improvements.push_str("## Failure Analysis\n");
            improvements.push_str("- Task failure detected, need enhanced error handling\n");
            improvements.push_str("- Consider adding validation checkpoints\n\n");
        }

        improvements.push_str("## Quality Metrics\n");
        improvements.push_str(&format!(
            "- Task completion: {:.1}%\n",
            task.quality.task_completion * 100.0
        ));
        improvements.push_str(&format!(
            "- Tool efficiency: {:.1}%\n",
            task.quality.tool_efficiency * 100.0
        ));

        improvements
    }

    fn generate_skill_name_from_topic(&self, topic: &str) -> String {
        let words: Vec<&str> = topic.split_whitespace().take(3).collect();
        let base = words.join("-");

        if base.len() < 3 {
            format!("skill-{}", base.replace(|c: char| !c.is_alphanumeric(), ""))
        } else {
            base.replace(|c: char| !c.is_alphanumeric(), "")
        }
    }

    fn generate_skill_content_from_trajectory(
        &self,
        trajectory: &crate::trajectory::Trajectory,
    ) -> String {
        let mut content = format!("# {}\n\n", trajectory.topic);
        content += &format!(
            "slug: {}\n\n",
            self.generate_skill_name_from_topic(&trajectory.topic)
        );

        content += "## Overview\n";
        content += &format!("This skill handles: {}. ", trajectory.topic);
        content += &format!(
            "Outcome: {:?}, Duration: {}ms\n\n",
            trajectory.outcome, trajectory.duration_ms
        );

        let tool_steps: Vec<_> = trajectory
            .steps
            .iter()
            .filter(|s| s.tool_calls.is_some())
            .collect();

        if !tool_steps.is_empty() {
            content += "## Procedure\n";
            for (i, step) in tool_steps.iter().enumerate() {
                if let Some(ref calls) = step.tool_calls {
                    for call in calls {
                        content += &format!(
                            "{}. Use `{}` with args: {}\n",
                            i + 1,
                            call.name,
                            call.arguments
                        );
                    }
                }
            }
            content += "\n";
        }

        if !trajectory.patterns.is_empty() {
            content += &format!(
                "## Patterns\n{}\n\n",
                trajectory.patterns.first().unwrap_or(&String::new())
            );
        }

        content += "## Quality\n";
        content += &format!(
            "- Task completion: {:.1}%\n",
            trajectory.quality.task_completion * 100.0
        );
        content += &format!(
            "- Tool efficiency: {:.1}%\n",
            trajectory.quality.tool_efficiency * 100.0
        );

        content
    }

    pub fn get_nudges(&self) -> Vec<PeriodicNudge> {
        self.nudges.read().unwrap().clone()
    }

    pub fn acknowledge_nudge(&self, id: &str) {
        let mut nudges = self.nudges.write().unwrap();
        if let Some(nudge) = nudges.iter_mut().find(|n| n.id == id) {
            nudge.acknowledged = true;
        }
    }
}
