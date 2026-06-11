// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use crate::dream_consolidation::{
    ConsolidationDataProvider, ConsolidationSuggestion, DistilledKnowledge, ExperienceRecord,
    KnowledgeType,
};
use crate::skill::Skill;
use crate::storage::TrajectoryStorage;
use crate::trajectory::{Trajectory, TrajectoryOutcome};

pub struct TrajectoryDreamDataProvider {
    storage: Arc<TrajectoryStorage>,
    knowledge_cache: RwLock<HashMap<String, DistilledKnowledge>>,
    suggestions_cache: RwLock<HashMap<String, ConsolidationSuggestion>>,
}

impl TrajectoryDreamDataProvider {
    pub fn new(storage: Arc<TrajectoryStorage>) -> Self {
        Self {
            storage,
            knowledge_cache: RwLock::new(HashMap::new()),
            suggestions_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn cached_knowledge_count(&self) -> usize {
        self.knowledge_cache.read().map(|c| c.len()).unwrap_or(0)
    }

    pub fn cached_suggestions_count(&self) -> usize {
        self.suggestions_cache.read().map(|c| c.len()).unwrap_or(0)
    }

    pub fn clear_caches(&self) {
        if let Ok(mut cache) = self.knowledge_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.suggestions_cache.write() {
            cache.clear();
        }
    }
}

fn outcome_quality_score(outcome: &TrajectoryOutcome) -> f64 {
    match outcome {
        TrajectoryOutcome::Success => 0.9,
        TrajectoryOutcome::Partial => 0.5,
        TrajectoryOutcome::Failure => 0.1,
        TrajectoryOutcome::Abandoned => 0.0,
    }
}

fn extract_tool_sequence(trajectory: &Trajectory) -> Vec<String> {
    trajectory
        .steps
        .iter()
        .filter_map(|step| {
            step.tool_calls
                .as_ref()
                .map(|calls| calls.iter().map(|c| c.name.clone()).collect::<Vec<_>>())
        })
        .flatten()
        .collect()
}

fn build_reasoning_summary(trajectory: &Trajectory) -> String {
    trajectory
        .steps
        .iter()
        .filter_map(|step| step.reasoning.as_ref())
        .take(5)
        .map(|r| {
            let truncated: String = r.chars().take(200).collect();
            truncated
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn trajectory_to_experience_record(trajectory: &Trajectory) -> ExperienceRecord {
    ExperienceRecord {
        id: trajectory.id.clone(),
        session_id: if trajectory.session_id.is_empty() {
            "unknown".to_string()
        } else {
            trajectory.session_id.clone()
        },
        topic: trajectory.topic.clone(),
        outcome: format!("{:?}", trajectory.outcome).to_lowercase(),
        quality_score: outcome_quality_score(&trajectory.outcome),
        tool_sequence: extract_tool_sequence(trajectory),
        reasoning_summary: build_reasoning_summary(trajectory),
        timestamp: trajectory.created_at,
    }
}

fn distilled_knowledge_to_skill(knowledge: &DistilledKnowledge) -> Skill {
    let name = format!(
        "{:?}-{}",
        knowledge.knowledge_type,
        &knowledge.content.chars().take(30).collect::<String>()
    );
    let description: String = knowledge.content.chars().take(200).collect();
    Skill::new(
        name,
        description,
        knowledge.content.clone(),
        format!("{:?}", knowledge.knowledge_type),
    )
}

impl ConsolidationDataProvider for TrajectoryDreamDataProvider {
    fn fetch_recent_experiences(
        &self,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ExperienceRecord>, String>> + Send + '_>> {
        let result = self
            .storage
            .get_trajectories(Some(limit))
            .map(|trajectories| {
                trajectories
                    .iter()
                    .map(trajectory_to_experience_record)
                    .collect()
            })
            .map_err(|e| e.to_string());

        Box::pin(async move { result })
    }

    fn fetch_experience_by_topic(
        &self,
        topic: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ExperienceRecord>, String>> + Send + '_>> {
        let topic_owned = topic.to_string();
        let result = self
            .storage
            .get_trajectories(None)
            .map(|trajectories| {
                trajectories
                    .iter()
                    .filter(|t| t.topic.contains(&topic_owned))
                    .map(trajectory_to_experience_record)
                    .collect()
            })
            .map_err(|e| e.to_string());

        Box::pin(async move { result })
    }

    fn store_distilled_knowledge(
        &self,
        knowledge: &DistilledKnowledge,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        let knowledge_clone = knowledge.clone();

        if let Ok(mut cache) = self.knowledge_cache.write() {
            cache.insert(knowledge.id.clone(), knowledge_clone.clone());
        }

        let skill = distilled_knowledge_to_skill(knowledge);
        let persist_result = self.storage.save_skill(&skill).map_err(|e| e.to_string());

        Box::pin(async move { persist_result })
    }

    fn store_suggestion(
        &self,
        suggestion: &ConsolidationSuggestion,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        let suggestion_clone = suggestion.clone();

        if let Ok(mut cache) = self.suggestions_cache.write() {
            cache.insert(suggestion.id.clone(), suggestion_clone);
        }

        Box::pin(async move { Ok(()) })
    }

    fn fetch_existing_knowledge(
        &self,
        knowledge_type: &KnowledgeType,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DistilledKnowledge>, String>> + Send + '_>> {
        let kt = knowledge_type.clone();
        let result = self
            .knowledge_cache
            .read()
            .map(|cache| {
                cache
                    .values()
                    .filter(|k| k.knowledge_type == kt)
                    .cloned()
                    .collect()
            })
            .map_err(|e| format!("Knowledge cache read lock poisoned: {}", e));

        Box::pin(async move { result })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream_consolidation::{ConsolidationSuggestion, SuggestionType};
    use crate::trajectory::{MessageRole, ToolCall, TrajectoryStep};
    use chrono::Utc;

    fn make_test_trajectory(
        _id: &str,
        session_id: &str,
        topic: &str,
        outcome: TrajectoryOutcome,
    ) -> Trajectory {
        Trajectory::new(
            session_id.to_string(),
            "user-1".to_string(),
            topic.to_string(),
            format!("Summary for {}", topic),
            outcome,
            5000,
            vec![
                TrajectoryStep {
                    timestamp_ms: 1000,
                    role: MessageRole::User,
                    content: "Do something".to_string(),
                    reasoning: None,
                    tool_calls: None,
                    tool_results: None,
                },
                TrajectoryStep {
                    timestamp_ms: 2000,
                    role: MessageRole::Assistant,
                    content: "Thinking...".to_string(),
                    reasoning: Some("I should use tool_a first".to_string()),
                    tool_calls: Some(vec![ToolCall {
                        id: "tc-1".to_string(),
                        name: "tool_a".to_string(),
                        arguments: "{}".to_string(),
                    }]),
                    tool_results: None,
                },
                TrajectoryStep {
                    timestamp_ms: 3000,
                    role: MessageRole::Assistant,
                    content: "Now doing more...".to_string(),
                    reasoning: Some("Then tool_b for the next step".to_string()),
                    tool_calls: Some(vec![ToolCall {
                        id: "tc-2".to_string(),
                        name: "tool_b".to_string(),
                        arguments: r#"{"key":"val"}"#.to_string(),
                    }]),
                    tool_results: None,
                },
            ],
        )
    }

    #[test]
    fn test_outcome_quality_score() {
        assert_eq!(outcome_quality_score(&TrajectoryOutcome::Success), 0.9);
        assert_eq!(outcome_quality_score(&TrajectoryOutcome::Partial), 0.5);
        assert_eq!(outcome_quality_score(&TrajectoryOutcome::Failure), 0.1);
        assert_eq!(outcome_quality_score(&TrajectoryOutcome::Abandoned), 0.0);
    }

    #[test]
    fn test_extract_tool_sequence() {
        let traj = make_test_trajectory("t1", "s1", "test", TrajectoryOutcome::Success);
        let tools = extract_tool_sequence(&traj);
        assert_eq!(tools, vec!["tool_a", "tool_b"]);
    }

    #[test]
    fn test_build_reasoning_summary() {
        let traj = make_test_trajectory("t1", "s1", "test", TrajectoryOutcome::Success);
        let summary = build_reasoning_summary(&traj);
        assert!(summary.contains("tool_a"));
        assert!(summary.contains("tool_b"));
    }

    #[test]
    fn test_trajectory_to_experience_record() {
        let traj = make_test_trajectory("t1", "s1", "file editing", TrajectoryOutcome::Success);
        let record = trajectory_to_experience_record(&traj);
        assert_eq!(record.id, traj.id);
        assert_eq!(record.session_id, "s1");
        assert_eq!(record.topic, "file editing");
        assert_eq!(record.outcome, "success");
        assert_eq!(record.quality_score, 0.9);
        assert_eq!(record.tool_sequence, vec!["tool_a", "tool_b"]);
        assert!(!record.reasoning_summary.is_empty());
    }

    #[test]
    fn test_trajectory_to_experience_record_empty_session() {
        let mut traj = make_test_trajectory("t2", "", "test", TrajectoryOutcome::Failure);
        traj.session_id = String::new();
        let record = trajectory_to_experience_record(&traj);
        assert_eq!(record.session_id, "unknown");
        assert_eq!(record.outcome, "failure");
        assert_eq!(record.quality_score, 0.1);
    }

    #[test]
    fn test_trajectory_to_experience_record_partial() {
        let traj = make_test_trajectory("t3", "s3", "debugging", TrajectoryOutcome::Partial);
        let record = trajectory_to_experience_record(&traj);
        assert_eq!(record.outcome, "partial");
        assert_eq!(record.quality_score, 0.5);
    }

    #[test]
    fn test_trajectory_to_experience_record_abandoned() {
        let traj = make_test_trajectory("t4", "s4", "refactor", TrajectoryOutcome::Abandoned);
        let record = trajectory_to_experience_record(&traj);
        assert_eq!(record.outcome, "abandoned");
        assert_eq!(record.quality_score, 0.0);
    }

    #[test]
    fn test_distilled_knowledge_to_skill() {
        let knowledge = DistilledKnowledge {
            id: "k1".to_string(),
            source_session_ids: vec!["s1".to_string()],
            knowledge_type: KnowledgeType::ToolUsagePattern,
            content: "Use tool_a then tool_b for file editing".to_string(),
            confidence: 0.85,
            applicability_tags: vec!["file ops".to_string()],
            created_at: Utc::now(),
        };
        let skill = distilled_knowledge_to_skill(&knowledge);
        assert!(skill.name.contains("ToolUsagePattern"));
        assert_eq!(skill.category, "ToolUsagePattern");
        assert_eq!(skill.content, knowledge.content);
    }

    #[test]
    fn test_knowledge_cache_store_and_retrieve() {
        let provider = TrajectoryDreamDataProvider::new(Arc::new(TrajectoryStorage::new(
            Arc::new(sea_orm::DatabaseConnection::default()),
        )));

        let knowledge = DistilledKnowledge {
            id: "k1".to_string(),
            source_session_ids: vec!["s1".to_string()],
            knowledge_type: KnowledgeType::ToolUsagePattern,
            content: "Pattern A".to_string(),
            confidence: 0.8,
            applicability_tags: vec!["test".to_string()],
            created_at: Utc::now(),
        };

        {
            let mut cache = provider.knowledge_cache.write().unwrap();
            cache.insert(knowledge.id.clone(), knowledge.clone());
        }

        assert_eq!(provider.cached_knowledge_count(), 1);

        let cached = provider
            .knowledge_cache
            .read()
            .unwrap()
            .get("k1")
            .cloned()
            .unwrap();
        assert_eq!(cached.content, "Pattern A");
    }

    #[test]
    fn test_suggestions_cache_store_and_retrieve() {
        let provider = TrajectoryDreamDataProvider::new(Arc::new(TrajectoryStorage::new(
            Arc::new(sea_orm::DatabaseConnection::default()),
        )));

        let suggestion = ConsolidationSuggestion {
            id: "sug1".to_string(),
            suggestion_type: SuggestionType::SkillImprovement,
            content: "Improve X".to_string(),
            confidence: 0.75,
            source_evidence: vec!["e1".to_string()],
            created_at: Utc::now(),
        };

        {
            let mut cache = provider.suggestions_cache.write().unwrap();
            cache.insert(suggestion.id.clone(), suggestion.clone());
        }

        assert_eq!(provider.cached_suggestions_count(), 1);

        let cached = provider
            .suggestions_cache
            .read()
            .unwrap()
            .get("sug1")
            .cloned()
            .unwrap();
        assert_eq!(cached.content, "Improve X");
    }

    #[test]
    fn test_fetch_existing_knowledge_filters_by_type() {
        let provider = TrajectoryDreamDataProvider::new(Arc::new(TrajectoryStorage::new(
            Arc::new(sea_orm::DatabaseConnection::default()),
        )));

        let k1 = DistilledKnowledge {
            id: "k1".to_string(),
            source_session_ids: vec!["s1".to_string()],
            knowledge_type: KnowledgeType::ToolUsagePattern,
            content: "Pattern".to_string(),
            confidence: 0.8,
            applicability_tags: vec![],
            created_at: Utc::now(),
        };
        let k2 = DistilledKnowledge {
            id: "k2".to_string(),
            source_session_ids: vec!["s2".to_string()],
            knowledge_type: KnowledgeType::ReasoningStrategy,
            content: "Strategy".to_string(),
            confidence: 0.7,
            applicability_tags: vec![],
            created_at: Utc::now(),
        };

        {
            let mut cache = provider.knowledge_cache.write().unwrap();
            cache.insert(k1.id.clone(), k1);
            cache.insert(k2.id.clone(), k2);
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let tool_knowledge: Vec<DistilledKnowledge> = rt
            .block_on(provider.fetch_existing_knowledge(&KnowledgeType::ToolUsagePattern))
            .unwrap();
        assert_eq!(tool_knowledge.len(), 1);
        assert_eq!(tool_knowledge[0].id, "k1");

        let reasoning_knowledge: Vec<DistilledKnowledge> = rt
            .block_on(provider.fetch_existing_knowledge(&KnowledgeType::ReasoningStrategy))
            .unwrap();
        assert_eq!(reasoning_knowledge.len(), 1);
        assert_eq!(reasoning_knowledge[0].id, "k2");

        let error_knowledge: Vec<DistilledKnowledge> = rt
            .block_on(provider.fetch_existing_knowledge(&KnowledgeType::ErrorRecovery))
            .unwrap();
        assert!(error_knowledge.is_empty());
    }

    #[test]
    fn test_store_suggestion_caches() {
        let provider = TrajectoryDreamDataProvider::new(Arc::new(TrajectoryStorage::new(
            Arc::new(sea_orm::DatabaseConnection::default()),
        )));

        let suggestion = ConsolidationSuggestion {
            id: "sug1".to_string(),
            suggestion_type: SuggestionType::ErrorPrevention,
            content: "Prevent errors".to_string(),
            confidence: 0.6,
            source_evidence: vec![],
            created_at: Utc::now(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(provider.store_suggestion(&suggestion)).unwrap();

        assert_eq!(provider.cached_suggestions_count(), 1);
    }

    #[test]
    fn test_clear_caches() {
        let provider = TrajectoryDreamDataProvider::new(Arc::new(TrajectoryStorage::new(
            Arc::new(sea_orm::DatabaseConnection::default()),
        )));

        {
            let mut kc = provider.knowledge_cache.write().unwrap();
            kc.insert(
                "k1".to_string(),
                DistilledKnowledge {
                    id: "k1".to_string(),
                    source_session_ids: vec![],
                    knowledge_type: KnowledgeType::ToolUsagePattern,
                    content: "test".to_string(),
                    confidence: 0.5,
                    applicability_tags: vec![],
                    created_at: Utc::now(),
                },
            );
            let mut sc = provider.suggestions_cache.write().unwrap();
            sc.insert(
                "s1".to_string(),
                ConsolidationSuggestion {
                    id: "s1".to_string(),
                    suggestion_type: SuggestionType::NewSkillProposal,
                    content: "test".to_string(),
                    confidence: 0.5,
                    source_evidence: vec![],
                    created_at: Utc::now(),
                },
            );
        }

        assert_eq!(provider.cached_knowledge_count(), 1);
        assert_eq!(provider.cached_suggestions_count(), 1);

        provider.clear_caches();

        assert_eq!(provider.cached_knowledge_count(), 0);
        assert_eq!(provider.cached_suggestions_count(), 0);
    }

    #[test]
    fn test_reasoning_summary_truncation() {
        let long_reasoning: String = "x".repeat(300);
        let mut traj = make_test_trajectory("t1", "s1", "test", TrajectoryOutcome::Success);
        traj.steps.push(TrajectoryStep {
            timestamp_ms: 4000,
            role: MessageRole::Assistant,
            content: "content".to_string(),
            reasoning: Some(long_reasoning.clone()),
            tool_calls: None,
            tool_results: None,
        });
        let summary = build_reasoning_summary(&traj);
        let parts: Vec<&str> = summary.split(" | ").collect();
        let last_part = parts.last().unwrap();
        assert!(last_part.len() <= 200);
    }

    #[test]
    fn test_extract_tool_sequence_empty() {
        let mut traj = make_test_trajectory("t1", "s1", "test", TrajectoryOutcome::Success);
        for step in &mut traj.steps {
            step.tool_calls = None;
        }
        let tools = extract_tool_sequence(&traj);
        assert!(tools.is_empty());
    }
}
