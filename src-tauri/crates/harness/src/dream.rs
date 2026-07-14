// SPDX-License-Identifier: AGPL-3.0-only
//! Dream Consolidation 契约 — 统一的类型定义和 trait 接口
//!
//! 定义经验回放、知识蒸馏、对比学习、建议生成的全套 DTO 和 trait 接口。
//! trajectory crate 中的实现层以此为单一权威来源。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
            min_interval_hours: 1,
            min_new_sessions: 3,
            max_consolidation_secs: 120,
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
// 经验记录
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

// ---------------------------------------------------------------------------
// 知识蒸馏
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KnowledgeType {
    ToolUsagePattern,
    ReasoningStrategy,
    ErrorRecovery,
    TaskDecomposition,
    OptimizationHint,
}

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

// ---------------------------------------------------------------------------
// 建议
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuggestionType {
    SkillImprovement,
    NewSkillProposal,
    ToolUsageOptimization,
    ErrorPrevention,
    WorkflowEnhancement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationSuggestion {
    pub id: String,
    pub suggestion_type: SuggestionType,
    pub content: String,
    pub confidence: f64,
    pub source_evidence: Vec<String>,
    pub created_at: DateTime<Utc>,
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
// 巩固状态
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
// 事件发射器
// ---------------------------------------------------------------------------

pub type DreamEventEmitter = Option<Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>>;

// ---------------------------------------------------------------------------
// 数据提供者 trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ConsolidationDataProvider: Send + Sync {
    /// 获取最近的经验记录（用于经验回放）
    async fn fetch_recent_experiences(&self, limit: usize) -> Result<Vec<ExperienceRecord>, String>;

    /// 按主题查询经验
    async fn fetch_experience_by_topic(&self, _topic: &str) -> Result<Vec<ExperienceRecord>, String> {
        self.fetch_recent_experiences(100).await
    }

    /// 存储已蒸馏的知识
    async fn store_distilled_knowledge(&self, knowledge: &DistilledKnowledge) -> Result<(), String>;

    /// 存储巩固建议
    async fn store_suggestion(&self, suggestion: &ConsolidationSuggestion) -> Result<(), String>;

    /// 查询已有知识（按类型过滤）
    async fn fetch_existing_knowledge(
        &self,
        knowledge_type: &KnowledgeType,
    ) -> Result<Vec<DistilledKnowledge>, String>;
}

// ---------------------------------------------------------------------------
// 巩固器 trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait DreamConsolidator: Send + Sync {
    /// 执行一次 Dream 巩固周期
    async fn consolidate(&self) -> Result<DreamConsolidationResult, String>;

    /// 检查是否应该执行巩固
    async fn should_consolidate(&self) -> Result<bool, String>;

    /// 获取当前配置
    async fn config(&self) -> DreamConsolidationConfig;
}
