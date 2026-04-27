//! Auto Memory Extractor - Extracts structured memories from agent trajectories using LLM
//!
//! This module analyzes completed trajectories and extracts:
//! - User preferences and habits
//! - Key facts about user's projects, environment, and workflow
//! - Important patterns that should be remembered
//! - Cross-session context that enables continuity

use crate::insight::{InsightCategory, LearningInsight};
use crate::memory::MemoryService;
use crate::pattern::PatternLearner;
use crate::storage::TrajectoryStorage;
use crate::trajectory::{Trajectory, TrajectoryOutcome};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

const MEMORY_EXTRACTION_MIN_STEPS: usize = 4;
const MAX_MEMORY_ENTRIES_PER_TRAJECTORY: usize = 5;
const _MEMORY_DECAY_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub memory_type: MemoryType,
    pub content: String,
    pub confidence: f64,
    pub source_trajectory: String,
    pub extraction_reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    Preference,
    Fact,
    Pattern,
    Context,
    Project,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Preference => "preference",
            MemoryType::Fact => "fact",
            MemoryType::Pattern => "pattern",
            MemoryType::Context => "context",
            MemoryType::Project => "project",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExtractionResult {
    pub extracted_memories: Vec<ExtractedMemory>,
    pub insights_generated: Vec<LearningInsight>,
    pub trajectories_analyzed: usize,
}

pub struct AutoMemoryExtractor {
    #[allow(dead_code)]
    storage: Arc<TrajectoryStorage>,
    memory_service: Arc<std::sync::RwLock<MemoryService>>,
    #[allow(dead_code)]
    pattern_learner: Arc<std::sync::RwLock<PatternLearner>>,
    recent_extractions: Vec<ExtractedMemory>,
    extraction_cache: HashMap<String, Vec<ExtractedMemory>>,
}

impl AutoMemoryExtractor {
    pub fn new(
        storage: Arc<TrajectoryStorage>,
        memory_service: Arc<std::sync::RwLock<MemoryService>>,
        pattern_learner: Arc<std::sync::RwLock<PatternLearner>>,
    ) -> Self {
        Self {
            storage,
            memory_service,
            pattern_learner,
            recent_extractions: Vec::new(),
            extraction_cache: HashMap::new(),
        }
    }

    pub fn analyze_trajectory(
        &mut self,
        trajectory: &Trajectory,
    ) -> Option<MemoryExtractionResult> {
        if trajectory.steps.len() < MEMORY_EXTRACTION_MIN_STEPS {
            return None;
        }

        if let Some(cached) = self.extraction_cache.get(&trajectory.id) {
            return Some(MemoryExtractionResult {
                extracted_memories: cached.clone(),
                insights_generated: Vec::new(),
                trajectories_analyzed: 1,
            });
        }

        let memories = self.extract_memories_from_trajectory(trajectory);
        let insights = self.generate_insights(&memories, trajectory);

        for memory in &memories {
            self.recent_extractions.push(memory.clone());
        }
        if self.recent_extractions.len() > 100 {
            self.recent_extractions.drain(0..50);
        }

        self.extraction_cache
            .insert(trajectory.id.clone(), memories.clone());

        Some(MemoryExtractionResult {
            extracted_memories: memories,
            insights_generated: insights,
            trajectories_analyzed: 1,
        })
    }

