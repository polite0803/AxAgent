// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axagent_harness::dream::{
    ConsolidationDataProvider, ConsolidationSuggestion, DistilledKnowledge, ExperienceRecord,
    KnowledgeType,
};
use chrono::{TimeZone, Utc};
use tokio::sync::RwLock;

use crate::dream_consolidation::ReplaySample;
use crate::memory_providers::service::{
    AddMemoryRequest, MemoryEntry, MemoryNature, MemoryProvenance, MemoryService, MemoryTier,
};
use crate::skill::Skill;
use crate::storage::TrajectoryStorage;
use crate::trajectory::{Trajectory, TrajectoryOutcome};

/// Dream 巩固系统从 Memory 融合数据时使用的 importance 阈值
const MEMORY_IMPORTANCE_THRESHOLD: f64 = 0.6;

pub struct TrajectoryDreamDataProvider {
    storage: Arc<TrajectoryStorage>,
    /// Memory 服务引用（可选，便于向后兼容；存在时用于融合高 importance 数据）
    memory_service: Option<Arc<RwLock<MemoryService>>>,
    knowledge_cache: RwLock<HashMap<String, DistilledKnowledge>>,
    suggestions_cache: RwLock<HashMap<String, ConsolidationSuggestion>>,
}

impl TrajectoryDreamDataProvider {
    pub fn new(storage: Arc<TrajectoryStorage>) -> Self {
        Self {
            storage,
            memory_service: None,
            knowledge_cache: RwLock::new(HashMap::new()),
            suggestions_cache: RwLock::new(HashMap::new()),
        }
    }

    /// 注入 MemoryService 引用，启用 Dream↔Memory 联动
    pub fn with_memory_service(mut self, memory_service: Arc<RwLock<MemoryService>>) -> Self {
        self.memory_service = Some(memory_service);
        self
    }

    pub async fn cached_knowledge_count(&self) -> usize {
        self.knowledge_cache.read().await.len()
    }

    pub async fn cached_suggestions_count(&self) -> usize {
        self.suggestions_cache.read().await.len()
    }

