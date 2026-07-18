// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流优化建议 trait 契约(三层 trait 之三:Optimizer)
//!
//! 不走完整遗传进化,只针对当前模板生成可执行优化建议。
//! 与 `commands/workflow_ai_diagnose.rs`(LLM 单次诊断)互补,不替代。
//!
//! 数据源:历史反思(`Reflection`)+ 模板(`WorkflowTemplateData`),
//! 产出 `WorkflowSuggestion`,可由 wiring 层批量应用或人工审核。

use crate::reflection_types::Reflection;
use crate::workflow_types::WorkflowTemplateData;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 优化建议 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSuggestion {
    pub id: String,
    pub category: SuggestionCategory,
    pub priority: SuggestionPriority,
    pub target_node_id: Option<String>,
    pub description: String,
    pub proposed_change: ProposedChange,
    pub confidence: f32,
    /// 预期收益(0.0-1.0),由 `estimate_impact` 计算。
    #[serde(default)]
    pub estimated_impact: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SuggestionCategory {
    NodeConfig,
    NodeReplacement,
    EdgeRewire,
    PromptRefine,
    ErrorHandling,
    VariableMisconfig,
    ResourceTuning,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SuggestionPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// 建议的变更(与 `workflow_reflection::ProposedChange` 同构,
/// 此处独立定义以避免 Optimizer 依赖 Reflector 模块)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProposedChange {
    UpdateConfig { node_id: String, patch: serde_json::Value },
    ReplaceNode { node_id: String, new_type: String, new_config: serde_json::Value },
    AddNode { after: String, node: serde_json::Value },
    RemoveNode { node_id: String },
    RewireEdge { from: String, to: String, new_target: String },
    RefinePrompt { node_id: String, new_prompt: String },
    TuneRetry { node_id: String, max_attempts: u32, backoff_ms: u64 },
}

// ── Trait 契约 ──

/// 工作流优化器:从反思结果生成可执行的优化建议(不修改模板,只产出建议)。
///
/// **三层 trait 之三**(Reflector / Evolver / Optimizer)。
///
/// 实现方:trajectory crate(复用 `ClosedLoopService` 的 nudge 机制
/// + `DreamConsolidator` 的跨模板模式蒸馏)。
/// 调用方:gateway(对外暴露 API)、wiring 层(批量后台任务)。
///
/// 与 `commands/workflow_ai_diagnose.rs` 互补:
/// - `workflow_ai_diagnose`:LLM 单次推理,冷启动/手动诊断
/// - 本 trait:基于历史反思的持续优化建议,自动触发
#[async_trait]
pub trait WorkflowOptimizer: Send + Sync {
    /// 基于单次反思生成优化建议。
    async fn suggest(
        &self,
        template: &WorkflowTemplateData,
        reflection: &Reflection,
    ) -> Result<Vec<WorkflowSuggestion>, String>;

    /// 基于历史反思批量生成优化建议。
    async fn suggest_batch(
        &self,
        template: &WorkflowTemplateData,
        reflections: &[Reflection],
    ) -> Result<Vec<WorkflowSuggestion>, String>;

    /// 应用建议到模板(返回新模板,不修改原模板)。
    ///
    /// 调用方决定是否持久化:可由人工审核后调用 `repo.update_template`,
    /// 或由 wiring 层自动应用(需配置 `auto_apply_threshold`)。
    async fn apply_suggestions(
        &self,
        template: &WorkflowTemplateData,
        suggestions: &[WorkflowSuggestion],
    ) -> Result<WorkflowTemplateData, String>;

    /// 评估建议的预期收益(用于优先级排序)。
    async fn estimate_impact(
        &self,
        template: &WorkflowTemplateData,
        suggestion: &WorkflowSuggestion,
    ) -> Result<f32, String>;
}
