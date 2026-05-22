//! 工作流共享类型定义。
//!
//! 节点类型统一为 axagent_core::workflow_types::WorkflowNode（15 种），
//! 执行统一由 WorkEngine + NodeDispatcher 负责。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use axagent_core::workflow_types::{WorkflowEdge, WorkflowNode};

// ── 节点运行时状态 ──

/// 节点运行时状态（等价于原 StepStatus）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

// ── 工作流容器 ──

/// 工作流运行时容器。nodes/edges 来自 WorkflowNode/WorkflowEdge，
/// 运行时状态（status/results/node_runtime_states）存储在内存中。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub status: WorkflowStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    /// 节点执行结果 keyed by node_id
    pub results: HashMap<String, serde_json::Value>,
    /// 每个节点的运行时状态
    pub node_states: HashMap<String, NodeRuntimeState>,
}

/// 单个节点的运行时追踪状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRuntimeState {
    pub status: NodeStatus,
    pub attempts: u32,
    pub error: Option<String>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

impl Default for NodeRuntimeState {
    fn default() -> Self {
        Self {
            status: NodeStatus::Pending,
            attempts: 0,
            error: None,
            started_at: None,
            completed_at: None,
        }
    }
}

// ── 工作流状态 ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Created,
    Running,
    Completed,
    PartiallyCompleted,
    Failed,
    Cancelled,
}

// ── 错误类型 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowError {
    DuplicateNodeId(String),
    InvalidDependency {
        node_id: String,
        missing_dep: String,
    },
    WorkflowNotFound,
    NodeNotFound,
    CycleDetected,
    SerializationError(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateNodeId(id) => write!(f, "重复节点 ID: {id}"),
            Self::InvalidDependency {
                node_id,
                missing_dep,
            } => write!(f, "节点 '{node_id}' 依赖不存在的 '{missing_dep}'"),
            Self::WorkflowNotFound => write!(f, "工作流未找到"),
            Self::NodeNotFound => write!(f, "节点未找到"),
            Self::CycleDetected => write!(f, "工作流中存在循环依赖"),
            Self::SerializationError(msg) => write!(f, "序列化错误: {msg}"),
        }
    }
}

impl std::error::Error for WorkflowError {}

// ── 辅助函数 ──

pub(crate) fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(crate) fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
