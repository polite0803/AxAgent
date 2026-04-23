//! Closed-loop learning service
//!
//! Provides proactive nudges and automated learning actions.
//! Replaces TypeScript `ClosedLoopLearning.ts` with Rust implementation.
//! Leverages existing `skill_evolution` module for genetic algorithm-based skill optimization.

use crate::TrajectoryStorage;
use crate::skill::Skill;
use serde::{Deserialize, Serialize};
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
    pub suggested_skill_name: String,
    pub suggested_skill_content: String,
    pub confidence: f64,
    pub trigger_event: String,
}

pub struct ClosedLoopService {
    storage: Arc<TrajectoryStorage>,
    config: ClosedLoopConfig,
    nudges: RwLock<Vec<PeriodicNudge>>,
    is_running: RwLock<bool>,
}

impl ClosedLoopService {
    pub fn new(storage: Arc<TrajectoryStorage>) -> Self {
        Self {
            storage,
            config: ClosedLoopConfig::default(),
            nudges: RwLock::new(Vec::new()),
            is_running: RwLock::new(false),
        }
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

        let recent_trajectories = self.storage
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
                title: "记忆整合建议".to_string(),
                description: format!("检测到关于\"{}\"的多个相关实体，可以整合到长期记忆中", theme),
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

        let recent_trajectories = self.storage
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
                    let proposal = self.propose_skill_improvement(similar, task);
                    if let Some(proposal) = proposal {
                        if proposal.confidence >= self.config.skill_creation_threshold {
                            nudges.push(PeriodicNudge {
                                id: format!("nudge_sc_{}_{}", chrono::Utc::now().timestamp_millis(), uuid::Uuid::new_v4()),
                                nudge_type: NudgeType::SkillCreation,
                                title: "技能创建建议".to_string(),
                                description: format!("建议创建技能\"{}\"来处理类似任务", proposal.suggested_skill_name),
                                suggested_action: proposal.suggested_skill_content.clone(),
                                urgency: if proposal.confidence >= 0.9 { "high".to_string() } else { "medium".to_string() },
                                auto_action: if proposal.confidence >= self.config.min_confidence_for_auto_action {
                                    Some(AutoAction {
                                        action_type: "create_skill".to_string(),
                                        target: serde_json::to_string(&proposal).unwrap_or_default(),
                                    })
                                } else {
                                    None
                                },
                                created_at: chrono::Utc::now().timestamp(),
                                acknowledged: false,
                            });
                        }
                    }
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
                id: format!("nudge_pl_{}_{}", chrono::Utc::now().timestamp_millis(), uuid::Uuid::new_v4()),
                nudge_type: NudgeType::PatternLearn,
                title: "模式学习建议".to_string(),
                description: format!("检测到失败模式\"{}\"，建议调整学习策略", pattern.name),
                suggested_action: format!("用户倾向于忽略以\"{}\"开头的建议，考虑换一种表达方式", pattern.name),
                urgency: if pattern.frequency > 5 { "high".to_string() } else { "low".to_string() },
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
                        // Save consolidated memory to persistent storage
                        let entry = crate::memory::memory::MemoryEntry {
                            id: format!("mem_{}", uuid::Uuid::new_v4()),
                            content: auto_action.target.clone(),
                            memory_type: "memory".to_string(),
                            updated_at: chrono::Utc::now().timestamp(),
                        };
                        if let Err(e) = self.storage.save_memory(&entry) {
                            tracing::warn!("Failed to auto-save memory: {}", e);
                        } else {
                            tracing::info!("Auto-saved consolidated memory: {:?}", &auto_action.target[..auto_action.target.len().min(50)]);
                        }
                    }
                    "create_skill" => {
                        // Save proposed skill to persistent storage
                        if let Ok(proposal) = serde_json::from_str::<SkillCreationProposal>(&auto_action.target) {
                            let now = chrono::Utc::now();
                            let skill = Skill {
                                id: uuid::Uuid::new_v4().to_string(),
                                name: proposal.suggested_skill_name.clone(),
                                description: proposal.task_description.clone(),
                                version: "1.0.0".to_string(),
                                content: proposal.suggested_skill_content.clone(),
                                category: "auto_created".to_string(),
                                tags: vec![],
                                platforms: vec![],
                                scenarios: vec![],
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
                                tracing::warn!("Failed to auto-create skill: {}", e);
                            } else {
                                tracing::info!("Auto-created skill: {}", proposal.suggested_skill_name);
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

    fn detect_common_theme(&self, trajectories: &[crate::trajectory::Trajectory]) -> Option<String> {
        let mut topic_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

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

    fn collect_entities_from_trajectories(&self, trajectories: &[crate::trajectory::Trajectory]) -> Vec<EntityRef> {
        let mut entities: std::collections::HashSet<String> = std::collections::HashSet::new();

        for traj in trajectories {
            for step in &traj.steps {
                let words: Vec<&str> = step.content.split_whitespace().collect();
                for word in words.iter().take(5) {
                    if word.len() > 4 && word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
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

    fn propose_memory_consolidation(&self, theme: &str, entities: &[EntityRef]) -> MemoryConsolidationTask {
        // Calculate confidence based on entity count and theme specificity
        // More entities and longer themes indicate stronger patterns
        let entity_factor = (entities.len() as f64 / 5.0).min(1.0); // normalize to max 1.0 at 5+ entities
        let theme_factor = (theme.len() as f64 / 10.0).min(1.0);   // normalize to max 1.0 at 10+ chars
        let confidence = 0.5 + 0.3 * entity_factor + 0.2 * theme_factor; // range: 0.5 - 1.0

        MemoryConsolidationTask {
            id: format!("mct_{}", uuid::Uuid::new_v4()),
            entities: entities.to_vec(),
            theme: theme.to_string(),
            summary: format!("关于 {} 的记忆整合", theme),
            memory_content: format!(
                "用户经常处理与 {} 相关的主题。关键实体包括: {}",
                theme,
                entities.iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join(", ")
            ),
            confidence,
        }
    }

    fn find_similar_skills(&self, topic: &str) -> Result<Vec<Skill>, anyhow::Error> {
        let all_skills = self.storage.get_all_skills()?;
        let topic_lower = topic.to_lowercase();

        let similar: Vec<Skill> = all_skills
            .into_iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&topic_lower) ||
                s.description.to_lowercase().contains(&topic_lower)
            })
            .collect();

        Ok(similar)
    }

    fn propose_skill_improvement(&self, existing_skill: &Skill, task: &crate::trajectory::Trajectory) -> Option<SkillCreationProposal> {
        // Calculate confidence based on existing skill success rate and task complexity
        let skill_factor = existing_skill.success_rate; // 0.0 - 1.0
        let complexity_factor = (task.steps.len() as f64 / 15.0).min(1.0); // normalize at 15+ steps
        let confidence = 0.4 + 0.3 * skill_factor + 0.3 * complexity_factor; // range: 0.4 - 1.0

        Some(SkillCreationProposal {
            task_description: task.topic.clone(),
            suggested_skill_name: format!("{}_v2", existing_skill.name),
            suggested_skill_content: existing_skill.content.clone(),
            confidence,
            trigger_event: format!("topic:{}", task.topic),
        })
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