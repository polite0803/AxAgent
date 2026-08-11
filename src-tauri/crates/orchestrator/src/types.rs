// SPDX-License-Identifier: AGPL-3.0-only

//! Core types for the Orchestrator system — 从 harness 重导出共享类型
//!
//! 共享类型（SubTask、DecompositionPlan、OrchestrationError 等）的权威定义
//! 已迁移至 `axagent-harness::industry_orchestration::plan`。
//! 仅保留 orchestrator 特有的事件和交接类型。

// ── 从 harness 重导出共享类型 ──
pub use axagent_harness::industry_orchestration::plan::{
    DecompositionPlan, OrchestrationError, OrchestrationStrategy, SubTask, SubTaskStatus,
};

// ── Orchestrator 特有类型 ──

use serde::{Deserialize, Serialize};

// ── WorkerAssignment ───────────────────────────────────────────────────

/// 将子任务分配给特定 worker Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerAssignment {
    /// 被分配的子任务
    pub sub_task_id: String,
    /// 生成的 worker 节点 ID
    pub worker_node_id: String,
    /// 分配的 Agent 角色
    pub role: String,
    /// 为 worker 生成的系统提示词
    pub system_prompt: String,
}

// ── StructuredHandover ─────────────────────────────────────────────────

/// Agent 之间的结构化交接协议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredHandover {
    /// Agent 完成的工作总结
    pub completed_work: String,
    /// 变更文件列表及摘要
    pub changes: Vec<ChangeRecord>,
    /// 下一个 Agent 应如何处理此输出
    pub next_steps: String,
    /// 仍未解决的已知问题
    pub remaining_issues: String,
    /// 下游 Agent 需要的依赖
    pub dependencies: String,
    /// 工作已验证的证据（如测试结果）
    pub validation_evidence: String,
}

impl StructuredHandover {
    /// 是否所有必需字段都非空
    pub fn is_complete(&self) -> bool {
        !self.completed_work.is_empty()
            && !self.changes.is_empty()
            && !self.next_steps.is_empty()
            && !self.remaining_issues.is_empty()
            && !self.dependencies.is_empty()
            && !self.validation_evidence.is_empty()
    }

    /// 返回缺失的字段名列表
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.completed_work.is_empty() {
            missing.push("completed_work");
        }
        if self.changes.is_empty() {
            missing.push("changes");
        }
        if self.next_steps.is_empty() {
            missing.push("next_steps");
        }
        if self.remaining_issues.is_empty() {
            missing.push("remaining_issues");
        }
        if self.dependencies.is_empty() {
            missing.push("dependencies");
        }
        if self.validation_evidence.is_empty() {
            missing.push("validation_evidence");
        }
        missing
    }
}

/// 交接中的文件变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    /// 文件路径
    pub file_path: String,
    /// 变更类型
    pub change_type: ChangeType,
    /// 变更摘要
    pub summary: String,
    /// 添加行数
    pub lines_added: Option<u32>,
    /// 删除行数
    pub lines_removed: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Create,
    Modify,
    Delete,
    Refactor,
    Format,
    Config,
}

// ── OrchestrationEvent ───────────────────────────────────────────────────

/// 编排期间发出的事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum OrchestrationEvent {
    /// 任务接收，分解开始
    DecompositionStarted { mission: String, strategy: String },
    /// 分解完成，生成 N 个子任务
    DecompositionCompleted { sub_task_count: usize, plan: DecompositionPlan },
    /// 子任务已派发至 worker
    SubTaskDispatched { sub_task_id: String, worker_node_id: String },
    /// 子任务成功完成
    SubTaskCompleted { sub_task_id: String, handover: Option<StructuredHandover> },
    /// 子任务失败
    SubTaskFailed { sub_task_id: String, error: String },
    /// 因失败触发重规划
    ReplanTriggered { failed_sub_tasks: Vec<String>, replan_round: u32 },
    /// 编排全部完成
    OrchestrationCompleted { total_sub_tasks: usize, completed: usize, failed: usize },
    /// 编排中止（超过最大重规划次数）
    OrchestrationAborted { reason: String },
}