    pub async fn clear_caches(&self) {
        self.knowledge_cache.write().await.clear();
        self.suggestions_cache.write().await.clear();
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

/// 将 Memory 条目转换为 ExperienceRecord
/// importance 直接映射为 quality_score（0.0-1.0 区间一致）
fn memory_entry_to_experience_record(entry: &MemoryEntry) -> ExperienceRecord {
    // 将 Unix 秒级时间戳转换为 DateTime<Utc>，失败则回退到当前时间
    let timestamp = Utc.timestamp_opt(entry.created_at, 0).single().unwrap_or_else(Utc::now);

    ExperienceRecord {
        id: format!("memory_{}", entry.id),
        session_id: entry
            .provenance
            .as_ref()
            .and_then(|p| p.conversation_id.clone())
            .unwrap_or_else(|| "memory".to_string()),
        topic: entry.memory_type.clone(),
        outcome: format!("memory_importance_{:.2}", entry.importance),
        quality_score: entry.importance,
        tool_sequence: Vec::new(),
        reasoning_summary: entry.content.chars().take(200).collect(),
        timestamp,
    }
}

fn distilled_knowledge_to_skill(knowledge: &DistilledKnowledge) -> Skill {
    let name = format!(
        "{:?}-{}",
        knowledge.knowledge_type,
        knowledge.content.chars().take(30).collect::<String>()
    );
    let description: String = knowledge.content.chars().take(200).collect();
    Skill::new(
        name,
        description,
        knowledge.content.clone(),
        format!("{:?}", knowledge.knowledge_type),
    )
}

#[async_trait]
impl ConsolidationDataProvider for TrajectoryDreamDataProvider {
    async fn fetch_recent_experiences(
        &self,
        limit: usize,
    ) -> Result<Vec<ExperienceRecord>, String> {
        let trajectories =
            self.storage.get_trajectories(Some(limit)).await.map_err(|e| e.to_string())?;
        let mut records: Vec<ExperienceRecord> =
            trajectories.iter().map(trajectory_to_experience_record).collect();

        // Dream↔Memory 联动：融合 Memory 中高 importance 的条目
        if let Some(memory_service) = &self.memory_service {
            let ms = memory_service.read().await;
            let working_memory = ms.get_working_memory().await;
            drop(ms);

            let trajectory_count = records.len();
            let mut memory_records: Vec<ExperienceRecord> = working_memory
                .entries
                .values()
                .filter(|e| !e.is_expired() && e.importance >= MEMORY_IMPORTANCE_THRESHOLD)
                .map(memory_entry_to_experience_record)
                .collect();
            let memory_count = memory_records.len();

            // 合并两个数据源，按 quality_score 降序排序（高价值优先）
            records.append(&mut memory_records);
            records.sort_by(|a, b| {
                b.quality_score.partial_cmp(&a.quality_score).unwrap_or(std::cmp::Ordering::Equal)
            });

            tracing::debug!(
                "[dream] fetch_recent_experiences: trajectory={}, memory={}, merged={}",
                trajectory_count,
                memory_count,
                records.len()
            );
        }

        Ok(records)
    }

    async fn fetch_experience_by_topic(
        &self,
        topic: &str,
    ) -> Result<Vec<ExperienceRecord>, String> {
        let trajectories = self.storage.get_trajectories(None).await.map_err(|e| e.to_string())?;
        Ok(trajectories
            .iter()
            .filter(|t| t.topic.contains(topic))
            .map(trajectory_to_experience_record)
            .collect())
    }

    async fn store_distilled_knowledge(
        &self,
        knowledge: &DistilledKnowledge,
    ) -> Result<(), String> {
        // 1. 写入内存缓存
        {
            let mut cache = self.knowledge_cache.write().await;
            cache.insert(knowledge.id.clone(), knowledge.clone());
        }

        // 2. 写入 trajectory_skills 表
        let skill = distilled_knowledge_to_skill(knowledge);
        self.storage.save_skill(&skill).await.map_err(|e| e.to_string())?;

        // 3. Dream↔Memory 联动：同时写入 MemoryService
        //    按 confidence 映射 tier 和 importance
        if let Some(memory_service) = &self.memory_service {
            let (tier, importance) = confidence_to_memory_tier(knowledge.confidence);

            let req = AddMemoryRequest {
                target: format!("dream_{:?}", knowledge.knowledge_type).to_lowercase(),
                content: knowledge.content.clone(),
                tier,
                importance,
                nature: MemoryNature::Semantic,
                provenance: Some(MemoryProvenance {
                    conversation_id: knowledge.source_session_ids.first().cloned(),
                    message_id: None,
                    extraction_method: "dream_consolidation".to_string(),
                }),
                tags: knowledge.applicability_tags.clone(),
                expires_at: None,
                namespace_id: None,
            };

            let ms = memory_service.read().await;
            let result = ms.add_memory_advanced(req).await;
            drop(ms);

            if !result.success {
                tracing::warn!(
                    "[dream] Memory 写入失败（不影响 trajectory_skills）: {}",
                    result.message
                );
            } else {
                tracing::debug!(
                    "[dream] 蒸馏知识已写入 Memory: tier={}, importance={:.2}",
                    tier.as_str(),
                    importance
                );
            }
        }

        Ok(())
    }

    async fn store_suggestion(&self, suggestion: &ConsolidationSuggestion) -> Result<(), String> {
        let mut cache = self.suggestions_cache.write().await;
        cache.insert(suggestion.id.clone(), suggestion.clone());
        Ok(())
    }

    async fn fetch_existing_knowledge(
        &self,
        knowledge_type: &KnowledgeType,
    ) -> Result<Vec<DistilledKnowledge>, String> {
        let cache = self.knowledge_cache.read().await;
        Ok(cache.values().filter(|k| k.knowledge_type == *knowledge_type).cloned().collect())
    }
}

/// 根据 distilled knowledge 的 confidence 映射到 Memory 的 tier 和 importance
/// - confidence >= 0.85 → LongTerm, importance=0.85
/// - 0.6 <= confidence < 0.85 → Working, importance=0.6
/// - confidence < 0.6 → ShortTerm, importance=0.4
fn confidence_to_memory_tier(confidence: f64) -> (MemoryTier, f64) {
    if confidence >= 0.85 {
        (MemoryTier::LongTerm, 0.85)
    } else if confidence >= 0.6 {
        (MemoryTier::Working, 0.6)
    } else {
        (MemoryTier::ShortTerm, 0.4)
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

    #[tokio::test]
    async fn test_knowledge_cache_store_and_retrieve() {
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
            let mut cache = provider.knowledge_cache.write().await;
            cache.insert(knowledge.id.clone(), knowledge.clone());
        }

        assert_eq!(provider.cached_knowledge_count().await, 1);

        let cached = provider.knowledge_cache.read().await.get("k1").cloned().unwrap();
        assert_eq!(cached.content, "Pattern A");
    }

    #[tokio::test]
    async fn test_suggestions_cache_store_and_retrieve() {
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
            let mut cache = provider.suggestions_cache.write().await;
            cache.insert(suggestion.id.clone(), suggestion.clone());
        }

        assert_eq!(provider.cached_suggestions_count().await, 1);

        let cached = provider.suggestions_cache.read().await.get("sug1").cloned().unwrap();
        assert_eq!(cached.content, "Improve X");
    }

    #[tokio::test]
    async fn test_fetch_existing_knowledge_filters_by_type() {
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
            let mut cache = provider.knowledge_cache.write().await;
            cache.insert(k1.id.clone(), k1);
            cache.insert(k2.id.clone(), k2);
        }

        let tool_knowledge: Vec<DistilledKnowledge> =
            provider.fetch_existing_knowledge(&KnowledgeType::ToolUsagePattern).await.unwrap();
        assert_eq!(tool_knowledge.len(), 1);
        assert_eq!(tool_knowledge[0].id, "k1");

        let reasoning_knowledge: Vec<DistilledKnowledge> =
            provider.fetch_existing_knowledge(&KnowledgeType::ReasoningStrategy).await.unwrap();
        assert_eq!(reasoning_knowledge.len(), 1);
        assert_eq!(reasoning_knowledge[0].id, "k2");

        let error_knowledge: Vec<DistilledKnowledge> =
            provider.fetch_existing_knowledge(&KnowledgeType::ErrorRecovery).await.unwrap();
        assert!(error_knowledge.is_empty());
    }