    fn extract_memories_from_trajectory(&self, trajectory: &Trajectory) -> Vec<ExtractedMemory> {
        let mut memories = Vec::new();
        let mut seen_content: HashMap<String, usize> = HashMap::new();

        let user_messages: Vec<_> = trajectory
            .steps
            .iter()
            .filter(|s| matches!(s.role, crate::trajectory::MessageRole::User))
            .collect();

        let assistant_messages: Vec<_> = trajectory
            .steps
            .iter()
            .filter(|s| matches!(s.role, crate::trajectory::MessageRole::Assistant))
            .collect();

        if let Some(first_user) = user_messages.first() {
            let content_lower = first_user.content.to_lowercase();
            if !content_lower.contains("hello")
                && !content_lower.contains("hi ")
                && !content_lower.contains("hey")
            {
                memories.push(ExtractedMemory {
                    memory_type: MemoryType::Context,
                    content: format!(
                        "User is working on: {}",
                        first_user.content.chars().take(200).collect::<String>()
                    ),
                    confidence: 0.7,
                    source_trajectory: trajectory.id.clone(),
                    extraction_reason: "First user message indicates task context".to_string(),
                });
                *seen_content.entry(first_user.content.clone()).or_insert(0) += 1;
            }
        }

        for (i, step) in assistant_messages.iter().enumerate() {
            if let Some(ref tool_calls) = step.tool_calls {
                if !tool_calls.is_empty() {
                    let tool_names: Vec<String> =
                        tool_calls.iter().map(|tc| tc.name.clone()).collect();
                    let unique_tools: Vec<String> = tool_names
                        .iter()
                        .cloned()
                        .collect::<std::collections::HashSet<_>>()
                        .iter()
                        .cloned()
                        .collect();

                    if unique_tools.len() >= 2 {
                        let pattern_key = unique_tools.join(",");
                        let count = seen_content.entry(pattern_key.clone()).or_insert(0);
                        *count += 1;

                        if *count >= 2 {
                            memories.push(ExtractedMemory {
                                memory_type: MemoryType::Pattern,
                                content: format!(
                                    "User frequently uses tools together: {}",
                                    unique_tools.join(" -> ")
                                ),
                                confidence: 0.8,
                                source_trajectory: trajectory.id.clone(),
                                extraction_reason: "Repeated tool combination detected".to_string(),
                            });
                        }
                    }
                }
            }

            if let Some(ref reasoning) = step.reasoning {
                if reasoning.len() > 100 && i == 0 {
                    let key = "detailed_reasoning".to_string();
                    let count = seen_content.entry(key).or_insert(0);
                    *count += 1;

                    if *count >= 2 {
                        memories.push(ExtractedMemory {
                            memory_type: MemoryType::Preference,
                            content: "User appreciates detailed reasoning and step-by-step problem solving".to_string(),
                            confidence: 0.75,
                            source_trajectory: trajectory.id.clone(),
                            extraction_reason: "Multiple detailed reasoning chains observed".to_string(),
                        });
                    }
                }
            }
        }

        match trajectory.outcome {
            TrajectoryOutcome::Success => {
                memories.push(ExtractedMemory {
                    memory_type: MemoryType::Fact,
                    content: format!("Task '{}' was completed successfully", trajectory.topic),
                    confidence: 0.9,
                    source_trajectory: trajectory.id.clone(),
                    extraction_reason: "Successful task completion".to_string(),
                });
            }
            TrajectoryOutcome::Failure => {
                let error_tools: usize = trajectory
                    .steps
                    .iter()
                    .filter_map(|s| {
                        s.tool_results.as_ref().and_then(|r| {
                            r.iter()
                                .find(|tr| tr.is_error || tr.output.contains("error"))
                                .map(|_| &s.tool_calls)
                        })
                    })
                    .count();

                if error_tools > 0 {
                    memories.push(ExtractedMemory {
                        memory_type: MemoryType::Context,
                        content: format!(
                            "Task '{}' failed - may need troubleshooting approach",
                            trajectory.topic
                        ),
                        confidence: 0.6,
                        source_trajectory: trajectory.id.clone(),
                        extraction_reason: "Failed task with error indicators".to_string(),
                    });
                }
            }
            TrajectoryOutcome::Partial => {
                memories.push(ExtractedMemory {
                    memory_type: MemoryType::Context,
                    content: format!(
                        "Task '{}' partially completed - follow-up may be needed",
                        trajectory.topic
                    ),
                    confidence: 0.65,
                    source_trajectory: trajectory.id.clone(),
                    extraction_reason: "Partial task completion".to_string(),
                });
            }
            TrajectoryOutcome::Abandoned => {}
        }

        let deduplicated: Vec<_> = memories
            .into_iter()
            .filter(|m| {
                let key = m.content.chars().take(50).collect::<String>();
                *seen_content.entry(key).or_insert(0) == 1
            })
            .take(MAX_MEMORY_ENTRIES_PER_TRAJECTORY)
            .collect();

        deduplicated
    }

    fn generate_insights(
        &self,
        memories: &[ExtractedMemory],
        trajectory: &Trajectory,
    ) -> Vec<LearningInsight> {
        let mut insights = Vec::new();

        for memory in memories {
            if memory.confidence >= 0.7 && memory.memory_type == MemoryType::Pattern {
                insights.push(LearningInsight {
                    id: format!("insight_{}_{}", trajectory.id, memory.memory_type.as_str()),
                    category: InsightCategory::Pattern,
                    title: format!(
                        "Detected: {}",
                        memory.content.chars().take(40).collect::<String>()
                    ),
                    description: memory.extraction_reason.clone(),
                    confidence: memory.confidence,
                    evidence: vec![memory.source_trajectory.clone()],
                    suggested_action: Some(
                        "Consider adding this pattern to user profile".to_string(),
                    ),
                    created_at: chrono::Utc::now().timestamp_millis(),
                });
            }
        }

        insights
    }

    pub fn get_recent_extractions(&self) -> Vec<ExtractedMemory> {
        self.recent_extractions.clone()
    }

    pub fn clear_cache(&mut self) {
        self.extraction_cache.clear();
    }

    pub fn apply_memories_to_service(&self, memories: &[ExtractedMemory]) -> anyhow::Result<usize> {
        let memory_service = self
            .memory_service
            .write()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut applied = 0;

        for memory in memories {
            let result = memory_service.add_memory(memory.memory_type.as_str(), &memory.content);
            if result.success {
                applied += 1;
            } else {
                tracing::warn!("Failed to add memory entry: {}", result.message);
            }
        }

        Ok(applied)
    }
}

impl Trajectory {
    pub fn extract_memory_candidates(&self) -> Vec<String> {
        let mut candidates = Vec::new();

        for step in &self.steps {
            if matches!(step.role, crate::trajectory::MessageRole::User)
                && step.content.len() > 20
                && step.content.len() < 500
            {
                candidates.push(step.content.clone());
            }
        }

        if let Some(last) = self.steps.last() {
            if matches!(last.role, crate::trajectory::MessageRole::Assistant) {
                if let Some(ref tool_calls) = last.tool_calls {
                    for tc in tool_calls {
                        if !tc.name.contains("read") && !tc.name.contains("write") {
                            candidates.push(format!(
                                "Tool used: {} with args: {}",
                                tc.name,
                                tc.arguments.chars().take(100).collect::<String>()
                            ));
                        }
                    }
                }
            }
        }

        candidates
    }
}
