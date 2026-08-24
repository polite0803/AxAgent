// SPDX-License-Identifier: AGPL-3.0-only

//! 反思系统共享 DTO
//!
//! 本模块定义反思系统的纯数据 DTO,被 agent crate 的 `Reflector` 与
//! trajectory crate 的 `WorkflowReflector` 实现共同复用。
//!
//! 设计原则:仅 DTO + 构造器(builder 方法),不含业务逻辑。
//! `Reflector` / `InsightGenerator` 等行为结构留在 agent crate。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── 任务执行记录(任务级反思的输入) ──

/// 任务执行记录:agent crate `Reflector::reflect()` 的输入。
///
/// 工作流反思器(`WorkflowReflector`)不直接使用本结构,
/// 而是使用 `crate::workflow_reflection::WorkflowExecutionRecord`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskExecutionRecord {
    pub task_id: String,
    pub task_description: String,
    pub result: Option<serde_json::Value>,
    pub success: bool,
    pub error: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub tools_used: Vec<String>,
    pub iterations: usize,
}

impl TaskExecutionRecord {
    pub fn new(
        task_id: String,
        task_description: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Self {
        Self {
            task_id,
            task_description,
            result: None,
            success: false,
            error: None,
            start_time,
            end_time,
            duration_ms: 0,
            tools_used: Vec::new(),
            iterations: 0,
        }
    }

    pub fn with_result(mut self, result: serde_json::Value) -> Self {
        self.result = Some(result);
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self.success = false;
        self
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools_used = tools;
        self
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn compute_duration(&mut self) {
        self.duration_ms =
            self.end_time.signed_duration_since(self.start_time).num_milliseconds() as u64;
    }
}

// ── 质量指标 ──

/// 六维度质量指标(任务级与工作流级反思共用)。
///
/// `overall_weighted_score` 范围 0.0-10.0,与 `Reflection::quality_score` 对齐。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityMetrics {
    pub task_success_score: f32,
    pub tool_efficiency_score: f32,
    pub iteration_efficiency_score: f32,
    pub time_efficiency_score: f32,
    pub error_recovery_score: f32,
    pub goal_completion_score: f32,
    pub overall_weighted_score: f32,
}

// ── 反思结果(任务级与工作流级共用) ──

/// 反思结果。
///
/// 工作流反思器复用本结构:
/// - `task_id` 字段存放 `workflow_id` 或 `execution_id`
/// - `error_patterns` / `reusable_patterns` / `improvement_suggestions` 直接复用
/// - 工作流专有的结构化数据(瓶颈节点、节点级模式等)通过 `metadata` 承载
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reflection {
    pub task_id: String,
    pub timestamp: DateTime<Utc>,
    pub quality_score: u8,
    pub quality_analysis: String,
    pub efficiency_analysis: String,
    pub error_patterns: Vec<String>,
    pub reusable_patterns: Vec<String>,
    pub knowledge_suggestions: Vec<String>,
    pub improvement_suggestions: Vec<String>,
    pub overall_summary: String,
    pub quality_metrics: Option<QualityMetrics>,
    /// 工作流反思专有结构化数据(任务级反思留空)。
    ///
    /// 序列化后为 `WorkflowReflectionMetadata` 结构。
    /// 任务级反思器不写入本字段,工作流反思器写入瓶颈节点、节点级模式等。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Reflection {
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            timestamp: Utc::now(),
            quality_score: 0,
            quality_analysis: String::new(),
            efficiency_analysis: String::new(),
            error_patterns: Vec::new(),
            reusable_patterns: Vec::new(),
            knowledge_suggestions: Vec::new(),
            improvement_suggestions: Vec::new(),
            overall_summary: String::new(),
            quality_metrics: None,
            metadata: None,
        }
    }

    pub fn with_quality(mut self, score: u8, analysis: String) -> Self {
        self.quality_score = score.clamp(1, 10);
        self.quality_analysis = analysis;
        self
    }

    pub fn with_quality_metrics(mut self, metrics: QualityMetrics) -> Self {
        self.quality_score = (metrics.overall_weighted_score.round() as u8).clamp(1, 10);
        self.quality_metrics = Some(metrics);
        self
    }

    pub fn with_efficiency(mut self, analysis: String) -> Self {
        self.efficiency_analysis = analysis;
        self
    }

    pub fn with_patterns(mut self, errors: Vec<String>, reusable: Vec<String>) -> Self {
        self.error_patterns = errors;
        self.reusable_patterns = reusable;
        self
    }

    pub fn with_knowledge(mut self, suggestions: Vec<String>) -> Self {
        self.knowledge_suggestions = suggestions;
        self
    }

    pub fn with_improvements(mut self, suggestions: Vec<String>) -> Self {
        self.improvement_suggestions = suggestions;
        self
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.overall_summary = summary;
        self
    }

    /// 写入工作流反思专有的结构化 metadata。
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

// ── 反思配置 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflectionConfig {
    pub enabled: bool,
    pub min_quality_threshold: u8,
    pub store_insights: bool,
    pub max_history: usize,
}

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self { enabled: true, min_quality_threshold: 5, store_insights: true, max_history: 100 }
    }
}