    #[tokio::test]
    async fn test_store_suggestion_caches() {
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

        provider.store_suggestion(&suggestion).await.unwrap();

        assert_eq!(provider.cached_suggestions_count().await, 1);
    }

    #[tokio::test]
    async fn test_clear_caches() {
        let provider = TrajectoryDreamDataProvider::new(Arc::new(TrajectoryStorage::new(
            Arc::new(sea_orm::DatabaseConnection::default()),
        )));

        {
            let mut kc = provider.knowledge_cache.write().await;
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
            let mut sc = provider.suggestions_cache.write().await;
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

        assert_eq!(provider.cached_knowledge_count().await, 1);
        assert_eq!(provider.cached_suggestions_count().await, 1);

        provider.clear_caches().await;

        assert_eq!(provider.cached_knowledge_count().await, 0);
        assert_eq!(provider.cached_suggestions_count().await, 0);
    }

    #[test]
    fn test_confidence_to_memory_tier() {
        assert_eq!(confidence_to_memory_tier(0.9), (MemoryTier::LongTerm, 0.85));
        assert_eq!(confidence_to_memory_tier(0.85), (MemoryTier::LongTerm, 0.85));
        assert_eq!(confidence_to_memory_tier(0.7), (MemoryTier::Working, 0.6));
        assert_eq!(confidence_to_memory_tier(0.6), (MemoryTier::Working, 0.6));
        assert_eq!(confidence_to_memory_tier(0.5), (MemoryTier::ShortTerm, 0.4));
        assert_eq!(confidence_to_memory_tier(0.0), (MemoryTier::ShortTerm, 0.4));
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
