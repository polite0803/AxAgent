// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流反思 trait 契约(三层 trait 之一:Reflector)
//!
//! 在工作流执行完成或节点级失败时触发反思,复用 `Reflection` DTO,
//! 工作流专有的结构化数据通过 `Reflection::metadata` 承载。
//!
//! 触发时机(由 wiring 层 / rt-workflow 调用方决定):
//! - 工作流整体执行完成(成功/失败/部分完成)
//! - 节点级失败(细粒度,流量较大)
//! - 异步 spawn 后台执行,不阻塞主流程

use crate::reflection_types::Reflection;
use crate::workflow_types::{WorkflowEdge, WorkflowNode};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 工作流执行记录(反思的输入) ──

/// 工作流执行记录:工作流反思器的输入。
///
/// 字段与 rt-workflow 的 `Workflow` + `NodeExecutionRecord` 对齐,
/// 但纯 DTO,不含方法。rt-workflow 负责构造本结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionRecord {
    pub workflow_id: String,
    pub execution_id: String,
    pub template_id: Option<String>,
    pub template_version: Option<i32>,
    pub status: WorkflowRunStatus,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub duration_ms: u64,
    pub nodes: Vec<NodeExecutionSnapshot>,
    pub edges: Vec<WorkflowEdge>,
    pub template_nodes: Vec<WorkflowNode>,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error_context: Option<WorkflowErrorContext>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowRunStatus {
    Completed,
    PartiallyCompleted,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionSnapshot {
    pub node_id: String,
    pub node_type: String,
    pub node_name: Option<String>,
    pub status: NodeRunStatus,
    pub attempts: u32,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub execution_time_ms: Option<u64>,
    pub error: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub sub_workflow_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeRunStatus {
    Completed,
    Failed,
    Skipped,
    Running,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowErrorContext {
    pub failed_node_id: String,
    pub failed_node_name: String,
    pub error_code: String,
    pub error_message: String,
    pub timestamp: i64,
    pub last_output: Option<serde_json::Value>,
}

// ── 工作流反思专有结构化数据(承载于 Reflection::metadata) ──

/// 工作流反思专有结构化数据,序列化后写入 `Reflection::metadata`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReflectionMetadata {
    pub workflow_id: String,
    pub execution_id: String,
    pub bottleneck_nodes: Vec<BottleneckNode>,
    pub node_patterns: Vec<WorkflowPattern>,
    pub failed_node_analysis: Option<NodeFailureAnalysis>,
    pub proposed_changes: Vec<ProposedChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckNode {
    pub node_id: String,
    pub node_type: String,
    pub reason: BottleneckReason,
    pub impact_score: f32,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BottleneckReason {
    HighLatency,
    HighFailureRate,
    HighRetryCount,
    ResourceHeavy,
    SequentialBlocking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub node_ids: Vec<String>,
    pub frequency: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeFailureAnalysis {
    pub node_id: String,
    pub root_cause: String,
    pub failure_category: FailureCategory,
    pub recovery_strategy: String,
    pub related_nodes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureCategory {
    ConfigError,
    InputMismatch,
    OutputSchemaMismatch,
    ToolUnavailable,
    Timeout,
    PermissionDenied,
    LlmError,
    LogicError,
    ExternalService,
    Unknown,
}

/// 建议的变更(可由反思器直接产出,或由 `WorkflowOptimizer` 进一步生成)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProposedChange {
    UpdateConfig {
        node_id: String,
        patch: serde_json::Value,
    },
    ReplaceNode {
        node_id: String,
        new_type: String,
        new_config: serde_json::Value,
    },
    /// `node` 装箱以避免 `large_enum_variant`(`WorkflowNode` 体积较大)。
    AddNode {
        after: String,
        node: Box<WorkflowNode>,
    },
    RemoveNode {
        node_id: String,
    },
    RewireEdge {
        from: String,
        to: String,
        new_target: String,
    },
    RefinePrompt {
        node_id: String,
        new_prompt: String,
    },
    TuneRetry {
        node_id: String,
        max_attempts: u32,
        backoff_ms: u64,
    },
}

// ── Trait 契约 ──

/// 工作流反思器:从执行记录生成反思结果。
///
/// **三层 trait 之一**(Reflector / Evolver / Optimizer)。
///
/// 实现方:trajectory crate(复用 `RLEngine` 奖励计算 + `PatternAnalyzer` 模式提取)。
/// 调用方:rt-workflow(执行完成钩子)、wiring 层(批量分析)。
///
/// 触发时机:
/// - `reflect()`:工作流整体执行完成
/// - `reflect_node()`:节点级失败(细粒度,流量较大)
///
/// 执行方式:异步(`tokio::spawn`),不阻塞主流程。
#[async_trait]
pub trait WorkflowReflector: Send + Sync {
    /// 工作流整体执行完成后的反思。
    ///
    /// 返回的 `Reflection` 中:
    /// - `task_id` 存放 `execution_id`
    /// - `metadata` 存放序列化后的 `WorkflowReflectionMetadata`
    async fn reflect(&self, record: &WorkflowExecutionRecord) -> Result<Reflection, String>;

    /// 节点级失败时的反思(细粒度)。
    ///
    /// 仅分析该节点及其依赖链,产出 `NodeFailureAnalysis` 与针对性 `ProposedChange`。
    /// 用于在执行过程中实时反馈,不阻塞工作流继续运行。
    async fn reflect_node(
        &self,
        record: &WorkflowExecutionRecord,
        failed_node: &NodeExecutionSnapshot,
    ) -> Result<Reflection, String>;

    /// 批量反思(用于模式挖掘)。
    async fn reflect_batch(
        &self,
        records: &[WorkflowExecutionRecord],
    ) -> Result<Vec<Reflection>, String> {
        let mut out = Vec::with_capacity(records.len());
        for r in records {
            out.push(self.reflect(r).await?);
        }
        Ok(out)
    }

    /// 跨执行的模式聚合(基于历史反思统计高频模式)。
    async fn aggregate_patterns(
        &self,
        records: &[WorkflowExecutionRecord],
    ) -> Result<Vec<WorkflowPattern>, String>;

    /// 历史反思查询(由实现层决定存储方式)。
    async fn get_history(&self, workflow_id: &str, limit: usize)
    -> Result<Vec<Reflection>, String>;
}
