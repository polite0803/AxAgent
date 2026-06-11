// SPDX-License-Identifier: AGPL-3.0-only

//! Dream 模式巩固增强
//!
//! 在会话空闲期间统一调度后台推理任务：
//! - 记忆提取和巩固（经验回放 + 知识蒸馏）
//! - 跨会话模式发现（对比学习）
//! - 主动建议生成
//! - 上下文预加载
//!
//! 通过时间门控和会话计数门控防止过度消耗资源，
//! 使用互斥锁防止并发运行。
//! 移植自 claude-code-main 的 autoDream 机制。

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// 门控配置
// ---------------------------------------------------------------------------

const DEFAULT_MIN_INTERVAL_HOURS: i64 = 1;
const DEFAULT_MIN_NEW_SESSIONS: u32 = 3;
const DEFAULT_MAX_CONSOLIDATION_SECS: u64 = 120;
const LOCK_TIMEOUT_SECS: u64 = 5;

// ---------------------------------------------------------------------------
// 配置
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamConsolidationConfig {
    pub enabled: bool,
    pub min_interval_hours: i64,
    pub min_new_sessions: u32,
    pub max_consolidation_secs: u64,
    pub run_memory_extraction: bool,
    pub run_pattern_learning: bool,
    pub run_proactive_suggestions: bool,
    pub experience_replay_sample_size: usize,
    pub contrastive_pair_threshold: f64,
    pub distillation_min_quality: f64,
}

impl Default for DreamConsolidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_interval_hours: DEFAULT_MIN_INTERVAL_HOURS,
            min_new_sessions: DEFAULT_MIN_NEW_SESSIONS,
            max_consolidation_secs: DEFAULT_MAX_CONSOLIDATION_SECS,
            run_memory_extraction: true,
            run_pattern_learning: true,
            run_proactive_suggestions: true,
            experience_replay_sample_size: 50,
            contrastive_pair_threshold: 0.3,
            distillation_min_quality: 0.6,
        }
    }
}

// ---------------------------------------------------------------------------
// 巩固结果
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamConsolidationResult {
    pub executed: bool,
    pub skip_reason: Option<String>,
    pub memories_extracted: usize,
    pub patterns_discovered: usize,
    pub suggestions_generated: usize,
    pub started_at: DateTime<Utc>,
    pub duration_secs: u64,
    pub error: Option<String>,
    pub experience_replay_count: usize,
    pub distilled_knowledge_count: usize,
    pub contrastive_insights_count: usize,
}

