// SPDX-License-Identifier: AGPL-3.0-only

//! 编排计划类型
//!
//! 定义任务分解、子任务、编排策略等核心类型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_types::ToolDef;

// ── Orchestration Strategy ──────────────────────────────────────────

/// 编排策略枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationStrategy {
    /// 串行执行
    #[default]
    Ordered,
    /// 并行扇出
    FanOut,
    /// 流水线（阶段内并行，阶段间串行）
    Pipeline,
    /// 竞速（取第一个完成的结果）
    Race,
    /// 辩论（多 agent 辩论后裁决）
    Debate,
    /// 动态（运行时确定拓扑）
    Dynamic,
}

impl OrchestrationStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ordered => "ordered",
            Self::FanOut => "fan_out",
            Self::Pipeline => "pipeline",
            Self::Race => "race",
            Self::Debate => "debate",
            Self::Dynamic => "dynamic",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "ordered" => Some(Self::Ordered),
            "fan_out" => Some(Self::FanOut),
            "pipeline" => Some(Self::Pipeline),
            "race" => Some(Self::Race),
            "debate" => Some(Self::Debate),
            "dynamic" => Some(Self::Dynamic),
            _ => None,
        }
    }
}

// ── SubTaskStatus ───────────────────────────────────────────────────

/// 子任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubTaskStatus {
    /// 等待依赖完成
    Pending,
    /// 就绪可派发
    Ready,
    /// 正在执行
    Running,
    /// 已完成
    Completed,
    /// 失败（可能触发重规划）
    Failed,
    /// 因上游失败或条件排除而跳过
    Skipped,
}

impl std::fmt::Display for SubTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Ready => write!(f, "Ready"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Skipped => write!(f, "Skipped"),
        }
    }
}

impl SubTaskStatus {
    /// 是否为终态（Completed / Failed / Skipped）—— 终态禁止回退到非终态（P1-4）
    pub fn is_terminal(&self) -> bool {
        matches!(self, SubTaskStatus::Completed | SubTaskStatus::Failed | SubTaskStatus::Skipped)
    }
}

// ── SubTask ───────────────────────────────────────────────────────────

/// 从高层任务分解出的单个子任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// 唯一标识符
    pub id: String,
    /// 可读任务名
    pub name: String,
    /// 详细任务描述
    pub description: String,
    /// 最适合此任务的 Agent 角色
    pub role: String,
    /// 必须在此任务开始前完成的前置子任务 ID
    pub dependencies: Vec<String>,
    /// 当前执行状态
    pub status: SubTaskStatus,
    /// Worker Agent 的系统提示词覆盖
    pub system_prompt: Option<String>,
    /// 执行上下文中的期望输出变量名
    pub output_var: String,
    /// 失败时的错误信息
    pub error: Option<String>,
    /// 重试次数
    pub attempts: u32,
    /// 最大允许重试次数
    pub max_retries: u32,
    /// 此子任务 Agent 可用的工具
    #[serde(default)]
    pub tools: Vec<ToolDef>,
    /// 是否使用多智能体协作
    #[serde(default)]
    pub multi_agent: bool,
    /// 多智能体协作模式
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordination_mode: Option<String>,
    /// 多智能体最大协作轮数
    #[serde(default)]
    pub max_rounds: u32,
    /// 是否支持并行执行
    #[serde(default)]
    pub parallel_supported: bool,
}

impl SubTask {
    pub fn new(id: String, name: String, description: String, role: String) -> Self {
        Self {
            id,
            name,
            description,
            role,
            dependencies: Vec::new(),
            status: SubTaskStatus::Pending,
            system_prompt: None,
            output_var: format!(
                "subtask_output_{}",
                uuid::Uuid::new_v4().to_string().replace('-', "_")
            ),
            error: None,
            attempts: 0,
            max_retries: 3,
            tools: Vec::new(),
            multi_agent: false,
            coordination_mode: None,
            max_rounds: 3,
            parallel_supported: false,
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    /// 设置为多智能体模式
    pub fn with_multi_agent(mut self, mode: &str, max_rounds: u32) -> Self {
        self.multi_agent = true;
        self.coordination_mode = Some(mode.to_string());
        self.max_rounds = max_rounds;
        self
    }

    /// 支持并行执行
    pub fn with_parallel(mut self) -> Self {
        self.parallel_supported = true;
        self
    }
}

// ── DecompositionPlan ────────────────────────────────────────────────────

/// 完整分解结果：mission → SubTask[]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionPlan {
    /// 原始任务描述
    pub mission: String,
    /// 分解使用的策略
    pub strategy: OrchestrationStrategy,
    /// 有序子任务列表
    pub sub_tasks: Vec<SubTask>,
    /// 最大并行 worker 数
    pub max_parallel: u32,
    /// 最大重规划轮数
    pub max_replans: u32,
    /// 当前重规划计数器
    pub replan_count: u32,
    /// 计划创建时间戳
    pub created_at: DateTime<Utc>,
}

impl DecompositionPlan {
    pub fn new(mission: String, strategy: OrchestrationStrategy) -> Self {
        Self {
            mission,
            strategy,
            sub_tasks: Vec::new(),
            max_parallel: 4,
            max_replans: 3,
            replan_count: 0,
            created_at: Utc::now(),
        }
    }

    /// 返回所有依赖已满足且可派发的子任务（Pending 等待依赖 / Ready 依赖已满足待派发，P1-1）
    pub fn ready_sub_tasks(&self) -> Vec<&SubTask> {
        self.sub_tasks
            .iter()
            .filter(|st| {
                matches!(st.status, SubTaskStatus::Pending | SubTaskStatus::Ready)
                    && st.dependencies.iter().all(|dep_id| {
                        self.sub_tasks
                            .iter()
                            .any(|s| s.id == *dep_id && s.status == SubTaskStatus::Completed)
                    })
            })
            .collect()
    }

    /// 是否所有子任务都处于终止状态
    pub fn is_terminal(&self) -> bool {
        self.sub_tasks.iter().all(|st| {
            matches!(
                st.status,
                SubTaskStatus::Completed | SubTaskStatus::Failed | SubTaskStatus::Skipped
            )
        })
    }

    /// 已完成子任务数
    pub fn completed_count(&self) -> usize {
        self.sub_tasks.iter().filter(|st| st.status == SubTaskStatus::Completed).count()
    }

    /// 失败子任务数
    pub fn failed_count(&self) -> usize {
        self.sub_tasks.iter().filter(|st| st.status == SubTaskStatus::Failed).count()
    }
}

// ── OrchestrationError ────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum OrchestrationError {
    #[error("Decomposition failed: {0}")]
    DecompositionFailed(String),

    #[error("Subgraph generation failed: {0}")]
    SubgraphGenerationFailed(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Replan failed: {0}")]
    ReplanFailed(String),

    #[error("Max replan rounds ({0}) exceeded")]
    MaxReplansExceeded(u32),

    #[error("Subtask not found: {0}")]
    SubTaskNotFound(String),

    #[error("No ready sub-tasks to dispatch")]
    NoReadyTasks,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Dispatch failed: {0}")]
    DispatchFailed(String),
}