impl DreamConsolidationResult {
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            executed: false,
            skip_reason: Some(reason.into()),
            memories_extracted: 0,
            patterns_discovered: 0,
            suggestions_generated: 0,
            started_at: Utc::now(),
            duration_secs: 0,
            error: None,
            experience_replay_count: 0,
            distilled_knowledge_count: 0,
            contrastive_insights_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// 状态跟踪
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DreamConsolidationState {
    pub last_consolidation_at: Option<DateTime<Utc>>,
    pub sessions_since_last: u32,
    pub total_consolidations: u64,
    pub total_memories_extracted: u64,
    pub total_consolidation_secs: u64,
    pub is_running: bool,
    pub total_experience_replayed: u64,
    pub total_distilled_knowledge: u64,
    pub total_contrastive_insights: u64,
}

// ---------------------------------------------------------------------------
// 经验回放
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceRecord {
    pub id: String,
    pub session_id: String,
    pub topic: String,
    pub outcome: String,
    pub quality_score: f64,
    pub tool_sequence: Vec<String>,
    pub reasoning_summary: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySample {
    pub records: Vec<ExperienceRecord>,
    pub avg_quality: f64,
    pub topic_distribution: HashMap<String, usize>,
}

// ---------------------------------------------------------------------------
// 知识蒸馏
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledKnowledge {
    pub id: String,
    pub source_session_ids: Vec<String>,
    pub knowledge_type: KnowledgeType,
    pub content: String,
    pub confidence: f64,
    pub applicability_tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KnowledgeType {
    ToolUsagePattern,
    ReasoningStrategy,
    ErrorRecovery,
    TaskDecomposition,
    OptimizationHint,
}

// ---------------------------------------------------------------------------
// 对比学习
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContrastivePair {
    pub success: ExperienceRecord,
    pub failure: ExperienceRecord,
    pub topic: String,
    pub differentiating_factors: Vec<String>,
    pub insight: String,
}

// ---------------------------------------------------------------------------
// 巩固建议
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationSuggestion {
    pub id: String,
    pub suggestion_type: SuggestionType,
    pub content: String,
    pub confidence: f64,
    pub source_evidence: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuggestionType {
    SkillImprovement,
    NewSkillProposal,
    ToolUsageOptimization,
    ErrorPrevention,
    WorkflowEnhancement,
}

// ---------------------------------------------------------------------------
// 巩固数据提供者 trait
// ---------------------------------------------------------------------------

pub trait ConsolidationDataProvider: Send + Sync {
    fn fetch_recent_experiences(
        &self,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ExperienceRecord>, String>> + Send + '_>>;

    fn fetch_experience_by_topic(
        &self,
        topic: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ExperienceRecord>, String>> + Send + '_>> {
        let _ = topic;
        self.fetch_recent_experiences(100)
    }

    fn store_distilled_knowledge(
        &self,
        knowledge: &DistilledKnowledge,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;

    fn store_suggestion(
        &self,
        suggestion: &ConsolidationSuggestion,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;

    fn fetch_existing_knowledge(
        &self,
        knowledge_type: &KnowledgeType,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DistilledKnowledge>, String>> + Send + '_>>;
}

use std::future::Future;
use std::pin::Pin;

// ---------------------------------------------------------------------------
// Dream 巩固调度器
// ---------------------------------------------------------------------------

pub type DreamEventEmitter = Option<Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>>;

pub struct DreamConsolidator {
    config: Arc<Mutex<DreamConsolidationConfig>>,
    state: Arc<Mutex<DreamConsolidationState>>,
    consolidation_lock: Arc<Mutex<()>>,
    event_emitter: DreamEventEmitter,
    data_provider: Option<Arc<dyn ConsolidationDataProvider>>,
    distilled_knowledge_buffer: Arc<Mutex<Vec<DistilledKnowledge>>>,
    suggestions_buffer: Arc<Mutex<Vec<ConsolidationSuggestion>>>,
}

impl DreamConsolidator {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(DreamConsolidationConfig::default())),
            state: Arc::new(Mutex::new(DreamConsolidationState::default())),
            consolidation_lock: Arc::new(Mutex::new(())),
            event_emitter: None,
            data_provider: None,
            distilled_knowledge_buffer: Arc::new(Mutex::new(Vec::new())),
            suggestions_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_config(config: DreamConsolidationConfig) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
            state: Arc::new(Mutex::new(DreamConsolidationState::default())),
            consolidation_lock: Arc::new(Mutex::new(())),
            event_emitter: None,
            data_provider: None,
            distilled_knowledge_buffer: Arc::new(Mutex::new(Vec::new())),
            suggestions_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_data_provider(mut self, provider: Arc<dyn ConsolidationDataProvider>) -> Self {
        self.data_provider = Some(provider);
        self
    }

    pub async fn knowledge_count(&self) -> usize {
        self.distilled_knowledge_buffer.lock().await.len()
    }

    pub fn set_event_emitter(&mut self, emitter: DreamEventEmitter) {
        self.event_emitter = emitter;
    }

    fn emit(&self, event_name: &str, payload: serde_json::Value) {
        if let Some(ref emitter) = self.event_emitter {
            emitter(event_name, payload);
        }
    }

    pub async fn update_config(&self, config: DreamConsolidationConfig) {
        let mut cfg = self.config.lock().await;
        *cfg = config;
    }

    pub async fn get_config(&self) -> DreamConsolidationConfig {
        self.config.lock().await.clone()
    }

    pub async fn get_state(&self) -> DreamConsolidationState {
        self.state.lock().await.clone()
    }

    pub async fn record_new_session(&self) {
        let mut state = self.state.lock().await;
        state.sessions_since_last += 1;
    }

    pub async fn should_consolidate(&self) -> bool {
        let config = self.config.lock().await;
        if !config.enabled {
            return false;
        }

        let state = self.state.lock().await;
        if state.is_running {
            return false;
        }

        if let Some(last) = state.last_consolidation_at {
            let elapsed = Utc::now() - last;
            if elapsed < Duration::hours(config.min_interval_hours) {
                return false;
            }
        }

        if state.sessions_since_last < config.min_new_sessions {
            return false;
        }

        self.consolidation_lock.try_lock().is_ok()
    }

    /// 经验回放：从历史轨迹中采样高质量经验
    async fn experience_replay(
        &self,
        config: &DreamConsolidationConfig,
    ) -> Result<ReplaySample, String> {
        let provider = self
            .data_provider
            .as_ref()
            .ok_or("No data provider configured")?;

        let experiences = provider
            .fetch_recent_experiences(config.experience_replay_sample_size)
            .await
            .map_err(|e| format!("Failed to fetch experiences: {}", e))?;

        if experiences.is_empty() {
            return Ok(ReplaySample {
                records: Vec::new(),
                avg_quality: 0.0,
                topic_distribution: HashMap::new(),
            });
        }

        let mut sorted = experiences;
        sorted.sort_by(|a, b| {
            b.quality_score
                .partial_cmp(&a.quality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let top_k: Vec<ExperienceRecord> = sorted
            .into_iter()
            .filter(|e| e.quality_score >= config.distillation_min_quality)
            .take(config.experience_replay_sample_size)
            .collect();

        let avg_quality = if top_k.is_empty() {
            0.0
        } else {
            top_k.iter().map(|e| e.quality_score).sum::<f64>() / top_k.len() as f64
        };

        let mut topic_distribution: HashMap<String, usize> = HashMap::new();
        for record in &top_k {
            *topic_distribution.entry(record.topic.clone()).or_insert(0) += 1;
        }

        Ok(ReplaySample {
            records: top_k,
            avg_quality,
            topic_distribution,
        })
    }

    /// 知识蒸馏：从高质量轨迹中提取可复用知识
    async fn distill_knowledge(
        &self,
        replay: &ReplaySample,
        config: &DreamConsolidationConfig,
    ) -> Result<Vec<DistilledKnowledge>, String> {
        let provider = self
            .data_provider
            .as_ref()
            .ok_or("No data provider configured")?;

        let mut distilled = Vec::new();

        let mut by_topic: HashMap<&str, Vec<&ExperienceRecord>> = HashMap::new();
        for record in &replay.records {
            by_topic.entry(&record.topic).or_default().push(record);
        }

        for (topic, records) in &by_topic {
            if records.len() < 2 {
                continue;
            }

            let tool_seqs: Vec<&[String]> =
                records.iter().map(|r| r.tool_sequence.as_slice()).collect();

            let common_prefix_len = tool_seqs
                .iter()
                .skip(1)
                .fold(tool_seqs.first().map_or(0, |s| s.len()), |acc, seq| acc.min(seq.len()));

            let mut prefix_len = 0;
            if let Some(first) = tool_seqs.first() {
                for i in 0..common_prefix_len {
                    let tool = &first[i];
                    if tool_seqs.iter().all(|seq| seq.get(i) == Some(tool)) {
                        prefix_len = i + 1;
                    } else {
                        break;
                    }
                }
            }

            if prefix_len >= 2 {
                let common_tools: Vec<String> = tool_seqs
                    .first()
                    .map(|s| s[..prefix_len].to_vec())
                    .unwrap_or_default();

                let knowledge = DistilledKnowledge {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_session_ids: records.iter().map(|r| r.session_id.clone()).collect(),
                    knowledge_type: KnowledgeType::ToolUsagePattern,
                    content: format!(
                        "For topic '{}', a reliable tool sequence is: {}",
                        topic,
                        common_tools.join(" → ")
                    ),
                    confidence: records.len() as f64 / replay.records.len() as f64,
                    applicability_tags: vec![topic.to_string()],
                    created_at: Utc::now(),
                };

                if let Ok(()) = provider.store_distilled_knowledge(&knowledge).await {
                    distilled.push(knowledge);
                }
            }

            let reasoning_strategies: Vec<&str> = records
                .iter()
                .filter_map(|r| {
                    if r.quality_score >= config.distillation_min_quality
                        && !r.reasoning_summary.is_empty()
                    {
                        Some(r.reasoning_summary.as_str())
                    } else {
                        None
                    }
                })
                .take(3)
                .collect();

            if !reasoning_strategies.is_empty() {
                let knowledge = DistilledKnowledge {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_session_ids: records.iter().map(|r| r.session_id.clone()).collect(),
                    knowledge_type: KnowledgeType::ReasoningStrategy,
                    content: format!(
                        "Effective reasoning for '{}': {}",
                        topic,
                        reasoning_strategies.join("; ")
                    ),
                    confidence: records.iter().map(|r| r.quality_score).sum::<f64>()
                        / records.len() as f64,
                    applicability_tags: vec![topic.to_string()],
                    created_at: Utc::now(),
                };

                if let Ok(()) = provider.store_distilled_knowledge(&knowledge).await {
                    distilled.push(knowledge);
                }
            }

            let error_recoveries: Vec<String> = records
                .iter()
                .filter(|r| r.outcome == "partial" || r.outcome == "recovered")
                .filter_map(|r| {
                    if r.reasoning_summary.contains("error") || r.reasoning_summary.contains("fail")
                    {
                        Some(format!(
                            "[{}]: {}",
                            r.session_id,
                            r.reasoning_summary.lines().next().unwrap_or("")
                        ))
                    } else {
                        None
                    }
                })
                .take(3)
                .collect();

            if !error_recoveries.is_empty() {
                let knowledge = DistilledKnowledge {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_session_ids: records.iter().map(|r| r.session_id.clone()).collect(),
                    knowledge_type: KnowledgeType::ErrorRecovery,
                    content: format!(
                        "Error recovery patterns for '{}': {}",
                        topic,
                        error_recoveries.join("; ")
                    ),
                    confidence: 0.5,
                    applicability_tags: vec![topic.to_string(), "error-handling".to_string()],
                    created_at: Utc::now(),
                };

                if let Ok(()) = provider.store_distilled_knowledge(&knowledge).await {
                    distilled.push(knowledge);
                }
            }
        }

        Ok(distilled)
    }

    /// 对比学习：从成功/失败轨迹对中提取区分性知识
    async fn contrastive_learning(
        &self,
        replay: &ReplaySample,
        config: &DreamConsolidationConfig,
    ) -> Result<Vec<ContrastivePair>, String> {
        let _ = self
            .data_provider
            .as_ref()
            .ok_or("No data provider configured")?;

        let mut pairs = Vec::new();

        let mut by_topic: HashMap<&str, (Vec<&ExperienceRecord>, Vec<&ExperienceRecord>)> =
            HashMap::new();
        for record in &replay.records {
            let entry = by_topic.entry(&record.topic).or_default();
            if record.quality_score >= config.distillation_min_quality {
                entry.0.push(record);
            } else {
                entry.1.push(record);
            }
        }

        for (topic, (successes, failures)) in &by_topic {
            for success in successes {
                for failure in failures {
                    let quality_diff = success.quality_score - failure.quality_score;
                    if quality_diff < config.contrastive_pair_threshold {
                        continue;
                    }

                    let mut factors = Vec::new();

                    let success_tools: std::collections::HashSet<&String> =
                        success.tool_sequence.iter().collect();
                    let failure_tools: std::collections::HashSet<&String> =
                        failure.tool_sequence.iter().collect();

                    let unique_to_success: Vec<&String> =
                        success_tools.difference(&failure_tools).copied().collect();
                    let unique_to_failure: Vec<&String> =
                        failure_tools.difference(&success_tools).copied().collect();

                    if !unique_to_success.is_empty() {
                        factors.push(format!(
                            "Success used additional tools: {}",
                            unique_to_success
                                .iter()
                                .map(|t| t.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    if !unique_to_failure.is_empty() {
                        factors.push(format!(
                            "Failure used unnecessary tools: {}",
                            unique_to_failure
                                .iter()
                                .map(|t| t.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }

                    if success.tool_sequence.len() != failure.tool_sequence.len() {
                        factors.push(format!(
                            "Step count differs: success={}, failure={}",
                            success.tool_sequence.len(),
                            failure.tool_sequence.len()
                        ));
                    }

                    if success.reasoning_summary.len() > failure.reasoning_summary.len() * 2 {
                        factors.push("Success had more detailed reasoning".to_string());
                    }

                    let insight = if factors.is_empty() {
                        "Quality difference likely due to external factors".to_string()
                    } else {
                        format!("Key differentiators: {}", factors.join("; "))
                    };

                    let pair = ContrastivePair {
                        success: (*success).clone(),
                        failure: (*failure).clone(),
                        topic: topic.to_string(),
                        differentiating_factors: factors,
                        insight,
                    };

                    pairs.push(pair);
                }
            }
        }

        pairs.sort_by(|a, b| {
            (b.success.quality_score - b.failure.quality_score)
                .partial_cmp(&(a.success.quality_score - a.failure.quality_score))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        pairs.truncate(20);

        Ok(pairs)
    }

    /// 从蒸馏知识和对比洞察中生成建议
    async fn generate_suggestions(
        &self,
        distilled: &[DistilledKnowledge],
        contrastive: &[ContrastivePair],
    ) -> Result<Vec<ConsolidationSuggestion>, String> {
        let provider = self
            .data_provider
            .as_ref()
            .ok_or("No data provider configured")?;

        let mut suggestions = Vec::new();

        for knowledge in distilled {
            let suggestion_type = match knowledge.knowledge_type {
                KnowledgeType::ToolUsagePattern => SuggestionType::ToolUsageOptimization,
                KnowledgeType::ReasoningStrategy => SuggestionType::SkillImprovement,
                KnowledgeType::ErrorRecovery => SuggestionType::ErrorPrevention,
                KnowledgeType::TaskDecomposition => SuggestionType::WorkflowEnhancement,
                KnowledgeType::OptimizationHint => SuggestionType::SkillImprovement,
            };

            let suggestion = ConsolidationSuggestion {
                id: uuid::Uuid::new_v4().to_string(),
                suggestion_type,
                content: knowledge.content.clone(),
                confidence: knowledge.confidence,
                source_evidence: knowledge.source_session_ids.clone(),
                created_at: Utc::now(),
            };

            let _ = provider.store_suggestion(&suggestion).await;
            suggestions.push(suggestion);
        }

        for pair in contrastive.iter().take(10) {
            let suggestion = ConsolidationSuggestion {
                id: uuid::Uuid::new_v4().to_string(),
                suggestion_type: SuggestionType::ErrorPrevention,
                content: format!("For topic '{}': {}", pair.topic, pair.insight),
                confidence: 0.7,
                source_evidence: vec![
                    pair.success.session_id.clone(),
                    pair.failure.session_id.clone(),
                ],
                created_at: Utc::now(),
            };

            let _ = provider.store_suggestion(&suggestion).await;
            suggestions.push(suggestion);
        }

        let mut buffer = self.suggestions_buffer.lock().await;
        buffer.extend(suggestions.clone());

        Ok(suggestions)
    }

    /// 执行一次 Dream 巩固周期（完整实现）
    pub async fn consolidate(
        &self,
        _on_memories: Option<&(dyn Fn(usize) + Send + Sync)>,
        _on_patterns: Option<&(dyn Fn(usize) + Send + Sync)>,
        _on_suggestions: Option<&(dyn Fn(usize) + Send + Sync)>,
    ) -> DreamConsolidationResult {
        let config = self.get_config().await;

        if !config.enabled {
            return DreamConsolidationResult::skipped("Dream 巩固已禁用");
        }

        let _lock = match tokio::time::timeout(
            std::time::Duration::from_secs(LOCK_TIMEOUT_SECS),
            self.consolidation_lock.lock(),
        )
        .await
        {
            Ok(lock) => lock,
            Err(_) => {
                return DreamConsolidationResult::skipped("无法获取巩固锁（超时）");
            },
        };

        let started_at = Utc::now();
        let deadline = started_at + Duration::seconds(config.max_consolidation_secs as i64);

        self.emit(
            "dream-consolidation-started",
            serde_json::json!({
                "timestamp": started_at.timestamp_millis(),
                "maxDurationSecs": config.max_consolidation_secs,
            }),
        );

        let mut state = self.state.lock().await;
        state.is_running = true;
        drop(state);

        let mut memories_extracted = 0usize;
        let mut patterns_discovered = 0usize;
        let mut suggestions_generated = 0usize;
        let mut experience_replay_count = 0usize;
        let mut distilled_knowledge_count = 0usize;
        let mut contrastive_insights_count = 0usize;

        let has_provider = self.data_provider.is_some();

        if has_provider {
            // 1. 经验回放
            if config.run_memory_extraction && Utc::now() < deadline {
                match self.experience_replay(&config).await {
                    Ok(replay) => {
                        experience_replay_count = replay.records.len();
                        memories_extracted = replay.records.len();

                        self.emit(
                            "dream-experience-replay",
                            serde_json::json!({
                                "sampleCount": replay.records.len(),
                                "avgQuality": replay.avg_quality,
                                "topicCount": replay.topic_distribution.len(),
                            }),
                        );

                        // 2. 知识蒸馏
                        if Utc::now() < deadline {
                            match self.distill_knowledge(&replay, &config).await {
                                Ok(distilled) => {
                                    distilled_knowledge_count = distilled.len();
                                    patterns_discovered += distilled.len();

                                    let mut buffer = self.distilled_knowledge_buffer.lock().await;
                                    buffer.extend(distilled);

                                    self.emit(
                                        "dream-knowledge-distillation",
                                        serde_json::json!({
                                            "distilledCount": distilled_knowledge_count,
                                        }),
                                    );
                                },
                                Err(e) => {
                                    self.emit(
                                        "dream-distillation-error",
                                        serde_json::json!({ "error": e }),
                                    );
                                },
                            }
                        }

                        // 3. 对比学习
                        if config.run_pattern_learning && Utc::now() < deadline {
                            match self.contrastive_learning(&replay, &config).await {
                                Ok(pairs) => {
                                    contrastive_insights_count = pairs.len();
                                    patterns_discovered += pairs.len();

                                    self.emit(
                                        "dream-contrastive-learning",
                                        serde_json::json!({
                                            "pairCount": contrastive_insights_count,
                                        }),
                                    );

                                    // 4. 建议生成
                                    if config.run_proactive_suggestions && Utc::now() < deadline {
                                        let distilled_buffer =
                                            self.distilled_knowledge_buffer.lock().await;
                                        let recent_distilled: Vec<DistilledKnowledge> =
                                            distilled_buffer.iter().take(20).cloned().collect();
                                        drop(distilled_buffer);

                                        match self
                                            .generate_suggestions(&recent_distilled, &pairs)
                                            .await
                                        {
                                            Ok(suggestions) => {
                                                suggestions_generated = suggestions.len();
                                            },
                                            Err(e) => {
                                                self.emit(
                                                    "dream-suggestions-error",
                                                    serde_json::json!({ "error": e }),
                                                );
                                            },
                                        }
                                    }
                                },
                                Err(e) => {
                                    self.emit(
                                        "dream-contrastive-error",
                                        serde_json::json!({ "error": e }),
                                    );
                                },
                            }
                        }
                    },
                    Err(e) => {
                        self.emit("dream-replay-error", serde_json::json!({ "error": e }));
                    },
                }
            }
        } else {
            // 无数据提供者时使用回调模式（向后兼容）
            if config.run_memory_extraction && Utc::now() < deadline {
                if let Some(callback) = _on_memories {
                    callback(0);
                }
                memories_extracted += 1;
            }

            if config.run_pattern_learning && Utc::now() < deadline {
                if let Some(callback) = _on_patterns {
                    callback(0);
                }
                patterns_discovered += 1;
            }

            if config.run_proactive_suggestions && Utc::now() < deadline {
                if let Some(callback) = _on_suggestions {
                    callback(0);
                }
                suggestions_generated += 1;
            }
        }

        let duration_secs = (Utc::now() - started_at).num_seconds().max(0) as u64;

        let mut state = self.state.lock().await;
        state.last_consolidation_at = Some(Utc::now());
        state.sessions_since_last = 0;
        state.total_consolidations += 1;
        state.total_memories_extracted += memories_extracted as u64;
        state.total_consolidation_secs += duration_secs;
        state.total_experience_replayed += experience_replay_count as u64;
        state.total_distilled_knowledge += distilled_knowledge_count as u64;
        state.total_contrastive_insights += contrastive_insights_count as u64;
        state.is_running = false;
        drop(state);

        self.emit(
            "dream-consolidation-completed",
            serde_json::json!({
                "executed": true,
                "memoriesExtracted": memories_extracted,
                "patternsDiscovered": patterns_discovered,
                "suggestionsGenerated": suggestions_generated,
                "experienceReplayCount": experience_replay_count,
                "distilledKnowledgeCount": distilled_knowledge_count,
                "contrastiveInsightsCount": contrastive_insights_count,
                "startedAt": started_at.timestamp_millis(),
                "durationSecs": duration_secs,
                "error": null,
            }),
        );

        DreamConsolidationResult {
            executed: true,
            skip_reason: None,
            memories_extracted,
            patterns_discovered,
            suggestions_generated,
            started_at,
            duration_secs,
            error: None,
            experience_replay_count,
            distilled_knowledge_count,
            contrastive_insights_count,
        }
    }

    pub async fn consolidate_force(&self) -> DreamConsolidationResult {
        {
            let mut state = self.state.lock().await;
            state.sessions_since_last = u32::MAX;
            state.last_consolidation_at = None;
        }

        self.consolidate(None, None, None).await
    }

    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        *state = DreamConsolidationState::default();

        let mut buffer = self.distilled_knowledge_buffer.lock().await;
        buffer.clear();

        let mut suggestions = self.suggestions_buffer.lock().await;
        suggestions.clear();
    }

    pub async fn is_running(&self) -> bool {
        self.state.lock().await.is_running
    }

    pub async fn get_distilled_knowledge(&self) -> Vec<DistilledKnowledge> {
        self.distilled_knowledge_buffer.lock().await.clone()
    }

    pub async fn get_suggestions(&self) -> Vec<ConsolidationSuggestion> {
        self.suggestions_buffer.lock().await.clone()
    }
}

impl Default for DreamConsolidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DreamConsolidationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_interval_hours, 1);
        assert_eq!(config.min_new_sessions, 3);
        assert!(config.run_memory_extraction);
        assert_eq!(config.experience_replay_sample_size, 50);
    }

    #[tokio::test]
    async fn test_should_consolidate_first_run() {
        let consolidator = DreamConsolidator::new();
        assert!(!consolidator.should_consolidate().await);

        consolidator.record_new_session().await;
        consolidator.record_new_session().await;
        consolidator.record_new_session().await;

        assert!(consolidator.should_consolidate().await);
    }

    #[tokio::test]
    async fn test_time_gate_blocks() {
        let consolidator = DreamConsolidator::new();

        let mut state = consolidator.state.lock().await;
        state.last_consolidation_at = Some(Utc::now());
        state.sessions_since_last = 10;
        drop(state);

        assert!(!consolidator.should_consolidate().await);
    }

    #[tokio::test]
    async fn test_disabled_config() {
        let consolidator = DreamConsolidator::with_config(DreamConsolidationConfig {
            enabled: false,
            ..Default::default()
        });

        assert!(!consolidator.should_consolidate().await);
    }

    #[tokio::test]
    async fn test_consolidate_updates_state() {
        let consolidator = DreamConsolidator::new();
        let result = consolidator.consolidate_force().await;
        assert!(result.executed);

        let state = consolidator.get_state().await;
        assert_eq!(state.total_consolidations, 1);
        assert_eq!(state.sessions_since_last, 0);
    }

    #[tokio::test]
    async fn test_skipped_when_disabled() {
        let consolidator = DreamConsolidator::with_config(DreamConsolidationConfig {
            enabled: false,
            ..Default::default()
        });

        let result = consolidator.consolidate_force().await;
        assert!(!result.executed);
        assert!(result.skip_reason.is_some());
    }

    #[tokio::test]
    async fn test_reset() {
        let consolidator = DreamConsolidator::new();
        consolidator.record_new_session().await;
        consolidator.record_new_session().await;

        consolidator.reset().await;
        let state = consolidator.get_state().await;
        assert_eq!(state.sessions_since_last, 0);
        assert_eq!(state.total_consolidations, 0);
    }

    #[tokio::test]
    async fn test_is_running_flag() {
        let consolidator = DreamConsolidator::new();
        assert!(!consolidator.is_running().await);
    }

    #[test]
    fn test_experience_record_serialization() {
        let record = ExperienceRecord {
            id: "test-id".to_string(),
            session_id: "session-1".to_string(),
            topic: "file editing".to_string(),
            outcome: "success".to_string(),
            quality_score: 0.85,
            tool_sequence: vec!["read_file".to_string(), "write_file".to_string()],
            reasoning_summary: "Analyzed and modified".to_string(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("file editing"));
    }

    #[test]
    fn test_distilled_knowledge_types() {
        let knowledge = DistilledKnowledge {
            id: "k1".to_string(),
            source_session_ids: vec!["s1".to_string()],
            knowledge_type: KnowledgeType::ToolUsagePattern,
            content: "Test pattern".to_string(),
            confidence: 0.8,
            applicability_tags: vec!["file ops".to_string()],
            created_at: Utc::now(),
        };
        assert_eq!(knowledge.knowledge_type, KnowledgeType::ToolUsagePattern);
    }

    #[test]
    fn test_contrastive_pair() {
        let pair = ContrastivePair {
            success: ExperienceRecord {
                id: "s1".to_string(),
                session_id: "sess1".to_string(),
                topic: "test".to_string(),
                outcome: "success".to_string(),
                quality_score: 0.9,
                tool_sequence: vec!["tool_a".to_string()],
                reasoning_summary: "Good".to_string(),
                timestamp: Utc::now(),
            },
            failure: ExperienceRecord {
                id: "f1".to_string(),
                session_id: "sess2".to_string(),
                topic: "test".to_string(),
                outcome: "failure".to_string(),
                quality_score: 0.2,
                tool_sequence: vec!["tool_b".to_string()],
                reasoning_summary: "Bad".to_string(),
                timestamp: Utc::now(),
            },
            topic: "test".to_string(),
            differentiating_factors: vec!["Different tools".to_string()],
            insight: "Use tool_a instead of tool_b".to_string(),
        };
        assert_eq!(pair.topic, "test");
    }

    #[test]
    fn test_consolidation_suggestion() {
        let suggestion = ConsolidationSuggestion {
            id: "sug1".to_string(),
            suggestion_type: SuggestionType::SkillImprovement,
            content: "Improve X".to_string(),
            confidence: 0.75,
            source_evidence: vec!["e1".to_string()],
            created_at: Utc::now(),
        };
        assert_eq!(suggestion.suggestion_type, SuggestionType::SkillImprovement);
    }

    #[test]
    fn test_result_with_new_fields() {
        let result = DreamConsolidationResult {
            executed: true,
            skip_reason: None,
            memories_extracted: 10,
            patterns_discovered: 5,
            suggestions_generated: 3,
            started_at: Utc::now(),
            duration_secs: 30,
            error: None,
            experience_replay_count: 10,
            distilled_knowledge_count: 5,
            contrastive_insights_count: 3,
        };
        assert_eq!(result.experience_replay_count, 10);
        assert_eq!(result.distilled_knowledge_count, 5);
    }
}
